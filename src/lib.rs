use std::option;

pub struct Name {
  raw: String,
  pub given_name: String,
  pub surname: String,
  pub middle_initials: String,
}

impl Name {
    pub fn new(name: &str) -> Option<Name> {
        let words: Vec<&str> = name.split_whitespace().collect();
        if words.len() < 2 {
            return None;
        }

        let parsed = Name {
            raw: name.to_string(),
            given_name: words[0].to_string(),
            surname: words[1..].join(" "),
            middle_initials: "".to_string(), // TODO
        };
        return Some(parsed);
    }

    pub fn display(&self) -> String {
        return format!("{} {}", self.given_name, self.surname);
    }
}
