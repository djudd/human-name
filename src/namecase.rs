use std::collections::HashSet;
use super::utils::capitalize;

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
            "E",
            "Y"
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
    let result = capitalize(word);

    if might_be_particle && UNCAPITALIZED_PARTICLES.contains(&*result) {
        result.to_lowercase()
    } else if result.starts_with("Mac") && result.len() > 4 && !MAC_EXCEPTIONS.contains(&*result) {
        "Mac".to_string() + &capitalize(&result[3..])
    } else if result.starts_with("Mc") && result.len() > 3 {
        "Mc".to_string() + &capitalize(&result[2..])
    } else if result.starts_with("O'") && result.len() > 2 {
        "O'".to_string() + &capitalize(&result[2..])
    } else {
        // Normal case
        result
    }
}
