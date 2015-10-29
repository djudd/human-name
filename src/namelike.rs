use super::utils;

pub fn is_name(word: &str) -> bool {
    if word.len() < 2 {
        false
    }
    else if word.ends_with('.') {
        false
    }
    else if word.chars().filter( |c| !c.is_alphabetic() ).count() > 2 {
        false
    }
    else if utils::is_missing_vowels(word) {
        false
    }
    else {
        true
    }
}
