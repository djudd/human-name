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
use unicode_segmentation::UnicodeSegmentation;
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
                initials.extend(word.word.chars()
                    .filter(|c| c.is_alphabetic())
                    .filter_map(|c| c.to_uppercase().next()));

                surname_index_in_names -= 1;
                suffix_index_in_names -= 1;
            } else if i < surname_index {
                initials.extend(word.namecased.split('-')
                    .filter_map(|w| w.chars().find(|c| c.is_alphabetic()))
                    .filter_map(|c| c.to_uppercase().next()));

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

    fn initials_consistent(&self, other: &Name) -> bool {
        if self.goes_by_middle_name() == other.goes_by_middle_name() {
            // Normal case: neither goes by a middle name (as far as we know)
            // or both do, so we require the first initial to be the same
            // and one set of middle initials to equal or contain the other
            if self.first_initial() != other.first_initial() {
                return false;
            }

            let my_middle = self.middle_initials();
            let their_middle = other.middle_initials();

            my_middle.is_none() || their_middle.is_none() ||
                my_middle.unwrap().contains(their_middle.unwrap()) ||
                their_middle.unwrap().contains(my_middle.unwrap())
        } else if self.goes_by_middle_name() {
            // Otherwise, we stop requiring the first initial to be the same,
            // because it might have been included in one context and omitted
            // elsewhere, but instead we assume that the name with the initial
            // prior to the middle name includes a full set of initials, so
            // we require the other version to be equal or included
            self.initials().contains(other.initials())
        } else {
            other.initials().contains(self.initials())
        }
    }

    fn given_and_middle_names_consistent(&self, other: &Name) -> bool {
        if self.surname_index == 0 || other.surname_index == 0 {
            return true;
        }

        // We know the initials are equal or one is a superset of the other if
        // we've got this far, so we know the longer string includes the first
        // letters of all the given & middle names
        let initials = if self.initials().len() > other.initials().len() {
            self.initials()
        } else {
            other.initials()
        };

        let mut my_names = self
            .words[0..self.surname_index]
            .iter()
            .flat_map(|w|w.split('-'))
            .peekable();

        let mut their_names = other
            .words[0..other.surname_index]
            .iter()
            .flat_map(|w|w.split('-'))
            .peekable();

        for initial in initials.chars() {
            if my_names.peek().is_none() || their_names.peek().is_none() {
                // None of the name-words were inconsistent
                return true;
            }

            // Only look at a name-word that corresponds to this initial; if the
            // next word doesn't, it means we only have an initial for this word
            // in a given version of the name
            let my_name: Option<&str> = if my_names.peek().unwrap().starts_with(initial) {
                my_names.next()
            } else {
                None
            };

            let their_name: Option<&str> = if their_names.peek().unwrap().starts_with(initial) {
                their_names.next()
            } else {
                None
            };

            // If we have two names for the same initial, require that they are
            // equal, ignoring case, accents, and non-alphabetic chars, or
            // that one starts with the other
            if my_name.is_some() && their_name.is_some() &&
            !eq_or_starts_with_ignoring_accents_nonalpha_and_case!(my_name.unwrap().chars(), their_name.unwrap().chars()) {
                return false;
            }
        }

        true
    }

    fn surname_consistent(&self, other: &Name) -> bool {
        let mut my_words = self
            .surnames()
            .iter()
            .flat_map(|w| w.unicode_words() )
            .rev();

        let mut their_words = other
            .surnames()
            .iter()
            .flat_map(|w| w.unicode_words() )
            .rev();

        let mut my_word = my_words.next();
        let mut their_word = their_words.next();
        let mut matching_chars = 0;

        // Require either an exact match (ignoring case etc), or a partial match
        // of len >= MIN_CHARS_FOR_EQ_BY_CONTAINS and breaking on a word boundary
        loop {
            // No words remaining for some surname - that's ok if it's true of
            // both, or if the components that match are long enough
            if my_word.is_none() || their_word.is_none() {
                return my_word == their_word || matching_chars >= MIN_CHARS_FOR_EQ_BY_CONTAINS;
            }

            macro_rules! reverse_lowercase_alpha_chars {
                ($word:expr) => {
                    $word.unwrap().chars().rev().filter_map(lowercase_if_alpha)
                }
            }

            let mut my_chars = reverse_lowercase_alpha_chars!(my_word);
            let mut their_chars = reverse_lowercase_alpha_chars!(their_word);

            let mut my_char = my_chars.next();
            let mut their_char = their_chars.next();

            loop {
                if my_char.is_none() && their_char.is_none() {
                    // The words matched exactly, try the next word
                    my_word = my_words.next();
                    their_word = their_words.next();
                    break;
                } else if my_char.is_none() {
                    // My word is a suffix of their word, check my next word
                    // against the rest of their word
                    my_word = my_words.next();
                    if my_word.is_none() {
                        // There is no next word, so this is a suffix-only match,
                        // and we don't allow those
                        return false;
                    } else {
                        // Continue the inner loop but incrementing through my
                        // next word
                        my_chars = reverse_lowercase_alpha_chars!(my_word);
                        my_char = my_chars.next();
                    }
                } else if their_char.is_none() {
                    // Their word is a suffix of my word, check their next word
                    // against the rest of my_words
                    their_word = their_words.next();
                    if their_word.is_none() {
                        // There is no next word, so this is a suffix-only match,
                        // and we don't allow those
                        return false;
                    } else {
                        // Continue the inner loop but incrementing through their
                        // next word
                        their_chars = reverse_lowercase_alpha_chars!(their_word);
                        their_char = their_chars.next();
                    }
                } else if my_char != their_char {
                    // We found a conflict and can short-circuit
                    return false;
                } else {
                    // Characters matched, continue the inner loop
                    matching_chars += 1;
                    my_char = my_chars.next();
                    their_char = their_chars.next();
                }
            }
        }
    }

    fn suffix_consistent(&self, other: &Name) -> bool {
        self.suffix().is_none() || other.suffix().is_none() || self.suffix() == other.suffix()
    }
}

// NOTE This is technically an invalid implementation of PartialEq because it is
// not transitive - "J. Doe" == "Jane Doe", and "J. Doe" == "John Doe", but
// "Jane Doe" != "John Doe". (It is, however, symmetric and reflexive.)
//
// Use with caution!
impl Eq for Name {}
impl PartialEq for Name {

    // Order matters, both for efficiency (initials check is the fastest,
    // coincidentally-identical surnames are less likely than for given names),
    // and for correctness (the given/middle names check assumes a positive result
    // for the middle initials check)
    #[cfg_attr(rustfmt, rustfmt_skip)]
    fn eq(&self, other: &Name) -> bool {
        self.initials_consistent(other) &&
        self.surname_consistent(other) &&
        self.given_and_middle_names_consistent(other) &&
        self.suffix_consistent(other)
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
        for c in surname_chars.filter_map(lowercase_if_alpha).take(MIN_CHARS_FOR_EQ_BY_CONTAINS) {
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
