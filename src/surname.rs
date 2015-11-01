use std::collections::HashSet;
use super::initials;

lazy_static! {
    static ref SURNAME_PREFIXES: HashSet<&'static str> = {
        let s: HashSet<&'static str> = [
            "abu",
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
            "de",
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
        ].iter().cloned().collect();
        s
    };
}

pub fn find_surname_index(words: &[&str], use_capitalization: bool) -> usize {
    if words.len() < 2 {
        panic!("find_surname_index on list of {} word(s)", words.len());
    } else if words.len() == 2 {
        return 1;
    }

    let iter = words[1..words.len()-1].iter().enumerate();
    for (i, word) in iter {
        let lower: &str = &word.to_lowercase();
        if SURNAME_PREFIXES.contains(lower) {
            // We found the probable start of the surname, so adjust the index
            // to be an index into the original slice and return
            return i+1;
        }
        else if i > 0 && (i+2) < words.len() && (lower == "y" || lower == "e") {
            // We found what looks like a conjunction in a Spanish or Portuguese
            // style surname (e.g. "Romero y Galdámez" or "Dato e Iradier"), so
            // the previous word was the start of the surname
            if !initials::is_initials(words[i], use_capitalization) && !initials::is_initials(words[i+2], use_capitalization) {
                return i;
            }
        }
    }

    // Default case: just assume the last word is the surname
    words.len() - 1
}
