//! Dates: `12/03/2026`, `12.03.2026`, `12-03-2026`, `12/3/26`, `2026-03-12`,
//! `12 במרץ 2026`, the bare month/year `3/2026`, and Hebrew-calendar dates such as
//! `כ״ז באלול` and `ט״ו בשבט`.
//!
//! Judgment calls made here:
//!
//! - **Gender.** The day of the month is masculine (יום is masculine) — `12/03` is
//!   "שנים עשר במרץ", not "שתים עשרה". The year is feminine (שנה is feminine), so
//!   2026 is "אלפיים עשרים ושש". Getting these backwards is the single most audible
//!   mistake a Hebrew date reader can make, so they are wired in, not inferred.
//! - **The first of the month is an ordinal**: "הראשון במרץ", never "אחד במרץ".
//!   Days 2-31 are plain cardinals, which is how Israelis actually say them.
//! - **Validation is a real calendar check.** The Python reference uses
//!   `datetime.date`; here it is a hand-rolled leap-year check, deliberately, so the
//!   crate does not gain a date dependency for four lines of arithmetic. That is what
//!   rules out `32/13/2026`, `00/05/2026`, `30/02/2026` and `29/02/2026`.
//! - **The configured order is a preference, not a straitjacket.** With
//!   `DateOrder::Mdy` the text `25/12/2026` has no month 25, so rather than declining a
//!   perfectly readable date the other order is tried. Only a date impossible in *both*
//!   orders is refused.
//! - **Two-digit years** follow the usual window: 00-29 -> 2000s, 30-99 -> 1900s.
//! - `2026-03-12` (ISO) is order-independent and is read regardless of `date_order`.
//!
//! A date-shaped span that fails validation is claimed and returned **unchanged**, not
//! declined. `32/13/2026` is far more likely to be a serial number than arithmetic, and
//! letting the ranges and numbers rules loose on it produced "אלפיים עשרים ושש עד שלוש
//! עשרה". Not understanding something is a reason to leave it alone.
//!
//! The Hebrew-calendar rule needs the month: a bare `כ״ז` is indistinguishable from a
//! gershayim acronym, and the abbreviations rule has the better claim on it. It sits at
//! DATE priority for the same reason — without it `א׳ בתשרי` was read "אלף בתשרי".

use fancy_regex::{Captures, Regex};

use crate::config::{Config, DateOrder, Gender, HebrewDateStyle};
use crate::numerals::{numeral, ordinal};
use crate::scanner::{Rule, priority};

// ---------------------------------------------------------------------------
// Calendar vocabulary
// ---------------------------------------------------------------------------

/// Gregorian month number -> the spelling we emit. March is written both מרץ and מרס;
/// both are accepted on input, מרץ is emitted.
const MONTHS: [&str; 12] = [
    "ינואר",
    "פברואר",
    "מרץ",
    "אפריל",
    "מאי",
    "יוני",
    "יולי",
    "אוגוסט",
    "ספטמבר",
    "אוקטובר",
    "נובמבר",
    "דצמבר",
];

/// Every spelling accepted on input -> month number. Includes the emitted forms.
const MONTH_NUMBERS: &[(&str, i64)] = &[
    ("ינואר", 1),
    ("פברואר", 2),
    ("מרץ", 3),
    ("אפריל", 4),
    ("מאי", 5),
    ("יוני", 6),
    ("יולי", 7),
    ("אוגוסט", 8),
    ("ספטמבר", 9),
    ("אוקטובר", 10),
    ("נובמבר", 11),
    ("דצמבר", 12),
    ("מרס", 3),
];

/// Hebrew-calendar months, in calendar order starting at תשרי. אדר א׳/אדר ב׳ in a leap
/// year are spelled out rather than abbreviated, since a g2p cannot read ׳.
const HEBREW_MONTHS: [&str; 12] = [
    "תשרי",
    "חשוון",
    "כסלו",
    "טבת",
    "שבט",
    "אדר",
    "ניסן",
    "אייר",
    "סיוון",
    "תמוז",
    "אב",
    "אלול",
];

/// Defective spellings that turn up in real text -> the plene form above.
const HEBREW_MONTH_VARIANTS: &[(&str, &str)] = &[
    ("חשון", "חשוון"),
    ("מרחשוון", "חשוון"),
    ("מרחשון", "חשוון"),
    ("כסליו", "כסלו"),
    ("סיון", "סיוון"),
    ("מנחם אב", "אב"),
    ("אדר א", "אדר ראשון"),
    ("אדר ב", "אדר שני"),
];

