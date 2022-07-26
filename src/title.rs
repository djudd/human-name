use super::case::capitalize_word;
use super::namepart::{Category, NamePart};
use super::suffix;
use crate::Cow;
use ahash::AHashMap;
use once_cell::sync::Lazy;
use std::cmp;

const TWO_CHAR_TITLES: [&str; 4] = ["mr", "ms", "sr", "dr"];

static HONORIFIC_PREFIXES: Lazy<AHashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut map = AHashMap::new();
    include!(concat!(env!("OUT_DIR"), "/honorific_prefixes.rs"));
    map
});

static HONORIFIC_SUFFIXES: Lazy<AHashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut map = AHashMap::new();
    include!(concat!(env!("OUT_DIR"), "/honorific_suffixes.rs"));
    map
});

fn might_be_title_part(word: &NamePart) -> bool {
    if word.counts.chars < 3 {
        // Allow any word with 1 or 2 characters as part of a title (but see below)
        true
    } else {
        match &word.category {
            Category::Name(ref namecased) => {
                HONORIFIC_PREFIXES.contains_key(namecased.as_ref())
                    || namecased.chars().any(char::is_numeric)
            }
            _ => true,
        }
    }
}

fn might_be_last_title_part(word: &NamePart) -> bool {
    // Don't allow 1 or 2-character words as the whole or final piece of
    // a title, except a set of very-common two-character title abbreviations,
    // because otherwise we are more likely dealing with initials
    match word.counts.alpha {
        0..=1 => false,
        2 if word.counts.chars == 2 => TWO_CHAR_TITLES
            .iter()
            .any(|title| title.eq_ignore_ascii_case(word.word)),
        _ => might_be_title_part(word),
    }
}

fn is_prefix_title(words: &[NamePart]) -> bool {
    match words.last() {
        Some(word) => {
            if !might_be_last_title_part(word) {
                return false;
            }
        }
        None => {
            return false;
        }
    }

    if words.len() > 1 {
        words[0..words.len() - 1].iter().all(might_be_title_part)
    } else {
        true
    }
}

fn is_postfix_title(word: &NamePart, might_be_initials: bool) -> bool {
    match word.category {
        Category::Name(ref namecased) => {
            HONORIFIC_SUFFIXES.contains_key(namecased.as_ref())
                || namecased.chars().any(char::is_numeric)
        }
        Category::Initials => !might_be_initials && word.counts.alpha > 1,
        _ => true,
    }
}

pub fn find_prefix_len(words: &[NamePart]) -> usize {
    let mut prefix_len = words.len() - 1;

    while prefix_len > 0 {
        let found_prefix = {
            let next_word = &words[prefix_len];
            (next_word.is_namelike() || next_word.is_initials())
                && is_prefix_title(&words[0..prefix_len])
        };

        if found_prefix {
            break;
        } else {
            prefix_len -= 1;
        }
    }

    prefix_len
}

pub fn find_postfix_index(words: &[NamePart], expect_initials: bool) -> usize {
    let last_nonpostfix_index = words.iter().rposition(|word| {
        suffix::generation_from_suffix(word, expect_initials).is_none()
            && !is_postfix_title(word, expect_initials)
    });

    let first_abbr_index = words
        .iter()
        .position(|word| !word.is_namelike() && !word.is_initials())
        .unwrap_or(words.len());

    cmp::min(
        first_abbr_index,
        match last_nonpostfix_index {
            Some(i) => i + 1,
            None => 0,
        },
    )
}

pub fn canonicalize_suffix<'a>(title: &'a NamePart<'a>) -> Cow<'a, str> {
    match &title.category {
        Category::Name(namecased) => {
            if let Some(canonical) = HONORIFIC_SUFFIXES.get(namecased.as_ref()) {
                Cow::Borrowed(canonical)
            } else {
                Cow::Borrowed(namecased)
            }
        }
        Category::Initials => {
            // If there's existing punctuation, assume formatting is intentional.
            if title.counts.chars != title.counts.alpha {
                return Cow::Borrowed(title.word);
            }

            // Otherwise, ignore case to check for a known canonical form (restricting
            // to ASCII just for simplicity since our list of honorifics is 100% ASCII).
            if title.counts.chars == title.counts.ascii_alpha {
                let capitalized = capitalize_word(title.word, true);
                if let Some(canonical) = HONORIFIC_SUFFIXES.get(capitalized.as_str()) {
                    return Cow::Borrowed(canonical);
                }
            }

            // Assume unrecognized honorifics are acronyms (given that we previously
            // categorized as initials). For length two or less, format with periods
            // (e.g. "M.D."), but skip periods for longer acronyms (e.g. "LCSW").
            if title.word.len() <= 2 {
                let mut result = String::with_capacity((title.counts.alpha * 2).into());
                title.with_initials(|c| {
                    for u in c.to_uppercase() {
                        result.push(u);
                    }
                    result.push('.');
                });
                Cow::Owned(result)
            } else {
                let mut result = String::with_capacity((title.counts.alpha).into());
                title.with_initials(|c| {
                    for u in c.to_uppercase() {
                        result.push(u);
                    }
                });
                Cow::Owned(result)
            }
        }
        Category::Abbreviation | Category::Other => Cow::Borrowed(title.word),
    }
}

