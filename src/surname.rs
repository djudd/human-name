use super::namepart::{Category, NamePart};

const VOWELLESS_SURNAMES: [&str; 4] = ["Ng", "Lv", "Mtz", "Hdz"];

const SINGLE_LETTER_CONJUNCTIONS: [&str; 4] = ["e", "y", "E", "Y"];

static SURNAME_PREFIXES: phf::Set<&'static str> =
    include!(concat!(env!("OUT_DIR"), "/surname_prefixes.rs"));

pub fn is_vowelless_surname(word: &str, use_capitalization: bool) -> bool {
    if use_capitalization {
        VOWELLESS_SURNAMES.contains(&word)
    } else {
        VOWELLESS_SURNAMES
            .iter()
            .any(|surname| surname.eq_ignore_ascii_case(word))
    }
}

pub fn find_surname_index(words: &[NamePart]) -> usize {
    if words.len() < 2 {
        return 0;
    }

    for (i, word) in words[0..words.len() - 1].iter().enumerate() {
        let key: &str = match word.category {
            Category::Name(ref namecased) => namecased,
            _ => word.word,
        };
        if SURNAME_PREFIXES.contains(key) {
            return i;
        }

        if i > 0 && SINGLE_LETTER_CONJUNCTIONS.contains(&word.word) {
            // We found what looks like a conjunction in a Spanish or Portuguese
            // style surname (e.g. "Romero y Galdámez" or "Dato e Iradier"), so
            // the previous word was the start of the surname
            if !words[i - 1].is_initials() && !words[i + 1].is_initials() {
                return i - 1;
            }
        }
    }

    // Default case: just assume the last word is the surname
    words.len() - 1
}

#[cfg(test)]
mod tests {
    use super::super::namepart::{Location, NamePart};
    use super::*;

    #[test]
    fn one_word() {
        let parts: Vec<_> = NamePart::all_from_text("Doe", true, Location::Start).collect();
        assert_eq!(0, find_surname_index(&*parts));
    }

    #[test]
    fn two_words() {
        let parts: Vec<_> = NamePart::all_from_text("Jane Doe", true, Location::Start).collect();
        assert_eq!(1, find_surname_index(&*parts));
    }

    #[test]
    fn three_words() {
        let parts: Vec<_> =
            NamePart::all_from_text("Jane Emily Doe", true, Location::Start).collect();
        assert_eq!(2, find_surname_index(&*parts));
    }

    #[test]
    fn conjunction_after_nothing() {
        let parts: Vec<_> = NamePart::all_from_text("y Velazquez", true, Location::Start).collect();
        assert_eq!(1, find_surname_index(&*parts));
    }

    #[test]
    fn conjunction_after_one() {
        let parts: Vec<_> =
            NamePart::all_from_text("Rodrigo y Velazquez", true, Location::Start).collect();
        assert_eq!(0, find_surname_index(&*parts));
    }

    #[test]
    fn conjunction_after_two() {
        let parts: Vec<_> =
            NamePart::all_from_text("Jane Rodrigo y Velazquez", true, Location::Start).collect();
        assert_eq!(1, find_surname_index(&*parts));
    }

    #[test]
    fn particle_after_nothing() {
        let parts: Vec<_> =
            NamePart::all_from_text("Abd al-Qader", true, Location::Start).collect();
        assert_eq!(0, find_surname_index(&*parts));
    }

    #[test]
    fn particle_after_one() {
        let parts: Vec<_> =
            NamePart::all_from_text("Jane Abd al-Qader", true, Location::Start).collect();
        assert_eq!(1, find_surname_index(&*parts));
    }

    #[test]
    fn particle_and_conjunction() {
        let parts: Vec<_> =
            NamePart::all_from_text("Alejandro de Aza y Cabra", true, Location::Start).collect();
        assert_eq!(1, find_surname_index(&*parts));
    }

    #[test]
    fn conjunction_and_particle() {
        let parts: Vec<_> =
            NamePart::all_from_text("Alejandro Cabra y de Aza", true, Location::Start).collect();
        assert_eq!(1, find_surname_index(&*parts));
    }
}
