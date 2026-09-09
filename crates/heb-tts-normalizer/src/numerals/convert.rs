//! Turn numbers into Hebrew words: cardinals, decimals, digit strings, ordinals.
//!
//! Judgment calls made here:
//!
//! * A cardinal is built as a list of components (milliards, millions, thousands, hundreds,
//!   tens, ones) and the conjunction ו־ is attached to the *last* component whenever there
//!   is more than one — this is what a speaker does: מאה ועשרים, אלף מאתיים שלושים וארבע.
//! * The thousands group changes shape with its count: 1 אלף, 2 אלפיים, 3-10 masculine
//!   construct + אלפים (חמשת אלפים), and 11+ plain masculine numeral + the *singular* אלף
//!   (עשרים אלף) — Hebrew stops pluralizing the unit past ten.
//! * מיליון / מיליארד behave the same way except they have no dual, so 2 takes the bare
//!   construct שני מיליון.
//! * Fraction digits are read one by one in the feminine, the unmarked counting series.

use crate::config::{Config, Gender};
use crate::numerals::words as w;

const MILLIARD: u64 = 1_000_000_000;
const MILLION: u64 = 1_000_000;
const THOUSAND: u64 = 1_000;

/// Cardinal in words.
///
/// `construct = true` gives the bound form used before a definite noun: 2 -> שני/שתי,
/// 3-10 masculine -> שלושת/ארבעת/חמשת… The feminine has no construct above 2, so it
/// falls back to the absolute form, which is what Hebrew actually says (שלוש הילדות).
pub fn numeral(n: i64, gender: Gender, construct: bool) -> String {
    if n < 0 {
        // `unsigned_abs` so i64::MIN does not overflow on negation.
        return format!(
            "{} {}",
            w::MINUS,
            unsigned(n.unsigned_abs(), gender, construct)
        );
    }
    unsigned(n as u64, gender, construct)
}

fn unsigned(n: u64, gender: Gender, construct: bool) -> String {
    // The construct tables only reach 10, so a wider `n` can never have a bound form.
    let bound = if construct && n <= 10 {
        w::construct(gender).get(&(n as i64))
    } else {
        None
    };
    match bound {
        Some(word) => (*word).to_string(),
        None => join(&components(n, gender)),
    }
}

/// `3, "5"` -> `שלוש נקודה חמש`. The fractional part is read digit by digit.
pub fn decimal(whole: i64, frac: &str, cfg: &Config, gender: Gender) -> String {
    format!(
        "{} {} {}",
        numeral(whole, gender, false),
        cfg.decimal_word,
        digits(frac)
    )
}

/// `"050"` -> `אפס חמש אפס`. Leading zeros are kept; non-digits are dropped.
///
/// ASCII digits only. Python's `str.isdigit()` would also accept Arabic-Indic and other
/// Unicode decimal digits, but nothing else in the pipeline can parse those into an
/// integer, so they are dropped here rather than read.
pub fn digits(s: &str) -> String {
    s.chars()
        .filter_map(|ch| ch.to_digit(10))
        .map(|d| w::ONES_FEM[d as usize])
        .collect::<Vec<_>>()
        .join(" ")
}

/// 1-10 -> ראשון..עשירי (feminine ראשונה..עשירית). Above 10 Hebrew uses the cardinal.
pub fn ordinal(n: i64, gender: Gender, definite: bool) -> String {
    let word = w::ordinals(gender)
        .get(&n)
        .map(|s| (*s).to_string())
        .unwrap_or_else(|| numeral(n, gender, false));
    if definite { format!("ה{word}") } else { word }
}

/// The spoken chunks of `n`, largest first, without any conjunction.
fn components(n: u64, gender: Gender) -> Vec<String> {
    if n == 0 {
        return vec![w::ZERO.to_string()];
    }

    let mut parts: Vec<String> = Vec::new();
    let mut rest = n;

    for (value, name) in [(MILLIARD, w::MILLIARD), (MILLION, w::MILLION)] {
        let count = rest / value;
        rest %= value;
        if count > 0 {
            parts.push(big_scale(count, name));
        }
    }

    let count = rest / THOUSAND;
    rest %= THOUSAND;
    if count > 0 {
        parts.push(thousands(count));
    }

    parts.extend(under_thousand(rest, gender));
    parts
}

fn under_thousand(n: u64, gender: Gender) -> Vec<String> {
    if n == 0 {
        return Vec::new();
    }
    let mut parts: Vec<String> = Vec::new();
    let hundreds = n / 100;
    let rest = n % 100;
    if hundreds > 0 {
        parts.push(w::HUNDREDS[hundreds as usize - 1].to_string());
    }
    if (10..20).contains(&rest) {
        parts.push(w::teens(gender)[rest as usize - 10].to_string());
    } else if rest > 0 {
        let tens = rest / 10;
        let ones = rest % 10;
        if tens > 0 {
            parts.push(w::TENS[tens as usize - 2].to_string());
        }
        if ones > 0 {
            parts.push(w::ones(gender)[ones as usize].to_string());
        }
    }
    parts
}

fn thousands(count: u64) -> String {
    if count == 1 {
        return w::THOUSAND.to_string();
    }
    if count == 2 {
        return w::TWO_THOUSAND.to_string();
    }
    if count <= 10 {
        // 3-10 take the masculine construct with the plural unit: חמשת אלפים.
        let bound = w::CONSTRUCT_MASC[&(count as i64)];
        return format!("{} {}", bound, w::THOUSANDS_PLURAL);
    }
    format!(
        "{} {}",
        numeral(count as i64, Gender::Masc, false),
        w::THOUSAND
    )
}

