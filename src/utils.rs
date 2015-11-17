use std::ascii::AsciiExt;
use unicode_normalization::char::canonical_combining_class;
use unicode_normalization::UnicodeNormalization;

pub const MIN_CHARS_FOR_EQ_BY_CONTAINS: usize = 4;
const VOWELS: [char; 12] = ['a', 'e', 'i', 'o', 'u', 'y', 'A', 'E', 'I', 'O', 'U', 'Y'];
const HYPHENS: [char; 11] = ['-', '\u{2010}', '‑', '‒','–', '—', '―', '−','－','﹘','﹣'];

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

pub fn is_capitalized_and_normalized(word: &str) -> bool {
    match word.chars().nth(0) {
        Some(c) => {
            if !c.is_ascii() || !c.is_uppercase() {
                return false;
            }
        }
        None => {
            return false;
        }
    }

    word.chars().skip(1).all(|c| c.is_ascii() && (c.is_lowercase() || !c.is_alphabetic()))
}

#[inline]
pub fn is_combining(c: char) -> bool {
    canonical_combining_class(c) > 0
}

pub fn capitalize_and_normalize(word: &str) -> String {
    let mut capitalize_next = true;

    word.chars()
        .filter_map(|c| {
            let result =
                if HYPHENS.contains(&c) {
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

#[macro_export]
macro_rules! lowercase_alpha_without_accents {
    ($chars:expr) => {
        $chars.filter_map( |c|
            if c.is_uppercase() {
                c.to_lowercase().next()
            } else if c.is_alphabetic() {
                Some(c)
            } else {
                None
            }
        )
    }
}

#[macro_export]
macro_rules! eq_or_starts_with_ignoring_accents_nonalpha_and_case {
    ($chars_a:expr, $chars_b:expr) => {
        {
            let result;

            let mut iter_a = lowercase_alpha_without_accents!($chars_a);
            let mut iter_b = lowercase_alpha_without_accents!($chars_b);

            let mut compared = 0;

            loop {
                let ca = iter_a.next();
                let cb = iter_b.next();

                if ca.is_none() && cb.is_none() {
                    result = true;
                    break;
                } else if ca.is_none() != cb.is_none() {
                    // Only allow containment, vs equality, when contained
                    // string has at least four characters
                    result = compared >= MIN_CHARS_FOR_EQ_BY_CONTAINS;
                    break;
                } else if ca != cb {
                    result = false;
                    break;
                }

                compared += 1;
            }

            result
        }
    };
}
