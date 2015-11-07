use super::utils;
use super::surname;
use super::namecase;
use std::borrow::Cow;
use std::ascii::AsciiExt;
use unicode_segmentation::UnicodeSegmentation;

// If Start and End overlap, use End
#[derive(Eq,PartialEq)]
pub enum Location {
    Start,
    Middle,
    End
}

pub struct NameParts<'a> {
    text: &'a str,
    trust_capitalization: bool,
    location: Location,
}

impl <'a>NameParts<'a> {
    fn next_location(&mut self) -> Location {
        if self.location == Location::Middle {
            // If the whole section is in the middle, so are all parts
            Location::Middle
        } else if self.location == Location::Start {
            self.location = Location::Middle;
            Location::Start
        } else if self.text.chars().find( |c| c.is_alphabetic() ).is_none() {
            Location::End
        } else {
            Location::Middle
        }
    }
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
            Some(NamePart::from_word(subword, self.trust_capitalization, self.next_location()))
        } else {
            // For ASCII, we split on whitespace only, and handle further
            // segmenting ourselves
            self.text = &self.text[next_whitespace..];
            Some(NamePart::from_word(word, self.trust_capitalization, self.next_location()))
        }
    }
}

#[derive(Eq,PartialEq)]
pub enum Category {
    Name,
    Initials,
    Abbreviation,
    Other,
}

pub struct NamePart<'a> {
    pub word: &'a str,
    pub chars: usize,
    pub category: Category,
    pub trust_capitalization: bool,
    pub location: Location,
}

impl <'a>NamePart<'a> {

    pub fn all_from_text(text: &str, trust_capitalization: bool, location: Location) -> NameParts {
        NameParts {
            text: text,
            trust_capitalization: trust_capitalization,
            location: location
        }
    }

    pub fn from_word(word: &str, trust_capitalization: bool, location: Location) -> NamePart {
        let chars = word.chars().count();

        let category =
            if chars == 1 && word.chars().nth(0).unwrap().is_ascii() {
                Category::Initials
            } else if chars == 1 {
                Category::Name
            } else if utils::is_period_separated(word) {
                Category::Initials
            } else if word.contains('.') {
                Category::Abbreviation
            } else if word.chars().filter( |c| !c.is_alphabetic() ).count() > 2 {
                Category::Other
            } else if utils::is_missing_vowels(word) {
                if trust_capitalization && word.chars().all(|c| !c.is_alphabetic() || c.is_uppercase()) {
                    Category::Initials
                } else if location == Location::End && surname::is_vowelless_surname(word, trust_capitalization) {
                    Category::Name
                } else if chars <= 5 {
                    Category::Initials
                } else {
                    Category::Other
                }
            } else {
                if chars <= 4 && trust_capitalization && word.chars().all(|c| !c.is_alphabetic() || c.is_uppercase()) {
                    Category::Initials
                }
                else {
                    Category::Name
                }
            };

        NamePart {
            word: word,
            chars: chars,
            category: category,
            trust_capitalization: trust_capitalization,
            location: location,
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

    #[inline]
    pub fn is_initials(&self) -> bool {
        self.category == Category::Initials
    }

    #[inline]
    pub fn is_namelike(&self) -> bool {
        self.category == Category::Name
    }

    #[inline]
    pub fn is_abbreviation(&self) -> bool {
        self.category == Category::Abbreviation
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