/// Letter values for reading a Hebrew-calendar date. Final forms carry their ordinary
/// value; they are vanishingly rare in a date but cost nothing to accept.
const GEMATRIA: &[(char, i64)] = &[
    ('א', 1),
    ('ב', 2),
    ('ג', 3),
    ('ד', 4),
    ('ה', 5),
    ('ו', 6),
    ('ז', 7),
    ('ח', 8),
    ('ט', 9),
    ('י', 10),
    ('כ', 20),
    ('ך', 20),
    ('ל', 30),
    ('מ', 40),
    ('ם', 40),
    ('נ', 50),
    ('ן', 50),
    ('ס', 60),
    ('ע', 70),
    ('פ', 80),
    ('ף', 80),
    ('צ', 90),
    ('ץ', 90),
    ('ק', 100),
    ('ר', 200),
    ('ש', 300),
    ('ת', 400),
];

/// Dates whose name is a word, not a reading. `ט״ו בשבט` is /tu bishvat/ to every
/// speaker; spelling it out as חמישה עשר בשבט is correct and wrong at the same time.
/// The last three are the mirror image: the fast days are lexicalised as numerals, so
/// splitting them into ט׳ ב׳ would be wrong. Either way the name beats both styles.
const HEBREW_DATE_NAMES: &[(i64, &str, &str)] = &[
    (15, "שבט", "טו בשבט"),
    (15, "אב", "טו באב"),
    (33, "עומר", "לג בעומר"),
    (9, "אב", "תשעה באב"),
    (10, "טבת", "עשרה בטבת"),
    (17, "תמוז", "שבעה עשר בתמוז"),
];

/// Not a month, but it counts days the same way and appears in the same shape.
const OMER: &str = "עומר";

/// Geresh, marking a single letter used as a numeral.
const GERESH: char = '׳';

fn month_name(number: i64) -> &'static str {
    MONTHS[(number - 1) as usize]
}

fn month_number(name: &str) -> Option<i64> {
    MONTH_NUMBERS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, v)| *v)
}

fn hebrew_month_canonical(name: &str) -> &str {
    HEBREW_MONTH_VARIANTS
        .iter()
        .find(|(v, _)| *v == name)
        .map(|(_, plene)| *plene)
        .unwrap_or(name)
}

fn hebrew_date_name(day: i64, month: &str) -> Option<&'static str> {
    HEBREW_DATE_NAMES
        .iter()
        .find(|(d, m, _)| *d == day && *m == month)
        .map(|(_, _, spoken)| *spoken)
}

fn letter_value(ch: char) -> Option<i64> {
    GEMATRIA.iter().find(|(c, _)| *c == ch).map(|(_, v)| *v)
}

/// Numeric value of a Hebrew numeral, ignoring its geresh/gershayim.
fn gematria(letters: &str) -> i64 {
    letters.chars().filter_map(letter_value).sum()
}

