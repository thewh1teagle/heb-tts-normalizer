//! Amounts of money: `₪25`, `25 ש״ח`, `$1,250.50`, `2 מיליון €`.
//!
//! # The lexicon
//!
//! A currency is two counted nouns (major and minor) plus the set of spellings that
//! select it. The nouns carry their own gender because Hebrew agreement is a property
//! of the word, not of the amount: `25 ₪` is *עשרים וחמישה שקלים* (masculine) while
//! the same 25 agorot is *עשרים וחמש אגורות* (feminine).
//!
//! Indeclinability is a first-class property of a unit rather than a per-currency
//! exception. אירו and פני simply have no plural — *עשרים וחמישה אירו*, never
//! *אירואים* — and any unit may be like that, so the rule never special-cases a code.
//!
//! # The rules
//!
//! Two rules rather than one because the symbol may lead or trail the amount and a
//! single pattern cannot reuse a group name for both positions. They share all the
//! rendering. Only the non-Hebrew spellings may lead: nobody writes *שקל 25*.
//!
//! Judgment calls:
//!
//! * Two fractional digits are money, not a decimal: `25.5` is fifty agorot, not
//!   "point five". Three or more digits after the point is no longer a price, so it
//!   falls back to a plain decimal reading. `cfg.read_minor_currency` turns the money
//!   reading off entirely and every fraction is then read as a decimal.
//! * The amount is glued to the unit's gender, never to a guess from the noun's
//!   shape, because the lexicon already knows it.

use std::collections::HashMap;

use fancy_regex::{Captures, Regex};
use once_cell::sync::Lazy;

use crate::config::{Config, Gender};
use crate::numerals::{decimal, numeral};
use crate::scanner::{Rule, priority};
use crate::text::HEB;

/// One counted noun of a currency (a shekel, an agora, a cent).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unit {
    pub singular: &'static str,
    /// The counting plural, or `None` for an indeclinable noun (אירו, פני).
    pub plural: Option<&'static str>,
    pub gender: Gender,
}

impl Unit {
    /// True when the noun keeps its singular shape after any numeral.
    pub fn indeclinable(&self) -> bool {
        self.plural.is_none()
    }

    /// The shape the noun takes when counting `n` of it.
    pub fn form(&self, n: i64) -> &'static str {
        if self.indeclinable() || n == 1 {
            return self.singular;
        }
        self.plural.unwrap_or(self.singular)
    }
}

/// A currency and every written form that selects it.
#[derive(Debug, Clone, Copy)]
pub struct Currency {
    pub code: &'static str,
    pub major: Unit,
    /// Hundredth part, or `None` where we never read one aloud.
    pub minor: Option<Unit>,
    pub spellings: &'static [&'static str],
}

const fn unit(singular: &'static str, plural: &'static str, gender: Gender) -> Unit {
    Unit {
        singular,
        plural: Some(plural),
        gender,
    }
}

const fn indeclinable(singular: &'static str, gender: Gender) -> Unit {
    Unit {
        singular,
        plural: None,
        gender,
    }
}

static CURRENCIES: &[Currency] = &[
    Currency {
        code: "ILS",
        major: unit("שקל", "שקלים", Gender::Masc),
        minor: Some(unit("אגורה", "אגורות", Gender::Fem)),
        spellings: &["₪", "ש״ח", "ש\"ח", "שקלים", "שקל", "ILS", "NIS"],
    },
    Currency {
        code: "USD",
        major: unit("דולר", "דולרים", Gender::Masc),
        minor: Some(unit("סנט", "סנטים", Gender::Masc)),
        spellings: &["$", "דולרים", "דולר", "USD"],
    },
    Currency {
        code: "EUR",
        // אירו is indeclinable: the plural *אירואים does not exist in speech.
        major: indeclinable("אירו", Gender::Masc),
        minor: Some(unit("סנט", "סנטים", Gender::Masc)),
        spellings: &["€", "אירו", "יורו", "EUR"],
    },
    Currency {
        code: "GBP",
        major: unit("פאונד", "פאונדים", Gender::Masc),
        // פני is borrowed whole and never counted as *פנים.
        minor: Some(indeclinable("פני", Gender::Masc)),
        spellings: &["£", "פאונדים", "פאונד", "שטרלינג", "GBP"],
    },
];

static BY_SPELLING: Lazy<HashMap<&'static str, &'static Currency>> = Lazy::new(|| {
    let mut map = HashMap::new();
    for cur in CURRENCIES {
        for s in cur.spellings {
            map.insert(*s, cur);
        }
    }
    map
});

/// Every spelling, in the order the table lists them.
fn all_spellings() -> Vec<&'static str> {
    CURRENCIES
        .iter()
        .flat_map(|c| c.spellings.iter().copied())
        .collect()
}

/// Regex alternation, longest first so שקלים wins over שקל. Length is in characters,
/// so a two-byte Hebrew letter does not outrank a Latin one.
fn alternation(spellings: &[&str]) -> String {
    let mut sorted: Vec<&str> = spellings.to_vec();
    sorted.sort_by_key(|s| std::cmp::Reverse(s.chars().count()));
    sorted
        .iter()
        .map(|s| fancy_regex::escape(s).into_owned())
        .collect::<Vec<_>>()
        .join("|")
}

fn is_hebrew(s: &str) -> bool {
    s.chars().any(|c| ('\u{0590}'..='\u{05FE}').contains(&c))
}

/// Scale words that stay words in the output: `2 מיליון ₪` -> שני מיליון שקלים.
static SCALES: &[&str] = &["מיליון", "מיליארד", "טריליון"];

