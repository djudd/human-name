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

// TODO Should these be Cows?
#[derive(RustcDecodable, RustcEncodable)]
pub struct Name {
  pub given_name: Option<String>,
  pub surname: String,
  pub middle_names: Option<String>,
  pub first_initial: char,
  pub middle_initials: Option<String>,
}

// Strip suffixes and titles, and find the start of the surname.
// These tasks are intrinsically connected because commas may indicate
// either a sort-ordered surname ("Smith, John") or a suffix ("John Smith, esq")
//
// This is where the meat of the parsing takes place
fn name_words_and_surname_index(name: &str, mixed_case: bool) -> (Vec<NamePart>, usize) {
    let mut words: Vec<NamePart> = Vec::new();
    let mut surname_index = 0;

    // Strip suffixes and then flip remaining words around (remaining) comma
    for (i, part) in name.split(",").enumerate() {
        if i > 1 && words.len() > 1 {
            // Anything after the second comma is a suffix
            break;
        }
        else if i == 0 || words.is_empty() {
            // We're in the surname part (if the format is "Smith, John"),
            // or the only actual name part (if the format is "John Smith,
            // esq." or just "John Smith")
            words.extend(NamePart::all_from_text(part, mixed_case, Location::End));
        }
        else {
            // We already processed one comma-separated part, which may
            // have been the surname (if this is the given name), or the full
            // name (if this is a suffix)
            let mut given_middle_or_suffix_words: Vec<NamePart> = NamePart::all_from_text(part, mixed_case, Location::Start).collect();

            while !given_middle_or_suffix_words.is_empty() {
                let word = given_middle_or_suffix_words.pop().unwrap();

                // Preserve parseability: don't strip an apparent suffix
                // that might still be part of the name, if it would take
                // the name below 2 words
                let keep_it_anyway =
                    given_middle_or_suffix_words.len() + words.len() < 2 &&
                    (word.is_initials() || word.is_namelike());

                if keep_it_anyway || !suffix::is_suffix(&word) {
                    surname_index += 1;
                    words.insert(0, word)
                }
            }
        }
    }

    // Strip non-comma-separated suffixes (e.g. "John Smith Jr.")
    while !words.is_empty() {
        {
            let word = words.last().unwrap();

            // Preserve parseability: don't strip an apparent suffix
            // that might still be part of the name, if it would take
            // the name below 2 words
            //
            // However, only allow names, not initials; initials should only
            // be at the end of the input if they are comma-separated
            let keep_it_anyway = words.len() <= 2 && word.is_namelike();
            if keep_it_anyway || !suffix::is_suffix(&word) {
                break
            }
        }

        words.pop();
    }

    if words.is_empty() {
        return (words, 0);
    }

    // Check for title as prefix (e.g. "Dr. John Smith" or "Right Hon. John Smith")
    let mut prefix_len = words.len() - 1;
    while prefix_len > 0 {
        if title::is_title(&words[0..prefix_len]) {
            for _ in 0..prefix_len {
                // Preserve parseability: don't strip an apparent title part
                // that might still be part of the name, if it would take
                // the name below 2 words
                //
                // However, only allow initials, not namelike strings; nothing
                // we'll recognize as a title is a likely given name
                let keep_it_anyway = words.len() <= 2 && words[0].is_initials();

                if !keep_it_anyway {
                    words.remove(0);
                    if surname_index > 0 {
                        surname_index -= 1;
                    }
                }
            }

            break
        }
        prefix_len -= 1;
    }

    if words.len() > 1 && (surname_index <= 0 || surname_index >= words.len()) {
        // We didn't get the surname from the formatting (e.g. "Smith, John"),
        // so we have to guess it
        surname_index = surname::find_surname_index(&words);
    }

    (words, surname_index)
}


impl Name {
    pub fn parse(name: &str) -> Option<Name> {
        if !name.chars().any(char::is_alphabetic) {
            return None
        }

        let mixed_case = is_mixed_case(name);
        let name = nickname::strip_nickname(name);
        let (words, surname_index) = name_words_and_surname_index(&name, mixed_case);

        if words.len() < 2 {
            // We need at least a first and last name, or we can't tell which we have
            return None;
        }
        else if words[surname_index..].iter().all( |w| !w.is_namelike() ) {
            // Looks like a bad parse, since the surname doesn't make sense
            return None;
        }

        // Take the remaining words, and strip out the initials (if present;
        // only allow one block of initials) and the first name (if present),
        // and whatever's left are the middle names
        let mut given_name: Option<String> = None;
        let mut middle_names: Vec<&str> = Vec::new();
        let mut middle_initials = String::new();

        for (i, word) in words[0..surname_index].iter().enumerate() {
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

        let surname = words[surname_index..]
            .iter()
            .map( |w| &*w.namecased )
            .join(" ");

        Some(Name {
            first_initial: words[0].initial(),
            given_name: given_name,
            surname: surname,
            middle_names: middle_names,
            middle_initials: middle_initials,
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
