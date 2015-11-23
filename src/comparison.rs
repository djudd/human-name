use super::utils::*;
use super::Name;
use unicode_segmentation::UnicodeSegmentation;

pub const MIN_SURNAME_CHAR_MATCH: usize = 4;
pub const MIN_GIVEN_NAME_CHAR_MATCH: usize = 3;

impl Name {

    /// Might this name represent the same person as another name?
    ///
    /// # Examples
    /// ```
    /// use human_name::Name;
    ///
    /// let j_doe = Name::parse("J. Doe").unwrap();
    /// let jane_doe = Name::parse("Jane Doe").unwrap();
    /// let john_m_doe = Name::parse("John M. Doe").unwrap();
    /// let john_l_doe = Name::parse("John L. Doe").unwrap();
    ///
    /// assert!(j_doe.consistent_with(&john_m_doe));
    /// assert!(j_doe.consistent_with(&john_l_doe));
    /// assert!(j_doe.consistent_with(&jane_doe));
    /// assert!(j_doe.consistent_with(&j_doe));
    /// assert!(!john_m_doe.consistent_with(&john_l_doe));
    /// assert!(!jane_doe.consistent_with(&john_l_doe));
    /// ```
    ///
    /// # Defining "consistency"
    ///
    /// Requires that all known parts are consistent, which means at minimum,
    /// the final words of the surnames match, and one ordered set of first
    /// and middle initials is a superset of the other. If given and/or middle
    /// names and/or suffixes are present in both names, they must match as well.
    ///
    /// Ignores case and non-alphanumeric characters, as well as accents and
    /// other combining marks. In the case of given and middle names, allows
    /// one name to be a prefix of the other, without requiring the prefix
    /// end at a word boundary as we do with surname suffix matches (this
    /// captures cases like "Jin Li"/"Jinli"/"Jin-Li", where the same name
    /// may be transliterated in different ways, as well as *some* nicknames).
    ///
    /// # Limitations
    ///
    /// There will be false positives "Jan Doe" is probably not "Jane Doe",
    /// and false negatives "Dave Judd" might be "David Judd". And, of course,
    /// even identical names do not necessarily represent the same person.
    ///
    /// Given limited information, we err on the side of false positives. This
    /// kind of matching will be most useful in cases where we already have
    /// reason to believe that a single individual's name appears twice, and we
    /// are trying to figure out exactly where, e.g. a particular author's index
    /// in the list of authors of a co-authored paper.
    ///
    #[cfg_attr(rustfmt, rustfmt_skip)]
    pub fn consistent_with(&self, other: &Name) -> bool {
        // Order matters, both for efficiency (initials check is the fastest,
        // coincidentally-identical surnames are less likely than for given names),
        // and for correctness (the given/middle names check assumes a positive result
        // for the middle initials check)
        self.initials_consistent(other) &&
        self.surname_consistent(other) &&
        self.given_and_middle_names_consistent(other) &&
        self.suffix_consistent(other)
    }

    fn initials_consistent(&self, other: &Name) -> bool {
        if self.goes_by_middle_name() == other.goes_by_middle_name() {
            // Normal case: neither goes by a middle name (as far as we know)
            // or both do, so we require the first initial to be the same
            // and one set of middle initials to equal or contain the other
            if self.first_initial() != other.first_initial() {
                return false;
            }

            let my_middle = self.middle_initials();
            let their_middle = other.middle_initials();

            my_middle.is_none() || their_middle.is_none() ||
                my_middle.unwrap().contains(their_middle.unwrap()) ||
                their_middle.unwrap().contains(my_middle.unwrap())
        } else if self.goes_by_middle_name() {
            // Otherwise, we stop requiring the first initial to be the same,
            // because it might have been included in one context and omitted
            // elsewhere, but instead we assume that the name with the initial
            // prior to the middle name includes a full set of initials, so
            // we require the other version to be equal or included
            self.initials().contains(other.initials())
        } else {
            other.initials().contains(self.initials())
        }
    }

