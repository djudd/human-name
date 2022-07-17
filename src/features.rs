#[derive(Debug, Clone)]
pub struct CharacterCounts {
    pub chars: u8,
    pub alpha: u8,
    pub upper: u8,
    pub ascii_alpha: u8,
}

pub fn categorize_chars(word: &str) -> CharacterCounts {
    debug_assert!(word.len() <= u8::max_value() as usize);

    let mut chars = 0;
    let mut alpha = 0;
    let mut upper = 0;
    let mut ascii_alpha = 0;

    for c in word.chars() {
        match c {
            'a'..='z' => {
                ascii_alpha += 1;
            }
            'A'..='Z' => {
                ascii_alpha += 1;
                upper += 1;
            }
            _ if c.is_uppercase() => {
                alpha += 1;
                upper += 1;
            }
            _ if c.is_alphabetic() => {
                alpha += 1;
            }
            _ => {
                chars += 1;
            }
        }
    }

    alpha += ascii_alpha;
    chars += alpha;

    CharacterCounts {
        chars,
        alpha,
        upper,
        ascii_alpha,
    }
}

pub fn has_no_vowels(word: &str) -> bool {
    const VOWELS: &[char] = &['a', 'e', 'i', 'o', 'u', 'y', 'A', 'E', 'I', 'O', 'U', 'Y'];
    !word.contains(VOWELS)
}

pub fn starts_with_consonant(word: &str) -> bool {
    word.chars()
        .next()
        .filter(|c| c.is_ascii_alphabetic() && !"aeiouAEIOU".contains(*c))
        .is_some()
}

pub fn starts_with_uppercase(word: &str) -> bool {
    word.chars().next().filter(|c| c.is_uppercase()).is_some()
}

pub fn has_sequential_alphas(word: &str) -> bool {
    let mut prev_alpha = false;
    for c in word.chars() {
        let alpha = c.is_alphabetic();
        if prev_alpha && alpha {
            return true;
        } else {
            prev_alpha = alpha;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequential_alphas() {
        assert!(has_sequential_alphas("ab"));
        assert!(has_sequential_alphas("abc"));
        assert!(has_sequential_alphas("a.bc"));
        assert!(has_sequential_alphas("鄭a"));
        assert!(!has_sequential_alphas(""));
        assert!(!has_sequential_alphas("a"));
        assert!(!has_sequential_alphas("a.b"));
        assert!(!has_sequential_alphas("鄭.a"));
        assert!(!has_sequential_alphas("ﾟ."));
    }
}
