use std::str::Chars;
use unidecode::unidecode_char;

#[inline]
fn transliterate(c: char) -> Chars<'static> {
    let s = unidecode_char(c);
    if s.is_empty() {}
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
    match c {
        'A'..='Z' => Some(c),
        _ => transliterate(c).find_map(ascii_to_upper_if_alpha),
    }
}

pub fn to_ascii_casefolded(text: &str) -> Option<impl Iterator<Item = char> + '_> {
    let mut result = text
        .chars()
        .flat_map(transliterate)
        .filter_map(ascii_to_lower_if_alpha)
        .peekable();

    let has_next = result.peek().is_some();
    if has_next {
        Some(result)
    } else {
        None
    }
}

pub fn to_ascii_casefolded_reversed(text: &str) -> Option<impl Iterator<Item = char> + '_> {
    let mut result = text
        .chars()
        .flat_map(transliterate)
        .rev()
        .filter_map(ascii_to_lower_if_alpha)
        .peekable();

    let has_next = result.peek().is_some();
    if has_next {
        Some(result)
    } else {
        None
    }
}

pub fn to_ascii_titlecase(s: &str) -> Option<String> {
    let mut result = s
        .chars()
        .flat_map(transliterate)
        .filter_map(ascii_to_lower_if_alpha);

    result.next().map(|initial| {
        let mut s = String::with_capacity(s.len());
        s.push(initial);
        s.extend(result);
        s
    })
}
