//! Numeric ranges: `3-5`, `3–5 ק״מ`, `2020-2024`.
//!
//! A range is read with עד between its ends: "3-5" is "שלוש עד חמש". Two things make it
//! more than a search-and-replace on the dash:
//!
//! - **A range that carries a unit or a noun is one span.** "3-5 ק״מ" has to come out
//!   "שלושה עד חמישה קילומטרים" — both numerals take the *unit's* gender, so the unit
//!   cannot be left for the units rule to pick up afterwards. That is why this rule sits
//!   above [`priority::UNIT`]: it claims the whole phrase, unit included.
//! - **The dash is not a minus.** "-5" alone is מינוס חמש and belongs to the numbers
//!   rule; the pattern here therefore demands digits on *both* sides of the dash.
//!
//! Judgment calls:
//!
//! - **Order is not checked.** "5-3" is still read as a range; refusing it would leave
//!   the digits to be read separately around a stray dash, which is worse than reading
//!   it.
//! - **An unrecognised tail is left alone.** If the word after the range is a stopword,
//!   or a unit abbreviation we cannot expand (`expand_units=false`, or one missing from
//!   the lexicon), the tail rule declines and the bare range matches instead — the word
//!   survives untouched for whatever rule wants it. The cost is that with
//!   `expand_units=false` the numerals fall back to the feminine, since the unit's
//!   gender is exactly what we chose not to look up.
//! - **Years need nothing special:** 2020-2024 is just two cardinals, "אלפיים ועשרים עד
//!   אלפיים עשרים וארבע".
//! - Decimal ends are supported ("2.5-3.5"); fraction digits are read one by one as
//!   usual.

use fancy_regex::{Captures, Regex};

use crate::config::{Config, Gender};
use crate::numerals::{decimal, noun_gender, numeral};
use crate::rules::units::unit_for;
use crate::scanner::{Rule, priority};

const LETTER: &str = "א-ת";
const HYPHEN: &str = "\\-\u{2011}\u{05be}\u{2013}\u{2014}";

/// Words that may follow a range without belonging to it.
#[rustfmt::skip]
const STOPWORDS: &[&str] = &[
    "של", "את", "או", "עד", "אבל", "כי", "גם", "לא", "הוא", "היא", "זה", "אם", "מתוך", "על",
    "אל", "יותר", "פחות", "בלבד", "בערך", "לפחות", "כמעט", "מהם", "לכל",
];

const END: &str = r"\d+(?:\.\d+)?";

/// Digits on *both* sides — that is the whole difference between a range and a minus
/// sign. The guards keep us out of the middle of a date or a time the way the numbers
/// rule does; a trailing full stop is fine, a trailing digit is not.
fn range_pattern() -> String {
    format!(r"(?<!\d)(?<![:/])(?P<a>{END})[ ]?[{HYPHEN}][ ]?(?P<b>{END})(?!\d)(?![:/]\d)")
}

/// One end of the range, decimal or integer.
fn read(token: &str, gender: Gender, cfg: &Config) -> String {
    match token.split_once('.') {
        Some((whole, frac)) if !frac.is_empty() => match whole.parse::<i64>() {
            Ok(w) => decimal(w, frac, cfg, gender),
            Err(_) => token.to_string(),
        },
        _ => match token.trim_end_matches('.').parse::<i64>() {
            Ok(n) => numeral(n, gender, false),
            Err(_) => token.to_string(),
        },
    }
}

fn span(m: &Captures<'_>, gender: Gender, cfg: &Config) -> Option<String> {
    let a = read(m.name("a")?.as_str(), gender, cfg);
    let b = read(m.name("b")?.as_str(), gender, cfg);
    Some(format!("{a} עד {b}"))
}

/// `3-5 ק״מ` / `3-5 ילדים` — the range and what it measures, agreeing.
struct RangeWithTailRule {
    pattern: Regex,
}

impl RangeWithTailRule {
    fn new() -> Self {
        // A unit abbreviation ("ק״מ", "מ׳") or a plain noun.
        let tail = format!(r"(?P<tail>[{LETTER}]+(?:[״׳][{LETTER}]*)?)(?![{LETTER}])");
        Self {
            pattern: Regex::new(&format!("{}[ ]{tail}", range_pattern()))
                .expect("range-with-tail pattern"),
        }
    }
}

impl Rule for RangeWithTailRule {
    fn name(&self) -> &'static str {
        "range-with-tail"
    }

    fn priority(&self) -> i32 {
        priority::RANGE
    }

    fn pattern(&self) -> &Regex {
        &self.pattern
    }

    fn render(&self, m: &Captures<'_>, _text: &str, cfg: &Config) -> Option<String> {
        let tail = m.name("tail")?.as_str();
        if let Some(unit) = unit_for(tail) {
            if !cfg.expand_units {
                return None; // leave the abbreviation alone; the bare range still reads
            }
            return Some(format!("{} {}", span(m, unit.gender, cfg)?, unit.plural));
        }
        // An abbreviation we do not know is not a noun either — hand the span back.
        if tail.contains('״')
            || tail.contains('׳')
            || tail.chars().count() < 2
            || STOPWORDS.contains(&tail)
        {
            return None;
        }
        Some(format!("{} {tail}", span(m, noun_gender(tail, cfg), cfg)?))
    }
}

/// `3-5` on its own, read in the feminine like any uncounted number.
struct RangeRule {
    pattern: Regex,
}

impl RangeRule {
    fn new() -> Self {
        Self {
            pattern: Regex::new(&range_pattern()).expect("range pattern"),
        }
    }
}

impl Rule for RangeRule {
    fn name(&self) -> &'static str {
        "range"
    }

    fn priority(&self) -> i32 {
        priority::RANGE
    }

    fn pattern(&self) -> &Regex {
        &self.pattern
    }

    fn render(&self, m: &Captures<'_>, _text: &str, cfg: &Config) -> Option<String> {
        span(m, Gender::Fem, cfg)
    }
}

pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(RangeWithTailRule::new()),
        Box::new(RangeRule::new()),
    ]
}
