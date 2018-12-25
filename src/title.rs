use super::namepart::{Category, NamePart};
use super::suffix;
use phf;
use std::cmp;

static TWO_CHAR_TITLES: [&'static str; 4] = ["mr", "ms", "sr", "dr"];

static PREFIX_TITLE_PARTS: phf::Set<&'static str> = phf_set! {
    "Aunt",
    "Auntie",
    "Attaché",
    "Dame",
    "Marchioness",
    "Marquess",
    "Marquis",
    "Marquise",
    "King",
    "King'S",
    "Queen",
    "Queen'S",
    "Abbess",
    "Abbot",
    "Academic",
    "Acolyte",
    "Adept",
    "Adjutant",
    "Adm",
    "Admiral",
    "Advocate",
    "Akhoond",
    "Air",
    "Ald",
    "Alderman",
    "Almoner",
    "Ambassador",
    "Amn",
    "Analytics",
    "Appellate",
    "Apprentice",
    "Arbitrator",
    "Archbishop",
    "Archdeacon",
    "Archdruid",
    "Archduchess",
    "Archduke",
    "Arhat",
    "Assistant",
    "Assoc",
    "Associate",
    "Asst",
    "Attache",
    "Attorney",
    "Ayatollah",
    "Baba",
    "Bailiff",
    "Banner",
    "Bard",
    "Baron",
    "Barrister",
    "Bearer",
    "Bench",
    "Bgen",
    "Bishop",
    "Blessed",
    "Bodhisattva",
    "Brig",
    "Brigadier",
    "Briggen",
    "Brother",
    "Buddha",
    "Burgess",
    "Business",
    "Bwana",
    "Canon",
    "Capt",
    "Captain",
    "Cardinal",
    "Chargé",
    "Catholicos",
    "Ccmsgt",
    "Cdr",
    "Ceo",
    "Cfo",
    "Chair",
    "Chairs",
    "Chancellor",
    "Chaplain",
    "Chief",
    "Chieftain",
    "Civil",
    "Clerk",
    "Cmd",
    "Cmdr",
    "Cmsaf",
    "Cmsgt",
    "Co-Chair",
    "Co-Chairs",
    "Coach",
    "Col",
    "Colonel",
    "Commander",
    "Commander-In-Chief",
    "Commodore",
    "Comptroller",
    "Controller",
    "Corporal",
    "Corporate",
    "Councillor",
    "Count",
    "Countess",
    "Courtier",
    "Cpl",
    "Cpo",
    "Cpt",
    "Credit",
    "Criminal",
    "Csm",
    "Curator",
    "Customs",
    "Cwo",
    "D'Affaires",
    "Deacon",
    "Delegate",
    "Deputy",
    "Designated",
    "Det",
    "Dir",
    "Director",
    "Discovery",
    "District",
    "Division",
    "Docent",
    "Docket",
    "Doctor",
    "Doyen",
    "Dpty",
    "Druid",
    "Duke",
    "Dutchess",
    "Edmi",
    "Edohen",
    "Effendi",
    "Ekegbian",
    "Elder",
    "Elerunwon",
    "Emperor",
    "Empress",
    "Ens",
    "Envoy",
    "Exec",
    "Executive",
    "Fadm",
    "Family",
    "Father",
    "Federal",
    "Field",
    "Financial",
    "First",
    "Flag",
    "Flying",
    "Flight",
    "Flt",
    "Foreign",
    "Forester",
    "Frau",
    "Friar",
    "Gen",
    "General",
    "Generalissimo",
    "Gentiluomo",
    "Giani",
    "Goodman",
    "Goodwife",
    "Governor",
    "Grand",
    "Group",
    "Guru",
    "Gyani",
    "Gysgt",
    "Hajji",
    "Headman",
    "Her",
    "Herr",
    "Hereditary",
    "High",
    "His",
    "Hon",
    "Honorable",
    "Honourable",
    "Imam",
    "Information",
    "Insp",
    "Intelligence",
    "Intendant",
    "Journeyman",
    "Judge",
    "Judicial",
    "Justice",
    "Junior",
    "Kingdom",
    "Knowledge",
    "Lady",
    "Lama",
    "Lamido",
    "Law",
    "Lcdr",
    "Lcpl",
    "Leader",
    "Lieutenant",
    "Lord",
    "Leut",
    "Lieut",
    "Ltc",
    "Ltcol",
    "Ltg",
    "Ltgen",
    "Ltjg",
    "Madam",
    "Madame",
    "Mag",
    "Mag-Judge",
    "Mag/Judge",
    "Magistrate",
    "Magistrate-Judge",
    "Maharajah",
    "Maharani",
    "Mahdi",
    "Maid",
    "Maj",
    "Majesty",
    "Majgen",
    "Major",
    "Manager",
    "Marcher",
    "Marketing",
    "Marshal",
    "Master",
    "Matriarch",
    "Matron",
    "Mayor",
    "Mcpo",
    "Mcpoc",
    "Mcpon",
    "Member",
    "Metropolitan",
    "Mgr",
    "Mgysgt",
    "Minister",
    "Miss",
    "Misses",
    "Mister",
    "Mme",
    "Monsignor",
    "Most",
    "Mother",
    "Mpco-Cg",
    "Mrs",
    "Msg",
    "Msgr",
    "Msgt",
    "Mufti",
    "Mullah",
    "Municipal",
    "Murshid",
    "Nanny",
    "National",
    "Nurse",
    "Officer",
    "Operating",
    "Pastor",
    "Patriarch",
    "Petty",
    "Pfc",
    "Pharaoh",
    "Pilot",
    "Pir",
    "Police",
    "Political",
    "Pope",
    "Prefect",
    "Prelate",
    "Premier",
    "Pres",
    "Presbyter",
    "President",
    "Presiding",
    "Priest",
    "Priestess",
    "Primate",
    "Prime",
    "Prin",
    "Prince",
    "Princess",
    "Principal",
    "Prior",
    "Private",
    "Pro",
    "Prof",
    "Professor",
    "Provost",
    "Pslc",
    "Pte",
    "Pursuivant",
    "Pvt",
    "Rabbi",
    "Radm",
    "Rangatira",
    "Ranger",
    "Rdml",
    "Rear",
    "Rebbe",
    "Registrar",
    "Rep",
    "Representative",
    "Resident",
    "Rev",
    "Revenue",
    "Reverend",
    "Reverand",
    "Revd",
    "Right",
    "Risk",
    "Royal",
    "Saint",
    "Sargent",
    "Sargeant",
    "Saoshyant",
    "Scpo",
    "Secretary",
    "Security",
    "Seigneur",
    "Senator",
    "Senior",
    "Senior-Judge",
    "Sergeant",
    "Servant",
    "Sfc",
    "Sgm",
    "Sgt",
    "Sgtmaj",
    "Sgtmajmc",
    "Shehu",
    "Sheikh",
    "Sheriff",
    "Siddha",
    "Sir",
    "Sister",
    "Sma",
    "Smsgt",
    "Solicitor",
    "Spc",
    "Speaker",
    "Special",
    "Sra",
    "Ssg",
    "Ssgt",
    "Staff",
    "State",
    "States",
    "Strategy",
    "Subaltern",
    "Subedar",
    "Sultan",
    "Sultana",
    "Superior",
    "Supreme",
    "Surgeon",
    "Swordbearer",
    "Sysselmann",
    "Tax",
    "Technical",
    "Timi",
    "Tirthankar",
    "Treasurer",
    "Tsar",
    "Tsarina",
    "Tsgt",
    "Uncle",
    "United",
    "Vadm",
    "Vardapet",
    "Venerable",
    "Verderer",
    "Very",
    "Vicar",
    "Vice",
    "Viscount",
    "Vizier",
    "Warden",
    "Warrant",
    "Wing",
    "Woodman",
    "And",
    "The",
    "Und",
};

