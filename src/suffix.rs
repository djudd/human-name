use crate::namepart::{Category, NamePart};
use ahash::AHashMap;
use once_cell::sync::Lazy;
use std::num::NonZeroU8;

static GENERATION_BY_SUFFIX: Lazy<AHashMap<&'static str, u8>> = Lazy::new(|| {
    let mut map = AHashMap::new();
    include!(concat!(env!("OUT_DIR"), "/generation_by_suffix.rs"));
    map
});

const SUFFIX_BY_GENERATION: [&str; 5] = ["Sr.", "Jr.", "III", "IV", "V"];

pub fn generation_from_suffix(part: &NamePart, might_be_initials: bool) -> Option<NonZeroU8> {
    match part.category {
        Category::Name(ref namecased) => {
            let namecased: &str = namecased;
            GENERATION_BY_SUFFIX.get(namecased).cloned()
        }
        Category::Abbreviation => {
            let without_period = &part.word[0..part.word.len() - 1];
            GENERATION_BY_SUFFIX.get(without_period).cloned()
        }
        Category::Initials if part.counts.chars > 1 || !might_be_initials => {
            GENERATION_BY_SUFFIX.get(part.word).cloned()
        }
        _ => None,
    }
    .and_then(NonZeroU8::new)
}

pub fn display_generational_suffix(generation: NonZeroU8) -> &'static str {
    SUFFIX_BY_GENERATION[usize::from(generation.get() - 1)]
}

#[cfg(test)]
mod tests {
    use super::super::namepart::{Location, NamePart};
    use super::*;

    #[test]
    fn doe() {
        let part = NamePart::from_word("Doe", true, Location::Start);
        assert_eq!(None, generation_from_suffix(&part, true));
    }

    #[test]
    fn jr() {
        let part = NamePart::from_word("Jr", true, Location::Start);
        assert_eq!(NonZeroU8::new(2), generation_from_suffix(&part, true));
    }

    #[test]
    fn jr_dot() {
        let part = NamePart::from_word("Jr", true, Location::Start);
        assert_eq!(NonZeroU8::new(2), generation_from_suffix(&part, true));
    }

    #[test]
    fn iv() {
        let part = NamePart::from_word("IV", true, Location::Start);
        assert_eq!(NonZeroU8::new(4), generation_from_suffix(&part, true));
    }

    #[test]
    fn i() {
        let part = NamePart::from_word("I", true, Location::Start);
        assert_eq!(None, generation_from_suffix(&part, true));
        assert_eq!(NonZeroU8::new(1), generation_from_suffix(&part, false));
    }
}
