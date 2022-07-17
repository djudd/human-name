use std::borrow::Cow;
use unicode_normalization::char::canonical_combining_class;
use unicode_normalization::{is_nfkd_quick, IsNormalized, UnicodeNormalization};

#[inline]
fn already_normalized(string: &str) -> bool {
    let mut banned_char = false;
    let normalized = is_nfkd_quick(string.chars().take_while(|&c| {
        banned_char = c.is_whitespace() && c != ' ';
        !banned_char
    }));
    normalized == IsNormalized::Yes && !banned_char
}

fn do_normalize(string: &str) -> String {
    string
        .chars()
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .nfkd()
        .collect()
}

pub fn normalize_nfkd_whitespace(string: &str) -> Cow<str> {
    if already_normalized(string) {
        Cow::Borrowed(string)
    } else {
        Cow::Owned(do_normalize(string))
    }
}

#[inline]
pub fn is_combining(c: char) -> bool {
    canonical_combining_class(c) > 0
}

pub fn combining_chars(word: &str) -> usize {
    word.chars().filter(|c| is_combining(*c)).count()
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "bench")]
    use super::*;

    #[cfg(feature = "bench")]
    use test::{black_box, Bencher};

    #[cfg(feature = "bench")]
    #[bench]
    fn normalize_ascii(b: &mut Bencher) {
        b.iter(|| black_box(normalize_nfkd_whitespace("James 'J' S. Brown MD").len()))
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn normalize_nfkd_stable(b: &mut Bencher) {
        b.iter(|| black_box(normalize_nfkd_whitespace("James «J» S. Brown MD").len()))
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn normalize_needs_fix(b: &mut Bencher) {
        b.iter(|| black_box(normalize_nfkd_whitespace("James 'J' S. Bröwn MD").len()))
    }
}
