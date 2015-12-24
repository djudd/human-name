use std::borrow::Cow;
use std::iter;
use phf;
use super::utils::*;

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

fn strip_from_index(nick_start_ix: usize, prev_char: char) -> usize {
    if prev_char.is_whitespace() {
        nick_start_ix - prev_char.len_utf8()
    } else {
        nick_start_ix
    }
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
            if close.is_some() {
                nick_start_ix = Some(i);
                nick_open_char = c;
                expected_close_char = close.unwrap().0;
                must_precede_whitespace = close.unwrap().1
            } else {
                prev_char = c;
            }
        } else if c == expected_close_char {
            let j = i + c.len_utf8();
            if j >= input.len() {
                return Cow::Borrowed(&input[0..nick_start_ix.unwrap()]);
            } else if !must_precede_whitespace || starts_with_whitespace(&input[j..]) {
                let strip_from = strip_from_index(nick_start_ix.unwrap(), prev_char);
                return Cow::Owned(input[0..strip_from].to_string() + &strip_nickname(&input[j..]));
            } else {
                return Cow::Owned(input[0..i].to_string() + &strip_nickname(&input[i..]));
            }
        }
    }

    if nick_start_ix.is_some() {
        if !must_precede_whitespace {
            // When there's, e.g., an opening parens, but no closing parens, strip the
            // rest of the string
            let strip_from = strip_from_index(nick_start_ix.unwrap(), prev_char);
            return Cow::Borrowed(&input[0..strip_from]);
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

struct NameVariants<'a> {
    original: &'a str,
    direct_variants: Option<&'a phf::Set<&'static str>>,
    prefix_variants: Option<&'a phf::Set<&'static str>>,
}

impl <'a>NameVariants<'a> {
    pub fn for_name(name: &'a str) -> NameVariants<'a> {
        NameVariants {
            original: name,
            direct_variants: NAMES_BY_IRREGULAR_NICK.get(name),
            prefix_variants: {
                if name.len() >= 4 && (name.ends_with("ie") || name.ends_with("ey")) {
                    NAMES_BY_NICK_PREFIX.get(&name[0..name.len() - 2])
                } else if name.len() >= 3 && name.ends_with('y') {
                    NAMES_BY_NICK_PREFIX.get(&name[0..name.len() - 1])
                } else {
                    None
                }
            },
        }
    }

    pub fn iter_with_original(&self) -> NameVariantIter {
        NameVariantIter {
            original: iter::once(self.original),
            direct_variants: self.direct_variants.map(|names| names.iter()),
            prefix_variants: self.prefix_variants.map(|names| names.iter()),
        }
    }
}

struct NameVariantIter<'a> {
    original: iter::Once<&'a str>,
    direct_variants: Option<phf::set::Iter<'a, &'static str>>,
    prefix_variants: Option<phf::set::Iter<'a, &'static str>>,
}

impl <'a>Iterator for NameVariantIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        if let Some(name) = self.original.next() {
            return Some(name);
        }

        if let Some(ref mut iter) = self.direct_variants {
            if let Some(name) = iter.next() {
                return Some(name);
            }
        }

        if let Some(ref mut iter) = self.prefix_variants {
            if let Some(name) = iter.next() {
                return Some(name);
            }
        }

        None
    }
}

pub fn have_matching_variants(original_a: &str, original_b: &str) -> bool {
    let original_a = to_ascii(original_a);
    let original_b = to_ascii(original_b);

    let a_variants = NameVariants::for_name(&*original_a);
    let b_variants = NameVariants::for_name(&*original_b);

    a_variants.iter_with_original()
              .any(|a| b_variants.iter_with_original().any(|b| variants_match(a, b)))
}

fn variants_match(a: &str, b: &str) -> bool {
    have_prefix_match(a, b) || is_final_syllables_of(a, b) || is_final_syllables_of(b, a) ||
    matches_without_diminutive(a, b) || matches_without_diminutive(b, a)
}

fn have_prefix_match(a: &str, b: &str) -> bool {
    if eq_or_starts_with!(a, b) {
        // Exception: Case where one variant is a feminized version of the other
        if a.len() == b.len() + 1 && a.ends_with('a') {
            false
        } else if b.len() == a.len() + 1 && b.ends_with('a') {
            false
        } else {
            true
        }
    } else {
        false
    }
}

fn matches_without_diminutive(a: &str, b: &str) -> bool {
    if DIMINUTIVE_EXCEPTIONS.contains(a) {
        false
    } else if a.len() > 2 && b.len() >= a.len() - 1 && (a.ends_with('y') || a.ends_with('e')) &&
       eq_or_starts_with!(a[0..a.len() - 1], b) {
        true
    } else if a.len() > 4 && b.len() >= a.len() - 2 && (a.ends_with("ie") || a.ends_with("ey")) &&
       eq_or_starts_with!(a[0..a.len() - 2], b) {
        true
    } else if a.len() > 5 && b.len() >= a.len() - 3 && b.ends_with('a') &&
       (a.ends_with("ita") || a.ends_with("ina")) &&
       eq_or_starts_with!(a[0..a.len() - 3], b) {
        true
    } else if a.len() > 5 && b.len() >= a.len() - 3 && b.ends_with('o') && a.ends_with("ito") &&
       eq_or_starts_with!(a[0..a.len() - 3], b) {
        true
    } else {
        false
    }
}