    fn given_and_middle_names_consistent(&self, other: &Name) -> bool {
        if self.surname_index == 0 || other.surname_index == 0 {
            return true;
        }

        // We know the initials are equal or one is a superset of the other if
        // we've got this far, so we know the longer string includes the first
        // letters of all the given & middle names
        let initials = if self.initials().len() > other.initials().len() {
            self.initials()
        } else {
            other.initials()
        };

        // We split on hyphens, but not other word boundaries that weren't already
        // split on, because we have done the same when setting initials
        let mut my_words = self
            .words[0..self.surname_index]
            .iter()
            .flat_map(|w|w.split('-'))
            .peekable();

        let mut their_words = other
            .words[0..other.surname_index]
            .iter()
            .flat_map(|w|w.split('-'))
            .peekable();

        // Align words using their initials, and for each initial where we know
        // the word for both names, require that the words are an exact match
        // (ignoring case etc), or that one is a prefix of the other with length
        // >= MIN_GIVEN_NAME_CHAR_MATCH.
        //
        // In cases where we have a word for name A but not for name B, if the
        // prior word for name A was just a prefix of the prior word for name B,
        // require the current word for name A to match the rest of the prior
        // word for name B (to catch cases like Jinli == Jin-Li == Jin Li,
        // != Jin-Yi)
        for initial in initials.chars() {
            if my_words.peek().is_none() || their_words.peek().is_none() {
                // None of the name-words were inconsistent
                return true;
            }

            // Only look at a name-word that corresponds to this initial; if the
            // next word doesn't, it means we only have an initial for this word
            // in a given version of the name
            let mut my_word: Option<&str> = if my_words.peek().unwrap().starts_with(initial) {
                my_words.next()
            } else {
                None
            };

            let mut their_word: Option<&str> = if their_words.peek().unwrap().starts_with(initial) {
                their_words.next()
            } else {
                None
            };

            // If we have two names for the same initial, require that they are
            // equal, ignoring case, accents, and non-alphabetic chars, or
            // that one starts with the other with an overlap of at least three
            // characters
            if my_word.is_some() && their_word.is_some() {
                macro_rules! lowercase_alpha_chars {
                    ($word:expr) => {
                        $word.unwrap().chars().filter_map(lowercase_if_alpha)
                    }
                }

                let mut my_chars = lowercase_alpha_chars!(my_word);
                let mut their_chars = lowercase_alpha_chars!(their_word);
                let mut matching_chars = 0;

                let mut my_char = my_chars.next();
                let mut their_char = their_chars.next();
                let mut exact_match = false;

                loop {
                    if my_char.is_none() && their_char.is_none() {
                        // The words matched exactly, try the next word
                        exact_match = true;
                        break;
                    } else if my_char.is_none() {
                        // My word is a prefix of their word, check my next word
                        // against the rest of their word *iff* they're out of
                        // words and I'm not
                        if their_words.peek().is_none() && !my_words.peek().is_none() {
                            my_word = my_words.next();
                            my_chars = lowercase_alpha_chars!(my_word);
                            my_char = my_chars.next();
                        } else {
                            break;
                        }
                    } else if their_char.is_none() {
                        // Their word is a prefix of my word, check their next
                        // word against the rest of my word *iff* I'm out of
                        // words and they're not
                        if my_words.peek().is_none() && !their_words.peek().is_none() {
                            their_word = their_words.next();
                            their_chars = lowercase_alpha_chars!(their_word);
                            their_char = their_chars.next();
                        } else {
                            break;
                        }
                    } else if my_char != their_char {
                        // We found a conflict and can short-circuit
                        return false;
                    } else {
                        // Characters matched, continue the inner loop
                        matching_chars += 1;
                        my_char = my_chars.next();
                        their_char = their_chars.next();
                    }
                }

                if !exact_match && matching_chars < MIN_GIVEN_NAME_CHAR_MATCH {
                    return false;
                }
            }
        }

        true
    }

    fn surname_consistent(&self, other: &Name) -> bool {
        let mut my_words = self
            .surnames()
            .iter()
            .flat_map(|w| w.unicode_words() )
            .rev();

        let mut their_words = other
            .surnames()
            .iter()
            .flat_map(|w| w.unicode_words() )
            .rev();

        let mut my_word = my_words.next();
        let mut their_word = their_words.next();
        let mut matching_chars = 0;

        // Require either an exact match (ignoring case etc), or a partial match
        // of len >= MIN_SURNAME_CHAR_MATCH and breaking on a word boundary
        loop {
            // No words remaining for some surname - that's ok if it's true of
            // both, or if the components that match are long enough
            if my_word.is_none() || their_word.is_none() {
                return my_word == their_word || matching_chars >= MIN_SURNAME_CHAR_MATCH;
            }

            macro_rules! reverse_lowercase_alpha_chars {
                ($word:expr) => {
                    $word.unwrap().chars().rev().filter_map(lowercase_if_alpha)
                }
            }

            let mut my_chars = reverse_lowercase_alpha_chars!(my_word);
            let mut their_chars = reverse_lowercase_alpha_chars!(their_word);

            let mut my_char = my_chars.next();
            let mut their_char = their_chars.next();

            loop {
                if my_char.is_none() && their_char.is_none() {
                    // The words matched exactly, try the next word
                    my_word = my_words.next();
                    their_word = their_words.next();
                    break;
                } else if my_char.is_none() {
                    // My word is a suffix of their word, check my next word
                    // against the rest of their word
                    my_word = my_words.next();
                    if my_word.is_none() {
                        // There is no next word, so this is a suffix-only match,
                        // and we don't allow those
                        return false;
                    } else {
                        // Continue the inner loop but incrementing through my
                        // next word
                        my_chars = reverse_lowercase_alpha_chars!(my_word);
                        my_char = my_chars.next();
                    }
                } else if their_char.is_none() {
                    // Their word is a suffix of my word, check their next word
                    // against the rest of my_words
                    their_word = their_words.next();
                    if their_word.is_none() {
                        // There is no next word, so this is a suffix-only match,
                        // and we don't allow those
                        return false;
                    } else {
                        // Continue the inner loop but incrementing through their
                        // next word
                        their_chars = reverse_lowercase_alpha_chars!(their_word);
                        their_char = their_chars.next();
                    }
                } else if my_char != their_char {
                    // We found a conflict and can short-circuit
                    return false;
                } else {
                    // Characters matched, continue the inner loop
                    matching_chars += 1;
                    my_char = my_chars.next();
                    their_char = their_chars.next();
                }
            }
        }
    }

    fn suffix_consistent(&self, other: &Name) -> bool {
        self.suffix().is_none() || other.suffix().is_none() || self.suffix() == other.suffix()
    }
}
