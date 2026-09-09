//! Hebrew abbreviations and acronyms, at [`priority::ABBREV`].
//!
//! One rule, two behaviours, because Hebrew abbreviations split in two:
//!
//! * a **spelled-out** abbreviation becomes its wording — `ד״ר כהן` -> דוקטור כהן.
//!   The gershayim stand in for words nobody says letter by letter: `ד״ר` is
//!   "doktor", not "dalet resh"; `ארה״ב` is "artsot ha-brit".
//! * a **lexicalised acronym** only loses its gershayim — `צה״ל` -> `צהל` — because a
//!   reader says /tsahal/, and expanding it would put words in the speaker's mouth
//!   and change the register. The smallest correct move is to drop the punctuation so
//!   the g2p sees a plain Hebrew word.
//!
//! Membership is a judgment call and two entries deserve a note: `ביה״ס` is *not* read
//! as a word — a speaker says "beit ha-sefer" — so it expands, even though it looks
//! like the lexicalised acronyms around it. `שליט״א` is the reverse: it looks like a
//! formula to spell out, but it is read /shlita/ as one word.
//!
//! Attached one-letter proclitics are matched alongside and put back: `בת״א` ->
//! בתל אביב, `מצה״ל` -> מצהל. Keeping them out of the lexicon means each entry is
//! written once instead of once per possible prefix.
//!
//! `cfg.expand_abbreviations = false` turns off the first behaviour only. Dropping the
//! gershayim off `צה״ל` is not an expansion — it is the difference between a word the
//! g2p can read and a string of letters with punctuation in the middle — so it always
//! happens. This rule sits below units on purpose: `ג׳` is grams after a number and the
//! letter gimel everywhere else, so `5 ג׳` must reach the unit rule first.

use std::collections::HashMap;

use fancy_regex::{Captures, Regex};
use once_cell::sync::Lazy;

use crate::config::Config;
use crate::scanner::{Rule, priority};
use crate::text::HEB;

/// Abbreviation -> the words a reader says in its place.
///
/// Keyed by the form as it appears *after* `text::clean`, i.e. with real gershayim
/// `״` and geresh `׳`, never the ASCII quotes.
const EXPANSIONS: &[(&str, &str)] = &[
    // titles and people
    ("ד״ר", "דוקטור"),
    ("פרופ׳", "פרופסור"),
    ("גב׳", "גברת"),
    ("עו״ד", "עורך דין"),
    ("רו״ח", "רואה חשבון"),
    ("יו״ר", "יושב ראש"),
    ("ח״כ", "חבר כנסת"),
    ("ז״ל", "זכרונו לברכה"),
    ("הי״ד", "השם ייקום דמו"),
    // addresses and references
    ("רח׳", "רחוב"),
    ("שד׳", "שדרות"),
    ("ת״ד", "תא דואר"),
    ("מס׳", "מספר"),
    ("עמ׳", "עמוד"),
    ("ר״ת", "ראשי תיבות"),
    ("בע״מ", "בערבון מוגבל"),
    // places
    ("ת״א", "תל אביב"),
    ("ארה״ב", "ארצות הברית"),
    ("בריה״מ", "ברית המועצות"),
    ("א״י", "ארץ ישראל"),
    ("ביה״ס", "בית הספר"),
    ("בי״ס", "בית ספר"),
    ("ביה״ח", "בית החולים"),
    ("ביהמ״ש", "בית המשפט"),
    // time
    ("אחה״צ", "אחר הצהריים"),
    ("לפנה״צ", "לפני הצהריים"),
    ("לפנה״ס", "לפני הספירה"),
    ("לספה״נ", "לספירת הנוצרים"),
    ("אח״כ", "אחר כך"),
    // connectives and stock phrases
    ("וכו׳", "וכולי"),
    ("וגו׳", "וגומר"),
    ("וכד׳", "וכדומה"),
    ("כ״א", "כל אחד"),
    ("כ״כ", "כל כך"),
    ("ע״י", "על ידי"),
    ("ע״פ", "על פי"),
    ("עפ״י", "על פי"),
    ("ע״ש", "על שם"),
    ("בד״כ", "בדרך כלל"),
    ("סה״כ", "סך הכל"),
    ("כנ״ל", "כנזכר לעיל"),
    ("א״כ", "אם כן"),
    ("צ״ל", "צריך להיות"),
    ("ד״ש", "דרישת שלום"),
    ("מו״ל", "מוציא לאור"),
    ("מו״מ", "משא ומתן"),
    ("בס״ד", "בסיעתא דשמיא"),
    ("ב״ה", "ברוך השם"),
    // single letters used as enumerators: "סעיף א׳", "יום ג׳"
    ("א׳", "אלף"),
    ("ב׳", "בית"),
    ("ג׳", "גימל"),
];

