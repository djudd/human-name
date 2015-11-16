use std::ascii::AsciiExt;
use unicode_normalization::UnicodeNormalization;

const VOWELS: [char; 12] = ['a', 'e', 'i', 'o', 'u', 'y', 'A', 'E', 'I', 'O', 'U', 'Y'];

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

pub fn capitalize(word: &str) -> String {
    let mut capitalize_next = true;
    word.chars()
        .filter_map(|c| {
            let result = if capitalize_next {
                c.to_uppercase().next()
            } else {
                c.to_lowercase().next()
            };

            capitalize_next = !c.is_alphanumeric();

            result
        })
        .collect()
}

pub fn is_missing_vowels(word: &str) -> bool {
    word.chars().all(|c| !c.is_alphabetic() || (c.is_ascii() && !VOWELS.contains(&c)))
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

macro_rules! eq_or_starts_with_normalized {
    ($chars_a:expr, $chars_b:expr) => {
        {
            let mut iter_a = $chars_a.nfkd().filter_map( |c|
                if c.is_uppercase() {
                    c.to_lowercase().next()
                } else if c.is_alphabetic() {
                    Some(c)
                } else {
                    None
                }
            );

            let mut iter_b = $chars_b.nfkd().filter_map( |c|
                if c.is_uppercase() {
                    c.to_lowercase().next()
                } else if c.is_alphabetic() {
                    Some(c)
                } else {
                    None
                }
            );

            let mut compared = 0;

            loop {
                let ca = iter_a.next();
                let cb = iter_b.next();

                if ca.is_none() && cb.is_none() {
                    return true;
                } else if ca.is_none() != cb.is_none() {
                    // Only allow containment, vs equality, when contained
                    // string has at least four characters
                    return compared > 3;
                } else if ca != cb {
                    return false;
                }

                compared += 1;
            }
        }
    };
}

pub fn eq_or_ends_with_ignoring_accents_punct_and_case(a: &[String], b: &[String]) -> bool {
    let iter_a = a.iter().flat_map(|w| w.chars()).rev();
    let iter_b = b.iter().flat_map(|w| w.chars()).rev();
    eq_or_starts_with_normalized!(iter_a, iter_b)
}

pub fn eq_or_starts_with_ignoring_accents_punct_and_case(a: &str, b: &str) -> bool {
    eq_or_starts_with_normalized!(a.chars(), b.chars())
}