pub fn canonicalize_prefix<'a>(title: &'a NamePart<'a>) -> Cow<'a, str> {
    match &title.category {
        Category::Name(namecased) => {
            if let Some(canonical) = HONORIFIC_PREFIXES.get(namecased.as_ref()) {
                Cow::Borrowed(canonical)
            } else {
                Cow::Borrowed(namecased)
            }
        }
        Category::Initials => {
            // If there's existing punctuation, assume formatting is intentional.
            if title.counts.chars != title.counts.alpha {
                return Cow::Borrowed(title.word);
            }

            // Otherwise, ignore case to check for a known canonical form (restricting
            // to ASCII just for simplicity since our list of honorifics is 100% ASCII).
            if title.counts.chars == title.counts.ascii_alpha {
                let capitalized = capitalize_word(title.word, true);
                if let Some(canonical) = HONORIFIC_PREFIXES.get(capitalized.as_str()) {
                    return Cow::Borrowed(canonical);
                }
            }

            // For unrecognized honorifics, canonicalize as an abbreviation (e.g. "Dr.").
            let mut result = String::with_capacity(usize::from(title.counts.alpha) + 1);
            title.with_initials(|c| {
                if result.is_empty() {
                    result.push(c);
                } else {
                    for l in c.to_lowercase() {
                        result.push(l);
                    }
                }
            });
            result.push('.');
            Cow::Owned(result)
        }
        Category::Abbreviation | Category::Other => Cow::Borrowed(title.word),
    }
}

#[cfg(test)]
mod tests {
    use super::super::namepart::{Location, NamePart};
    use super::*;

    #[test]
    fn canonicalize_doctor_prefix() {
        assert_eq!(
            "Dr.",
            canonicalize_prefix(&NamePart::from_word("DR", true, Location::Start))
        );
        assert_eq!(
            "Dr.",
            canonicalize_prefix(&NamePart::from_word("Dr", true, Location::Start))
        );
        assert_eq!(
            "Dr.",
            canonicalize_prefix(&NamePart::from_word("dr", true, Location::Start))
        );
        assert_eq!(
            "Dr.",
            canonicalize_prefix(&NamePart::from_word("Doctor", true, Location::Start))
        );
        assert_eq!(
            "Dr.",
            canonicalize_prefix(&NamePart::from_word("Dr.", true, Location::Start))
        );
    }

    #[test]
    fn canonicalize_mister_prefix() {
        assert_eq!(
            "Mr.",
            canonicalize_prefix(&NamePart::from_word("MR", true, Location::Start))
        );
        assert_eq!(
            "Mr.",
            canonicalize_prefix(&NamePart::from_word("Mr", true, Location::Start))
        );
        assert_eq!(
            "Mr.",
            canonicalize_prefix(&NamePart::from_word("mr", true, Location::Start))
        );
        assert_eq!(
            "Mr.",
            canonicalize_prefix(&NamePart::from_word("Mister", true, Location::Start))
        );
        assert_eq!(
            "Mr.",
            canonicalize_prefix(&NamePart::from_word("Master", true, Location::Start))
        );
        assert_eq!(
            "Mr.",
            canonicalize_prefix(&NamePart::from_word("Mr.", true, Location::Start))
        );
    }

    #[test]
    fn canonicalize_mrs_prefix() {
        assert_eq!(
            "Mrs.",
            canonicalize_prefix(&NamePart::from_word("MRS", true, Location::Start))
        );
        assert_eq!(
            "Mrs.",
            canonicalize_prefix(&NamePart::from_word("Mrs", true, Location::Start))
        );
        assert_eq!(
            "Mrs.",
            canonicalize_prefix(&NamePart::from_word("mrs", true, Location::Start))
        );
        assert_eq!(
            "Mrs.",
            canonicalize_prefix(&NamePart::from_word("Missus", true, Location::Start))
        );
        assert_eq!(
            "Mrs.",
            canonicalize_prefix(&NamePart::from_word("Mrs.", true, Location::Start))
        );
    }

