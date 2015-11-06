use phf;
use std::ascii::AsciiExt;
use super::namepart::NamePart;

static VOWELLESS_SURNAMES: [&'static str; 4] = [
    "Ng",
    "Lv",
    "Mtz",
    "Hdz",
];

static SINGLE_LETTER_CONJUNCTIONS: [&'static str; 4] = [
    "e",
    "y",
    "E",
    "Y",
];

static SURNAME_PREFIXES: phf::Set<&'static str> = phf_set! {
    "abu",
    "abd",
    "bar",
    "ben",
    "bon",
    "bin",
    "da",
    "das",
    "dal",
    "de",
    "del",
    "dela",
    "der",
    "di",
    "dí",
    "do",
    "dos",
    "ibn",
    "la",
    "le",
    "san",
    "santa",
    "st",
    "ste",
    "ter",
    "van",
    "vel",
    "von",
    "Abu",
    "Abd",
    "Bar",
    "Ben",
    "Bon",
    "Bin",
    "Da",
    "Das",
    "Dal",
    "De",
    "Del",
    "Dela",
    "Der",
    "Di",
    "Dí",
    "Do",
    "Dos",
    "Ibn",
    "La",
    "Le",
    "San",
    "Santa",
    "St",
    "Ste",
    "Ter",
    "Van",
    "Vel",
    "Von",
    "ABU",
    "ABD",
    "BAR",
    "BEN",
    "BON",
    "BIN",
    "DA",
    "DAS",
    "DAL",
    "DE",
    "DEL",
    "DELA",
    "DER",
    "DI",
    "DÍ",
    "DO",
    "DOS",
    "IBN",
    "LA",
    "LE",
    "SAN",
    "SANTA",
    "ST",
    "STE",
    "TER",
    "VAN",
    "VEL",
    "VON",
};

pub fn is_vowelless_surname(word: &str, use_capitalization: bool) -> bool {
    if use_capitalization {
        VOWELLESS_SURNAMES.contains(&word)
    } else {
        VOWELLESS_SURNAMES.iter().any( |surname| surname.eq_ignore_ascii_case(word) )
    }
}

pub fn find_surname_index(words: &[NamePart]) -> usize {
    if words.len() < 2 {
        panic!("find_surname_index on list of {} word(s)", words.len());
    } else if words.len() == 2 {
        return 1;
    }

    for (i, word) in words[1..words.len()-1].iter().enumerate() {
        if SURNAME_PREFIXES.contains(word.word) {
            // We found the probable start of the surname, so adjust the index
            // to be an index into the original slice and return
            return i+1;
        }
    }

    for (i, word) in words[2..words.len()-1].iter().enumerate() {
        if SINGLE_LETTER_CONJUNCTIONS.contains(&word.word) {
            // We found what looks like a conjunction in a Spanish or Portuguese
            // style surname (e.g. "Romero y Galdámez" or "Dato e Iradier"), so
            // the previous word was the start of the surname
            if !words[i+1].is_initials && !words[i+3].is_initials {
                return i+1;
            }
        }
    }

    // Default case: just assume the last word is the surname
    words.len() - 1
}
