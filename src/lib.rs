extern crate regex;

#[macro_use]
extern crate lazy_static;

mod suffix;
mod nickname;
mod title;
mod surname;

pub struct Name {
  pub given_name: Option<String>,
  pub surname: String,
  pub middle_names: Option<String>,
  pub first_initial: char,
  pub middle_initials: Option<String>,
}

fn first_char(s: &str) -> char {
    s.chars().nth(0).unwrap()
}

impl Name {
    pub fn parse(name: &str) -> Option<Name> {
        let mut words: Vec<&str> = Vec::new();
        let mut surname_index = 0;

        // Strip suffixes and nicknames, and then flip remaining words around
        // (remaining) comma (for formats like "Smith, John"; but suffixes may
        // also contain commas, e.g. "John Smith, esq.")
        let mut parts = name.rsplit(",").peekable();
        loop {
            match parts.next() {
                Some(part) => {
                    let is_first_part = parts.peek().is_none();

                    // If we decided any words after a comma aren't a suffix,
                    // that means the part before, which we're about to process,
                    // is the surname
                    if is_first_part && !words.is_empty() {
                        surname_index = words.len();
                    }

                    if is_first_part || !suffix::is_suffix(part) {
                        for word in part.split_whitespace() {
                            if !nickname::is_nickname(word) {
                                words.push(word);
                            }
                        }
                    }
                }
                None => { break }
            }
        }
    
        // Check for non-comma-separated suffix
        if words.len() > 1 && suffix::is_suffix(words.last().unwrap()) {
            words.pop();
        }

        // Check for title as prefix
        let mut prefix_len = words.len() - 1;
        while prefix_len > 0 {
            if title::is_title(&words[0..prefix_len]) {
                for _ in 0..prefix_len {
                    words.remove(0);
                }
                break;
            }
            prefix_len -= 1;
        }

        let words_len = words.len();
        if words_len < 2 {
            // We need at least a first and last name, or we can't tell which we have
            return None;
        }

        if surname_index <= 0 || surname_index >= words_len {
            // We didn't get the surname from the formatting (e.g. "Smith, John"),
            // so we have to guess it
            surname_index = surname::find_surname_index(&words);
        }

        let has_middle_names = words_len - surname_index > 1;
        let parsed = Name {
            first_initial: first_char(words[0]),
            given_name: Some(words[0].to_string()),
            surname: words[surname_index..].join(" "),
            middle_names: if has_middle_names {
                Some(words[1..surname_index].join(" "))
            } else { None },
            middle_initials: if has_middle_names {
                Some(words[1..surname_index].iter().map(|w| first_char(w)).collect())
            } else { None },
        };
        return Some(parsed);
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
