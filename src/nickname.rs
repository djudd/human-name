use super::transliterate;
use case::*;
use features::starts_with_consonant;
use phf::{phf_map, phf_set};
use std::borrow::Cow;
use std::iter;

// Returns tuple (close_char, must_precede_whitespace)
#[inline]
fn expected_close_char_if_opens_nickname(
    c: char,
    follows_whitespace: bool,
) -> Option<(char, bool)> {
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

    if close.is_some() {
        // Treat, e.g., opening parens as the start of a nickname
        // regardless of where it occurs
        return close;
    }

    if follows_whitespace {
        // Treat, e.g., quote character as the start of a nickname
        // only if it occurs after whitespace; otherwise, it
        // might be in-name punctuation
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

struct NickOpen {
    start_index: usize,
    open_char: char,
    follows_space: bool,
    expect_close_char: char,
    expect_closing_space: bool,
}

#[inline]
fn find_nick_open(input: &str) -> Option<NickOpen> {
    let mut follows_space = false;

    for (i, c) in input.char_indices() {
        if let Some((expect_close_char, expect_closing_space)) =
            expected_close_char_if_opens_nickname(c, follows_space)
        {
            return Some(NickOpen {
                start_index: i,
                open_char: c,
                follows_space,
                expect_close_char,
                expect_closing_space,
            });
        }

        follows_space = c == ' ';
    }

    None
}

#[cold]
fn find_close_and_strip(input: &str, open: NickOpen) -> Cow<str> {
    let NickOpen {
        start_index,
        open_char,
        follows_space,
        expect_close_char,
        expect_closing_space,
    } = open;

    let search_from = start_index + open_char.len_utf8();
    let strip_from = if follows_space {
        start_index - 1
    } else {
        start_index
    };

    if let Some(found_at) = input[search_from..].find(expect_close_char) {
        let i = found_at + search_from;
        let j = i + expect_close_char.len_utf8();

        if j >= input.len() {
            Cow::Borrowed(&input[0..start_index])
        } else if !expect_closing_space || input[j..].starts_with(' ') {
            Cow::Owned(input[0..strip_from].to_string() + &strip_nickname(&input[j..]))
        } else {
            Cow::Owned(input[0..i].to_string() + &strip_nickname(&input[i..]))
        }
    } else if !expect_closing_space {
        // When there's, e.g., an opening parens, but no closing parens, strip the
        // rest of the string
        Cow::Borrowed(&input[0..strip_from])
    } else {
        // Otherwise, even if there's an unmatched opening quote, don't
        // modify the string; assume an unmatched opening quote was just
        // in-name punctuation
        //
        // However, in that case, we need to check the remainder of the
        // string for actual nicknames, whose opening character we might
        // have missed while looking for the first closing character
        if search_from >= input.len() {
            Cow::Borrowed(input)
        } else {
            Cow::Owned(input[0..search_from].to_string() + &strip_nickname(&input[search_from..]))
        }
    }
}

// Optimized for the case where there is no nickname, and secondarily for the
// case where there is only one. Two or more probably means bad input.
pub fn strip_nickname(input: &str) -> Cow<str> {
    if let Some(open) = find_nick_open(input) {
        find_close_and_strip(input, open)
    } else {
        Cow::Borrowed(input)
    }
}

struct NameVariants<'a> {
    original: &'a str,
    direct_variants: Option<&'a [&'static str]>,
    prefix_variants: Option<&'a [&'static str]>,
}

