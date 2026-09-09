//! Bare numbers written in digits: counts, decimals, thousands separators, signs, ordinals.
//!
//! Covered here, from most specific to least:
//!
//! - `1,234` — comma is a thousands separator, so this is one number. A bare `1.234`
//!   is *not*: in Hebrew typography the dot is the decimal point, so `1.234` reads as
//!   "אחת נקודה שתיים שלוש ארבע". There is no comma-decimal form to support.
//! - `3.5` — decimals, fraction digits read one by one ("שלוש נקודה חמש").
//! - `-5` / `+5` — a leading sign reads as מינוס / פלוס.
//! - `3 ילדים` — a plain integer agrees with the noun that follows it, which means the
//!   match has to *include* the noun (`count_phrase` consumes it and orders it
//!   correctly: `1 ילד` becomes "ילד אחד", `3 הילדים` becomes "שלושת הילדים"). With
//!   nothing to agree with, a bare number is read in the feminine — that is how Hebrew
//!   counts in the abstract.
//! - `ה-3` — an ordinal, at its own priority so it beats the cardinal reading.
//! - `ב-5` / `ל-3` — a one-letter prefix glued to a number with a hyphen. This rule eats
//!   *only* the prefix and the hyphen, handing the digits straight back to the scanner,
//!   so currency and units still get their shot: "ו-25₪" comes out
//!   "ועשרים וחמישה שקלים".
//!
//! Judgment calls:
//!
//! - **What counts as a countable noun.** Anything else would need a lexicon we do not
//!   have, so the test is deliberately crude and conservative: a Hebrew word of two or
//!   more letters that is not in [`STOPWORDS`] (particles, conjunctions, pronouns and
//!   the handful of adverbs that plausibly follow a number). "3 עד 5" must not become
//!   "שלושה עד 5", and a wrong *decline* only costs us the feminine default, while a
//!   wrong *accept* mangles the sentence — so when in doubt the rule declines.
//! - **`ה-3` is masculine.** "ה-3" is read השלישי, not השלישית; without knowing the
//!   elided noun there is no better guess, and the masculine is the citation form.
//! - **Only `ה` + hyphen + digits is an ordinal.** "פרק 3" stays a cardinal
//!   ("פרק שלוש"), because Hebrew writes an ordinal after a noun that way just as often
//!   as a cardinal.
//! - **Left alone on purpose:** leading zeros (`007` is a phone/serial matter, not
//!   ours), digit groups the higher-priority rules already own, and `1,5` — a comma
//!   decimal is not Hebrew usage, so it stays two numbers around a comma.

use fancy_regex::{Captures, Regex};

use crate::config::{Config, Gender};
use crate::numerals::words::LEXICON;
use crate::numerals::{count_phrase, decimal, numeral, ordinal};
use crate::scanner::{Rule, priority};

/// Hebrew letters only — `text::HEB` also spans niqqud and punctuation, which a
/// countable noun never contains once `clean` has run.
const LETTER: &str = "א-ת";

/// ASCII hyphen, non-breaking hyphen, maqaf, en dash, em dash.
const HYPHEN: &str = "\\-\u{2011}\u{05be}\u{2013}\u{2014}";

/// Words that may follow a number without being counted by it. Particles, conjunctions,
/// pronouns and comparatives — the things that would otherwise be mistaken for a noun.
#[rustfmt::skip]
const STOPWORDS: &[&str] = &[
    "של", "את", "עד", "או", "אבל", "כי", "גם", "לא", "הוא", "היא", "הם", "הן", "זה", "זו",
    "אם", "כדי", "מתוך", "על", "אל", "מן", "כמו", "יש", "אין", "היה", "היו", "רק", "כבר", "אך",
    "אז", "לפני", "אחרי", "ועד", "בלבד", "בערך", "יותר", "פחות", "מעל", "מתחת", "כולל", "כמעט",
    "אולי", "אפילו", "אשר", "כדאי", "צריך", "אפשר",
];

/// A number, optionally with thousands separators, optionally with a decimal part.
/// The grouped alternative comes first so `1,234` is never read as a bare `1`.
const NUMBER: &str = r"(?P<num>\d{1,3}(?:,\d{3})+|\d+)(?:\.(?P<frac>\d+))?";
const INTEGER: &str = r"(?P<num>\d{1,3}(?:,\d{3})+|\d+)";

