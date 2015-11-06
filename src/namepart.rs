use super::utils;
use super::surname;
use std::ascii::AsciiExt;

pub struct NamePart<'a> {
    pub word: &'a str,
    pub chars: usize,
    pub is_initials: bool,
    pub is_namelike: bool,
}

impl <'a>NamePart<'a> {

    pub fn from_word(word: &str, use_capitalization: bool) -> NamePart {
        let chars = word.chars().count();

        let mut initials = false;
        let mut namelike = false;

        if chars == 1 {
            initials = true;
            namelike = !word.chars().nth(0).unwrap().is_ascii();
        } else if utils::is_period_separated(word) {
            initials = true;
            namelike = false;
        } else if word.ends_with('.') {
            // Abbreviation
        } else if word.chars().filter( |c| !c.is_alphabetic() ).count() > 2 {
            // Weird/junk
        } else if utils::is_missing_vowels(word) {
            initials = chars <= 4 && (!use_capitalization || word.chars().all(|c| !c.is_alphabetic() || c.is_uppercase()));
            namelike = surname::is_vowelless_surname(word, use_capitalization);
        } else {
            initials = chars <= 4 && use_capitalization && word.chars().all(|c| !c.is_alphabetic() || c.is_uppercase());
            namelike = !initials;
        }

        NamePart {
            word: word,
            chars: chars,
            is_initials: initials,
            is_namelike: namelike,
        }
    }

    pub fn initial(&self) -> char {
        utils::first_alphabetical_char(self.word)
            .unwrap()
            .to_uppercase()
            .next()
            .unwrap()
    }
}

impl <'a>From<NamePart<'a>> for &'a str {
	fn from(part: NamePart<'a>) -> &'a str { part.word }
}
