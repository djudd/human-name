use super::utils::*;
use unicode_segmentation::UnicodeSegmentation;

const MAX_LEN: usize = u8::max_value() as usize;

#[derive(Debug, Clone)]
pub struct Segment<'a> {
    pub word: &'a str,
    pub counts: CharacterCounts,
}

pub struct Segments<'a> {
    text: &'a str,
    current_word: &'a str,
}

static AMPERSAND: Segment = Segment {
    word: "&",
    counts: CharacterCounts {
        chars: 1,
        alpha: 0,
        upper: 0,
        ascii_alpha: 0,
        ascii_vowels: 0,
    },
};

impl<'a> Segments<'a> {
    pub fn from_text(text: &'a str) -> Segments<'a> {
        Segments {
            text,
            current_word: "",
        }
    }
}

impl<'a> Iterator for Segments<'a> {
    type Item = Segment<'a>;

    fn next(&mut self) -> Option<Segment<'a>> {
        // If we're in the middle of a word that needs sub-segmentation by
        // unicode rules, handle that
        if !self.current_word.is_empty() {
            if let Some((start, subword, counts)) = self
                .current_word
                .split_word_bound_indices()
                .map(|(start, subword)| (start, subword, categorize_chars(subword)))
                .find(|(_, _, counts)| counts.alpha > 0)
            {
                self.current_word = &self.current_word[start + subword.len()..];
                return Some(Segment {
                    word: subword,
                    counts,
                });
            } else {
                self.current_word = "";
            }
        }

        // Otherwise, skip any leading whitespace
        self.text = self.text.trim_start();

        if self.text.is_empty() {
            return None;
        }

        // Now look for the next whitespace that remains
        let next_whitespace = self.text.find(' ').unwrap_or_else(|| self.text.len());
        let next_inner_period = self.text[0..next_whitespace].find('.');
        let next_boundary = match next_inner_period {
            Some(i) => i + 1,
            None => next_whitespace,
        };

        let word = &self.text[0..next_boundary];
        self.text = &self.text[next_boundary..];

        if word.len() > MAX_LEN {
            self.next()
        } else if word == "&" {
            // Special case: only allowed word without alphabetical characters
            Some(AMPERSAND.clone())
        } else {
            let counts = categorize_chars(word);
            if counts.alpha == 0 {
                // Not a word, skip it by recursing
                self.next()
            } else if counts.ascii_alpha == 0 {
                // For completely non-ASCII words, likely Hangul or similar,
                // we defer to the unicode_segmentation library
                self.current_word = word;
                self.next()
            } else {
                // For ASCII, we split on whitespace and periods only
                Some(Segment { word, counts })
            }
        }
    }
}
