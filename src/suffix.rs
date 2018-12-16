use phf;
use namepart::NamePart;

static GENERATION_BY_SUFFIX: phf::Map<&'static str, usize> = phf_map! {
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
};

static SUFFIX_BY_GENERATION: [&'static str; 5] = ["Sr.", "Jr.", "III", "IV", "V"];

pub fn generation_from_suffix(part: &NamePart, might_be_initials: bool) -> Option<usize> {
    let namecased = &*part.namecased;

    if part.is_namelike() || (part.is_initials() && !(part.chars == 1 && might_be_initials)) {
        GENERATION_BY_SUFFIX.get(namecased).cloned()
    } else if part.is_abbreviation() {
        GENERATION_BY_SUFFIX.get(&namecased[0..namecased.len() - 1]).cloned()
    } else {
        None
    }
}

pub fn display_generational_suffix(generation: usize) -> &'static str {
    SUFFIX_BY_GENERATION[generation - 1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::namepart::{Location, NamePart};

    #[test]
    fn doe() {
        let part = NamePart::from_word("Doe", true, Location::Start);
        assert_eq!(None, generation_from_suffix(&part, true));
    }

    #[test]
    fn jr() {
        let part = NamePart::from_word("Jr", true, Location::Start);
        assert_eq!(Some(2), generation_from_suffix(&part, true));
    }

    #[test]
    fn jr_dot() {
        let part = NamePart::from_word("Jr", true, Location::Start);
        assert_eq!(Some(2), generation_from_suffix(&part, true));
    }

    #[test]
    fn iv() {
        let part = NamePart::from_word("IV", true, Location::Start);
        assert_eq!(Some(4), generation_from_suffix(&part, true));
    }

    #[test]
    fn i() {
        let part = NamePart::from_word("I", true, Location::Start);
        assert_eq!(None, generation_from_suffix(&part, true));
        assert_eq!(Some(1), generation_from_suffix(&part, false));
    }
}
