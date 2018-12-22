use super::utils::*;
use super::surname;
use super::namecase;
use std::borrow::Cow;
use phf;
use unicode_segmentation::UnicodeSegmentation;

// If Start and End overlap, use End
#[derive(Eq,PartialEq,Debug)]
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

impl <'a>NameParts<'a> {
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

impl <'a>Iterator for NameParts<'a> {
    type Item = NamePart<'a>;

    fn next(&mut self) -> Option<NamePart<'a>> {
        // Skip any leading whitespace
        self.text = self.text.trim_start();

        if self.text.is_empty() {
            return None;
        }

        // Now look for the next whitespace that remains
        let next_whitespace = self.text.find(char::is_whitespace).unwrap_or_else(|| self.text.len());
        let next_inner_period = self.text[0..next_whitespace].find('.');
        let next_boundary = match next_inner_period {
            Some(i) => i + 1,
            None => next_whitespace,
        };

        let word = &self.text[0..next_boundary];

        if word == "&" {
            // Special case: only allowed word without alphabetical characters
            self.text = &self.text[next_boundary..];
            Some(NamePart {
                word,
                chars: 1,
                category: Category::Other,
                namecased: Cow::Borrowed(word),
            })
        } else if !word.chars().any(char::is_alphabetic) {
            // Not a word, skip it by recursing
            self.text = &self.text[next_boundary..];
            self.next()
        } else if !word.chars().any(|c| c.is_ascii()) {
            // For non-ASCII, we defer to the unicode_segmentation library
            let (next_word_boundary, subword) = word.split_word_bound_indices()
                                                    .find(|&(_, subword)| {
                                                        subword.chars().any(char::is_alphabetic)
                                                    })
                                                    .unwrap();
            self.text = &self.text[next_word_boundary + subword.len()..];
            Some(NamePart::from_word(subword, self.trust_capitalization, self.next_location()))
        } else {
            // For ASCII, we split on whitespace and periods only
            self.text = &self.text[next_boundary..];
            Some(NamePart::from_word(word, self.trust_capitalization, self.next_location()))
        }
    }
}

#[derive(Eq,PartialEq,Debug)]
pub enum Category {
    Name,
    Initials,
    Abbreviation,
    Other,
}

#[derive(Debug)]
pub struct NamePart<'a> {
    pub word: &'a str,
    pub chars: usize,
    pub category: Category,
    pub namecased: Cow<'a, str>,
}

impl <'a>NamePart<'a> {

    pub fn all_from_text(text: &str, trust_capitalization: bool, location: Location) -> NameParts {
        NameParts {
            text,
            trust_capitalization,
            location,
        }
    }

    #[allow(clippy::if_same_then_else)]
    #[allow(clippy::collapsible_if)]
    pub fn from_word(word: &str, trust_capitalization: bool, location: Location) -> NamePart {
        let chars = word.chars().count();
        debug_assert!(chars > 0);

        let ascii = word.chars().all(|c| c.is_ascii());
        debug_assert!(word.chars().any(char::is_alphabetic));

        let category = if chars == 1 && ascii {
            Category::Initials
        } else if chars == 1 {
            Category::Name
        } else if word.ends_with('.') {
            if chars > 2 && has_sequential_alphas(word) {
                Category::Abbreviation
            } else {
                Category::Initials
            }
        } else if word.chars()
                              .filter(|c| !c.is_alphabetic() && !is_combining(*c))
                              .count() > 2 {
            Category::Other
        } else if is_missing_vowels(word) {
            if trust_capitalization &&
               word.chars().all(|c| !c.is_alphabetic() || c.is_uppercase()) {
                Category::Initials
            } else if location == Location::End &&
               surname::is_vowelless_surname(word, trust_capitalization) {
                Category::Name
            } else if chars <= 5 {
                Category::Initials
            } else {
                Category::Other
            }
        } else {
            if chars <= 5 && trust_capitalization &&
               word.chars().all(|c| !c.is_alphabetic() || c.is_uppercase()) {
                Category::Initials
            } else if chars == 2 && !trust_capitalization && !TWO_LETTER_GIVEN_NAMES.contains(word) {
                Category::Initials
            } else {
                Category::Name
            }
        };

        let namecased = if trust_capitalization && is_plausibly_capitalized(word) {
            Cow::Borrowed(word)
        } else {
            let might_be_particle = location == Location::Middle;
            Cow::Owned(namecase::namecase(word, might_be_particle))
        };

        NamePart {
            word,
            chars,
            category,
            namecased,
        }
    }

