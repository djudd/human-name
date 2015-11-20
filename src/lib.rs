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
mod parse;
mod comparison;
mod serialization;

use std::borrow::Cow;
use std::hash::{Hash, Hasher};
use itertools::Itertools;
use utils::{is_mixed_case, lowercase_if_alpha};

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
                initials.extend(word.namecased.chars()
                    .filter(|c| c.is_alphabetic())
                    .flat_map(|c| c.to_uppercase()));

                surname_index_in_names -= 1;
                suffix_index_in_names -= 1;
            } else if i < surname_index {
                initials.extend(word.namecased.split('-')
                    .filter_map(|w| w.chars().find(|c| c.is_alphabetic())));

                names.push(word.namecased.into_owned());
            } else if i < suffix_index {
                names.push(word.namecased.into_owned());
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
}

// NOTE This is technically an invalid implementation of PartialEq because it is
// not transitive - "J. Doe" == "Jane Doe", and "J. Doe" == "John Doe", but
// "Jane Doe" != "John Doe". (It is, however, symmetric and reflexive.)
//
// Use with caution!
impl Eq for Name {}
impl PartialEq for Name {
    fn eq(&self, other: &Name) -> bool {
        self.consistent_with(other)
    }
}

// NOTE This hash function is prone to collisions!
//
// We can only use the last four alphabetical characters of the surname, because
// that's all we're guaranteed to use in the equality test. That means if names
// are ASCII, we only have 19 bits of variability.
//
// That means if you are working with a lot of names and you expect surnames
// to be similar or identical, you might be better off avoiding hash-based
// datastructures (or using a custom hash and alternate equality test).
//
// We can't use more characters of the surname because we treat names as equal
// when one surname ends with the other and the smaller is at least four
// characters, to catch cases like "Iria Gayo" == "Iria del Río Gayo".
//
// We can't use the first initial because we might ignore it if someone goes
// by a middle name, to catch cases like "H. Manuel Alperin" == "Manuel Alperin."
impl Hash for Name {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let surname_chars = self.surnames().iter().flat_map(|w| w.chars()).rev();
        for c in surname_chars.filter_map(lowercase_if_alpha).take(comparison::MIN_SURNAME_CHAR_MATCH) {
            c.hash(state);
        }
    }
}


