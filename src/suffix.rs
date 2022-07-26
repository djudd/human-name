use crate::namepart::{Category, NamePart};
use phf::phf_map;
use std::num::NonZeroU8;

static GENERATION_BY_SUFFIX: phf::Map<&'static str, u8> = phf_map! {
    // Namecased
    "1" => 1,
    "2" => 2,
    "3" => 3,
    "4" => 4,
    "5" => 5,
    "1st" => 1,
    "2nd" => 2,
    "3rd" => 3,
    "4th" => 4,
    "5th" => 5,
    "I" => 1,
    "Ii" => 2,
    "Iii" => 3,
    "Iv" => 4,
    "V" => 5,
    "Père" => 1,
    "Fils" => 2,
    "Júnior" => 2,
    "Filho" => 2,
    "Neto" => 3,
    "Junior" => 2,
    "Senior" => 1,
    "Jr" => 2,
    "Jnr" => 2,
    "Sr" => 1,
    "Snr" => 1,

    // Uppercased
    "1ST" => 1,
    "2ND" => 2,
    "3RD" => 3,
    "4TH" => 4,
    "5TH" => 5,
    "II" => 2,
    "III" => 3,
    "IV" => 4,
    "PÈRE" => 1,
    "FILS" => 2,
    "JÚNIOR" => 2,
    "FILHO" => 2,
    "NETO" => 3,
    "JUNIOR" => 2,
    "SENIOR" => 1,
    "JR" => 2,
    "JNR" => 2,
    "SR" => 1,
    "SNR" => 1,

    // Lowercased
    "i" => 1,
    "ii" => 2,
    "iii" => 3,
    "iv" => 4,
    "v" => 5,
    "père" => 1,
    "fils" => 2,
    "júnior" => 2,
    "filho" => 2,
    "neto" => 3,
    "junior" => 2,
    "senior" => 1,
    "jr" => 2,
    "jnr" => 2,
    "sr" => 1,
    "snr" => 1,
};

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
