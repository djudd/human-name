//! A library for parsing and comparing human names.
//!
//! See the documentation of the `Name` struct for details.

#![doc(html_root_url = "https://djudd.github.io/human-name/")]
#![feature(libc)]
#![feature(plugin)]
#![feature(test)]
#![feature(proc_macro_hygiene)]

extern crate phf;
extern crate smallstr;
extern crate smallvec;
extern crate test;
extern crate unicode_normalization;
extern crate unicode_segmentation;
extern crate unidecode;

#[cfg(feature = "serialization")]
extern crate serde;
#[cfg(feature = "serialization")]
#[macro_use]
extern crate serde_derive;

#[macro_use]
mod utils;
mod comparison;
mod namecase;
mod namepart;
mod nickname;
mod parse;
mod segment;
mod suffix;
mod surname;
mod title;
mod web_match;

pub mod external;

#[cfg(feature = "name_eq_hash")]
mod eq_hash;

#[cfg(feature = "serialization")]
mod serialization;

use namepart::NamePart;
use smallstr::SmallString;
use smallvec::SmallVec;
use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::slice::Iter;
use utils::{lowercase_if_alpha, normalize_nfkd_hyphens_spaces, transliterate};

pub const MAX_NAME_LEN: usize = 1024;
pub const MAX_SEGMENT_LEN: usize = segment::MAX_LEN;
pub const MAX_SEGMENTS: usize = parse::MAX_WORDS;

/// Represents a parsed human name.
///
/// Guaranteed to contain (what we think is) a surname, a first initial, and
/// nothing more. May also contain given & middle names, middle initials, and/or
/// a generational suffix.
///
/// Construct a Name using `parse`:
///
/// ```
/// use human_name::Name;
///
/// let name = Name::parse("Jane Doe").unwrap();
/// ```
///
/// Once you have a Name, you may extract is components, convert it to JSON,
/// or compare it with another Name to see if they are consistent with representing
/// the same person (see docs on `consistent_with` for details).
#[derive(Clone, Debug)]
pub struct Name {
    text: SmallString<[u8; 36]>,
    word_indices_in_text: SmallVec<[Range<usize>; 5]>,
    surname_index: usize,
    generation_from_suffix: Option<u8>,
    initials: SmallString<[u8; 8]>,
    word_indices_in_initials: SmallVec<[Range<usize>; 3]>,
    pub hash: u64,
}

impl Name {
    /// Parses a string represent a single person's full name into a canonical
    /// representation.
    ///
    /// # Examples
    /// ```
    /// use human_name::Name;
    ///
    /// let name = Name::parse("Jane Doe").unwrap();
    /// assert_eq!("Doe", name.surname());
    /// assert_eq!(Some("Jane"), name.given_name());
    ///
    /// let name = Name::parse("Doe, J").unwrap();
    /// assert_eq!("Doe", name.surname());
    /// assert_eq!(None, name.given_name());
    /// assert_eq!('J', name.first_initial());
    ///
    /// let name = Name::parse("Dr. Juan Alberto T. Velasquez y Garcia III").unwrap();
    /// assert_eq!("Velasquez y Garcia", name.surname());
    /// assert_eq!(Some("Juan"), name.given_name());
    /// assert_eq!(Some("AT"), name.middle_initials());
    /// assert_eq!(Some("III"), name.suffix());
    /// ```
    ///
    /// # Supported formats
    ///
    /// Supports a variety of formats, including prefix and postfix titles,
    /// parenthesized nicknames, initials with and without periods, and sort
    /// order ("Doe, Jane"). Makes use of heuristics based on case when
    /// applicable (e.g., "AL Doe" is parsed as "A. L. Doe", while "Al Doe" is
    /// parsed as a given name and surname), as well as _small_ sets of known
    /// particles, conjunctions, titles, etc.
    ///
    /// # Limitations
    ///
    /// Errs on the side of producing parse output rather than giving up, so
    /// this function is _not_ suitable as a way of guessing whether a given
    /// string actually represents a name.
    ///
    /// However, success requires at least an apparent surname and first initial.
    /// Single-word names cannot be parsed (you may or may not wish to assume
    /// they are given names).
    ///
    /// Does not preserve titles (other than generational suffixes such as "III")
    /// or nicknames. Does not handle plural forms specially: "Mr. & Mrs. John
    /// Doe" will be parsed as "John Doe", and "Jane Doe, et al" will be parsed
    /// as "Jane Doe".
    ///
    /// Works best on Latin names - i.e., data from North or South America or
    /// Europe. Does not understand surname-first formats without commas: "Kim
    /// Il-sung" will be parsed as having the first name "Kim".
    ///
    /// Handles non-Latin unicode strings, but without any particular intelligence.
    /// Attempts at least to fail nicely, such that either `parse` returns `None`,
    /// or calling `display_full()` on the parsed result returns the input,
    /// plus or minus whitespace.
    ///
    /// Of course, [there is no perfect algorithm](http://www.kalzumeus.com/2010/06/17/falsehoods-programmers-believe-about-names/)
    /// for canonicalizing names. The goal here is to do the best we can without
    /// large statistical models.
    pub fn parse(name: &str) -> Option<Name> {
        if name.len() >= MAX_NAME_LEN {
            return None;
        }

        let name = normalize_nfkd_hyphens_spaces(&name);
        let name = nickname::strip_nickname(&name);

        let (words, surname_index, generation_from_suffix) = parse::parse(&*name)?;

        let mut name =
            Name::initialize_struct(&words, surname_index, generation_from_suffix, name.len());

        let mut s = DefaultHasher::new();
        name.surname_hash(&mut s);
        name.hash = s.finish();

        Some(name)
    }

