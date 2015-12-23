use std::ascii::AsciiExt;
use std::borrow::Cow;
use super::utils::*;
use super::nickname::have_matching_variants;
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
    ///
    /// let zheng_he = Name::parse("Zheng He").unwrap();
    /// let han_chars = Name::parse("鄭和").unwrap();
    /// assert!(han_chars.consistent_with(&zheng_he));
    /// ```
    ///
    /// # Defining "consistency"
    ///
    /// Requires that all known parts are consistent, which means at minimum,
    /// the final words of the surnames match, and one ordered set of first
    /// and middle initials is a superset of the other. If given and/or middle
    /// names and/or suffixes are present in both names, they must match as well.
    ///
    /// Transliterates everything to ASCII before comparison using the naive
    /// algorithm of [unidecode](https://github.com/chowdhurya/rust-unidecode/)
    /// (which ignores context), and ignores case, accents and combining marks.
    ///
    /// In the case of given and middle names, allows one name to be a prefix of
    /// the other, without requiring the prefix end at a word boundary as we do
    /// with surname suffix matches, and supports matching a small number of
    /// common nicknames and nickname patterns based on the root name.
    ///
    /// # Limitations
    ///
    /// There will be false positives ("Jan Doe" is probably not "Jane Doe"),
    /// and false negatives ("James Hanson" might be "James Hansen"). And, of
    /// course, even identical names do not necessarily represent the same person.
    ///
    /// Given limited information, we err on the side of false positives. This
    /// kind of matching will be most useful in cases where we already have
    /// reason to believe that a single individual's name appears twice, and we
    /// are trying to figure out exactly where, e.g. a particular author's index
    /// in the list of authors of a co-authored paper.
    ///
    #[cfg_attr(rustfmt, rustfmt_skip)]
    pub fn consistent_with(&self, other: &Name) -> bool {
        // Fast path
        if self.memoized_surname_hash() != other.memoized_surname_hash() {
            return false;
        }

        // Check given name(s) first because if we got this far, we know that
        // at least the last characters of the surnames are consistent
        self.given_and_middle_names_consistent(other) &&
        self.surname_consistent(other) &&
        self.suffix_consistent(other)
    }

    fn given_and_middle_names_consistent(&self, other: &Name) -> bool {
        if self.initials().len() >= other.initials().len() {
            self.given_and_middle_names_consistent_with_less_complete(other)
        } else {
            other.given_and_middle_names_consistent_with_less_complete(self)
        }
    }

    fn given_and_middle_names_consistent_with_less_complete(&self, other: &Name) -> bool {
        // Handle simple cases first, where we only have to worry about one name
        // and/or initial.
        if self.middle_initials().is_none() && other.middle_initials().is_none() {
            if self.given_name().is_none() || other.given_name().is_none() {
                return to_ascii_letter(self.first_initial()) ==
                       to_ascii_letter(other.first_initial());
            } else {
                return have_matching_variants(self.given_name().unwrap(),
                                              other.given_name().unwrap());
            }
        }

        // Unless we have both given names, we require that the first initials
        // match (if we do have the names, we'll check for nicknames, etc,
        // which might violate this requirement.)
        let any_missing_given_name = self.given_name().is_none() ||
                                     other.given_name().is_none() ||
                                     self.goes_by_middle_name() ||
                                     other.goes_by_middle_name();

        let my_initials = &*self.transliterated_initials();
        let their_initials = &*other.transliterated_initials();

        if any_missing_given_name {
            if self.goes_by_middle_name() {
                // Edge case: if we go by a middle name, another version of the
                // name might use the middle name's initial as the first initial.
                //
                // However, only check in this direction because we know `self`
                // has more complete initials, so their initials won't contain
                // ours unless they're equal.
                if !my_initials.contains(&their_initials) {
                    return false;
                }
            } else {
                // Normal case, given the absence of (true) given names
                //
                // Using byte offsets is ok because we already converted to ASCII
                if my_initials.chars().nth(0) != their_initials.chars().nth(0) {
                    return false;
                }
            }
        }

        // If we have middle initials, check their consistency alone before
        // looking at names (& we know "self" has the more complete initials).
        if self.middle_initials().is_some() && other.middle_initials().is_some() {
            // Using byte offsets is ok because we already converted to ASCII
            if !my_initials[1..].contains(&their_initials[1..]) {
                return false;
            }
        }

        // Unless both versions of the name have given or middle names, that's
        // all we have to do.
        if self.surname_index == 0 || other.surname_index == 0 {
            return true;
        }

        // In the case where we have to compare multiple given or middle names,
        // align words using their initials, and for each initial where we know
        // the word for both names, transliterate to ASCII lowercase, then require
        // that the words are an exact match, or that one is a prefix of the other,
        // or that one is a recognized nickname or spelling variant of the other.

        let mut their_initials = their_initials.chars().peekable();
        let mut their_initial_index = 0;
        let mut their_word_indices = other.word_indices_in_initials.iter().peekable();
        let mut their_words = other.words.iter();
        let mut suffix_for_prior_prefix_match: Option<&str> = None;
        let mut consistency_refuted = false;
        let mut looked_up_nicknames = false;

        self.with_each_given_name_or_initial(&mut |my_part| {
            macro_rules! require_word_match {
                ($my_word:expr, $their_word:expr) => { {
                    let mut my_chars = $my_word.chars().flat_map(transliterate).filter_map(lowercase_if_alpha);
                    let mut their_chars = $their_word.chars().flat_map(transliterate).filter_map(lowercase_if_alpha);
                    let mut matched = 0;

                    loop {
                        let my_char = my_chars.next();
                        let their_char = their_chars.next();

                        if my_char.is_none() && their_char.is_none() {
                            // Exact match; continue
                            suffix_for_prior_prefix_match = None;
                            return;
                        } else if (my_char.is_none() || their_char.is_none()) && matched >= MIN_GIVEN_NAME_CHAR_MATCH {
                            // Prefix match; continue
                            if my_char.is_none() && their_initials.peek().is_none() {
                                // We'll only use this when the name with fewer words
                                // (theirs) is out of words, but the last matching part
                                // of the name with more words (ours) was only a prefix
                                // (see NOTE below)
                                suffix_for_prior_prefix_match = Some(&$their_word[matched..]);
                            }
                            return;
                        } else if my_char != their_char {
                            // Failed match; abort, but first, if we haven't before,
                            // try nickname database (we only allow one nickname
                            // per full name)
                            consistency_refuted = looked_up_nicknames || !have_matching_variants($my_word, $their_word);
                            looked_up_nicknames = true;
                            return;
                        } else {
                            matched += 1;
                        }
                    }
                } }
            }

            macro_rules! skip_n_of_their_initials {
                ($count:expr) => {
                    for _ in 0..$count {
                        their_initials.next();
                        their_initial_index += 1;
                    }
                }
            }

            if consistency_refuted {
                // We already found words that fail to match
                return;
            }

            if suffix_for_prior_prefix_match.is_some() {
                // NOTE It's not uncommon for representations of a name to be
                // inconsistent in whether two parts of a given name are separated
                // by a space, a hyphen, or nothing (especially with transliterated
                // names).
                //
                // This is one of the reasons we accept prefix-only matches for
                // given & middle names. However, doing so potentially opens us up
                // to more false positives, and we want to mitigate that.
                //
                // When it looks like we have a name part following a space or hyphen
                // which might reasonably be treated as part of the same actual name
                // as the preceding part, we don't want to just accept a prefix-only
                // match on the preceding part as sufficient for a full match.
                // Instead we'll continue after the prefix match by comparing the
                // following part to the suffix, just as if the two parts hadn't
                // been separated by a space or hyphen.
                //
                // This separates, e.g., "Jinli" from "Jin Yi", or "Xiaofeng" from
                // "Xiao Peng".
                //
                // This logic is imperfect in the presence of middle initials,
                // but that's an edge case to an edge case.
                if let NameWordOrInitial::Word(word) = my_part {
                    consistency_refuted = !eq_or_starts_with!(suffix_for_prior_prefix_match.unwrap(), word);
                    return;
                }
            }

            if their_initials.peek().is_none() {
                // We've matched everything available
                return;
            }

            let my_initial = to_ascii_letter(my_part.initial());
            let their_initial = *their_initials.peek().unwrap();
            if my_initial.is_none() || my_initial.unwrap() != their_initial {
                // We have an initial they don't, or an invalid one, continue
                return;
            }

            // The initials match, so we know we'll want to increment both
            skip_n_of_their_initials!(my_part.initial_count());

            if their_word_indices.peek().is_none() || their_word_indices.peek().unwrap().0 >= their_initial_index {
                // They don't have a word for this initial
                return;
            }

            let &(j, k) = their_word_indices.next().unwrap();

            // Edge case: Their word is hyphenated and so corresponds to
            // multiple initials (but ours isn't, or this would be redundant)
            if (k - j) > my_part.initial_count() {
                skip_n_of_their_initials!(k - j - my_part.initial_count());
            }

            let their_word = their_words.next().unwrap();

            match my_part {
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

        !consistency_refuted && their_initials.peek().is_none()
    }

    fn transliterated_initials(&self) -> Cow<str> {
        if self.initials().is_ascii() {
            Cow::Borrowed(self.initials())
        } else {
            Cow::Owned(self.initials()
                .chars()
                .filter_map(to_ascii_letter)
                .collect::<String>())
        }
    }

    fn surname_consistent(&self, other: &Name) -> bool {
        // Fast path
        if self.simple_surname() && other.simple_surname() {
            return self.surname().eq_ignore_ascii_case(&*other.surname());
        }

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
            if my_word.is_none() && their_word.is_none() {
                return true;
            } else if my_word.is_none() || their_word.is_none() {
                return matching_chars >= MIN_SURNAME_CHAR_MATCH;
            }

            macro_rules! reverse_lowercase_alpha_chars {
                ($word:expr) => {
                    $word.unwrap().chars().flat_map(transliterate).rev().filter_map(lowercase_if_alpha)
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

    fn simple_surname(&self) -> bool {
        self.surnames().len() == 1 && self.surname().chars().all(is_ascii_alphabetic)
    }

    fn suffix_consistent(&self, other: &Name) -> bool {
        self.generation_from_suffix.is_none() || other.generation_from_suffix.is_none() ||
        self.generation_from_suffix == other.generation_from_suffix
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

    pub fn initial_count(&self) -> usize {
        match self {
            &NameWordOrInitial::Word(word) => {
                // TODO Replace this hack
                word.chars().filter(|c| *c == '-').count() + 1
            }
            &NameWordOrInitial::Initial(_) => {
                1
            }
        }
    }
}
