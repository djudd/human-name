//! A library for parsing and comparing human names.
//!
//! See the documentation of the `Name` struct for details.

#![doc(html_root_url = "https://djudd.github.io/human-name/")]
#![cfg_attr(feature = "bench", feature(test))]

extern crate phf;
extern crate smallstr;
extern crate smallvec;
extern crate unicode_normalization;
extern crate unicode_segmentation;
extern crate unidecode;

#[cfg(test)]
#[cfg(feature = "bench")]
extern crate test;

#[cfg(feature = "serialization")]
extern crate serde;
#[cfg(feature = "serialization")]
#[macro_use]
extern crate serde_derive;
#[cfg(test)]
extern crate alloc_counter;

mod case;
mod comparison;
mod decomposition;
mod features;
mod namecase;
mod namepart;
mod nickname;
mod parse;
mod segment;
mod suffix;
mod surname;
mod title;
mod transliterate;
mod web_match;
mod word;

pub mod external;

#[cfg(feature = "name_eq_hash")]
mod eq_hash;

#[cfg(feature = "serialization")]
mod serialization;

use crate::decomposition::normalize_nfkd_whitespace;
use crate::word::{WordIndices, Words};
use smallstr::SmallString;
use smallvec::SmallVec;
use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::convert::TryInto;
use std::hash::{Hash, Hasher};
use std::num::NonZeroU8;
use std::ops::Range;

#[cfg(test)]
use alloc_counter::AllocCounterSystem;

#[cfg(test)]
#[global_allocator]
static A: AllocCounterSystem = AllocCounterSystem;

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
    text: SmallString<[u8; 32]>,
    word_indices_in_text: WordIndices,
    surname_index: u16, // u16 must be sufficient since it can represent MAX_NAME_LEN
    generation_from_suffix: Option<NonZeroU8>,
    initials: SmallString<[u8; 8]>,
    word_indices_in_initials: WordIndices,
    honorifics: Option<Box<Honorifics>>,
    pub hash: u64,
}

#[derive(Clone, Debug)]
struct Honorifics {
    prefix: Option<String>,
    suffix: Option<String>,
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
    /// assert_eq!(Some("III"), name.generational_suffix());
    /// assert_eq!(Some("Dr."), name.honorific_prefix());
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

        let name = normalize_nfkd_whitespace(name);
        let name = nickname::strip_nickname(&name);
        let parsed = parse::parse(&name)?;

        let mut name = Name::initialize_struct(&parsed, name.len());

        let mut s = DefaultHasher::new();
        name.surname_hash(&mut s);
        name.hash = s.finish();

