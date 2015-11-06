use std::ascii::AsciiExt;

const VOWELS: [char; 12] = ['a','e','i','o','u','y','A','E','I','O','U','Y'];

pub fn first_alphabetical_char(s: &str) -> Option<char> {
    s.chars().find( |c| c.is_alphabetic() )
}

pub fn is_mixed_case(s: &str) -> bool {
    let mut has_lowercase = false;
    let mut has_uppercase = false;

    for c in s.chars() {
        if c.is_uppercase() { has_uppercase = true; };
        if c.is_lowercase() { has_lowercase = true; };
        if has_lowercase && has_uppercase {
            return true
        };
    }

    false
}

pub fn capitalize(word: &str) -> String {
    let mut capitalize_next = true;
    word.chars().filter_map( |c| {
        let result = if capitalize_next {
            c.to_uppercase().next()
        } else {
            c.to_lowercase().next()
        };

        capitalize_next = !c.is_alphanumeric();

        result
    }).collect()
}

pub fn is_missing_vowels(word: &str) -> bool {
    word.chars().all(|c| !c.is_alphabetic() || (c.is_ascii() && !VOWELS.contains(&c)))
}

// Fairly lax check, which allows initial or trailing periods, or neither,
// and allows double periods (as likely to be typos as to indicate something
// other than initials), but forbids two neighboring non-periods.
pub fn is_period_separated(word: &str) -> bool {
    let mut chars = word.chars().peekable();

    loop {
        match chars.next() {
            Some(c) => {
                match chars.peek() {
                    Some(nc) => {
                        if (c != '.') && (*nc != '.') {
                            return false;
                        }
                    }
                    None => { break }
                }
            }
            None => { break }
        }
    }

    true
}
