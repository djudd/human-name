use regex::Regex;

lazy_static! {
    static ref NICKNAME: Regex = {
        Regex::new(r#"(?i)('[^']+')|("[^"]+")|\([^\)]+\)"#)
    }.ok().unwrap();
}

pub fn is_nickname(word: &str) -> bool {
    NICKNAME.is_match(word)
}
