//! Percentages: `12%`, `12.5%`, `12 אחוז`, and the per-mille sign `5‰`.
//!
//! The one thing worth getting right here: **אחוז is masculine**. `12%` is
//! *שנים עשר אחוזים*, not *שתים עשרה אחוזים* — the feminine bare-counting form that a
//! naive digit-to-word pass produces is the usual mistake.
//!
//! פרומיל is a borrowing and is not counted in the plural, so `5‰` is *חמישה פרומיל*.
//!
//! When the noun is already spelled out (`12 אחוז`) it is kept exactly as written and
//! only the digits are converted; picking a number for the writer would be a rewrite,
//! not a normalization. Ranges (`3%-5%`) belong to the ranges rule.

use fancy_regex::{Captures, Regex};

use crate::config::{Config, Gender};
use crate::numerals::{decimal, numeral};
use crate::scanner::{Rule, priority};
use crate::text::HEB;

const AMOUNT: &str = r"\d{1,3}(?:,\d{3})+(?:\.\d+)?|\d+(?:\.\d+)?";
const NOT_NUMBER_BEFORE: &str = r"(?<![\d,.])";

const PERCENT_SINGULAR: &str = "אחוז";
const PERCENT_PLURAL: &str = "אחוזים";
/// Indeclinable, hence one form for every count.
const PERMILLE: &str = "פרומיל";

/// Spelled-out nouns we recognise after the digits, longest first.
const SPELLED: &str = "אחוזים|אחוז|פרומילים|פרומיל";

const MAX_DIGITS: usize = 15;

fn not_word_after() -> String {
    format!(r"(?![{HEB}0-9A-Za-z׳״])")
}

fn phrase(n: i64, noun: &str) -> String {
    if n == 1 {
        // One follows its noun, and אחוז is masculine: אחוז אחד.
        return format!("{noun} אחד");
    }
    if n == 2 {
        return format!("{} {noun}", numeral(2, Gender::Masc, true));
    }
    format!("{} {noun}", numeral(n, Gender::Masc, false))
}

fn render_amount(
    raw: &str,
    noun_written: Option<&str>,
    sign: Option<&str>,
    cfg: &Config,
) -> Option<String> {
    let bare = raw.replace(',', "");
    let (whole_s, frac) = match bare.split_once('.') {
        Some((w, f)) => (w, Some(f)),
        None => (bare.as_str(), None),
    };
    if whole_s.len() > MAX_DIGITS {
        return None;
    }
    let whole: i64 = whole_s.parse().ok()?;

    let noun: &str = match noun_written {
        Some(written) => written,
        None if sign == Some("‰") => PERMILLE,
        None if whole == 1 && frac.is_none() => PERCENT_SINGULAR,
        None => PERCENT_PLURAL,
    };

    Some(match frac {
        Some(frac) => format!("{} {noun}", decimal(whole, frac, cfg, Gender::Masc)),
        None => phrase(whole, noun),
    })
}

struct SignRule {
    re: Regex,
}

impl SignRule {
    fn new() -> Self {
        let pattern = format!(r"{NOT_NUMBER_BEFORE}(?P<amount>{AMOUNT})\s?(?P<sign>[%‰])");
        Self {
            re: Regex::new(&pattern).expect("percent sign pattern"),
        }
    }
}

impl Rule for SignRule {
    fn name(&self) -> &'static str {
        "percent-sign"
    }

    fn priority(&self) -> i32 {
        priority::PERCENT
    }

    fn pattern(&self) -> &Regex {
        &self.re
    }

    fn render(&self, m: &Captures<'_>, _text: &str, cfg: &Config) -> Option<String> {
        render_amount(
            m.name("amount")?.as_str(),
            None,
            m.name("sign").map(|s| s.as_str()),
            cfg,
        )
    }
}

struct WordRule {
    re: Regex,
}

impl WordRule {
    fn new() -> Self {
        let pattern = format!(
            r"{NOT_NUMBER_BEFORE}(?P<amount>{AMOUNT})\s(?P<noun>{SPELLED}){}",
            not_word_after()
        );
        Self {
            re: Regex::new(&pattern).expect("percent word pattern"),
        }
    }
}

impl Rule for WordRule {
    fn name(&self) -> &'static str {
        "percent-word"
    }

    fn priority(&self) -> i32 {
        priority::PERCENT
    }

    fn pattern(&self) -> &Regex {
        &self.re
    }

    fn render(&self, m: &Captures<'_>, _text: &str, cfg: &Config) -> Option<String> {
        render_amount(
            m.name("amount")?.as_str(),
            Some(m.name("noun")?.as_str()),
            None,
            cfg,
        )
    }
}

pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![Box::new(SignRule::new()), Box::new(WordRule::new())]
}
