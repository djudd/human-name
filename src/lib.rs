#![feature(drain)]
#![feature(plugin)]
#![plugin(phf_macros)]

extern crate phf;
extern crate itertools;
extern crate unicode_segmentation;
extern crate unicode_normalization;
extern crate rustc_serialize;

#[macro_use]
mod utils;
mod suffix;
mod nickname;
mod title;
mod surname;
mod namecase;
mod namepart;
mod parse;

use std::borrow::Cow;
use std::hash::{Hash, Hasher};
use itertools::Itertools;
use rustc_serialize::json::{self, ToJson, Json};
use unicode_normalization::UnicodeNormalization;
use utils::*;

pub struct Name {
    words: Vec<String>,
    surname_index: usize,
    suffix_index: usize,
    initials: String,
}

impl Name {
    pub fn parse(name: &str) -> Option<Name> {
        if name.len() >= 1000 || !name.chars().any(char::is_alphabetic) {
            return None;
        }

        let mixed_case = is_mixed_case(name);
        let name = nickname::strip_nickname(name);

        let result = parse::parse(&*name, mixed_case);
        if result.is_none() {
            return None;
        }

        let (words, surname_index, suffix_index) = result.unwrap();

        let mut names: Vec<String> = Vec::with_capacity(words.len());
        let mut initials = String::with_capacity(surname_index);
        let mut surname_index_in_names = surname_index;
        let mut suffix_index_in_names = suffix_index;

        for (i, word) in words.into_iter().enumerate() {
            if word.is_initials() && i < surname_index {
                initials.extend(word.word
                                    .chars()
                                    .filter(|c| c.is_alphabetic())
                                    .filter_map(|w| w.to_uppercase().next()));

                surname_index_in_names -= 1;
                suffix_index_in_names -= 1;
            } else if i < surname_index {
                initials.push(word.initial());

                let owned: String = word.namecased.into_owned();
                names.push(owned);
            } else if i < suffix_index {
                let owned: String = word.namecased.into_owned();
                names.push(owned);
            } else {
                names.push(suffix::namecase(&word));
            }
        }

        names.shrink_to_fit();

        Some(Name {
            words: names,
            surname_index: surname_index_in_names,
            suffix_index: suffix_index_in_names,
            initials: initials,
        })
    }

    pub fn first_initial(&self) -> char {
        self.initials.chars().nth(0).unwrap()
    }

    pub fn given_name(&self) -> Option<&str> {
        if self.surname_index > 0 {
            Some(&*self.words[0])
        } else {
            None
        }
    }

    pub fn goes_by_middle_name(&self) -> bool {
        self.given_name().is_some() && !self.given_name().unwrap().starts_with(self.first_initial())
    }

    pub fn initials(&self) -> &str {
        &self.initials
    }

    pub fn middle_names(&self) -> Option<&[String]> {
        if self.surname_index > 1 {
            Some(&self.words[1..self.surname_index])
        } else {
            None
        }
    }

    pub fn middle_name(&self) -> Option<Cow<str>> {
        match self.middle_names() {
            Some(words) => {
                if words.len() == 1 {
                    Some(Cow::Borrowed(&*words[0]))
                } else {
                    Some(Cow::Owned(words.join(" ")))
                }
            }
            None => None,
        }
    }

    pub fn middle_initials(&self) -> Option<&str> {
        match self.initials().char_indices().skip(1).nth(0) {
            Some((i, _)) => Some(&self.initials[i..]),
            None => None,
        }
    }

    pub fn surnames(&self) -> &[String] {
        &self.words[self.surname_index..self.suffix_index]
    }

    pub fn surname(&self) -> Cow<str> {
        if self.surnames().len() > 1 {
            Cow::Owned(self.surnames().join(" "))
        } else {
            Cow::Borrowed(&*self.surnames()[0])
        }
    }

    pub fn suffix(&self) -> Option<&str> {
        if self.words.len() > self.suffix_index {
            Some(&*self.words[self.suffix_index])
        } else {
            None
        }
    }

    pub fn display_short(&self) -> String {
        match self.given_name() {
            Some(ref name) => {
                format!("{} {}", name, self.surname())
            }
            None => {
                format!("{}. {}", self.first_initial(), self.surname())
            }
        }
    }

    fn surname_eq(&self, other: &Name) -> bool {
        let mine = self.surnames().iter().flat_map(|w| w.chars()).rev();
        let theirs = other.surnames().iter().flat_map(|w| w.chars()).rev();
        eq_or_starts_with_ignoring_accents_nonalpha_and_case!(mine, theirs)
    }

