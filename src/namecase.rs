use itertools::Itertools;
use std::collections::HashSet;

lazy_static! {
    // Store capitalized versions because we check after doing the initial,
    // naive capitalization
   
    static ref UNCAPITALIZED_PARTICLES: HashSet<&'static str> = {
        let s: HashSet<&'static str> = [
            "Da",
            "Das",
            "Dal",
            "De",
            "Del",
            "Dela",
            "Der",
            "De",
            "Di",
            "Dí",
            "Do",
            "Dos",
            "La",
            "Le",
            "Ter",
            "Van",
            "Vel",
            "Von",
        ].iter().cloned().collect();
        s
    };

    static ref MAC_EXCEPTIONS: HashSet<&'static str> = {
        let s: HashSet<&'static str> = [
            "Machin",
            "Machlin",
            "Machar",
            "Mackle",
            "Macklin",
            "Mackie",
            "Machado",
            "Macevicius",
            "Maciulis",
            "Macias",
        ].iter().cloned().collect();
        s
    };
}

pub fn namecase(word: &str, might_be_particle: bool) -> String {
    let result: String = word.chars().enumerate().filter_map( |(i,c)|
        if i == 0 {
            c.to_uppercase().next()
        } else {
            c.to_lowercase().next()
        }
    ).collect();

    if might_be_particle && UNCAPITALIZED_PARTICLES.contains(&*result) {
        result.to_lowercase()
    } else if result.starts_with("Mac") && result.len() > 4 && !MAC_EXCEPTIONS.contains(&*result) {
        "Mac".to_string() + &result[3..]
    } else {
        // Normal case
        result
    }
}

pub fn namecase_and_join(words: &[&str], might_include_particle: bool) -> String {
    words
        .iter()
        .enumerate()
        .map( |(i,w)| namecase(w, might_include_particle && i < words.len()-1) )
        .join(" ")
}
