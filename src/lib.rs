extern crate regex;

#[macro_use]
extern crate lazy_static;

mod suffix;

use std::option;
use regex::Regex;

pub struct Name {
  raw: String,
  pub given_name: String,
  pub surname: String,
  pub middle_names: String,
}

fn is_nickname(word: &str) -> bool {
    // TODO
    false
}

impl Name {
    pub fn parse(name: &str) -> Option<Name> {
        let mut words: Vec<&str> = Vec::new();

        // Strip suffixes and nicknames, and then flip remaining words around
        // (remaining) comma (for formats like "Smith, John"; but suffixes may
        // also contain commas, e.g. "John Smith, esq.")
        for part in name.rsplit(',') {
            // TODO Skip suffix check for first part
            // TODO Skip nickname check for first & last words
            if !suffix::is_suffix(part) {
                for word in part.split_whitespace() {
                    if !is_nickname(word) {
                        words.push(word);
                    }
                }
            }
        }

        // We need at least a first and last name, or we can't tell which we have
        if words.len() < 2 {
            return None;
        }

        // TODO Check if last word is suffix

        let parsed = Name {
            raw: name.to_string(),
            given_name: words[0].to_string(),
            surname: words[1..].join(" "),
            middle_names: "".to_string(), // TODO
        };
        return Some(parsed);
    }

    pub fn display(&self) -> String {
        return format!("{} {}", self.given_name, self.surname);
    }

}
