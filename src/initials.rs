use super::utils;

// Fairly lax check, which allows initial or trailing periods, or neither,
// and allows double periods (as likely to be typos as to indicate something
// other than initials), but forbids two neighboring non-periods.
fn is_period_separated(word: &str) -> bool {
    let mut chars = word.chars().peekable();

    loop {
        match chars.next() {
            Some(c) => {
                match chars.peek() {
                    Some(nc) => {
                        if (c != '.') && (*nc != '.') {
                            return false;
                        }
                    }
                    None => { break }
                }
            }
            None => { break }
        }
    }

    true
}

pub fn is_initials(word: &str, use_capitalization: bool) -> bool {
    if word.len() == 1 {
        return true;
    }
    else if is_period_separated(word) {
        return true;
    }
    else if word.len() > 4 {
        return false;
    }

    if use_capitalization {
        // The context tells us that capitalization is meaningful in this name,
        // so assume short all-caps strings are initials, and others aren't
        word.chars().all(|c| !c.is_alphabetic() || c.is_uppercase())
    }
    else {
        // The context tells us capitalization isn't meaningful here, so do our
        // best to distinguish initials from short given-names by checking for
        // vowels
        utils::is_missing_vowels(&word)
    }
}

pub fn first_initial(word: &str) -> char {
    utils::first_alphabetical_char(word)
        .unwrap()
        .to_uppercase()
        .next()
        .unwrap()
}