        Some(name)
    }

    fn initialize_struct(parsed: &parse::Name, name_len: usize) -> Name {
        let words = parsed.words();
        let last_word = words.len() - 1;
        let surname_index = parsed.surname_index;

        let mut text = SmallString::with_capacity(name_len);
        let mut initials = SmallString::with_capacity(surname_index);

        let mut surname_index_in_names = surname_index;
        let mut word_indices_in_initials = WordIndices::new();
        let mut word_indices_in_text = WordIndices::new();

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

        if let Some(generation) = parsed.generation {
            text.push_str(", ");
            text.push_str(suffix::display_generational_suffix(generation));
        }

        debug_assert!(!text.is_empty(), "Names are empty!");
        debug_assert!(!initials.is_empty(), "Initials are empty!");

        let honorifics = {
            let prefix = parsed.honorific_prefix().map(Cow::into_owned);
            let suffix = parsed.honorific_suffix().map(Cow::into_owned);

            if prefix.is_some() || suffix.is_some() {
                Some(Box::new(Honorifics { prefix, suffix }))
            } else {
                None
            }
        };

        text.shrink_to_fit();
        word_indices_in_text.shrink_to_fit();
        initials.shrink_to_fit();
        word_indices_in_initials.shrink_to_fit();

        Name {
            text,
            word_indices_in_text,
            surname_index: surname_index_in_names.try_into().unwrap(),
            generation_from_suffix: parsed.generation,
            initials,
            word_indices_in_initials,
            honorifics,
            hash: 0,
        }
    }

    /// First initial (always present)
    pub fn first_initial(&self) -> char {
        self.initials.chars().next().unwrap()
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
        self.given_iter().next()
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
        if let Some(&Range { start, .. }) = self.word_indices_in_initials.get(0) {
            start > 0
        } else {
            false
        }
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
            .nth(1)
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
        let surname_indices = &self.word_indices_in_text[self.surname_index.into()..];
        let start = surname_indices[0].start;
        let end = surname_indices[surname_indices.len() - 1].end;
        &self.text[start.into()..end.into()]
    }

    /// Generational suffix, if present
    ///
    /// ```
    /// use human_name::Name;
    ///
    /// let name = Name::parse("Gary Payton II").unwrap();
    /// assert_eq!(Some("Jr."), name.generational_suffix());
    /// ```
    pub fn generational_suffix(&self) -> Option<&str> {
        self.generation_from_suffix
            .map(suffix::display_generational_suffix)
    }

    /// Generational suffix, if present
    #[deprecated(since = "1.1.0", note = "Use `generational_suffix` instead")]
    pub fn suffix(&self) -> Option<&str> {
        self.generational_suffix()
    }

    /// Honorific prefix(es), if present
    ///
    /// ```
    /// use human_name::Name;
    ///
    /// let name = Name::parse("Rev. Dr. Martin Luther King, Jr.").unwrap();
    /// assert_eq!(Some("Rev. Dr."), name.honorific_prefix());
    /// ```
    pub fn honorific_prefix(&self) -> Option<&str> {
        self.honorifics
            .as_ref()
            .and_then(|h| h.prefix.as_ref())
            .map(|p| p.as_ref())
    }

    /// Honorific suffix(es), if present
    ///
    /// ```
    /// use human_name::Name;
    ///
    /// let name = Name::parse("Stephen Strange, MD").unwrap();
    /// assert_eq!(Some("M.D."), name.honorific_suffix());
    /// ```
    pub fn honorific_suffix(&self) -> Option<&str> {
        self.honorifics
            .as_ref()
            .and_then(|h| h.suffix.as_ref())
            .map(|s| s.as_ref())
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
    /// Does not include honorifics.
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
    /// including given name, middle names, surname and generational suffix.
    ///
    /// Does not include honorifics.
    ///
    /// ```
    /// use human_name::Name;
    ///
    /// let name = Name::parse("DR JOHN ALLEN Q DE LA MACDONALD JR").unwrap();
    /// assert_eq!("John Allen Q. de la MacDonald, Jr.", name.display_full());
    ///
    /// let name = Name::parse("Air Chief Marshal Sir Harrieta ('Harry') Keōpūolani Nāhiʻenaʻena, GBE, KCB, ADC").unwrap();
    /// assert_eq!("Harrieta Keōpūolani Nāhiʻenaʻena", name.display_full());
    /// ```
    pub fn display_full(&self) -> &str {
        &self.text
    }

    /// The full name, or as much of it as was preserved from the input,
    /// including given name, middle names, surname, generational suffix,
    /// and honorifics.
    ///
    /// ```
    /// use human_name::Name;
    ///
    /// let name = Name::parse("DR JOHN ALLEN Q DE LA MACDONALD JR").unwrap();
    /// assert_eq!("Dr. John Allen Q. de la MacDonald, Jr.", name.display_full_with_honorifics());
    ///
    /// let name = Name::parse("Air Chief Marshal Sir Harrieta ('Harry') Keōpūolani Nāhiʻenaʻena, GBE, KCB, ADC").unwrap();
    /// assert_eq!("Air Chief Marshal Sir Harrieta Keōpūolani Nāhiʻenaʻena GBE KCB ADC", name.display_full_with_honorifics());
    /// ```
    pub fn display_full_with_honorifics(&self) -> Cow<str> {
        if let Some(honorifics) = self.honorifics.as_ref() {
            let mut result = String::with_capacity(
                honorifics.prefix.as_ref().map(|t| t.len() + 1).unwrap_or(0)
                    + self.text.len()
                    + honorifics.suffix.as_ref().map(|t| t.len() + 1).unwrap_or(0),
            );
            if let Some(prefix) = &honorifics.prefix {
                result.push_str(prefix);
                result.push(' ');
            }
            result.push_str(&self.text);
            if let Some(suffix) = &honorifics.suffix {
                result.push(' ');
                result.push_str(suffix);
            }
            Cow::Owned(result)
        } else {
            Cow::Borrowed(&self.text)
        }
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
            .rev()
            .flat_map(transliterate::to_ascii_casefolded_reversed)
            .take(comparison::MIN_SURNAME_CHAR_MATCH)
        {
            c.hash(state);
        }
    }

    #[inline]
    fn surname_words(&self) -> usize {
        self.word_indices_in_text.len() - usize::from(self.surname_index)
    }

    #[inline]
    fn surname_iter(&self) -> Words {
        self.word_iter(self.surname_index.into()..self.word_indices_in_text.len())
    }

    #[inline]
    fn middle_name_iter(&self) -> Option<Words> {
        if self.surname_index > 1 {
            Some(self.word_iter(1..self.surname_index.into()))
        } else {
            None
        }
    }

    #[inline]
    fn given_iter(&self) -> Words {
        self.word_iter(0..self.surname_index.into())
    }

    #[inline]
    fn word_iter(&self, range: Range<usize>) -> Words {
        Words::new(&self.text, &self.word_indices_in_text[range])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc_counter::deny_alloc;

    #[cfg(feature = "bench")]
    use test::{black_box, Bencher};

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    #[test]
    fn struct_size() {
        assert_eq!(136, std::mem::size_of::<Name>());
    }

    #[test]
    fn fast_path_parse_does_not_allocate() {
        deny_alloc(|| Name::parse("Jane Doe").unwrap());
        deny_alloc(|| Name::parse("J. Doe").unwrap());
    }

    #[test]
    fn fast_path_eq_does_not_allocate() {
        let n1 = Name::parse("Jane Doe").unwrap();
        let n2 = Name::parse("John Doe").unwrap();
        let n3 = Name::parse("J. Doe").unwrap();
        deny_alloc(|| {
            assert!(!n1.consistent_with(&n2));
            assert!(n1.consistent_with(&n3));
        });
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn initialize_struct_initial_surname(b: &mut Bencher) {
        let name = "J. Doe";
        let parsed = parse::parse(&*name).unwrap();
        b.iter(|| black_box(Name::initialize_struct(&parsed, name.len()).byte_len()))
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn initialize_struct_first_last(b: &mut Bencher) {
        let name = "John Doe";
        let parsed = parse::parse(&*name).unwrap();
        b.iter(|| black_box(Name::initialize_struct(&parsed, name.len()).byte_len()))
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn initialize_struct_complex(b: &mut Bencher) {
        let name = "John Allen Q.R. de la MacDonald Jr.";
        let parsed = parse::parse(&*name).unwrap();
        b.iter(|| black_box(Name::initialize_struct(&parsed, name.len()).byte_len()))
    }
}