impl<'a> NameVariants<'a> {
    pub fn for_name(name: &'a str) -> NameVariants<'a> {
        NameVariants {
            original: name,
            direct_variants: NAMES_BY_IRREGULAR_NICK.get(name).copied(),
            prefix_variants: {
                if name.len() >= 4 && (name.ends_with("ie") || name.ends_with("ey")) {
                    NAMES_BY_NICK_PREFIX.get(&name[0..name.len() - 2]).copied()
                } else if name.len() >= 3 && name.ends_with('y') {
                    NAMES_BY_NICK_PREFIX.get(&name[0..name.len() - 1]).copied()
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
    direct_variants: Option<std::slice::Iter<'a, &'static str>>,
    prefix_variants: Option<std::slice::Iter<'a, &'static str>>,
}

impl<'a> Iterator for NameVariantIter<'a> {
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

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.original.len()
            + self
                .direct_variants
                .as_ref()
                .map(|vs| vs.len())
                .unwrap_or(0)
            + self
                .prefix_variants
                .as_ref()
                .map(|vs| vs.len())
                .unwrap_or(0);
        (size, Some(size))
    }
}

impl<'a> ExactSizeIterator for NameVariantIter<'a> {}

pub fn have_matching_variants(original_a: &str, original_b: &str) -> bool {
    let original_a = transliterate::to_ascii_titlecase(original_a);
    let original_b = transliterate::to_ascii_titlecase(original_b);

    let a_variants = NameVariants::for_name(&original_a);
    let b_variants = NameVariants::for_name(&original_b);

    a_variants.iter_with_original().any(|a| {
        b_variants
            .iter_with_original()
            .any(|b| variants_match(a, b))
    })
}

#[inline]
fn variants_match(a: &str, b: &str) -> bool {
    let (longer, shorter) = if a.len() >= b.len() { (a, b) } else { (b, a) };

    have_prefix_match(longer, shorter)
        || is_final_syllables_of(shorter, longer)
        || matches_without_diminutive(a, b)
        || matches_without_diminutive(b, a)
}

#[inline]
fn have_prefix_match(longer: &str, shorter: &str) -> bool {
    eq_or_starts_with(longer, shorter) && !is_simple_feminization(longer, shorter)
}

#[inline]
fn is_simple_feminization(longer: &str, shorter: &str) -> bool {
    longer.len() == shorter.len() + 1 && longer.ends_with('a')
}

#[inline]
fn matches_without_diminutive(a: &str, b: &str) -> bool {
    matches_without_y_or_e(a, b)
        || matches_without_ie_or_ey(a, b)
        || matches_without_ita_or_ina(a, b)
        || matches_without_ito(a, b)
}

#[inline]
fn matches_without_y_or_e(a: &str, b: &str) -> bool {
    a.len() > 2
        && b.len() >= a.len() - 1
        && (a.ends_with('y') || a.ends_with('e'))
        && matches_after_removing_diminutive(a, b, 1)
}

#[inline]
fn matches_without_ie_or_ey(a: &str, b: &str) -> bool {
    a.len() > 4
        && b.len() >= a.len() - 2
        && (a.ends_with("ie") || a.ends_with("ey"))
        && matches_after_removing_diminutive(a, b, 2)
}

#[inline]
fn matches_without_ita_or_ina(a: &str, b: &str) -> bool {
    a.len() > 5
        && b.len() >= a.len() - 3
        && b.ends_with('a')
        && (a.ends_with("ita") || a.ends_with("ina"))
        && matches_after_removing_diminutive(a, b, 3)
}

#[inline]
fn matches_without_ito(a: &str, b: &str) -> bool {
    a.len() > 5
        && b.len() >= a.len() - 3
        && b.ends_with('o')
        && a.ends_with("ito")
        && matches_after_removing_diminutive(a, b, 3)
}

#[inline]
fn matches_after_removing_diminutive(a: &str, b: &str, diminutive_len: usize) -> bool {
    eq_or_starts_with(&a[0..a.len() - diminutive_len], b) && !DIMINUTIVE_EXCEPTIONS.contains(a)
}

#[inline]
fn is_final_syllables_of(needle: &str, haystack: &str) -> bool {
    if needle.len() == haystack.len() - 1
        && !starts_with_consonant(haystack)
        && eq_or_ends_with(needle, haystack)
    {
        true
    } else if haystack.len() < 4 || needle.len() < 2 || needle.len() > haystack.len() - 2 {
        false
    } else if starts_with_consonant(needle)
        || needle.starts_with("Ann")
        || haystack.starts_with("Mary")
    {
        eq_or_ends_with(needle, haystack) && !FINAL_SYLLABLES_EXCEPTIONS.contains(needle)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "bench")]
    use test::{black_box, Bencher};

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
    fn variants() {
        assert_eq!(
            vec!["Ada", "Adelaide", "Adele", "Adelina", "Adeline"],
            NameVariants::for_name("Ada")
                .iter_with_original()
                .collect::<Vec<_>>()
        );
        assert_eq!(
            vec!["Adele"],
            NameVariants::for_name("Adele")
                .iter_with_original()
                .collect::<Vec<_>>()
        );
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
        assert_eq!(
            "Robert Mr. Bob' Roberts",
            strip_nickname("Robert Mr. Bob' Roberts")
        );
    }

    #[test]
    fn unspaced_quotes() {
        assert_eq!("Ro'bert R'oberts", strip_nickname("Ro'bert R'oberts"));
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn strip_nick_no_nick(b: &mut Bencher) {
        b.iter(|| {
            black_box(strip_nickname("James T. Kirk").len());
        })
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn strip_nick_with_nick(b: &mut Bencher) {
        b.iter(|| {
            black_box(strip_nickname("James T. 'Jimmy' Kirk").len());
        })
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn have_matching_variants_false(b: &mut Bencher) {
        b.iter(|| {
            black_box(have_matching_variants("David", "Daniel"));
        })
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn have_matching_variants_true(b: &mut Bencher) {
        b.iter(|| {
            black_box(have_matching_variants("David", "Dave"));
        })
    }
}

static NAMES_BY_NICK_PREFIX: phf::Map<&'static str, &'static [&'static str]> = phf_map! {
    "Ann" => &[ "Agnes", "Antoinette", "Marianna", "Roseanne", "Anabelle", "Luann" ],
    "Babb" => &[ "Barbara" ],
    "Bais" => &[ "Elizabeth" ],
    "Baiss" => &[ "Elizabeth" ],
    "Bald" => &[ "Archibald" ],
    "Barber" => &[ "Barbara" ],
    "Beck" => &[ "Rebecca" ],
    "Beed" => &[ "Obedience" ],
    "Bern" => &[ "Barnabas" ],
    "Bess" => &[ "Elizabeth" ],
    "Bets" => &[ "Elizabeth" ],
    "Bett" => &[ "Elizabeth" ],
    "Bill" => &[ "William" ],
    "Bird" => &[ "Roberta" ],
    "Bits" => &[ "Elizabeth" ],
    "Bonn" => &[ "Bonita" ],
    "Brad" => &[ "Broderick" ],
    "Bradl" => &[ "Bradford" ],
    "Cadd" => &[ "Caroline" ],
    "Camm" => &[ "Camille" ],
    "Carl" => &[ "Karla" ],
    "Cath" => &[ "Katherine" ],
    "Cecel" => &[ "Cecilia" ],
    "Creas" => &[ "Lucretia" ],
    "Criss" => &[ "Christiana" ],
    "Dac" => &[ "Candace" ],
    "Dais" => &[ "Margaret" ],
    "Darr" => &[ "Darlene" ],
    "Deann" => &[ "Geraldine" ],
    "Debb" => &[ "Deborah" ],
    "Dell" => &[ "Deliverance" ],
    "Dens" => &[ "Prudence" ],
    "Desr" => &[ "Desiree" ],
    "Dill" => &[ "Deliverance" ],
    "Doll" => &[ "Dorothy" ],
    "Donn" => &[ "Donald" ],
    "Dos" => &[ "Eudoris" ],
    "Doss" => &[ "Eudoris" ],
    "Dott" => &[ "Dorothy" ],
    "Edd" => &[ "Edmund", "Edward", "Edgar", "Edith" ],
    "Edn" => &[ "Edith" ],
    "Eff" => &[ "Euphemia" ],
    "Emm" => &[ "Emeline", "Emily" ],
    "Ern" => &[ "Earnest" ],
    "Fall" => &[ "Eliphalet" ],
    "Fan" => &[ "Estefania" ],
    "Fann" => &[ "Frances" ],
    "Ferb" => &[ "Pharaba" ],
    "Finn" => &[ "Phineas" ],
    "Floss" => &[ "Florence" ],
    "Gats" => &[ "Augustus" ],
    "Gatsb" => &[ "Augustus" ],
    "Gatt" => &[ "Gertrude" ],
    "Gen" => &[ "Eugenia" ],
    "Genc" => &[ "Genevieve" ],
    "Geoffr" => &[ "Jefferson" ],
    "Ginn" => &[ "Virginia" ],
    "Gus" => &[ "Augusta" ],
    "Hall" => &[ "Mahalla" ],
    "Happ" => &[ "Karonhappuck" ],
    "Hatt" => &[ "Harriet" ],
    "Heid" => &[ "Adelaide" ],
    "Helm" => &[ "Wilhelmina" ],
    "Hess" => &[ "Hester" ],
    "Hil" => &[ "Hiram" ],
    "Hitt" => &[ "Mehitabel" ],
    "Horr" => &[ "Horace" ],
    "Hum" => &[ "Posthuma" ],
    "Igg" => &[ "Ignatius" ],
    "Izz" => &[ "Isidore", "Isabelle", "Isobel" ],
    "Jak" => &[ "Jacqueline" ],
    "Jeffr" => &[ "Jefferson" ],
    "Jimm" => &[ "James" ],
    "Jin" => &[ "Virginia" ],
    "Jinc" => &[ "Jane" ],
    "Jos" => &[ "Josephine" ],
    "Kar" => &[ "Caroline" ],
    "Kas" => &[ "Casey" ],
    "Kat" => &[ "Katherine", "Catherine" ],
    "Kenj" => &[ "Kendra" ],
    "Ker" => &[ "Caroline" ],
    "Kerst" => &[ "Christiana" ],
    "Kezz" => &[ "Keziah" ],
    "Kimm" => &[ "Kimberly" ],
    "Kiss" => &[ "Calista" ],
    "Kits" => &[ "Katherine" ],
    "Kitt" => &[ "Katherine" ],
    "Krist" => &[ "Christiana", "Christine" ],
    "Kymberl" => &[ "Kimberly" ],
    "Laff" => &[ "Lafayette" ],
    "Lain" => &[ "Elaine" ],
    "Lann" => &[ "Roland" ],
    "Larr" => &[ "Lawrence" ],
    "Laur" => &[ "Lawrence" ],
    "Leaf" => &[ "Relief" ],
    "Leff" => &[ "Lafayette" ],
    "Lenn" => &[ "Leonard" ],
    "Less" => &[ "Celeste" ],
    "Lev" => &[ "Aleva" ],
    "Liv" => &[ "Olivia" ],
    "Lizz" => &[ "Elizabeth" ],
    "Lod" => &[ "Melody" ],
    "Lonn" => &[ "Lawrence" ],
    "Lyd" => &[ "Linda" ],
    "Lydd" => &[ "Linda" ],
    "Madd" => &[ "Madeline", "Madeleine" ],
    "Mais" => &[ "Margaret" ],
    "Malach" => &[ "Malcolm" ],
    "Mam" => &[ "Mary" ],
    "Marger" => &[ "Margaret" ],
    "Marjor" => &[ "Margaret" ],
    "Maver" => &[ "Mavine" ],
    "Midd" => &[ "Madeline" ],
    "Morr" => &[ "Seymour" ],
    "Moss" => &[ "Maurice" ],
    "Nabb" => &[ "Abigail" ],
    "Napp" => &[ "Napoleon" ],
    "Nepp" => &[ "Penelope" ],
    "Ness" => &[ "Agnes" ],
    "Nibb" => &[ "Isabella" ],
    "Nic" => &[ "Vernisee" ],
    "Nikk" => &[ "Nicolena" ],
    "Noll" => &[ "Olivia" ],
    "Non" => &[ "Joanna" ],
    "Norr" => &[ "Honora" ],
    "Onn" => &[ "Iona" ],
    "Oph" => &[ "Theophilus" ],
    "Oss" => &[ "Oswald" ],
    "Ozz" => &[ "Oswald" ],
    "Padd" => &[ "Patrick" ],
    "Parsun" => &[ "Parthenia" ],
    "Pasoon" => &[ "Parthenia" ],
    "Pedd" => &[ "Experience" ],
    "Pegg" => &[ "Margaret" ],
    "Pen" => &[ "Philipina" ],
    "Penn" => &[ "Penelope" ],
    "Perr" => &[ "Pelegrine" ],
    "Phill" => &[ "Adelphia" ],
    "Phoen" => &[ "Tryphena" ],
    "Phos" => &[ "Tryphosia" ],
    "Pok" => &[ "Pocahontas" ],
    "Pon" => &[ "Napoleon" ],
    "Priss" => &[ "Priscilla" ],
    "Quill" => &[ "Aquilla" ],
    "Rodd" => &[ "Rodney" ],
    "Roll" => &[ "Roland" ],
    "Rox" => &[ "Roseanne" ],
    "Rub" => &[ "Reuben" ],
    "Rust" => &[ "Russell" ],
    "Sad" => &[ "Sarah" ],
    "Sall" => &[ "Sarah" ],
    "Samm" => &[ "Samuel", "Samantha" ],
    "Scott" => &[ "Prescott" ],
    "Sen" => &[ "Eseneth" ],
    "Sharr" => &[ "Sharon" ],
    "Sher" => &[ "Sharon" ],
    "Sl" => &[ "Sylvester" ],
    "Smitt" => &[ "Smith" ],
    "Soll" => &[ "Solomon" ],
    "Such" => &[ "Susannah" ],
    "Surr" => &[ "Sarah" ],
    "Suz" => &[ "Susannah", "Susan" ],
    "Sydn" => &[ "Sidney" ],
    "Tabb" => &[ "Tabitha" ],
    "Tall" => &[ "Natalie" ],
    "Tamm" => &[ "Tamara" ],
    "Tell" => &[ "Aristotle" ],
    "Tens" => &[ "Hortense" ],
    "Tent" => &[ "Content" ],
    "Tess" => &[ "Theresa" ],
    "Then" => &[ "Parthenia" ],
    "Tibb" => &[ "Isabella" ],
    "Tic" => &[ "Theresa" ],
    "Timm" => &[ "Timothy" ],
    "Tipp" => &[ "Tipton" ],
    "Tips" => &[ "Tipton" ],
    "Tomm" => &[ "Thomas" ],
    "Tor" => &[ "Victoria" ],
    "Torr" => &[ "Victoria" ],
    "Trac" => &[ "Theresa" ],
    "Trud" => &[ "Gertrude" ],
    "Valer" => &[ "Valentina" ],
    "Vall" => &[ "Valentina" ],
    "Vang" => &[ "Evangeline" ],
    "Vann" => &[ "Vanessa" ],
    "Verg" => &[ "Virginia" ],
    "Vess" => &[ "Sylvester" ],
    "Vin" => &[ "Lavinia" ],
    "Vonn" => &[ "Veronica" ],
    "Wend" => &[ "Gwendolyn" ],
    "Zad" => &[ "Isaiah" ],
    "Zadd" => &[ "Arzada" ],
    "Zoll" => &[ "Solomon" ],
    "Abb" => &[ "Abigail", "Abner", "Absalom", "Abiodun" ],
    "Add" => &[ "Adaline", "Adelaide", "Adelphia", "Agatha", "Ada", "Adele", "Adeline", "Adelina" ],
    "Agg" => &[ "Agatha", "Agnes", "Augusta" ],
    "All" => &[ "Aileen", "Alberta", "Alice", "Almena", "Alison" ],
    "Arr" => &[ "Arabella", "Armena" ],
    "Benn" => &[ "Benedict", "Benjamin", "Benedetta" ],
    "Berr" => &[ "Barry", "Greenberry", "Littleberry" ],
    "Bert" => &[ "Alberta", "Roberta" ],
    "Bidd" => &[ "Bridget", "Obedience" ],
    "Bobb" => &[ "Barbara", "Robert", "Roberta" ],
    "Brid" => &[ "Bertha" ],
    "Call" => &[ "Caldonia", "California", "Calpurnia", "Caroline", "Camilla" ],
    "Carr" => &[ "Caroline", "Karonhappuck" ],
    "Cass" => &[ "Alexandria", "Caroline", "Katherine" ],
    "Cind" => &[ "Cynthia", "Luciana", "Lucinda" ],
    "Ciss" => &[ "Cecilia", "Clarissa", "Frances", "Priscilla" ],
    "Conn" => &[ "Conrad", "Constance", "Cornelius", "Cornelia", "Constanza" ],
    "Dann" => &[ "Daniel", "Sheridan" ],
    "Dic" => &[ "Diana", "Edith", "Eurydice", "Laodicia" ],
    "Dod" => &[ "Delores", "Dorothy" ],
    "Ebb" => &[ "Abel", "Ebenezer" ],
    "Ed" => &[ "Adam" ],
    "El" => &[ "Alice" ],
    "Ell" => &[ "Alexandria", "Eleanor", "Elmira", "Elwood" ],
    "Els" => &[ "Alice", "Elizabeth" ],
    "Emil" => &[ "Amelia", "Emeline" ],
    "Ess" => &[ "Estella", "Hester" ],
    "Ett" => &[ "Carthaette", "Henrietta" ],
    "Frank" => &[ "Francis", "Veronica", "Francesca" ],
    "Fredd" => &[ "Alfred", "Alfreda", "Frederic", "Frederick", "Winifred" ],
    "Fron" => &[ "Sophronia", "Veronica" ],
    "Gabb" => &[ "Gabriel", "Gabrielle" ],
    "Gerr" => &[ "Gerald", "Geraldine", "Gerard", "Gerardo" ],
    "Guss" => &[ "Augusta", "Augustus" ],
    "Harr" => &[ "Harold", "Henry" ],
    "Hett" => &[ "Henrietta", "Hester", "Mehitabel" ],
    "Iss" => &[ "Isabella", "Isidora" ],
    "Jack" => &[ "Jacqueline", "Jaclyn", "Jacquelyn" ],
    "Jazz" => &[ "Jazmin", "Jasmine" ],
    "Jenn" => &[ "Eugenia", "Genevieve", "Jane", "Virginia" ],
    "Jerr" => &[ "Gerald", "Geraldine", "Jeremiah" ],
    "Jins" => &[ "Genevieve", "Jane" ],
    "Jod" => &[ "Joanna", "Joseph", "Josephine" ],
    "Johnn" => &[ "John", "Jonathan" ],
    "Lett" => &[ "Charlotte", "Letitia" ],
    "Libb" => &[ "Elizabeth", "Libuse" ],
    "Lidd" => &[ "Elizabeth", "Linda" ],
    "Lind" => &[ "Celinda", "Lyndon", "Melinda" ],
    "Loll" => &[ "Charlotte", "Delores", "Lillian" ],
    "Lorr" => &[ "Lauryn", "Lawrence", "Loretta" ],
    "Lott" => &[ "Carlotta", "Charlotte" ],
    "Lynd" => &[ "Linda" ],
    "Magg" => &[ "Madeline", "Margaret" ],
    "Mand" => &[ "Amanda", "Miranda" ],
    "Mann" => &[ "Emanuel", "Manuel" ],
    "Mar" => &[ "Maureen", "Miriam", "Mitzi", "Maura", "Moira" ],
    "Matt" => &[ "Martha", "Matilda" ],
    "Mell" => &[ "Amelia", "Melinda", "Permelia" ],
    "Merc" => &[ "Mary" ],
    "Mick" => &[ "Michael", "Michelle" ],
    "Mill" => &[ "Amelia", "Armilda", "Camille", "Emeline", "Melissa", "Mildred", "Permelia", "Milicent" ],
    "Mim" => &[ "Jemima", "Mary", "Mildred", "Miriam" ],
    "Mind" => &[ "Arminda", "Melinda" ],
    "Minn" => &[ "Almina", "Mary", "Minerva", "Wilhelmina" ],
    "Miss" => &[ "Melissa", "Millicent" ],
    "Mitt" => &[ "Mehitabel", "Submit" ],
    "Mitz" => &[ "Mary", "Miriam" ],
    "Moll" => &[ "Amalia", "Amelia", "Martha", "Mary" ],
    "Mont" => &[ "Lamont" ],
    "Mor" => &[ "Maurice", "Seymour" ],
    "Nanc" => &[ "Agnes", "Anna" ],
    "Nann" => &[ "Anna", "Hannah", "Nancy" ],
    "Natt" => &[ "Asenath", "Natalie", "Nathaniel" ],
    "Neel" => &[ "Cornelia", "Cornelius" ],
    "Nell" => &[ "Cornelia", "Eleanor", "Helen" ],
    "Nerv" => &[ "Manerva", "Minerva" ],
    "Nett" => &[ "Antoinette", "Henrietta", "Jane", "Juanita", "Natalie", "Ninell", "Pernetta" ],
    "Nick" => &[ "Nicholas", "Nicolena" ],
    "Oll" => &[ "Oliver", "Olivia" ],
    "Pats" => &[ "Martha", "Patricia", "Patrick" ],
    "Patt" => &[ "Martha", "Matilda", "Parthenia", "Patience", "Patricia" ],
    "Phen" => &[ "Josephine", "Parthenia", "Tryphena" ],
    "Poll" => &[ "Paulina" ],
    "Rand" => &[ "Miranda" ],
    "Reen" => &[ "Irene", "Maureen", "Sabrina" ],
    "Regg" => &[ "Regina", "Reginald" ],
    "Renn" => &[ "Irene", "Reginald" ],
    "Rich" => &[ "Alderick", "Derrick" ],
    "Rick" => &[ "Broderick", "Cedrick", "Eric", "Richard" ],
    "Rill" => &[ "Aurelia", "Aurilla" ],
    "Robb" => &[ "Robert", "Roberta" ],
    "Ronn" => &[ "Aaron", "Cameron", "Ronald", "Veronica" ],
    "Ros" => &[ "Euphrosina" ],
    "Sand" => &[ "Alexander", "Alexandria" ],
    "Shell" => &[ "Michelle", "Rachel", "Sheldon" ],
    "Sherr" => &[ "Charlotte", "Shirley" ],
    "Sonn" => &[ "Anderson", "Jefferson", "Judson" ],
    "Stac" => &[ "Anastasia", "Eustacia" ],
    "Suk" => &[ "Sarah", "Susannah" ],
    "Tedd" => &[ "Edward", "Theodore" ],
    "Terr" => &[ "Theresa", "Terence" ],
    "Till" => &[ "Matilda", "Temperance", "Tilford" ],
    "Ton" => &[ "Anthony", "Antoinette", "Clifton", "Antonio", "Antoni" ],
    "Triss" => &[ "Beatrice", "Theresa" ],
    "Trix" => &[ "Beatrice", "Patricia" ],
    "Vick" => &[ "Veronica", "Victoria" ],
    "Vinn" => &[ "Calvin", "Lavinia", "Vincent" ],
    "Will" => &[ "Wilda", "Wilfred", "Wilhelmina", "Wilma" ],
    "Winn" => &[ "Edwina", "Winfield", "Winifred" ],
    "Wood" => &[ "Elwood" ],
};

static NAMES_BY_IRREGULAR_NICK: phf::Map<&'static str, &'static [&'static str]> = phf_map! {
    "Abagail" => &[ "Abigail" ],
    "Abdo" => &[ "Abdu", "Abdul", "Abdullah" ],
    "Abertina" => &[ "Alberta" ],
    "Abiah" => &[ "Abijah" ],
    "Abram" => &[ "Abraham" ],
    "Acuilla" => &[ "Aquilla" ],
    "Ada" => &[ "Adelaide", "Adele", "Adelina", "Adeline" ],
    "Adaline" => &[ "Adelaide" ],
    "Adela" => &[ "Adaline" ],
    "Adelia" => &[ "Adelaide" ],
    "Adeline" => &[ "Adelaide" ],
    "Adeliza" => &[ "Adelaide" ],
    "Adi" => &[ "Hadi" ],
    "Adia" => &[ "Nadia" ],
    "Ado" => &[ "Rudolphus" ],
    "Adolf" => &[ "Rudolphus" ],
    "Adolphus" => &[ "Rudolphus" ],
    "Adoph" => &[ "Rudolphus" ],
    "Adrianna" => &[ "Adriana" ],
    "Adrienne" => &[ "Adriana" ],
    "Agnes" => &[ "Agatha", "Agnieszka" ],
    "Aileen" => &[ "Ellen" ],
    "Aimee" => &[ "Amy" ],
    "Ain" => &[ "Nuru", "Lain" ],
    "Aini" => &[ "Nuraini" ],
    "Aji" => &[ "Naji" ],
    "Akin" => &[ "Akın" ],
    "Ala" => &[ "Ayala" ],
    "Alaina" => &[ "Alana" ],
    "Alan" => &[ "Allan" ],
    "Albert" => &[ "Adelbert" ],
    "Albertine" => &[ "Alberta" ],
    "Aldi" => &[ "Renaldi", "Reynaldi", "Naldi" ],
    "Alec" => &[ "Alexander" ],
    "Alex" => &[ "Alejandro" ],
    "Alexandra" => &[ "Alexandria" ],
    "Alexei" => &[ "Alexander" ],
    "Alice" => &[ "Alisha", "Alison" ],
    "Alicia" => &[ "Alice" ],
    "Alim" => &[ "Salim" ],
    "Alina" => &[ "Alyna" ],
    "Aline" => &[ "Adaline", "Alline" ],
    "Alisha" => &[ "Alice" ],
    "Alison" => &[ "Alice" ],
    "Alissa" => &[ "Alice" ],
    "Alistair" => &[ "Alastair" ],
    "Alla" => &[ "Alexandria" ],
    "Alle" => &[ "Alessandra" ],
    "Allen" => &[ "Allan" ],
    "Allyson" => &[ "Alice" ],
    "Alonso" => &[ "Alonzo" ],
    "Alonzo" => &[ "Alphonzo" ],
    "Alphus" => &[ "Alphinias" ],
    "Alyson" => &[ "Alice" ],
    "Amabel" => &[ "Mehitabel" ],
    "Amalia" => &[ "Amelia" ],
    "Amar" => &[ "Ammar" ],
    "Amie" => &[ "Amy" ],
    "Amilia" => &[ "Amy" ],
    "Amin" => &[ "Ameen", "Alamin" ],
    "Amir" => &[ "Samir" ],
    "Amos" => &[ "Moses" ],
    "Ance" => &[ "Anderson", "Anselm" ],
    "Andre" => &[ "Anderson" ],
    "Andreas" => &[ "Andrew" ],
    "Andrei" => &[ "Andrew" ],
    "Andria" => &[ "Andrea" ],
    "Angela" => &[ "Angelica", "Angeline", "Angelina" ],
    "Ania" => &[ "Rahmania" ],
    "Anil" => &[ "Anıl" ],
    "Anja" => &[ "Sanjay" ],
    "Anju" => &[ "Anjali", "Anjana" ],
    "Ann" => &[ "Agnes", "Antoinette", "Marianna", "Nancy", "Roseanne", "Ana", "Anita", "Anika", "Ansley", "Antonia", "Anya" ],
    "Anna" => &[ "Ania", "Annette" ],
    "Anne" => &[ "Luann", "Marianna" ],
    "Antoine" => &[ "Anthony", "Anton" ],
    "Antonia" => &[ "Antoinette" ],
    "Antonio" => &[ "Anthony" ],
    "Antony" => &[ "Anthony" ],
    "Antos" => &[ "Antonella" ],
    "Aphinius" => &[ "Alphinias" ],
    "Aphrodite" => &[ "Epaphroditius", "Epaphroditus" ],
    "Aran" => &[ "Karan" ],
    "Archelous" => &[ "Archibald" ],
    "Ardi" => &[ "Nardi" ],
    "Arek" => &[ "Arkadiusz" ],
    "Arianna" => &[ "Ariana" ],
    "Aris" => &[ "Ariez" ],
    "Armanda" => &[ "Amanda" ],
    "Arno" => &[ "Arnaud" ],
    "Aron" => &[ "Aaron" ],
    "Arron" => &[ "Aaron" ],
    "Arslan" => &[ "Arsalan" ],
    "Arya" => &[ "Acharya" ],
    "Asad" => &[ "Assad" ],
    "Asahel" => &[ "Asaph" ],
    "Ashe" => &[ "Tinashe" ],
    "Ashlee" => &[ "Ashley" ],
    "Ashleigh" => &[ "Ashley" ],
    "Asli" => &[ "Aslı", "Aslıhan" ],
    "Assene" => &[ "Asenath" ],
    "Astri" => &[ "Lastri" ],
    "Aubrey" => &[ "Audrey" ],
    "Audra" => &[ "Audrey" ],
    "Augustine" => &[ "Augustus" ],
    "Aura" => &[ "Aurelia" ],
    "Aurilla" => &[ "Aurelia" ],
    "Austen" => &[ "Austin" ],
    "Austin" => &[ "Augustine" ],
    "Ava" => &[ "Avice" ],
    "Axl" => &[ "Axel" ],
    "Aydin" => &[ "aydın" ],
    "Ayu" => &[ "Aiu" ],
    "Azarich" => &[ "Azariah" ],
    "Aziz" => &[ "Abdelaziz" ],
    "Azza" => &[ "Munazza" ],
    "Bab" => &[ "Barbara" ],
    "Babs" => &[ "Barbara" ],
    "Baig" => &[ "Mirza" ],
    "Baldo" => &[ "Archibald" ],
    "Banks" => &[ "Bankole" ],
    "Barnard" => &[ "Barnabas" ],
    "Bartek" => &[ "Bartosz" ],
    "Bartel" => &[ "Bartholomew" ],
    "Bartlomiej" => &[ "Bartłomiej" ],
    "Basia" => &[ "Barbara" ],
    "Basil" => &[ "Bazaleel" ],
    "Bat" => &[ "Bartholomew" ],
    "Bea" => &[ "Blanche" ],
    "Bear" => &[ "Barry" ],
    "Beatrix" => &[ "Beatrice" ],
    "Beatriz" => &[ "Beatrice" ],
    "Beck" => &[ "Rebecca" ],
    "Bede" => &[ "Obedience" ],
    "Bela" => &[ "William" ],
    "Bell" => &[ "Arabella", "Belinda" ],
    "Bella" => &[ "Mehitabel" ],
    "Belle" => &[ "Arabella", "Belinda", "Isabella", "Rosabella" ],
    "Bennett" => &[ "Benedict" ],
    "Bernard" => &[ "Barnabas" ],
    "Bert" => &[ "Alberta", "Elbertson", "Roberta" ],
    "Bess" => &[ "Elizabeth" ],
    "Bethia" => &[ "Elizabeth" ],
    "Beto" => &[ "Alberto" ],
    "Betsy" => &[ "Betty" ],
    "Bex" => &[ "Rebecca" ],
    "Bia" => &[ "Beatriz" ],
    "Biah" => &[ "Abijah" ],
    "Bibi" => &[ "Bianca" ],
    "Bige" => &[ "Abijah" ],
    "Bill" => &[ "William" ],
    "Bird" => &[ "Albert" ],
    "Bjorn" => &[ "Bjørn" ],
    "Blanca" => &[ "Blanche" ],
    "Bo" => &[ "Beaufort", "Beauregard" ],
    "Bob" => &[ "Robert" ],
    "Bobbi" => &[ "Roberta" ],
    "Breanna" => &[ "Brianna" ],
    "Bree" => &[ "Aubrey" ],
    "Brendon" => &[ "Brendan" ],
    "Brian" => &[ "Bryant" ],
    "Briana" => &[ "Brianna" ],
    "Bridgit" => &[ "Bridget" ],
    "Brito" => &[ "Britto" ],
    "Britney" => &[ "Brittani", "Brittany", "Brittney" ],
    "Bruna" => &[ "Brunna" ],
    "Bryan" => &[ "Brian", "Brayan" ],
    "Bryant" => &[ "Brian" ],
    "Bub" => &[ "Mahbubur" ],
    "Buck" => &[ "Charles" ],
    "Burt" => &[ "Bert", "Egbert" ],
    "Cager" => &[ "Micajah" ],
    "Caitlyn" => &[ "Caitlin" ],
    "Callum" => &[ "Calum" ],
    "Candace" => &[ "Candice" ],
    "Cano" => &[ "Kano" ],
    "Car" => &[ "Charlotte" ],
    "Cara" => &[ "Keri" ],
    "Cari" => &[ "Keri" ],
    "Carl" => &[ "Charles" ],
    "Carla" => &[ "Carli", "Karla" ],
    "Carlitos" => &[ "Carlos" ],
    "Carlotta" => &[ "Charlotte" ],
    "Carmella" => &[ "Carmela" ],
    "Carmen" => &[ "Karmen" ],
    "Carolyn" => &[ "Caroline" ],
    "Carrie" => &[ "Keri" ],
    "Casper" => &[ "Jasper" ],
    "Cass" => &[ "Caswell" ],
    "Castle" => &[ "Castillo" ],
    "Catherine" => &[ "Katherine" ],
    "Cathleen" => &[ "Katherine" ],
    "Cathryn" => &[ "Catherin" ],
    "Caz" => &[ "Caroline" ],
    "Ceall" => &[ "Lucille" ],
    "Cecelia" => &[ "Cecilia" ],
    "Cecilia" => &[ "Sheila" ],
    "Celia" => &[ "Cecilia", "Celeste" ],
    "Celina" => &[ "Selina" ],
    "Cene" => &[ "Cyrenius" ],
    "Cenia" => &[ "Laodicia" ],
    "Chad" => &[ "Charles" ],
    "Chan" => &[ "Chauncy" ],
    "Chantal" => &[ "Chantel" ],
    "Chat" => &[ "Charity" ],
    "Chaudhary" => &[ "Choudhary" ],
    "Chaudhry" => &[ "Chaudhary" ],
    "Chet" => &[ "Chesley" ],
    "Chick" => &[ "Charles" ],
    "Chip" => &[ "Charles" ],
    "Christa" => &[ "Christine" ],
    "Christine" => &[ "Christiana" ],
    "Christopher" => &[ "Christian" ],
    "Chrystal" => &[ "Crystal" ],
    "Chuck" => &[ "Charles" ],
    "Ciara" => &[ "Cierra" ],
    "Cibyl" => &[ "Sibbilla" ],
    "Cil" => &[ "Priscilla" ],
    "Cilla" => &[ "Cecilia" ],
    "Ciller" => &[ "Priscilla" ],
    "Cinthia" => &[ "Cynthia" ],
    "Claas" => &[ "Nicholas" ],
    "Claes" => &[ "Nicholas" ],
    "Clair" => &[ "Clarence", "Clarissa" ],
    "Claire" => &[ "Clarissa", "Clara" ],
    "Clara" => &[ "Clarissa" ],
    "Clare" => &[ "Claire" ],
    "Clarice" => &[ "Clarissa" ],
    "Clarisa" => &[ "Clara" ],
    "Clarissa" => &[ "Clara" ],
    "Claus" => &[ "Claudia" ],
    "Cliff" => &[ "Clifton" ],
    "Clo" => &[ "Chloe" ],
    "Clum" => &[ "Columbus" ],
    "Collin" => &[ "Colin" ],
    "Cono" => &[ "Cornelius" ],
    "Cora" => &[ "Corinne", "Corina" ],
    "Crece" => &[ "Lucretia" ],
    "Crese" => &[ "Lucretia" ],
    "Cris" => &[ "Christiana" ],
    "Cristian" => &[ "Christian", "Cristhian" ],
    "Cristina" => &[ "Christiana"],
    "Curg" => &[ "Lecurgus" ],
    "Curt" => &[ "Courtney" ],
    "Dahl" => &[ "Dalton" ],
    "Damaris" => &[ "Demerias" ],
    "Damian" => &[ "Damien" ],
    "Damon" => &[ "Damien" ],
    "Dana" => &[ "Daniel", "Daniela", "Daniella" ],
    "Danelle" => &[ "Danielle" ],
    "Danial" => &[ "Daniel" ],
    "Danni" => &[ "Danielle" ],
    "Darek" => &[ "Dariusz" ],
    "Daria" => &[ "Dasha" ],
    "Darin" => &[ "Daren" ],
    "Darla" => &[ "Dara" ],
    "Darlene" => &[ "Dara" ],
    "Darrell" => &[ "Daryl" ],
    "Darren" => &[ "Daren" ],
    "Darrin" => &[ "Daren" ],
    "Darryl" => &[ "Daryl" ],
    "Daz" => &[ "Darren" ],
    "Deanna" => &[ "Dana" ],
    "Debbe" => &[ "Deborah" ],
    "Debi" => &[ "Deborah" ],
    "Debra" => &[ "Deborah" ],
    "Dee" => &[ "Audrey", "Dorothy" ],
    "Deea" => &[ "Andreea" ],
    "Deedee" => &[ "Deidre", "Nadine" ],
    "Deena" => &[ "Dana" ],
    "Deia" => &[ "Andreia" ],
    "Deidre" => &[ "Deirdra" ],
    "Del" => &[ "Adelbert" ],
    "Delia" => &[ "Adaline", "Dahlia" ],
    "Delilah" => &[ "Dahlia" ],
    "Dell" => &[ "Adaline", "Adelaide", "Adelphia", "Delilah", "Delores", "Rhodella" ],
    "Della" => &[ "Adelaide", "Delilah", "Deliverance", "Delores", "Dahlia" ],
    "Delpha" => &[ "Philadelphia" ],
    "Delphina" => &[ "Adelphia" ],
    "Demaris" => &[ "Demerias" ],
    "Dena" => &[ "Dana" ],
    "Denis" => &[ "Dennis" ],
    "Denys" => &[ "Denise" ],
    "Denyse" => &[ "Denise" ],
    "Derick" => &[ "Derek" ],
    "Derrick" => &[ "Derek" ],
    "Desree" => &[ "Desiree" ],
    "Dessa" => &[ "Andressa" ],
    "Devon" => &[ "Devin" ],
    "Dewayne" => &[ "Duane" ],
    "Deza" => &[ "Andreza" ],
    "Diana" => &[ "Dinah" ],
    "Dianna" => &[ "Dinah" ],
    "Dick" => &[ "Melchizedek", "Richard", "Zadock" ],
    "Dickon" => &[ "Richard" ],
    "Dilbert" => &[ "Delbert" ],
    "Dimmis" => &[ "Demerias" ],
    "Dina" => &[ "Geraldine" ],
    "Dipak" => &[ "Deepak" ],
    "Dirch" => &[ "Derrick" ],
    "Ditus" => &[ "Aphrodite" ],
    "Diya" => &[ "Divya" ],
    "Dob" => &[ "Robert" ],
    "Dobbin" => &[ "Robert" ],
    "Doda" => &[ "Dorothy" ],
    "Dode" => &[ "Dorothy" ],
    "Dolf" => &[ "Randolph", "Rudolphus" ],
    "Dolph" => &[ "Rudolphus" ],
    "Dominick" => &[ "Dominic" ],
    "Dona" => &[ "Caldonia" ],
    "Donna" => &[ "Fredonia" ],
    "Dora" => &[ "Dorothy", "Theodosia" ],
    "Dori" => &[ "Dora" ],
    "Dorinda" => &[ "Dorothy" ],
    "Doris" => &[ "Dorothy" ],
    "Dortha" => &[ "Dorothy" ],
    "Dos" => &[ "Reis" ],
    "Dot" => &[ "Dorothy" ],
    "Dotha" => &[ "Dorothy" ],
    "Drew" => &[ "Woodrow" ],
    "Dru" => &[ "Andrew" ],
    "Duda" => &[ "Eduarda" ],
    "Dunk" => &[ "Duncan" ],
    "Dwane" => &[ "Duane" ],
    "Dwayne" => &[ "Duane" ],
    "Dyce" => &[ "Aphrodite" ],
    "Dyche" => &[ "Aphrodite" ],
    "Dyer" => &[ "Jedediah", "Obadiah", "Zedediah" ],
    "Eb" => &[ "Abel" ],
    "Eddy" => &[ "Reddy" ],
    "Edgar" => &[ "Edward" ],
    "Edith" => &[ "Adaline" ],
    "Edmund" => &[ "Edward", "Edmond" ],
    "Edna" => &[ "Edith" ],
    "Edwin" => &[ "Edith" ],
    "Eid" => &[ "Reid" ],
    "Eileen" => &[ "Aileen", "Helena", "Ellen" ],
    "Eko" => &[ "Echo" ],
    "Elaine" => &[ "Eleanor", "Helena", "Lainey", "Alaina", "Ellen" ],
    "Elbert" => &[ "Adelbert", "Albert", "Alberta" ],
    "Eleanor" => &[ "Helena", "Ellen" ],
    "Eleanora" => &[ "Ellen" ],
    "Eleazar" => &[ "Eleazer" ],
    "Eleck" => &[ "Alexander" ],
    "Electa" => &[ "Electra" ],
    "Elena" => &[ "Helena", "Mariaelena", "Ellen" ],
    "Elenor" => &[ "Leonora" ],
    "Elenora" => &[ "Eleanor" ],
    "Elic" => &[ "Alexandria" ],
    "Elicia" => &[ "Alice" ],
    "Elina" => &[ "Ellen" ],
    "Elinamifia" => &[ "Eleanor" ],
    "Elinor" => &[ "Ellen" ],
    "Eliot" => &[ "Elliott" ],
    "Elis" => &[ "Elizabeth" ],
    "Elisa" => &[ "Elizabeth" ],
    "Elisabeth" => &[ "Elizabeth" ],
    "Elise" => &[ "Elizabeth" ],
    "Elisha" => &[ "Alice" ],
    "Elissa" => &[ "Elizabeth" ],
    "Eliza" => &[ "Elizabeth" ],
    "Ella" => &[ "Eleanor", "Gabrielle", "Helena", "Luella", "Ellen" ],
    "Ellen" => &[ "Eleanor", "Helena" ],
    "Ellender" => &[ "Helena" ],
    "Ellis" => &[ "Alice", "Ellen" ],
    "Ells" => &[ "Elwood" ],
    "Elnora" => &[ "Eleanor" ],
    "Elsa" => &[ "Elizabeth" ],
    "Ema" => &[ "Emma", "Emily" ],
    "Emelia" => &[ "Emily" ],
    "Emely" => &[ "Emily" ],
    "Emelyn" => &[ "Emily" ],
    "Emilia" => &[ "Emily" ],
    "Emiline" => &[ "Emeline" ],
    "Emm" => &[ "Emeline" ],
    "Emma" => &[ "Emeline" ],
    "Emmaline" => &[ "Emily" ],
    "Emmanuel" => &[ "Emanuel" ],
    "Emme" => &[ "Emily" ],
    "Emmeline" => &[ "Emily" ],
    "Emmer" => &[ "Emeline" ],
    "Emmet" => &[ "Emmit" ],
    "Emmit" => &[ "Emmota" ],
    "Ender" => &[ "Mahender" ],
    "Endra" => &[ "Harendra", "Birendra" ],
    "Eppa" => &[ "Aphrodite" ],
    "Ericka" => &[ "Erica" ],
    "Erik" => &[ "Erick", "Eric" ],
    "Erika" => &[ "Erica" ],
    "Erin" => &[ "Aaron" ],
    "Erma" => &[ "Emeline" ],
    "Erna" => &[ "Ernestine" ],
    "Ernest" => &[ "Earnest" ],
    "Erwin" => &[ "Irwin" ],
    "Esa" => &[ "Mahesa" ],
    "Essa" => &[ "Vanessa" ],
    "Ester" => &[ "Esther" ],
    "Esther" => &[ "Hester" ],
    "Etta" => &[ "Carthaette", "Henrietta", "Loretta", "Ethel" ],
    "Eva" => &[ "Evelyn" ],
    "Eve" => &[ "Genevieve" ],
    "Evelin" => &[ "Evelyn" ],
    "Evelina" => &[ "Evelyn" ],
    "Eves" => &[ "Neves" ],
    "Fadi" => &[ "Fahad" ],
    "Faisal" => &[ "Faysal" ],
    "Fan" => &[ "Frances" ],
    "Farah" => &[ "Farrah" ],
    "Fate" => &[ "Lafayette" ],
    "Felicia" => &[ "Felicity" ],
    "Fena" => &[ "Euphrosina" ],
    "Fenee" => &[ "Euphrosina" ],
    "Fernando" => &[ "Ferdinand" ],
    "Ferns" => &[ "Fernandes" ],
    "Fidelia" => &[ "Bedelia" ],
    "Fifi" => &[ "Fiona" ],
    "Fina" => &[ "Josephine" ],
    "Finnius" => &[ "Alphinias" ],
    "Flick" => &[ "Felicity" ],
    "Flora" => &[ "Florence" ],
    "Floss" => &[ "Florence" ],
    "Francis" => &[ "Frances" ],
    "Franco" => &[ "Franko" ],
    "Frank" => &[ "Francis" ],
    "Frankisek" => &[ "Francis" ],
    "Franklin" => &[ "Francis" ],
    "Franz" => &[ "Francis", "Francesco" ],
    "Freda" => &[ "Frederica" ],
    "Frederik" => &[ "Frederick" ],
    "Fredric" => &[ "Frederick" ],
    "Fredrick" => &[ "Frederic" ],
    "Fredricka" => &[ "Frederica" ],
    "Fredrik" => &[ "Frederick" ],
    "Frieda" => &[ "Alfreda", "Frederica" ],
    "Frish" => &[ "Frederick" ],
    "Frits" => &[ "Frederick" ],
    "Fritz" => &[ "Frederick" ],
    "Frona" => &[ "Sophronia" ],
    "Fronia" => &[ "Sophronia" ],
    "Gabriela" => &[ "Gabrielle" ],
    "Gani" => &[ "Ganesh" ],
    "Gay" => &[ "Gerhardt" ],
    "Gee" => &[ "Jehu" ],
    "Gema" => &[ "Gemma" ],
    "Gen" => &[ "Virginia" ],
    "Gene" => &[ "Eugenia" ],
    "Geoff" => &[ "Jeff" ],
    "Geoffrey" => &[ "Jeffrey" ],
    "Georgia" => &[ "Georgina" ],
    "Georgios" => &[ "George" ],
    "Geri" => &[ "Geraldine" ],
    "Ghia" => &[ "Nghia" ],
    "Giang" => &[ "Huong" ],
    "Gib" => &[ "Gilbert" ],
    "Gigi" => &[ "Gisele" ],
    "Gina" => &[ "Virginia", "Georgina" ],
    "Ginger" => &[ "Virginia" ],
    "Gladys" => &[ "Gwen" ],
    "Goes" => &[ "Bagus" ],
    "Gosia" => &[ "Malgorzata" ],
    "Graeme" => &[ "Graham" ],
    "Gram" => &[ "Graham" ],
    "Greta" => &[ "Margaret" ],
    "Gretta" => &[ "Margaret" ],
    "Grissel" => &[ "Griselda" ],
    "Gum" => &[ "Montgomery" ],
    "Gunter" => &[ "Guenter" ],
    "Gunther" => &[ "Guenther" ],
    "Gus" => &[ "Augusta", "Augustus" ],
    "Gwyneth" => &[ "Gwen" ],
    "Habib" => &[ "Habeeb" ],
    "Hadad" => &[ "Haddad" ],
    "Hailey" => &[ "Haley" ],
    "Hakim" => &[ "Hakeem" ],
    "Hal" => &[ "Harold", "Henry", "Howard" ],
    "Hamad" => &[ "Hammad" ],
    "Hamp" => &[ "Hamilton" ],
    "Hanh" => &[ "Khanh" ],
    "Hank" => &[ "Harold", "Henrietta", "Henry" ],
    "Hans" => &[ "John" ],
    "Harman" => &[ "Herman" ],
    "Harris" => &[ "Harrison" ],
    "Hayley" => &[ "Haley" ],
    "Hebsabeth" => &[ "Hepsabah" ],
    "Heide" => &[ "Adelaide" ],
    "Helen" => &[ "Aileen", "Elaine", "Eleanor" ],
    "Hema" => &[ "Latha", "Atha" ],
    "Hence" => &[ "Henry" ],
    "Henk" => &[ "Hendrick" ],
    "Hephsibah" => &[ "Hepsabah" ],
    "Hepsabel" => &[ "Hepsabah" ],
    "Hepsibah" => &[ "Hepsabah" ],
    "Heri" => &[ "Herry" ],
    "Hermoine" => &[ "Hermione" ],
    "Hilary" => &[ "Hillary" ],
    "Hoa" => &[ "Khoa" ],
    "Hopp" => &[ "Hopkins" ],
    "Horatio" => &[ "Horace" ],
    "Hugh" => &[ "Hubert", "Jehu" ],
    "Hugo" => &[ "Hubert", "Hugh" ],
    "Huma" => &[ "Kabir" ],
    "Hung" => &[ "Nhung" ],
    "Hussein" => &[ "Hussien" ],
    "Huy" => &[ "Thuy" ],
    "Hy" => &[ "Hezekiah", "Hiram" ],
    "Iam" => &[ "Ilham" ],
    "Ib" => &[ "Isabella" ],
    "Ida" => &[ "Ada" ],
    "Ike" => &[ "Isaac" ],
    "Ilah" => &[ "Fazilah" ],
    "Illa" => &[ "Faradilla", "Dilla" ],
    "Ima" => &[ "Chandima" ],
    "Iman" => &[ "Budiman" ],
    "Immanuel" => &[ "Emanuel" ],
    "Ina" => &[ "Lavinia" ],
    "Inda" => &[ "Arabinda" ],
    "Inez" => &[ "Agnes" ],
    "Ing" => &[ "Ning" ],
    "Ingrum" => &[ "Ningrum" ],
    "Ink" => &[ "Link" ],
    "Inta" => &[ "Sinta" ],
    "Ioannis" => &[ "Yanis" ],
    "Iott" => &[ "Elliott" ],
    "Iran" => &[ "Kiran" ],
    "Irani" => &[ "Khairani" ],
    "Isa" => &[ "Nisa" ],
    "Isaak" => &[ "Isaac" ],
    "Isabela" => &[ "Isabella" ],
    "Isham" => &[ "Hisham" ],
    "Isiah" => &[ "Isaiah" ],
    "Issac" => &[ "Isaac" ],
    "Ivan" => &[ "John" ],
    "Ivette" => &[ "Yvette" ],
    "Ivi" => &[ "Ivana" ],
    "Izabel" => &[ "Isabella" ],
    "Jaap" => &[ "Jacob" ],
    "Jack" => &[ "John", "Jacques" ],
    "Jacklin" => &[ "Jacqueline" ],
    "Jacklyn" => &[ "Jacqueline" ],
    "Jaclin" => &[ "Jacqueline" ],
    "Jaclyn" => &[ "Jacqueline" ],
    "Jaime" => &[ "Jamie", "James" ],
    "Jake" => &[ "Jacob" ],
    "Jamil" => &[ "Jameel" ],
    "Jan" => &[ "John" ],
    "Jaques" => &[ "John" ],
    "Jaroslaw" => &[ "Jarosław" ],
    "Jayce" => &[ "Jane", "Joyce" ],
    "Jayhugh" => &[ "Jehu" ],
    "Jazmin" => &[ "Jasmin" ],
    "Jazmine" => &[ "Jasmin" ],
    "Jazz" => &[ "Jazmin", "Jasmine" ],
    "Jean" => &[ "Genevieve", "Jane", "Joanna", "John" ],
    "Jeanette" => &[ "Jane" ],
    "Jeanne" => &[ "Jane" ],
    "Jeannie" => &[ "Jane" ],
    "Jedidiah" => &[ "Jedediah" ],
    "Jeffery" => &[ "Jeffrey" ],
    "Jem" => &[ "James" ],
    "Jemma" => &[ "Jemima" ],
    "Jena" => &[ "Jane" ],
    "Jenifer" => &[ "Jennifer" ],
    "Jenna" => &[ "Jane" ],
    "Jerimiah" => &[ "Jeremiah" ],
    "Jerry" => &[ "Geri" ],
    "Jill" => &[ "Julia" ],
    "Jim" => &[ "James" ],
    "Jitu" => &[ "Jitendra" ],
    "Jme" => &[ "Jamie" ],
    "Jock" => &[ "John" ],
    "Joey" => &[ "Joseph", "Josephine" ],
    "Johan" => &[ "John" ],
    "Johana" => &[ "Joanna", "Joan" ],
    "Johann" => &[ "John" ],
    "Johanna" => &[ "Joanna", "Joan" ],
    "Johannah" => &[ "Joanna", "Joan" ],
    "John" => &[ "Jonathan", "Jonathon" ],
    "Johnna" => &[ "Joan" ],
    "Jon" => &[ "John" ],
    "Jorg" => &[ "Joerg" ],
    "Jorge" => &[ "George" ],
    "Jorgen" => &[ "Jørgen" ],
    "Jose" => &[ "Joseph" ],
    "Josef" => &[ "Joseph" ],
    "Josefa" => &[ "Joseph" ],
    "Josefina" => &[ "Josephine" ],
    "Josepha" => &[ "Josephine" ],
    "Josephine" => &[ "Pheney" ],
    "Josh" => &[ "Josuah" ],
    "Joshua" => &[ "Josuah" ],
    "Josias" => &[ "Josiah" ],
    "Joss" => &[ "Jocelyn" ],
    "Josue" => &[ "Josuah" ],
    "Jr" => &[ "Junior" ],
    "Julian" => &[ "Julias" ],
    "Julien" => &[ "Julian" ],
    "Juliet" => &[ "Julia" ],
    "Juliette" => &[ "Julia" ],
    "Julius" => &[ "Julias" ],
    "Jurgen" => &[ "Juergen" ],
    "Justus" => &[ "Justin" ],
    "Kaitlin" => &[ "Caitlin" ],
    "Kaitlyn" => &[ "Caitlin" ],
    "Kami" => &[ "Kamran" ],
    "Karel" => &[ "Charles" ],
    "Karen" => &[ "Karonhappuck" ],
    "Karim" => &[ "Kareem" ],
    "Karina" => &[ "Karen" ],
    "Karissa" => &[ "Keri" ],
    "Karl" => &[ "Charles", "Carl" ],
    "Kasey" => &[ "Casey" ],
    "Kasia" => &[ "Katarzyna" ],
    "Kata" => &[ "Catalina" ],
    "Katarina" => &[ "Katherine", "Katherin" ],
    "Kate" => &[ "Catherin" ],
    "Katelyn" => &[ "Caitlin" ],
    "Katelynn" => &[ "Caitlin" ],
    "Katerina" => &[ "Catherine", "Katherine" ],
    "Katheryn" => &[ "Katherine", "Catherine" ],
    "Kathi" => &[ "Katherine", "Catherine" ],
    "Kathleen" => &[ "Katherine", "Catherine" ],
    "Kathrine" => &[ "Katherine", "Catherine" ],
    "Kathryn" => &[ "Katherine", "Catherine" ],
    "Kathy" => &[ "Catherine" ],
    "Kati" => &[ "Katalin" ],
    "Katlyn" => &[ "Caitlin" ],
    "Kaur" => &[ "Sidhu" ],
    "Kc" => &[ "Casey" ],
    "Keely" => &[ "Kelly" ],
    "Kendall" => &[ "Kenneth" ],
    "Kendrick" => &[ "Kenneth" ],
    "Kenj" => &[ "Kendra" ],
    "Kenny" => &[ "Kehinde" ],
    "Kent" => &[ "Kenneth" ],
    "Kerri" => &[ "Keri" ],
    "Kerry" => &[ "Keri" ],
    "Kester" => &[ "Christopher" ],
    "Kez" => &[ "Kerry" ],
    "Keziah" => &[ "Kesiah" ],
    "Khushi" => &[ "Khushboo" ],
    "Kiara" => &[ "Keri" ],
    "Kid" => &[ "Keziah" ],
    "Kit" => &[ "Christian", "Christopher", "Katherine" ],
    "Kizza" => &[ "Keziah" ],
    "Knowell" => &[ "Noel" ],
    "Kostas" => &[ "Konstantinos" ],
    "Kris" => &[ "Christiana", "Christine" ],
    "Krista" => &[ "Christiana", "Christine" ],
    "Kristi" => &[ "Christiana", "Christine" ],
    "Kristian" => &[ "Christiana", "Christine" ],
    "Kristin" => &[ "Christiana", "Christine" ],
    "Kristina" => &[ "Christiana", "Christine" ],
    "Kristine" => &[ "Christiana", "Christine" ],
    "Krystal" => &[ "Crystal" ],
    "Kuba" => &[ "Jakub" ],
    "Kurt" => &[ "Curtis" ],
    "Kurtis" => &[ "Curtis" ],
    "Ky" => &[ "Hezekiah" ],
    "Kym" => &[ "Kimberly" ],
    "Laci" => &[ "Laszlo" ],
    "Lalo" => &[ "Eduardo" ],
    "Lanna" => &[ "Eleanor" ],
    "Lara" => &[ "Laura" ],
    "Lark" => &[ "Clark" ],
    "Larry" => &[ "Olanrewaju" ],
    "Lars" => &[ "Lawrence" ],
    "Latha" => &[ "Hemal" ],
    "Latisha" => &[ "Latasha" ],
    "Laura" => &[ "Laurinda", "Loretta", "Lauri" ],
    "Laurence" => &[ "Lawrence" ],
    "Lazar" => &[ "Eleazer" ],
    "Lb" => &[ "Littleberry" ],
    "Leafa" => &[ "Relief" ],
    "Lecta" => &[ "Electra" ],
    "Lee" => &[ "Elias", "Shirley" ],
    "Leet" => &[ "Philetus" ],
    "Left" => &[ "Eliphalet", "Lafayette" ],
    "Leja" => &[ "Alejandra" ],
    "Len" => &[ "Leonard" ],
    "Lena" => &[ "Adaline", "Aileen", "Angela", "Arlene", "Caroline", "Darlene", "Evaline", "Madeline", "Magdelina", "Selina", "Ellen" ],
    "Lenhart" => &[ "Leonard" ],
    "Lenora" => &[ "Ellen" ],
    "Leo" => &[ "Leandro" ],
    "Leon" => &[ "Lionel" ],
    "Leonora" => &[ "Eleanor" ],
    "Leslie" => &[ "Lesley" ],
    "Lester" => &[ "Leslie" ],
    "Leticia" => &[ "Leta" ],
    "Lettice" => &[ "Letitia" ],
    "Leve" => &[ "Aleva" ],
    "Lexa" => &[ "Alexandria" ],
    "Lexi" => &[ "Alexis" ],
    "Li" => &[ "Lee" ],
    "Lib" => &[ "Elizabeth" ],
    "Liba" => &[ "Libuse" ],
    "Lidia" => &[ "Linda" ],
    "Lig" => &[ "Elijah" ],
    "Lige" => &[ "Elijah" ],
    "Lil" => &[ "Delilah" ],
    "Lila" => &[ "Delilah" ],
    "Lillah" => &[ "Lillian" ],
    "Lina" => &[ "Emeline", "Linda" ],
    "Lineau" => &[ "Leonard" ],
    "Linette" => &[ "Linda" ],
    "Link" => &[ "Lincoln" ],
    "Linsey" => &[ "Lindsey" ],
    "Linz" => &[ "Lindsey" ],
    "Lisa" => &[ "Elizabeth", "Melissa" ],
    "Lise" => &[ "Elizabeth" ],
    "Lisette" => &[ "Elizabeth" ],
    "Lish" => &[ "Alice" ],
    "Lissa" => &[ "Larissa" ],
    "Liz" => &[ "Elizabeth" ],
    "Liza" => &[ "Adelaide", "Elizabeth" ],
    "Lloyd" => &[ "Floyd" ],
    "Loenore" => &[ "Leonora" ],
    "Lois" => &[ "Heloise", "Louise" ],
    "Lola" => &[ "Delores" ],
    "Loli" => &[ "Dolores" ],
    "Lon" => &[ "Alonzo", "Lawrence" ],
    "Lonson" => &[ "Alanson" ],
    "Lora" => &[ "Laura" ],
    "Lorena" => &[ "Lori" ],
    "Loretta" => &[ "Lori" ],
    "Lorinda" => &[ "Laurinda" ],
    "Lorne" => &[ "Lawrence" ],
    "Lorraine" => &[ "Lori" ],
    "Los" => &[ "Angeles" ],
    "Lotta" => &[ "Charlotte" ],
    "Lou" => &[ "Luann", "Lucille", "Lucinda", "Lewis", "Luisa", "Luella" ],
    "Louann" => &[ "Luann" ],
    "Louanne" => &[ "Luann" ],
    "Louie" => &[ "Lewis" ],
    "Louis" => &[ "Lewis" ],
    "Lousie" => &[ "Eliza", "Louise", "Louisa", "Lois", "Louetta", "Elouise", "Eloise", "Heloise" ],
    "Louvina" => &[ "Lavinia" ],
    "Louvinia" => &[ "Lavinia" ],
    "Loyd" => &[ "Lloyd" ],
    "Lr" => &[ "Leroy" ],
    "Luana" => &[ "Luanna" ],
    "Lucas" => &[ "Lucias" ],
    "Lucien" => &[ "Lucian" ],
    "Lucinda" => &[ "Cynthia" ],
    "Luis" => &[ "Lewis" ],
    "Luke" => &[ "Lucias", "Luthor", "Lucas" ],
    "Lula" => &[ "Luella" ],
    "Lulu" => &[ "Luann", "Luciana", "Lou" ],
    "Lum" => &[ "Columbus" ],
    "Lupita" => &[ "Guadalupe" ],
    "Luz" => &[ "Lou" ],
    "Lyn" => &[ "Belinda" ],
    "Lynda" => &[ "Linda" ],
    "Lynette" => &[ "Linda" ],
    "Lynn" => &[ "Caroline", "Celinda", "Linda", "Lyndon" ],
    "Lynne" => &[ "Belinda", "Melinda" ],
    "Lynsey" => &[ "Lindsey" ],
    "Mabel" => &[ "Mehitabel" ],
    "Mac" => &[ "Malcolm" ],
    "Maciek" => &[ "Maciej" ],
    "Madeleine" => &[ "Madeline" ],
    "Madelyn" => &[ "Madeline" ],
    "Madge" => &[ "Madeline", "Magdelina", "Margaret" ],
    "Magda" => &[ "Madeline", "Magdelina" ],
    "Magdalen" => &[ "Magdelina" ],
    "Mahdi" => &[ "Mehdi" ],
    "Mahi" => &[ "Mahesh" ],
    "Maida" => &[ "Madeline", "Magdelina", "Magdalena" ],
    "Maira" => &[ "Mary" ],
    "Maka" => &[ "Macarena" ],
    "Malgorzata" => &[ "Małgorzata" ],
    "Malik" => &[ "Malick" ],
    "Malina" => &[ "Malinda" ],
    "Malu" => &[ "Luiza" ],
    "Manh" => &[ "Hung" ],
    "Manu" => &[ "Manoj", "Emmanuel", "Emanuela", "Emanuele" ],
    "Manuel" => &[ "Manolo" ],
    "Mara" => &[ "Margaret" ],
    "Maranda" => &[ "Margaret" ],
    "Marc" => &[ "Mark" ],
    "Marcella" => &[ "Marci" ],
    "Marco" => &[ "Marko" ],
    "Marcos" => &[ "Markos" ],
    "Marcus" => &[ "Mark" ],
    "Margaret" => &[ "Gretchen" ],
    "Margauerite" => &[ "Margarita" ],
    "Margo" => &[ "Margaret" ],
    "Margot" => &[ "Margaret" ],
    "Mari" => &[ "Mary" ],
    "Mariam" => &[ "Mary" ],
    "Marian" => &[ "Marion", "Mary" ],
    "Mariana" => &[ "Mary" ],
    "Marianna" => &[ "Maryanne", "Mary" ],
    "Marianne" => &[ "Maryanne", "Mary" ],
    "Marie" => &[ "Mary" ],
    "Marina" => &[ "Mary" ],
    "Maris" => &[ "Demerias" ],
    "Marisol" => &[ "Marysol" ],
    "Marissa" => &[ "Mary" ],
    "Marjorie" => &[ "Mary" ],
    "Mark" => &[ "Marcus", "Marco" ],
    "Marlene" => &[ "Marla" ],
    "Marx" => &[ "Marques" ],
    "Maryam" => &[ "Mariam" ],
    "Mat" => &[ "Martha" ],
    "Mathew" => &[ "Matthew" ],
    "Mathias" => &[ "Matthew" ],
    "Mathilda" => &[ "Matilda" ],
    "Matias" => &[ "Mathias" ],
    "Matthias" => &[ "Matthew" ],
    "Maud" => &[ "Madeline", "Matilda" ],
    "Maura" => &[ "Maureen" ],
    "Mauro" => &[ "Mauricio" ],
    "Max" => &[ "Massimo" ],
    "Mayor" => &[ "Mayowa" ],
    "Meagan" => &[ "Megan" ],
    "Meaghan" => &[ "Megan" ],
    "Medora" => &[ "Dorothy" ],
    "Mees" => &[ "Bartholomew" ],
    "Meg" => &[ "Margaret", "Meagan" ],
    "Megan" => &[ "Margaret", "Meggie" ],
    "Meghan" => &[ "Megan" ],
    "Mehdi" => &[ "Mahdi" ],
    "Mehetabel" => &[ "Mehitabel" ],
    "Mehetable" => &[ "Mehitabel" ],
    "Mehitable" => &[ "Mehitabel" ],
    "Mel" => &[ "Amelia" ],
    "Melina" => &[ "Melinda" ],
    "Melissa" => &[ "Milicent" ],
    "Mell" => &[ "Mildred" ],
    "Melo" => &[ "Mello" ],
    "Memo" => &[ "Mehmet", "Guillermo" ],
    "Merlyn" => &[ "Merlin" ],
    "Mero" => &[ "Marwa" ],
    "Mert" => &[ "Myrtle" ],
    "Merv" => &[ "Marvin" ],
    "Mervyn" => &[ "Marvin" ],
    "Meta" => &[ "Margaret" ],
    "Metta" => &[ "Margaret" ],
    "Meus" => &[ "Bartholomew" ],
    "Mia" => &[ "Marianna" ],
    "Michaela" => &[ "Michelle" ],
    "Michal" => &[ "Michał" ],
    "Micheal" => &[ "Michael" ],
    "Mick" => &[ "Michael" ],
    "Midge" => &[ "Margaret" ],
    "Miera" => &[ "Amira" ],
    "Mike" => &[ "Michael", "Miguel" ],
    "Mikele" => &[ "Michele" ],
    "Miki" => &[ "Michela" ],
    "Mikolaj" => &[ "Mikołaj" ],
    "Milla" => &[ "Camila" ],
    "Mina" => &[ "Mindwell", "Minerva" ],
    "Minerva" => &[ "Manerva" ],
    "Mira" => &[ "Mary" ],
    "Miranda" => &[ "Mary" ],
    "Miriam" => &[ "Mirian", "Mairim", "Mary" ],
    "Misra" => &[ "Mishra" ],
    "Mitchel" => &[ "Mitchell" ],
    "Mock" => &[ "Democrates" ],
    "Mohamad" => &[ "Mohammed" ],
    "Mohamed" => &[ "Mohammed" ],
    "Mohammad" => &[ "Mohammed" ],
    "Mohd" => &[ "Mohammed" ],
    "Moll" => &[ "Mary" ],
    "Monique" => &[ "Monica" ],
    "Montesque" => &[ "Montgomery" ],
    "Morris" => &[ "Maurice" ],
    "Moses" => &[ "Amos" ],
    "Moss" => &[ "Moses" ],
    "Mostafa" => &[ "Moustafa" ],
    "Muhammad" => &[ "Mohammed" ],
    "Muhammed" => &[ "Mohammed" ],
    "Murat" => &[ "Murad" ],
    "Myles" => &[ "Miles" ],
    "Myra" => &[ "Almira", "Elmira", "Amirah" ],
    "Nace" => &[ "Ignatius" ],
    "Nacho" => &[ "Ignacio" ],
    "Nada" => &[ "Nadine" ],
    "Nadia" => &[ "Nadezhda", "Nadya" ],
    "Naldo" => &[ "Reginald", "Ronald" ],
    "Nan" => &[ "Anna", "Hannah" ],
    "Nana" => &[ "Anna" ],
    "Naqvi" => &[ "Haider" ],
    "Naser" => &[ "Nasser" ],
    "Nate" => &[ "Ignatius" ],
    "Nati" => &[ "Natalia" ],
    "Neal" => &[ "Cornelius", "Neil" ],
    "Ned" => &[ "Edmund", "Edward", "Edwin" ],
    "Neil" => &[ "Cornelius" ],
    "Nell" => &[ "Eleanor", "Helena", "Cornelia" ],
    "Nelle" => &[ "Eleanor", "Helena", "Cornelia" ],
    "Nessa" => &[ "Agnes" ],
    "Net" => &[ "Antoinette" ],
    "Neto" => &[ "Netto", "Ernesto" ],
    "Netta" => &[ "Antoinette" ],
    "Neva" => &[ "Genevieve" ],
    "Nha" => &[ "Bruna" ],
    "Nib" => &[ "Isabella" ],
    "Nichole" => &[ "Nicole" ],
    "Nick" => &[ "Dominic", "Nicholas" ],
    "Nickolas" => &[ "Nicholas" ],
    "Nicodemus" => &[ "Nicholas" ],
    "Nicolas" => &[ "Nicholas" ],
    "Nicolay" => &[ "Nikolai" ],
    "Niel" => &[ "Cornelius" ],
    "Night" => &[ "Knight" ],
    "Niki" => &[ "Nikolett" ],
    "Nikki" => &[ "Nicola", "Nicole", "Nikita" ],
    "Niko" => &[ "Nicolas" ],
    "Nikos" => &[ "Nikolaos" ],
    "Nina" => &[ "Enedina" ],
    "Noemi" => &[ "Naomi" ],
    "Nomi" => &[ "Noman" ],
    "Nora" => &[ "Eleanor" ],
    "Norah" => &[ "Honora" ],
    "Norma" => &[ "Nora" ],
    "Nowell" => &[ "Noel" ],
    "Nura" => &[ "Amalina" ],
    "Obed" => &[ "Obadiah" ],
    "Odo" => &[ "Odell" ],
    "Ofa" => &[ "Mustofa", "Mostofa" ],
    "Ola" => &[ "Aleksandra" ],
    "Olga" => &[ "Olia" ],
    "Oliver" => &[ "Oliveira" ],
    "Olph" => &[ "Rudolphus" ],
    "Ondra" => &[ "Ondrej" ],
    "Ono" => &[ "Tono", "Margono", "Martono", "Hartono" ],
    "Ora" => &[ "Aurelia", "Aurilla" ],
    "Ore" => &[ "Moore" ],
    "Orilla" => &[ "Aurelia", "Aurilla" ],
    "Orlando" => &[ "Roland" ],
    "Orphelia" => &[ "Ophelia" ],
    "Oscar" => &[ "Oskar" ],
    "Osman" => &[ "Othman" ],
    "Oswald" => &[ "Waldo" ],
    "Otis" => &[ "Othello" ],
    "Pancho" => &[ "Francisco" ],
    "Panos" => &[ "Panagiotis" ],
    "Parmelia" => &[ "Amelia" ],
    "Pate" => &[ "Peter" ],
    "Pati" => &[ "Patrycja" ],
    "Pato" => &[ "Patricio" ],
    "Pauli" => &[ "Paula" ],
    "Pauline" => &[ "Paula" ],
    "Pawel" => &[ "Paweł" ],
    "Peg" => &[ "Margaret" ],
    "Permelia" => &[ "Amelia" ],
    "Pheobe" => &[ "Tryphena" ],
    "Pherbia" => &[ "Pharaba" ],
    "Pheriba" => &[ "Pharaba" ],
    "Phidelia" => &[ "Bedelia", "Fidelia" ],
    "Phililpa" => &[ "Philipina" ],
    "Phillip" => &[ "Philip" ],
    "Phineas" => &[ "Alphinias" ],
    "Phoebe" => &[ "Philipina", "Phebe" ],
    "Pinar" => &[ "Pınar" ],
    "Pino" => &[ "Giuseppe" ],
    "Pip" => &[ "Philip" ],
    "Pipe" => &[ "Felipe" ],
    "Ples" => &[ "Pleasant" ],
    "Poe" => &[ "Putri" ],
    "Pola" => &[ "Paola" ],
    "Polo" => &[ "Leopoldo" ],
    "Poncho" => &[ "Alfonso" ],
    "Puss" => &[ "Philadelphia", "Prudence" ],
    "Quil" => &[ "Aquilla" ],
    "Quinn" => &[ "Quince" ],
    "Quint" => &[ "Quince" ],
    "Rachael" => &[ "Rachel" ],
    "Racheal" => &[ "Rachel" ],
    "Raech" => &[ "Rachel" ],
    "Rafal" => &[ "Rafał" ],
    "Raff" => &[ "Raphael" ],
    "Rahim" => &[ "Raheem" ],
    "Rajiv" => &[ "Rajeev" ],
    "Raju" => &[ "Rajendra" ],
    "Ralf" => &[ "Ralph" ],
    "Ralph" => &[ "Raphael" ],
    "Ramadan" => &[ "Ramadhan" ],
    "Rana" => &[ "Lorraine" ],
    "Randall" => &[ "Randolph" ],
    "Ravi" => &[ "Ramakrishna" ],
    "Ray" => &[ "Regina" ],
    "Reba" => &[ "Rebecca" ],
    "Rebeca" => &[ "Rebecca" ],
    "Rebecka" => &[ "Rebecca" ],
    "Rebekah" => &[ "Rebecca" ],
    "Reece" => &[ "Rees" ],
    "Refina" => &[ "Rufina" ],
    "Regis" => &[ "Reginaldo" ],
    "Rena" => &[ "Irene", "Maureen", "Sabrina", "Regina" ],
    "Renae" => &[ "Rene" ],
    "Renaldo" => &[ "Reginald" ],
    "Retta" => &[ "Henrietta", "Chiara" ],
    "Reynold" => &[ "Reginald" ],
    "Rhoda" => &[ "Rhodella" ],
    "Ricardo" => &[ "Richard" ],
    "Rich" => &[ "Alderick" ],
    "Rick" => &[ "Eric", "Richard" ],
    "Ricka" => &[ "Frederica" ],
    "Rico" => &[ "Ricardo" ],
    "Riki" => &[ "Riccardo" ],
    "Rita" => &[ "Margaret" ],
    "Rod" => &[ "Roger" ],
    "Rodger" => &[ "Roger" ],
    "Roland" => &[ "Orlando" ],
    "Rolf" => &[ "Rudolphus" ],
    "Rollo" => &[ "Roland", "Rudolphus" ],
    "Ron" => &[ "Veronica" ],
    "Ronna" => &[ "Veronica" ],
    "Rosabella" => &[ "Isabella" ],
    "Rosable" => &[ "Rosabella" ],
    "Rosalinda" => &[ "Rosalyn" ],
    "Roso" => &[ "Osorio" ],
    "Rowland" => &[ "Roland" ],
    "Rox" => &[ "Roseanne" ],
    "Roxane" => &[ "Roseanne" ],
    "Roxanna" => &[ "Roseanne" ],
    "Roxanne" => &[ "Roseanne" ],
    "Roz" => &[ "Rosabella", "Rosalyn", "Roseanne" ],
    "Rube" => &[ "Reuben" ],
    "Ruben" => &[ "Reuben" ],
    "Rupert" => &[ "Robert" ],
    "Rye" => &[ "Zachariah" ],
    "Sabe" => &[ "Isabella" ],
    "Sabra" => &[ "Isabella" ],
    "Sabrina" => &[ "Sabina" ],
    "Sadiq" => &[ "Abubakar" ],
    "Sahara" => &[ "Sarah" ],
    "Sal" => &[ "Solomon" ],
    "Sale" => &[ "Halo" ],
    "Salim" => &[ "Saleem" ],
    "Salina" => &[ "Selina" ],
    "Salmon" => &[ "Solomon" ],
    "Samson" => &[ "Sampson" ],
    "Sandra" => &[ "Alexandria" ],
    "Sangi" => &[ "Sangeetha" ],
    "Sanz" => &[ "Sanchez" ],
    "Sariah" => &[ "Sarah" ],
    "Sarn" => &[ "Arnold" ],
    "Sasha" => &[ "Alexander", "Alexandria" ],
    "Saul" => &[ "Solomon" ],
    "Sean" => &[ "Shaun", "Shawn" ],
    "Selena" => &[ "Selina" ],
    "Sene" => &[ "Asenath" ],
    "Serena" => &[ "Sabrina" ],
    "Serene" => &[ "Cyrenius" ],
    "Seymore" => &[ "Seymour" ],
    "Shaik" => &[ "Basha" ],
    "Shana" => &[ "Shannon" ],
    "Shane" => &[ "Shaun" ],
    "Shanna" => &[ "Shannon" ],
    "Sharyn" => &[ "Sharon" ],
    "Shaun" => &[ "Shawn" ],
    "Shauna" => &[ "Shawna" ],
    "Shawn" => &[ "Shaun" ],
    "Shayla" => &[ "Sheila" ],
    "Shayne" => &[ "Shaun", "Shane" ],
    "Shelton" => &[ "Sheldon" ],
    "Sher" => &[ "Sharon" ],
    "Sheron" => &[ "Sharon" ],
    "Sheryl" => &[ "Sharon" ],
    "Sheryn" => &[ "Sharon" ],
    "Si" => &[ "Cyrus", "Josiah", "Sylvester" ],
    "Sibbell" => &[ "Sibbilla" ],
    "Sibyl" => &[ "Sibbilla" ],
    "Sigmund" => &[ "Sigismund" ],
    "Silla" => &[ "Priscilla" ],
    "Silver" => &[ "Sylvester" ],
    "Silvester" => &[ "Sylvester" ],
    "Silvia" => &[ "Sylvia" ],
    "Simeon" => &[ "Simon" ],
    "Simon" => &[ "Simeon" ],
    "Sion" => &[ "Simeon" ],
    "Sis" => &[ "Frances" ],
    "Siti" => &[ "Fatimah", "City" ],
    "Siva" => &[ "Shiva" ],
    "Smit" => &[ "Mitchell" ],
    "Sofia" => &[ "Sophia" ],
    "Sonja" => &[ "Sonia" ],
    "Sonya" => &[ "Sonia" ],
    "Sophia" => &[ "Sophronia" ],
    "Soren" => &[ "Søren" ],
    "Spar" => &[ "Parker" ],
    "Srah" => &[ "Rahman" ],
    "Stefan" => &[ "Stephen" ],
    "Stefanie" => &[ "Stephani" ],
    "Stephan" => &[ "Stephen" ],
    "Steve" => &[ "Stephen" ],
    "Steven" => &[ "Stephen" ],
    "Stewart" => &[ "Stuart" ],
    "Summer" => &[ "Sumner" ],
    "Susana" => &[ "Susannah" ],
    "Susi" => &[ "Susan", "Susannah" ],
    "Suzanna" => &[ "Susan" ],
    "Suzanne" => &[ "Susannah", "Susan" ],
    "Suzette" => &[ "Susan" ],
    "Swene" => &[ "Cyrenius" ],
    "Syah" => &[ "Firman" ],
    "Sybrina" => &[ "Sabrina" ],
    "Syd" => &[ "Sidney" ],
    "Sydney" => &[ "Sidney" ],
    "Sylvanus" => &[ "Sylvester" ],
    "Tabatha" => &[ "Tabitha" ],
    "Tad" => &[ "Thaddeus", "Theodore" ],
    "Tamarra" => &[ "Tamara" ],
    "Tammy" => &[ "Tami" ],
    "Tamzine" => &[ "Thomasine" ],
    "Tanya" => &[ "Tania" ],
    "Tata" => &[ "Tatiana" ],
    "Tave" => &[ "Octavia" ],
    "Ted" => &[ "Edmund", "Edward", "Theodore" ],
    "Temera" => &[ "Tamara" ],
    "Terence" => &[ "Terrence" ],
    "Teresa" => &[ "Theresa" ],
    "Terrance" => &[ "Terrence", "Terence" ],
    "Terrence" => &[ "Terence" ],
    "Terry" => &[ "Teri" ],
    "Tess" => &[ "Esther", "Theresa" ],
    "Tessa" => &[ "Theresa" ],
    "Than" => &[ "Nathaniel" ],
    "Theodora" => &[ "Theodosia" ],
    "Theodore" => &[ "Theodrick" ],
    "Thias" => &[ "Matthew" ],
    "Thirsa" => &[ "Theresa" ],
    "Thomas" => &[ "Thomasin" ],
    "Thomasa" => &[ "Thomasine" ],
    "Thriza" => &[ "Theresa" ],
    "Thursa" => &[ "Theresa" ],
    "Tiah" => &[ "Azariah" ],
    "Tick" => &[ "Felicity" ],
    "Tierra" => &[ "Tiara" ],
    "Tiffani" => &[ "Tiffany" ],
    "Timi" => &[ "Timea" ],
    "Tina" => &[ "Augusta", "Christiana", "Ernestine" ],
    "Tish" => &[ "Letitia", "Patricia" ],
    "Tom" => &[ "Thomas" ],
    "Tomas" => &[ "Thomas" ],
    "Tomek" => &[ "Tomasz" ],
    "Tomi" => &[ "Tamas", "Tomas" ],
    "Toni" => &[ "Antonia" ],
    "Trina" => &[ "Katherine" ],
    "Trish" => &[ "Beatrice", "Patricia" ],
    "Trisha" => &[ "Beatrice", "Patricia" ],
    "Trix" => &[ "Beatrice" ],
    "Tung" => &[ "Nguyen" ],
    "Uddin" => &[ "Khairuddin", "Amiruddin", "Alauddin" ],
    "Ugo" => &[ "Hugo" ],
    "Ulana" => &[ "Maulana" ],
    "Ullah" => &[ "Sanaullah", "Khairullah", "Amirullah", "Amrullah" ],
    "Uma" => &[ "Maheswari" ],
    "Ung" => &[ "Leung", "Hanung" ],
    "Ur" => &[ "Rehman" ],
    "Ura" => &[ "Mastura" ],
    "Uran" => &[ "Duran" ],
    "Uri" => &[ "Oriol", "Mashuri", "Kasturi" ],
    "Utz" => &[ "Ionut" ],
    "Uyen" => &[ "Huyen" ],
    "Valarie" => &[ "Valerie" ],
    "Valeda" => &[ "Valentina" ],
    "Valeria" => &[ "Valerie" ],
    "Vanna" => &[ "Vanessa" ],
    "Vera" => &[ "Veronica" ],
    "Verna" => &[ "Laverne" ],
    "Vest" => &[ "Sylvester" ],
    "Vet" => &[ "Sylvester" ],
    "Vick" => &[ "Victor" ],
    "Vina" => &[ "Lavinia" ],
    "Viola" => &[ "Violet" ],
    "Vivien" => &[ "Vivian" ],
    "Vivienne" => &[ "Vivian" ],
    "Volodia" => &[ "Vladimir" ],
    "Waldo" => &[ "Oswald" ],
    "Wat" => &[ "Walter" ],
    "Webb" => &[ "Webster" ],
    "Wenefred" => &[ "Winifred" ],
    "Westley" => &[ "Wesley" ],
    "Wib" => &[ "Wilber" ],
    "Wilber" => &[ "Gilbert" ],
    "Wilbur" => &[ "Wilber" ],
    "Wiley" => &[ "William" ],
    "Wilhelm" => &[ "William" ],
    "Will" => &[ "Wilber", "Wilfred", "Wilhelm" ],
    "Willa" => &[ "Wilma", "William" ],
    "Willis" => &[ "William" ],
    "Wilma" => &[ "Wilhelmina" ],
    "Winnet" => &[ "Winifred" ],
    "Wyncha" => &[ "Lavinia" ],
    "Xan" => &[ "Alexandria", "Alexandre" ],
    "Xena" => &[ "Christiana" ],
    "Xina" => &[ "Christiana" ],
    "Xu" => &[ "Hsu" ],
    "Yasmin" => &[ "Jasmin" ],
    "Yolonda" => &[ "Yolanda" ],
    "Zacharias" => &[ "Zachariah" ],
    "Zack" => &[ "Zach" ],
    "Zadock" => &[ "Melchizedek" ],
    "Zay" => &[ "Isaiah" ],
    "Zed" => &[ "Zadock" ],
    "Zeke" => &[ "Ezekiel", "Isaac", "Zachariah" ],
    "Zella" => &[ "Zelphia" ],
    "Zeph" => &[ "Zepaniah" ],
    "Zhang" => &[ "Cheung" ],
    "Zhou" => &[ "Chou", "Chow" ],
    "Zubiah" => &[ "Azubah" ],
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
