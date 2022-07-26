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

fn casefolded_alphas(
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

    #[test]
    fn capitalization() {
        assert_eq!("A", capitalize_word("a", true));
        assert_eq!("Aa", capitalize_word("aa", true));
        assert_eq!("Aa", capitalize_word("AA", true));
        assert_eq!("Aa-Bb", capitalize_word("aa-bb", false));
        assert_eq!("Aa-Bb", capitalize_word("AA-BB", false));
        assert_eq!("Ss", capitalize_word("ß", false));
    }
}
