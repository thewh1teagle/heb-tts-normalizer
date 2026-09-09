//! Reading-style options. Everything the caller might reasonably disagree about.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// How to read a time of day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Clock {
    /// `14:30` -> שתיים וחצי אחר הצהריים
    #[default]
    #[serde(rename = "12")]
    H12,
    /// `14:30` -> ארבע עשרה שלושים
    #[serde(rename = "24")]
    H24,
}

/// How to disambiguate `03/04/2026`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DateOrder {
    /// Israeli convention.
    #[default]
    Dmy,
    Mdy,
}

/// How to read the gematria in a Hebrew-calendar date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum HebrewDateStyle {
    /// `כ״ז באלול` -> `כ׳ ז׳ באלול`, left for the g2p to voice.
    #[default]
    Letters,
    /// `כ״ז באלול` -> עשרים ושבעה באלול
    Numbers,
}

/// Grammatical gender of a counted noun.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Gender {
    #[serde(rename = "m")]
    Masc,
    #[serde(rename = "f")]
    Fem,
}

/// Reading style. The defaults are the ones an Israeli speaker would use.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub clock: Clock,
    pub date_order: DateOrder,
    /// Fixed names (ט״ו בשבט, תשעה באב) ignore this — they are names, not readings.
    pub hebrew_date_style: HebrewDateStyle,

    /// Currency assumed for a bare amount that is clearly money but carries no symbol.
    pub default_currency: String,
    /// Read agorot/cents after the major unit.
    pub read_minor_currency: bool,

    /// Word between the integer and fractional part of a decimal.
    pub decimal_word: String,

    /// Spell out unit abbreviations. Off leaves them untouched.
    pub expand_units: bool,
    /// Expand common abbreviations (`ד״ר` -> דוקטור).
    pub expand_abbreviations: bool,

    /// Drop Markdown syntax, keeping the text it wraps.
    pub strip_markdown: bool,
    /// Collapse whitespace and remove zero-width / bidi control characters.
    pub clean_whitespace: bool,

    /// Extra noun genders, consulted before the built-in lexicon and the heuristics.
    /// Also makes a word count as a noun, which is the escape hatch for jargon.
    pub gender_overrides: HashMap<String, Gender>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            clock: Clock::default(),
            date_order: DateOrder::default(),
            hebrew_date_style: HebrewDateStyle::default(),
            default_currency: "ILS".to_string(),
            read_minor_currency: true,
            decimal_word: "נקודה".to_string(),
            expand_units: true,
            expand_abbreviations: true,
            strip_markdown: true,
            clean_whitespace: true,
            gender_overrides: HashMap::new(),
        }
    }
}

impl Config {
    /// Parse a config from JSON. Unknown keys are an error, so a typo is loud.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        if json.trim().is_empty() {
            return Ok(Self::default());
        }
        serde_json::from_str(json)
    }
}
