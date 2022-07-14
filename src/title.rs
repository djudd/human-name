use super::namepart::{Category, NamePart};
use super::suffix;
use super::utils;
use phf::phf_map;
use std::cmp;
use Cow;

static TWO_CHAR_TITLES: [&str; 4] = ["mr", "ms", "sr", "dr"];

static PREFIX_HONORIFICS: phf::Map<&'static str, &'static str> = phf_map! {
    "Aunt" => "Aunt",
    "Auntie" => "Auntie",
    "Attaché" => "Attaché",
    "Dame" => "Dame",
    "Marchioness" => "Marchioness",
    "Marquess" => "Marquess",
    "Marquis" => "Marquis",
    "Marquise" => "Marquise",
    "King" => "King",
    "King'S" => "King's",
    "Queen" => "Queen",
    "Queen'S" => "Queen's",
    "Abbess" => "Abbess",
    "Abbot" => "Abbot",
    "Acad" => "Acad.",
    "Academic" => "Acad.",
    "Academian" => "Acad.",
    "Acolyte" => "Acolyte",
    "Adept" => "Adept",
    "Adjutant" => "Adjutant",
    "Adm" => "Adm.",
    "Admiral" => "Adm.",
    "Administrative" => "Adm.",
    "Administrator" => "Adm.",
    "Administrater" => "Adm.",
    "Admin" => "Adm.",
    "Advocate" => "Advocate",
    "Akhoond" => "Akhoond",
    "Air" => "Air",
    "Amn" => "Amn.",
    "Airman" => "Amn.",
    "Ald" => "Ald.",
    "Alderman" => "Ald.",
    "Almoner" => "Almoner",
    "Ambassador" => "Amb.",
    "Amb" => "Amb.",
    "Analytics" => "Analytics",
    "Appellate" => "Appellate",
    "Apprentice" => "Apprentice",
    "Arbitrator" => "Arbitrator",
    "Archbishop" => "Archbishop",
    "Archdeacon" => "Archdeacon",
    "Archdruid" => "Archdruid",
    "Archduchess" => "Archduchess",
    "Archduke" => "Archduke",
    "Arhat" => "Arhat",
    "As" => "Asst.",
    "Assistant" => "Asst.",
    "Assoc" => "Assoc.",
    "Associate" => "Assoc.",
    "Asst" => "Asst.",
    "Attache" => "Attache",
    "Attorney" => "Attorney",
    "Ayatollah" => "Ayatollah",
    "Baba" => "Baba",
    "Bachelor" => "Bachelor",
    "Baccalaureus" => "Baccalaureus",
    "Bailiff" => "Bailiff",
    "Banner" => "Banner",
    "Bard" => "Bard",
    "Baron" => "Baron",
    "Barrister" => "Barrister",
    "Bearer" => "Bearer",
    "Bench" => "Bench",
    "Bgen" => "Brig. Gen.",
    "Bishop" => "Bishop",
    "Blessed" => "Blessed",
    "Bodhisattva" => "Bodhisattva",
    "Brig" => "Brig.",
    "Brigadier" => "Brig.",
    "Briggen" => "Briggen",
    "Brother" => "Br.",
    "Br" => "Br.",
    "Buddha" => "Buddha",
    "Burgess" => "Burgess",
    "Business" => "Business",
    "Bwana" => "Bwana",
    "Canon" => "Canon",
    "Capt" => "Capt.",
    "Captain" => "Capt.",
    "Cardinal" => "Cardinal",
    "Chargé" => "Chargé",
    "Catholicos" => "Catholicos",
    "Ccmsgt" => "CCM",
    "Cdr" => "Cdr.",
    "Ceo" => "CEO",
    "Cfo" => "CFO",
    "Chair" => "Chair",
    "Chairs" => "Chairs",
    "Chancellor" => "Chancellor",
    "Chaplain" => "Chaplain",
    "Chief" => "Chief",
    "Chieftain" => "Chieftain",
    "Civil" => "Civil",
    "Clerk" => "Clerk",
    "Cmd" => "Cmd.",
    "Cmdr" => "Cmdr.",
    "Cmsaf" => "CMSAF",
    "Cmsgt" => "CMSgt",
    "Co-Chair" => "Co-Chair",
    "Co-Chairs" => "Co-Chairs",
    "Coach" => "Coach",
    "Col" => "Col.",
    "Colonel" => "Col.,",
    "Commander" => "Cmdr.",
    "Commander-In-Chief" => "Commander-In-Chief",
    "Commodore" => "Commodore",
    "Comptroller" => "Comptroller",
    "Controller" => "Controller",
    "Corporal" => "Cpl.",
    "Corporate" => "Corporate",
    "Councillor" => "Councillor",
    "Count" => "Count",
    "Countess" => "Countess",
    "Courtier" => "Courtier",
    "Cpl" => "Cpl.",
    "Cpo" => "CPO",
    "Cpt" => "Capt.",
    "Credit" => "Credit",
    "Criminal" => "Criminal",
    "Csm" => "CSM",
    "Curator" => "Curator",
    "Customs" => "Customs",
    "Cwo" => "CWO",
    "D'Affaires" => "D'Affaires",
    "Deacon" => "Deacon",
    "Delegate" => "Delegate",
    "Deputy" => "Deputy",
    "Designated" => "Designated",
    "Det" => "Det.",
    "Detective" => "Det.",
    "Dir" => "Dir.",
    "Director" => "Dir.",
    "Discovery" => "Discovery",
    "District" => "District",
    "Division" => "Division",
    "Docent" => "Docent",
    "Docket" => "Docket",
    "Doctor" => "Dr.",
    "Doc" => "Dr.",
    "Doyen" => "Doyen",
    "Dpty" => "Deputy",
    "Druid" => "Druid",
    "Duke" => "Duke",
    "Duchess" => "Duchess",
    "Edmi" => "Edmi",
    "Edohen" => "Edohen",
    "Effendi" => "Effendi",
    "Ekegbian" => "Ekegbian",
    "Elder" => "Elder",
    "Elerunwon" => "Elerunwon",
    "Emperor" => "Emperor",
    "Empress" => "Empress",
    "Engineer" => "Eng.",
    "Ens" => "Ens.",
    "Ensign" => "Ensign",
    "Envoy" => "Envoy",
    "Exec" => "Exec.",
    "Executive" => "Exec.",
    "Fadm" => "FADM",
    "Family" => "Family",
    "Father" => "Fr.",
    "Fr" => "Fr.",
    "Federal" => "Federal",
    "Field" => "Field",
    "Financial" => "Financial",
    "First" => "First",
    "Flag" => "Flag",
    "Flying" => "Flying",
    "Flight" => "Flt.",
    "Flt" => "Flt.",
    "Foreign" => "Foreign",
    "Forester" => "Forester",
    "Frau" => "Frau",
    "Friar" => "Friar",
    "Gen" => "Gen.",
    "General" => "Gen.",
    "Generalissimo" => "Gen.",
    "Gentiluomo" => "Gentiluomo",
    "Giani" => "Giani",
    "Goodman" => "Goodman",
    "Goodwife" => "Goodwife",
    "Gov" => "Gov.",
    "Governer" => "Gov.",
    "Governor" => "Gov.",
    "Grand" => "Grand",
    "Group" => "Group",
    "Guru" => "Guru",
    "Gyani" => "Gyani",
    "Gysgt" => "GySgt",
    "Hajji" => "Hajji",
    "Headman" => "Headman",
    "Her" => "Her",
    "Herr" => "Herr",
    "Hereditary" => "Hereditary",
    "High" => "High",
    "His" => "His",
    "Hon" => "Hon.",
    "Honorable" => "Hon.",
    "Honourable" => "Hon.",
    "Imam" => "Imam",
    "Information" => "Information",
    "Insp" => "Insp.",
    "Inspector" => "Insp.",
    "Intelligence" => "Intelligence",
    "Intendant" => "Intendant",
    "Journeyman" => "Journeyman",
    "Judge" => "Judge",
    "Judicial" => "Judicial",
    "Justice" => "Justice",
    "Junior" => "Jr.",
    "Jr" => "Jr.",
    "Kingdom" => "Kingdom",
    "Knowledge" => "Knowledge",
    "Lady" => "Lady",
    "Lama" => "Lama",
    "Lamido" => "Lamido",
    "Law" => "Law",
    "Lcdr" => "LCDR",
    "Lcpl" => "LCpl",
    "Leader" => "Leader",
    "Lieutenant" => "Lt.",
    "Lord" => "Lord",
    "Leut" => "Lt.",
    "Lieut" => "Lt.",
    "Ltc" => "Lt. Col.",
    "Ltcol" => "Lt. Col.",
    "Ltg" => "Lt. Gen.",
    "Ltgen" => "Lt. Gen.",
    "Ltjg" => "LTJG",
    "Madam" => "Madam",
    "Madame" => "Mme.",
    "Mag" => "Mag.",
    "Mag-Judge" => "Magistrate Judge",
    "Mag/Judge" => "Magistrate udge",
    "Magistrate" => "Magistrate",
    "Magistrate-Judge" => "Magistrate Judge",
    "Maharajah" => "Maharajah",
    "Maharani" => "Maharani",
    "Mahdi" => "Mahdi",
    "Maid" => "Maid",
    "Maj" => "Maj.",
    "Majesty" => "Majesty",
    "Majgen" => "Maj. Gen.",
    "Major" => "Maj.",
    "Manager" => "Mgr.",
    "Marcher" => "Marcher",
    "Marketing" => "Marketing",
    "Marshal" => "Marshal",
    "Master" => "Mr.",
    "Matriarch" => "Matriarch",
    "Matron" => "Matron",
    "Mayor" => "Mayor",
    "Mcpo" => "MCPO",
    "Mcpoc" => "MCPOC",
    "Mcpon" => "MCPON",
    "Member" => "Member",
    "Metropolitan" => "Metropolitan",
    "Mgr" => "Mgr.",
    "Mgysgt" => "MGySgt",
    "Minister" => "Minister",
    "Miss" => "Ms.",
    "Misses" => "Misses",
    "Mister" => "Mr.",
    "Mme" => "Mme.",
    "Monsignor" => "Msgr.",
    "Most" => "Most",
    "Mother" => "Mother",
    "Mpco-Cg" => "MCPOCG",
    "Mrs" => "Mrs.",
    "Missus" => "Mrs.",
    "Msg" => "MSG",
    "Msgr" => "Msgr.",
    "Msgt" => "MSgt",
    "Mufti" => "Mufti",
    "Mullah" => "Mullah",
    "Municipal" => "Municipal",
    "Murshid" => "Murshid",
    "Mx" => "Mx.",
    "Mz" => "Mz.",
    "Nanny" => "Nanny",
    "National" => "National",
    "Nurse" => "Nurse",
    "Officer" => "Ofc.",
    "Ofc" => "Ofc.",
    "Operating" => "Operating",
    "Pastor" => "Pastor",
    "Patriarch" => "Patriarch",
    "Petty" => "Petty",
    "Pfc" => "PFC",
    "Pharaoh" => "Pharaoh",
    "Pilot" => "Pilot",
    "Pir" => "Pir",
    "Police" => "Police",
    "Political" => "Political",
    "Pope" => "Pope",
    "Prefect" => "Prefect",
    "Prelate" => "Prelate",
    "Premier" => "Premier",
    "Pres" => "Pres.",
    "Presbyter" => "Presbyter",
    "President" => "Pres.",
    "Presiding" => "Presiding",
    "Priest" => "Priest",
    "Priestess" => "Priestess",
    "Primate" => "Primate",
    "Prime" => "Prime",
    "Prin" => "Prin.",
    "Prince" => "Prince",
    "Princess" => "Princess",
    "Principal" => "Prin.",
    "Prior" => "Prior",
    "Private" => "Pvt.",
    "Pro" => "Pro",
    "Prof" => "Prof.",
    "Professor" => "Prof.",
    "Provost" => "Provost",
    "Pte" => "Pte.",
    "Pursuivant" => "Pursuivant",
    "Pvt" => "Pvt.",
    "Rabbi" => "Rabbi",
    "Radm" => "RADM",
    "Rangatira" => "Rangatira",
    "Ranger" => "Ranger",
    "Rdml" => "RDML",
    "Rear" => "Rear",
    "Rebbe" => "Rebbe",
    "Registrar" => "Registrar",
    "Rep" => "Rep.",
    "Representative" => "Rep.",
    "Resident" => "Resident",
    "Rev" => "Rev.",
    "Revenue" => "Revenue",
    "Reverend" => "Rev.",
    "Reverand" => "Rev.",
    "Revd" => "Rev.",
    "Rev'D" => "Rev.",
    "Right" => "Right",
    "Risk" => "Risk",
    "Royal" => "Royal",
    "Saint" => "Saint",
    "Sargent" => "Sgt.",
    "Sargeant" => "Sgt.",
    "Saoshyant" => "Saoshyant",
    "Scpo" => "SCPO",
    "Secretary" => "Sec.",
    "Sec" => "Sec.",
    "Security" => "Security",
    "Seigneur" => "Seigneur",
    "Senator" => "Sen.",
    "Sen" => "Sen.",
    "Senior" => "Senior",
    "Senior-Judge" => "Senior-Judge",
    "Sergeant" => "Sgt.",
    "Servant" => "Servant",
    "Sfc" => "SFC",
    "Sgm" => "SGM",
    "Sgt" => "Sgt.",
    "Sgtmaj" => "SGM",
    "Sgtmajmc" => "SMMC",
    "Shehu" => "Shehu",
    "Sheikh" => "Sheikh",
    "Sheriff" => "Sheriff",
    "Siddha" => "Siddha",
    "Sir" => "Sir",
    "Sister" => "Sr.",
    "Sr" => "Sr.",
    "Sma" => "SMA",
    "Smsgt" => "SMSgt",
    "Solicitor" => "Solicitor",
    "Spc" => "SPC",
    "Speaker" => "Speaker",
    "Special" => "Special",
    "Specialist" => "Specialist",
    "Sra" => "SrA",
    "Ssg" => "SSG",
    "Ssgt" => "SSgt",
    "Staff" => "Staff",
    "State" => "State",
    "States" => "States",
    "Strategy" => "Strategy",
    "Subaltern" => "Subaltern",
    "Subedar" => "Subedar",
    "Sultan" => "Sultan",
    "Sultana" => "Sultana",
    "Superior" => "Superior",
    "Superintendent" => "Supt.",
    "Supt" => "Supt.",
    "Supreme" => "Supreme",
    "Surgeon" => "Surgeon",
    "Swordbearer" => "Swordbearer",
    "Sysselmann" => "Sysselmann",
    "Tax" => "Tax",
    "Technical" => "Technical",
    "Timi" => "Timi",
    "Tirthankar" => "Tirthankar",
    "Treasurer" => "Treas.",
    "Treas" => "Treas.",
    "Tsar" => "Tsar",
    "Tsarina" => "Tsarina",
    "Tsgt" => "TSgt",
    "Uncle" => "Uncle",
    "United" => "United",
    "Vadm" => "VAdm",
    "Vardapet" => "Vardapet",
    "Venerable" => "Venerable",
    "Verderer" => "Verderer",
    "Very" => "Very",
    "Vicar" => "Vicar",
    "Vice" => "Vice",
    "Viscount" => "Viscount",
    "Vizier" => "Vizier",
    "Warden" => "Warden",
    "Warrant" => "Warrant",
    "Wing" => "Wing",
    "Woodman" => "Woodman",
    "Icdr" => "ICDr.",
    "Judr" => "JUDr.",
    "Mddr" => "MDDr.",
    "Bca" => "BcA.",
    "Mga" => "MgA.",
    "Md" => "M.D.",
    "Dvm" => "DVM",
    "Paeddr" => "PaedDr.",
    "Pharmdr" => "PharmDr.",
    "Phdr" => "PhDr.",
    "Phmr" => "PhMr.",
    "Rcdr" => "RCDr.",
    "Rndr" => "RNDr.",
    "Dsc" => "DSc.",
    "Rsdr" => "RSDr.",
    "Rtdr" => "RTDr.",
    "Thdr" => "ThDr.",
    "Thd" => "Th.D.",
    "Phd" => "Ph.D.",
    "Thlic" => "ThLic.",
    "Thmgr" => "ThMgr.",
    "Artd" => "ArtD.",
    "Dis" => "DiS.",

    "And" => "and",
    "The" => "The",
    "Und" => "und",
};

