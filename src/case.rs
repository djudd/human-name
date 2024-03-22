use crate::decomposition::is_combining;

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
            CaseMapping::Empty => None,
            CaseMapping::Single(x) => {
                let _ = std::mem::replace(self, CaseMapping::Empty);
                Some(x)
            }
            CaseMapping::Double(x, y) => {
                let _ = std::mem::replace(self, CaseMapping::Single(y));
                Some(x)
            }
            CaseMapping::Triple(x, y, z) => {
                let _ = std::mem::replace(self, CaseMapping::Double(y, z));
                Some(x)
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = match self {
            CaseMapping::Empty => 0,
            CaseMapping::Single(_) => 1,
            CaseMapping::Double(_, _) => 2,
            CaseMapping::Triple(_, _, _) => 3,
        };
        (size, Some(size))
    }
}

impl DoubleEndedIterator for CaseMapping {
    #[inline]
    fn next_back(&mut self) -> Option<char> {
        match *self {
            CaseMapping::Empty => None,
            CaseMapping::Single(x) => {
                let _ = std::mem::replace(self, CaseMapping::Empty);
                Some(x)
            }
            CaseMapping::Double(x, y) => {
                let _ = std::mem::replace(self, CaseMapping::Single(x));
                Some(y)
            }
            CaseMapping::Triple(x, y, z) => {
                let _ = std::mem::replace(self, CaseMapping::Double(x, y));
                Some(z)
            }
        }
    }
}

impl ExactSizeIterator for CaseMapping {}

fn casefolded_alphas(text: &str) -> impl std::iter::DoubleEndedIterator<Item = char> + '_ {
    // It would be more correct to use unicode case folding here,
    // but unicode-case-mapping 0.4 only supports simple case folding
    // and not multi-character folding, which isn't really better.
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

#[inline]
pub fn eq_casefolded_alpha_prefix(a: &str, b: &str) -> bool {
    !casefolded_alphas(a)
        .zip(casefolded_alphas(b))
        .any(|(a, b)| a != b)
}

#[inline]
pub fn eq_casefolded_alpha_suffix(a: &str, b: &str) -> bool {
    !casefolded_alphas(a)
        .rev()
        .zip(casefolded_alphas(b).rev())
        .any(|(a, b)| a != b)
}

// Specialized for name-casing
pub fn capitalize_word(word: &str, simple: bool) -> String {
    const NONASCII_HYPHENS: &str = "\u{2010}‑‒–—―−－﹘﹣";

    debug_assert!(
        simple == word.chars().all(|c| c.is_ascii_alphabetic()),
        "{:?} did not match simple={}",
        word,
        simple
    );

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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "bench")]
    use test::{black_box, Bencher};

    #[test]
    fn capitalization() {
        assert_eq!("A", capitalize_word("a", true));
        assert_eq!("Aa", capitalize_word("aa", true));
        assert_eq!("Aa", capitalize_word("AA", true));
        assert_eq!("Aa-Bb", capitalize_word("aa-bb", false));
        assert_eq!("Aa-Bb", capitalize_word("AA-BB", false));
        assert_eq!("Ss", capitalize_word("ß", false));
        assert_eq!("ʥar", capitalize_word("ʥar", false));
    }

    #[test]
    fn prefix() {
        assert!(eq_casefolded_alpha_prefix("foo", "foo"));
        assert!(eq_casefolded_alpha_prefix("foo", "FOObar"));
        assert!(eq_casefolded_alpha_prefix("FOObar", "foo"));
        assert!(!eq_casefolded_alpha_prefix("bar", "fooBAR"));
        assert!(!eq_casefolded_alpha_prefix("fooBAR", "bar"));
        // TODO Proper case-folding support should fix this
        //assert!(eq_casefolded_alpha_prefix("fooß", "foossar"));
        //assert!(eq_casefolded_alpha_prefix("foossar", "fooß"));
        assert!(eq_casefolded_alpha_prefix("f😃o1", "f😰o2"));
        assert!(eq_casefolded_alpha_prefix("fo😃1", "foobar"));
        assert!(eq_casefolded_alpha_prefix("", ""));
        assert!(eq_casefolded_alpha_prefix("", "foo"));
        assert!(eq_casefolded_alpha_prefix("😃", "😰"));
    }

    #[test]
    fn suffix() {
        assert!(eq_casefolded_alpha_suffix("foo", "foo"));
        assert!(eq_casefolded_alpha_suffix("bar", "fooBAR"));
        assert!(eq_casefolded_alpha_suffix("fooBAR", "bar"));
        assert!(!eq_casefolded_alpha_suffix("foo", "FOObar"));
        assert!(!eq_casefolded_alpha_suffix("FOObar", "foo"));
        // TODO Proper case-folding support should fix this
        //assert!(eq_casefolded_alpha_suffix("fooß", "foossar"));
        //assert!(eq_casefolded_alpha_suffix("foossar", "fooß"));
        assert!(eq_casefolded_alpha_suffix("f😃o1", "f😰o2"));
        assert!(eq_casefolded_alpha_suffix("ba😃r1", "foobar"));
        assert!(eq_casefolded_alpha_suffix("", ""));
        assert!(eq_casefolded_alpha_suffix("", "foo"));
        assert!(eq_casefolded_alpha_suffix("😃", "😰"));
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
}