    fn given_name_eq(&self, other: &Name) -> bool {
        let mine = self.given_name();
        let theirs = other.given_name();

        mine.is_none() || theirs.is_none() ||
        eq_or_starts_with_ignoring_accents_nonalpha_and_case!(mine.unwrap(), theirs.unwrap())
    }

    fn middle_names_eq(&self, other: &Name) -> bool {
        if self.middle_names().is_none() || other.middle_names().is_none() {
            return true;
        }

        // We know they're equal or one contains the other if we've got this far,
        // so we know the longer string includes the first letters of all the
        // names
        let my_initials = self.middle_initials().unwrap();
        let their_initials = other.middle_initials().unwrap();
        let initials = if my_initials.len() > their_initials.len() {
            my_initials
        } else {
            their_initials
        };

        let mut my_names = self.middle_names().unwrap().iter().peekable();
        let mut their_names = other.middle_names().unwrap().iter().peekable();

        for initial in initials.chars() {
            if my_names.peek().is_none() || their_names.peek().is_none() {
                // None of the names were inconsistent and we already know
                // the initials match
                return true;
            }

            // Only look at a name that corresponds to this initial; if the next
            // name doesn't, it means we only have an initial for this name
            let my_name: Option<&String> = if my_names.peek().unwrap().starts_with(initial) {
                my_names.next()
            } else {
                None
            };
            let their_name: Option<&String> = if their_names.peek().unwrap().starts_with(initial) {
                their_names.next()
            } else {
                None
            };

            // If we have two names for the same initial, require that they
            // match using the same logic as for a given name
            if my_name.is_some() && their_name.is_some() &&
            !eq_or_starts_with_ignoring_accents_nonalpha_and_case!(my_name.unwrap(), their_name.unwrap()) {
                return false;
            }
        }

        true
    }

    fn middle_initials_eq(&self, other: &Name) -> bool {
        let mine = self.middle_initials();
        let theirs = other.middle_initials();

        mine.is_none() || theirs.is_none() || mine.unwrap().contains(theirs.unwrap()) ||
        theirs.unwrap().contains(mine.unwrap())
    }

    fn suffix_eq(&self, other: &Name) -> bool {
        self.suffix().is_none() || other.suffix().is_none() || self.suffix() == other.suffix()
    }
}

// NOTE This is technically an invalid implementation of Eq because it is not
// transitive - "J. Doe" == "Jane Doe", and "J. Doe" == "John Doe", but
// "Jane Doe" != "John Doe". (It is, however, symmetric and reflexive.)
//
// Use with caution!
impl PartialEq for Name {

    // Order matters, both for efficiency (single-initial check is the fastest,
    // coincidentally-identical surnames are less likely than for given names),
    // and for correctness (the middle names check assumes a positive result
    // for the middle initials check)
    #[cfg_attr(rustfmt, rustfmt_skip)]
    fn eq(&self, other: &Name) -> bool {
        self.first_initial() == other.first_initial() &&
        self.surname_eq(other) &&
        self.given_name_eq(other) &&
        self.middle_initials_eq(other) &&
        self.middle_names_eq(other) &&
        self.suffix_eq(other)
    }
}

// We only use the first initial and last four alphabetical characters of the
// surname, because that's all we're guaranteed to use in the equality test.
//
// That still gives us 23.5 bits assuming ASCII characters, which isn't great
// but shouldn't be terrible for small-to-moderate sets
//
// TODO Would be nice to make this configurable
impl Hash for Name {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.first_initial().hash(state);

        let surname_chars = self.surnames().iter().flat_map(|w| w.chars()).rev();
        for c in lowercase_alpha_without_accents!(surname_chars).take(MIN_CHARS_FOR_EQ_BY_CONTAINS) {
            c.hash(state);
        }
    }
}

#[derive(RustcEncodable)]
struct SerializeableName {
    pub given_name: Option<String>,
    pub surname: String,
    pub middle_names: Option<String>,
    pub first_initial: char,
    pub middle_initials: Option<String>,
    pub suffix: Option<String>,
}

impl SerializeableName {
    pub fn new(name: &Name) -> SerializeableName {
        SerializeableName {
            given_name: name.given_name().map(|s| s.to_string()),
            surname: name.surname().to_string(),
            middle_names: name.middle_name().map(|s| s.to_string()),
            first_initial: name.first_initial(),
            middle_initials: name.middle_initials().map(|s| s.to_string()),
            suffix: name.suffix().map(|s| s.to_string()),
        }
    }
}

impl ToJson for Name {
    fn to_json(&self) -> Json {
        Json::String(json::encode(&SerializeableName::new(self)).unwrap())
    }
}
