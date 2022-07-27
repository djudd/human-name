use super::transliterate;
use crate::case::*;
use crate::features::starts_with_consonant;
use ahash::AHashMap;
use once_cell::sync::Lazy;
use std::borrow::Cow;
use std::iter;

static NAMES_BY_NICK_PREFIX: Lazy<AHashMap<&'static str, &'static [&'static str]>> =
    Lazy::new(|| {
        let mut map = AHashMap::new();
        include!(concat!(env!("OUT_DIR"), "/names_by_nick_prefix.rs"));
        map
    });

static NAMES_BY_IRREGULAR_NICK: Lazy<AHashMap<&'static str, &'static [&'static str]>> =
    Lazy::new(|| {
        let mut map = AHashMap::new();
        include!(concat!(env!("OUT_DIR"), "/names_by_irregular_nick.rs"));
        map
    });

const DIMINUTIVE_EXCEPTIONS: [&str; 6] = ["Mary", "Joy", "Roy", "Guy", "Amy", "Troy"];

const FINAL_SYLLABLES_EXCEPTIONS: [&str; 1] = [
    "Nathan", // Probably != Jonathan
];

// Returns tuple (close_char, must_precede_whitespace)
#[inline]
fn expected_close_char_if_opens_nickname(
    c: char,
    follows_whitespace: bool,
) -> Option<(char, bool)> {
    let close = match c {
        '(' => Some((')', false)),
        '[' => Some((']', false)),
        '<' => Some(('>', false)),
        '“' => Some(('”', false)),
        '〝' => Some(('〞', false)),
        '‹' => Some(('›', false)),
        '«' => Some(('»', false)),
        _ => None,
    };

    if close.is_some() {
        // Treat, e.g., opening parens as the start of a nickname
        // regardless of where it occurs
        return close;
    }

    if follows_whitespace {
        // Treat, e.g., quote character as the start of a nickname
        // only if it occurs after whitespace; otherwise, it
        // might be in-name punctuation
        match c {
            '\'' => Some(('\'', true)),
            '"' => Some(('"', true)),
            '‘' => Some(('’', true)),
            _ => None,
        }
    } else {
        None
    }
}

struct NickOpen {
    start_index: usize,
    open_char: char,
    follows_space: bool,
    expect_close_char: char,
    expect_closing_space: bool,
}

#[inline]
fn find_nick_open(input: &str) -> Option<NickOpen> {
    let mut follows_space = false;

    for (i, c) in input.char_indices() {
        if let Some((expect_close_char, expect_closing_space)) =
            expected_close_char_if_opens_nickname(c, follows_space)
        {
            return Some(NickOpen {
                start_index: i,
                open_char: c,
                follows_space,
                expect_close_char,
                expect_closing_space,
            });
        }

        follows_space = c == ' ';
    }

    None
}

#[cold]
fn find_close_and_strip(input: &str, open: NickOpen) -> Cow<str> {
    let NickOpen {
        start_index,
        open_char,
        follows_space,
        expect_close_char,
        expect_closing_space,
    } = open;

    let search_from = start_index + open_char.len_utf8();
    let strip_from = if follows_space {
        start_index - 1
    } else {
        start_index
    };

    if let Some(found_at) = input[search_from..].find(expect_close_char) {
        let i = found_at + search_from;
        let j = i + expect_close_char.len_utf8();

        if j >= input.len() {
            Cow::Borrowed(&input[0..start_index])
        } else if !expect_closing_space || input[j..].starts_with(' ') {
            Cow::Owned(input[0..strip_from].to_string() + &strip_nickname(&input[j..]))
        } else {
            Cow::Owned(input[0..i].to_string() + &strip_nickname(&input[i..]))
        }
    } else if !expect_closing_space {
        // When there's, e.g., an opening parens, but no closing parens, strip the
        // rest of the string
        Cow::Borrowed(&input[0..strip_from])
    } else {
        // Otherwise, even if there's an unmatched opening quote, don't
        // modify the string; assume an unmatched opening quote was just
        // in-name punctuation
        //
        // However, in that case, we need to check the remainder of the
        // string for actual nicknames, whose opening character we might
        // have missed while looking for the first closing character
        if search_from >= input.len() {
            Cow::Borrowed(input)
        } else {
            Cow::Owned(input[0..search_from].to_string() + &strip_nickname(&input[search_from..]))
        }
    }
}

// Optimized for the case where there is no nickname, and secondarily for the
// case where there is only one. Two or more probably means bad input.
pub fn strip_nickname(input: &str) -> Cow<str> {
    if let Some(open) = find_nick_open(input) {
        find_close_and_strip(input, open)
    } else {
        Cow::Borrowed(input)
    }
}

struct NameVariants<'a> {
    original: &'a str,
    direct_variants: Option<&'a [&'static str]>,
    prefix_variants: Option<&'a [&'static str]>,
}