static POSTFIX_HONORIFICS: phf::Map<&'static str, &'static str> = phf_map! {
    "Esq" => "Esq.",
    "Esquire" => "Esq.",
    "Attorney-at-law" => "Attorney-at-law",
    "Msc" => "M.Sc",
    "Bcompt" => "BCompt",
    "Phd" => "Ph.D.",
    "Rph" => "RPh",
    "Chb" => "ChB",
    "Freng" => "FREng",
    "Meng" => "M.Eng",
    "Bgdipbus" => "BGDipBus",
    "Dip" => "Dip",
    "Diplphys" => "Dipl.Phys",
    "Mhsc" => "M.H.Sc.",
    "Bcomm" => "B.Comm",
    "Beng" => "B.Eng",
    "Bacc" => "B.Acc",
    "Mtech" => "M.Tech",
    "Bec" => "B.Ec",
    "Capom" => "CAP-OM",
    "Peng" => "P.Eng",
    "Bch" => "BCh",
    "Mbbchir" => "MBBChir",
    "Mbchba" => "MBChBa",
    "Mphil" => "MPhil",
    "Lld" => "LL.D",
    "Dlit" => "D.Lit",
    "Dclinpsy" => "DClinPsy",
    "Dsc" => "DSc",
    "Mres" => "M.Res",
    "Psyd" => "Psy.D",
    "Pharmd" => "Pharm.D",
    "Bacom" => "BACom",
    "Badmin" => "BAdmin",
    "Baecon" => "BAEcon",
    "Bagr" => "BAgr",
    "Balaw" => "BALaw",
    "Bappsc" => "BAppSc",
    "Barch" => "BArch",
    "Barchsc" => "BArchSc",
    "Barelst" => "BARelSt",
    "Basc" => "BASc",
    "Basoc" => "BASoc",
    "Batheol" => "BATheol",
    "Bbus" => "BBus",
    "Bchem" => "BChem",
    "Bclinsci" => "BClinSci",
    "Bcombst" => "BCombSt",
    "Bcommedcommdev" => "BCommEdCommDev",
    "Bcomp" => "BComp",
    "Bcomsc" => "BComSc",
    "Bcoun" => "BCoun",
    "Bdes" => "BDes",
    "Becon" => "BEcon",
    "Beconfin" => "BEcon&Fin",
    "Beconsci" => "BEconSci",
    "Bed" => "BEd",
    "Bfin" => "BFin",
    "Bhealthsc" => "BHealthSc",
    "Bhsc" => "BHSc",
    "Bhy" => "BHy",
    "Bjur" => "BJur",
    "Blegsc" => "BLegSc",
    "Blib" => "BLib",
    "Bling" => "BLing",
    "Blitt" => "BLitt",
    "Blittcelt" => "BLittCelt",
    "Bmedsc" => "BMedSc",
    "Bmet" => "BMet",
    "Bmid" => "BMid",
    "Bmin" => "BMin",
    "Bmsc" => "BMSc",
    "Bmus" => "BMus",
    "Bmused" => "BMusEd",
    "Bmusperf" => "BMusPerf",
    "Bnurs" => "BNurs",
    "Boptom" => "BOptom",
    "Bpharm" => "BPharm",
    "Bphil" => "BPhil",
    "Tchg" => "Tchg",
    "Med" => "MEd",
    "Bachelor" => "Bachelor",
    "Ceng" => "C.Eng",
    "Bphys" => "BPhys",
    "Bphysio" => "BPhysio",
    "Bpl" => "BPl",
    "Bradiog" => "BRadiog",
    "Bsc" => "B.Sc",
    "Bscagr" => "BScAgr",
    "Bscec" => "BScEc",
    "Bscecon" => "BScEcon",
    "Bscfor" => "BScFor",
    "Bsocsc" => "BSocSc",
    "Bstsu" => "BStSu",
    "Btchg" => "BTchg",
    "Btech" => "BTech",
    "Bteched" => "BTechEd",
    "Bth" => "BTh",
    "Btheol" => "BTheol",
    "Edb" => "EdB",
    "Littb" => "LittB",
    "Musb" => "MusB",
    "Scbtech" => "ScBTech",
    "Cfa" => "CFA",
    "Llb" => "LL.B",
    "Llm" => "LL.M",
    "Solicitor" => "Solicitor",
    "Cenv" => "CEnv",
    "Bcom" => "B.Com",
    "Mec" => "MEc",
    "Hdip" => "HDip",
    "Et" => "et",
    "Al" => "al.",
};

