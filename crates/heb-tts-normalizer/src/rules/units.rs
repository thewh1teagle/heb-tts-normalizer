//! Numbers followed by a unit of measure: `5 ק״מ` -> חמישה קילומטרים.
//!
//! The module carries both the unit lexicon and the two rules that read it, because
//! the lexicon is small and only ever used here and by the ranges rule.
//!
//! # The lexicon
//!
//! Gender is the whole reason the table exists. Hebrew numerals agree with the noun
//! they count, and the unit *is* the noun: `5 ק״מ` is חמישה קילומטרים (masculine)
//! while `5 דקות` is חמש דקות (feminine). Every unit is registered under every
//! spelling it plausibly appears in — the Hebrew abbreviation with its
//! gershayim/geresh (`ק״מ`, `מ׳`), the word in both numbers, and the Latin symbol
//! (`km`) — all mapping to the same [`Unit`].
//!
//! [`crate::text::clean`] has already turned ASCII `"`/`'` between Hebrew letters
//! into `״`/`׳`, so the keys use the real Hebrew punctuation only.
//!
//! # The rules
//!
//! `unit`
//! : A number plus one of the spellings in the lexicon. The count picks the shape:
//!   `1` follows its noun (קילומטר אחד) because Hebrew treats אחד/אחת as an
//!   adjective; `2` takes the bound form שני/שתי, since שניים cannot stand before a
//!   noun; `0` and `3`+ take an agreeing numeral and the plural.
//!
//!   The decimal row is the judgment call. A fraction is not a plural count —
//!   Hebrew keeps the singular after it, as with a measurement read off an
//!   instrument: "שתיים נקודה חמש קילוגרם", never "…קילוגרמים". The number is read
//!   as a bare quantity, so it takes the feminine (the citation form used for
//!   counting) regardless of the unit's gender.
//!
//! `degree`
//! : The `°` sign, with or without a scale letter. מעלה is feminine. Temperature is
//!   the one place a decimal keeps the *plural* noun — a fever is "שלושים ושמונה
//!   נקודה חמש מעלות", never "…מעלה" — so it does not share the singular rule above.
//!
//! `cfg.expand_units = false` makes `unit` decline, leaving the abbreviation for a
//! caller who prefers it. `degree` ignores the flag on purpose: `°` is a symbol, not
//! a word, and handing it to a g2p unexpanded produces nothing readable.

use std::collections::HashMap;

use fancy_regex::{Captures, Regex};
use once_cell::sync::Lazy;

use crate::config::{Config, Gender};
use crate::numerals::{decimal, numeral};
use crate::scanner::{Rule, priority};
use crate::text::HEB;

/// One unit of measure, in the two forms a Hebrew numeral can demand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unit {
    /// Form used after 1 and after a decimal ("קילוגרם").
    pub singular: &'static str,
    /// Form used from 2 upwards ("קילוגרמים").
    pub plural: &'static str,
    /// Gender the counting numeral must agree with.
    pub gender: Gender,
}

const M: Gender = Gender::Masc;
const F: Gender = Gender::Fem;