fn big_scale(count: u64, name: &str) -> String {
    if count == 1 {
        return name.to_string();
    }
    if count == 2 {
        return format!("{} {}", w::CONSTRUCT_MASC[&2], name);
    }
    format!("{} {}", numeral(count as i64, Gender::Masc, false), name)
}

/// Space-separate the components, with ו־ on the last one when there are several.
fn join(parts: &[String]) -> String {
    match parts {
        [] => String::new(),
        [one] => one.clone(),
        _ => {
            let (last, head) = parts.split_last().expect("non-empty");
            format!("{} ו{}", head.join(" "), last)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(n: i64) -> String {
        numeral(n, Gender::Fem, false)
    }
    fn m(n: i64) -> String {
        numeral(n, Gender::Masc, false)
    }

    #[test]
    fn feminine_cardinals() {
        assert_eq!(f(0), "אפס");
        assert_eq!(f(1), "אחת");
        assert_eq!(f(2), "שתיים");
        assert_eq!(f(3), "שלוש");
        assert_eq!(f(10), "עשר");
        assert_eq!(f(11), "אחת עשרה");
        assert_eq!(f(12), "שתים עשרה");
        assert_eq!(f(15), "חמש עשרה");
        assert_eq!(f(20), "עשרים");
        assert_eq!(f(25), "עשרים וחמש");
        assert_eq!(f(100), "מאה");
        assert_eq!(f(101), "מאה ואחת");
        assert_eq!(f(200), "מאתיים");
        assert_eq!(f(300), "שלוש מאות");
        assert_eq!(f(999), "תשע מאות תשעים ותשע");
        assert_eq!(f(1000), "אלף");
        assert_eq!(f(2000), "אלפיים");
        assert_eq!(f(3000), "שלושת אלפים");
        assert_eq!(f(10000), "עשרת אלפים");
        assert_eq!(f(11000), "אחד עשר אלף");
        assert_eq!(f(100000), "מאה אלף");
        assert_eq!(f(1234), "אלף מאתיים שלושים וארבע");
        assert_eq!(f(1000000), "מיליון");
        assert_eq!(f(2500000), "שני מיליון וחמש מאות אלף");
    }

    #[test]
    fn masculine_cardinals() {
        assert_eq!(m(0), "אפס");
        assert_eq!(m(1), "אחד");
        assert_eq!(m(2), "שניים");
        assert_eq!(m(3), "שלושה");
        assert_eq!(m(10), "עשרה");
        assert_eq!(m(11), "אחד עשר");
        assert_eq!(m(12), "שנים עשר");
        assert_eq!(m(15), "חמישה עשר");
        assert_eq!(m(20), "עשרים");
        assert_eq!(m(25), "עשרים וחמישה");
        assert_eq!(m(100), "מאה");
        assert_eq!(m(101), "מאה ואחד");
        assert_eq!(m(200), "מאתיים");
        assert_eq!(m(300), "שלוש מאות");
        assert_eq!(m(999), "תשע מאות תשעים ותשעה");
        assert_eq!(m(1000), "אלף");
        assert_eq!(m(2000), "אלפיים");
        assert_eq!(m(3000), "שלושת אלפים");
        assert_eq!(m(10000), "עשרת אלפים");
        assert_eq!(m(11000), "אחד עשר אלף");
        assert_eq!(m(100000), "מאה אלף");
        assert_eq!(m(1234), "אלף מאתיים שלושים וארבעה");
        assert_eq!(m(1000000), "מיליון");
        assert_eq!(m(2500000), "שני מיליון וחמש מאות אלף");
    }

    #[test]
    fn negatives_and_construct() {
        assert_eq!(f(-5), "מינוס חמש");
        assert_eq!(numeral(2, Gender::Fem, true), "שתי");
        assert_eq!(numeral(2, Gender::Masc, true), "שני");
        assert_eq!(numeral(3, Gender::Masc, true), "שלושת");
        // The feminine has no construct above 2, so it falls back to the absolute form.
        assert_eq!(numeral(3, Gender::Fem, true), "שלוש");
        assert_eq!(numeral(11, Gender::Masc, true), "אחד עשר");
    }

    #[test]
    fn digits_and_decimal() {
        assert_eq!(digits("050"), "אפס חמש אפס");
        assert_eq!(digits(""), "");
        assert_eq!(digits("1-2"), "אחת שתיים");
        // ASCII-only: a Unicode decimal digit is dropped, not read.
        assert_eq!(digits("٥7"), "שבע");
        let cfg = Config::default();
        assert_eq!(decimal(3, "5", &cfg, Gender::Fem), "שלוש נקודה חמש");
        assert_eq!(decimal(0, "25", &cfg, Gender::Fem), "אפס נקודה שתיים חמש");
    }

    #[test]
    fn ordinals() {
        assert_eq!(ordinal(1, Gender::Masc, false), "ראשון");
        assert_eq!(ordinal(1, Gender::Fem, false), "ראשונה");
        assert_eq!(ordinal(3, Gender::Fem, true), "השלישית");
        assert_eq!(ordinal(10, Gender::Masc, true), "העשירי");
        // Above ten Hebrew reuses the cardinal.
        assert_eq!(ordinal(11, Gender::Masc, false), "אחד עשר");
        assert_eq!(ordinal(12, Gender::Fem, true), "השתים עשרה");
    }
}
