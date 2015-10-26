extern crate regex;

#[macro_use]
extern crate lazy_static;

mod suffix;
mod nickname;
mod title;

pub struct Name {
  raw: String,
  pub given_name: String,
  pub surname: String,
  pub middle_names: String,
}

impl Name {
    pub fn parse(name: &str) -> Option<Name> {
        let mut words: Vec<&str> = Vec::new();

        // Strip suffixes and nicknames, and then flip remaining words around
        // (remaining) comma (for formats like "Smith, John"; but suffixes may
        // also contain commas, e.g. "John Smith, esq.")
        let mut parts = name.rsplit(',').peekable();
        loop {
            match parts.next() {
                Some(part) => {
                    let is_first_part = parts.peek().is_none();
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

        // We need at least a first and last name, or we can't tell which we have
        if words.len() < 2 {
            return None;
        }

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
