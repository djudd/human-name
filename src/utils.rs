use std::borrow::Cow;
use unicode_normalization::char::canonical_combining_class;
use unicode_normalization::{is_nfkd_quick, IsNormalized, UnicodeNormalization};

pub fn is_mixed_case(s: &str) -> bool {
    let mut has_lowercase = false;
    let mut has_uppercase = false;
    let mut iter = s.chars();

    loop {
        match iter.next() {
            Some(c) => {
                if c.is_uppercase() {
                    has_uppercase = true;
                    break;
                } else if c.is_lowercase() {
                    has_lowercase = true;
                    break;
                }
            }
            None => {
                return false;
            }
        }
    }

    if has_lowercase {
        iter.any(|c| c.is_uppercase())
    } else {
        debug_assert!(has_uppercase);
        iter.any(|c| c.is_lowercase())
    }
}

#[inline]
pub fn is_combining(c: char) -> bool {
    canonical_combining_class(c) > 0
}

// Specialized for name-casing
pub fn capitalize_word(word: &str, simple: bool) -> String {
    const NONASCII_HYPHENS: &str = "\u{2010}‑‒–—―−－﹘﹣";

    debug_assert!(simple == word.chars().all(|c| c.is_ascii_alphabetic()));

    if simple {
        let bytes = word.as_bytes();
        let mut result = String::with_capacity(word.len());
        result.push(bytes[0].to_ascii_uppercase() as char);
        result.extend(bytes[1..].iter().map(|c| c.to_ascii_lowercase() as char));
        result
    } else {
        let mut capitalize_next = true;
        let mut result = String::with_capacity(word.len());

        for c in word.chars() {
            let mapped = if capitalize_next {
                CaseMapping::titlecase(c)
            } else {
                CaseMapping::lowercase(c)
            };

            if !matches!(mapped, CaseMapping::Empty) {
                result.extend(mapped);
                capitalize_next = false;
            } else {
                // No titlecase mapping, which is a prerequisite for being a separator
                capitalize_next = !c.is_alphanumeric() && !is_combining(c);
                if capitalize_next && NONASCII_HYPHENS.contains(c) {
                    result.push('-');
                } else {
                    result.push(c);
                }
            }
        }

        result
    }
}

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

#[derive(Debug, Clone)]
pub struct CharacterCounts {
    pub chars: u8,
    pub alpha: u8,
    pub upper: u8,
    pub ascii_alpha: u8,
}

pub fn categorize_chars(word: &str) -> CharacterCounts {
    debug_assert!(word.len() <= u8::max_value() as usize);

    let mut chars = 0;
    let mut alpha = 0;
    let mut upper = 0;
    let mut ascii_alpha = 0;

    for c in word.chars() {
        match c {
            'a'..='z' => {
                ascii_alpha += 1;
            }
            'A'..='Z' => {
                ascii_alpha += 1;
                upper += 1;
            }
            _ if c.is_uppercase() => {
                alpha += 1;
                upper += 1;
            }
            _ if c.is_alphabetic() => {
                alpha += 1;
            }
            _ => {
                chars += 1;
            }
        }
    }

    alpha += ascii_alpha;
    chars += alpha;

    CharacterCounts {
        chars,
        alpha,
        upper,
        ascii_alpha,
    }
}

pub fn has_no_vowels(word: &str) -> bool {
    const VOWELS: &[char] = &['a', 'e', 'i', 'o', 'u', 'y', 'A', 'E', 'I', 'O', 'U', 'Y'];
    !word.contains(VOWELS)
}

pub fn starts_with_consonant(word: &str) -> bool {
    word.chars()
        .next()
        .filter(|c| c.is_ascii_alphabetic() && !"aeiouAEIOU".contains(*c))
        .is_some()
}

pub fn starts_with_uppercase(word: &str) -> bool {
    word.chars().next().filter(|c| c.is_uppercase()).is_some()
}

pub fn combining_chars(word: &str) -> usize {
    word.chars().filter(|c| is_combining(*c)).count()
}

pub fn has_sequential_alphas(word: &str) -> bool {
    let mut prev_alpha = false;
    for c in word.chars() {
        let alpha = c.is_alphabetic();
        if prev_alpha && alpha {
            return true;
        } else {
            prev_alpha = alpha;
        }
    }

    false
}

#[derive(Debug)]
enum CaseMapping {
    Empty,
    Single(char),
    Double(char, char),
    Triple(char, char, char),
}

impl CaseMapping {
    #[inline]
    fn lowercase(c: char) -> CaseMapping {
        let [x, y] = unicode_case_mapping::to_lowercase(c);
        // SAFETY: We're trusting that the unicode_case_mapping crate outputs
        // only valid chars or zero
        unsafe { Self::chars_from_u32(x, y, 0) }
    }

    #[inline]
    fn titlecase(c: char) -> CaseMapping {
        let [x, y, z] = unicode_case_mapping::to_titlecase(c);
        // SAFETY: We're trusting that the unicode_case_mapping crate outputs
        // only valid chars or zero
        unsafe { Self::chars_from_u32(x, y, z) }
    }