/// Never start mid-number, and never read one limb of a date or a time. If the dates
/// rule declined `32/13/2026` it is malformed, not three numbers, so these guards keep
/// the whole thing in digits rather than reading half of it aloud.
/// The last clause refuses a lone thousands group. In the map scale `1:250,000` the
/// `250` is declined for its colon, and the scan then restarts at `000` — which on its
/// own is a perfectly good number, so the scale came out as `1:250,אפס`, half words and
/// half digits.
///
/// It looks only at the comma and at the digits themselves, never at the text on either
/// side. Anything else — the digit before the comma, the punctuation after the group —
/// is something an earlier rule or `finalize` may already have rewritten, and the same
/// input would then read differently on a second pass. Only an exact three-digit group
/// is refused, so the comma decimal `1,5` still reads אחת,חמש.
const BEFORE: &str = r"(?<!\d)(?<![:/])(?!(?<=,)\d{3}(?!\d))";
const AFTER: &str = r"(?!\d)(?![:/]\d)";

/// Plural morphology. A counted Hebrew noun is almost always plural ("שלושה ילדים"),
/// so requiring it is what separates a real count from a number that merely happens to
/// sit before a word — "כביש 6 סמוך", "הרצל 25 תל אביב", "פי 3 בעשור".
const PLURAL_SUFFIXES: [&str; 2] = ["ים", "ות"];

/// Singular nouns that are counted anyway. Scale words are the whole list: "2 מיליון"
/// is a count, and there is no plural to look for.
const COUNTABLE_SINGULARS: [&str; 5] = ["מיליון", "מיליארד", "טריליון", "אלף", "אלפים"];

/// "פי 3" is a fixed idiom and always masculine — "פי שלושה", never "פי שלוש". Without
/// this it would agree with whatever word happened to follow.
const TIMES_IDIOM: &str = "פי";

/// Whether `word` is plausibly a noun that `n` counts.
///
/// Deliberately strict. Getting this wrong does not merely pick the wrong gender: for
/// `n == 1` the numeral moves behind its noun, so a false positive reorders the
/// sentence. A false negative only costs the bare feminine form, which is the correct
/// reading for a number that isn't counting anything.
fn is_countable(word: &str, n: i64, cfg: &Config) -> bool {
    let len = word.chars().count();
    if len < 2 || STOPWORDS.contains(&word) {
        return false;
    }
    if cfg.gender_overrides.contains_key(word) {
        // The caller named it and gave it a gender, so it is a noun by declaration.
        // This is the escape hatch for jargon and loanwords no lexicon will carry.
        return true;
    }
    if COUNTABLE_SINGULARS.contains(&word) {
        return true;
    }
    if n == 1 {
        // The reordering case: only a noun we actually know about earns it.
        return LEXICON.contains_key(word);
    }
    // A construct plural ends in ־י, not ־ים: "61 חברי כנסת", "3 בתי ספר".
    PLURAL_SUFFIXES.iter().any(|s| word.ends_with(s)) || (len >= 3 && word.ends_with('י'))
}

/// Spoken form of one numeric token, decimal or not.
fn read(num: &str, frac: Option<&str>, cfg: &Config, gender: Gender) -> String {
    let whole: i64 = match num.replace(',', "").parse() {
        Ok(v) => v,
        Err(_) => return num.to_string(),
    };
    match frac {
        None => numeral(whole, gender, false),
        Some(f) => decimal(whole, f, cfg, gender),
    }
}

fn is_hebrew_letter(c: char) -> bool {
    ('\u{05D0}'..='\u{05EA}').contains(&c)
}

/// Whether `word` is the token immediately before the match at `start`.
///
/// The Python reference does this with a `([א-ת]+)\s+$` search over the prefix; the
/// backwards scan is the same thing, and avoids a variable-width lookbehind in the hot
/// pattern.
fn preceded_by(text: &str, start: usize, word: &str) -> bool {
    let prefix = &text[..start];
    let trimmed = prefix.trim_end_matches(char::is_whitespace);
    if trimmed.len() == prefix.len() {
        return false; // no whitespace between: not a separate token
    }
    let letters: String = trimmed
        .chars()
        .rev()
        .take_while(|c| is_hebrew_letter(*c))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    letters == word
}

/// `3 ילדים` — an integer agreeing with the noun it counts.
struct CountRule {
    pattern: Regex,
}

impl CountRule {
    fn new() -> Self {
        let noun = format!("(?P<noun>[{LETTER}]{{2,}})(?![{LETTER}])");
        Self {
            pattern: Regex::new(&format!(r"{BEFORE}{INTEGER}(?!\.?\d){AFTER}[ ]{noun}"))
                .expect("count pattern"),
        }
    }
}