#[allow(clippy::if_same_then_else)]
fn might_be_title_part(word: &NamePart) -> bool {
    if word.counts.chars < 3 {
        // Allow any word with 1 or 2 characters as part of a title (but see below)
        true
    } else {
        match &word.category {
            Category::Name(ref namecased) => {
                let namecased: &str = namecased;
                PREFIX_HONORIFICS.contains_key(namecased) || namecased.chars().any(char::is_numeric)
            }
            _ => true,
        }
    }
}

fn might_be_last_title_part(word: &NamePart) -> bool {
    // Don't allow 1 or 2-character words as the whole or final piece of
    // a title, except a set of very-common two-character title abbreviations,
    // because otherwise we are more likely dealing with initials
    match word.counts.alpha {
        0..=1 => false,
        2 if word.counts.chars == 2 => TWO_CHAR_TITLES
            .iter()
            .any(|title| title.eq_ignore_ascii_case(word.word)),
        _ => might_be_title_part(word),
    }
}

fn is_prefix_title(words: &[NamePart]) -> bool {
    match words.last() {
        Some(word) => {
            if !might_be_last_title_part(word) {
                return false;
            }
        }
        None => {
            return false;
        }
    }

    if words.len() > 1 {
        words[0..words.len() - 1].iter().all(might_be_title_part)
    } else {
        true
    }
}

