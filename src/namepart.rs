use super::namecase;
use super::surname;
use super::utils::*;
use phf;
use std::borrow::Cow;
use unicode_segmentation::UnicodeSegmentation;

// If Start and End overlap, use End
#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum Location {
    Start,
    Middle,
    End,
}

pub struct NameParts<'a> {
    text: &'a str,
    trust_capitalization: bool,
    location: Location,
}

impl<'a> NameParts<'a> {
    fn next_location(&mut self) -> Location {
        if self.location == Location::Middle {
            // If the whole section is in the middle, so are all parts
            Location::Middle
        } else if self.location == Location::Start {
            self.location = Location::Middle;
            Location::Start
        } else if self.text.chars().find(|c| c.is_alphabetic()).is_none() {
            Location::End
        } else {
            Location::Middle
        }
    }
}

impl<'a> Iterator for NameParts<'a> {
    type Item = NamePart<'a>;

    fn next(&mut self) -> Option<NamePart<'a>> {
        // Skip any leading whitespace
        self.text = self.text.trim_start();

        if self.text.is_empty() {
            return None;
        }

        // Now look for the next whitespace that remains
        let next_whitespace = self
            .text
            .find(char::is_whitespace)
            .unwrap_or_else(|| self.text.len());
        let next_inner_period = self.text[0..next_whitespace].find('.');
        let next_boundary = match next_inner_period {
            Some(i) => i + 1,
            None => next_whitespace,
        };

        let word = &self.text[0..next_boundary];
        if word.len() > u8::max_value() as usize {
            self.text = &self.text[next_boundary..];
            self.next()
        } else if word == "&" {
            // Special case: only allowed word without alphabetical characters
            self.text = &self.text[next_boundary..];
            Some(NamePart {
                word,
                chars: 1,
                category: Category::Other,
            })
        } else {
            let counts = categorize_chars(word);

            if counts.alpha == 0 {
                // Not a word, skip it by recursing
                self.text = &self.text[next_boundary..];
                self.next()
            } else if counts.ascii_alpha == 0 {
                // For non-ASCII, we defer to the unicode_segmentation library
                let (next_word_boundary, subword) = word
                    .split_word_bound_indices()
                    .find(|&(_, subword)| subword.chars().any(char::is_alphabetic))
                    .unwrap();
                self.text = &self.text[next_word_boundary + subword.len()..];
                Some(NamePart::from_word(
                    subword,
                    self.trust_capitalization,
                    self.next_location(),
                ))
            } else {
                // For ASCII, we split on whitespace and periods only
                self.text = &self.text[next_boundary..];
                Some(NamePart::from_word_and_counts(
                    word,
                    counts,
                    self.trust_capitalization,
                    self.next_location(),
                ))
            }
        }
    }
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum Category<'a> {
    Name(Cow<'a, str>),
    Initials,
    Abbreviation,
    Other,
}

#[derive(Debug, Clone)]
pub struct NamePart<'a> {
    pub word: &'a str,
    pub chars: u8,
    pub category: Category<'a>,
}

impl<'a> NamePart<'a> {
    pub fn all_from_text(text: &str, trust_capitalization: bool, location: Location) -> NameParts {
        NameParts {
            text,
            trust_capitalization,
            location,
        }
    }

    pub fn from_word(word: &str, trust_capitalization: bool, location: Location) -> NamePart {
        NamePart::from_word_and_counts(word, categorize_chars(word), trust_capitalization, location)
    }

    #[allow(clippy::if_same_then_else)]
    pub fn from_word_and_counts(
        word: &str,
        counts: CharacterCounts,
        trust_capitalization: bool,
        location: Location,
    ) -> NamePart {
        let CharacterCounts {
            chars,
            alpha,
            upper,
            ascii_alpha,
            ascii_vowels,
        } = counts;

        debug_assert!(alpha > 0);

        let all_upper = alpha == upper;

        let namecased = || {
            if upper == 1
                && (all_upper
                    || (trust_capitalization && word.chars().take(1).all(char::is_uppercase)))
            {
                Cow::Borrowed(word)
            } else {
                let might_be_particle = location == Location::Middle;
                Cow::Owned(namecase::namecase(word, might_be_particle))
            }
        };

        let category = if chars == 1 {
            if ascii_alpha == chars {
                Category::Initials
            } else {
                Category::Name(namecased())
            }
        } else if word.ends_with('.') {
            if alpha >= 2 && has_sequential_alphas(word) {
                Category::Abbreviation
            } else {
                Category::Initials
            }
        } else if chars - alpha > 2
            && chars - alpha - word.chars().filter(|c| is_combining(*c)).count() as u8 > 2
        {
            Category::Other
        } else if ascii_alpha > 0 && ascii_vowels == 0 {
            if trust_capitalization && all_upper {
                Category::Initials
            } else if location == Location::End
                && surname::is_vowelless_surname(word, trust_capitalization)
            {
                Category::Name(namecased())
            } else if chars <= 5 {
                Category::Initials
            } else {
                Category::Other
            }
        } else if chars <= 5 && trust_capitalization && all_upper {
            Category::Initials
        } else if chars == 2 && !trust_capitalization && !TWO_LETTER_GIVEN_NAMES.contains(word) {
            Category::Initials
        } else {
            Category::Name(namecased())
        };

        NamePart {
            word,
            chars,
            category,
        }
    }

    #[inline]
    pub fn is_initials(&self) -> bool {
        self.category == Category::Initials
    }

