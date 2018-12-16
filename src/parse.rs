use std::cmp;
use super::title;
use super::surname;
use super::suffix;
use super::namepart::{NamePart, Location, Category};

struct ParseOp<'a> {
    surname_index: usize,
    generation_from_suffix: Option<usize>,
    maybe_not_prefix: Option<NamePart<'a>>,
    maybe_not_postfix: Option<NamePart<'a>>,
    use_capitalization: bool,
}

pub fn parse(name: &str,
             use_capitalization: bool)
             -> Option<(Vec<NamePart>, usize, Option<usize>)> {
    let op = ParseOp {
        surname_index: 0,
        generation_from_suffix: None,
        maybe_not_prefix: None,
        maybe_not_postfix: None,
        use_capitalization,
    };

    let (words, surname_index, generation_from_suffix) = op.run(name);

    let successful = words.len() >= 2 && words.iter().all(|w| w.is_namelike() || w.is_initials()) &&
                     surname_index > 0 && surname_index < 6 &&
                     words[surname_index..].iter().any(|w| w.is_namelike());

    if successful {
        Some((words, surname_index, generation_from_suffix))
    } else {
        None
    }
}

impl <'a>ParseOp<'a> {

    fn run(mut self, name: &'a str) -> (Vec<NamePart<'a>>, usize, Option<usize>) {
        let mut words: Vec<NamePart> = Vec::with_capacity(2);

        // Separate comma-separated titles and suffixes, then flip remaining words
        // around remaining comma, if any
        for part in name.split(',') {
            if words.is_empty() {
                // We're in the surname part (if the format is "Smith, John"),
                // or the only actual name part (if the format is "John Smith,
                // esq." or just "John Smith")
                words = self.handle_before_comma(part, words);
            } else if self.surname_index == 0 {
                // We already processed one comma-separated part, but we think
                // it was just the surname, so this might be the given name or
                // initials
                words = self.handle_after_comma(part, words);
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
        if ParseOp::fixably_invalid(&words, self.surname_index) {
            if self.maybe_not_postfix.is_some() {
                words.push(self.maybe_not_postfix.unwrap());
            } else if self.maybe_not_prefix.is_some() {
                words.insert(0, self.maybe_not_prefix.unwrap());
            }
        }

        // Anything trailing that looks like initials is probably a stray postfix
        while !words.is_empty() && !words.last().unwrap().is_namelike() {
            let mut removed = words.pop().unwrap();

            // If we guessed the surname previously, our guess is no longer valid
            self.surname_index = 0;

            // If the alternative is having less than two words and giving up,
            // check if we can treat the last word as a name if we just ignore
            // case; this handles the not-quite-rare-enough case of an all-caps
            // last name (e.g.Neto John SMITH), among others
            if self.use_capitalization &&
                ParseOp::fixably_invalid(&words, self.surname_index) &&
                NamePart::from_word(&*removed.namecased, false, Location::End).is_namelike() {

                removed.category = Category::Name;
                words.push(removed);
                break;
            }
        }

        // Handle case where we thought the whole before-comma part was a surname,
        // but we never found a plausible given name or initial afterwards,
        // as well as the reset just above
        if self.surname_index == 0 && words.len() > 1 {
            self.surname_index = surname::find_surname_index(&words[1..]) + 1;
        }

        (words, self.surname_index, self.generation_from_suffix)
    }

    fn fixably_invalid(words: &[NamePart], surname_index: usize) -> bool {
        words.len() < 2 || !words[surname_index..].iter().any(|w| w.is_namelike())
    }

    // Called only until any words are found
    fn handle_before_comma(&mut self,
                           part: &'a str,
                           mut words: Vec<NamePart<'a>>)
                           -> Vec<NamePart<'a>> {
        debug_assert!(words.is_empty() && self.surname_index == 0,
                "Invalid state for handle_before_comma!");

        words.extend(NamePart::all_from_text(part, self.use_capitalization, Location::End));

        if words.is_empty() {
            return words;
        }

        // Check for title as prefix (e.g. "Dr. John Smith" or "Right Hon.
        // John Smith")
        let prefix_title = if words.len() > 1 {
            title::strip_prefix_title(&mut words)
        } else {
            None
        };

        // Strip non-comma-separated titles & suffixes (e.g. "John Smith Jr.")
        self.strip_postfixes(&mut words, false);

        if prefix_title.is_some() {
            // Finding a prefix title means the next word is a first name or
            // initial (we don't support "Dr. Smith, John")
            self.found_prefix_title(prefix_title.unwrap());
            self.surname_index = surname::find_surname_index(&words[1..]) + 1;
        } else {
            // Have to guess whether this is just the surname (as in "Smith, John")
            // or the full name (as in "John Smith")
            //
            // Note we might be wrong, and have to go back, if we think the given
            // name is coming after a comma, but it never does
            self.surname_index = surname::find_surname_index(&words);
        }

        words
    }

    // Called after the first comma, until we find a given name or first initial
    fn handle_after_comma(&mut self,
                          part: &'a str,
                          mut words: Vec<NamePart<'a>>)
                          -> Vec<NamePart<'a>> {
        debug_assert!(!words.is_empty() && self.surname_index == 0,
                "Invalid state for handle_after_comma!");

        let mut given_middle_or_postfix_words: Vec<NamePart> =
            NamePart::all_from_text(part, self.use_capitalization, Location::Start).collect();

        if given_middle_or_postfix_words.is_empty() {
            return words;
        }

        // Handle (unusual) formats like "Smith, Dr. John M."
        if given_middle_or_postfix_words.len() > 1 {
            if let Some(prefix_title) =
                   title::strip_prefix_title(&mut given_middle_or_postfix_words) {
                self.found_prefix_title(prefix_title);
            }
        }

        // Handle isolated suffixes or titles as well as (unusual) formats like
        // "Smith, John Jr."
        self.strip_postfixes(&mut given_middle_or_postfix_words, true);

        // Now if there are any words left, they include the given name or first
        // initial (in a format like "Smith, John" or "Smith, J. M."), so we put
        // them in front
        if !given_middle_or_postfix_words.is_empty() {
            let surname_words = words;
            words = given_middle_or_postfix_words;
            self.surname_index = words.len();
            words.extend(surname_words);
        }

        words
    }

    // Called on any parts remaining after full name is found
    fn handle_after_surname(&mut self, part: &'a str) {
        debug_assert!(self.surname_index > 0,
                "Invalid state for handle_after_surname!");

        if self.maybe_not_postfix.is_some() && self.generation_from_suffix.is_some() {
            return;
        }

        let mut postfix_words = NamePart::all_from_text(part,
                                                        self.use_capitalization,
                                                        Location::End);
        while self.maybe_not_postfix.is_none() || self.generation_from_suffix.is_none() {
            match postfix_words.next() {
                Some(word) => {
                    if let Some(generation) = suffix::generation_from_suffix(&word, false) {
                        self.found_suffix(word, generation);
                    } else {
                        self.found_postfix_title(word);
                    }
                }
                None => {
                    break;
                }
            }
        }
    }

    fn strip_postfixes(&mut self, words: &mut Vec<NamePart<'a>>, after_comma: bool) {
        let skip = if after_comma {
            0
        } else {
            1
        };
        let expect_initials = after_comma && self.surname_index == 0;

        let last_nonpostfix_index = words[skip..].iter().rposition(|word| {
            suffix::generation_from_suffix(&word, expect_initials).is_none() &&
            !title::is_postfix_title(&word, expect_initials)
        });

        let first_abbr_index = words[skip..]
                                   .iter()
                                   .position(|word| !word.is_namelike() && !word.is_initials())
                                   .unwrap_or_else(|| words[skip..].len()) +
                               skip;

        let first_postfix_index = cmp::min(first_abbr_index,
                                           match last_nonpostfix_index {
                                               Some(i) => i + 1 + skip,
                                               None => skip,
                                           });

        if first_postfix_index < words.len() {
            while words.len() > first_postfix_index + 1 {
                words.pop();
            }

            let first_postfix = words.pop().unwrap();
            if let Some(generation) = suffix::generation_from_suffix(&first_postfix,
                                                                     expect_initials) {
                self.found_suffix(first_postfix, generation);
            } else {
                self.found_postfix_title(first_postfix);
            }
        }
    }

    fn found_suffix(&mut self, suffix: NamePart<'a>, generation: usize) {
        if self.generation_from_suffix.is_none() {
            self.generation_from_suffix = Some(generation);
        }

        self.found_postfix_title(suffix);
    }

    // We throw away most postfix titles, but keep the first one that's namelike,
    // just in case we make a mistake and it turns out by process of elimination
    // that this must actually be a surname
    fn found_postfix_title(&mut self, postfix: NamePart<'a>) {
        if self.maybe_not_postfix.is_none() && (postfix.is_namelike() || postfix.is_initials()) {
            self.maybe_not_postfix = Some(postfix);
        }
    }

    // Ditto prefixes
    fn found_prefix_title(&mut self, prefix: Vec<NamePart<'a>>) {
        if self.maybe_not_prefix.is_none() {
            if let Some(word) = prefix.into_iter()
                                      .rev()
                                      .find(|word| word.is_namelike() || word.is_initials()) {
                self.maybe_not_prefix = Some(word);
            }
        }
    }
}