    fn initialize_struct(
        words: &[NamePart],
        surname_index: usize,
        generation_from_suffix: Option<u8>,
        name_len: usize,
    ) -> Name {
        let last_word = words.len() - 1;

        let mut text = SmallString::with_capacity(name_len);
        let mut initials = SmallString::with_capacity(surname_index);

        let mut surname_index_in_names = surname_index;
        let mut word_indices_in_initials = SmallVec::with_capacity(surname_index);
        let mut word_indices_in_text = SmallVec::with_capacity(words.len());

        for (i, word) in words.iter().enumerate() {
            if word.is_initials() && i < surname_index {
                word.with_initials(|c| {
                    text.push(c);
                    text.push_str(". ");

                    initials.push(c);
                });

                surname_index_in_names -= 1;
            } else {
                let prior_len = text.len();
                word.with_namecased(|s| text.push_str(s));
                word_indices_in_text.push(prior_len..text.len());

                if i < last_word {
                    text.push(' ');

                    if i < surname_index {
                        debug_assert!(word.is_namelike());

                        let prior_len = initials.len();
                        word.with_initials(|c| initials.push(c));
                        word_indices_in_initials.push(prior_len..initials.len());
                    }
                }
            }
        }

        if let Some(suffix) = generation_from_suffix {
            text.push_str(", ");
            text.push_str(suffix::display_generational_suffix(suffix));
        }

        debug_assert!(!text.is_empty(), "Names are empty!");
        debug_assert!(!initials.is_empty(), "Initials are empty!");

        text.shrink_to_fit();
        word_indices_in_text.shrink_to_fit();
        initials.shrink_to_fit();
        word_indices_in_initials.shrink_to_fit();

        Name {
            text,
            word_indices_in_text,
            surname_index: surname_index_in_names,
            generation_from_suffix,
            initials,
            word_indices_in_initials,
            hash: 0,
        }
    }

    /// First initial (always present)
    pub fn first_initial(&self) -> char {
        self.initials.chars().nth(0).unwrap()
    }

    /// Given name as a string, if present
    ///
    /// ```
    /// use human_name::Name;
    ///
    /// let name = Name::parse("Jane Doe").unwrap();
    /// assert_eq!(Some("Jane"), name.given_name());
    ///
    /// let name = Name::parse("J. Doe").unwrap();
    /// assert_eq!(None, name.given_name());
    /// ```
    pub fn given_name(&self) -> Option<&str> {
        self.given_iter().nth(0)
    }

    /// Does this person use a middle name in place of their given name?
    ///
    /// ```
    /// use human_name::Name;
    ///
    /// let name = Name::parse("Jane Doe").unwrap();
    /// assert!(!name.goes_by_middle_name());
    ///
    /// let name = Name::parse("J. Doe").unwrap();
    /// assert!(!name.goes_by_middle_name());
    ///
    /// let name = Name::parse("T Boone Pickens").unwrap();
    /// assert!(name.goes_by_middle_name());
    /// ```
    pub fn goes_by_middle_name(&self) -> bool {
        self.word_indices_in_initials
            .iter()
            .take(1)
            .any(|r| r.start > 0)
    }

    /// First and middle initials as a string (always present)
    ///
    /// ```
    /// use human_name::Name;
    ///
    /// let name = Name::parse("Jane Doe").unwrap();
    /// assert_eq!("J", name.initials());
    ///
    /// let name = Name::parse("James T. Kirk").unwrap();
    /// assert_eq!("JT", name.initials());
    /// ```
    pub fn initials(&self) -> &str {
        &self.initials
    }

    /// Middle names as an array of words, if present
    pub fn middle_names(&self) -> Option<SmallVec<[&str; 3]>> {
        self.middle_name_iter().map(|i| i.collect())
    }

