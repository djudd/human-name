use super::utils;
use super::surname;
use super::namecase;
use std::borrow::Cow;
use std::ascii::AsciiExt;
use unicode_segmentation::UnicodeSegmentation;

pub struct NameParts<'a> {
    text: &'a str,
    trust_capitalization: bool,
}

impl <'a>Iterator for NameParts<'a> {
    type Item = NamePart<'a>;

    fn next(&mut self) -> Option<NamePart<'a>> {
        // Skip any leading whitespace
        match self.text.char_indices().find( |&(_,c)| !c.is_whitespace() ) {
            Some((i,_)) => {
                if i > 0 {
                    self.text = &self.text[i..];
                }
            },
            None => {
                return None;
            }
        };

        // Now look for the next whitespace that remains
        let next_whitespace = match self.text.char_indices().find( |&(_,c)| c.is_whitespace() ) {
            Some((i,_)) => i,
            None => self.text.len()
        };

        let word = &self.text[0..next_whitespace];

        if !word.chars().any(char::is_alphabetic) {
            // Not a word, skip it by recursing
            self.text = &self.text[next_whitespace..];
            self.next()
        } else if !word.chars().any( |c| c.is_ascii() ) {
            // For non-ASCII, we defer to the unicode_segmentation library
            let (next_word_boundary, subword) = word.split_word_bound_indices().find( |&(_,subword)|
               subword.chars().any(char::is_alphabetic)
            ).unwrap();
            self.text = &self.text[next_word_boundary+subword.len()..];
            Some(NamePart::from_word(subword, self.trust_capitalization))
        } else {
            // For ASCII, we split on whitespace only, and handle further
            // segmenting ourselves
            self.text = &self.text[next_whitespace..];
            Some(NamePart::from_word(word, self.trust_capitalization))
        }
    }
}


pub struct NamePart<'a> {
    pub word: &'a str,
    pub chars: usize,
    pub is_initials: bool,
    pub is_namelike: bool,
    pub trust_capitalization: bool,
}

impl <'a>NamePart<'a> {

    pub fn all_from_text(text: &str, trust_capitalization: bool) -> NameParts {
        NameParts {
            text: text,
            trust_capitalization: trust_capitalization,
        }
    }

    pub fn from_word(word: &str, trust_capitalization: bool) -> NamePart {
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
            initials = chars <= 4 && (!trust_capitalization || word.chars().all(|c| !c.is_alphabetic() || c.is_uppercase()));
            namelike = surname::is_vowelless_surname(word, trust_capitalization);
        } else {
            initials = chars <= 4 && trust_capitalization && word.chars().all(|c| !c.is_alphabetic() || c.is_uppercase());
            namelike = !initials;
        }

        NamePart {
            word: word,
            chars: chars,
            is_initials: initials,
            is_namelike: namelike,
            trust_capitalization: trust_capitalization,
        }
    }

    pub fn initial(&self) -> char {
        self.word
            .chars()
            .find(|c| c.is_alphabetic())
            .unwrap()
            .to_uppercase()
            .next()
            .unwrap()
    }

    pub fn namecase(&self, might_be_particle: bool) -> Cow<str> {
        if self.trust_capitalization {
            Cow::Borrowed(self.word)
        } else {
            Cow::Owned(namecase::namecase(self.word, might_be_particle))
        }
    }
}

impl <'a>From<NamePart<'a>> for &'a str {
	fn from(part: NamePart<'a>) -> &'a str { part.word }
}
