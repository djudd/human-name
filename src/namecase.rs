use super::case::capitalize_word;
use ahash::AHashSet;
use once_cell::sync::Lazy;
use phf::phf_set;

// Stores capitalized versions because we check after doing the initial,
// naive capitalization
static UNCAPITALIZED_PARTICLES: Lazy<AHashSet<&'static str>> = Lazy::new(|| {
    let mut set = AHashSet::new();
    include!(concat!(env!("OUT_DIR"), "/uncapitalized_particles.rs"));
    set
});

static MAC_EXCEPTIONS: phf::Set<&'static str> = phf_set! {
    "Machin",
    "Machlin",
    "Machar",
    "Mackle",
    "Macklin",
    "Mackie",
    "Macevicius",
    "Maciulis",
    "Macias",
};

#[allow(clippy::if_same_then_else)]
fn capitalize_after_mac(word: &str) -> bool {
    if word.len() <= 4 {
        false
    } else if word.ends_with('o') && word != "Macmurdo" {
        false
    } else if ["a", "c", "i", "z", "j"].iter().any(|c| word.ends_with(c)) {
        false
    } else {
        !MAC_EXCEPTIONS.contains(word)
    }
}

pub fn namecase(word: &str, ascii_alpha: bool, might_be_particle: bool) -> String {
    let result = capitalize_word(word, ascii_alpha);

    if might_be_particle && UNCAPITALIZED_PARTICLES.contains(&*result) {
        result.to_lowercase()
    } else if result.starts_with("Mac") && capitalize_after_mac(&result) {
        "Mac".to_string() + &capitalize_word(&result[3..], ascii_alpha)
    } else if result.starts_with("Mc") && result.len() > 3 {
        "Mc".to_string() + &capitalize_word(&result[2..], ascii_alpha)
    } else if result.starts_with("Al-") && result.len() > 3 {
        "al-".to_string() + &result[3..]
    } else {
        // Normal case
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple() {
        assert_eq!("Doe", namecase("doe", true, true));
    }

    #[test]
    fn conjunction() {
        assert_eq!("y", namecase("y", true, true));
        assert_eq!("Y", namecase("y", true, false));
    }

    #[test]
    fn particle() {
        assert_eq!("de", namecase("de", true, true));
        assert_eq!("De", namecase("de", true, false));
        assert_eq!("dí", namecase("dí", false, true));
    }

    #[test]
    fn mcallen() {
        assert_eq!("McAllen", namecase("mcallen", true, true));
    }

    #[test]
    fn macmurdo() {
        assert_eq!("MacMurdo", namecase("macmurdo", true, true));
    }

    #[test]
    fn machlin() {
        assert_eq!("Machlin", namecase("machlin", true, true));
    }

    #[test]
    fn maciej() {
        assert_eq!("Maciej", namecase("maciej", true, true));
    }

    #[test]
    fn mach() {
        assert_eq!("Mach", namecase("mach", true, true));
    }

    #[test]
    fn macadaidh() {
        assert_eq!("MacAdaidh", namecase("macadaidh", true, true));
    }

    #[test]
    fn al_amir() {
        assert_eq!("al-Amir", namecase("al-amir", false, true));
    }
}
