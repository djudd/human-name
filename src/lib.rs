//! A library for parsing and comparing human names.
//!
//! See the documentation of the `Name` struct for details.

#![doc(html_root_url = "https://djudd.github.io/human-name/")]
#![cfg_attr(feature = "bench", feature(test))]

extern crate crossbeam_utils;
extern crate smallvec;
extern crate unicode_normalization;
extern crate unicode_segmentation;
extern crate unidecode;

#[cfg(test)]
#[cfg(feature = "bench")]
extern crate test;

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
mod word;

#[cfg(feature = "ffi")]
pub mod external;

#[cfg(feature = "name_eq_hash")]
mod eq_hash;

#[cfg(feature = "serialization")]
mod serialization;

use crate::decomposition::normalize_nfkd_whitespace;
use crate::word::{Location, Words};
use compact_str::CompactString;
use crossbeam_utils::atomic::AtomicCell;
use smallvec::SmallVec;
use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::convert::TryInto;
use std::hash::{Hash, Hasher};
use std::num::NonZeroU8;

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
#[derive(Debug)]
pub struct Name {
    text: CompactString, // stores concatenation of display_full() and initials()
    locations: SmallVec<[Location; 6]>, // stores concatenation of word locations in full text and given name locations in initials
    given_name_words: u8,               // support no more than 256
    surname_words: u8,                  // support no more than 256
    initials_len: u8,                   // support no more than 256
    generation: Option<NonZeroU8>,
    honorifics: Option<Box<Honorifics>>,
    surname_hash: AtomicCell<Option<u32>>,
}

#[derive(Clone, Debug)]
struct Honorifics {
    prefix: Option<Box<str>>,
    suffix: Option<Box<str>>,
}

impl Clone for Name {
    fn clone(&self) -> Self {
        Name {
            text: self.text.clone(),
            locations: self.locations.clone(),
            given_name_words: self.given_name_words,
            surname_words: self.surname_words,
            initials_len: self.initials_len,
            generation: self.generation,
            honorifics: self.honorifics.clone(),
            surname_hash: Default::default(),
        }
    }
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