static POSTFIX_TITLES: phf::Set<&'static str> = phf_set! {
    "Esq",
    "Esquire",
    "Attorney-at-law",
    "Et",
    "Al",
};

#[allow(clippy::if_same_then_else)]
fn might_be_title_part(word: &NamePart) -> bool {
    if word.chars < 3 {
        // Allow any word with 1 or 2 characters as part of a title (but see below)
        true
    } else {
        match &word.category {
            Category::Name(ref namecased) => {
                let namecased: &str = &*namecased;
                PREFIX_TITLE_PARTS.contains(namecased) || namecased.chars().any(char::is_numeric)
            }
            _ => true,
        }
    }
}

fn might_be_last_title_part(word: &NamePart) -> bool {
    // Don't allow 1 or 2-character words as the whole or final piece of
    // a title, except a set of very-common two-character title abbreviations,
    // because otherwise we are more likely dealing with initials
    if word.chars < 3 || word.word.chars().filter(|c| c.is_alphanumeric()).count() < 2 {
        word.chars == 2
            && TWO_CHAR_TITLES
                .iter()
                .any(|title| title.eq_ignore_ascii_case(word.word))
    } else {
        might_be_title_part(word)
    }
}

fn is_prefix_title(words: &[NamePart]) -> bool {
    match words.last() {
        Some(word) => {
            if !might_be_last_title_part(&word) {
                return false;
            }
        }
        None => {
            return false;
        }
    }

    if words.len() > 1 {
        words[0..words.len() - 1]
            .iter()
            .all(|word| might_be_title_part(&word))
    } else {
        true
    }
}

