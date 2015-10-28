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

fn requires_capitalization_afterwards(c: char) -> bool {
    // TODO character class rather than hardcoded apostrophe
    !c.is_alphanumeric() && c != '\''
}

pub fn capitalize(word: &str) -> String {
    let mut capitalize_next = true;
    word.chars().filter_map( |c|
        if capitalize_next {
            capitalize_next = requires_capitalization_afterwards(c);
            c.to_uppercase().next()
        } else {
            capitalize_next = requires_capitalization_afterwards(c);
            c.to_lowercase().next()
        }
    ).collect()
}