    #[inline]
    pub fn is_initials(&self) -> bool {
        self.category == Category::Initials
    }

    #[inline]
    pub fn is_namelike(&self) -> bool {
        self.category == Category::Name
    }

    #[inline]
    pub fn is_abbreviation(&self) -> bool {
        self.category == Category::Abbreviation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_word() {
        assert_eq!(1,
                   NamePart::all_from_text("John", true, Location::Start).count());
    }

    #[test]
    fn two_words() {
        assert_eq!(2,
                   NamePart::all_from_text("&* John Doe! ☃", true, Location::Start).count());
    }

    #[test]
    fn only_junk() {
        assert_eq!(0,
                   NamePart::all_from_text(" ... 23 ", true, Location::Start).count());
    }

    #[test]
    fn single_ascii() {
        assert_eq!(Category::Initials,
                   NamePart::from_word("I", true, Location::Start).category);
    }

    #[test]
    fn single_han() {
        assert_eq!(Category::Name,
                   NamePart::from_word("鄭", true, Location::Start).category);
    }

    #[test]
    fn abbreviated_ascii() {
        assert_eq!(Category::Initials,
                   NamePart::from_word("I.", true, Location::Start).category);
    }

    #[test]
    fn abbreviated_double_ascii() {
        assert_eq!(Category::Abbreviation,
                   NamePart::from_word("MI.", true, Location::Start).category);
    }

    #[test]
    fn double_abbreviated_double_ascii() {
        assert_eq!(Category::Initials,
                   NamePart::from_word("M.I.", true, Location::Start).category);
    }

    #[test]
    fn junk() {
        assert_eq!(Category::Other,
                   NamePart::from_word("503(a)", true, Location::Start).category);
    }

    #[test]
    fn no_vowels() {
        assert_eq!(Category::Initials,
                   NamePart::from_word("JM", true, Location::Start).category);
        assert_eq!(Category::Initials,
                   NamePart::from_word("jm", true, Location::Start).category);
        assert_eq!(Category::Initials,
                   NamePart::from_word("JM", false, Location::Start).category);
        assert_eq!(Category::Initials,
                   NamePart::from_word("JMMMMM", true, Location::Start).category);
        assert_eq!(Category::Other,
                   NamePart::from_word("jmmmmm", true, Location::Start).category);
        assert_eq!(Category::Other,
                   NamePart::from_word("JMMMMM", false, Location::Start).category);
    }

    #[test]
    fn vowelless_surname() {
        assert_eq!(Category::Initials,
                   NamePart::from_word("NG", true, Location::Start).category);
        assert_eq!(Category::Initials,
                   NamePart::from_word("Ng", true, Location::Start).category);
        assert_eq!(Category::Name,
                   NamePart::from_word("Ng", true, Location::End).category);
        assert_eq!(Category::Name,
                   NamePart::from_word("NG", false, Location::End).category);
        assert_eq!(Category::Initials,
                   NamePart::from_word("NG", true, Location::End).category);
    }

    #[test]
    fn word() {
        assert_eq!(Category::Initials,
                   NamePart::from_word("JEM", true, Location::Start).category);
        assert_eq!(Category::Name,
                   NamePart::from_word("Jem", true, Location::Start).category);
        assert_eq!(Category::Name,
                   NamePart::from_word("JEM", false, Location::Start).category);
    }

    #[test]
    fn two_letters() {
        assert_eq!(Category::Name,
                   NamePart::from_word("Al", true, Location::Start).category);
        assert_eq!(Category::Initials,
                   NamePart::from_word("AL", true, Location::Start).category);
        assert_eq!(Category::Name,
                   NamePart::from_word("AL", false, Location::Start).category);
        assert_eq!(Category::Name,
                   NamePart::from_word("At", true, Location::Start).category);
        assert_eq!(Category::Initials,
                   NamePart::from_word("AT", true, Location::Start).category);
        assert_eq!(Category::Initials,
                   NamePart::from_word("AT", false, Location::Start).category);
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
