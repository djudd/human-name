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

use utils::*;
use itertools::Itertools;

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
        if name.len() >= 1000 || !name.chars().any(char::is_alphabetic) {
            return None
        }

        let mixed_case = is_mixed_case(name);
        let name = nickname::strip_nickname(name);

        let result = parse::parse(&*name, mixed_case);
        if result.is_none() {
            return None
        }

        let (words, surname_index, suffix_index) = result.unwrap();

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

        let surname = words[surname_index..suffix_index]
            .iter()
            .map( |w| &*w.namecased )
            .join(" ");

        let suffix = match words[suffix_index..].iter().nth(0) {
            Some(word) => Some(suffix::namecase(&word)),
            None => None,
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
