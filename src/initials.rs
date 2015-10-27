const VOWELS: [char; 12] = ['a','e','i','o','u','y','A','E','I','O','U','Y'];

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

    if use_capitalization {
        // The context tells us that capitalization is meaningful in this name,
        // so assume short all-caps strings are initials, and others aren't
        word.len() < 5 && word.chars().all(|c| c.is_uppercase())
    }
    else {
        // The context tells us capitalization isn't meaningful here, so do our
        // best to distinguish initials from short given-names by checking for vowels
        // and non-alphabetic characters
        //
        // TODO We should de-accent before the vowel-check, and/or determine
        // that the string is all-ASCII
        word.len() < 4 && word.chars().all(|c| !c.is_alphabetic() || !VOWELS.contains(&c))
    }
}
