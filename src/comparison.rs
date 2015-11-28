use super::utils::*;
use super::{Name, NameWordOrInitial};
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
        // and for correctness (the given/middle names check assumes a positive
        // result for the middle initials check)
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
        // we've got this far
        if self.initials().len() >= other.initials().len() {
            self.given_and_middle_names_consistent_with_less_complete(other)
        } else {
            other.given_and_middle_names_consistent_with_less_complete(self)
        }
    }

    fn given_and_middle_names_consistent_with_less_complete(&self, other: &Name) -> bool {
        // Align words using their initials, and for each initial where we know
        // the word for both names, require that the words are an exact match
        // (ignoring case etc), or that one is a prefix of the other with length
        // >= MIN_GIVEN_NAME_CHAR_MATCH.

        let mut their_initials = other.initials().chars().peekable();
        let mut their_initial_index = 0;
        let mut their_word_indices = other.word_indices_in_initials.iter().peekable();
        let mut their_words = other.words[0..other.suffix_index].iter();
        let mut suffix_for_prior_prefix_match: Option<&str> = None;
        let mut consistency_refuted = false;

        self.with_each_given_name_or_initial( &mut |part, _| {
            macro_rules! require_word_match {
                ($my_word:expr, $their_word:expr) => { {
                    let mut my_chars = $my_word.chars().filter_map(lowercase_if_alpha);
                    let mut their_chars = $their_word.chars().filter_map(lowercase_if_alpha);
                    let mut matched = 0;

                    loop {
                        let my_char = my_chars.next();
                        let their_char = their_chars.next();

                        if my_char.is_none() && their_char.is_none() {
                            // Exact match; continue
                            return;
                        } else if (my_char.is_none() || their_char.is_none()) && matched >= MIN_GIVEN_NAME_CHAR_MATCH {
                            // Prefix match; continue
                            if my_char.is_none() && their_initials.peek().is_none() {
                                // We'll only use this when the name with fewer words
                                // (theirs) is out of words, but the last matching part
                                // of the name with more words (ours) was only a prefix
                                // (see comment below)
                                suffix_for_prior_prefix_match = Some(&$their_word[matched..]);
                            }
                            return;
                        } else if my_char != their_char {
                            // Failed match; abort
                            consistency_refuted = true;
                            return;
                        } else {
                            matched += 1;
                        }
                    }
                } }
            }

            if consistency_refuted {
                // We already found words that fail to match
                return;
            }

            if their_initials.peek().is_none() || their_word_indices.peek().is_none() {
                if suffix_for_prior_prefix_match.is_some() {
                    // Edge case: where we have a word and they don't, if our prior
                    // word was just a prefix of their prior word, require our current
                    // word to match the rest of their prior word (to catch cases
                    // like Jinli == Jin-Li == Jin Li, != Jin Yi).
                    //
                    // This logic is imperfect in the presence of middle initials,
                    // but that's an edge case to an edge case.
                    if let NameWordOrInitial::Word(my_word) = part {
                        require_word_match!(my_word, suffix_for_prior_prefix_match.unwrap());
                    }
                }

                // We've matched everything available
                return;
            }

            let my_initial = part.initial();
            if my_initial != *their_initials.peek().unwrap() {
                // We have an initial they don't, continue
                return;
            }

            // The initials match, so we know we'll want to increment both
            their_initials.next();
            their_initial_index += 1;
            suffix_for_prior_prefix_match = None;

            if their_word_indices.peek().unwrap().0 != their_initial_index - 1 {
                // They don't have a word for this initial
                return;
            }

            let &(j, k) = their_word_indices.next().unwrap();

            // Edge case: Their word is hyphenated and so corresponds to
            // multiple initials
            for _ in j+1..k {
                their_initials.next();
                their_initial_index += 1;
            }

            let their_word = their_words.next().unwrap();

            match part {
                NameWordOrInitial::Initial(_) => {
                    // We don't have a word for this initial
                    return;
                },
                NameWordOrInitial::Word(my_word) => {
                    // Both have words for this initial, so check if they match
                    require_word_match!(my_word, their_word);
                },
            }
        });

        !consistency_refuted
    }

    fn surname_consistent(&self, other: &Name) -> bool {
        let mut my_words = self.surnames()
                               .iter()
                               .flat_map(|w| w.unicode_words())
                               .rev();

        let mut their_words = other.surnames()
                                   .iter()
                                   .flat_map(|w| w.unicode_words())
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

impl<'a> NameWordOrInitial<'a> {
    pub fn initial(&self) -> char {
        match self {
            &NameWordOrInitial::Word(word) => {
                word.chars().nth(0).unwrap()
            }
            &NameWordOrInitial::Initial(initial) => {
                initial
            }
        }
    }
}
