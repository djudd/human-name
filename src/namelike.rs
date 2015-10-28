use std::ascii::AsciiExt;
use super::initials;

const VOWELS: [char; 12] = ['a','e','i','o','u','y','A','E','I','O','U','Y'];

pub fn is_unlikely_name(word: &str) -> bool {
    word.chars().all(|c| !c.is_alphabetic() || (c.is_ascii() && !VOWELS.contains(&c)))
}

// TODO Refactor: as-is, is_unlikely_name could be called twice
pub fn may_be_name_or_initials(word: &str, use_capitalization: bool) -> bool {
    initials::is_initials(word, use_capitalization) || !is_unlikely_name(word)
}
