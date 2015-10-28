use regex::Regex;

lazy_static! {
    static ref NICKNAME: Regex = {
        Regex::new(r#"(?i)('[^']+')|("[^"]+")|\([^\)]+\)"#)
    }.ok().unwrap();
}

pub fn is_nickname(word: &str) -> bool {
    NICKNAME.is_match(word)
}

/*
// Optimized for the case where there is no nickname, and secondarily for the
// case where there is one; in the common case, we want to minimize allocations,
// even if that makes our algorithm non-optimal in the extreme edge case of
// multiple nicknames
pub fn strip_nicknames(s: &str) -> &str {
    let mut nick_start_ix = 0; // This counts as not-found; we won't classify the whole string as a nickname
    let mut expected_close_char = '0';

    let mut iter = s.chars().enumerate().peekable();
    loop {
        match iter.next() {
            Some((i, c)) => {
                if nick_start_ix == 0 {
                    if c == '(' { // TODO opening parens class
                        // Treat opening parens as the start of a nickname
                        // regardless of where it occurs
                        nick_start_ix = i;
                        expected_close_char = ')';
                    }
                    else if c.is_whitespace() {
                        // Treat quote character as the start of a nickname
                        // only if it occurs after whitespace; otherwise, it
                        // might be in-name puntuation
                        let (_, nc) = iter.peek();
                        if nc == '"' { // TODO quote class
                            nick_start_ix = i+1;
                            expected_close_char = '"';
                        }
                    }
                } else if c == expected_close_char {
                    return strip_nicknames(s[0..nick_start_ix] + s[i+1..]);
                }
            },
            None => { break }
        }
    }

    if nick_start_ix > 0 && expected_close_char == ')' {
        // When there's an opening parens, but no closing parens, strip the
        // rest of the string
        s[0..nick_start_ix]
    } else {
        // Otherwise, even if there's an unmatched opening quote, return the
        // string as-is; assume an unmatched opening quote was just in-name
        // punctuation
        s
    }
}*/