    #[test]
    fn canonicalize_prof_prefix() {
        assert_eq!(
            "Prof.",
            canonicalize_prefix(&NamePart::from_word("PROF", true, Location::Start))
        );
        assert_eq!(
            "Prof.",
            canonicalize_prefix(&NamePart::from_word("Prof", true, Location::Start))
        );
        assert_eq!(
            "Prof.",
            canonicalize_prefix(&NamePart::from_word("prof", true, Location::Start))
        );
        assert_eq!(
            "Prof.",
            canonicalize_prefix(&NamePart::from_word("Professor", true, Location::Start))
        );
        assert_eq!(
            "Prof.",
            canonicalize_prefix(&NamePart::from_word("Prof.", true, Location::Start))
        );
    }

    #[test]
    fn canonicalize_sir_prefix() {
        assert_eq!(
            "Sir",
            canonicalize_prefix(&NamePart::from_word("Sir", true, Location::Start))
        );
        assert_eq!(
            "Sir",
            canonicalize_prefix(&NamePart::from_word("Sir", true, Location::Start))
        );
        assert_eq!(
            "Sir",
            canonicalize_prefix(&NamePart::from_word("Sir", true, Location::Start))
        );
    }

    #[test]
    fn canonicalize_unrecognized_prefix() {
        assert_eq!(
            "Abc.",
            canonicalize_prefix(&NamePart::from_word("ABC", true, Location::Start))
        );
        assert_eq!(
            "Abc",
            canonicalize_prefix(&NamePart::from_word("Abc", true, Location::Start))
        );
        assert_eq!(
            "Abc",
            canonicalize_prefix(&NamePart::from_word("abc", true, Location::Start))
        );
        assert_eq!(
            "Abc.",
            canonicalize_prefix(&NamePart::from_word("Abc.", true, Location::Start))
        );

        assert_eq!(
            "Xx.",
            canonicalize_prefix(&NamePart::from_word("XX", true, Location::Start))
        );
        assert_eq!(
            "Xx.",
            canonicalize_prefix(&NamePart::from_word("Xx", true, Location::Start))
        );
        assert_eq!(
            "Xx.",
            canonicalize_prefix(&NamePart::from_word("xx", true, Location::Start))
        );
        assert_eq!(
            "Xx.",
            canonicalize_prefix(&NamePart::from_word("Xx.", true, Location::Start))
        );
    }

    #[test]
    fn canonicalize_phd_suffix() {
        assert_eq!(
            "Ph.D.",
            canonicalize_suffix(&NamePart::from_word("phd", true, Location::End))
        );
        assert_eq!(
            "Ph.D.",
            canonicalize_suffix(&NamePart::from_word("Phd", true, Location::End))
        );
        assert_eq!(
            "Ph.D.",
            canonicalize_suffix(&NamePart::from_word("PHD", true, Location::End))
        );
        assert_eq!(
            "Ph.D.",
            canonicalize_suffix(&NamePart::from_word("Ph.D.", true, Location::End))
        );
    }

    #[test]
    fn canonicalize_md_suffix() {
        assert_eq!(
            "M.D.",
            canonicalize_suffix(&NamePart::from_word("MD", true, Location::End))
        );
        assert_eq!(
            "M.D.",
            canonicalize_suffix(&NamePart::from_word("Md", true, Location::End))
        );
        assert_eq!(
            "M.D.",
            canonicalize_suffix(&NamePart::from_word("md", true, Location::End))
        );
        assert_eq!(
            "M.D.",
            canonicalize_suffix(&NamePart::from_word("M.D.", true, Location::End))
        );
    }

    #[test]
    fn canonicalize_esq_suffix() {
        assert_eq!(
            "Esq.",
            canonicalize_suffix(&NamePart::from_word("ESQ", true, Location::End))
        );
        assert_eq!(
            "Esq.",
            canonicalize_suffix(&NamePart::from_word("Esq", true, Location::End))
        );
        assert_eq!(
            "Esq.",
            canonicalize_suffix(&NamePart::from_word("esq", true, Location::End))
        );
        assert_eq!(
            "Esq.",
            canonicalize_suffix(&NamePart::from_word("Esquire", true, Location::End))
        );
        assert_eq!(
            "Esq.",
            canonicalize_suffix(&NamePart::from_word("Esq.", true, Location::End))
        );
    }

