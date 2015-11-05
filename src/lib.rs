extern crate itertools;
extern crate unicode_segmentation;
extern crate unicode_normalization;
extern crate rustc_serialize;

#[macro_use]
extern crate lazy_static;

mod utils;
mod suffix;
mod nickname;
mod title;
mod surname;
mod initials;
mod namecase;
mod namelike;

use itertools::Itertools;
use utils::*;
use namecase::namecase;
use unicode_segmentation::UnicodeSegmentation;
use std::ascii::AsciiExt;

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
//
// TODO Consider a refactoring that uses split_word_bounds(), and might
// merge nickname removal into the main parse loop
fn name_words_and_surname_index(name: &str, mixed_case: bool) -> (Vec<&str>, usize) {
    let mut words: Vec<&str> = Vec::new();
    let mut surname_index = 0;

    // Strip suffixes and then flip remaining words around (remaining) comma
    for (i, part) in name.split(",").enumerate() {
        if i > 1 {
            // Anything after the second comma is a suffix
            break;
        }
        else if i == 0 || words.is_empty() {
            // We're in the surname part (if the format is "Smith, John"),
            // or the only actual name part (if the format is "John Smith,
            // esq." or just "John Smith")
            for word in part.split_whitespace() {
                if first_alphabetical_char(word).is_none() {
                    continue;
                } else if word.chars().all( |c| !c.is_ascii() ) {
                    // Trust unicode word boundaries, because we've got nothing better
                    words.extend(word.unicode_words());
                } else {
                    // Don't trust unicode word boundaries, because they'll split hyphenated names
                    // and names with apostrophes
                    words.push(word);
                }
            }
        }
        else {
            // We already processed one comma-separated part, which may
            // have been the surname (if this is the given name), or the full
            // name (if this is a suffix)
            let mut given_middle_or_suffix_words: Vec<&str> = Vec::new();
            for word in part.split_whitespace() {
                if first_alphabetical_char(word).is_none() {
                    continue;
                } else if word.chars().all( |c| !c.is_ascii() ) {
                    // Trust unicode word boundaries, because we've got nothing better
                    given_middle_or_suffix_words.extend(word.unicode_words());
                } else {
                    // Don't trust unicode word boundaries, because they'll split hyphenated names
                    // and names with apostrophes
                    given_middle_or_suffix_words.push(word);
                }
            }

            while !given_middle_or_suffix_words.is_empty() {
                let word = given_middle_or_suffix_words.pop().unwrap();

                let is_suffix = suffix::is_suffix(word);

                // Preserve parseability: don't strip an apparent suffix
                // that might still be part of the name, if it would take
                // the name below 2 words
                let keep_it_anyway =
                    is_suffix &&
                    given_middle_or_suffix_words.len() + words.len() < 2 &&
                    (initials::is_initials(word, mixed_case) || namelike::is_name(word, false));

                if !is_suffix || keep_it_anyway {
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
            if !suffix::is_suffix(word) {
                break
            }

            // Preserve parseability: don't strip an apparent suffix
            // that might still be part of the name, if it would take
            // the name below 2 words
            let keep_it_anyway = words.len() <= 2 &&
                (initials::is_initials(word, mixed_case) || namelike::is_name(word, true));
            if keep_it_anyway {
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
                let word = words[0];

                // Preserve parseability: don't strip an apparent title part
                // that might still be part of the name, if it would take
                // the name below 2 words
                //
                // However, only allow initials, not namelike strings; nothing
                // we'll recognize as a title is a likely given name
                let keep_it_anyway = words.len() <= 2 &&
                    initials::is_initials(word, mixed_case);

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
        surname_index = surname::find_surname_index(&words, mixed_case);
    }

    (words, surname_index)
}


impl Name {
    pub fn parse(name: &str) -> Option<Name> {
        if first_alphabetical_char(name).is_none() {
            return None
        }

        let mixed_case = is_mixed_case(name);
        let name = nickname::strip_nickname(name);
        let (words, surname_index) = name_words_and_surname_index(&name, mixed_case);

        if words.len() < 2 {
            // We need at least a first and last name, or we can't tell which we have
            return None;
        }

        let first_initial = first_alphabetical_char(words[0])
            .unwrap()
            .to_uppercase()
            .next()
            .unwrap();

        // TODO Benchmark special casing 2-word-remaining names to avoid initializing vectors

        // Take the remaining words, and strip out the initials (if present;
        // only allow one block of initials) and the first name (if present),
        // and whatever's left are the middle names
        let mut given_name = None;
        let mut middle_names: Vec<&str> = Vec::new();
        let mut middle_initials = String::new();

        if words[surname_index..].iter().all( |w| !namelike::is_name(w, true) ) {
            return None;
        }

        for (i, word) in words[0..surname_index].iter().enumerate() {
            if initials::is_initials(word, mixed_case) {
                let start = if i == 0 { 1 } else { 0 };
                if word.len() > start {
                    middle_initials.extend(
                        word
                            .chars()
                            .skip(start)
                            .filter( |c| c.is_alphabetic() )
                            .filter_map( |w| w.to_uppercase().next() ));
                }
            } else if given_name.is_none() {
                given_name = Some(word.to_string());
            } else {
                middle_names.push(word);
                middle_initials.push(
                    first_alphabetical_char(word)
                        .unwrap()
                        .to_uppercase()
                        .next()
                        .unwrap());
            }
        }

        let given_name =
            if given_name.is_none() || mixed_case {
                given_name
            } else {
                Some(namecase(&given_name.unwrap(), false))
            };

        let middle_names =
            if middle_names.is_empty() {
                None
            } else if mixed_case {
                Some(middle_names.join(" "))
            } else {
                Some(middle_names.iter().map( |w| namecase(w, false) ).join(" "))
            };

        let middle_initials =
            if middle_initials.is_empty() {
                None
            } else {
                Some(middle_initials)
            };

        let surname =
            if mixed_case {
                words[surname_index..].iter().join(" ")
            } else {
                let last_surname_word_ix = words.len() - surname_index - 1;
                words[surname_index..]
                    .iter()
                    .enumerate()
                    .map( |(i, w)| namecase(w, i < last_surname_word_ix) )
                    .join(" ")
            };

        Some(Name {
            first_initial: first_initial,
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