fn is_postfix_title(word: &NamePart, might_be_initials: bool) -> bool {
    match word.category {
        Category::Name(ref namecased) => {
            let namecased: &str = namecased;
            POSTFIX_HONORIFICS.contains_key(namecased) || namecased.chars().any(char::is_numeric)
        }
        Category::Initials => !might_be_initials && word.counts.alpha > 1,
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
        suffix::generation_from_suffix(word, expect_initials).is_none()
            && !is_postfix_title(word, expect_initials)
    });

    let first_abbr_index = words
        .iter()
        .position(|word| !word.is_namelike() && !word.is_initials())
        .unwrap_or(words.len());

    cmp::min(
        first_abbr_index,
        match last_nonpostfix_index {
            Some(i) => i + 1,
            None => 0,
        },
    )
}

pub fn canonicalize_suffix<'a>(title: &'a NamePart<'a>) -> Cow<'a, str> {
    match &title.category {
        Category::Name(namecased) => {
            if let Some(canonical) = POSTFIX_HONORIFICS.get(namecased) {
                Cow::Borrowed(canonical)
            } else {
                Cow::Borrowed(namecased)
            }
        }
        Category::Initials => {
            // If there's existing punctuation, assume formatting is intentional.
            if title.counts.chars != title.counts.alpha {
                return Cow::Borrowed(title.word);
            }

            // Otherwise, ignore case to check for a known canonical form (restricting
            // to ASCII just for simplicity since our list of honorifics is 100% ASCII).
            if title.counts.chars == title.counts.ascii_alpha {
                let capitalized = utils::capitalize_word(title.word, true);
                if let Some(canonical) = POSTFIX_HONORIFICS.get(&capitalized) {
                    return Cow::Borrowed(canonical);
                }
            }

            // Assume unrecognized honorifics are acronyms (given that we previously
            // categorized as initials). For length two or less, format with periods
            // (e.g. "M.D."), but skip periods for longer acronyms (e.g. "LCSW").
            if title.word.len() <= 2 {
                let mut result = String::with_capacity((title.counts.alpha * 2).into());
                title.with_initials(|c| {
                    for u in c.to_uppercase() {
                        result.push(u);
                    }
                    result.push('.');
                });
                Cow::Owned(result)
            } else {
                let mut result = String::with_capacity((title.counts.alpha).into());
                title.with_initials(|c| {
                    for u in c.to_uppercase() {
                        result.push(u);
                    }
                });
                Cow::Owned(result)
            }
        }
        Category::Abbreviation | Category::Other => Cow::Borrowed(title.word),
    }
}