impl<'a> NameVariants<'a> {
    pub fn for_name(name: &'a str) -> NameVariants<'a> {
        NameVariants {
            original: name,
            direct_variants: NAMES_BY_IRREGULAR_NICK.get(name).copied(),
            prefix_variants: {
                if name.len() >= 4 && (name.ends_with("ie") || name.ends_with("ey")) {
                    NAMES_BY_NICK_PREFIX.get(&name[0..name.len() - 2]).copied()
                } else if name.len() >= 3 && name.ends_with('y') {
                    NAMES_BY_NICK_PREFIX.get(&name[0..name.len() - 1]).copied()
                } else {
                    None
                }
            },
        }
    }

    pub fn iter_with_original(&self) -> NameVariantIter {
        NameVariantIter {
            original: iter::once(self.original),
            direct_variants: self.direct_variants.map(|names| names.iter()),
            prefix_variants: self.prefix_variants.map(|names| names.iter()),
        }
    }
}

struct NameVariantIter<'a> {
    original: iter::Once<&'a str>,
    direct_variants: Option<std::slice::Iter<'a, &'static str>>,
    prefix_variants: Option<std::slice::Iter<'a, &'static str>>,
}

impl<'a> Iterator for NameVariantIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        if let Some(name) = self.original.next() {
            return Some(name);
        }

        if let Some(ref mut iter) = self.direct_variants {
            if let Some(name) = iter.next() {
                return Some(name);
            }
        }

        if let Some(ref mut iter) = self.prefix_variants {
            if let Some(name) = iter.next() {
                return Some(name);
            }
        }

        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.original.len()
            + self
                .direct_variants
                .as_ref()
                .map(|vs| vs.len())
                .unwrap_or(0)
            + self
                .prefix_variants
                .as_ref()
                .map(|vs| vs.len())
                .unwrap_or(0);
        (size, Some(size))
    }
}

impl<'a> ExactSizeIterator for NameVariantIter<'a> {}

fn transliterate_if_non_ascii(s: &str) -> Cow<str> {
    if s.is_ascii() && s.bytes().all(|b| b.is_ascii_alphabetic()) {
        // We were already titlecased by namecase::namecase,
        // so we don't need to do anything
        Cow::Borrowed(s)
    } else {
        transliterate::to_ascii_titlecase(s)
            .map(Cow::Owned)
            .unwrap_or(Cow::Borrowed(s))
    }
}

pub fn have_matching_variants(original_a: &str, original_b: &str) -> bool {
    let original_a = transliterate_if_non_ascii(original_a);
    let original_b = transliterate_if_non_ascii(original_b);

    let a_variants = NameVariants::for_name(&original_a);
    let b_variants = NameVariants::for_name(&original_b);

    a_variants.iter_with_original().any(|a| {
        b_variants
            .iter_with_original()
            .any(|b| variants_match(a, b))
    })
}

#[inline]
fn variants_match(a: &str, b: &str) -> bool {
    let (longer, shorter) = if a.len() >= b.len() { (a, b) } else { (b, a) };

    have_prefix_match(longer, shorter)
        || is_final_syllables_of(shorter, longer)
        || matches_without_diminutive(a, b)
        || matches_without_diminutive(b, a)
}