/// Acronym -> the same acronym as a plain word, for the ones a reader pronounces
/// rather than unpacks. Removing the gershayim is the entire transformation.
///
/// Values are given explicitly rather than computed, so a spelling that needs help can
/// get it without touching the rule.
const READ_AS_WORD: &[(&str, &str)] = &[
    ("צה״ל", "צהל"),
    ("אמ״ן", "אמן"),
    ("שב״כ", "שבכ"),
    ("מד״א", "מדא"),
    ("רמטכ״ל", "רמטכל"),
    ("קק״ל", "קקל"),
    ("תנ״ך", "תנך"),
    ("ש״ס", "שס"),
    ("בג״ץ", "בגץ"),
    ("מנכ״ל", "מנכל"),
    ("סמנכ״ל", "סמנכל"),
    ("נגמ״ש", "נגמש"),
    ("חו״ל", "חול"),
    ("דו״ח", "דוח"),
    ("דו״חות", "דוחות"),
    ("תפו״ז", "תפוז"),
    ("צל״ש", "צלש"),
    ("שליט״א", "שליטא"),
];

/// Single-letter proclitics that glue onto an abbreviation: בת״א, לד״ר, מצה״ל.
/// Matched separately and re-attached, so the lexicon stays free of every prefixed form.
const PREFIXES: &str = "ובלכמשה";

static EXPANSION_MAP: Lazy<HashMap<&'static str, &'static str>> =
    Lazy::new(|| EXPANSIONS.iter().copied().collect());

static WORD_MAP: Lazy<HashMap<&'static str, &'static str>> =
    Lazy::new(|| READ_AS_WORD.iter().copied().collect());

/// Every abbreviation the rule can match, longest first so `עפ״י` beats `ע״פ` and
/// `ביהמ״ש` beats the `ביה״ס`-shaped prefixes.
static ABBREVIATIONS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    let mut all: Vec<&'static str> = EXPANSIONS
        .iter()
        .chain(READ_AS_WORD.iter())
        .map(|(k, _)| *k)
        .collect();
    // Stable, so entries of equal length keep their lexicon order.
    all.sort_by_key(|a| std::cmp::Reverse(a.chars().count()));
    all
});

/// A known abbreviation, with an optional attached prefix letter.
struct AbbrevRule {
    re: Regex,
}

impl AbbrevRule {
    fn new() -> Self {
        // An abbreviation may not start or end in the middle of a Hebrew word, and the
        // geresh/gershayim count as part of one: `ג׳ינס` must not yield "גימל".
        let edge = format!("[{HEB}׳״]");
        let alts = ABBREVIATIONS
            .iter()
            .map(|a| fancy_regex::escape(a).into_owned())
            .collect::<Vec<_>>()
            .join("|");
        let pattern = format!("(?<!{edge})([{PREFIXES}]?)({alts})(?!{edge})");
        Self {
            re: Regex::new(&pattern).expect("abbrev pattern"),
        }
    }
}

impl Rule for AbbrevRule {
    fn name(&self) -> &'static str {
        "abbrev"
    }

    fn priority(&self) -> i32 {
        priority::ABBREV
    }

    fn pattern(&self) -> &Regex {
        &self.re
    }

    fn render(&self, m: &Captures<'_>, _text: &str, cfg: &Config) -> Option<String> {
        let prefix = m.get(1).map_or("", |g| g.as_str());
        let abbr = m.get(2)?.as_str();
        let word = match WORD_MAP.get(abbr) {
            Some(w) => *w,
            None => {
                if !cfg.expand_abbreviations {
                    return None;
                }
                *EXPANSION_MAP.get(abbr)?
            }
        };
        Some(format!("{prefix}{word}"))
    }
}

pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![Box::new(AbbrevRule::new())]
}
