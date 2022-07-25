use super::case::*;
use super::Name;
use smallvec::SmallVec;
use std::borrow::Cow;

impl Name {
    /// Does this name appear to match a munged string such as an email
    /// localpart or URL slug, where whitespace has been removed?
    ///
    /// # Examples
    ///
    /// ```
    /// use human_name::Name;
    /// let name = Name::parse("Jane A. Doe").unwrap();
    ///
    /// assert!(name.matches_slug_or_localpart("jane.doe"));
    /// assert!(!name.matches_slug_or_localpart("john.doe"));
    ///
    /// assert!(name.matches_slug_or_localpart("janedoe"));
    /// assert!(!name.matches_slug_or_localpart("johndoe"));
    ///
    /// assert!(name.matches_slug_or_localpart("jad"));
    /// assert!(!name.matches_slug_or_localpart("jd"));
    ///
    /// assert!(name.matches_slug_or_localpart("janed"));
    /// assert!(!name.matches_slug_or_localpart("jane"));
    /// assert!(!name.matches_slug_or_localpart("johnd"));
    ///
    /// ```
    #[deprecated(
        since = "1.1.1",
        note = "This functionality is incomplete and unsupported, and retained only for backwards compatibility"
    )]
    pub fn matches_slug_or_localpart(&self, string: &str) -> bool {
        if string.is_empty() {
            return false;
        }

        // Special case: Nice punctuation lets us actually parse a name directly
        if string.chars().any(Self::is_nonalphanumeric) {
            let subbed = string
                .split(Self::is_nonalphanumeric)
                .filter(|p| !p.is_empty())
                .collect::<SmallVec<[&str; 5]>>()
                .join(" ");

            if let Some(name) = Name::parse(&subbed) {
                if name.consistent_with(self) {
                    return true;
                }
            }
        }

        let normed: Cow<str> = if string
            .chars()
            .all(|c| c.is_alphabetic() && c.is_lowercase())
        {
            Cow::Borrowed(string)
        } else {
            Cow::Owned(
                string
                    .chars()
                    .filter_map(Self::lowercase_if_alpha)
                    .collect::<String>(),
            )
        };

        if normed.is_empty() {
            return false;
        }

        // Special case: Full initials
        let full_initials_len = usize::from(self.initials_len + self.surname_words);
        if full_initials_len > 2 && normed.len() == full_initials_len {
            let mut initials = String::with_capacity(full_initials_len);
            initials.extend(self.initials().chars().flat_map(char::to_lowercase));
            initials.extend(
                self.surname_iter()
                    .filter_map(|n| n.chars().next())
                    .flat_map(char::to_lowercase),
            );

            if *normed == initials {
                return true;
            }
        }

        // Special case: Given name plus surname initial
        if let Some(name) = self.given_name() {
            let name_and_initial_len = name.len() + usize::from(self.surname_words);
            if normed.len() == name_and_initial_len {
                let mut name_and_initial = String::with_capacity(name_and_initial_len);
                name_and_initial.extend(name.chars().flat_map(char::to_lowercase));
                name_and_initial.extend(
                    self.surname_iter()
                        .filter_map(|n| n.chars().next())
                        .flat_map(char::to_lowercase),
                );

                if *normed == name_and_initial {
                    return true;
                }
            }
        }

        // Now, the default case:
        //
        // We find as much of the surname as we can, treat the rest of the input
        // as prefix and suffix, and examine those to see if they might match the
        // rest of the name.
        let search_result = self.find_surname_in(&normed);
        if search_result.is_none() {
            return false;
        }

        let (match_begin, match_len, found_exact_surname) = search_result.unwrap();

        let prefix = if match_begin > 0 {
            Some(&normed[0..match_begin])
        } else {
            None
        };

        let suffix = if match_begin + match_len < normed.len() {
            Some(&normed[match_begin + match_len..])
        } else {
            None
        };

        if prefix.map(|s| s.len()).unwrap_or(0) < 2 && suffix.map(|s| s.len()).unwrap_or(0) < 2 {
            // Don't allow just a two-letter surname match to result in an overall match
            if match_len < 3 {
                return false;
            }

            // Don't allow just a 3 or 4-char part-surname match to result in an overall match
            if match_len < 5 && !found_exact_surname {
                return false;
            }
        }

        let allow_unknowns = found_exact_surname && (prefix.is_none() || suffix.is_none());

        (prefix.is_none() || self.matches_remaining_name_parts(prefix.unwrap(), allow_unknowns))
            && (suffix.is_none()
                || self.matches_remaining_name_parts(suffix.unwrap(), allow_unknowns))
    }

    fn find_surname_in(&self, haystack: &str) -> Option<(usize, usize, bool)> {
        let lower_surname: String = self
            .surname_iter()
            .flat_map(|n| n.chars().filter_map(Self::lowercase_if_alpha))
            .collect();
        if lower_surname.len() < 2 {
            return None;
        }

        let mut match_begin = haystack.rfind(&lower_surname);
        let mut match_len = lower_surname.len();

        while match_begin.is_none() {
            match_len -= lower_surname[0..match_len]
                .chars()
                .next_back()
                .unwrap()
                .len_utf8();
            if match_len > 2 {
                match_begin = haystack.rfind(&lower_surname[0..match_len]);
            } else {
                break;
            }
        }

        match_begin.map(|i| (i, match_len, match_len == lower_surname.len()))
    }

    fn matches_remaining_name_parts(&self, part: &str, allow_unknowns: bool) -> bool {
        let lower_first_initial = self.first_initial().to_lowercase().next().unwrap();
        let given_names: Option<Cow<str>> = if self.given_name_words == 1 {
            self.given_name().map(Cow::Borrowed)
        } else if self.given_name_words > 0 {
            Some(self.given_iter().join())
        } else {
            None
        };

        if let Some(ref name) = given_names {
            // Allow just given name, or partial given name, as part
            if name.len() >= part.len() && eq_casefolded_alpha_prefix(part, name) {
                return true;
            }
        } else if allow_unknowns {
            // Allow possible given name starting with first initial when given
            // name is unknown and surname matched exactly
            if part.starts_with(lower_first_initial) {
                return true;
            }
        }

        if self.middle_initials().is_some() {
            // Allow just initials, or partial initials, as part
            if self.initials().len() >= part.len()
                && eq_casefolded_alpha_prefix(part, self.initials())
            {
                return true;
            }
        } else if allow_unknowns {
            // Allow possible initials starting with first initial when middle
            // initials are unknown and surname matched exactly (assuming maximum
            // likely number of first & middle initials is three)
            if part.len() < 4 && part.starts_with(lower_first_initial) {
                return true;
            }
        }

        if let Some(ref name) = given_names {
            if part.len() > name.len() && eq_casefolded_alpha_prefix(part, name) {
                let remainder = &part[name.len()..];

                // Allow given name *plus* middle initials as part (with heuristic
                // when middle initials are unknown and surname matched exactly,
                // assuming maximum likely number of middle initials is two)
                if let Some(initials) = self.middle_initials() {
                    if initials.len() >= remainder.len()
                        && eq_casefolded_alpha_prefix(remainder, initials)
                    {
                        return true;
                    }
                } else if allow_unknowns && remainder.len() < 3 {
                    return true;
                }
            }
        }

        if let Some(initials) = self.middle_initials() {
            if part.len() > initials.len() && eq_casefolded_alpha_suffix(initials, part) {
                let remainder = &part[0..part.len() - initials.len()];

                // Allow partial given name, plus known middle initials, as part
                if let Some(name) = self.given_name() {
                    if eq_casefolded_alpha_prefix(remainder, name) {
                        return true;
                    }
                }
            }
        }

        if self.goes_by_middle_name()
            && part.len() == lower_first_initial.len_utf8()
            && part.starts_with(lower_first_initial)
        {
            return true;
        }

        false
    }

    fn lowercase_if_alpha(c: char) -> Option<char> {
        if c.is_uppercase() {
            c.to_lowercase().next()
        } else if c.is_alphabetic() {
            Some(c)
        } else {
            None
        }
    }

    // Sadly necessary because string split gives "type of this value must be known"
    // compilation error when passed a closure in some contexts
    fn is_nonalphanumeric(c: char) -> bool {
        !c.is_alphanumeric()
    }
}