#[inline]
fn have_prefix_match(longer: &str, shorter: &str) -> bool {
    eq_casefolded_alpha_prefix(longer, shorter) && !is_simple_feminization(longer, shorter)
}

#[inline]
fn is_simple_feminization(longer: &str, shorter: &str) -> bool {
    longer.len() == shorter.len() + 1 && longer.ends_with('a')
}

#[inline]
fn matches_without_diminutive(a: &str, b: &str) -> bool {
    matches_without_y_or_e(a, b)
        || matches_without_ie_or_ey(a, b)
        || matches_without_ita_or_ina(a, b)
        || matches_without_ito(a, b)
}

#[inline]
fn matches_without_y_or_e(a: &str, b: &str) -> bool {
    a.len() > 2
        && b.len() >= a.len() - 1
        && (a.ends_with('y') || a.ends_with('e'))
        && matches_after_removing_diminutive(a, b, 1)
}

#[inline]
fn matches_without_ie_or_ey(a: &str, b: &str) -> bool {
    a.len() > 4
        && b.len() >= a.len() - 2
        && (a.ends_with("ie") || a.ends_with("ey"))
        && matches_after_removing_diminutive(a, b, 2)
}

#[inline]
fn matches_without_ita_or_ina(a: &str, b: &str) -> bool {
    a.len() > 5
        && b.len() >= a.len() - 3
        && b.ends_with('a')
        && (a.ends_with("ita") || a.ends_with("ina"))
        && matches_after_removing_diminutive(a, b, 3)
}

#[inline]
fn matches_without_ito(a: &str, b: &str) -> bool {
    a.len() > 5
        && b.len() >= a.len() - 3
        && b.ends_with('o')
        && a.ends_with("ito")
        && matches_after_removing_diminutive(a, b, 3)
}

#[inline(never)]
fn matches_after_removing_diminutive(a: &str, b: &str, diminutive_len: usize) -> bool {
    eq_casefolded_alpha_prefix(&a[0..a.len() - diminutive_len], b)
        && !DIMINUTIVE_EXCEPTIONS.contains(&a)
}

