//! The same text read several ways.
//!
//! Reading style is a property of the caller, not of the text: a news bulletin wants a
//! 24-hour clock, a voice assistant wants "שתיים וחצי".
//!
//! ```text
//! cargo run --example reading_style
//! ```

use std::collections::HashMap;

use heb_tts_normalizer::{Clock, Config, DateOrder, Gender, HebrewDateStyle, normalize};

const TEXT: &str = "הטיסה ב-14:30 בתאריך 03/04/2026, והמשקל 2 ק״ג";

fn show(label: &str, text: &str, cfg: &Config) {
    println!("{label}:");
    println!("  {}\n", normalize(text, cfg));
}

fn main() {
    show("default (12-hour, day/month)", TEXT, &Config::default());

    show(
        "24-hour clock, month/day",
        TEXT,
        &Config {
            clock: Clock::H24,
            date_order: DateOrder::Mdy,
            ..Config::default()
        },
    );

    show(
        "units left alone, for a pipeline that expands them itself",
        TEXT,
        &Config {
            expand_units: false,
            ..Config::default()
        },
    );

    // A Hebrew-calendar date is read letter by letter by default, which leaves the
    // gematria to the g2p. Ask for numbers and it is spoken as a count instead.
    let hebrew_date = "הטקס יתקיים כ״ז באלול";
    show(
        "Hebrew date, default letters",
        hebrew_date,
        &Config::default(),
    );
    show(
        "Hebrew date as numbers",
        hebrew_date,
        &Config {
            hebrew_date_style: HebrewDateStyle::Numbers,
            ..Config::default()
        },
    );

    // Gender agreement comes from a lexicon plus morphology, and neither can know a
    // loanword or a term of art. Declaring one makes it countable and fixes its
    // agreement.
    let loanword = "3 סטוריז";
    show("a word it cannot know", loanword, &Config::default());
    show(
        "the same word, taught",
        loanword,
        &Config {
            gender_overrides: HashMap::from([("סטוריז".to_string(), Gender::Masc)]),
            ..Config::default()
        },
    );
}