    #[test]
    fn canonicalize_unrecognized_suffix() {
        assert_eq!(
            "ABC",
            canonicalize_suffix(&NamePart::from_word("ABC", true, Location::End))
        );
        assert_eq!(
            "Abc",
            canonicalize_suffix(&NamePart::from_word("Abc", true, Location::End))
        );
        assert_eq!(
            "Abc",
            canonicalize_suffix(&NamePart::from_word("abc", true, Location::End))
        );
        assert_eq!(
            "A.B.C.",
            canonicalize_suffix(&NamePart::from_word("A.B.C.", true, Location::End))
        );

        assert_eq!(
            "X.X.",
            canonicalize_suffix(&NamePart::from_word("XX", true, Location::End))
        );
        assert_eq!(
            "X.X.",
            canonicalize_suffix(&NamePart::from_word("Xx", true, Location::End))
        );
        assert_eq!(
            "X.X.",
            canonicalize_suffix(&NamePart::from_word("xx", true, Location::End))
        );
        assert_eq!(
            "Xx.",
            canonicalize_suffix(&NamePart::from_word("Xx.", true, Location::End))
        );
    }

    #[test]
    fn is_postfix_title_esq() {
        let part = NamePart::from_word("esq", true, Location::Start);
        assert!(is_postfix_title(&part, true));
    }

    #[test]
    fn is_postfix_title_et_al() {
        let parts: Vec<_> = NamePart::all_from_text("et al", true, Location::Start).collect();
        for part in parts {
            assert!(is_postfix_title(&part, true));
        }
    }

    #[test]
    fn is_postfix_title_abbr() {
        let part = NamePart::from_word("asd.", true, Location::Start);
        assert!(is_postfix_title(&part, true));
    }

    #[test]
    fn is_postfix_title_initialism() {
        let part = NamePart::from_word("a.s.d.", true, Location::Start);
        assert!(is_postfix_title(&part, false));
        assert!(!is_postfix_title(&part, true));
    }

    #[test]
    fn find_prefix_len_none() {
        let parts: Vec<_> = NamePart::all_from_text("Jane Doe", true, Location::Start).collect();
        let prefix = find_prefix_len(&parts);
        assert_eq!(
            "Jane Doe",
            parts[prefix..]
                .iter()
                .fold("".to_string(), |s, p| s + " " + p.word)
                .trim()
        );
    }

    #[test]
    fn find_prefix_len_abbr() {
        let parts: Vec<_> =
            NamePart::all_from_text("Dr. Jane Doe", true, Location::Start).collect();
        let prefix = find_prefix_len(&parts);
        assert_eq!(
            "Jane Doe",
            parts[prefix..]
                .iter()
                .fold("".to_string(), |s, p| s + " " + p.word)
                .trim()
        );
    }

    #[test]
    fn find_prefix_len_multi_abbr() {
        let parts: Vec<_> =
            NamePart::all_from_text("Revd. Dr. Jane Doe", true, Location::Start).collect();
        let prefix = find_prefix_len(&parts);
        assert_eq!(
            "Jane Doe",
            parts[prefix..]
                .iter()
                .fold("".to_string(), |s, p| s + " " + p.word)
                .trim()
        );
    }

    #[test]
    fn find_prefix_len_word() {
        let parts: Vec<_> =
            NamePart::all_from_text("Lady Jane Doe", true, Location::Start).collect();
        let prefix = find_prefix_len(&parts);
        assert_eq!(
            "Jane Doe",
            parts[prefix..]
                .iter()
                .fold("".to_string(), |s, p| s + " " + p.word)
                .trim()
        );
    }

    #[test]
    fn find_prefix_len_multi_word() {
        let parts: Vec<_> =
            NamePart::all_from_text("1st (B) Ltc Jane Doe", true, Location::Start).collect();
        let prefix = find_prefix_len(&parts);
        assert_eq!(
            "Jane Doe",
            parts[prefix..]
                .iter()
                .fold("".to_string(), |s, p| s + " " + p.word)
                .trim()
        );
    }

    #[test]
    fn find_prefix_len_short() {
        let parts: Vec<_> = NamePart::all_from_text("Dr. Doe", true, Location::Start).collect();
        let prefix = find_prefix_len(&parts);
        assert_eq!(
            "Doe",
            parts[prefix..]
                .iter()
                .fold("".to_string(), |s, p| s + " " + p.word)
                .trim()
        );
    }
}