    /// Middle names as a string, if present
    ///
    /// ```
    /// use human_name::Name;
    ///
    /// let name = Name::parse("Jane Doe").unwrap();
    /// assert_eq!(None, name.middle_name());
    ///
    /// let name = Name::parse("James T. Kirk").unwrap();
    /// assert_eq!(None, name.middle_name());
    ///
    /// let name = Name::parse("James Tiberius Kirk").unwrap();
    /// assert_eq!("Tiberius", name.middle_name().unwrap());
    ///
    /// let name = Name::parse("Able Baker Charlie Delta").unwrap();
    /// assert_eq!("Baker Charlie", name.middle_name().unwrap());
    /// ```
    pub fn middle_name(&self) -> Option<Cow<str>> {
        self.middle_name_iter().map(|i| i.join())
    }

    /// Middle initials as a string, if present
    ///
    /// ```
    /// use human_name::Name;
    ///
    /// let name = Name::parse("Jane Doe").unwrap();
    /// assert_eq!(None, name.middle_initials());
    ///
    /// let name = Name::parse("James T. Kirk").unwrap();
    /// assert_eq!("T", name.middle_initials().unwrap());
    ///
    /// let name = Name::parse("James Tiberius Kirk").unwrap();
    /// assert_eq!("T", name.middle_initials().unwrap());
    ///
    /// let name = Name::parse("Able Baker Charlie Delta").unwrap();
    /// assert_eq!("BC", name.middle_initials().unwrap());
    /// ```
    pub fn middle_initials(&self) -> Option<&str> {
        self.initials()
            .char_indices()
            .skip(1)
            .nth(0)
            .map(|(i, _)| &self.initials[i..])
    }

    /// Surname as a slice of words (always present)
    pub fn surnames(&self) -> SmallVec<[&str; 3]> {
        self.surname_iter().collect()
    }

    /// Surname as a string (always present)
    ///
    /// ```
    /// use human_name::Name;
    ///
    /// let name = Name::parse("Jane Doe").unwrap();
    /// assert_eq!("Doe", name.surname());
    ///
    /// let name = Name::parse("JOHN ALLEN Q DE LA MACDONALD JR").unwrap();
    /// assert_eq!("de la MacDonald", name.surname());
    /// ```
    pub fn surname(&self) -> &str {
        let start = self.word_indices_in_text[self.surname_index].start;
        let end = self.word_indices_in_text.last().unwrap().end;
        &self.text[start..end]
    }

    /// Generational suffix, if present
    pub fn suffix(&self) -> Option<&str> {
        self.generation_from_suffix
            .map(suffix::display_generational_suffix)
    }

    /// First initial (with period) and surname.
    ///
    /// ```
    /// use human_name::Name;
    ///
    /// let name = Name::parse("J. Doe").unwrap();
    /// assert_eq!("J. Doe", name.display_initial_surname());
    ///
    /// let name = Name::parse("James T. Kirk").unwrap();
    /// assert_eq!("J. Kirk", name.display_initial_surname());
    ///
    /// let name = Name::parse("JOHN ALLEN Q DE LA MACDONALD JR").unwrap();
    /// assert_eq!("J. de la MacDonald", name.display_initial_surname());
    /// ```
    pub fn display_initial_surname(&self) -> Cow<str> {
        if self.surname_index == 0
            && self.initials.len() == 1
            && self.generation_from_suffix.is_none()
        {
            Cow::Borrowed(&self.text)
        } else {
            Cow::Owned(format!("{}. {}", self.first_initial(), self.surname()))
        }
    }

    /// Given name and surname, if given name is known, otherwise first initial
    /// and surname.
    ///
    /// ```
    /// use human_name::Name;
    ///
    /// let name = Name::parse("J. Doe").unwrap();
    /// assert_eq!("J. Doe", name.display_first_last());
    ///
    /// let name = Name::parse("Jane Doe").unwrap();
    /// assert_eq!("Jane Doe", name.display_first_last());
    ///
    /// let name = Name::parse("James T. Kirk").unwrap();
    /// assert_eq!("James Kirk", name.display_first_last());
    ///
    /// let name = Name::parse("JOHN ALLEN Q DE LA MACDONALD JR").unwrap();
    /// assert_eq!("John de la MacDonald", name.display_first_last());
    /// ```
    pub fn display_first_last(&self) -> Cow<str> {
        if self.surname_index <= 1
            && self.initials.len() == 1
            && self.generation_from_suffix.is_none()
        {
            Cow::Borrowed(&self.text)
        } else if let Some(ref name) = self.given_name() {
            Cow::Owned(format!("{} {}", name, self.surname()))
        } else {
            self.display_initial_surname()
        }
    }

