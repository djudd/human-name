use std::ascii::AsciiExt;
use unicode_normalization::char::canonical_combining_class;
use unicode_normalization::UnicodeNormalization;

const VOWELS: &'static str = "aeiouyAEIOUY";
const HYPHENS: &'static str = "-\u{2010}‑‒–—―−－﹘﹣";

pub fn is_mixed_case(s: &str) -> bool {
    let mut has_lowercase = false;
    let mut has_uppercase = false;

    for c in s.chars() {
        if c.is_uppercase() {
            has_uppercase = true;
        };
        if c.is_lowercase() {
            has_lowercase = true;
        };
        if has_lowercase && has_uppercase {
            return true;
        };
    }

    false
}

pub fn is_capitalized(word: &str) -> bool {
    match word.chars().nth(0) {
        Some(c) => {
            if !c.is_uppercase() {
                return false;
            }
        }
        None => {
            return false;
        }
    }

    word.chars().skip(1).all(|c| c.is_lowercase() || !c.is_alphabetic())
}

#[inline]
pub fn is_combining(c: char) -> bool {
    canonical_combining_class(c) > 0
}

#[inline]
pub fn lowercase_if_alpha(c: char) -> Option<char> {
    if c.is_uppercase() {
        c.to_lowercase().next()
    } else if c.is_alphabetic() {
        Some(c)
    } else {
        None
    }
}

pub fn capitalize_and_normalize(word: &str) -> String {
    let mut capitalize_next = true;

    word.chars()
        .filter_map(|c| {
            let result = if HYPHENS.contains(c) {
                Some('-')
            } else if !c.is_alphanumeric() {
                Some(c)
            } else if capitalize_next {
                c.to_uppercase().next()
            } else {
                c.to_lowercase().next()
            };

            capitalize_next = !c.is_alphanumeric() && !is_combining(c);

            result
        })
        .nfkd()
        .collect()
}

pub fn is_missing_vowels(word: &str) -> bool {
    word.chars().all(|c| !c.is_alphabetic() || (c.is_ascii() && !VOWELS.contains(c)))
}

pub fn starts_with_consonant(word: &str) -> bool {
    match word.chars().nth(0) {
        Some(c) => c.is_alphabetic() && c.is_ascii() && (c == 'y' || c == 'Y' || !VOWELS.contains(c)),
        None => false
    }
}

pub fn has_sequential_alphas(word: &str) -> bool {
    let mut iter = word.chars().peekable();
    loop {
        match iter.next() {
            Some(c) => {
                match iter.peek() {
                    Some(nc) => {
                        if c.is_alphabetic() && nc.is_alphabetic() {
                            return true;
                        }
                    }
                    None => {
                        break;
                    }
                }
            }
            None => {
                break;
            }
        }
    }

    false
}

#[macro_export]
macro_rules! have_prefix_match {
    ($a:expr, $b:expr) => { {
        let mut chars_a = $a.chars().filter_map(lowercase_if_alpha);
        let mut chars_b = $b.chars().filter_map(lowercase_if_alpha);
        let result;

        loop {
            let a = chars_a.next();
            let b = chars_b.next();

            if a.is_none() || b.is_none() {
                result = true;
                break;
            } else if a != b {
                result = false;
                break;
            }
        }

        result
    } }
}

#[macro_export]
macro_rules! is_suffix_of {
    ($needle:expr, $haystack:expr) => { {
        let mut n_chars= $needle.chars().rev().filter_map(lowercase_if_alpha);
        let mut h_chars = $haystack.chars().rev().filter_map(lowercase_if_alpha);
        let result;

        loop {
            let n = n_chars.next();
            let h = h_chars.next();

            if n.is_none() {
                result = true;
                break;
            } else if n != h {
                result = false;
                break;
            }
        }

        result
    } }
}
