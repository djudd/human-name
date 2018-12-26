use super::namepart::{Location, NamePart};
use super::suffix;
use super::surname;
use super::title;
use super::utils::is_mixed_case;
use smallvec::SmallVec;

#[derive(Debug)]
struct ParseOp<'a> {
    words: SmallVec<[NamePart<'a>; 7]>,
    surname_index: usize,
    generation_from_suffix: Option<usize>,
    possible_false_prefix: Option<NamePart<'a>>,
    possible_false_postfix: Option<NamePart<'a>>,
    use_capitalization: bool,
}

pub fn parse(name: &str) -> Option<(SmallVec<[NamePart; 7]>, usize, Option<usize>)> {
    let mut op = ParseOp {
        words: SmallVec::new(),
        surname_index: 0,
        generation_from_suffix: None,
        possible_false_prefix: None,
        possible_false_postfix: None,
        use_capitalization: is_mixed_case(name),
    };

    if op.run(name) {
        Some((op.words, op.surname_index, op.generation_from_suffix))
    } else {
        None
    }
}

impl<'a> ParseOp<'a> {
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
            if let Some(ref postfix) = self.possible_false_postfix {
                self.words.push(postfix.clone());
            } else if let Some(ref prefix) = self.possible_false_prefix {
                self.words.insert(0, prefix.clone());
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
                && self.possible_false_prefix.is_none()
                && self.possible_false_postfix.is_none(),
            "Invalid state for handle_no_comma!"
        );

        self.words.extend(
            NamePart::all_from_text(name, self.use_capitalization, Location::Start)
                .skip_while(|word| !word.is_namelike() && !word.is_initials()),
        );

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
            if self.words.len() + self.possible_false_prefix.iter().count() > 2 {
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
                && self.possible_false_prefix.is_none()
                && self.possible_false_postfix.is_none(),
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

        if self.possible_false_postfix.is_some() && self.generation_from_suffix.is_some() {
            return;
        }

        let mut postfix_words =
            NamePart::all_from_text(part, self.use_capitalization, Location::End);
        while self.possible_false_postfix.is_none() || self.generation_from_suffix.is_none() {
            if let Some(word) = postfix_words.next() {
                self.found_suffix_or_postfix(word, false);
            } else {
                break;
            }
        }
    }

    fn strip_prefix(&mut self, len: usize) {
        for i in (0..len).rev() {
            let word = self.words.remove(i);
            self.found_prefix(word);
        }
    }

    fn strip_unsaved_prefix(&mut self, words: &mut SmallVec<[NamePart<'a>; 5]>, len: usize) {
        for i in (0..len).rev() {
            self.found_prefix(words.remove(i));
        }
    }

    fn found_prefix(&mut self, prefix: NamePart<'a>) {
        // We drop prefixes, but keep the last word that's namelike,
        // just in case we make a mistake and it turns out by process of
        // elimination that this must actually be a given name
        if self.possible_false_prefix.is_none() && (prefix.is_namelike() || prefix.is_initials()) {
            self.possible_false_prefix = Some(prefix);
        }
    }

    fn strip_postfix(&mut self, index: usize) {
        if index < self.words.len() {
            let postfix = self.words.swap_remove(index);
            self.found_suffix_or_postfix(postfix, false);
            self.words.truncate(index);
        }
    }

    fn strip_unsaved_postfix(&mut self, words: &mut SmallVec<[NamePart<'a>; 5]>, index: usize) {
        if index < words.len() {
            let postfix = words.swap_remove(index);
            self.found_suffix_or_postfix(postfix, false);
            words.truncate(index);
        }
    }

    fn found_suffix_or_postfix(&mut self, postfix: NamePart<'a>, expect_initials: bool) {
        if self.generation_from_suffix.is_none() {
            self.generation_from_suffix = suffix::generation_from_suffix(&postfix, expect_initials);
        }

        // We throw away most postfix titles, but keep the first one that's namelike,
        // just in case we make a mistake and it turns out by process of elimination
        // that this must actually be a surname
        if self.possible_false_postfix.is_none() && (postfix.is_namelike() || postfix.is_initials())
        {
            self.possible_false_postfix = Some(postfix);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::{black_box, Bencher};

    #[test]
    fn first_last() {
        let (parts, surname_index, generation) = parse("John Doe").unwrap();
        assert_eq!("John", parts[0].word);
        assert_eq!("Doe", parts[1].word);
        assert_eq!(1, surname_index);
        assert_eq!(None, generation);
    }

    #[test]
    fn initial_last() {
        let (parts, surname_index, generation) = parse("J. Doe").unwrap();
        assert_eq!("J.", parts[0].word);
        assert_eq!("Doe", parts[1].word);
        assert_eq!(1, surname_index);
        assert_eq!(None, generation);
    }

    #[test]
    fn last_first() {
        let (parts, surname_index, generation) = parse("Doe, John").unwrap();
        assert_eq!("John", parts[0].word);
        assert_eq!("Doe", parts[1].word);
        assert_eq!(1, surname_index);
        assert_eq!(None, generation);
    }

    #[test]
    fn last_initial() {
        let (parts, surname_index, generation) = parse("Doe, J.").unwrap();
        assert_eq!("J.", parts[0].word);
        assert_eq!("Doe", parts[1].word);
        assert_eq!(1, surname_index);
        assert_eq!(None, generation);
    }

    #[test]
    fn suffix() {
        let (parts, surname_index, generation) = parse("John Doe III").unwrap();
        assert_eq!("John", parts[0].word);
        assert_eq!("Doe", parts[1].word);
        assert_eq!(1, surname_index);
        assert_eq!(Some(3), generation);
    }

    #[test]
    fn suffix_comma() {
        let (parts, surname_index, generation) = parse("Doe, John III").unwrap();
        assert_eq!("John", parts[0].word);
        assert_eq!("Doe", parts[1].word);
        assert_eq!(1, surname_index);
        assert_eq!(Some(3), generation);
    }

    #[test]
    fn intermediate_suffix() {
        let (parts, surname_index, generation) = parse("Doe, II, John").unwrap();
        assert_eq!("John", parts[0].word);
        assert_eq!("Doe", parts[1].word);
        assert_eq!(1, surname_index);
        assert_eq!(Some(2), generation);

        let (parts, surname_index, generation) = parse("Griffey, Jr., Ken").unwrap();
        assert_eq!("Ken", parts[0].word);
        assert_eq!("Griffey", parts[1].word);
        assert_eq!(1, surname_index);
        assert_eq!(Some(2), generation);
    }

    #[bench]
    fn parse_simple(b: &mut Bencher) {
        b.iter(|| black_box(parse("John Doe").is_some()))
    }

    #[bench]
    fn parse_nonascii(b: &mut Bencher) {
        b.iter(|| black_box(parse("이용희").is_some()))
    }

    #[bench]
    fn parse_comma(b: &mut Bencher) {
        b.iter(|| black_box(parse("Doe, John").is_some()))
    }

    #[bench]
    fn parse_all_caps(b: &mut Bencher) {
        b.iter(|| black_box(parse("JOHN DOE").is_some()))
    }

    #[bench]
    fn parse_complex(b: &mut Bencher) {
        b.iter(|| black_box(parse("James S. Brown MD, FRCS, FDSRCS").is_some()))
    }
}