        Name::initialize_struct(&parsed, name.len())
    }

    fn initialize_struct(parsed: &parse::Name, name_len: usize) -> Option<Name> {
        let words = parsed.words();
        let surname_index = parsed.surname_index;

        let mut text = CompactString::with_capacity(name_len + surname_index);
        let mut initials = CompactString::with_capacity(surname_index);

        let mut locations = SmallVec::with_capacity(words.len() + surname_index);
        let mut locations_in_initials: SmallVec<[Location; 4]> =
            SmallVec::with_capacity(surname_index);

        for word in &words[..surname_index] {
            if word.is_initials() {
                word.with_initials(|c| {
                    text.push(c);
                    text.push_str(". ");

                    initials.push(c);
                });
            } else {
                let prior_len = text.len();
                word.with_namecased(|s| text.push_str(s));
                locations.push(Location::new(prior_len..text.len())?);

                let prior_len = initials.len();
                word.with_initials(|c| initials.push(c));
                locations_in_initials.push(Location::new(prior_len..initials.len())?);

                text.push(' ');
            }
        }

        let surname_words = &words[surname_index..];
        for (i, word) in surname_words.iter().enumerate() {
            let prior_len = text.len();
            word.with_namecased(|s| text.push_str(s));
            locations.push(Location::new(prior_len..text.len())?);

            if i < surname_words.len() - 1 {
                text.push(' ');
            }
        }

        debug_assert!(!text.is_empty(), "Names are empty!");
        debug_assert!(!initials.is_empty(), "Initials are empty!");

        let generation = parsed.generation;
        let honorifics = {
            let prefix = parsed
                .honorific_prefix()
                .map(|s| s.into_owned().into_boxed_str());
            let suffix = parsed
                .honorific_suffix()
                .map(|s| s.into_owned().into_boxed_str());

            if prefix.is_some() || suffix.is_some() {
                Some(Box::new(Honorifics { prefix, suffix }))
            } else {
                None
            }
        };

        let surname_words = (locations.len() - locations_in_initials.len())
            .try_into()
            .ok()?;
        let given_name_words = locations_in_initials.len().try_into().ok()?;
        let initials_len = initials.len().try_into().ok()?;

        text.push_str(&initials);
        text.shrink_to_fit();

        locations.extend_from_slice(&locations_in_initials);
        locations.shrink_to_fit();

        Some(Name {
            text,
            locations,
            given_name_words,
            surname_words,
            initials_len,
            generation,
            honorifics,
            surname_hash: Default::default(),
        })
    }

    /// First initial (always present)
    pub fn first_initial(&self) -> char {
        self.initials().chars().next().unwrap()
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
        if let Some(loc) = self.given_names_in_initials().first() {
            loc.range().start > 0
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
    #[inline]
    pub fn initials(&self) -> &str {
        &self.text[self.name_bytes()..]
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
            .map(|(i, _)| &self.text[self.name_bytes() + i..])
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
        let start = self.surname_locations()[0].range().start;
        let end = self.surname_end_in_text();
        &self.text[start..end]
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
        self.generation.map(suffix::display_generational_suffix)
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
    /// assert_eq!(Some("MD"), name.honorific_suffix());
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
        if self.given_name_words == 0 && self.initials_len == 1 {
            Cow::Borrowed(&self.text[..self.surname_end_in_text()])
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
        if self.given_name_words <= 1 && self.initials_len == 1 {
            Cow::Borrowed(&self.text[..self.surname_end_in_text()])
        } else if let Some(ref name) = self.given_name() {
            Cow::Owned(format!("{} {}", name, self.surname()))
        } else {
            self.display_initial_surname()
        }
    }

    /// Number of bytes in the full name as UTF-8 in NFKD normal form, including
    /// spaces and punctuation.
    ///
    /// Includes generational suffix, but does not include honorifics.
    ///
    /// ```
    /// use human_name::Name;
    ///
    /// let name = Name::parse("JOHN ALLEN Q DE LA MACDÖNALD JR").unwrap();
    /// assert_eq!("John Allen Q. de la MacDönald, Jr.".len(), name.byte_len());
    /// ```
    #[inline]
    pub fn byte_len(&self) -> usize {
        const SEPARATOR_LEN: usize = ", ".len();

        self.name_bytes()
            + self
                .generational_suffix()
                .map(|g| g.len() + SEPARATOR_LEN)
                .unwrap_or(0)
    }

    #[inline]
    fn name_bytes(&self) -> usize {
        self.text.len() - usize::from(self.initials_len)
    }

    /// The full name, or as much of it as was preserved from the input,
    /// including given name, middle names, surname and generational suffix.
    ///
    /// Includes generational suffix, but does not include honorifics.
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
    #[inline]
    pub fn display_full(&self) -> Cow<str> {
        let name = &self.text[..self.name_bytes()];
        if let Some(suffix) = self.generational_suffix() {
            let mut result = name.to_string();
            result.push_str(", ");
            result.push_str(suffix);
            Cow::Owned(result)
        } else {
            Cow::Borrowed(name)
        }
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
                    + self.byte_len()
                    + honorifics.suffix.as_ref().map(|t| t.len() + 1).unwrap_or(0),
            );
            if let Some(prefix) = &honorifics.prefix {
                result.push_str(prefix);
                result.push(' ');
            }
            result.push_str(&self.display_full());
            if let Some(suffix) = &honorifics.suffix {
                result.push(' ');
                result.push_str(suffix);
            }
            Cow::Owned(result)
        } else {
            self.display_full()
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
    /// because that's all we're guaranteed to use in the consistency test,
    /// and we attempt to convert to lowercase ASCII, giving us only have 19
    /// bits of variability.
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
    pub fn surname_hash(&self) -> u64 {
        if let Some(hash) = self.surname_hash.load() {
            return hash.into();
        }

        let mut s = DefaultHasher::new();
        self.hash_surname(&mut s);

        // Since we only have ~19 bits of input (per above),
        // there's no point keeping a longer hash.
        let hash = s.finish() as u32;
        self.surname_hash.store(Some(hash));
        hash.into()
    }

    fn hash_surname<H: Hasher>(&self, state: &mut H) {
        for c in self
            .surname_iter()
            .rev()
            .flat_map(|word| {
                transliterate::to_ascii_casefolded_reversed(word)
                    .into_iter()
                    .flatten()
            })
            .take(comparison::MIN_SURNAME_CHAR_MATCH)
        {
            c.hash(state);
        }
    }

    #[inline]
    fn surname_end_in_text(&self) -> usize {
        self.surname_locations()[usize::from(self.surname_words) - 1]
            .range()
            .end
    }

    #[inline]
    fn surname_iter(
        &self,
    ) -> Words<impl Iterator<Item = Location> + DoubleEndedIterator + ExactSizeIterator + '_> {
        self.word_iter(self.surname_locations())
    }

    #[inline]
    fn middle_name_iter(
        &self,
    ) -> Option<Words<impl Iterator<Item = Location> + DoubleEndedIterator + ExactSizeIterator + '_>>
    {
        if self.given_name_words > 1 {
            Some(self.word_iter(&self.given_name_locations()[1..]))
        } else {
            None
        }
    }

    #[inline]
    fn given_iter(
        &self,
    ) -> Words<impl Iterator<Item = Location> + DoubleEndedIterator + ExactSizeIterator + '_> {
        self.word_iter(self.given_name_locations())
    }

    #[inline]
    fn word_iter<'a>(
        &'a self,
        locations: &'a [Location],
    ) -> Words<'_, impl Iterator<Item = Location> + DoubleEndedIterator + ExactSizeIterator + '_>
    {
        Words::new(&self.text, locations.iter().copied())
    }

    #[inline]
    fn given_name_locations(&self) -> &[Location] {
        &self.locations[..self.given_name_words.into()]
    }

    #[inline]
    fn surname_locations(&self) -> &[Location] {
        &self.locations
            [self.given_name_words.into()..(self.given_name_words + self.surname_words).into()]
    }

    #[inline]
    fn given_names_in_initials(&self) -> &[Location] {
        &self.locations[(self.given_name_words + self.surname_words).into()..]
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
        assert_eq!(80, std::mem::size_of::<Name>());
        assert_eq!(32, std::mem::size_of::<Honorifics>());
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

    #[test]
    fn parse_high_proportion_of_combining_chars() {
        let name = Name::parse(".ΰ\u{330}\u{610}`");
        assert!(name.is_none());
    }

    #[test]
    fn parse_very_long_honorific_prefix() {
        // It would probably also be fine to fail to parse this, but we shouldn't panic
        let name = Name::parse("%%%%%hLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLe pl Puc");
        assert_eq!("H. Lllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllll. E. P. L. Puc", name.unwrap().display_full_with_honorifics());
    }

    #[test]
    fn eq_non_alphanumeric_initials() {
        // It would probably also be fine to fail to parse one of these, but we shouldn't panic
        let a = Name::parse("\u{3}\n\u{4}\u{19}Joo\n'lA").unwrap();
        let b = Name::parse("H8\n'lA/").unwrap();
        assert!(!a.consistent_with(&b));
    }

    #[test]
    fn eq_empty_transliterated_initials() {
        // It would probably also be fine to fail to parse `b` or find consistency, but we shouldn't panic
        let a = Name::parse("Ng\nmac").unwrap();
        let b = Name::parse("\u{65c}\nmac\n").unwrap();
        assert!(!a.consistent_with(&b));
    }

    #[test]
    fn digits() {
        let a = Name::parse("111 222");
        assert!(a.is_none());

        let a = Name::parse("One-1 Ones").unwrap();
        let b = Name::parse("One-2 Ones").unwrap();
        assert!(a.consistent_with(&b));

        let a = Name::parse("One Ones-1").unwrap();
        let b = Name::parse("One Ones-2").unwrap();
        assert!(!a.consistent_with(&b));

        let a = Name::parse("One Ones1").unwrap();
        let b = Name::parse("One Ones2").unwrap();
        assert!(!a.consistent_with(&b));

        let a = Name::parse("One1 Ones").unwrap();
        let b = Name::parse("One2 Ones").unwrap();
        assert!(a.consistent_with(&b));

        let a = Name::parse("One 1 Ones").unwrap();
        let b = Name::parse("One 2 Ones").unwrap();
        assert!(a.consistent_with(&b));
    }

    #[test]
    fn non_bmp_alphas() {
        let a = Name::parse("𐒴𐓘 𐓊𐓙").unwrap();
        let b = Name::parse("𐒴𐓘 𐒵 𐓊𐓙").unwrap();
        assert_eq!("𐒴𐓘 𐓊𐓙", a.display_first_last());
        assert_eq!("𐒴𐓘 𐓊𐓙", b.display_first_last());
        assert!(a.consistent_with(&b));

        let c = Name::parse("𐒴𐓘 𐒵𐓙").unwrap();
        assert_eq!("𐒴𐓘 𐒵𐓙", c.display_first_last());
        assert!(!a.consistent_with(&c));

        let d = Name::parse("𐒴𐓘 𐓍 𐓊𐓙").unwrap();
        assert_eq!("𐒴𐓘 𐓊𐓙", d.display_first_last());
        assert!(a.consistent_with(&d));
        assert!(!b.consistent_with(&d));

        let a = Name::parse("𐒴𐓘-𐓊𐓙 𐓍𐓙").unwrap();
        assert_eq!("𐒴𐓘 𐓍𐓙", a.display_first_last()); // Preserving the original would probably be better but this documents current behavior
        assert!(a.consistent_with(&a));
        let b = Name::parse("𐒴𐓘 𐓊𐓙-𐓍𐓙").unwrap();
        assert_eq!("𐒴𐓘 𐓍𐓙", b.display_first_last()); // Preserving the original would probably be better but this documents current behavior
        assert!(b.consistent_with(&b));
        let c = Name::parse("𐒴𐓘 𐓊𐓙 𐓍𐓙").unwrap();
        assert_eq!("𐒴𐓘 𐓍𐓙", c.display_first_last());
        assert!(c.consistent_with(&c));

        assert!(a.consistent_with(&b));
        assert!(a.consistent_with(&c));
        assert!(b.consistent_with(&c));
    }

    #[test]
    fn stops_being_nfkd() {
        // Some string split stops this from being NFKD after it's normalized, which is ~fine
        // but at one point produced a panic on a debug assertion.
        let input = "\u{5c4}((\0)\u{64f}()()\u{5c4}\u{64f}\u{612}";
        assert!(Name::parse(input).is_none());
    }

    #[test]
    fn emojis() {
        let a = Name::parse("😃 😃");
        assert!(a.is_none());

        let a = Name::parse("smile-😃 smiley").unwrap();
        let b = Name::parse("smile-😰 smiley").unwrap();
        assert!(a.consistent_with(&b));

        let a = Name::parse("smile smiley-😃").unwrap();
        let b = Name::parse("smile smiley-😰").unwrap();
        assert!(a.consistent_with(&b));

        let a = Name::parse("smile 😃 smiley").unwrap();
        let b = Name::parse("smile 😰 smiley").unwrap();
        assert!(a.consistent_with(&b));

        let a = Name::parse("smile-😃 smiley").unwrap();
        let b = Name::parse("smile-😰 smiley").unwrap();
        assert!(a.consistent_with(&b));

        let a = Name::parse("smile😃 smiley").unwrap();
        let b = Name::parse("smile😰 smiley").unwrap();
        assert!(a.consistent_with(&b));

        let a = Name::parse("smile smiley😃").unwrap();
        let b = Name::parse("smile smiley😰").unwrap();
        assert!(a.consistent_with(&b));
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn initialize_struct_initial_surname(b: &mut Bencher) {
        let name = "J. Doe";
        let parsed = parse::parse(&*name).unwrap();
        b.iter(|| {
            black_box(
                Name::initialize_struct(&parsed, name.len())
                    .unwrap()
                    .byte_len(),
            )
        })
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn initialize_struct_first_last(b: &mut Bencher) {
        let name = "John Doe";
        let parsed = parse::parse(&*name).unwrap();
        b.iter(|| {
            black_box(
                Name::initialize_struct(&parsed, name.len())
                    .unwrap()
                    .byte_len(),
            )
        })
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn initialize_struct_complex(b: &mut Bencher) {
        let name = "John Allen Q.R. de la MacDonald Jr.";
        let parsed = parse::parse(&*name).unwrap();
        b.iter(|| {
            black_box(
                Name::initialize_struct(&parsed, name.len())
                    .unwrap()
                    .byte_len(),
            )
        })
    }
}

#[cfg(feature = "bench")]
#[cfg(test)]
mod bench {
    use super::Name;
    use std::fs::File;
    use std::io::prelude::*;
    use std::io::BufReader;

    #[cfg(feature = "bench")]
    use test::{black_box, Bencher};

    #[bench]
    fn bench_parsing_first_last(b: &mut Bencher) {
        b.iter(|| {
            let parsed = Name::parse("Juan Garcia");
            black_box(parsed.is_none())
        })
    }

    #[bench]
    fn bench_parsing_sort_order(b: &mut Bencher) {
        b.iter(|| {
            let parsed = Name::parse("Garcia, J.Q.");
            black_box(parsed.is_none())
        })
    }

    #[bench]
    fn bench_parsing_needs_namecase(b: &mut Bencher) {
        b.iter(|| {
            let parsed = Name::parse("JAIME GARCIA");
            black_box(parsed.is_none())
        })
    }

    #[bench]
    fn bench_parsing_unparseable(b: &mut Bencher) {
        b.iter(|| {
            let parsed = Name::parse("foo@bar.com");
            black_box(parsed.is_none())
        })
    }

    #[bench]
    fn bench_parsing_complex(b: &mut Bencher) {
        let name = "鈴木 Velasquez y Garcia, Dr. Juan Q. 'Don Juan' Xavier III";
        b.iter(|| {
            let parsed = Name::parse(name);
            black_box(parsed.is_none())
        })
    }

    #[bench]
    fn bench_equality_equal(b: &mut Bencher) {
        let x = Name::parse("Jane Doe").unwrap();
        let y = Name::parse("Jane H. Doe").unwrap();

        b.iter(|| black_box(x.consistent_with(&y)))
    }

    #[bench]
    fn bench_equality_not_equal(b: &mut Bencher) {
        let x = Name::parse("Jane Doe").unwrap();
        let y = Name::parse("Foo Bar").unwrap();

        b.iter(|| black_box(x.consistent_with(&y)))
    }

    #[bench]
    fn bench_equality_close_to_equal(b: &mut Bencher) {
        let x = Name::parse("Jane Doe").unwrap();
        let y = Name::parse("John Doe").unwrap();

        b.iter(|| black_box(x.consistent_with(&y)))
    }

    #[bench]
    fn bench_parsing_many(b: &mut Bencher) {
        let f = File::open("tests/benchmark-names.txt").ok().unwrap();
        let reader = BufReader::new(f);
        let names: Vec<String> = reader.lines().map(|l| l.ok().unwrap()).collect();

        b.iter(move || {
            let mut valid = 0;
            let mut invalid = 0;

            for name in names.iter() {
                let parsed = Name::parse(&name);
                if parsed.is_none() {
                    invalid += 1;
                } else {
                    valid += 1;
                }
            }

            black_box(valid);
            black_box(invalid);
        })
    }

    #[bench]
    fn bench_equality_many(b: &mut Bencher) {
        let f = File::open("tests/benchmark-names.txt").ok().unwrap();
        let reader = BufReader::new(f);
        let names: Vec<Name> = reader
            .lines()
            .filter_map(|l| Name::parse(&l.ok().unwrap()))
            .collect();

        b.iter(|| {
            let mut matches = 0;

            for a in &names[..64] {
                for b in &names {
                    if a.consistent_with(&b) {
                        matches += 1;
                    }
                }
            }

            black_box(matches);
        })
    }
}