    #[inline]
    pub fn is_namelike(&self) -> bool {
        match self.category {
            Category::Name(_) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_word() {
        assert_eq!(
            1,
            NamePart::all_from_text("John", true, Location::Start).count()
        );
    }

    #[test]
    fn two_words() {
        assert_eq!(
            2,
            NamePart::all_from_text("&* John Doe! ☃", true, Location::Start).count()
        );
    }

    #[test]
    fn only_junk() {
        assert_eq!(
            0,
            NamePart::all_from_text(" ... 23 ", true, Location::Start).count()
        );
    }

    #[test]
    fn single_ascii() {
        assert!(NamePart::from_word("I", true, Location::Start).is_initials());
    }

    #[test]
    fn single_han() {
        assert!(NamePart::from_word("鄭", true, Location::Start).is_namelike());
    }

    #[test]
    fn abbreviated_ascii() {
        assert!(NamePart::from_word("I.", true, Location::Start).is_initials());
    }

    #[test]
    fn abbreviated_double_ascii() {
        assert_eq!(
            Category::Abbreviation,
            NamePart::from_word("MI.", true, Location::Start).category
        );
    }

    #[test]
    fn double_abbreviated_double_ascii() {
        assert!(NamePart::from_word("M.I.", true, Location::Start).is_initials());
    }

    #[test]
    fn junk() {
        assert_eq!(
            Category::Other,
            NamePart::from_word("503(a)", true, Location::Start).category
        );
    }

    #[test]
    fn no_vowels() {
        assert!(NamePart::from_word("JM", true, Location::Start).is_initials());
        assert!(NamePart::from_word("jm", true, Location::Start).is_initials());
        assert!(NamePart::from_word("JM", false, Location::Start).is_initials());
        assert!(NamePart::from_word("JMMMMM", true, Location::Start).is_initials());
        assert_eq!(
            Category::Other,
            NamePart::from_word("jmmmmm", true, Location::Start).category
        );
        assert_eq!(
            Category::Other,
            NamePart::from_word("JMMMMM", false, Location::Start).category
        );
    }

    #[test]
    fn vowelless_surname() {
        assert!(NamePart::from_word("NG", true, Location::Start).is_initials());
        assert!(NamePart::from_word("Ng", true, Location::Start).is_initials());
        assert!(NamePart::from_word("Ng", true, Location::End).is_namelike());
        assert!(NamePart::from_word("NG", false, Location::End).is_namelike());
        assert!(NamePart::from_word("NG", true, Location::End).is_initials());
    }

    #[test]
    fn word() {
        assert!(NamePart::from_word("JEM", true, Location::Start).is_initials());
        assert!(NamePart::from_word("Jem", true, Location::Start).is_namelike());
        assert!(NamePart::from_word("JEM", false, Location::Start).is_namelike());
    }

    #[test]
    fn two_letters() {
        assert!(NamePart::from_word("Al", true, Location::Start).is_namelike());
        assert!(NamePart::from_word("AL", true, Location::Start).is_initials());
        assert!(NamePart::from_word("AL", false, Location::Start).is_namelike());
        assert!(NamePart::from_word("At", true, Location::Start).is_namelike());
        assert!(NamePart::from_word("AT", true, Location::Start).is_initials());
        assert!(NamePart::from_word("AT", false, Location::Start).is_initials());
    }
}

// Everything with a vowel reasonably popular in the Social Security data:
// https://www.ssa.gov/oact/babynames/limits.html
static TWO_LETTER_GIVEN_NAMES: phf::Set<&'static str> = phf_set! {
    "Jo",
    "JO",
    "jo",
    "Ty",
    "TY",
    "ty",
    "Ed",
    "ED",
    "ed",
    "Al",
    "AL",
    "al",
    "Bo",
    "BO",
    "bo",
    "Lu",
    "LU",
    "lu",
    "Cy",
    "CY",
    "cy",
    "An",
    "AN",
    "an",
    "La",
    "LA",
    "la",
    "Aj",
    "AJ",
    "aj",
    "Le",
    "LE",
    "le",
    "Om",
    "OM",
    "om",
    "Pa",
    "PA",
    "pa",
    "De",
    "DE",
    "de",
    "Ky",
    "KY",
    "ky",
    "My",
    "MY",
    "my",
    "Vy",
    "VY",
    "vy",
    "Vi",
    "VI",
    "vi",
    "Ka",
    "KA",
    "ka",
    "Sy",
    "SY",
    "sy",
    "Vu",
    "VU",
    "vu",
    "Yu",
    "YU",
    "yu",
    "Mi",
    "MI",
    "mi",
    "Su",
    "SU",
    "su",
    "Ma",
    "MA",
    "ma",
    "Ha",
    "HA",
    "ha",
    "Ki",
    "KI",
    "ki",
    "Tu",
    "TU",
    "tu",
    "Ji",
    "JI",
    "ji",
    "Ja",
    "JA",
    "ja",
    "Ly",
    "LY",
    "ly",
    "Li",
    "LI",
    "li",
    "Ai",
    "AI",
    "ai",
    "Ry",
    "RY",
    "ry",
    "Ab",
    "AB",
    "ab",
    "Ho",
    "HO",
    "ho",
    "Da",
    "DA",
    "da",
    "Oz",
    "OZ",
    "oz",
    "El",
    "EL",
    "el",
    "Na",
    "NA",
    "na",
    "Yi",
    "YI",
    "yi",
    "Em",
    "EM",
    "em",
    "Di",
    "DI",
    "di",
    "Go",
    "GO",
    "go",
    "Ev",
    "EV",
    "ev",
    "Mo",
    "MO",
    "mo",
    "Lo",
    "LO",
    "lo",
    "Ra",
    "RA",
    "ra",
    "Do",
    "DO",
    "do",
    "Gi",
    "GI",
    "gi",
};