impl Rule for CountRule {
    fn name(&self) -> &'static str {
        "count"
    }

    fn priority(&self) -> i32 {
        priority::NUMBER
    }

    fn pattern(&self) -> &Regex {
        &self.pattern
    }

    fn render(&self, m: &Captures<'_>, text: &str, cfg: &Config) -> Option<String> {
        let noun = m.name("noun")?.as_str();
        let n: i64 = m.name("num")?.as_str().replace(',', "").parse().ok()?;
        let start = m.get(0)?.start();
        if preceded_by(text, start, TIMES_IDIOM) {
            return Some(format!("{} {noun}", numeral(n, Gender::Masc, false)));
        }
        if !is_countable(noun, n, cfg) {
            return None;
        }
        Some(count_phrase(n, noun, cfg))
    }
}

/// Any other number: bare, decimal, grouped, or signed.
struct NumberRule {
    pattern: Regex,
}

impl NumberRule {
    fn new() -> Self {
        Self {
            pattern: Regex::new(&format!(r"{BEFORE}(?P<sign>[-+])?{NUMBER}{AFTER}"))
                .expect("number pattern"),
        }
    }
}

impl Rule for NumberRule {
    fn name(&self) -> &'static str {
        "number"
    }

    fn priority(&self) -> i32 {
        priority::NUMBER
    }

    fn pattern(&self) -> &Regex {
        &self.pattern
    }

    fn render(&self, m: &Captures<'_>, text: &str, cfg: &Config) -> Option<String> {
        let start = m.get(0)?.start();
        let sign = m.name("sign").map(|s| s.as_str());
        if sign.is_some() {
            // "ב-5" is a prefixed number, not minus five; the hyphen belongs to the letter.
            if let Some(before) = text[..start].chars().next_back()
                && (is_hebrew_letter(before) || HYPHEN.contains(before))
            {
                return None;
            }
        }
        // "פי 3" is masculine whether or not a noun follows, so the idiom is checked
        // here as well as in CountRule — at the end of a sentence only this rule runs.
        let gender = if preceded_by(text, start, TIMES_IDIOM) {
            Gender::Masc
        } else {
            Gender::Fem
        };
        let spoken = read(
            m.name("num")?.as_str(),
            m.name("frac").map(|f| f.as_str()),
            cfg,
            gender,
        );
        Some(match sign {
            Some("-") => format!("מינוס {spoken}"),
            Some("+") => format!("פלוס {spoken}"),
            _ => spoken,
        })
    }
}

/// `ה-3` / `ה־3` — the one spelling that is unambiguously an ordinal.
struct OrdinalRule {
    pattern: Regex,
}

impl OrdinalRule {
    fn new() -> Self {
        Self {
            pattern: Regex::new(&format!(r"ה[{HYPHEN}](?P<num>\d{{1,3}}){AFTER}"))
                .expect("ordinal pattern"),
        }
    }
}

impl Rule for OrdinalRule {
    fn name(&self) -> &'static str {
        "ordinal"
    }

    fn priority(&self) -> i32 {
        priority::ORDINAL
    }

    fn pattern(&self) -> &Regex {
        &self.pattern
    }

    fn render(&self, m: &Captures<'_>, _text: &str, _cfg: &Config) -> Option<String> {
        let n: i64 = m.name("num")?.as_str().parse().ok()?;
        Some(ordinal(n, Gender::Masc, true))
    }
}

/// `ב-5`, `ל-3`, `ו-25₪` — swallow the hyphen, leave the digits to the scanner.
///
/// Consuming only the prefix keeps the number available to currency, units and the
/// range rules, all of which key off the digits themselves.
struct PrefixRule {
    pattern: Regex,
}

impl PrefixRule {
    fn new() -> Self {
        // Two-letter prefixes ("וב-", "מל-") are allowed; anything ending in ה is not,
        // so "מה-3" still reaches the ordinal rule as "מ" + "ה-3".
        //
        // `של` is spelled out of it. It is the one real word the two-letter class can
        // make, and swallowing its hyphen turns `של-18 מעלות` into `שלשמונה עשרה` — a
        // word that does not exist, with the minus sign gone.
        Self {
            pattern: Regex::new(&format!(
                r"(?<![{LETTER}])(?P<pre>[בוכלמש]{{1,2}})(?<!של)[{HYPHEN}](?=[\d₪$€£])"
            ))
            .expect("prefix pattern"),
        }
    }
}

impl Rule for PrefixRule {
    fn name(&self) -> &'static str {
        "number-prefix"
    }

    fn priority(&self) -> i32 {
        priority::NUMBER
    }

    fn pattern(&self) -> &Regex {
        &self.pattern
    }

    fn render(&self, m: &Captures<'_>, _text: &str, _cfg: &Config) -> Option<String> {
        Some(m.name("pre")?.as_str().to_string())
    }
}

/// The four rules of this module, in the Python reference's order.
pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(CountRule::new()),
        Box::new(NumberRule::new()),
        Box::new(OrdinalRule::new()),
        Box::new(PrefixRule::new()),
    ]
}