    // SAFETY: All arguments must be valid characters
    #[inline]
    unsafe fn chars_from_u32(x: u32, y: u32, z: u32) -> CaseMapping {
        debug_assert!([x, y, z].iter().all(|c| char::from_u32(*c).is_some()));

        if x > 0 {
            let x = char::from_u32_unchecked(x);
            if y > 0 {
                let y = char::from_u32_unchecked(y);
                if z > 0 {
                    let z = char::from_u32_unchecked(z);
                    CaseMapping::Triple(x, y, z)
                } else {
                    CaseMapping::Double(x, y)
                }
            } else {
                CaseMapping::Single(x)
            }
        } else {
            CaseMapping::Empty
        }
    }
}

impl Iterator for CaseMapping {
    type Item = char;

    #[inline]
    fn next(&mut self) -> Option<char> {
        match *self {
            CaseMapping::Triple(x, y, z) => {
                let _ = std::mem::replace(self, CaseMapping::Double(y, z));
                Some(x)
            }
            CaseMapping::Double(x, y) => {
                let _ = std::mem::replace(self, CaseMapping::Single(y));
                Some(x)
            }
            CaseMapping::Single(x) => {
                let _ = std::mem::replace(self, CaseMapping::Empty);
                Some(x)
            }
            CaseMapping::Empty => None,
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = match self {
            CaseMapping::Triple(_, _, _) => 3,
            CaseMapping::Double(_, _) => 2,
            CaseMapping::Single(_) => 1,
            CaseMapping::Empty => 0,
        };
        (size, Some(size))
    }
}

impl DoubleEndedIterator for CaseMapping {
    #[inline]
    fn next_back(&mut self) -> Option<char> {
        match *self {
            CaseMapping::Triple(x, y, z) => {
                let _ = std::mem::replace(self, CaseMapping::Double(x, y));
                Some(z)
            }
            CaseMapping::Double(x, y) => {
                let _ = std::mem::replace(self, CaseMapping::Single(x));
                Some(y)
            }
            CaseMapping::Single(x) => {
                let _ = std::mem::replace(self, CaseMapping::Empty);
                Some(x)
            }
            CaseMapping::Empty => None,
        }
    }
}

impl ExactSizeIterator for CaseMapping {}

fn case_folded_alpha_chars(
    text: &str,
) -> impl Iterator<Item = char> + std::iter::DoubleEndedIterator + '_ {
    // It would be more correct to use unicode case folding here,
    // but the only crate I've found which doesn't allocate by default
    // (operating on iterators rather than strings) is `caseless`,
    // and it has the problems that (a) its iterator isn't double-ended
    // and (b) it's slower.
    text.chars().flat_map(|c| {
        let mapped = CaseMapping::lowercase(c);
        if !matches!(mapped, CaseMapping::Empty) {
            mapped
        } else if c.is_alphabetic() {
            CaseMapping::Single(c)
        } else {
            CaseMapping::Empty
        }
    })
}

pub fn eq_or_starts_with(a: &str, b: &str) -> bool {
    let mut chars_a = case_folded_alpha_chars(a);
    let mut chars_b = case_folded_alpha_chars(b);
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
}

pub fn eq_or_ends_with(needle: &str, haystack: &str) -> bool {
    let mut n_chars = case_folded_alpha_chars(needle).rev();
    let mut h_chars = case_folded_alpha_chars(haystack).rev();
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "bench")]
    use test::{black_box, Bencher};

    #[test]
    fn sequential_alphas() {
        assert!(has_sequential_alphas("ab"));
        assert!(has_sequential_alphas("abc"));
        assert!(has_sequential_alphas("a.bc"));
        assert!(has_sequential_alphas("鄭a"));
        assert!(!has_sequential_alphas(""));
        assert!(!has_sequential_alphas("a"));
        assert!(!has_sequential_alphas("a.b"));
        assert!(!has_sequential_alphas("鄭.a"));
        assert!(!has_sequential_alphas("ﾟ."));
    }

    #[test]
    fn capitalization() {
        assert_eq!("A", capitalize_word("a", true));
        assert_eq!("Aa", capitalize_word("aa", true));
        assert_eq!("Aa", capitalize_word("AA", true));
        assert_eq!("Aa-Bb", capitalize_word("aa-bb", false));
        assert_eq!("Aa-Bb", capitalize_word("AA-BB", false));
        assert_eq!("Ss", capitalize_word("ß", false));
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn is_mixed_case_false(b: &mut Bencher) {
        b.iter(|| black_box(is_mixed_case("JOHN MACDONALD")))
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn is_mixed_case_true(b: &mut Bencher) {
        b.iter(|| black_box(is_mixed_case("J. MacDonald")))
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn capitalize_uppercase_word(b: &mut Bencher) {
        b.iter(|| black_box(capitalize_word("JONATHAN", true)))
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn capitalize_complex_word(b: &mut Bencher) {
        b.iter(|| black_box(capitalize_word("föö-bar", false)))
    }

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
