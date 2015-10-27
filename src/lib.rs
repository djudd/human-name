extern crate regex;

#[macro_use]
extern crate lazy_static;

mod suffix;
mod nickname;
mod title;
mod surname;
mod initials;

pub struct Name {
  pub given_name: Option<String>,
  pub surname: String,
  pub middle_names: Option<String>,
  pub first_initial: char,
  pub middle_initials: Option<String>,
}

fn first_alphabetical_char(s: &str) -> Option<char> {
    s.chars().find( |c| c.is_alphabetic() )
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
                            if !first_alphabetical_char(word).is_none() && !nickname::is_nickname(word) {
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

        if words.len() < 2 {
            // We need at least a first and last name, or we can't tell which we have
            return None;
        }

        // TODO Special case 2-word-remaining names to avoid initializing vectors

        if surname_index <= 0 || surname_index >= words.len() {
            // We didn't get the surname from the formatting (e.g. "Smith, John"),
            // so we have to guess it
            surname_index = surname::find_surname_index(&words);
        }

        let has_lowercase = words.iter().any( |w| w.chars().any( |c| c.is_lowercase() ) );
        let has_uppercase = words.iter().any( |w| w.chars().any( |c| c.is_uppercase() ) );
        let mixed_case = has_lowercase && has_uppercase;

        let first_initial = first_alphabetical_char(words[0])
            .unwrap()
            .to_uppercase()
            .next()
            .unwrap();

        // Take the remaining words, and strip out the initials (if present; 
        // only allow one block of initials) and the first name (if present),
        // and whatever's left are the middle names
        let mut middle_initials: String = "".to_string();
        let mut given_name = None;
        let mut middle_names: Vec<&str> = Vec::new();

        for (i, word) in words[0..surname_index].iter().enumerate() {
            if initials::is_initials(word, mixed_case) {
                let start = if i == 0 { 1 } else { 0 };
                if word.len() > start {
                    middle_initials.extend(
                        word[start..]
                            .chars()
                            .filter( |c| c.is_alphabetic() )
                            .filter_map( |w| w.to_uppercase().next() ));
                }
            } else if given_name.is_none() {
                given_name = Some(word.to_string());
            } else {
                middle_names.push(word);
                middle_initials.push(first_alphabetical_char(word).unwrap());
            }
        }

        let parsed = Name {
            first_initial: first_initial,
            given_name: given_name,
            surname: words[surname_index..].join(" "),
            middle_names: if middle_names.is_empty() { None } else { Some(middle_names.join(" ")) },
            middle_initials: if middle_initials.is_empty() { None } else { Some(middle_initials) },
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
