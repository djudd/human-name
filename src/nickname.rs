use std::borrow::Cow;

fn expected_close_char_if_opens_nickname(c: char, prev: char) -> Option<char> {
    let close = match c {
        '(' => Some(')'),
        '[' => Some(']'),
        '<' => Some('>'),
        '“' => Some('”'),
        '〝' => Some('〞'),
        '‹' => Some('›'),
        _ => None
    };

    if !close.is_none() {
        // Treat, e.g., opening parens as the start of a nickname
        // regardless of where it occurs
        return close;
    }

    if prev.is_whitespace() {
        // Treat, e.g., quote character as the start of a nickname
        // only if it occurs after whitespace; otherwise, it
        // might be in-name puntuation
        match c {
            '\'' => Some('\''),
            '"' => Some('"'),
            '‘' => Some('’'),
            _ => None
        }
    } else {
        None
    }
}

pub fn second_char_index(s: &str) -> Option<usize> {
    let mut iter = s.char_indices();

    let first = iter.next();
    if first.is_none() { return None; }

    let second = iter.next();
    if second.is_none() { return None; }

    Some(second.unwrap().0)
}

// Optimized for the case where there is no nickname, and secondarily for the
// case where there is only one. Two or more probably means bad input.
pub fn strip_nickname(input: &str) -> Cow<str> {
    let mut nick_start_ix = 0; // This counts as not-found; we won't classify the whole string as a nickname
    let mut expected_close_char = '\0';
    let mut prev_char = '\0';

    for (i,c) in input.char_indices() {
        if nick_start_ix == 0 {
            let close = expected_close_char_if_opens_nickname(c, prev_char);
            if !close.is_none() {
                nick_start_ix = i;
                expected_close_char = close.unwrap();
            }
        } else if c == expected_close_char {
            match second_char_index(&input[i..]) {
                Some(j) => {
                    return Cow::Owned(input[0..nick_start_ix].to_string() + " " + &strip_nickname(&input[i+j..]));
                }
                None => {
                    return Cow::Borrowed(&input[0..nick_start_ix]);
                }
            }
        }

        prev_char = c;
    }

    if nick_start_ix > 0 {
        if expected_close_char == ')' {
            // When there's an opening parens, but no closing parens, strip the
            // rest of the string
            return Cow::Borrowed(&input[0..nick_start_ix]);
        } else { match second_char_index(&input[nick_start_ix..]) {
            // Otherwise, even if there's an unmatched opening quote, don't
            // modify the string; assume an unmatched opening quote was just
            // in-name punctuation
            //
            // However, in that case, we need to check the remainder of the
            // string for actual nicknames, whose opening character we might
            // have missed while looking for the first closing character
            Some(i) => {
                return Cow::Owned(input[0..nick_start_ix+i].to_string() + " " + &strip_nickname(&input[nick_start_ix+i..]));
            }
            None => {
                return Cow::Borrowed(input);
            }
        } }
    }

    Cow::Borrowed(input)
}
