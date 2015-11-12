use phf;
use namepart::NamePart;

static NUMERIC_SUFFIXES: phf::Set<&'static str> = phf_set! {
    "2",
    "3",
    "4",
    "5",
    "2nd",
    "3rd",
    "4th",
    "5th",
    "2RD",
    "3RD",
    "4TH",
    "5TH",
    "I",
    "II",
    "III",
    "IV",
    "V",
    "i",
    "ii",
    "iii",
    "iv",
    "v",
};

static ABBREVIATION_SUFFIXES: phf::Set<&'static str> = phf_set! {
    "Jr",
    "Jnr",
    "Sr",
    "Snr",
};

pub fn is_suffix(part: &NamePart) -> bool {
    let namecased = &*part.namecased;

    if part.is_namelike() || part.is_initials() {
        NUMERIC_SUFFIXES.contains(part.word) || ABBREVIATION_SUFFIXES.contains(namecased)
    } else if part.is_abbreviation() {
        ABBREVIATION_SUFFIXES.contains(&namecased[0..namecased.len()-1])
    } else {
        false
    }
}

pub fn namecase(part: &NamePart) -> String {
    if part.is_abbreviation() {
        part.namecased.to_string()
    } else if NUMERIC_SUFFIXES.contains(part.word) {
        part.word.to_uppercase()
    } else {
        format!("{}.", part.namecased)
    }
}