fn is_postfix_title(word: &NamePart, might_be_initials: bool) -> bool {
    match word.category {
        Category::Name(ref namecased) => {
            let namecased: &str = &*namecased;
            POSTFIX_TITLES.contains(namecased) || namecased.chars().any(char::is_numeric)
        }
        Category::Initials => {
            !might_be_initials && word.word.chars().filter(|c| c.is_alphabetic()).count() > 1
        }
        _ => true,
    }
}

pub fn find_prefix_len(words: &[NamePart]) -> usize {
    let mut prefix_len = words.len() - 1;

    while prefix_len > 0 {
        let found_prefix = {
            let next_word = &words[prefix_len];
            (next_word.is_namelike() || next_word.is_initials())
                && is_prefix_title(&words[0..prefix_len])
        };

        if found_prefix {
            break;
        } else {
            prefix_len -= 1;
        }
    }

    prefix_len
}

pub fn find_postfix_index(words: &[NamePart], expect_initials: bool) -> usize {
    let last_nonpostfix_index = words.iter().rposition(|word| {
        suffix::generation_from_suffix(&word, expect_initials).is_none()
            && !is_postfix_title(&word, expect_initials)
    });

    let first_abbr_index = words
        .iter()
        .position(|word| !word.is_namelike() && !word.is_initials())
        .unwrap_or_else(|| words.len());

    cmp::min(
        first_abbr_index,
        match last_nonpostfix_index {
            Some(i) => i + 1,
            None => 0,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::super::namepart::{Location, NamePart};
    use super::*;

    #[test]
    fn is_postfix_title_esq() {
        let part = NamePart::from_word("esq", true, Location::Start);
        assert!(is_postfix_title(&part, true));
    }

    #[test]
    fn is_postfix_title_et_al() {
        let parts: Vec<_> = NamePart::all_from_text("et al", true, Location::Start).collect();
        for part in parts {
            assert!(is_postfix_title(&part, true));
        }
    }

    #[test]
    fn is_postfix_title_abbr() {
        let part = NamePart::from_word("asd.", true, Location::Start);
        assert!(is_postfix_title(&part, true));
    }

    #[test]
    fn is_postfix_title_initialism() {
        let part = NamePart::from_word("a.s.d.", true, Location::Start);
        assert!(is_postfix_title(&part, false));
        assert!(!is_postfix_title(&part, true));
    }

    #[test]
    fn find_prefix_len_none() {
        let parts: Vec<_> = NamePart::all_from_text("Jane Doe", true, Location::Start).collect();
        let prefix = find_prefix_len(&parts);
        assert_eq!(
            "Jane Doe",
            parts[prefix..]
                .iter()
                .fold("".to_string(), |s, ref p| s + " " + p.word)
                .trim()
        );
    }

    #[test]
    fn find_prefix_len_abbr() {
        let parts: Vec<_> =
            NamePart::all_from_text("Dr. Jane Doe", true, Location::Start).collect();
        let prefix = find_prefix_len(&parts);
        assert_eq!(
            "Jane Doe",
            parts[prefix..]
                .iter()
                .fold("".to_string(), |s, ref p| s + " " + p.word)
                .trim()
        );
    }

    #[test]
    fn find_prefix_len_multi_abbr() {
        let parts: Vec<_> =
            NamePart::all_from_text("Revd. Dr. Jane Doe", true, Location::Start).collect();
        let prefix = find_prefix_len(&parts);
        assert_eq!(
            "Jane Doe",
            parts[prefix..]
                .iter()
                .fold("".to_string(), |s, ref p| s + " " + p.word)
                .trim()
        );
    }

    #[test]
    fn find_prefix_len_word() {
        let parts: Vec<_> =
            NamePart::all_from_text("Lady Jane Doe", true, Location::Start).collect();
        let prefix = find_prefix_len(&parts);
        assert_eq!(
            "Jane Doe",
            parts[prefix..]
                .iter()
                .fold("".to_string(), |s, ref p| s + " " + p.word)
                .trim()
        );
    }

    #[test]
    fn find_prefix_len_multi_word() {
        let parts: Vec<_> =
            NamePart::all_from_text("1st (B) Ltc Jane Doe", true, Location::Start).collect();
        let prefix = find_prefix_len(&parts);
        assert_eq!(
            "Jane Doe",
            parts[prefix..]
                .iter()
                .fold("".to_string(), |s, ref p| s + " " + p.word)
                .trim()
        );
    }

    #[test]
    fn find_prefix_len_short() {
        let parts: Vec<_> = NamePart::all_from_text("Dr. Doe", true, Location::Start).collect();
        let prefix = find_prefix_len(&parts);
        assert_eq!(
            "Doe",
            parts[prefix..]
                .iter()
                .fold("".to_string(), |s, ref p| s + " " + p.word)
                .trim()
        );
    }
}