pub fn canonicalize_prefix<'a>(title: &'a NamePart<'a>) -> Cow<'a, str> {
    match &title.category {
        Category::Name(namecased) => {
            if let Some(canonical) = PREFIX_HONORIFICS.get(namecased) {
                Cow::Borrowed(canonical)
            } else {
                Cow::Borrowed(namecased)
            }
        }
        Category::Initials => {
            // If there's existing punctuation, assume formatting is intentional.
            if title.counts.chars != title.counts.alpha {
                return Cow::Borrowed(title.word);
            }

            // Otherwise, ignore case to check for a known canonical form (restricting
            // to ASCII just for simplicity since our list of honorifics is 100% ASCII).
            if title.counts.chars == title.counts.ascii_alpha {
                let capitalized = utils::capitalize_word(title.word, true);
                if let Some(canonical) = PREFIX_HONORIFICS.get(&capitalized) {
                    return Cow::Borrowed(canonical);
                }
            }

            // For unrecognized honorifics, canonicalize as an abbreviation (e.g. "Dr.").
            let mut result = String::with_capacity((title.counts.alpha + 1).into());
            title.with_initials(|c| {
                if result.is_empty() {
                    result.push(c);
                } else {
                    for l in c.to_lowercase() {
                        result.push(l);
                    }
                }
            });
            result.push('.');
            Cow::Owned(result)
        }
        Category::Abbreviation | Category::Other => Cow::Borrowed(title.word),
    }
}

