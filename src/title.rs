use std::collections::HashSet;
use super::namepart::NamePart;

static TWO_CHAR_TITLES: [&'static str; 4] = [
    "mr",
    "ms",
    "sr",
    "dr",
];

lazy_static! {
    static ref TITLE_PARTS: HashSet<&'static str> = {
        let s: HashSet<&'static str> = [
            "aunt",
            "auntie",
            "attaché",
            "dame",
            "marchioness",
            "marquess",
            "marquis",
            "marquise",
            "king",
            "king's",
            "queen",
            "queen's",
            "1lt",
            "1st",
            "1sgt",
            "1stlt",
            "1stsgt",
            "2lt",
            "2nd",
            "2ndlt",
            "a1c",
            "abbess",
            "abbot",
            "academic",
            "acolyte",
            "adept",
            "adjutant",
            "adm",
            "admiral",
            "advocate",
            "akhoond",
            "air",
            "ald",
            "alderman",
            "almoner",
            "ambassador",
            "amn",
            "analytics",
            "appellate",
            "apprentice",
            "arbitrator",
            "archbishop",
            "archdeacon",
            "archdruid",
            "archduchess",
            "archduke",
            "arhat",
            "assistant",
            "assoc",
            "associate",
            "asst",
            "attache",
            "attorney",
            "ayatollah",
            "baba",
            "bailiff",
            "banner",
            "bard",
            "baron",
            "barrister",
            "bearer",
            "bench",
            "bgen",
            "bishop",
            "blessed",
            "bodhisattva",
            "brig",
            "brigadier",
            "briggen",
            "brother",
            "buddha",
            "burgess",
            "business",
            "bwana",
            "canon",
            "capt",
            "captain",
            "cardinal",
            "chargé",
            "catholicos",
            "ccmsgt",
            "cdr",
            "ceo",
            "cfo",
            "chair",
            "chairs",
            "chancellor",
            "chaplain",
            "chief",
            "chieftain",
            "civil",
            "clerk",
            "cmd",
            "cmdr",
            "cmsaf",
            "cmsgt",
            "co-chair",
            "co-chairs",
            "coach",
            "col",
            "colonel",
            "commander",
            "commander-in-chief",
            "commodore",
            "comptroller",
            "controller",
            "corporal",
            "corporate",
            "councillor",
            "count",
            "countess",
            "courtier",
            "cpl",
            "cpo",
            "cpt",
            "credit",
            "criminal",
            "csm",
            "curator",
            "customs",
            "cwo",
            "cwo-2",
            "cwo-3",
            "cwo-4",
            "cwo-5",
            "cwo2",
            "cwo3",
            "cwo4",
            "cwo5",
            "d'affaires",
            "deacon",
            "delegate",
            "deputy",
            "designated",
            "det",
            "dir",
            "director",
            "discovery",
            "district",
            "division",
            "docent",
            "docket",
            "doctor",
            "doyen",
            "dpty",
            "druid",
            "duke",
            "dutchess",
            "edmi",
            "edohen",
            "effendi",
            "ekegbian",
            "elder",
            "elerunwon",
            "emperor",
            "empress",
            "ens",
            "envoy",
            "exec",
            "executive",
            "fadm",
            "family",
            "father",
            "federal",
            "field",
            "financial",
            "first",
            "flag",
            "flying",
            "flight",
            "flt",
            "foreign",
            "forester",
            "frau",
            "friar",
            "gen",
            "general",
            "generalissimo",
            "gentiluomo",
            "giani",
            "goodman",
            "goodwife",
            "governor",
            "grand",
            "group",
            "guru",
            "gyani",
            "gysgt",
            "hajji",
            "headman",
            "her",
            "herr",
            "hereditary",
            "high",
            "his",
            "hon",
            "honorable",
            "honourable",
            "imam",
            "information",
            "insp",
            "intelligence",
            "intendant",
            "journeyman",
            "judge",
            "judicial",
            "justice",
            "junior",
            "kingdom",
            "knowledge",
            "lady",
            "lama",
            "lamido",
            "law",
            "lcdr",
            "lcpl",
            "leader",
            "lieutenant",
            "lord",
            "leut",
            "lieut",
            "ltc",
            "ltcol",
            "ltg",
            "ltgen",
            "ltjg",
            "madam",
            "madame",
            "mag",
            "mag-judge",
            "mag/judge",
            "magistrate",
            "magistrate-judge",
            "maharajah",
            "maharani",
            "mahdi",
            "maid",
            "maj",
            "majesty",
            "majgen",
            "major",
            "manager",
            "marcher",
            "marketing",
            "marshal",
            "master",
            "matriarch",
            "matron",
            "mayor",
            "mcpo",
            "mcpoc",
            "mcpon",
            "member",
            "metropolitan",
            "mgr",
            "mgysgt",
            "minister",
            "miss",
            "misses",
            "mister",
            "mme",
            "monsignor",
            "most",
            "mother",
            "mpco-cg",
            "mrs",
            "msg",
            "msgr",
            "msgt",
            "mufti",
            "mullah",
            "municipal",
            "murshid",
            "nanny",
            "national",
            "nurse",
            "officer",
            "operating",
            "pastor",
            "patriarch",
            "petty",
            "pfc",
            "pharaoh",
            "pilot",
            "pir",
            "po1",
            "po2",
            "po3",
            "police",
            "political",
            "pope",
            "prefect",
            "prelate",
            "premier",
            "pres",
            "presbyter",
            "president",
            "presiding",
            "priest",
            "priestess",
            "primate",
            "prime",
            "prin",
            "prince",
            "princess",
            "principal",
            "prior",
            "private",
            "pro",
            "prof",
            "professor",
            "provost",
            "pslc",
            "pte",
            "pursuivant",
            "pv2",
            "pvt",
            "rabbi",
            "radm",
            "rangatira",
            "ranger",
            "rdml",
            "rear",
            "rebbe",
            "registrar",
            "rep",
            "representative",
            "resident",
            "rev",
            "revenue",
            "reverend",
            "reverand",
            "revd",
            "right",
            "risk",
            "royal",
            "saint",
            "sargent",
            "sargeant",
            "saoshyant",
            "scpo",
            "secretary",
            "security",
            "seigneur",
            "senator",
            "senior",
            "senior-judge",
            "sergeant",
            "servant",
            "sfc",
            "sgm",
            "sgt",
            "sgtmaj",
            "sgtmajmc",
            "shehu",
            "sheikh",
            "sheriff",
            "siddha",
            "sir",
            "sister",
            "sma",
            "smsgt",
            "solicitor",
            "spc",
            "speaker",
            "special",
            "sra",
            "ssg",
            "ssgt",
            "staff",
            "state",
            "states",
            "strategy",
            "subaltern",
            "subedar",
            "sultan",
            "sultana",
            "superior",
            "supreme",
            "surgeon",
            "swordbearer",
            "sysselmann",
            "tax",
            "technical",
            "timi",
            "tirthankar",
            "treasurer",
            "tsar",
            "tsarina",
            "tsgt",
            "uncle",
            "united",
            "vadm",
            "vardapet",
            "venerable",
            "verderer",
            "very",
            "vicar",
            "vice",
            "viscount",
            "vizier",
            "warden",
            "warrant",
            "wing",
            "wo-1",
            "wo-2",
            "wo-3",
            "wo-4",
            "wo-5",
            "wo1",
            "wo2",
            "wo3",
            "wo4",
            "wo5",
            "woodman",
            "and",
            "the",
            "und",
        ].iter().cloned().collect();
        s
    };
}

