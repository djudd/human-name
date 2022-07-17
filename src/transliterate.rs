use std::borrow::Cow;
use std::str::Chars;
use unidecode::unidecode_char;

#[inline]
fn transliterate(c: char) -> Chars<'static> {
    // We should maybe use unicode case folding here as an initial pass,
    // but without a concrete motivating case (yet) it doesn't seem worth
    // the cost.
    unidecode_char(c).chars()
}

#[inline]
fn ascii_to_lower_if_alpha(c: char) -> Option<char> {
    debug_assert!(c.is_ascii(), "{}", c.to_string());

    if c.is_ascii_lowercase() {
        Some(c)
    } else if c.is_ascii_uppercase() {
        Some(c.to_ascii_lowercase())
    } else {
        None
    }
}

#[inline]
fn ascii_to_upper_if_alpha(c: char) -> Option<char> {
    debug_assert!(c.is_ascii(), "{}", c.to_string());

    if c.is_ascii_uppercase() {
        Some(c)
    } else if c.is_ascii_lowercase() {
        Some(c.to_ascii_uppercase())
    } else {
        None
    }
}

#[inline]
pub fn to_ascii_initial(c: char) -> Option<char> {
    debug_assert!(c.is_uppercase(), "{}", c.to_string());

    match c {
        'A'..='Z' => Some(c),
        _ => transliterate(c).find_map(ascii_to_upper_if_alpha),
    }
}

pub fn to_ascii_casefolded(text: &str) -> impl Iterator<Item = char> + '_ {
    text.chars()
        .flat_map(transliterate)
        .filter_map(ascii_to_lower_if_alpha)
}

pub fn to_ascii_casefolded_reversed(text: &str) -> impl Iterator<Item = char> + '_ {
    text.chars()
        .flat_map(transliterate)
        .rev()
        .filter_map(ascii_to_lower_if_alpha)
}

pub fn to_ascii_titlecase(s: &str) -> Cow<str> {
    if s.is_ascii() && s.bytes().all(|b| b.is_ascii_alphabetic()) {
        Cow::Borrowed(s)
    } else {
        let mut capitalized_any = false;

        Cow::Owned(
            s.chars()
                .flat_map(transliterate)
                .filter_map(|c| {
                    if !capitalized_any {
                        let result = ascii_to_upper_if_alpha(c);
                        if result.is_some() {
                            capitalized_any = true;
                        }
                        result
                    } else {
                        ascii_to_lower_if_alpha(c)
                    }
                })
                .collect(),
        )
    }
}