#[cfg(test)]
mod tests {
    use super::super::namepart::{Location, NamePart};
    use super::*;

    #[test]
    fn canonicalize_doctor_prefix() {
        assert_eq!(
            "Dr.",
            canonicalize_prefix(&NamePart::from_word("DR", true, Location::Start))
        );
        assert_eq!(
            "Dr.",
            canonicalize_prefix(&NamePart::from_word("Dr", true, Location::Start))
        );
        assert_eq!(
            "Dr.",
            canonicalize_prefix(&NamePart::from_word("dr", true, Location::Start))
        );
        assert_eq!(
            "Dr.",
            canonicalize_prefix(&NamePart::from_word("Doctor", true, Location::Start))
        );
        assert_eq!(
            "Dr.",
            canonicalize_prefix(&NamePart::from_word("Dr.", true, Location::Start))
        );
    }

    #[test]
    fn canonicalize_mister_prefix() {
        assert_eq!(
            "Mr.",
            canonicalize_prefix(&NamePart::from_word("MR", true, Location::Start))
        );
        assert_eq!(
            "Mr.",
            canonicalize_prefix(&NamePart::from_word("Mr", true, Location::Start))
        );
        assert_eq!(
            "Mr.",
            canonicalize_prefix(&NamePart::from_word("mr", true, Location::Start))
        );
        assert_eq!(
            "Mr.",
            canonicalize_prefix(&NamePart::from_word("Mister", true, Location::Start))
        );
        assert_eq!(
            "Mr.",
            canonicalize_prefix(&NamePart::from_word("Master", true, Location::Start))
        );
        assert_eq!(
            "Mr.",
            canonicalize_prefix(&NamePart::from_word("Mr.", true, Location::Start))
        );
    }

