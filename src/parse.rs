use super::case::is_mixed_case;
use super::namepart::{Location, NamePart};
use super::suffix;
use super::surname;
use super::title;
use crate::Cow;
use smallvec::SmallVec;
use std::num::NonZeroU8;

pub struct Name<'a> {
    parts: SmallVec<[NamePart<'a>; 7]>,
    pub surname_index: usize,
    pub generation: Option<NonZeroU8>,
    reversed_prefixes: Vec<NamePart<'a>>,
    honorific_suffixes: Vec<NamePart<'a>>,
}

impl<'a> Name<'a> {
    pub fn words(&self) -> &[NamePart<'a>] {
        self.parts.as_ref()
    }

    pub fn honorific_prefix(&self) -> Option<Cow<str>> {
        match self.reversed_prefixes.len() {
            0 => None,
            1 => self
                .reversed_prefixes
                .get(0)
                .map(title::canonicalize_prefix),
            _ => Some(Cow::Owned(
                self.reversed_prefixes
                    .iter()
                    .rev()
                    .map(title::canonicalize_prefix)
                    .collect::<SmallVec<[Cow<str>; 4]>>()
                    .join(" "),
            )),
        }
    }

    pub fn honorific_suffix(&self) -> Option<Cow<str>> {
        match self.honorific_suffixes.len() {
            0 => None,
            1 => self
                .honorific_suffixes
                .get(0)
                .map(title::canonicalize_suffix),
            _ => Some(Cow::Owned(
                self.honorific_suffixes
                    .iter()
                    .map(title::canonicalize_suffix)
                    .collect::<SmallVec<[Cow<str>; 4]>>()
                    .join(" "),
            )),
        }
    }
}

#[derive(Debug)]
struct ParseOp<'a> {
    // Output
    words: SmallVec<[NamePart<'a>; 7]>,
    surname_index: usize,
    generation_from_suffix: Option<NonZeroU8>,
    reversed_prefixes: Vec<NamePart<'a>>,
    honorific_suffixes: Vec<NamePart<'a>>,

    // Working space
    use_capitalization: bool,
}

pub const MAX_WORDS: usize = u8::max_value() as usize;

pub fn parse(name: &str) -> Option<Name> {
    let mut op = ParseOp {
        words: SmallVec::new(),
        surname_index: 0,
        generation_from_suffix: None,
        reversed_prefixes: Vec::new(),
        honorific_suffixes: Vec::new(),
        use_capitalization: is_mixed_case(name),
    };

    if op.run(name) {
        Some(Name {
            parts: op.words,
            surname_index: op.surname_index,
            generation: op.generation_from_suffix,
            reversed_prefixes: op.reversed_prefixes,
            honorific_suffixes: op.honorific_suffixes,
        })
    } else {
        None
    }
}

impl<'a> ParseOp<'a> {
    /// Responsible for the main parse operation: segments input by commas,
    /// then by word separators, while assigning a preliminary categorization
    /// to each segment, and then determines a final categorization for each
    /// segment taking into account the sequence of statements.
    ///
    /// Returns true on success & false on inability to parse into a plausible
    /// name.
    fn run(&mut self, name: &'a str) -> bool {
        // Separate comma-separated titles and suffixes, then flip remaining words
        // around remaining comma, if any
        let mut parts = name.split(',').peekable();
        while let Some(part) = parts.next() {
            let first_part = self.words.is_empty();
            let last_part = parts.peek().is_none();

            if first_part && last_part {
                // Simple case
                self.handle_no_comma(part);
            } else if first_part {
                // We're in the surname part if the format is like "Smith, John",
                // or the only actual name part if the format is "John Smith,
                // esq."
                self.handle_before_comma(part);
            } else if self.surname_index == 0 {
                // We already processed one comma-separated part, but we think
                // it was just the surname, so this might be the given name or
                // initials
                let must_include_given = last_part && self.words.len() == 1;
                self.handle_after_comma(part, must_include_given);
            } else {
                // We already found the full name, so this is a comma-separated
                // postfix title or suffix
                self.handle_after_surname(part);
            }
        }

        // If there are two or fewer words, e.g. "JOHN MA", we treat
        // ambiguous strings like "MA" as surnames rather than titles
        // (not initials, which should only be at the end of the input
        // if they are comma-separated, and we already handled that case)
        if !self.valid() {
            if let Some(i) = self.possible_false_postfix() {
                self.words.push(self.honorific_suffixes.remove(i));
            } else if let Some(i) = self.possible_false_prefix() {
                self.words.insert(0, self.reversed_prefixes.remove(i));
            }
        }

        // Anything trailing that looks like initials is probably a stray postfix
        while self.words.last().filter(|w| !w.is_namelike()).is_some() {
            let removed = self.words.pop().unwrap();

            // If we guessed the surname previously, our guess is no longer valid
            self.surname_index = 0;

            // If the alternative is having less than two words and giving up,
            // check if we can treat the last word as a name if we just ignore
            // case; this handles the not-quite-rare-enough case of an all-caps
            // last name (e.g. Neto John SMITH), among others
            if self.use_capitalization && !self.valid() {
                let word = NamePart::from_word(removed.word, false, Location::End);
                if word.is_namelike() {
                    self.words.push(word);
                    break;
                }
            }
        }

        // Handle case where we thought the whole before-comma part was a surname,
        // but we never found a plausible given name or initial afterwards,
        // as well as the reset just above
        if self.surname_index == 0 && self.words.len() > 1 {
            self.surname_index = surname::find_surname_index(&self.words[1..]) + 1;
        }

        // Check the plausibility of what we've found
        self.valid()
    }

