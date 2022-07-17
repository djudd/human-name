use super::features::*;
use super::namecase;
use super::segment::{Segment, Segments};
use super::surname;
use decomposition::combining_chars;
use phf::phf_set;
use std::borrow::Cow;
use std::iter::Peekable;

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum Location {
    Start,
    Middle,
    End,
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
    pub counts: CharacterCounts,
    pub category: Category<'a>,
}

pub struct NameParts<'a> {
    segments: Peekable<Segments<'a>>,
    location: Location,
    trust_capitalization: bool,
}

impl<'a> Iterator for NameParts<'a> {
    type Item = NamePart<'a>;

    fn next(&mut self) -> Option<NamePart<'a>> {
        self.segments.next().map(|Segment { word, counts }| {
            let location = if self.location == Location::Start {
                self.location = Location::Middle;
                Location::Start
            } else if self.segments.peek().is_none() {
                Location::End
            } else {
                Location::Middle
            };

            NamePart::from_word_and_counts(word, counts, self.trust_capitalization, location)
        })
    }
}

impl<'a> NamePart<'a> {
    pub fn all_from_text(
        text: &'a str,
        trust_capitalization: bool,
        location: Location,
    ) -> NameParts<'a> {
        NameParts {
            segments: Segments::from_text(text).peekable(),
            location,
            trust_capitalization,
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
        } = counts;

        debug_assert!(alpha > 0 || word == "&");

        let namecased = || {
            if trust_capitalization && starts_with_uppercase(word) {
                Cow::Borrowed(word)
            } else {
                let might_be_particle = location == Location::Middle;
                Cow::Owned(namecase::namecase(
                    word,
                    chars == ascii_alpha,
                    might_be_particle,
                ))
            }
        };

        let category = if chars == 1 {
            if ascii_alpha == chars {
                Category::Initials
            } else if upper > 0 {
                Category::Name(Cow::Borrowed(word))
            } else if alpha > 0 {
                Category::Name(Cow::Owned(word.to_uppercase()))
            } else {
                Category::Other
            }
        } else if word.ends_with('.') {
            if alpha >= 2 && has_sequential_alphas(word) {
                Category::Abbreviation
            } else {
                Category::Initials
            }
        } else if chars - alpha > 2 && chars - alpha - combining_chars(word) as u8 > 2 {
            Category::Other
        } else if trust_capitalization && alpha == upper {
            if chars <= 5 || (ascii_alpha > 0 && has_no_vowels(word)) {
                Category::Initials
            } else {
                Category::Name(namecased())
            }
        } else if ascii_alpha > 0 && has_no_vowels(word) {
            if location == Location::End
                && surname::is_vowelless_surname(word, trust_capitalization)
            {
                Category::Name(namecased())
            } else if chars <= 5 {
                Category::Initials
            } else {
                Category::Other
            }
        } else if chars == 2 && !trust_capitalization && !TWO_LETTER_GIVEN_NAMES.contains(word) {
            Category::Initials
        } else {
            Category::Name(namecased())
        };

        NamePart {
            word,
            counts,
            category,
        }
    }

    #[inline]
    pub fn is_initials(&self) -> bool {
        self.category == Category::Initials
    }

    #[inline]
    pub fn is_namelike(&self) -> bool {
        matches!(self.category, Category::Name(_))
    }

    // Called on Initials and also on given or middle Names
    pub fn with_initials<F>(&self, mut f: F)
    where
        F: FnMut(char),
    {
        match self.category {
            Category::Name(ref namecased) if !namecased.contains('-') && self.counts.upper > 0 => {
                f(namecased.chars().next().unwrap())
            }
            Category::Name(ref namecased) => namecased
                .split('-')
                .filter_map(|w| w.chars().find(|c| c.is_alphabetic()))
                // It's unclear whether we would want to look at multiple characters
                // if the lowercase character maps to multiple uppercase; we don't
                // currently have any known real-life example data.
                .filter_map(|c| c.to_uppercase().next())
                .for_each(f),
            Category::Initials if self.counts.upper == self.counts.chars => {
                self.word.chars().for_each(f)
            }
            Category::Initials => {
                for c in self.word.chars() {
                    if c.is_uppercase() {
                        f(c)
                    } else if c.is_alphabetic() {
                        // It's unclear whether we would want to look at multiple characters
                        // if the lowercase character maps to multiple uppercase; we don't
                        // currently have any known real-life example data.
                        f(c.to_uppercase().next().unwrap())
                    }
                }
            }
            _ => panic!("Called extract_initials on {:?}", self),
        }
    }

    // Normally called on a Name, but may be called on Initials if part was mis-categorized
    pub fn with_namecased<F>(&self, mut f: F)
    where
        F: FnMut(&str),
    {
        match self.category {
            Category::Name(ref namecased) => f(namecased),
            Category::Initials
                if self.counts.upper == 1
                    && (self.counts.alpha == 1 || starts_with_uppercase(self.word))
                    && (self.word != "Y" && self.word != "E") =>
            {
                f(self.word)
            }
            Category::Initials => {
                let namecased = namecase::namecase(
                    self.word,
                    self.counts.chars == self.counts.ascii_alpha,
                    true,
                );
                f(&namecased)
            }
            _ => panic!("Called extract_initials on {:?}", self),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "bench")]
    use test::{black_box, Bencher};

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

    #[cfg(feature = "bench")]
    #[bench]
    fn all_from_text_simple(b: &mut Bencher) {
        b.iter(|| black_box(NamePart::all_from_text("John Doe", true, Location::Start).count()))
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn all_from_text_initials(b: &mut Bencher) {
        b.iter(|| black_box(NamePart::all_from_text("J. Doe", true, Location::Start).count()))
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn all_from_text_nonascii(b: &mut Bencher) {
        b.iter(|| black_box(NamePart::all_from_text("이용희", false, Location::Start).count()))
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn all_from_text_all_caps(b: &mut Bencher) {
        b.iter(|| black_box(NamePart::all_from_text("JOHN DOE", false, Location::Start).count()))
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

    #[cfg(feature = "bench")]
    #[bench]
    fn from_word_simple(b: &mut Bencher) {
        let name = "Jonathan";
        let counts = categorize_chars(name);
        b.iter(|| {
            black_box(NamePart::from_word_and_counts(
                name,
                counts.clone(),
                true,
                Location::Start,
            ))
        })
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn from_word_initials(b: &mut Bencher) {
        let name = "J.";
        let counts = categorize_chars(name);
        b.iter(|| {
            black_box(NamePart::from_word_and_counts(
                name,
                counts.clone(),
                true,
                Location::Start,
            ))
        })
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn from_word_nonascii(b: &mut Bencher) {
        let name = "희";
        let counts = categorize_chars(name);
        b.iter(|| {
            black_box(NamePart::from_word_and_counts(
                name,
                counts.clone(),
                false,
                Location::Start,
            ))
        })
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn from_word_all_caps(b: &mut Bencher) {
        let name = "JONATHAN";
        let counts = categorize_chars(name);
        b.iter(|| {
            black_box(NamePart::from_word_and_counts(
                name,
                counts.clone(),
                false,
                Location::Start,
            ))
        })
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
