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
}

fn strip_prefix_title(words: &mut Vec<NamePart>, try_to_keep_two_words: bool) -> bool {
    let mut prefix_len = words.len() - 1;
    while prefix_len > 0 {
        let found_prefix = {
            let next_word = &words[prefix_len];
            if try_to_keep_two_words && words.len() - prefix_len <= 1 && words[prefix_len-1].is_initials() {
                // If there is only one word after the prefix, e.g. "DR SMITH",
                // given prefix of "DR", we treat ambiguous strings like "DR"
                // as more likely to be initials than a title (there are no
                // similarly ambiguous given names among our title word list)
                false
            }
            else {
                (next_word.is_namelike() || next_word.is_initials()) &&
                    title::is_prefix_title(&words[0..prefix_len])
            }
        };

        if found_prefix {
            words.drain(0..prefix_len);
            return true;
        }

        prefix_len -= 1;
    }

    false
}

// Strip suffixes and titles, and find the start of the surname.
// These tasks are intrinsically connected because commas may indicate
// either a sort-ordered surname ("Smith, John") or a suffix ("John Smith, esq")
//
// This is where the meat of the parsing takes place
fn name_words_and_surname_index(name: &str, mixed_case: bool) -> (Vec<NamePart>, usize) {
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
                found_first_name_or_initial = strip_prefix_title(&mut words, true);
            }

            // Strip non-comma-separated titles & suffixes (e.g. "John Smith Jr.")
            let postfix_position = words.iter().skip(1).position( |word|
                suffix::is_suffix(&word) || title::is_postfix_title(&word)
            );
            if postfix_position.is_some() {
                let range = (postfix_position.unwrap() + 1)..words.len(); // Off-by-one since we skipped the first word
                postfixes.extend(words.drain(range));
            }

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
                found_first_name_or_initial = strip_prefix_title(&mut given_middle_or_postfix_words, false);
            }

            // Check for (more common) formats like "Smith, John" or "Smith, J. M."
            if !found_first_name_or_initial {
                let word = given_middle_or_postfix_words.first().unwrap();
                if word.is_namelike() {
                    found_first_name_or_initial = true;
                } else if word.is_initials() {
                    // Handle a special case like "Ben Smith, II" or "John E Smith,
                    // III", where we might have mistakenly classified the whole
                    // first comma-separated part as a surname (due to the "ben"
                    // prefix and "e" conjunction rules respectively)
                    found_first_name_or_initial = words.len() == 1 ||
                        given_middle_or_postfix_words.iter().any( |word|
                            !suffix::is_suffix(word) && !title::is_postfix_title(word)
                        );
                }
            }

            // Now we've decided: either this is the given name or first initial,
            // in which case we put it in front, or it's a suffix or title
            if found_first_name_or_initial {
                // Check for (unusual) formats like "Smith, Dr. John Jr." (but
                // only look for suffixes or abbreviations, anything title-like
                // that might be a name or initials is more likely the latter)
                let postfix_position = given_middle_or_postfix_words.iter().position( |word|
                    word.is_abbreviation() || suffix::is_suffix(&word)
                );

                if postfix_position.is_some() {
                    let range = postfix_position.unwrap()..given_middle_or_postfix_words.len();
                    postfixes.extend(given_middle_or_postfix_words.drain(range));
                }

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

    if words.len() < 2 {
        // Failed parse
        return (words, 0);
    }

    if surname_index <= 0 || surname_index >= words.len() {
        // We didn't get the surname from the formatting (e.g. "Smith, John"),
        // so we have to guess it
        surname_index = surname::find_surname_index(&words[1..]) + 1;
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