/// Separate a Hebrew numeral into individually marked letters: כ״ז -> כ׳ ז׳.
///
/// The point is disambiguation, not pronunciation. Written with gershayim, כ״ז is
/// indistinguishable from an acronym; written as כ׳ ז׳ it is unmistakably two letters
/// read one at a time, which is what a g2p needs in order to say them.
fn split_letters(numeral_text: &str) -> String {
    numeral_text
        .chars()
        .filter(|c| letter_value(*c).is_some())
        .map(|c| format!("{c}{GERESH}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Longest first, so "מרץ" cannot shadow a longer spelling that starts the same way.
fn longest_first(mut names: Vec<&str>) -> String {
    // A stable sort, so equal-length spellings keep their table order.
    names.sort_by_key(|name| std::cmp::Reverse(name.chars().count()));
    names.join("|")
}

// ---------------------------------------------------------------------------
// Calendar arithmetic
// ---------------------------------------------------------------------------

fn is_leap(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in_month(month: i64, year: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// A real calendar check, matching what `datetime.date` accepts in the reference.
fn is_valid(day: i64, month: i64, year: i64) -> bool {
    (1..=9999).contains(&year)
        && (1..=12).contains(&month)
        && day >= 1
        && day <= days_in_month(month, year)
}

/// Two-digit years: 00-29 are this century, 30-99 the last one.
fn expand_year(raw: &str) -> i64 {
    let value: i64 = raw.parse().unwrap_or(0);
    if raw.len() == 4 {
        return value;
    }
    if value <= 29 {
        2000 + value
    } else {
        1900 + value
    }
}

fn day_words(day: i64) -> String {
    // The 1st is the only day said as an ordinal; the rest are cardinals.
    if day == 1 {
        ordinal(1, Gender::Masc, true)
    } else {
        numeral(day, Gender::Masc, false)
    }
}

fn spell(day: Option<i64>, month: i64, year: Option<i64>) -> String {
    let mut parts: Vec<String> = Vec::new();
    match day {
        Some(d) => parts.push(format!("{} ב{}", day_words(d), month_name(month))),
        None => parts.push(month_name(month).to_string()),
    }
    if let Some(y) = year {
        parts.push(numeral(y, Gender::Fem, false));
    }
    parts.join(" ")
}

/// Map the two leading numbers onto (day, month) using the configured order.
///
/// Falls back to the opposite order when the configured one is impossible, which is
/// what makes `25/12/2026` readable even under MDY.
fn resolve(first: i64, second: i64, year: i64, cfg: &Config) -> Option<(i64, i64)> {
    let orders = match cfg.date_order {
        DateOrder::Mdy => [(second, first), (first, second)],
        DateOrder::Dmy => [(first, second), (second, first)],
    };
    orders
        .into_iter()
        .find(|(day, month)| is_valid(*day, *month, year))
}

/// A date must not start in the middle of a number, and must not be the head of a
/// longer numeric run. A preceding "-" is fine: "ב-12/03/2026" is ordinary Hebrew.
const BEFORE: &str = r"(?<![\d./])";
const AFTER: &str = r"(?!\d)(?![./-]\d)";

fn capture<'t>(m: &Captures<'t>, index: usize) -> Option<&'t str> {
    m.get(index).map(|g| g.as_str())
}

fn number(m: &Captures<'_>, index: usize) -> i64 {
    capture(m, index).and_then(|s| s.parse().ok()).unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Rules
// ---------------------------------------------------------------------------

/// `12/03/2026`, `12.03.2026`, `12-03-2026`, `12/3/26`.
struct NumericDateRule {
    re: Regex,
}

impl NumericDateRule {
    fn new() -> Self {
        Self {
            re: Regex::new(&format!(
                r"{BEFORE}(\d{{1,2}})([./-])(\d{{1,2}})\2(\d{{4}}|\d{{2}}){AFTER}"
            ))
            .expect("date-numeric"),
        }
    }
}

impl Rule for NumericDateRule {
    fn name(&self) -> &'static str {
        "date-numeric"
    }

    fn priority(&self) -> i32 {
        priority::DATE
    }

    fn pattern(&self) -> &Regex {
        &self.re
    }

    fn render(&self, m: &Captures<'_>, _text: &str, cfg: &Config) -> Option<String> {
        let year = expand_year(capture(m, 4)?);
        match resolve(number(m, 1), number(m, 3), year, cfg) {
            // Date-shaped but impossible: keep it, don't let it be read.
            None => Some(capture(m, 0)?.to_string()),
            Some((day, month)) => Some(spell(Some(day), month, Some(year))),
        }
    }
}

/// `2026-03-12`. Unambiguous, so `date_order` does not apply.
struct IsoDateRule {
    re: Regex,
}

impl IsoDateRule {
    fn new() -> Self {
        Self {
            re: Regex::new(&format!(r"{BEFORE}(\d{{4}})-(\d{{2}})-(\d{{2}}){AFTER}"))
                .expect("date-iso"),
        }
    }
}

impl Rule for IsoDateRule {
    fn name(&self) -> &'static str {
        "date-iso"
    }

    fn priority(&self) -> i32 {
        priority::DATE
    }

    fn pattern(&self) -> &Regex {
        &self.re
    }

    fn render(&self, m: &Captures<'_>, _text: &str, _cfg: &Config) -> Option<String> {
        let (year, month, day) = (number(m, 1), number(m, 2), number(m, 3));
        if !is_valid(day, month, year) {
            return Some(capture(m, 0)?.to_string());
        }
        Some(spell(Some(day), month, Some(year)))
    }
}

/// `12 במרץ 2026` and the year-less `12 במרץ`.
struct TextualDateRule {
    re: Regex,
}

impl TextualDateRule {
    fn new() -> Self {
        let months = longest_first(MONTH_NUMBERS.iter().map(|(n, _)| *n).collect());
        Self {
            re: Regex::new(&format!(
                r"{BEFORE}(\d{{1,2}})\s+ב[־-]?({months})(?:\s+(\d{{4}})(?!\d))?"
            ))
            .expect("date-textual"),
        }
    }
}

impl Rule for TextualDateRule {
    fn name(&self) -> &'static str {
        "date-textual"
    }

    fn priority(&self) -> i32 {
        priority::DATE
    }

    fn pattern(&self) -> &Regex {
        &self.re
    }

    fn render(&self, m: &Captures<'_>, _text: &str, _cfg: &Config) -> Option<String> {
        let day = number(m, 1);
        let month = month_number(capture(m, 2)?)?;
        let year = capture(m, 3).and_then(|s| s.parse::<i64>().ok());
        // No year means no leap-year question; a February 29th still has to be allowed.
        if !is_valid(day, month, year.unwrap_or(2024)) {
            return Some(capture(m, 0)?.to_string());
        }
        Some(spell(Some(day), month, year))
    }
}

/// A bare `3/2026` is month/year — never a day, in either order.
struct MonthYearRule {
    re: Regex,
}

impl MonthYearRule {
    fn new() -> Self {
        Self {
            re: Regex::new(&format!(r"{BEFORE}(\d{{1,2}})/(\d{{4}}){AFTER}"))
                .expect("date-month-year"),
        }
    }
}

impl Rule for MonthYearRule {
    fn name(&self) -> &'static str {
        "date-month-year"
    }

    fn priority(&self) -> i32 {
        priority::DATE
    }

    fn pattern(&self) -> &Regex {
        &self.re
    }

    fn render(&self, m: &Captures<'_>, _text: &str, _cfg: &Config) -> Option<String> {
        let (month, year) = (number(m, 1), number(m, 2));
        if !(1..=12).contains(&month) {
            return Some(capture(m, 0)?.to_string());
        }
        Some(spell(None, month, Some(year)))
    }
}