fn is_final_syllables_of(needle: &str, haystack: &str) -> bool {
    if needle.len() == haystack.len() - 1 && !starts_with_consonant(haystack) &&
       eq_or_ends_with!(needle, haystack) {
        true
    } else if haystack.len() < 4 || needle.len() < 2 || needle.len() > haystack.len() - 2 {
        false
    } else if starts_with_consonant(needle) || needle.starts_with("Ann") || haystack.starts_with("Mary") {
        eq_or_ends_with!(needle, haystack) && !FINAL_SYLLABLES_EXCEPTIONS.contains(needle)
    } else {
        false
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nick_and_name() {
        assert!(have_matching_variants("Dave", "David"));
        assert!(have_matching_variants("David", "Dave"));
        assert!(have_matching_variants("Kenneth", "Kenny"));
        assert!(have_matching_variants("Kenny", "Kenneth"));
        assert!(have_matching_variants("Edward", "Eddie"));
        assert!(have_matching_variants("Eddie", "Edward"));
        assert!(have_matching_variants("Dot", "Dorothy"));
        assert!(have_matching_variants("Dorothy", "Dot"));
        assert!(have_matching_variants("Leroy", "Roy"));
        assert!(have_matching_variants("Roy", "Leroy"));
        assert!(have_matching_variants("Ann", "Agnes"));
        assert!(have_matching_variants("Annie", "Luann"));
        assert!(have_matching_variants("Marianne", "Mary"));
        assert!(have_matching_variants("Marianne", "Anne"));
    }

    #[test]
    fn matching_nicks() {
        assert!(have_matching_variants("Trisha", "Trix"));
        assert!(have_matching_variants("Trix", "Trisha"));
        assert!(have_matching_variants("Kenny", "Ken"));
        assert!(have_matching_variants("Ken", "Kenny"));
        assert!(have_matching_variants("Ned", "Eddie"));
        assert!(have_matching_variants("Eddie", "Ned"));
        assert!(have_matching_variants("Davy", "Dave"));
        assert!(have_matching_variants("Dave", "Davy"));
        assert!(have_matching_variants("Lon", "Al")); // Alonzo
        assert!(have_matching_variants("Al", "Lon")); // Alonzo
        assert!(have_matching_variants("Lousie", "Lulu"));
    }

    #[test]
    fn nonmatching_nicks() {
        assert!(!have_matching_variants("Xina", "Xander"));
        assert!(!have_matching_variants("Xander", "Xina"));
        assert!(!have_matching_variants("Andy", "Xander"));
        assert!(!have_matching_variants("Xander", "Andy"));
        assert!(!have_matching_variants("Molly", "Annie"));
        assert!(!have_matching_variants("Christopher", "Tina"));
        assert!(!have_matching_variants("Molly", "Mark"));
        assert!(!have_matching_variants("Patricia", "Rick"));
    }

    #[test]
    fn nonmatching_names() {
        assert!(!have_matching_variants("Antoinette", "Luanne"));
        assert!(!have_matching_variants("Luanne", "Antoinette"));
        assert!(!have_matching_variants("Jane", "John"));
        assert!(!have_matching_variants("John", "Jane"));
        assert!(!have_matching_variants("John", "Nathan"));
        assert!(!have_matching_variants("Mary", "Margeret"));
        assert!(!have_matching_variants("Annette", "Johanna"));
    }

    #[test]
    fn strip_nothing() {
        assert_eq!("Robert Roberts", strip_nickname("Robert Roberts"));
    }

    #[test]
    fn strip_parens() {
        assert_eq!("Robert Roberts", strip_nickname("Robert (Mr. Bob) Roberts"));
    }

    #[test]
    fn unmatched_parens() {
        assert_eq!("Robert", strip_nickname("Robert (Mr. Bob"));
    }

    #[test]
    fn strip_quotes() {
        assert_eq!("Robert Roberts", strip_nickname("Robert 'Mr. Bob' Roberts"));
    }

    #[test]
    fn unmatched_quote() {
        assert_eq!("Robert Mr. Bob' Roberts",
                   strip_nickname("Robert Mr. Bob' Roberts"));
    }

    #[test]
    fn unspaced_quotes() {
        assert_eq!("Ro'bert R'oberts", strip_nickname("Ro'bert R'oberts"));
    }
}

// There's no reason not to just use arrays for the values except that it won't compile :(
static NAMES_BY_NICK_PREFIX: phf::Map<&'static str, phf::Set<&'static str>> = phf_map! {
    "Ann" => phf_set! { "Agnes", "Antoinette", "Marianna", "Roseanne", "Anabelle", "Luann" },
    "Babb" => phf_set! { "Barbara" },
    "Bais" => phf_set! { "Elizabeth" },
    "Baiss" => phf_set! { "Elizabeth" },
    "Bald" => phf_set! { "Archibald" },
    "Barber" => phf_set! { "Barbara" },
    "Beck" => phf_set! { "Rebecca" },
    "Beed" => phf_set! { "Obedience" },
    "Bern" => phf_set! { "Barnabas" },
    "Bess" => phf_set! { "Elizabeth" },
    "Bets" => phf_set! { "Elizabeth" },
    "Bett" => phf_set! { "Elizabeth" },
    "Bill" => phf_set! { "William" },
    "Bird" => phf_set! { "Roberta" },
    "Bits" => phf_set! { "Elizabeth" },
    "Bonn" => phf_set! { "Bonita" },
    "Brad" => phf_set! { "Broderick" },
    "Bradl" => phf_set! { "Bradford" },
    "Cadd" => phf_set! { "Caroline" },
    "Camm" => phf_set! { "Camille" },
    "Cath" => phf_set! { "Katherine" },
    "Cecel" => phf_set! { "Cecilia" },
    "Creas" => phf_set! { "Lucretia" },
    "Criss" => phf_set! { "Christiana" },
    "Dac" => phf_set! { "Candace" },
    "Dais" => phf_set! { "Margaret" },
    "Darr" => phf_set! { "Darlene" },
    "Deann" => phf_set! { "Geraldine" },
    "Debb" => phf_set! { "Deborah" },
    "Dell" => phf_set! { "Deliverance" },
    "Dens" => phf_set! { "Prudence" },
    "Desr" => phf_set! { "Desiree" },
    "Dill" => phf_set! { "Deliverance" },
    "Doll" => phf_set! { "Dorothy" },
    "Donn" => phf_set! { "Donald" },
    "Dos" => phf_set! { "Eudoris" },
    "Doss" => phf_set! { "Eudoris" },
    "Dott" => phf_set! { "Dorothy" },
    "Edd" => phf_set! { "Edmund", "Edward", "Edgar" },
    "Edn" => phf_set! { "Edith" },
    "Eff" => phf_set! { "Euphemia" },
    "Emm" => phf_set! { "Emeline" },
    "Ern" => phf_set! { "Earnest" },
    "Fall" => phf_set! { "Eliphalet" },
    "Fan" => phf_set! { "Estefania" },
    "Fann" => phf_set! { "Frances" },
    "Ferb" => phf_set! { "Pharaba" },
    "Finn" => phf_set! { "Phineas" },
    "Floss" => phf_set! { "Florence" },
    "Gats" => phf_set! { "Augustus" },
    "Gatsb" => phf_set! { "Augustus" },
    "Gatt" => phf_set! { "Gertrude" },
    "Gen" => phf_set! { "Eugenia" },
    "Genc" => phf_set! { "Genevieve" },
    "Geoffr" => phf_set! { "Jefferson" },
    "Ginn" => phf_set! { "Virginia" },
    "Gus" => phf_set! { "Augusta" },
    "Hall" => phf_set! { "Mahalla" },
    "Happ" => phf_set! { "Karonhappuck" },
    "Hatt" => phf_set! { "Harriet" },
    "Heid" => phf_set! { "Adelaide" },
    "Helm" => phf_set! { "Wilhelmina" },
    "Hess" => phf_set! { "Hester" },
    "Hil" => phf_set! { "Hiram" },
    "Hitt" => phf_set! { "Mehitabel" },
    "Horr" => phf_set! { "Horace" },
    "Hum" => phf_set! { "Posthuma" },
    "Igg" => phf_set! { "Ignatius" },
    "Izz" => phf_set! { "Isidore", "Isabelle", "Isobel" },
    "Jak" => phf_set! { "Jacqueline" },
    "Jeffr" => phf_set! { "Jefferson" },
    "Jimm" => phf_set! { "James" },
    "Jin" => phf_set! { "Virginia" },
    "Jinc" => phf_set! { "Jane" },
    "Kar" => phf_set! { "Caroline" },
    "Kas" => phf_set! { "Casey" },
    "Kenj" => phf_set! { "Kendra" },
    "Ker" => phf_set! { "Caroline" },
    "Kerst" => phf_set! { "Christiana" },
    "Kezz" => phf_set! { "Keziah" },
    "Kimm" => phf_set! { "Kimberly" },
    "Kiss" => phf_set! { "Calista" },
    "Kits" => phf_set! { "Katherine" },
    "Kitt" => phf_set! { "Katherine" },
    "Krist" => phf_set! { "Christiana" },
    "Kymberl" => phf_set! { "Kimberly" },
    "Laff" => phf_set! { "Lafayette" },
    "Lain" => phf_set! { "Elaine" },
    "Lann" => phf_set! { "Roland" },
    "Larr" => phf_set! { "Lawrence" },
    "Laur" => phf_set! { "Lawrence" },
    "Leaf" => phf_set! { "Relief" },
    "Leff" => phf_set! { "Lafayette" },
    "Lenn" => phf_set! { "Leonard" },
    "Less" => phf_set! { "Celeste" },
    "Lev" => phf_set! { "Aleva" },
    "Liv" => phf_set! { "Olivia" },
    "Lizz" => phf_set! { "Elizabeth" },
    "Lod" => phf_set! { "Melody" },
    "Lonn" => phf_set! { "Lawrence" },
    "Lyd" => phf_set! { "Linda" },
    "Lydd" => phf_set! { "Linda" },
    "Madd" => phf_set! { "Madeline", "Madeleine" },
    "Mais" => phf_set! { "Margaret" },
    "Malach" => phf_set! { "Malcolm" },
    "Mam" => phf_set! { "Mary" },
    "Marger" => phf_set! { "Margaret" },
    "Marjor" => phf_set! { "Margaret" },
    "Maver" => phf_set! { "Mavine" },
    "Midd" => phf_set! { "Madeline" },
    "Morr" => phf_set! { "Seymour" },
    "Moss" => phf_set! { "Maurice" },
    "Nabb" => phf_set! { "Abigail" },
    "Napp" => phf_set! { "Napoleon" },
    "Nepp" => phf_set! { "Penelope" },
    "Ness" => phf_set! { "Agnes" },
    "Nibb" => phf_set! { "Isabella" },
    "Nic" => phf_set! { "Vernisee" },
    "Nikk" => phf_set! { "Nicolena" },
    "Noll" => phf_set! { "Olivia" },
    "Non" => phf_set! { "Joanna" },
    "Norr" => phf_set! { "Honora" },
    "Onn" => phf_set! { "Iona" },
    "Oph" => phf_set! { "Theophilus" },
    "Oss" => phf_set! { "Oswald" },
    "Ozz" => phf_set! { "Oswald" },
    "Padd" => phf_set! { "Patrick" },
    "Parsun" => phf_set! { "Parthenia" },
    "Pasoon" => phf_set! { "Parthenia" },
    "Pedd" => phf_set! { "Experience" },
    "Pegg" => phf_set! { "Margaret" },
    "Pen" => phf_set! { "Philipina" },
    "Penn" => phf_set! { "Penelope" },
    "Perr" => phf_set! { "Pelegrine" },
    "Phill" => phf_set! { "Adelphia" },
    "Phoen" => phf_set! { "Tryphena" },
    "Phos" => phf_set! { "Tryphosia" },
    "Pok" => phf_set! { "Pocahontas" },
    "Pon" => phf_set! { "Napoleon" },
    "Priss" => phf_set! { "Priscilla" },
    "Quill" => phf_set! { "Aquilla" },
    "Rodd" => phf_set! { "Rodney" },
    "Roll" => phf_set! { "Roland" },
    "Rox" => phf_set! { "Roseanne" },
    "Rub" => phf_set! { "Reuben" },
    "Rust" => phf_set! { "Russell" },
    "Sad" => phf_set! { "Sarah" },
    "Sall" => phf_set! { "Sarah" },
    "Samm" => phf_set! { "Samuel", "Samantha" },
    "Scott" => phf_set! { "Prescott" },
    "Sen" => phf_set! { "Eseneth" },
    "Sharr" => phf_set! { "Sharon" },
    "Sher" => phf_set! { "Sharon" },
    "Sl" => phf_set! { "Sylvester" },
    "Smitt" => phf_set! { "Smith" },
    "Soll" => phf_set! { "Solomon" },
    "Such" => phf_set! { "Susannah" },
    "Surr" => phf_set! { "Sarah" },
    "Suz" => phf_set! { "Susannah", "Susan" },
    "Sydn" => phf_set! { "Sidney" },
    "Tabb" => phf_set! { "Tabitha" },
    "Tall" => phf_set! { "Natalie" },
    "Tamm" => phf_set! { "Tamara" },
    "Tell" => phf_set! { "Aristotle" },
    "Tens" => phf_set! { "Hortense" },
    "Tent" => phf_set! { "Content" },
    "Tess" => phf_set! { "Theresa" },
    "Then" => phf_set! { "Parthenia" },
    "Tibb" => phf_set! { "Isabella" },
    "Tic" => phf_set! { "Theresa" },
    "Timm" => phf_set! { "Timothy" },
    "Tipp" => phf_set! { "Tipton" },
    "Tips" => phf_set! { "Tipton" },
    "Tomm" => phf_set! { "Thomas" },
    "Tor" => phf_set! { "Victoria" },
    "Torr" => phf_set! { "Victoria" },
    "Trac" => phf_set! { "Theresa" },
    "Trud" => phf_set! { "Gertrude" },
    "Valer" => phf_set! { "Valentina" },
    "Vall" => phf_set! { "Valentina" },
    "Vang" => phf_set! { "Evangeline" },
    "Vann" => phf_set! { "Vanessa" },
    "Verg" => phf_set! { "Virginia" },
    "Vess" => phf_set! { "Sylvester" },
    "Vic" => phf_set! { "Lewvisa" },
    "Vin" => phf_set! { "Lavinia" },
    "Vonn" => phf_set! { "Veronica" },
    "Wend" => phf_set! { "Gwendolyn" },
    "Zad" => phf_set! { "Isaiah" },
    "Zadd" => phf_set! { "Arzada" },
    "Zoll" => phf_set! { "Solomon" },
    "Abb" => phf_set! { "Abigail", "Abner", "Absalom", "Abiodun" },
    "Add" => phf_set! { "Adaline", "Adelaide", "Adelphia", "Agatha" },
    "Agg" => phf_set! { "Agatha", "Agnes", "Augusta" },
    "All" => phf_set! { "Aileen", "Alberta", "Alice", "Almena", "Alison" },
    "Arr" => phf_set! { "Arabella", "Armena" },
    "Benn" => phf_set! { "Benedict", "Benjamin", "Benedetta" },
    "Berr" => phf_set! { "Barry", "Greenberry", "Littleberry" },
    "Bert" => phf_set! { "Alberta", "Roberta" },
    "Bidd" => phf_set! { "Bridget", "Obedience" },
    "Bobb" => phf_set! { "Barbara", "Robert", "Roberta" },
    "Brid" => phf_set! { "Bertha" },
    "Call" => phf_set! { "Caldonia", "California", "Calpurnia", "Caroline" },
    "Carr" => phf_set! { "Caroline", "Karonhappuck" },
    "Cass" => phf_set! { "Alexandria", "Caroline", "Katherine" },
    "Cind" => phf_set! { "Cynthia", "Luciana", "Lucinda" },
    "Ciss" => phf_set! { "Cecilia", "Clarissa", "Frances", "Priscilla" },
    "Conn" => phf_set! { "Conrad", "Constance", "Cornelius", "Cornelia", "Constanza" },
    "Dann" => phf_set! { "Daniel", "Sheridan" },
    "Dic" => phf_set! { "Diana", "Edith", "Eurydice", "Laodicia" },
    "Dod" => phf_set! { "Delores", "Dorothy" },
    "Ebb" => phf_set! { "Abel", "Ebenezer" },
    "Ed" => phf_set! { "Adam" },
    "El" => phf_set! { "Alice" },
    "Ell" => phf_set! { "Alexandria", "Eleanor", "Elmira", "Elwood" },
    "Els" => phf_set! { "Alice", "Elizabeth" },
    "Emil" => phf_set! { "Amelia", "Emeline" },
    "Ess" => phf_set! { "Estella", "Hester" },
    "Ett" => phf_set! { "Carthaette", "Henrietta" },
    "Frank" => phf_set! { "Francis", "Veronica", "Francesca" },
    "Fredd" => phf_set! { "Alfred", "Alfreda", "Frederica", "Frederick", "Winifred" },
    "Fron" => phf_set! { "Sophronia", "Veronica" },
    "Gabb" => phf_set! { "Gabriel", "Gabrielle" },
    "Gerr" => phf_set! { "Gerald", "Geraldine", "Gerard", "Gerardo" },
    "Guss" => phf_set! { "Augusta", "Augustus" },
    "Harr" => phf_set! { "Harold", "Henry" },
    "Hett" => phf_set! { "Henrietta", "Hester", "Mehitabel" },
    "Iss" => phf_set! { "Isabella", "Isidora" },
    "Jack" => phf_set! { "Jacqueline", "Jaclyn", "Jacquelyn" },
    "Jazz" => phf_set! { "Jazmin", "Jasmine" },
    "Jenn" => phf_set! { "Eugenia", "Genevieve", "Jane", "Virginia" },
    "Jerr" => phf_set! { "Gerald", "Geraldine", "Jeremiah" },
    "Jins" => phf_set! { "Genevieve", "Jane" },
    "Jod" => phf_set! { "Joanna", "Joseph", "Josephine" },
    "Johnn" => phf_set! { "John", "Jonathan" },
    "Lett" => phf_set! { "Charlotte", "Letitia" },
    "Libb" => phf_set! { "Elizabeth", "Libuse" },
    "Lidd" => phf_set! { "Elizabeth", "Linda" },
    "Lind" => phf_set! { "Celinda", "Lyndon", "Melinda" },
    "Loll" => phf_set! { "Charlotte", "Delores", "Lillian" },
    "Lorr" => phf_set! { "Lauryn", "Lawrence", "Loretta" },
    "Lott" => phf_set! { "Carlotta", "Charlotte" },
    "Lynd" => phf_set! { "Linda" },
    "Magg" => phf_set! { "Madeline", "Margaret" },
    "Mand" => phf_set! { "Amanda", "Miranda" },
    "Mann" => phf_set! { "Emanuel", "Manuel" },
    "Mar" => phf_set! { "Maureen", "Miriam", "Mitzi", "Maura", "Moira" },
    "Matt" => phf_set! { "Martha", "Matilda" },
    "Mell" => phf_set! { "Amelia", "Melinda", "Permelia" },
    "Merc" => phf_set! { "Mary" },
    "Mick" => phf_set! { "Michael", "Michelle" },
    "Mill" => phf_set! { "Amelia", "Armilda", "Camille", "Emeline", "Melissa", "Mildred", "Permelia" },
    "Mim" => phf_set! { "Jemima", "Mary", "Mildred", "Miriam" },
    "Mind" => phf_set! { "Arminda", "Melinda" },
    "Minn" => phf_set! { "Almina", "Mary", "Minerva", "Wilhelmina" },
    "Miss" => phf_set! { "Melissa", "Millicent" },
    "Mitt" => phf_set! { "Mehitabel", "Submit" },
    "Mitz" => phf_set! { "Mary", "Miriam" },
    "Moll" => phf_set! { "Amalia", "Amelia", "Martha", "Mary" },
    "Mont" => phf_set! { "Lamont" },
    "Mor" => phf_set! { "Maurice", "Seymour" },
    "Nanc" => phf_set! { "Agnes", "Anna" },
    "Nann" => phf_set! { "Anna", "Hannah", "Nancy" },
    "Natt" => phf_set! { "Asenath", "Natalie", "Nathaniel" },
    "Neel" => phf_set! { "Cornelia", "Cornelius" },
    "Nell" => phf_set! { "Cornelia", "Eleanor", "Helena" },
    "Nerv" => phf_set! { "Manerva", "Minerva" },
    "Nett" => phf_set! { "Antoinette", "Henrietta", "Jane", "Juanita", "Natalie", "Ninell", "Pernetta" },
    "Nick" => phf_set! { "Nicholas", "Nicolena" },
    "Oll" => phf_set! { "Oliver", "Olivia" },
    "Pats" => phf_set! { "Martha", "Patricia", "Patrick" },
    "Patt" => phf_set! { "Martha", "Matilda", "Parthenia", "Patience", "Patricia" },
    "Phen" => phf_set! { "Josephine", "Parthenia", "Tryphena" },
    "Poll" => phf_set! { "Paulina" },
    "Rand" => phf_set! { "Miranda" },
    "Reen" => phf_set! { "Irene", "Maureen", "Sabrina" },
    "Regg" => phf_set! { "Regina", "Reginald" },
    "Renn" => phf_set! { "Irene", "Reginald" },
    "Rich" => phf_set! { "Alderick", "Derrick" },
    "Rick" => phf_set! { "Broderick", "Cedrick", "Eric", "Richard" },
    "Rill" => phf_set! { "Aurelia", "Aurilla" },
    "Robb" => phf_set! { "Robert", "Roberta" },
    "Ronn" => phf_set! { "Aaron", "Cameron", "Ronald", "Veronica" },
    "Ros" => phf_set! { "Euphrosina" },
    "Sand" => phf_set! { "Alexander", "Alexandria" },
    "Shell" => phf_set! { "Michelle", "Rachel", "Sheldon" },
    "Sherr" => phf_set! { "Charlotte", "Shirley" },
    "Sonn" => phf_set! { "Anderson", "Jefferson", "Judson" },
    "Stac" => phf_set! { "Anastasia", "Eustacia" },
    "Suk" => phf_set! { "Sarah", "Susannah" },
    "Tedd" => phf_set! { "Edward", "Theodore" },
    "Terr" => phf_set! { "Theresa" },
    "Till" => phf_set! { "Matilda", "Temperance", "Tilford" },
    "Ton" => phf_set! { "Anthony", "Antoinette", "Clifton", "Antonio", "Antoni" },
    "Triss" => phf_set! { "Beatrice", "Theresa" },
    "Trix" => phf_set! { "Beatrice", "Patricia" },
    "Vick" => phf_set! { "Veronica", "Victoria" },
    "Vinn" => phf_set! { "Calvin", "Lavinia", "Vincent" },
    "Will" => phf_set! { "Wilda", "Wilfred", "Wilhelmina" },
    "Winn" => phf_set! { "Edwina", "Winfield", "Winifred" },
    "Wood" => phf_set! { "Elwood" },
};

// There's no reason not to just use arrays for the values except that it won't compile :(
static NAMES_BY_IRREGULAR_NICK: phf::Map<&'static str, phf::Set<&'static str>> = phf_map! {
    "Abdo" => phf_set! { "Abdu", "Abdul", "Abdullah" },
    "Abertina" => phf_set! { "Alberta" },
    "Abiah" => phf_set! { "Abijah" },
    "Abram" => phf_set! { "Abraham" },
    "Acuilla" => phf_set! { "Aquilla" },
    "Adaline" => phf_set! { "Adelaide" },
    "Adela" => phf_set! { "Adaline" },
    "Adelia" => phf_set! { "Adelaide" },
    "Adeline" => phf_set! { "Adelaide" },
    "Adeliza" => phf_set! { "Adelaide" },
    "Adi" => phf_set! { "Hadi" },
    "Adia" => phf_set! { "Nadia" },
    "Ado" => phf_set! { "Rudolphus" },
    "Adolf" => phf_set! { "Rudolphus" },
    "Adolphus" => phf_set! { "Rudolphus" },
    "Adoph" => phf_set! { "Rudolphus" },
    "Agnes" => phf_set! { "Agatha", "Agnieszka" },
    "Ain" => phf_set! { "Nuru", "Lain" },
    "Aini" => phf_set! { "Nuraini" },
    "Aji" => phf_set! { "Naji" },
    "Akin" => phf_set! { "Akın" },
    "Ala" => phf_set! { "Ayala" },
    "Albert" => phf_set! { "Adelbert" },
    "Albertine" => phf_set! { "Alberta" },
    "Aldi" => phf_set! { "Renaldi", "Reynaldi", "Naldi" },
    "Alec" => phf_set! { "Alexander" },
    "Alex" => phf_set! { "Alejandro" },
    "Alexandra" => phf_set! { "Alexandria" },
    "Alexei" => phf_set! { "Alexander" },
    "Alice" => phf_set! { "Alisha", "Alison" },
    "Alim" => phf_set! { "Salim" },
    "Alina" => phf_set! { "Alyna" },
    "Aline" => phf_set! { "Adaline", "Alline" },
    "Alla" => phf_set! { "Alexandria" },
    "Alle" => phf_set! { "Alessandra" },
    "Alonzo" => phf_set! { "Alphonzo" },
    "Alphus" => phf_set! { "Alphinias" },
    "Amabel" => phf_set! { "Mehitabel" },
    "Amar" => phf_set! { "Ammar" },
    "Amin" => phf_set! { "Ameen", "Alamin" },
    "Amir" => phf_set! { "Samir" },
    "Amos" => phf_set! { "Moses" },
    "Ance" => phf_set! { "Anderson", "Anselm" },
    "Andre" => phf_set! { "Anderson" },
    "Andreas" => phf_set! { "Andrew" },
    "Andrei" => phf_set! { "Andrew" },
    "Andria" => phf_set! { "Andrea" },
    "Angela" => phf_set! { "Angelica", "Angeline" },
    "Ania" => phf_set! { "Rahmania" },
    "Anil" => phf_set! { "Anıl" },
    "Anja" => phf_set! { "Sanjay" },
    "Anju" => phf_set! { "Anjali", "Anjana" },
    "Ann" => phf_set! { "Agnes", "Antoinette", "Marianna", "Nancy", "Roseanne", "Anabelle" },
    "Anna" => phf_set! { "Ania", "Annette" },
    "Anne" => phf_set! { "Luann", "Marianna" },
    "Antoine" => phf_set! { "Anthony" },
    "Antonia" => phf_set! { "Antoinette" },
    "Antonio" => phf_set! { "Anthony" },
    "Antos" => phf_set! { "Antonella" },
    "Aphinius" => phf_set! { "Alphinias" },
    "Aphrodite" => phf_set! { "Epaphroditius", "Epaphroditus" },
    "Aran" => phf_set! { "Karan" },
    "Archelous" => phf_set! { "Archibald" },
    "Ardi" => phf_set! { "Nardi" },
    "Arek" => phf_set! { "Arkadiusz" },
    "Aris" => phf_set! { "Ariez" },
    "Armanda" => phf_set! { "Amanda" },
    "Arno" => phf_set! { "Arnaud" },
    "Arslan" => phf_set! { "Arsalan" },
    "Arya" => phf_set! { "Acharya" },
    "Asad" => phf_set! { "Assad" },
    "Asahel" => phf_set! { "Asaph" },
    "Ashe" => phf_set! { "Tinashe" },
    "Asli" => phf_set! { "Aslı", "Aslıhan" },
    "Assene" => phf_set! { "Asenath" },
    "Astri" => phf_set! { "Lastri" },
    "Augustine" => phf_set! { "Augustus" },
    "Aura" => phf_set! { "Aurelia" },
    "Aurilla" => phf_set! { "Aurelia" },
    "Austin" => phf_set! { "Augustine" },
    "Axl" => phf_set! { "Axel" },
    "Aydin" => phf_set! { "aydın" },
    "Ayu" => phf_set! { "Aiu" },
    "Azarich" => phf_set! { "Azariah" },
    "Aziz" => phf_set! { "Abdelaziz" },
    "Azza" => phf_set! { "Munazza" },
    "Bab" => phf_set! { "Barbara" },
    "Babs" => phf_set! { "Barbara" },
    "Baig" => phf_set! { "Mirza" },
    "Baldo" => phf_set! { "Archibald" },
    "Banks" => phf_set! { "Bankole" },
    "Barnard" => phf_set! { "Barnabas" },
    "Bartek" => phf_set! { "Bartosz" },
    "Bartel" => phf_set! { "Bartholomew" },
    "Bartlomiej" => phf_set! { "Bartłomiej" },
    "Basia" => phf_set! { "Barbara" },
    "Basil" => phf_set! { "Bazaleel" },
    "Bat" => phf_set! { "Bartholomew" },
    "Bea" => phf_set! { "Blanche" },
    "Bear" => phf_set! { "Barry" },
    "Beck" => phf_set! { "Rebecca" },
    "Bede" => phf_set! { "Obedience" },
    "Bela" => phf_set! { "William" },
    "Bell" => phf_set! { "Arabella", "Belinda" },
    "Bella" => phf_set! { "Mehitabel" },
    "Belle" => phf_set! { "Arabella", "Belinda", "Isabella", "Rosabella" },
    "Bennett" => phf_set! { "Benedict" },
    "Bernard" => phf_set! { "Barnabas" },
    "Bert" => phf_set! { "Alberta", "Elbertson", "Roberta" },
    "Bess" => phf_set! { "Elizabeth" },
    "Bethia" => phf_set! { "Elizabeth" },
    "Beto" => phf_set! { "Alberto" },
    "Bex" => phf_set! { "Rebecca" },
    "Bia" => phf_set! { "Beatriz" },
    "Biah" => phf_set! { "Abijah" },
    "Bibi" => phf_set! { "Bianca" },
    "Bige" => phf_set! { "Abijah" },
    "Bill" => phf_set! { "William" },
    "Bird" => phf_set! { "Albert" },
    "Bjorn" => phf_set! { "Bjørn" },
    "Bo" => phf_set! { "Beaufort", "Beauregard" },
    "Bob" => phf_set! { "Robert" },
    "Bree" => phf_set! { "Aubrey" },
    "Brian" => phf_set! { "Bryant" },
    "Bridgit" => phf_set! { "Bedelia" },
    "Brito" => phf_set! { "Britto" },
    "Bruna" => phf_set! { "Brunna" },
    "Bryan" => phf_set! { "Brian", "Brayan" },
    "Bub" => phf_set! { "Mahbubur" },
    "Buck" => phf_set! { "Charles" },
    "Burt" => phf_set! { "Bert", "Egbert" },
    "Cager" => phf_set! { "Micajah" },
    "Cano" => phf_set! { "Kano" },
    "Car" => phf_set! { "Charlotte" },
    "Carl" => phf_set! { "Charles" },
    "Carla" => phf_set! { "Carli" },
    "Carlitos" => phf_set! { "Carlos" },
    "Carlotta" => phf_set! { "Charlotte" },
    "Carmen" => phf_set! { "Karmen" },
    "Carolyn" => phf_set! { "Caroline" },
    "Casper" => phf_set! { "Jasper" },
    "Cass" => phf_set! { "Caswell" },
    "Castle" => phf_set! { "Castillo" },
    "Catherine" => phf_set! { "Katherine" },
    "Cathleen" => phf_set! { "Katherine" },
    "Caz" => phf_set! { "Caroline" },
    "Ceall" => phf_set! { "Lucille" },
    "Cecilia" => phf_set! { "Sheila" },
    "Celia" => phf_set! { "Cecilia", "Celeste" },
    "Celina" => phf_set! { "Selina" },
    "Cene" => phf_set! { "Cyrenius" },
    "Cenia" => phf_set! { "Laodicia" },
    "Chad" => phf_set! { "Charles" },
    "Chan" => phf_set! { "Chauncy" },
    "Chat" => phf_set! { "Charity" },
    "Chaudhary" => phf_set! { "Choudhary" },
    "Chaudhry" => phf_set! { "Chaudhary" },
    "Chet" => phf_set! { "Chesley" },
    "Chick" => phf_set! { "Charles" },
    "Chip" => phf_set! { "Charles" },
    "Christine" => phf_set! { "Christiana" },
    "Christopher" => phf_set! { "Christian" },
    "Chuck" => phf_set! { "Charles" },
    "Cibyl" => phf_set! { "Sibbilla" },
    "Cil" => phf_set! { "Priscilla" },
    "Cilla" => phf_set! { "Cecilia" },
    "Ciller" => phf_set! { "Priscilla" },
    "Cinthia" => phf_set! { "Cynthia" },
    "Claas" => phf_set! { "Nicholas" },
    "Claes" => phf_set! { "Nicholas" },
    "Clair" => phf_set! { "Clarence", "Clarissa" },
    "Claire" => phf_set! { "Clarissa" },
    "Clara" => phf_set! { "Clarissa" },
    "Clarice" => phf_set! { "Clarissa" },
    "Claus" => phf_set! { "Claudia" },
    "Cliff" => phf_set! { "Clifton" },
    "Clo" => phf_set! { "Chloe" },
    "Clum" => phf_set! { "Columbus" },
    "Cono" => phf_set! { "Cornelius" },
    "Cora" => phf_set! { "Corinne" },
    "Crece" => phf_set! { "Lucretia" },
    "Crese" => phf_set! { "Lucretia" },
    "Cris" => phf_set! { "Christiana" },
    "Cristian" => phf_set! { "Christian", "Cristhian" },
    "Cristina" => phf_set! { "Christiana" },
    "Curg" => phf_set! { "Lecurgus" },
    "Curt" => phf_set! { "Courtney" },
    "Dahl" => phf_set! { "Dalton" },
    "Damaris" => phf_set! { "Demerias" },
    "Danelle" => phf_set! { "Danielle" },
    "Danial" => phf_set! { "Daniel" },
    "Danni" => phf_set! { "Danielle" },
    "Darek" => phf_set! { "Dariusz" },
    "Daria" => phf_set! { "Dasha" },
    "Daz" => phf_set! { "Darren" },
    "Debbe" => phf_set! { "Deborah" },
    "Debi" => phf_set! { "Deborah" },
    "Debra" => phf_set! { "Deborah", "Debbie" },
    "Dee" => phf_set! { "Audrey", "Dorothy" },
    "Deedee" => phf_set! { "Deidre", "Nadine" },
    "Deea" => phf_set! { "Andreea" },
    "Deia" => phf_set! { "Andreia" },
    "Del" => phf_set! { "Adelbert" },
    "Delia" => phf_set! { "Adaline" },
    "Dell" => phf_set! { "Adaline", "Adelaide", "Adelphia", "Delilah", "Delores", "Rhodella" },
    "Della" => phf_set! { "Adelaide", "Delilah", "Deliverance", "Delores" },
    "Delpha" => phf_set! { "Philadelphia" },
    "Delphina" => phf_set! { "Adelphia" },
    "Demaris" => phf_set! { "Demerias" },
    "Denys" => phf_set! { "Denise" },
    "Denyse" => phf_set! { "Denise" },
    "Desree" => phf_set! { "Desiree" },
    "Dessa" => phf_set! { "Andressa" },
    "Dewayne" => phf_set! { "Duane" },
    "Deza" => phf_set! { "Andreza" },
    "Dick" => phf_set! { "Melchizedek", "Richard", "Zadock" },
    "Dickon" => phf_set! { "Richard" },
    "Dilbert" => phf_set! { "Delbert" },
    "Dimmis" => phf_set! { "Demerias" },
    "Dina" => phf_set! { "Geraldine" },
    "Dipak" => phf_set! { "Deepak" },
    "Dirch" => phf_set! { "Derrick" },
    "Ditus" => phf_set! { "Aphrodite" },
    "Diya" => phf_set! { "Divya" },
    "Dob" => phf_set! { "Robert" },
    "Dobbin" => phf_set! { "Robert" },
    "Doda" => phf_set! { "Dorothy" },
    "Dode" => phf_set! { "Dorothy" },
    "Dolf" => phf_set! { "Randolph", "Rudolphus" },
    "Dolph" => phf_set! { "Rudolphus" },
    "Dona" => phf_set! { "Caldonia" },
    "Donna" => phf_set! { "Fredonia" },
    "Dora" => phf_set! { "Dorothy", "Theodosia" },
    "Dori" => phf_set! { "Dora" },
    "Dorinda" => phf_set! { "Dorothy" },
    "Doris" => phf_set! { "Dorothy" },
    "Dortha" => phf_set! { "Dorothy" },
    "Dos" => phf_set! { "Reis" },
    "Dot" => phf_set! { "Dorothy" },
    "Dotha" => phf_set! { "Dorothy" },
    "Drew" => phf_set! { "Woodrow" },
    "Dru" => phf_set! { "Andrew" },
    "Duda" => phf_set! { "Eduarda" },
    "Dunk" => phf_set! { "Duncan" },
    "Dwane" => phf_set! { "Duane" },
    "Dwayne" => phf_set! { "Duane" },
    "Dyce" => phf_set! { "Aphrodite" },
    "Dyche" => phf_set! { "Aphrodite" },
    "Dyer" => phf_set! { "Jedediah", "Obadiah", "Zedediah" },
    "Eb" => phf_set! { "Abel" },
    "Eddy" => phf_set! { "Reddy" },
    "Edgar" => phf_set! { "Edward" },
    "Edith" => phf_set! { "Adaline" },
    "Edmund" => phf_set! { "Edward" },
    "Edna" => phf_set! { "Edith" },
    "Eid" => phf_set! { "Reid" },
    "Eileen" => phf_set! { "Aileen", "Helena" },
    "Eko" => phf_set! { "Echo" },
    "Elaine" => phf_set! { "Eleanor", "Helena", "Lainey", "Alaina" },
    "Elbert" => phf_set! { "Adelbert", "Albert", "Alberta" },
    "Eleanor" => phf_set! { "Helena" },
    "Eleck" => phf_set! { "Alexander" },
    "Electa" => phf_set! { "Electra" },
    "Elena" => phf_set! { "Helena", "Mariaelena" },
    "Elenor" => phf_set! { "Leonora" },
    "Elenora" => phf_set! { "Eleanor" },
    "Elic" => phf_set! { "Alexandria" },
    "Elicia" => phf_set! { "Alice" },
    "Elinamifia" => phf_set! { "Eleanor" },
    "Elis" => phf_set! { "Elizabeth" },
    "Elisabeth" => phf_set! { "Elizabeth" },
    "Elisha" => phf_set! { "Alice" },
    "Elissa" => phf_set! { "Elizabeth" },
    "Ella" => phf_set! { "Eleanor", "Gabrielle", "Helena", "Luella" },
    "Ellen" => phf_set! { "Eleanor", "Helena" },
    "Ellender" => phf_set! { "Helena" },
    "Ellis" => phf_set! { "Alice" },
    "Ells" => phf_set! { "Elwood" },
    "Elnora" => phf_set! { "Eleanor" },
    "Ema" => phf_set! { "Emma" },
    "Emiline" => phf_set! { "Emeline" },
    "Emm" => phf_set! { "Emeline" },
    "Emma" => phf_set! { "Emeline" },
    "Emmer" => phf_set! { "Emeline" },
    "Ender" => phf_set! { "Mahender" },
    "Endra" => phf_set! { "Harendra", "Birendra" },
    "Eppa" => phf_set! { "Aphrodite" },
    "Erik" => phf_set! { "Erick", "Eric" },
    "Erin" => phf_set! { "Aaron" },
    "Erma" => phf_set! { "Emeline" },
    "Erna" => phf_set! { "Ernestine" },
    "Ernest" => phf_set! { "Earnest" },
    "Erwin" => phf_set! { "Irwin" },
    "Esa" => phf_set! { "Mahesa" },
    "Essa" => phf_set! { "Vanessa" },
    "Ester" => phf_set! { "Esther" },
    "Esther" => phf_set! { "Hester" },
    "Etta" => phf_set! { "Carthaette", "Henrietta", "Loretta" },
    "Eve" => phf_set! { "Genevieve" },
    "Evelina" => phf_set! { "Evelyn" },
    "Eves" => phf_set! { "Neves" },
    "Fadi" => phf_set! { "Fahad" },
    "Faisal" => phf_set! { "Faysal" },
    "Fan" => phf_set! { "Frances" },
    "Farah" => phf_set! { "Farrah" },
    "Fate" => phf_set! { "Lafayette" },
    "Felicia" => phf_set! { "Felicity" },
    "Fena" => phf_set! { "Euphrosina" },
    "Fenee" => phf_set! { "Euphrosina" },
    "Ferns" => phf_set! { "Fernandes" },
    "Fidelia" => phf_set! { "Bedelia" },
    "Fifi" => phf_set! { "Fiona" },
    "Fina" => phf_set! { "Josephine" },
    "Finnius" => phf_set! { "Alphinias" },
    "Flick" => phf_set! { "Felicity" },
    "Flora" => phf_set! { "Florence" },
    "Floss" => phf_set! { "Florence" },
    "Franco" => phf_set! { "Franko" },
    "Frank" => phf_set! { "Francis" },
    "Frankisek" => phf_set! { "Francis" },
    "Franklin" => phf_set! { "Francis" },
    "Franz" => phf_set! { "Francis", "Francesco" },
    "Freda" => phf_set! { "Frederica" },
    "Frederik" => phf_set! { "Frederick" },
    "Fredric" => phf_set! { "Frederick" },
    "Fredricka" => phf_set! { "Frederica" },
    "Fredrik" => phf_set! { "Frederick" },
    "Frieda" => phf_set! { "Alfreda", "Frederica" },
    "Frish" => phf_set! { "Frederick" },
    "Frits" => phf_set! { "Frederick" },
    "Fritz" => phf_set! { "Frederick" },
    "Frona" => phf_set! { "Sophronia" },
    "Fronia" => phf_set! { "Sophronia" },
    "Gani" => phf_set! { "Ganesh" },
    "Gay" => phf_set! { "Gerhardt" },
    "Gee" => phf_set! { "Jehu" },
    "Gema" => phf_set! { "Gemma" },
    "Gen" => phf_set! { "Virginia" },
    "Gene" => phf_set! { "Eugenia" },
    "Geoff" => phf_set! { "Jefferson" },
    "Georgios" => phf_set! { "George" },
    "Geri" => phf_set! { "Geraldine" },
    "Ghia" => phf_set! { "Nghia" },
    "Giang" => phf_set! { "Huong" },
    "Gib" => phf_set! { "Gilbert" },
    "Gigi" => phf_set! { "Gisele" },
    "Gina" => phf_set! { "Virginia" },
    "Ginger" => phf_set! { "Virginia" },
    "Goes" => phf_set! { "Bagus" },
    "Gosia" => phf_set! { "Malgorzata" },
    "Gram" => phf_set! { "Graham" },
    "Gregg" => phf_set! { "Gregory" },
    "Greta" => phf_set! { "Margaret" },
    "Gretta" => phf_set! { "Margaret" },
    "Grissel" => phf_set! { "Griselda" },
    "Gum" => phf_set! { "Montgomery" },
    "Gunter" => phf_set! { "Guenter" },
    "Gunther" => phf_set! { "Guenther" },
    "Gus" => phf_set! { "Augusta", "Augustus" },
    "Habib" => phf_set! { "Habeeb" },
    "Hadad" => phf_set! { "Haddad" },
    "Hakim" => phf_set! { "Hakeem" },
    "Hal" => phf_set! { "Harold", "Henry", "Howard" },
    "Hamad" => phf_set! { "Hammad" },
    "Hamp" => phf_set! { "Hamilton" },
    "Hanh" => phf_set! { "Khanh" },
    "Hank" => phf_set! { "Harold", "Henrietta", "Henry" },
    "Hans" => phf_set! { "John" },
    "Harman" => phf_set! { "Herman" },
    "Hebsabeth" => phf_set! { "Hepsabah" },
    "Heide" => phf_set! { "Adelaide" },
    "Helen" => phf_set! { "Aileen", "Elaine", "Eleanor" },
    "Hema" => phf_set! { "Latha", "Atha" },
    "Hence" => phf_set! { "Henry" },
    "Henk" => phf_set! { "Hendrick" },
    "Hephsibah" => phf_set! { "Hepsabah" },
    "Hepsabel" => phf_set! { "Hepsabah" },
    "Hepsibah" => phf_set! { "Hepsabah" },
    "Heri" => phf_set! { "Herry" },
    "Hermoine" => phf_set! { "Hermione" },
    "Hoa" => phf_set! { "Khoa" },
    "Hopp" => phf_set! { "Hopkins" },
    "Horatio" => phf_set! { "Horace" },
    "Hugh" => phf_set! { "Hubert", "Jehu" },
    "Hugo" => phf_set! { "Hubert" },
    "Huma" => phf_set! { "Kabir" },
    "Hung" => phf_set! { "Nhung" },
    "Hussein" => phf_set! { "Hussien" },
    "Huy" => phf_set! { "Thuy" },
    "Hy" => phf_set! { "Hezekiah", "Hiram" },
    "Iam" => phf_set! { "Ilham" },
    "Ib" => phf_set! { "Isabella" },
    "Ike" => phf_set! { "Isaac" },
    "Ilah" => phf_set! { "Fazilah" },
    "Illa" => phf_set! { "Faradilla", "Dilla" },
    "Ima" => phf_set! { "Chandima" },
    "Iman" => phf_set! { "Budiman" },
    "Immanuel" => phf_set! { "Emanuel" },
    "Ina" => phf_set! { "Lavinia" },
    "Inda" => phf_set! { "Arabinda" },
    "Inez" => phf_set! { "Agnes" },
    "Ing" => phf_set! { "Ning" },
    "Ingrum" => phf_set! { "Ningrum" },
    "Ink" => phf_set! { "Link" },
    "Inta" => phf_set! { "Sinta" },
    "Ioannis" => phf_set! { "Yanis" },
    "Iott" => phf_set! { "Elliott" },
    "Iran" => phf_set! { "Kiran" },
    "Irani" => phf_set! { "Khairani" },
    "Isa" => phf_set! { "Nisa" },
    "Isham" => phf_set! { "Hisham" },
    "Ivan" => phf_set! { "John" },
    "Ivi" => phf_set! { "Ivana" },
    "Jaap" => phf_set! { "Jacob" },
    "Jack" => phf_set! { "John" },
    "Jacklin" => phf_set! { "Jacqueline" },
    "Jacklyn" => phf_set! { "Jacqueline" },
    "Jaclin" => phf_set! { "Jacqueline" },
    "Jaclyn" => phf_set! { "Jacqueline" },
    "Jake" => phf_set! { "Jacob" },
    "Jamil" => phf_set! { "Jameel" },
    "Jan" => phf_set! { "John" },
    "Jaques" => phf_set! { "John" },
    "Jaroslaw" => phf_set! { "Jarosław" },
    "Jayce" => phf_set! { "Jane" },
    "Jayhugh" => phf_set! { "Jehu" },
    "Jazz" => phf_set! { "Jazmin", "Jasmine" },
    "Jean" => phf_set! { "Genevieve", "Jane", "Joanna", "John" },
    "Jeanne" => phf_set! { "Jane" },
    "Jedidiah" => phf_set! { "Jedediah" },
    "Jem" => phf_set! { "James" },
    "Jemma" => phf_set! { "Jemima" },
    "Jill" => phf_set! { "Julia" },
    "Jim" => phf_set! { "James" },
    "Jitu" => phf_set! { "Jitendra" },
    "Jme" => phf_set! { "Jamie" },
    "Jock" => phf_set! { "John" },
    "Joey" => phf_set! { "Joseph", "Josephine" },
    "Johanna" => phf_set! { "Joanna" },
    "Johannah" => phf_set! { "Joanna" },
    "John" => phf_set! { "Jonathan" },
    "Jorg" => phf_set! { "Joerg" },
    "Jorge" => phf_set! { "George" },
    "Jorgen" => phf_set! { "Jørgen" },
    "Josefa" => phf_set! { "Joseph" },
    "Josepha" => phf_set! { "Josephine" },
    "Josephine" => phf_set! { "Pheney" },
    "Joss" => phf_set! { "Jocelyn" },
    "Jr" => phf_set! { "Junior" },
    "Julian" => phf_set! { "Julias" },
    "Juliet" => phf_set! { "Julia" },
    "Julius" => phf_set! { "Julias" },
    "Jurgen" => phf_set! { "Juergen" },
    "Justus" => phf_set! { "Justin" },
    "Kc" => phf_set! { "Casey" },
    "Kami" => phf_set! { "Kamran" },
    "Karel" => phf_set! { "Charles" },
    "Karen" => phf_set! { "Karonhappuck" },
    "Karim" => phf_set! { "Kareem" },
    "Karl" => phf_set! { "Charles" },
    "Kasia" => phf_set! { "Katarzyna" },
    "Kata" => phf_set! { "Catalina" },
    "Katarina" => phf_set! { "Katherine" },
    "Kathi" => phf_set! { "Katharina" },
    "Kathleen" => phf_set! { "Katherine" },
    "Kathryn" => phf_set! { "Katherine" },
    "Kati" => phf_set! { "Katalin" },
    "Kaur" => phf_set! { "Sidhu" },
    "Kendall" => phf_set! { "Kenneth" },
    "Kendrick" => phf_set! { "Kenneth" },
    "Kenj" => phf_set! { "Kendra" },
    "Kenny" => phf_set! { "Kehinde" },
    "Kent" => phf_set! { "Kenneth" },
    "Kester" => phf_set! { "Christopher" },
    "Kez" => phf_set! { "Kerry" },
    "Khushi" => phf_set! { "Khushboo" },
    "Kid" => phf_set! { "Keziah" },
    "Kit" => phf_set! { "Christian", "Christopher", "Katherine" },
    "Kizza" => phf_set! { "Keziah" },
    "Knowell" => phf_set! { "Noel" },
    "Kostas" => phf_set! { "Konstantinos" },
    "Kris" => phf_set! { "Christiana" },
    "Kristine" => phf_set! { "Christiana" },
    "Kuba" => phf_set! { "Jakub" },
    "Kurt" => phf_set! { "Curtis" },
    "Kurtis" => phf_set! { "Curtis" },
    "Ky" => phf_set! { "Hezekiah" },
    "Kym" => phf_set! { "Kimberly" },
    "Lr" => phf_set! { "Leroy" },
    "Laci" => phf_set! { "Laszlo" },
    "Lalo" => phf_set! { "Eduardo" },
    "Lanna" => phf_set! { "Eleanor" },
    "Lark" => phf_set! { "Clark" },
    "Larry" => phf_set! { "Olanrewaju" },
    "Lars" => phf_set! { "Lawrence" },
    "Latha" => phf_set! { "Hemal" },
    "Laura" => phf_set! { "Laurinda", "Loretta", "Lauri" },
    "Laurence" => phf_set! { "Lawrence" },
    "Lazar" => phf_set! { "Eleazer" },
    "Lb" => phf_set! { "Littleberry" },
    "Leafa" => phf_set! { "Relief" },
    "Lecta" => phf_set! { "Electra" },
    "Lee" => phf_set! { "Elias", "Shirley" },
    "Leet" => phf_set! { "Philetus" },
    "Left" => phf_set! { "Eliphalet", "Lafayette" },
    "Leja" => phf_set! { "Alejandra" },
    "Len" => phf_set! { "Leonard" },
    "Lena" => phf_set! { "Adaline", "Aileen", "Angela", "Arlene", "Caroline", "Darlene", "Evaline", "Madeline", "Magdelina", "Selina" },
    "Lenhart" => phf_set! { "Leonard" },
    "Leo" => phf_set! { "Leandro" },
    "Leon" => phf_set! { "Lionel" },
    "Leonora" => phf_set! { "Eleanor" },
    "Lester" => phf_set! { "Leslie" },
    "Lettice" => phf_set! { "Letitia" },
    "Leve" => phf_set! { "Aleva" },
    "Lewis" => phf_set! { "Louis" },
    "Lexa" => phf_set! { "Alexandria" },
    "Lexi" => phf_set! { "Alexis" },
    "Li" => phf_set! { "Lee" },
    "Lib" => phf_set! { "Elizabeth" },
    "Liba" => phf_set! { "Libuse" },
    "Lidia" => phf_set! { "Linda" },
    "Lig" => phf_set! { "Elijah" },
    "Lige" => phf_set! { "Elijah" },
    "Lil" => phf_set! { "Delilah" },
    "Lila" => phf_set! { "Delilah" },
    "Lillah" => phf_set! { "Lillian" },
    "Lina" => phf_set! { "Emeline" },
    "Lineau" => phf_set! { "Leonard" },
    "Linette" => phf_set! { "Linda" },
    "Link" => phf_set! { "Lincoln" },
    "Linz" => phf_set! { "Lindsey" },
    "Lisa" => phf_set! { "Elizabeth", "Melissa" },
    "Lise" => phf_set! { "Elizabeth" },
    "Lish" => phf_set! { "Alice" },
    "Lissa" => phf_set! { "Larissa" },
    "Liz" => phf_set! { "Elizabeth" },
    "Liza" => phf_set! { "Adelaide", "Elizabeth" },
    "Lloyd" => phf_set! { "Floyd" },
    "Loenore" => phf_set! { "Leonora" },
    "Lois" => phf_set! { "Heloise", "Louise" },
    "Lola" => phf_set! { "Delores" },
    "Loli" => phf_set! { "Dolores" },
    "Lon" => phf_set! { "Alonzo", "Lawrence" },
    "Lonson" => phf_set! { "Alanson" },
    "Lorinda" => phf_set! { "Laurinda" },
    "Lorne" => phf_set! { "Lawrence" },
    "Los" => phf_set! { "Angeles" },
    "Lotta" => phf_set! { "Charlotte" },
    "Lou" => phf_set! { "Luann", "Lucille", "Lucinda" },
    "Louann" => phf_set! { "Luann" },
    "Louanne" => phf_set! { "Luann" },
    "Lousie" => phf_set! { "Eliza", "Louise", "Louisa", "Lois", "Louetta", "Elouise", "Eloise", "Heloise" },
    "Louvina" => phf_set! { "Lavinia" },
    "Louvinia" => phf_set! { "Lavinia" },
    "Loyd" => phf_set! { "Lloyd" },
    "Luana" => phf_set! { "Luanna" },
    "Lucas" => phf_set! { "Lucias" },
    "Lucinda" => phf_set! { "Cynthia" },
    "Luke" => phf_set! { "Lucias", "Luthor", "Lucas" },
    "Lula" => phf_set! { "Luella" },
    "Lulu" => phf_set! { "Luann", "Luciana", "Lou" },
    "Lum" => phf_set! { "Columbus" },
    "Lupita" => phf_set! { "Guadalupe" },
    "Lyn" => phf_set! { "Belinda" },
    "Lynette" => phf_set! { "Linda" },
    "Lynn" => phf_set! { "Caroline", "Celinda", "Linda", "Lyndon" },
    "Lynne" => phf_set! { "Belinda", "Melinda" },
    "Mabel" => phf_set! { "Mehitabel" },
    "Mac" => phf_set! { "Malcolm" },
    "Maciek" => phf_set! { "Maciej" },
    "Madeleine" => phf_set! { "Madeline" },
    "Madge" => phf_set! { "Madeline", "Magdelina", "Margaret" },
    "Magda" => phf_set! { "Madeline", "Magdelina" },
    "Magdalen" => phf_set! { "Magdelina" },
    "Mahdi" => phf_set! { "Mehdi" },
    "Mahi" => phf_set! { "Mahesh" },
    "Maida" => phf_set! { "Madeline", "Magdelina", "Magdalena" },
    "Maka" => phf_set! { "Macarena" },
    "Malgorzata" => phf_set! { "Małgorzata" },
    "Malik" => phf_set! { "Malick" },
    "Malu" => phf_set! { "Luiza" },
    "Manh" => phf_set! { "Hung" },
    "Manu" => phf_set! { "Manoj", "Emmanuel", "Emanuela", "Emanuele" },
    "Manuel" => phf_set! { "Manolo" },
    "Marco" => phf_set! { "Marko" },
    "Marcos" => phf_set! { "Markos" },
    "Margaret" => phf_set! { "Gretchen" },
    "Margauerite" => phf_set! { "Margarita" },
    "Margo" => phf_set! { "Margaret" },
    "Marianna" => phf_set! { "Maryanne" },
    "Marianne" => phf_set! { "Maryanne" },
    "Maris" => phf_set! { "Demerias" },
    "Marisol" => phf_set! { "Marysol" },
    "Mark" => phf_set! { "Marcus", "Marco" },
    "Marx" => phf_set! { "Marques" },
    "Maryam" => phf_set! { "Mariam" },
    "Mat" => phf_set! { "Martha" },
    "Mathilda" => phf_set! { "Matilda" },
    "Matias" => phf_set! { "Mathias" },
    "Matthias" => phf_set! { "Matthew" },
    "Maud" => phf_set! { "Madeline", "Matilda" },
    "Mauro" => phf_set! { "Mauricio" },
    "Max" => phf_set! { "Massimo" },
    "Mayor" => phf_set! { "Mayowa" },
    "Medora" => phf_set! { "Dorothy" },
    "Mees" => phf_set! { "Bartholomew" },
    "Meg" => phf_set! { "Margaret", "Meagan" },
    "Megan" => phf_set! { "Margaret", "Meggie" },
    "Mehdi" => phf_set! { "Mahdi" },
    "Mehetabel" => phf_set! { "Mehitabel" },
    "Mehetable" => phf_set! { "Mehitabel" },
    "Mehitable" => phf_set! { "Mehitabel" },
    "Miera" => phf_set! { "Amira" },
    "Mel" => phf_set! { "Amelia" },
    "Mell" => phf_set! { "Mildred" },
    "Melo" => phf_set! { "Mello" },
    "Memo" => phf_set! { "Mehmet", "Guillermo" },
    "Merlyn" => phf_set! { "Merlin" },
    "Mero" => phf_set! { "Marwa" },
    "Mert" => phf_set! { "Myrtle" },
    "Merv" => phf_set! { "Marvin" },
    "Mervyn" => phf_set! { "Marvin" },
    "Meta" => phf_set! { "Margaret" },
    "Metta" => phf_set! { "Margaret" },
    "Meus" => phf_set! { "Bartholomew" },
    "Mia" => phf_set! { "Marianna" },
    "Michal" => phf_set! { "Michał" },
    "Mick" => phf_set! { "Michael" },
    "Midge" => phf_set! { "Margaret" },
    "Mike" => phf_set! { "Michael", "Miguel" },
    "Mikele" => phf_set! { "Michele" },
    "Miki" => phf_set! { "Michela" },
    "Mikolaj" => phf_set! { "Mikołaj" },
    "Milla" => phf_set! { "Camila" },
    "Mina" => phf_set! { "Mindwell", "Minerva" },
    "Minerva" => phf_set! { "Manerva" },
    "Miriam" => phf_set! { "Mirian", "Mairim" },
    "Misra" => phf_set! { "Mishra" },
    "Mock" => phf_set! { "Democrates" },
    "Mohd" => phf_set! { "Mohammed" },
    "Mohamad" => phf_set! { "Mohammed" },
    "Mohamed" => phf_set! { "Mohammed" },
    "Mohammad" => phf_set! { "Mohammed" },
    "Muhammed" => phf_set! { "Mohammed" },
    "Muhammad" => phf_set! { "Mohammed" },
    "Moll" => phf_set! { "Mary" },
    "Montesque" => phf_set! { "Montgomery" },
    "Morris" => phf_set! { "Maurice" },
    "Moses" => phf_set! { "Amos" },
    "Moss" => phf_set! { "Moses" },
    "Mostafa" => phf_set! { "Moustafa" },
    "Murat" => phf_set! { "Murad" },
    "Myra" => phf_set! { "Almira", "Elmira", "Amirah" },
    "Nace" => phf_set! { "Ignatius" },
    "Nacho" => phf_set! { "Ignacio" },
    "Nada" => phf_set! { "Nadine" },
    "Nadia" => phf_set! { "Nadezhda", "Nadya" },
    "Naldo" => phf_set! { "Reginald", "Ronald" },
    "Nan" => phf_set! { "Anna", "Hannah" },
    "Nana" => phf_set! { "Anna" },
    "Naqvi" => phf_set! { "Haider" },
    "Naser" => phf_set! { "Nasser" },
    "Nate" => phf_set! { "Ignatius" },
    "Nati" => phf_set! { "Natalia" },
    "Neal" => phf_set! { "Cornelius" },
    "Ned" => phf_set! { "Edmund", "Edward", "Edwin" },
    "Neil" => phf_set! { "Cornelius" },
    "Nell" => phf_set! { "Eleanor", "Helena", "Cornelia" },
    "Nelle" => phf_set! { "Eleanor", "Helena", "Cornelia" },
    "Nessa" => phf_set! { "Agnes" },
    "Net" => phf_set! { "Antoinette" },
    "Neto" => phf_set! { "Netto", "Ernesto" },
    "Netta" => phf_set! { "Antoinette" },
    "Neva" => phf_set! { "Genevieve" },
    "Nha" => phf_set! { "Bruna" },
    "Nib" => phf_set! { "Isabella" },
    "Nick" => phf_set! { "Dominic", "Nicholas" },
    "Nicodemus" => phf_set! { "Nicholas" },
    "Nicolas" => phf_set! { "Nicholas" },
    "Nicolay" => phf_set! { "Nikolai" },
    "Niel" => phf_set! { "Cornelius" },
    "Night" => phf_set! { "Knight" },
    "Niki" => phf_set! { "Nikolett" },
    "Nikki" => phf_set! { "Nicola", "Nicole", "Nikita" },
    "Niko" => phf_set! { "Nicolas" },
    "Nikos" => phf_set! { "Nikolaos" },
    "Nina" => phf_set! { "Enedina" },
    "Nomi" => phf_set! { "Noman" },
    "Nora" => phf_set! { "Eleanor" },
    "Norah" => phf_set! { "Honora" },
    "Nowell" => phf_set! { "Noel" },
    "Nura" => phf_set! { "Amalina" },
    "Obed" => phf_set! { "Obadiah" },
    "Odo" => phf_set! { "Odell" },
    "Ofa" => phf_set! { "Mustofa", "Mostofa" },
    "Ola" => phf_set! { "Aleksandra" },
    "Olga" => phf_set! { "Olia" },
    "Oliver" => phf_set! { "Oliveira" },
    "Olph" => phf_set! { "Rudolphus" },
    "Ondra" => phf_set! { "Ondrej" },
    "Ono" => phf_set! { "Tono", "Margono", "Martono", "Hartono" },
    "Ora" => phf_set! { "Aurelia", "Aurilla" },
    "Ore" => phf_set! { "Moore" },
    "Orilla" => phf_set! { "Aurelia", "Aurilla" },
    "Orlando" => phf_set! { "Roland" },
    "Orphelia" => phf_set! { "Ophelia" },
    "Oscar" => phf_set! { "Oskar" },
    "Osman" => phf_set! { "Othman" },
    "Oswald" => phf_set! { "Waldo" },
    "Otis" => phf_set! { "Othello" },
    "Pancho" => phf_set! { "Francisco" },
    "Panos" => phf_set! { "Panagiotis" },
    "Parmelia" => phf_set! { "Amelia" },
    "Pate" => phf_set! { "Peter" },
    "Pati" => phf_set! { "Patrycja" },
    "Pato" => phf_set! { "Patricio" },
    "Pauli" => phf_set! { "Paula" },
    "Pawel" => phf_set! { "Paweł" },
    "Peg" => phf_set! { "Margaret" },
    "Permelia" => phf_set! { "Amelia" },
    "Pheobe" => phf_set! { "Tryphena" },
    "Pherbia" => phf_set! { "Pharaba" },
    "Pheriba" => phf_set! { "Pharaba" },
    "Phidelia" => phf_set! { "Bedelia", "Fidelia" },
    "Phililpa" => phf_set! { "Philipina" },
    "Phineas" => phf_set! { "Alphinias" },
    "Phoebe" => phf_set! { "Philipina" },
    "Pinar" => phf_set! { "Pınar" },
    "Pino" => phf_set! { "Giuseppe" },
    "Pip" => phf_set! { "Philip" },
    "Pipe" => phf_set! { "Felipe" },
    "Ples" => phf_set! { "Pleasant" },
    "Poe" => phf_set! { "Putri" },
    "Pola" => phf_set! { "Paola" },
    "Polo" => phf_set! { "Leopoldo" },
    "Poncho" => phf_set! { "Alfonso" },
    "Puss" => phf_set! { "Philadelphia", "Prudence" },
    "Quil" => phf_set! { "Aquilla" },
    "Quinn" => phf_set! { "Quince" },
    "Quint" => phf_set! { "Quince" },
    "Raech" => phf_set! { "Rachel" },
    "Rafal" => phf_set! { "Rafał" },
    "Raff" => phf_set! { "Raphael" },
    "Rahim" => phf_set! { "Raheem" },
    "Rajiv" => phf_set! { "Rajeev" },
    "Raju" => phf_set! { "Rajendra" },
    "Ralf" => phf_set! { "Ralph" },
    "Ralph" => phf_set! { "Raphael" },
    "Ramadan" => phf_set! { "Ramadhan" },
    "Rana" => phf_set! { "Lorraine" },
    "Randall" => phf_set! { "Randolph" },
    "Ravi" => phf_set! { "Ramakrishna" },
    "Ray" => phf_set! { "Regina" },
    "Reba" => phf_set! { "Rebecca" },
    "Refina" => phf_set! { "Rufina" },
    "Regis" => phf_set! { "Reginaldo" },
    "Rena" => phf_set! { "Irene", "Maureen", "Sabrina" },
    "Renaldo" => phf_set! { "Reginald" },
    "Retta" => phf_set! { "Henrietta", "Chiara" },
    "Reynold" => phf_set! { "Reginald" },
    "Rhoda" => phf_set! { "Rhodella" },
    "Ricardo" => phf_set! { "Richard" },
    "Rich" => phf_set! { "Alderick" },
    "Rick" => phf_set! { "Eric", "Richard" },
    "Ricka" => phf_set! { "Frederica" },
    "Rico" => phf_set! { "Ricardo" },
    "Riki" => phf_set! { "Riccardo" },
    "Rita" => phf_set! { "Margaret" },
    "Rod" => phf_set! { "Roger" },
    "Rodger" => phf_set! { "Roger" },
    "Roland" => phf_set! { "Orlando" },
    "Rolf" => phf_set! { "Rudolphus" },
    "Rollo" => phf_set! { "Roland", "Rudolphus" },
    "Ron" => phf_set! { "Veronica" },
    "Ronna" => phf_set! { "Veronica" },
    "Rosabella" => phf_set! { "Isabella" },
    "Rosable" => phf_set! { "Rosabella" },
    "Rosalinda" => phf_set! { "Rosalyn" },
    "Roso" => phf_set! { "Osorio" },
    "Rowland" => phf_set! { "Roland" },
    "Rox" => phf_set! { "Roseanne" },
    "Roxane" => phf_set! { "Roseanne" },
    "Roxanna" => phf_set! { "Roseanne" },
    "Roxanne" => phf_set! { "Roseanne" },
    "Roz" => phf_set! { "Rosabella", "Rosalyn", "Roseanne" },
    "Rube" => phf_set! { "Reuben" },
    "Rupert" => phf_set! { "Robert" },
    "Rye" => phf_set! { "Zachariah" },
    "Sabe" => phf_set! { "Isabella" },
    "Sabra" => phf_set! { "Isabella" },
    "Sadiq" => phf_set! { "Abubakar" },
    "Sal" => phf_set! { "Solomon" },
    "Sale" => phf_set! { "Halo" },
    "Salim" => phf_set! { "Saleem" },
    "Salmon" => phf_set! { "Solomon" },
    "Samantha" => phf_set! { "Samuel" },
    "Samson" => phf_set! { "Sampson" },
    "Sandra" => phf_set! { "Alexandria" },
    "Sangi" => phf_set! { "Sangeetha" },
    "Sanz" => phf_set! { "Sanchez" },
    "Sarn" => phf_set! { "Arnold" },
    "Sasha" => phf_set! { "Alexander", "Alexandria" },
    "Saul" => phf_set! { "Solomon" },
    "Sean" => phf_set! { "Shaun" },
    "Sene" => phf_set! { "Asenath" },
    "Serena" => phf_set! { "Sabrina" },
    "Serene" => phf_set! { "Cyrenius" },
    "Seymore" => phf_set! { "Seymour" },
    "Shaik" => phf_set! { "Basha" },
    "Shane" => phf_set! { "Shaun" },
    "Sharyn" => phf_set! { "Sharon" },
    "Shawn" => phf_set! { "Shaun" },
    "Shayne" => phf_set! { "Shaun" },
    "Shelton" => phf_set! { "Sheldon" },
    "Sher" => phf_set! { "Sharon" },
    "Sheron" => phf_set! { "Sharon" },
    "Sheryl" => phf_set! { "Sharon" },
    "Sheryn" => phf_set! { "Sharon" },
    "Si" => phf_set! { "Cyrus", "Josiah", "Sylvester" },
    "Sibbell" => phf_set! { "Sibbilla" },
    "Sibyl" => phf_set! { "Sibbilla" },
    "Sigmund" => phf_set! { "Sigismund" },
    "Silla" => phf_set! { "Priscilla" },
    "Silver" => phf_set! { "Sylvester" },
    "Silvester" => phf_set! { "Sylvester" },
    "Simon" => phf_set! { "Simeon" },
    "Sion" => phf_set! { "Simeon" },
    "Sis" => phf_set! { "Frances" },
    "Siti" => phf_set! { "Fatimah", "City" },
    "Siva" => phf_set! { "Shiva" },
    "Smit" => phf_set! { "Mitchell" },
    "Sophia" => phf_set! { "Sophronia" },
    "Soren" => phf_set! { "Søren" },
    "Spar" => phf_set! { "Parker" },
    "Srah" => phf_set! { "Rahman" },
    "Steve" => phf_set! { "Stephen" },
    "Steven" => phf_set! { "Stephen" },
    "Stewart" => phf_set! { "Stuart" },
    "Susi" => phf_set! { "Susan", "Susannah" },
    "Susana" => phf_set! { "Susannah" },
    "Suzanne" => phf_set! { "Susannah" },
    "Swene" => phf_set! { "Cyrenius" },
    "Syah" => phf_set! { "Firman" },
    "Sybrina" => phf_set! { "Sabrina" },
    "Syd" => phf_set! { "Sidney" },
    "Sylvanus" => phf_set! { "Sylvester" },
    "Tad" => phf_set! { "Thaddeus", "Theodore" },
    "Tamarra" => phf_set! { "Tamara" },
    "Tamzine" => phf_set! { "Thomasine" },
    "Tata" => phf_set! { "Tatiana" },
    "Tave" => phf_set! { "Octavia" },
    "Ted" => phf_set! { "Edmund", "Edward", "Theodore" },
    "Temera" => phf_set! { "Tamara" },
    "Terence" => phf_set! { "Terrence" },
    "Teresa" => phf_set! { "Theresa" },
    "Terrance" => phf_set! { "Terrence" },
    "Tess" => phf_set! { "Esther", "Theresa" },
    "Tessa" => phf_set! { "Theresa" },
    "Than" => phf_set! { "Nathaniel" },
    "Theodora" => phf_set! { "Theodosia" },
    "Theodore" => phf_set! { "Theodrick" },
    "Thias" => phf_set! { "Matthew" },
    "Thirsa" => phf_set! { "Theresa" },
    "Thomasa" => phf_set! { "Thomasine" },
    "Thriza" => phf_set! { "Theresa" },
    "Thursa" => phf_set! { "Theresa" },
    "Tiah" => phf_set! { "Azariah" },
    "Tick" => phf_set! { "Felicity" },
    "Timi" => phf_set! { "Timea" },
    "Tina" => phf_set! { "Augusta", "Christiana", "Ernestine" },
    "Tish" => phf_set! { "Letitia", "Patricia" },
    "Tom" => phf_set! { "Thomas" },
    "Tomek" => phf_set! { "Tomasz" },
    "Tomi" => phf_set! { "Tamas", "Tomas" },
    "Toni" => phf_set! { "Antonia" },
    "Trina" => phf_set! { "Katherine" },
    "Trish" => phf_set! { "Patricia" },
    "Trisha" => phf_set! { "Beatrice" },
    "Trix" => phf_set! { "Beatrice" },
    "Tung" => phf_set! { "Nguyen" },
    "Uddin" => phf_set! { "Khairuddin", "Amiruddin", "Alauddin" },
    "Ugo" => phf_set! { "Hugo" },
    "Ulana" => phf_set! { "Maulana" },
    "Ullah" => phf_set! { "Sanaullah", "Khairullah", "Amirullah", "Amrullah" },
    "Uma" => phf_set! { "Maheswari" },
    "Ung" => phf_set! { "Leung", "Hanung" },
    "Ur" => phf_set! { "Rehman" },
    "Ura" => phf_set! { "Mastura" },
    "Uran" => phf_set! { "Duran" },
    "Uri" => phf_set! { "Oriol", "Mashuri", "Kasturi" },
    "Utz" => phf_set! { "Ionut" },
    "Uyen" => phf_set! { "Huyen" },
    "Valeda" => phf_set! { "Valentina" },
    "Vanna" => phf_set! { "Vanessa" },
    "Verna" => phf_set! { "Laverne" },
    "Vest" => phf_set! { "Sylvester" },
    "Vet" => phf_set! { "Sylvester" },
    "Vick" => phf_set! { "Victor" },
    "Vina" => phf_set! { "Lavinia" },
    "Viola" => phf_set! { "Violet" },
    "Volodia" => phf_set! { "Vladimir" },
    "Waldo" => phf_set! { "Oswald" },
    "Wat" => phf_set! { "Walter" },
    "Webb" => phf_set! { "Webster" },
    "Wenefred" => phf_set! { "Winifred" },
    "Wib" => phf_set! { "Wilber" },
    "Wilber" => phf_set! { "Gilbert" },
    "Wilbur" => phf_set! { "Wilber" },
    "Wilhelm" => phf_set! { "William" },
    "Will" => phf_set! { "Wilber", "Wilfred", "Wilhelm" },
    "Willis" => phf_set! { "William" },
    "Wilma" => phf_set! { "Wilhelmina" },
    "Winnet" => phf_set! { "Winifred" },
    "Wyncha" => phf_set! { "Lavinia" },
    "Xan" => phf_set! { "Alexandria", "Alexandre" },
    "Xena" => phf_set! { "Christiana" },
    "Xina" => phf_set! { "Christiana" },
    "Xu" => phf_set! { "Hsu" },
    "Yolonda" => phf_set! { "Yolanda" },
    "Zacharias" => phf_set! { "Zachariah" },
    "Zack" => phf_set! { "Zach" },
    "Zadock" => phf_set! { "Melchizedek" },
    "Zay" => phf_set! { "Isaiah" },
    "Zed" => phf_set! { "Zadock" },
    "Zeke" => phf_set! { "Ezekiel", "Isaac", "Zachariah" },
    "Zella" => phf_set! { "Zelphia" },
    "Zeph" => phf_set! { "Zepaniah" },
    "Zhang" => phf_set! { "Cheung" },
    "Zhou" => phf_set! { "Chou", "Chow" },
    "Zubiah" => phf_set! { "Azubah" },
};

static DIMINUTIVE_EXCEPTIONS: phf::Set<&'static str> = phf_set! {
    "Mary",
    "Joy",
    "Roy",
    "Guy",
    "Amy",
    "Troy",
};

static FINAL_SYLLABLES_EXCEPTIONS: phf::Set<&'static str> = phf_set! {
    "Nathan", // Probably != Jonathan
};