/// Optionally comma-grouped, optionally fractional. The comma form is listed first so
/// `1,250` is not truncated to `1`.
const AMOUNT: &str = r"\d{1,3}(?:,\d{3})+(?:\.\d+)?|\d+(?:\.\d+)?";

/// Nothing numeric may touch the amount, so a malformed `1,25 ₪` is left to the
/// number rules instead of being half-eaten here.
const NOT_NUMBER_BEFORE: &str = r"(?<![\d,.])";

/// Beyond this the numeral spelling is noise, not speech.
const MAX_DIGITS: usize = 15;

fn not_word_after() -> String {
    format!(r"(?![{HEB}0-9A-Za-z׳״])")
}

/// `n` of `unit`, agreeing and correctly ordered.
fn count(n: i64, u: &Unit) -> String {
    if n == 1 {
        // One follows its noun: שקל אחד, אגורה אחת.
        return format!("{} {}", u.singular, numeral(1, u.gender, false));
    }
    if n == 2 {
        // Two is always construct before a noun: שני שקלים, שתי אגורות.
        return format!("{} {}", numeral(2, u.gender, true), u.form(2));
    }
    format!("{} {}", numeral(n, u.gender, false), u.form(n))
}

/// `2 מיליון ₪` -> שני מיליון שקלים. The scale word counts, the unit stays plural.
fn scaled(n: i64, scale: &str, u: &Unit) -> String {
    let head = if n == 1 {
        scale.to_string()
    } else if n == 2 {
        format!("{} {}", numeral(2, Gender::Masc, true), scale)
    } else {
        format!("{} {}", numeral(n, Gender::Masc, false), scale)
    };
    format!("{head} {}", u.form(2))
}

fn render_amount(raw: &str, scale: Option<&str>, cur: &Currency, cfg: &Config) -> Option<String> {
    let bare = raw.replace(',', "");
    let (whole_s, frac) = match bare.split_once('.') {
        Some((w, f)) => (w, Some(f)),
        None => (bare.as_str(), None),
    };
    if whole_s.len() > MAX_DIGITS {
        return None;
    }
    let whole: i64 = whole_s.parse().ok()?;
    let major = &cur.major;

    if let Some(scale) = scale {
        return Some(match frac {
            None => scaled(whole, scale, major),
            Some(frac) => format!(
                "{} {scale} {}",
                decimal(whole, frac, cfg, Gender::Masc),
                major.form(2)
            ),
        });
    }

    let frac = match frac {
        None => return Some(count(whole, major)),
        Some(frac) => frac,
    };

    let money = cfg.read_minor_currency && cur.minor.is_some() && frac.chars().count() <= 2;
    if !money {
        // Not a price any more — read it as a number, the unit staying plural.
        return Some(format!(
            "{} {}",
            decimal(whole, frac, cfg, major.gender),
            major.form(2)
        ));
    }

    let minor = cur.minor.as_ref()?;
    let mut padded = frac.to_string();
    while padded.len() < 2 {
        padded.push('0');
    }
    let minor_value: i64 = padded.parse().ok()?;
    if minor_value == 0 {
        return Some(count(whole, major));
    }
    let minor_words = count(minor_value, minor);
    if whole == 0 {
        return Some(minor_words);
    }
    Some(format!("{} ו{minor_words}", count(whole, major)))
}

fn render_match(m: &Captures<'_>, cfg: &Config) -> Option<String> {
    let cur = *BY_SPELLING.get(m.name("cur")?.as_str())?;
    let scale = m.name("scale").map(|s| s.as_str());
    render_amount(m.name("amount")?.as_str(), scale, cur, cfg)
}

struct SuffixRule {
    re: Regex,
}

impl SuffixRule {
    fn new() -> Self {
        let pattern = format!(
            r"{NOT_NUMBER_BEFORE}(?P<amount>{AMOUNT})(?:\s(?P<scale>{}))?\s?(?P<cur>{}){}",
            alternation(SCALES),
            alternation(&all_spellings()),
            not_word_after(),
        );
        Self {
            re: Regex::new(&pattern).expect("currency suffix pattern"),
        }
    }
}

impl Rule for SuffixRule {
    fn name(&self) -> &'static str {
        "currency-suffix"
    }

    fn priority(&self) -> i32 {
        priority::CURRENCY
    }

    fn pattern(&self) -> &Regex {
        &self.re
    }

    fn render(&self, m: &Captures<'_>, _text: &str, cfg: &Config) -> Option<String> {
        render_match(m, cfg)
    }
}

struct PrefixRule {
    re: Regex,
}

impl PrefixRule {
    fn new() -> Self {
        // Hebrew currency names never precede an amount, so they are excluded here.
        let leading: Vec<&str> = all_spellings()
            .into_iter()
            .filter(|s| !is_hebrew(s))
            .collect();
        let pattern = format!(
            r"(?<![0-9A-Za-z])(?P<cur>{})\s?(?P<amount>{AMOUNT})(?:\s(?P<scale>{}))?{}",
            alternation(&leading),
            alternation(SCALES),
            not_word_after(),
        );
        Self {
            re: Regex::new(&pattern).expect("currency prefix pattern"),
        }
    }
}

impl Rule for PrefixRule {
    fn name(&self) -> &'static str {
        "currency-prefix"
    }

    fn priority(&self) -> i32 {
        priority::CURRENCY
    }

    fn pattern(&self) -> &Regex {
        &self.re
    }

    fn render(&self, m: &Captures<'_>, _text: &str, cfg: &Config) -> Option<String> {
        render_match(m, cfg)
    }
}

pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![Box::new(SuffixRule::new()), Box::new(PrefixRule::new())]
}