#[inline]
fn is_final_syllables_of(needle: &str, haystack: &str) -> bool {
    if needle.len() == haystack.len() - 1
        && !starts_with_consonant(haystack)
        && eq_casefolded_alpha_suffix(needle, haystack)
    {
        true
    } else if haystack.len() < 4 || needle.len() < 2 || needle.len() > haystack.len() - 2 {
        false
    } else if starts_with_consonant(needle)
        || needle.starts_with("Ann")
        || haystack.starts_with("Mary")
    {
        eq_casefolded_alpha_suffix(needle, haystack)
            && !FINAL_SYLLABLES_EXCEPTIONS.contains(&needle)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "bench")]
    use test::{black_box, Bencher};

    #[test]
    fn nick_and_name() {
        assert!(have_matching_variants("Dave", "David"));
        assert!(have_matching_variants("David", "Dave"));
        assert!(have_matching_variants("Kenneth", "Kenny"));
        assert!(have_matching_variants("Kenny", "Kenneth"));
        assert!(have_matching_variants("Edward", "Eddie"));
        assert!(have_matching_variants("Eddie", "Edward"));
        assert!(have_matching_variants("Dot", "Dorothy"));
        assert!(have_matching_variants("Dorothy", "Dot"));
        assert!(have_matching_variants("Leroy", "Roy"));
        assert!(have_matching_variants("Roy", "Leroy"));
        assert!(have_matching_variants("Ann", "Agnes"));
        assert!(have_matching_variants("Annie", "Luann"));
        assert!(have_matching_variants("Marianne", "Mary"));
        assert!(have_matching_variants("Marianne", "Anne"));
    }

    #[test]
    fn matching_nicks() {
        assert!(have_matching_variants("Trisha", "Trix"));
        assert!(have_matching_variants("Trix", "Trisha"));
        assert!(have_matching_variants("Kenny", "Ken"));
        assert!(have_matching_variants("Ken", "Kenny"));
        assert!(have_matching_variants("Ned", "Eddie"));
        assert!(have_matching_variants("Eddie", "Ned"));
        assert!(have_matching_variants("Davy", "Dave"));
        assert!(have_matching_variants("Dave", "Davy"));
        assert!(have_matching_variants("Lon", "Al")); // Alonzo
        assert!(have_matching_variants("Al", "Lon")); // Alonzo
        assert!(have_matching_variants("Lousie", "Lulu"));
    }

    #[test]
    fn nonmatching_nicks() {
        assert!(!have_matching_variants("Xina", "Xander"));
        assert!(!have_matching_variants("Xander", "Xina"));
        assert!(!have_matching_variants("Andy", "Xander"));
        assert!(!have_matching_variants("Xander", "Andy"));
        assert!(!have_matching_variants("Molly", "Annie"));
        assert!(!have_matching_variants("Christopher", "Tina"));
        assert!(!have_matching_variants("Molly", "Mark"));
        assert!(!have_matching_variants("Patricia", "Rick"));
    }

    #[test]
    fn nonmatching_names() {
        assert!(!have_matching_variants("Antoinette", "Luanne"));
        assert!(!have_matching_variants("Luanne", "Antoinette"));
        assert!(!have_matching_variants("Jane", "John"));
        assert!(!have_matching_variants("John", "Jane"));
        assert!(!have_matching_variants("John", "Nathan"));
        assert!(!have_matching_variants("Mary", "Margeret"));
        assert!(!have_matching_variants("Annette", "Johanna"));
    }

    #[test]
    fn non_bmp_alphas() {
        assert!(have_matching_variants("𐒴𐓘", "𐒴𐓘"));
        assert!(!have_matching_variants("𐒴𐓘", "𐒴𐓙"));
    }

    #[test]
    fn emojis() {
        //assert!(have_matching_variants("😃", "😃"));
        //assert!(!have_matching_variants("😃", "😰"));
    }

    #[test]
    fn variants() {
        assert_eq!(
            vec!["Ada", "Adelaide", "Adele", "Adelina", "Adeline"],
            NameVariants::for_name("Ada")
                .iter_with_original()
                .collect::<Vec<_>>()
        );
        assert_eq!(
            vec!["Adele"],
            NameVariants::for_name("Adele")
                .iter_with_original()
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn strip_nothing() {
        assert_eq!("Robert Roberts", strip_nickname("Robert Roberts"));
    }

    #[test]
    fn strip_parens() {
        assert_eq!("Robert Roberts", strip_nickname("Robert (Mr. Bob) Roberts"));
    }

    #[test]
    fn unmatched_parens() {
        assert_eq!("Robert", strip_nickname("Robert (Mr. Bob"));
    }

    #[test]
    fn strip_quotes() {
        assert_eq!("Robert Roberts", strip_nickname("Robert 'Mr. Bob' Roberts"));
    }

    #[test]
    fn unmatched_quote() {
        assert_eq!(
            "Robert Mr. Bob' Roberts",
            strip_nickname("Robert Mr. Bob' Roberts")
        );
    }

    #[test]
    fn unspaced_quotes() {
        assert_eq!("Ro'bert R'oberts", strip_nickname("Ro'bert R'oberts"));
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn strip_nick_no_nick(b: &mut Bencher) {
        b.iter(|| {
            black_box(strip_nickname("James T. Kirk").len());
        })
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn strip_nick_with_nick(b: &mut Bencher) {
        b.iter(|| {
            black_box(strip_nickname("James T. 'Jimmy' Kirk").len());
        })
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn have_matching_variants_false(b: &mut Bencher) {
        b.iter(|| {
            black_box(have_matching_variants("David", "Daniel"));
        })
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn have_matching_variants_true(b: &mut Bencher) {
        b.iter(|| {
            black_box(have_matching_variants("David", "Dave"));
        })
    }
}
