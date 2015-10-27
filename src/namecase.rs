use itertools::Itertools;

pub fn namecase(word: &str) -> String {
    let result: String = word.chars().enumerate().filter_map( |(i,c)|
        if i == 0 {
            c.to_uppercase().next()
        } else {
            c.to_lowercase().next()
        }
    ).collect();
    result
}

pub fn namecase_and_join(words: &[&str]) -> String {
    words
        .iter()
        .map( |w| namecase(w) )
        .join(" ")
}