    #[test]
    fn canonicalize_mrs_prefix() {
        assert_eq!(
            "Mrs.",
            canonicalize_prefix(&NamePart::from_word("MRS", true, Location::Start))
        );
        assert_eq!(
            "Mrs.",
            canonicalize_prefix(&NamePart::from_word("Mrs", true, Location::Start))
        );
        assert_eq!(
            "Mrs.",
            canonicalize_prefix(&NamePart::from_word("mrs", true, Location::Start))
        );
        assert_eq!(
            "Mrs.",
            canonicalize_prefix(&NamePart::from_word("Missus", true, Location::Start))
        );
        assert_eq!(
            "Mrs.",
            canonicalize_prefix(&NamePart::from_word("Mrs.", true, Location::Start))
        );
    }

    #[test]
    fn canonicalize_prof_prefix() {
        assert_eq!(
            "Prof.",
            canonicalize_prefix(&NamePart::from_word("PROF", true, Location::Start))
        );
        assert_eq!(
            "Prof.",
            canonicalize_prefix(&NamePart::from_word("Prof", true, Location::Start))
        );
        assert_eq!(
            "Prof.",
            canonicalize_prefix(&NamePart::from_word("prof", true, Location::Start))
        );
        assert_eq!(
            "Prof.",
            canonicalize_prefix(&NamePart::from_word("Professor", true, Location::Start))
        );
        assert_eq!(
            "Prof.",
            canonicalize_prefix(&NamePart::from_word("Prof.", true, Location::Start))
        );
    }

    #[test]
    fn canonicalize_sir_prefix() {
        assert_eq!(
            "Sir",
            canonicalize_prefix(&NamePart::from_word("Sir", true, Location::Start))
        );
        assert_eq!(
            "Sir",
            canonicalize_prefix(&NamePart::from_word("Sir", true, Location::Start))
        );
        assert_eq!(
            "Sir",
            canonicalize_prefix(&NamePart::from_word("Sir", true, Location::Start))
        );
    }