fn is_title_part(word: &str) -> bool {
    let key: &str = &word.to_lowercase();
    TITLE_PARTS.contains(key)
}

fn might_be_title_part(word: &str) -> bool {
    if word.len() < 3 {
        // Allow any word with 1 or 2 characters as part of a title (but see below)
        return true;
    }

    match word.find(".") {
        Some(index) => {
            // Allow any period-terminated abbrevation as part of a title
            // (but not necessarily if there are intermediate periods, because
            // those might indicate this is a sequence of initials)
            if index == word.len() - 1 {
                true
            }
            else {
                word.split('.').all( |s| s.len() < 3 || is_title_part(s) )
            }
        }
        None => {
            is_title_part(word)
        }
    }
}

fn might_be_last_title_part(word: &str) -> bool {
    // Don't allow 1 or 2-character words as the whole or final piece of
    // a title, except a set of very-common two-character title abbreviations,
    // because otherwise we are more likely dealing with initials
    if word.len() == 1 {
        false
    }
    else if word.len() == 2 {
        let key: &str = &word.to_lowercase();
        TWO_CHAR_TITLES.contains(&key)
    }
    else if might_be_title_part(word) {
        !word.contains('.') || might_be_last_title_part(word.split('.').last().unwrap())
    }
    else {
        false
    }
}

pub fn is_title(words: &[NamePart]) -> bool {
    match words.last() {
        Some(word) => {
            if !might_be_last_title_part(word.word) {
                return false
            }
        }
        None => {
            return false;
        }
    }

    if words.len() > 1 {
        words[0..words.len()-1].iter().all( |word| might_be_title_part(word.word) )
    }
    else {
        true
    }
}