/// `כ״ז באלול`, `א׳ בתשרי`, `ט״ו בשבט`, `י״ד בתשרי תשפ״ו`.
struct HebrewDateRule {
    re: Regex,
}

impl HebrewDateRule {
    fn new() -> Self {
        // A Hebrew numeral: two letters around a gershayim (י״ד, ט״ו) or one with a
        // geresh (א׳).
        let day = r"(?:[א-ת]״[א-ת]|[א-ת]׳)";
        // Months plus their defective spellings, and the Omer, which counts days the
        // same way.
        let mut names: Vec<&str> = HEBREW_MONTHS.to_vec();
        names.extend(HEBREW_MONTH_VARIANTS.iter().map(|(v, _)| *v));
        names.push(OMER);
        let months = longest_first(names);
        // A Hebrew year: תשפ״ו, optionally with the millennium ה׳. Read as the word it is.
        let year = r"(?:ה[׳'])?[א-ת]{1,3}״[א-ת]";
        let pattern = format!(r"(?<![א-ת])(?P<day>{day})\s+(?P<prep>[בל]?)(?P<month>{months})")
            + &format!(r"(?![א-ת])(?:\s+(?P<year>{year}))?");
        Self {
            re: Regex::new(&pattern).expect("date-hebrew"),
        }
    }
}

impl Rule for HebrewDateRule {
    fn name(&self) -> &'static str {
        "date-hebrew"
    }

    fn priority(&self) -> i32 {
        priority::DATE
    }

    fn pattern(&self) -> &Regex {
        &self.re
    }

    fn render(&self, m: &Captures<'_>, _text: &str, cfg: &Config) -> Option<String> {
        let raw_day = m.name("day")?.as_str();
        let day = gematria(raw_day);
        let month = hebrew_month_canonical(m.name("month")?.as_str());
        let limit = if month == OMER { 49 } else { 30 };
        if !(1..=limit).contains(&day) {
            // Numeral-shaped but not a day: leave it alone.
            return Some(m.get(0)?.as_str().to_string());
        }

        let prep = match m.name("prep").map(|g| g.as_str()).unwrap_or("") {
            "" => "ב",
            other => other,
        };
        let mut spoken = if let Some(named) = hebrew_date_name(day, month) {
            // A fixed name beats both styles: ט״ו בשבט is /tu bishvat/ and ט׳ באב is
            // תשעה באב, whatever the config says.
            named.to_string()
        } else if cfg.hebrew_date_style == HebrewDateStyle::Letters {
            format!("{} {prep}{month}", split_letters(raw_day))
        } else {
            // The day of a Hebrew month is masculine, as with a Gregorian date.
            format!("{} {prep}{month}", numeral(day, Gender::Masc, false))
        };
        if let Some(year) = m.name("year") {
            // A Hebrew year is said as a word (תשפ״ו is /tashpav/), so drop the marks
            // and let the g2p read what is left.
            let bare: String = year
                .as_str()
                .chars()
                .filter(|c| !matches!(c, '״' | '׳' | '\''))
                .collect();
            spoken.push(' ');
            spoken.push_str(&bare);
        }
        Some(spoken)
    }
}

pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(NumericDateRule::new()),
        Box::new(IsoDateRule::new()),
        Box::new(TextualDateRule::new()),
        Box::new(MonthYearRule::new()),
        Box::new(HebrewDateRule::new()),
    ]
}
