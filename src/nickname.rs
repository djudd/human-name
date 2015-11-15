use std::borrow::Cow;

// Returns tuple (close_char, must_precede_whitespace)
fn expected_close_char_if_opens_nickname(c: char,
                                         follows_whitespace: bool)
                                         -> Option<(char, bool)> {
    let close = match c {
        '(' => Some((')', false)),
        '[' => Some((']', false)),
        '<' => Some(('>', false)),
        '“' => Some(('”', false)),
        '〝' => Some(('〞', false)),
        '‹' => Some(('›', false)),
        '«' => Some(('»', false)),
        _ => None,
    };

    if !close.is_none() {
        // Treat, e.g., opening parens as the start of a nickname
        // regardless of where it occurs
        return close;
    }

    if follows_whitespace {
        // Treat, e.g., quote character as the start of a nickname
        // only if it occurs after whitespace; otherwise, it
        // might be in-name puntuation
        match c {
            '\'' => Some(('\'', true)),
            '"' => Some(('"', true)),
            '‘' => Some(('’', true)),
            _ => None,
        }
    } else {
        None
    }
}

fn starts_with_whitespace(text: &str) -> bool {
    text.chars().nth(0).unwrap().is_whitespace()
}

// Optimized for the case where there is no nickname, and secondarily for the
// case where there is only one. Two or more probably means bad input.
pub fn strip_nickname(input: &str) -> Cow<str> {
    let mut nick_start_ix = None;
    let mut nick_open_char = '\0';
    let mut expected_close_char = '\0';
    let mut must_precede_whitespace = false;
    let mut prev_char = '\0';

    for (i, c) in input.char_indices() {
        if nick_start_ix.is_none() {
            let close = expected_close_char_if_opens_nickname(c, prev_char.is_whitespace());
            if !close.is_none() {
                nick_start_ix = Some(i);
                nick_open_char = c;
                expected_close_char = close.unwrap().0;
                must_precede_whitespace = close.unwrap().1
            }
        } else if c == expected_close_char {
            let j = i + c.len_utf8();
            if j >= input.len() {
                return Cow::Borrowed(&input[0..nick_start_ix.unwrap()]);
            } else if !must_precede_whitespace || starts_with_whitespace(&input[j..]) {
                return Cow::Owned(input[0..nick_start_ix.unwrap()].to_string() +
                                  &strip_nickname(&input[j..]));
            } else {
                return Cow::Owned(input[0..i].to_string() + &strip_nickname(&input[i..]));
            }
        }

        prev_char = c;
    }

    if nick_start_ix.is_some() {
        if !must_precede_whitespace {
            // When there's, e.g., an opening parens, but no closing parens, strip the
            // rest of the string
            return Cow::Borrowed(&input[0..nick_start_ix.unwrap()]);
        } else {
            let i = nick_start_ix.unwrap() + nick_open_char.len_utf8();
            // Otherwise, even if there's an unmatched opening quote, don't
            // modify the string; assume an unmatched opening quote was just
            // in-name punctuation
            //
            // However, in that case, we need to check the remainder of the
            // string for actual nicknames, whose opening character we might
            // have missed while looking for the first closing character
            if i >= input.len() {
                return Cow::Borrowed(input);
            } else {
                return Cow::Owned(input[0..i].to_string() + &strip_nickname(&input[i..]));
            }
        }
    }

    Cow::Borrowed(input)
}
