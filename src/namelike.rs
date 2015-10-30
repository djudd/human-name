use super::utils;
use std::collections::HashSet;

lazy_static! {
    static ref VOWELLESS_SURNAMES: HashSet<&'static str> = {
        let s: HashSet<&'static str> = [
            "ng",
        ].iter().cloned().collect();
        s
    };
}

fn is_vowelless_surname(word: &str) -> bool {
    let key = word.to_lowercase();
    VOWELLESS_SURNAMES.contains(&*key)
}

pub fn is_name(word: &str, surname: bool) -> bool {
    if word.len() < 2 {
        false
    }
    else if word.ends_with('.') {
        false
    }
    else if word.chars().filter( |c| !c.is_alphabetic() ).count() > 2 {
        false
    }
    else if utils::is_missing_vowels(word) && (!surname || !is_vowelless_surname(word)) {
        false
    }
    else {
        true
    }
}
