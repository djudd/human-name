#![feature(drain)]
#![feature(plugin)]
#![plugin(phf_macros)]

extern crate phf;
extern crate itertools;
extern crate unicode_segmentation;
extern crate unicode_normalization;
extern crate rustc_serialize;

mod utils;
mod suffix;
mod nickname;
mod title;
mod surname;
mod namecase;
mod namepart;

use itertools::Itertools;
use utils::*;
use namepart::{NamePart, Location};

struct ParseResult<'a> {
    words: Vec<NamePart<'a>>,
    surname_index: usize,
    suffix_index: usize,
}

impl <'a>ParseResult<'a> {

    pub fn given_and_middle_names_or_initials(&self) -> &'a [NamePart] {
        &self.words[0..self.surname_index]
    }

    pub fn surname_words(&self) -> &'a [NamePart] {
        &self.words[self.surname_index..self.suffix_index]
    }

    pub fn suffix(&self) -> Option<&NamePart> {
        self.words[self.suffix_index..].iter().nth(0)
    }

}

struct ParseOp<'a> {
    surname_index: usize,
    suffix: Option<NamePart<'a>>,
    maybe_not_postfix: Option<NamePart<'a>>,
    use_capitalization: bool,
}

impl <'a>ParseOp<'a> {

    pub fn parse(name: &'a str, use_capitalization: bool) -> Option<ParseResult<'a>> {
        let op = ParseOp {
            surname_index: 0,
            suffix: None,
            maybe_not_postfix: None,
            use_capitalization: use_capitalization,
        };

        let (words, surname_index, suffix_index) = op.run(name);

        let successful = words.len() >= 2 &&
            words[0..suffix_index].iter().all( |w| w.is_namelike() || w.is_initials() ) &&
            surname_index > 0 &&
            suffix_index > surname_index &&
            words[surname_index..suffix_index].iter().any( |w| w.is_namelike() );

        if successful {
            Some(ParseResult {
                words: words,
                surname_index: surname_index,
                suffix_index: suffix_index,
            })
        }
        else {
            None
        }
    }

    fn run(mut self, name: &'a str) -> (Vec<NamePart<'a>>, usize, usize) {
        let mut words: Vec<NamePart> = Vec::new();

        // Separate comma-separated titles and suffixes, then flip remaining words
        // around remaining comma, if any
        for (i, part) in name.split(",").enumerate() {
            if i == 0 || words.is_empty() {
                // We're in the surname part (if the format is "Smith, John"),
                // or the only actual name part (if the format is "John Smith,
                // esq." or just "John Smith")
                words = self.handle_before_comma(part, words);
            }
            else if self.surname_index == 0 {
                // We already processed one comma-separated part, but we think
                // it was just the surname, so this might be the given name or
                // initials
                words = self.handle_after_comma(part, words);
            }
            else {
                // We already found the full name, so this is a comma-separated
                // postfix title or suffix
                self.handle_definite_postfixes(part);
            }
        }

        // If there are two or fewer words, e.g. "JOHN MA", we treat
        // ambiguous strings like "MA" as surnames rather than titles
        // (not initials, which should only be at the end of the input
        // if they are comma-separated, and we already handled that case)
        if words.len() < 2 && self.maybe_not_postfix.is_some() {
            words.push(self.maybe_not_postfix.unwrap());
        }

        // Anything trailing that looks like initials is probably a stray postfix
        while !words.is_empty() && !words.last().unwrap().is_namelike() {
            words.pop();

            // If we guessed the surname was one of these trailing non-names,
            // try again
            if self.surname_index >= words.len() {
                self.surname_index = 0;
            }
        }

        // Handle case where we thought the whole before-comma part was a surname,
        // but we never found a plausible given name or initial afterwards,
        // as well as the reset just above
        if self.surname_index == 0 && words.len() > 1 {
            self.surname_index = surname::find_surname_index(&words[1..]) + 1;
        }

        // Append the suffix last
        let suffix_index = words.len();
        if self.suffix.is_some() {
            words.push(self.suffix.unwrap());
        }

        (words, self.surname_index, suffix_index)
    }

    // Called only until any words are found
    fn handle_before_comma(&mut self, part: &'a str, mut words: Vec<NamePart<'a>>) -> Vec<NamePart<'a>> {
        assert!(words.is_empty() && self.surname_index == 0, "Invalid state for handle_before_comma!");

        words.extend(NamePart::all_from_text(part, self.use_capitalization, Location::End));

        if words.is_empty() { return words }

        // Check for title as prefix (e.g. "Dr. John Smith" or "Right Hon.
        // John Smith")
        let mut stripped_prefix_title = false;
        if words.len() > 1 {
            stripped_prefix_title = title::strip_prefix_title(&mut words, true);
        }

        // Strip non-comma-separated titles & suffixes (e.g. "John Smith Jr.")
        self.strip_postfixes(&mut words, false);

        if stripped_prefix_title {
            // Finding a prefix title means the next word is a first name or
            // initial (we don't support "Dr. Smith, John")
            self.surname_index = surname::find_surname_index(&words[1..]) + 1;
        }
        else {
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
    fn handle_after_comma(&mut self, part: &'a str, mut words: Vec<NamePart<'a>>) -> Vec<NamePart<'a>> {
        assert!(!words.is_empty() && self.surname_index == 0, "Invalid state for handle_after_comma!");

        let mut given_middle_or_postfix_words: Vec<NamePart> =
            NamePart::all_from_text(part, self.use_capitalization, Location::Start).collect();

        if given_middle_or_postfix_words.is_empty() { return words }

        // Handle (unusual) formats like "Smith, Dr. John M."
        if given_middle_or_postfix_words.len() > 1 {
            title::strip_prefix_title(&mut given_middle_or_postfix_words, false);
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
    fn handle_definite_postfixes(&mut self, part: &'a str) {
        if self.maybe_not_postfix.is_some() && self.suffix.is_some() { return }

        let mut postfix_words = NamePart::all_from_text(part, self.use_capitalization, Location::End);
        while self.maybe_not_postfix.is_none() || self.suffix.is_none() {
            match postfix_words.next() {
                Some(word) => {
                    if suffix::is_suffix(&word) {
                        self.found_suffix(word);
                    }
                    else {
                        self.found_postfix_title(word);
                    }
                },
                None => { break }
            }
        }
    }

    fn strip_postfixes<'b>(&mut self, words: &mut Vec<NamePart<'a>>, include_first_word: bool) {
        let mut postfix_is_suffix = false;

        let postfix_position = words.iter().enumerate().position( |(i, word)|
            if i == 0 && !include_first_word {
                false
            }
            else if suffix::is_suffix(&word) {
                postfix_is_suffix = true;
                true
            }
            else if title::is_postfix_title(&word) {
                true
            }
            else {
                false
            }
        );

        if postfix_position.is_some() {
            let additional_postfixes_range = (postfix_position.unwrap()+1)..words.len();
            words.drain(additional_postfixes_range);

            let first_postfix = words.pop().unwrap();
            if postfix_is_suffix {
                self.found_suffix(first_postfix)
            }
            else {
                self.found_postfix_title(first_postfix)
            }
        }
    }

    fn found_suffix(&mut self, suffix: NamePart<'a>) {
        if self.suffix.is_none() {
            self.suffix = Some(suffix);
        }
        else {
            self.found_postfix_title(suffix);
        }
    }

    // We throw away most postfix titles, but keep the first one that's namelike,
    // just in case we make a mistake and it turns out by process of elimination
    // that this must actually be a surname
    fn found_postfix_title(&mut self, postfix: NamePart<'a>) {
        if self.maybe_not_postfix.is_none() && postfix.is_namelike() {
            self.maybe_not_postfix = Some(postfix);
        }
    }
}

#[derive(RustcDecodable, RustcEncodable)]
pub struct Name {
  pub given_name: Option<String>,
  pub surname: String,
  pub middle_names: Option<String>,
  pub first_initial: char,
  pub middle_initials: Option<String>,
  pub suffix: Option<String>,
}

impl Name {
    pub fn parse(name: &str) -> Option<Name> {
        if !name.chars().any(char::is_alphabetic) {
            return None
        }

        let mixed_case = is_mixed_case(name);
        let name = nickname::strip_nickname(name);

        let result = match ParseOp::parse(&*name, mixed_case) {
            Some(value) => value,
            None => { return None },
        };

        let first_initial = result.given_and_middle_names_or_initials()[0].initial();

        // Take the remaining words, and strip out the initials (if present;
        // only allow one block of initials) and the first name (if present),
        // and whatever's left are the middle names
        let mut given_name: Option<String> = None;
        let mut middle_names: Vec<&str> = Vec::new();
        let mut middle_initials = String::new();

        for (i, word) in result.given_and_middle_names_or_initials().iter().enumerate() {
            if word.is_initials() {
                let start = if i == 0 { 1 } else { 0 };
                if word.chars > start {
                    middle_initials.extend(
                        word.word
                            .chars()
                            .filter( |c| c.is_alphabetic() )
                            .skip(start)
                            .filter_map( |w| w.to_uppercase().next() ));
                }
            } else if given_name.is_none() {
                given_name = Some(word.namecased.to_string());
            } else {
                middle_names.push(&*word.namecased);
                middle_initials.push(word.initial());
            }
        }

        let middle_names =
            if middle_names.is_empty() {
                None
            } else {
                Some(middle_names.join(" "))
            };

        let middle_initials =
            if middle_initials.is_empty() {
                None
            } else {
                Some(middle_initials)
            };

        let surname = result
            .surname_words()
            .iter()
            .map( |w| &*w.namecased )
            .join(" ");

        let suffix = match result.suffix() {
            Some(word) => Some(suffix::namecase(&word)),
            None => None
        };

        Some(Name {
            first_initial: first_initial,
            given_name: given_name,
            surname: surname,
            middle_names: middle_names,
            middle_initials: middle_initials,
            suffix: suffix,
        })
    }

    pub fn display(&self) -> String {
        match self.given_name {
            Some(ref name) => {
                format!("{} {}", name, self.surname)
            }
            None => {
                format!("{}. {}", self.first_initial, self.surname)
            }
        }
    }
}