    /// Number of bytes in the full name as UTF-8 in NFKD normal form, including
    /// spaces and punctuation.
    ///
    /// ```
    /// use human_name::Name;
    ///
    /// let name = Name::parse("JOHN ALLEN Q DE LA MACDÖNALD JR").unwrap();
    /// assert_eq!("John Allen Q. de la MacDönald, Jr.".len(), name.byte_len());
    /// ```
    pub fn byte_len(&self) -> usize {
        self.text.len()
    }

    /// The full name, or as much of it as was preserved from the input,
    /// including given name, middle names, surname and suffix.
    ///
    /// ```
    /// use human_name::Name;
    ///
    /// let name = Name::parse("JOHN ALLEN Q DE LA MACDONALD JR").unwrap();
    /// assert_eq!("John Allen Q. de la MacDonald, Jr.", name.display_full());
    /// ```
    pub fn display_full(&self) -> &str {
        &self.text
    }

    /// Implements a hash for a name that is always identical for two names that
    /// may be consistent according to our matching algorithm.
    ///
    /// ### WARNING
    ///
    /// This hash function is prone to collisions!
    ///
    /// We can only use the last four alphabetical characters of the surname,
    /// because that's all we're guaranteed to use in the consistency test. That
    /// means if names are ASCII, we only have 19 bits of variability.
    ///
    /// That means if you are working with a lot of names and you expect surnames
    /// to be similar or identical, you might be better off avoiding hash-based
    /// datastructures (or using a custom hash and matching algorithm).
    ///
    /// We can't use more characters of the surname because we treat names as equal
    /// when one surname ends with the other and the smaller is at least four
    /// characters, to catch cases like "Iria Gayo" == "Iria del Río Gayo".
    ///
    /// We can't use the first initial because we might ignore it if someone goes
    /// by a middle name or nickname, or due to transliteration.
    pub fn surname_hash<H: Hasher>(&self, state: &mut H) {
        for c in self
            .surname_iter()
            .flat_map(|w| w.chars())
            .flat_map(transliterate)
            .rev()
            .filter_map(lowercase_if_alpha)
            .take(comparison::MIN_SURNAME_CHAR_MATCH)
        {
            c.hash(state);
        }
    }

    #[inline]
    fn surname_words(&self) -> usize {
        self.word_indices_in_text.len() - self.surname_index
    }

    #[inline]
    fn surname_iter(&self) -> Words {
        self.word_iter(self.surname_index..self.word_indices_in_text.len())
    }

    #[inline]
    fn middle_name_iter(&self) -> Option<Words> {
        if self.surname_index > 1 {
            Some(self.word_iter(1..self.surname_index))
        } else {
            None
        }
    }

    #[inline]
    fn given_iter(&self) -> Words {
        self.word_iter(0..self.surname_index)
    }

    #[inline]
    fn word_iter(&self, range: Range<usize>) -> Words {
        Words {
            text: &self.text,
            indices: self.word_indices_in_text[range].iter(),
        }
    }
}

struct Words<'a> {
    text: &'a str,
    indices: Iter<'a, Range<usize>>,
}

impl<'a> Words<'a> {
    pub fn join(mut self) -> Cow<'a, str> {
        match self.len() {
            0 => Cow::Borrowed(""),
            1 => Cow::Borrowed(self.next().unwrap()),
            _ => Cow::Owned(self.collect::<SmallVec<[&str; 5]>>().join(" ")),
        }
    }
}

impl<'a> Iterator for Words<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        self.indices.next().map(|range| &self.text[range.clone()])
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.indices.size_hint()
    }
}

impl<'a> DoubleEndedIterator for Words<'a> {
    fn next_back(&mut self) -> Option<&'a str> {
        self.indices
            .next_back()
            .map(|range| &self.text[range.clone()])
    }
}

impl<'a> ExactSizeIterator for Words<'a> {}

#[cfg(test)]
mod tests {
    use super::*;
    use test::{black_box, Bencher};

    #[bench]
    fn initialize_struct_initial_surname(b: &mut Bencher) {
        let name = "J. Doe";
        let (words, surname_index, generation) = parse::parse(&*name).unwrap();
        b.iter(|| {
            black_box(
                Name::initialize_struct(&words, surname_index, generation, name.len()).byte_len(),
            )
        })
    }

    #[bench]
    fn initialize_struct_first_last(b: &mut Bencher) {
        let name = "John Doe";
        let (words, surname_index, generation) = parse::parse(&*name).unwrap();
        b.iter(|| {
            black_box(
                Name::initialize_struct(&words, surname_index, generation, name.len()).byte_len(),
            )
        })
    }

    #[bench]
    fn initialize_struct_complex(b: &mut Bencher) {
        let name = "John Allen Q.R. de la MacDonald Jr.";
        let (words, surname_index, generation) = parse::parse(&*name).unwrap();
        b.iter(|| {
            black_box(
                Name::initialize_struct(&words, surname_index, generation, name.len()).byte_len(),
            )
        })
    }
}