    fn valid(&self) -> bool {
        self.words.len() >= 2
            && self.words.len() <= MAX_WORDS
            && self
                .words
                .iter()
                .all(|w| w.is_namelike() || w.is_initials())
            && self.words[self.surname_index..]
                .iter()
                .any(|w| w.is_namelike())
    }

    fn handle_no_comma(&mut self, name: &'a str) {
        debug_assert!(
            self.words.is_empty()
                && self.surname_index == 0
                && self.possible_false_prefix().is_none()
                && self.possible_false_postfix().is_none(),
            "Invalid state for handle_no_comma!"
        );

        let mut in_prefix = true;
        for word in NamePart::all_from_text(name, self.use_capitalization, Location::Start) {
            if in_prefix && (word.is_namelike() || word.is_initials()) {
                in_prefix = false;
            }

            if in_prefix {
                self.reversed_prefixes.insert(0, word);
            } else {
                self.words.push(word);
            }
        }

        if self.words.is_empty() {
            return;
        }

        // Check for title as prefix (e.g. "Dr. John Smith" or "Right Hon.
        // John Smith")
        let prefix_title_len = if self.words.len() > 2 {
            title::find_prefix_len(&self.words)
        } else {
            0
        };
        self.strip_prefix(prefix_title_len);

        // Strip non-comma-separated titles & suffixes (e.g. "John Smith Jr.")
        let first_postfix_index =
            if self.words.len() + self.possible_false_prefix().iter().count() > 2 {
                title::find_postfix_index(&self.words[1..], false) + 1
            } else {
                self.words.len()
            };
        self.strip_postfix(first_postfix_index);

        self.surname_index = surname::find_surname_index(&self.words[1..]) + 1;
    }

    // Called only until any words are found
    fn handle_before_comma(&mut self, part: &'a str) {
        debug_assert!(
            self.words.is_empty()
                && self.surname_index == 0
                && self.possible_false_prefix().is_none()
                && self.possible_false_postfix().is_none(),
            "Invalid state for handle_before_comma!"
        );

        self.words.extend(NamePart::all_from_text(
            part,
            self.use_capitalization,
            Location::End,
        ));

        if self.words.is_empty() {
            return;
        }

        // Check for title as prefix (e.g. "Dr. John Smith, Jr.")
        let prefix_title_len = title::find_prefix_len(&self.words);
        self.strip_prefix(prefix_title_len);

        // Strip non-comma-separated titles & suffixes (e.g. "John Smith Jr., MD")
        let first_postfix_index = title::find_postfix_index(&self.words[1..], false) + 1;
        self.strip_postfix(first_postfix_index);

        if prefix_title_len > 0 {
            // Finding a prefix title means the next word is a first name or
            // initial (we don't support "Dr. Smith, John")
            self.surname_index = surname::find_surname_index(&self.words[1..]) + 1;
        } else {
            // Have to guess whether this is just the surname (as in "Smith, John")
            // or the full name (as in "John Smith")
            //
            // Note we might be wrong, and have to go back, if we think the given
            // name is coming after a comma, but it never does
            self.surname_index = surname::find_surname_index(&self.words);
        }
    }