/// `(spellings, singular, plural, gender)`. The first spelling is the canonical
/// abbreviation; the rest are alternate ways the same unit gets written.
#[rustfmt::skip]
#[allow(clippy::type_complexity)]
static TABLE: &[(&[&str], &str, &str, Gender)] = &[
    // -- length ---------------------------------------------------------------
    (&["מ״מ", "מילימטר", "מילימטרים", "mm"], "מילימטר", "מילימטרים", M),
    (&["ס״מ", "סנטימטר", "סנטימטרים", "cm"], "סנטימטר", "סנטימטרים", M),
    (&["מ׳", "מטר", "מטרים", "m"], "מטר", "מטרים", M),
    (&["ק״מ", "קילומטר", "קילומטרים", "km"], "קילומטר", "קילומטרים", M),
    // -- weight ---------------------------------------------------------------
    (&["מ״ג", "מיליגרם", "מיליגרמים", "mg"], "מיליגרם", "מיליגרמים", M),
    (&["ג׳", "גרם", "גרמים", "gr", "g"], "גרם", "גרמים", M),
    (&["ק״ג", "קילוגרם", "קילוגרמים", "kg"], "קילוגרם", "קילוגרמים", M),
    // קילו is invariant in speech: "שלושה קילו", never "קילואים".
    (&["קילו"], "קילו", "קילו", M),
    (&["טון", "טונות", "ton"], "טון", "טונות", M),
    // -- volume ---------------------------------------------------------------
    (&["מ״ל", "מיליליטר", "מיליליטרים", "ml"], "מיליליטר", "מיליליטרים", M),
    (&["ליטר", "ליטרים", "lt"], "ליטר", "ליטרים", M),
    // קוב (cubic metre) is invariant too: "חמישה קוב".
    (&["קוב"], "קוב", "קוב", M),
    (&["מ״ק", "מטר מעוקב", "מטרים מעוקבים"], "מטר מעוקב", "מטרים מעוקבים", M),
    // -- time -----------------------------------------------------------------
    (&["שנ׳", "שנייה", "שניה", "שניות"], "שנייה", "שניות", F),
    (&["דק׳", "דקה", "דקות"], "דקה", "דקות", F),
    (&["שע׳", "שעה", "שעות"], "שעה", "שעות", F),
    (&["יום", "ימים"], "יום", "ימים", M),
    (&["שבוע", "שבועות"], "שבוע", "שבועות", M),
    (&["חודש", "חודשים"], "חודש", "חודשים", M),
    // plural of שנה is שנים (not שניים) — the numeral still takes the feminine form.
    (&["שנה", "שנים"], "שנה", "שנים", F),
    // -- data -----------------------------------------------------------------
    (&["ק״ב", "קילובייט", "KB", "kb"], "קילובייט", "קילובייטים", M),
    (&["מ״ב", "מגהבייט", "מגה בייט", "MB", "mb"], "מגהבייט", "מגהבייטים", M),
    (&["ג״ב", "גיגהבייט", "גיגה בייט", "GB", "gb"], "גיגהבייט", "גיגהבייטים", M),
    (&["ט״ב", "טרהבייט", "טרה בייט", "TB", "tb"], "טרהבייט", "טרהבייטים", M),
    // -- speed ----------------------------------------------------------------
    (&["קמ״ש", "קילומטר לשעה", "km/h", "kmh"], "קילומטר לשעה", "קילומטרים לשעה", M),
    // -- temperature ----------------------------------------------------------
    (&["מעלות", "מעלה"], "מעלה", "מעלות", F),
    // -- area -----------------------------------------------------------------
    (&["מ״ר", "מטר רבוע", "מטרים רבועים"], "מטר רבוע", "מטרים רבועים", M),
    (&["ס״ר", "סנטימטר רבוע"], "סנטימטר רבוע", "סנטימטרים רבועים", M),
    (&["ק״ר", "קילומטר רבוע"], "קילומטר רבוע", "קילומטרים רבועים", M),
    (&["דונם", "דונמים"], "דונם", "דונמים", M),
];

/// Every written form of every unit, keyed by the exact string as it appears in text.
static UNITS: Lazy<HashMap<&'static str, Unit>> = Lazy::new(|| {
    let mut map = HashMap::new();
    for (forms, singular, plural, gender) in TABLE {
        for form in *forms {
            map.insert(
                *form,
                Unit {
                    singular,
                    plural,
                    gender: *gender,
                },
            );
        }
    }
    map
});

/// Written forms, longest first — so an alternation prefers `ק״מ` over `מ׳` and
/// `מגה בייט` over `מגה`. Length is counted in characters, not bytes: the Hebrew
/// forms are two bytes per letter and a byte sort would put every Latin symbol last.
static UNIT_FORMS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    let mut forms: Vec<&'static str> = TABLE.iter().flat_map(|(f, ..)| f.iter().copied()).collect();
    // Stable, so equal-length forms keep the table's order.
    forms.sort_by_key(|f| std::cmp::Reverse(f.chars().count()));
    forms
});

/// Temperature scales that may follow a degree sign.
static SCALES: &[(&str, &str)] = &[
    ("C", "צלזיוס"),
    ("c", "צלזיוס"),
    ("צ", "צלזיוס"),
    ("F", "פרנהייט"),
    ("f", "פרנהייט"),
    ("פ", "פרנהייט"),
];

/// Look up a unit by any of its written spellings.
pub fn unit_for(abbrev: &str) -> Option<&'static Unit> {
    UNITS.get(abbrev)
}

/// Every written form, longest first, ready for a regex alternation.
pub fn unit_forms() -> &'static [&'static str] {
    &UNIT_FORMS
}

fn scale_for(letter: &str) -> Option<&'static str> {
    SCALES.iter().find(|(k, _)| *k == letter).map(|(_, v)| *v)
}

/// An integer, optionally with thousands commas, optionally with a decimal part.
const NUMBER: &str = r"\d{1,3}(?:,\d{3})+(?:\.\d+)?|\d+(?:\.\d+)?";