    #[test]
    fn canonicalize_unrecognized_prefix() {
        assert_eq!(
            "Abc.",
            canonicalize_prefix(&NamePart::from_word("ABC", true, Location::Start))
        );
        assert_eq!(
            "Abc",
            canonicalize_prefix(&NamePart::from_word("Abc", true, Location::Start))
        );
        assert_eq!(
            "Abc",
            canonicalize_prefix(&NamePart::from_word("abc", true, Location::Start))
        );
        assert_eq!(
            "Abc.",
            canonicalize_prefix(&NamePart::from_word("Abc.", true, Location::Start))
        );

        assert_eq!(
            "Xx.",
            canonicalize_prefix(&NamePart::from_word("XX", true, Location::Start))
        );
        assert_eq!(
            "Xx.",
            canonicalize_prefix(&NamePart::from_word("Xx", true, Location::Start))
        );
        assert_eq!(
            "Xx.",
            canonicalize_prefix(&NamePart::from_word("xx", true, Location::Start))
        );
        assert_eq!(
            "Xx.",
            canonicalize_prefix(&NamePart::from_word("Xx.", true, Location::Start))
        );
    }

    #[test]
    fn canonicalize_phd_suffix() {
        assert_eq!(
            "Ph.D.",
            canonicalize_suffix(&NamePart::from_word("phd", true, Location::End))
        );
        assert_eq!(
            "Ph.D.",
            canonicalize_suffix(&NamePart::from_word("Phd", true, Location::End))
        );
        assert_eq!(
            "Ph.D.",
            canonicalize_suffix(&NamePart::from_word("PHD", true, Location::End))
        );
        assert_eq!(
            "Ph.D.",
            canonicalize_suffix(&NamePart::from_word("Ph.D.", true, Location::End))
        );
    }

    #[test]
    fn canonicalize_md_suffix() {
        assert_eq!(
            "M.D.",
            canonicalize_suffix(&NamePart::from_word("MD", true, Location::End))
        );
        assert_eq!(
            "M.D.",
            canonicalize_suffix(&NamePart::from_word("Md", true, Location::End))
        );
        assert_eq!(
            "M.D.",
            canonicalize_suffix(&NamePart::from_word("md", true, Location::End))
        );
        assert_eq!(
            "M.D.",
            canonicalize_suffix(&NamePart::from_word("M.D.", true, Location::End))
        );
    }

    #[test]
    fn canonicalize_esq_suffix() {
        assert_eq!(
            "Esq.",
            canonicalize_suffix(&NamePart::from_word("ESQ", true, Location::End))
        );
        assert_eq!(
            "Esq.",
            canonicalize_suffix(&NamePart::from_word("Esq", true, Location::End))
        );
        assert_eq!(
            "Esq.",
            canonicalize_suffix(&NamePart::from_word("esq", true, Location::End))
        );
        assert_eq!(
            "Esq.",
            canonicalize_suffix(&NamePart::from_word("Esquire", true, Location::End))
        );
        assert_eq!(
            "Esq.",
            canonicalize_suffix(&NamePart::from_word("Esq.", true, Location::End))
        );
    }

    #[test]
    fn canonicalize_unrecognized_suffix() {
        assert_eq!(
            "ABC",
            canonicalize_suffix(&NamePart::from_word("ABC", true, Location::End))
        );
        assert_eq!(
            "Abc",
            canonicalize_suffix(&NamePart::from_word("Abc", true, Location::End))
        );
        assert_eq!(
            "Abc",
            canonicalize_suffix(&NamePart::from_word("abc", true, Location::End))
        );
        assert_eq!(
            "A.B.C.",
            canonicalize_suffix(&NamePart::from_word("A.B.C.", true, Location::End))
        );

        assert_eq!(
            "X.X.",
            canonicalize_suffix(&NamePart::from_word("XX", true, Location::End))
        );
        assert_eq!(
            "X.X.",
            canonicalize_suffix(&NamePart::from_word("Xx", true, Location::End))
        );
        assert_eq!(
            "X.X.",
            canonicalize_suffix(&NamePart::from_word("xx", true, Location::End))
        );
        assert_eq!(
            "Xx.",
            canonicalize_suffix(&NamePart::from_word("Xx.", true, Location::End))
        );
    }

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