    // Called after the first comma, until we find a given name or first initial
    fn handle_after_comma(&mut self, part: &'a str, must_include_given: bool) {
        debug_assert!(
            !self.words.is_empty() && self.surname_index == 0,
            "Invalid state for handle_after_comma!"
        );

        let mut given_middle_or_postfix_words: SmallVec<[NamePart<'a>; 5]> =
            NamePart::all_from_text(part, self.use_capitalization, Location::Start).collect();

        if given_middle_or_postfix_words.is_empty() {
            return;
        }

        // Handle (unusual) formats like "Smith, Dr. John M."
        if given_middle_or_postfix_words.len() > 1 {
            let prefix_len = title::find_prefix_len(&given_middle_or_postfix_words);
            self.strip_unsaved_prefix(&mut given_middle_or_postfix_words, prefix_len);
        }

        // Handle isolated suffixes or titles as well as (unusual) formats like
        // "Smith, John Jr." and "Smith, Jr., John"
        let first_postfix_index = if must_include_given {
            title::find_postfix_index(&given_middle_or_postfix_words[1..], true) + 1
        } else {
            title::find_postfix_index(&given_middle_or_postfix_words, true)
        };
        self.strip_unsaved_postfix(&mut given_middle_or_postfix_words, first_postfix_index);

        // Now if there are any words left, they include the given name or first
        // initial (in a format like "Smith, John" or "Smith, J. M."), so we put
        // them in front
        if !given_middle_or_postfix_words.is_empty() {
            self.surname_index = given_middle_or_postfix_words.len();

            self.words.reserve(given_middle_or_postfix_words.len());
            self.words
                .insert_many(0, given_middle_or_postfix_words.into_iter());
        }
    }

    // Called on any parts remaining after full name is found
    fn handle_after_surname(&mut self, part: &'a str) {
        debug_assert!(
            self.surname_index > 0,
            "Invalid state for handle_after_surname!"
        );

        for word in NamePart::all_from_text(part, self.use_capitalization, Location::End) {
            self.found_suffix_or_postfix(word, false);
        }
    }

    fn strip_prefix(&mut self, len: usize) {
        for i in (0..len).rev() {
            let word = self.words.remove(i);
            self.reversed_prefixes.push(word);
        }
    }

    fn strip_unsaved_prefix(&mut self, words: &mut SmallVec<[NamePart<'a>; 5]>, len: usize) {
        for i in (0..len).rev() {
            self.reversed_prefixes.push(words.remove(i));
        }
    }

    // Find the last prefix that's namelike, just in case we made a mistake
    // and it turns out by process of elimination that this must actually be
    // a given name
    fn possible_false_prefix(&self) -> Option<usize> {
        self.reversed_prefixes
            .iter()
            .position(|p| p.is_namelike() || p.is_initials())
    }

    // Find the first suffix that's namelike, just in case we make a mistake
    // and it turns out by process of elimination that this must actually be
    // a surname
    fn possible_false_postfix(&self) -> Option<usize> {
        self.honorific_suffixes
            .iter()
            .position(|p| p.is_namelike() || p.is_initials())
    }

    fn strip_postfix(&mut self, index: usize) {
        if index < self.words.len() {
            let postfixes = self
                .words
                .drain(index..)
                .collect::<SmallVec<[NamePart<'a>; 5]>>();
            for postfix in postfixes {
                self.found_suffix_or_postfix(postfix, false);
            }
            self.words.truncate(index);
        }
    }

    fn strip_unsaved_postfix(&mut self, words: &mut SmallVec<[NamePart<'a>; 5]>, index: usize) {
        if index < words.len() {
            for postfix in words.drain(index..) {
                self.found_suffix_or_postfix(postfix, false);
            }
        }
    }

    fn found_suffix_or_postfix(&mut self, postfix: NamePart<'a>, expect_initials: bool) {
        if self.generation_from_suffix.is_none() {
            if let Some(gen) = suffix::generation_from_suffix(&postfix, expect_initials) {
                self.generation_from_suffix = Some(gen);
                return;
            }
        }

        self.honorific_suffixes.push(postfix);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "bench")]
    use test::{black_box, Bencher};

    #[test]
    fn first_last() {
        let Name {
            parts,
            surname_index,
            generation,
            ..
        } = parse("John Doe").unwrap();
        assert_eq!("John", parts[0].word);
        assert_eq!("Doe", parts[1].word);
        assert_eq!(1, surname_index);
        assert_eq!(None, generation);
    }

    #[test]
    fn initial_last() {
        let Name {
            parts,
            surname_index,
            generation,
            ..
        } = parse("J. Doe").unwrap();
        assert_eq!("J.", parts[0].word);
        assert_eq!("Doe", parts[1].word);
        assert_eq!(1, surname_index);
        assert_eq!(None, generation);
    }

    #[test]
    fn last_first() {
        let Name {
            parts,
            surname_index,
            generation,
            ..
        } = parse("Doe, John").unwrap();
        assert_eq!("John", parts[0].word);
        assert_eq!("Doe", parts[1].word);
        assert_eq!(1, surname_index);
        assert_eq!(None, generation);
    }

    #[test]
    fn last_initial() {
        let Name {
            parts,
            surname_index,
            generation,
            ..
        } = parse("Doe, J.").unwrap();
        assert_eq!("J.", parts[0].word);
        assert_eq!("Doe", parts[1].word);
        assert_eq!(1, surname_index);
        assert_eq!(None, generation);
    }

    #[test]
    fn suffix() {
        let Name {
            parts,
            surname_index,
            generation,
            ..
        } = parse("John Doe III").unwrap();
        assert_eq!("John", parts[0].word);
        assert_eq!("Doe", parts[1].word);
        assert_eq!(1, surname_index);
        assert_eq!(NonZeroU8::new(3), generation);
    }

    #[test]
    fn suffix_comma() {
        let Name {
            parts,
            surname_index,
            generation,
            ..
        } = parse("Doe, John III").unwrap();
        assert_eq!("John", parts[0].word);
        assert_eq!("Doe", parts[1].word);
        assert_eq!(1, surname_index);
        assert_eq!(NonZeroU8::new(3), generation);
    }

    #[test]
    fn intermediate_suffix() {
        let Name {
            parts,
            surname_index,
            generation,
            ..
        } = parse("Doe, II, John").unwrap();
        assert_eq!("John", parts[0].word);
        assert_eq!("Doe", parts[1].word);
        assert_eq!(1, surname_index);
        assert_eq!(NonZeroU8::new(2), generation);

        let Name {
            parts,
            surname_index,
            generation,
            ..
        } = parse("Griffey, Jr., Ken").unwrap();
        assert_eq!("Ken", parts[0].word);
        assert_eq!("Griffey", parts[1].word);
        assert_eq!(1, surname_index);
        assert_eq!(NonZeroU8::new(2), generation);
    }

    #[test]
    fn honorifics() {
        let name = parse("Lt Col Sir John Doe, X, YY, ZZZ").unwrap();
        assert_eq!("Lt. Col. Sir", name.honorific_prefix().unwrap());
        assert_eq!("X. Y.Y. ZZZ", name.honorific_suffix().unwrap());

        let name = parse("Doe, Lt Col Sir John, X, YY, ZZZ").unwrap();
        assert_eq!("Lt. Col. Sir", name.honorific_prefix().unwrap());
        assert_eq!("X. Y.Y. ZZZ", name.honorific_suffix().unwrap());

        let name = parse("Air Chief Marshal Sir Stuart William Peach, GBE, KCB, ADC, DL").unwrap();
        assert_eq!("Air Chief Marshal Sir", name.honorific_prefix().unwrap());
        assert_eq!("GBE KCB ADC D.L.", name.honorific_suffix().unwrap());

        let name = parse("Air Chief Marshal Sir Stuart William Peach GBE KCB ADC DL").unwrap();
        assert_eq!("Air Chief Marshal Sir", name.honorific_prefix().unwrap());
        assert_eq!("GBE KCB ADC D.L.", name.honorific_suffix().unwrap());

        let name = parse("Peach, Air Chief Marshal Sir Stuart William, GBE KCB ADC DL").unwrap();
        assert_eq!("Air Chief Marshal Sir", name.honorific_prefix().unwrap());
        assert_eq!("GBE KCB ADC D.L.", name.honorific_suffix().unwrap());
    }

    // Treating this as an honorific suffix isn't really right, but it does produce
    // the correct display output, and "et al" is unfortunately common in some data
    // so we need to do something vaguely sane at least.
    #[test]
    fn et_al() {
        let name = parse("Dr. Jane Doe, et al").unwrap();
        assert_eq!("et al.", name.honorific_suffix().unwrap());

        let name = parse("DR JANE DOE ET AL").unwrap();
        assert_eq!("et al.", name.honorific_suffix().unwrap());
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn parse_simple(b: &mut Bencher) {
        b.iter(|| black_box(parse("John Doe").is_some()))
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn parse_nonascii(b: &mut Bencher) {
        b.iter(|| black_box(parse("이용희").is_some()))
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn parse_comma(b: &mut Bencher) {
        b.iter(|| black_box(parse("Doe, John").is_some()))
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn parse_all_caps(b: &mut Bencher) {
        b.iter(|| black_box(parse("JOHN DOE").is_some()))
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn parse_complex(b: &mut Bencher) {
        b.iter(|| black_box(parse("James S. Brown MD, FRCS, FDSRCS").is_some()))
    }
}