/// An optional leading minus, claimed here so the number rule does not grab `-5` and
/// leave `°C` stranded. A hyphen after a Hebrew letter is the maqaf of a proclitic
/// (`מ-5 ק״מ`, "from 5 km"), never a sign, so the lookbehind rules it out.
fn sign_pattern() -> String {
    format!(r"(?:(?<![{HEB}\d])(?P<sign>[-−]) ?)?")
}

/// A unit must not run into a following letter or digit: `5 מ׳` is metres, `5 מטרו`
/// is not. Latin symbols need the same guard (`5 kg` vs `5 kgs`).
fn tail_pattern() -> String {
    format!(r"(?![{HEB}A-Za-z0-9׳״])")
}

fn unit_alternation() -> String {
    unit_forms()
        .iter()
        .map(|f| fancy_regex::escape(f).into_owned())
        .collect::<Vec<_>>()
        .join("|")
}

/// Numeral and unit, agreeing and in the order Hebrew puts them.
fn say(n: i64, unit: &Unit) -> String {
    if n == 1 {
        return format!("{} {}", unit.singular, numeral(1, unit.gender, false));
    }
    if n == 2 {
        return format!("{} {}", numeral(2, unit.gender, true), unit.plural);
    }
    format!("{} {}", numeral(n, unit.gender, false), unit.plural)
}

fn signed(m: &Captures<'_>, said: String) -> String {
    if m.name("sign").is_some() {
        format!("מינוס {said}")
    } else {
        said
    }
}

struct UnitRule {
    re: Regex,
}

impl UnitRule {
    fn new() -> Self {
        let pattern = format!(
            r"{}(?<![\d.,])(?P<num>{})[ \t]?(?P<unit>{}){}",
            sign_pattern(),
            NUMBER,
            unit_alternation(),
            tail_pattern(),
        );
        Self {
            re: Regex::new(&pattern).expect("unit pattern"),
        }
    }
}

impl Rule for UnitRule {
    fn name(&self) -> &'static str {
        "unit"
    }

    fn priority(&self) -> i32 {
        priority::UNIT
    }

    fn pattern(&self) -> &Regex {
        &self.re
    }

    fn render(&self, m: &Captures<'_>, _text: &str, cfg: &Config) -> Option<String> {
        if !cfg.expand_units {
            return None;
        }
        let unit = unit_for(m.name("unit")?.as_str())?;
        let raw = m.name("num")?.as_str().replace(',', "");
        if let Some((whole, frac)) = raw.split_once('.') {
            let whole: i64 = whole.parse().ok()?;
            // A measured fraction is not a count: singular noun, bare feminine number.
            let said = format!(
                "{} {}",
                decimal(whole, frac, cfg, Gender::Fem),
                unit.singular
            );
            return Some(signed(m, said));
        }
        Some(signed(m, say(raw.parse().ok()?, unit)))
    }
}

struct DegreeRule {
    re: Regex,
}

impl DegreeRule {
    fn new() -> Self {
        let scales = SCALES.iter().map(|(k, _)| *k).collect::<Vec<_>>().join("|");
        let pattern = format!(
            r"{}(?<![\d.,])(?P<num>{})[ \t]?°[ \t]?(?P<scale>{})?(?![{}A-Za-z0-9])",
            sign_pattern(),
            NUMBER,
            scales,
            HEB,
        );
        Self {
            re: Regex::new(&pattern).expect("degree pattern"),
        }
    }
}

impl Rule for DegreeRule {
    fn name(&self) -> &'static str {
        "degree"
    }

    fn priority(&self) -> i32 {
        priority::UNIT
    }

    fn pattern(&self) -> &Regex {
        &self.re
    }

    fn render(&self, m: &Captures<'_>, _text: &str, cfg: &Config) -> Option<String> {
        let raw = m.name("num")?.as_str().replace(',', "");
        let scale = m.name("scale").and_then(|s| scale_for(s.as_str()));
        let said = if let Some((whole, frac)) = raw.split_once('.') {
            let whole: i64 = whole.parse().ok()?;
            // Temperature keeps the plural after a fraction: "…נקודה חמש מעלות".
            format!("{} מעלות", decimal(whole, frac, cfg, Gender::Fem))
        } else {
            say(raw.parse().ok()?, unit_for("מעלות")?)
        };
        let said = signed(m, said);
        Some(match scale {
            Some(scale) => format!("{said} {scale}"),
            None => said,
        })
    }
}

pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![Box::new(UnitRule::new()), Box::new(DegreeRule::new())]
}
