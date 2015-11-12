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

#[derive(RustcDecodable, RustcEncodable)]
pub struct Name {
  pub given_name: Option<String>,
  pub surname: String,
  pub middle_names: Option<String>,
  pub first_initial: char,
  pub middle_initials: Option<String>,
  pub suffix: Option<String>,
}

fn strip_postfixes_and_find_suffix<'a>(words: &mut Vec<NamePart<'a>>, include_first_word: bool, postfixes: &mut Vec<NamePart<'a>>) { // -> Option<NamePart> {
    let postfix_position = words.iter().enumerate().position( |(i, word)|
        if i == 0 && !include_first_word {
            false
        }
        else {
            suffix::is_suffix(&word) || title::is_postfix_title(&word)
        }
    );

    if postfix_position.is_some() {
        let range = postfix_position.unwrap()..words.len();
        postfixes.extend(words.drain(range));
    }
}

// Strip suffixes and titles, and find the start of the surname.
// These tasks are intrinsically connected because commas may indicate
// either a sort-ordered surname ("Smith, John") or a suffix ("John Smith, esq")
//
// This is where the meat of the parsing takes place
fn name_words_and_surname_index(name: &str, mixed_case: bool) -> (Vec<NamePart>, usize, Option<NamePart>) {
    let mut words: Vec<NamePart> = Vec::new();
    let mut postfixes: Vec<NamePart> = Vec::new();
    let mut surname_index = 0;
    let mut found_first_name_or_initial = false;

    // Separate comma-separated titles and suffixes, then flip remaining words
    // around remaining comma, if any
    for (i, part) in name.split(",").enumerate() {
        if i == 0 || words.is_empty() {
            // We're in the surname part (if the format is "Smith, John"),
            // or the only actual name part (if the format is "John Smith,
            // esq." or just "John Smith")
            words.extend(NamePart::all_from_text(part, mixed_case, Location::End));
            if words.is_empty() { continue }

            // Check for title as prefix (e.g. "Dr. John Smith" or "Right Hon.
            // John Smith"); finding a prefix title means the next word is a
            // first name or initial (we don't support "Dr. Smith, John")
            if words.len() > 1 {
                let stripped_anything = title::strip_prefix_title(&mut words, true);
                found_first_name_or_initial = stripped_anything;
            }

            // Strip non-comma-separated titles & suffixes (e.g. "John Smith Jr.")
            strip_postfixes_and_find_suffix(&mut words, false, &mut postfixes);

            if !found_first_name_or_initial {
                found_first_name_or_initial = surname::find_surname_index(&*words) > 0;
            }
        }
        else if found_first_name_or_initial {
            // We already found the full name, so this is a comma-separated
            // postfix title or suffix
            postfixes.extend(NamePart::all_from_text(part, mixed_case, Location::End));
        }
        else {
            // We already processed one comma-separated part, which may
            // have been the surname (if this is the given name), or the full
            // name (if this is a suffix or title)
            let mut given_middle_or_postfix_words: Vec<NamePart> =
                NamePart::all_from_text(part, mixed_case, Location::Start).collect();
            if given_middle_or_postfix_words.is_empty() { continue }

            // Check for (unusual) formats like "Smith, Dr. John M."
            if !found_first_name_or_initial && given_middle_or_postfix_words.len() > 1 {
                let stripped_anything = title::strip_prefix_title(&mut given_middle_or_postfix_words, false);
                found_first_name_or_initial = stripped_anything;
            }

            // Check for (more common) formats like "Smith, John" or "Smith, J. M."
            if !found_first_name_or_initial {
                found_first_name_or_initial =
                    given_middle_or_postfix_words.iter().any( |word|
                        !suffix::is_suffix(&word) && !title::is_postfix_title(&word)
                    );
            }

            // Now we've decided: either this is the given name or first initial,
            // in which case we put it in front, or it's a suffix or title
            if found_first_name_or_initial {
                // Check for (unusual) formats like "Smith, John Jr."
                strip_postfixes_and_find_suffix(&mut given_middle_or_postfix_words, true, &mut postfixes);

                let surname_words = words;
                words = given_middle_or_postfix_words;
                surname_index = words.len();
                words.extend(surname_words);
            }
            else {
                postfixes.extend(given_middle_or_postfix_words);
            }
        }
    }

    // If there are two or fewer words, e.g. "JOHN MA", we treat
    // ambiguous strings like "MA" as surnames rather than titles
    // (not initials, which should only be at the end of the input
    // if they are comma-separated, and we already handled that case)
    if words.len() < 2 && !postfixes.is_empty() && postfixes[0].is_namelike() {
        words.push(postfixes.remove(0));
    }

    // Anything trailing that looks like initials is probably a stray suffix
    while !words.is_empty() && words.last().unwrap().is_initials() {
        words.pop();
    }

    if words.len() < 2 {
        // Failed parse
        return (words, 0, None);
    }

    if surname_index <= 0 || surname_index >= words.len() {
        // We didn't get the surname from the formatting (e.g. "Smith, John"),
        // so we have to guess it
        surname_index = surname::find_surname_index(&words[1..]) + 1;
    }

    let mut suffix = None;
    if postfixes.len() > 0 {
        suffix = postfixes.into_iter().find( |word| suffix::is_suffix(&word) );
    }

    (words, surname_index, suffix)
}


impl Name {
    pub fn parse(name: &str) -> Option<Name> {
        if !name.chars().any(char::is_alphabetic) {
            return None
        }

        let mixed_case = is_mixed_case(name);
        let name = nickname::strip_nickname(name);
        let (words, surname_index, suffix) = name_words_and_surname_index(&name, mixed_case);

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

        let suffix = match suffix {
            Some(word) => Some(suffix::namecase(&word)),
            None => None
        };

        Some(Name {
            first_initial: words[0].initial(),
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
