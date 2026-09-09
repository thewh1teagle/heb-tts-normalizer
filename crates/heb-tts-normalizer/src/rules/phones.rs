//! Telephone-shaped digit runs, read one digit at a time.
//!
//! Covers the Israeli formats a reader actually meets: mobile and landline numbers with
//! or without separators (`050-1234567`, `03-1234567`, `0501234567`), the international
//! `+972` form, star numbers (`*6555`), `1-800`/`1-700` service lines, emergency short
//! codes, national ID numbers, credit-card groups and IP-like dotted quads.
//!
//! # Judgment calls
//!
//! * This rule sits above dates and numbers, so it must be *stingy*. Every pattern here
//!   demands a real phone shape — a known prefix (`0`+area code, `+972`, `1-800`, `*`)
//!   or an explicit grouping. A bare run of seven or nine digits is never claimed; that
//!   way `1234567` still reaches the numbers rule as a quantity.
//! * Separators (`-`, `–`, `.`, space) are silent. Nobody says "מקף" mid-number.
//! * Emergency codes (100/101/102…) and ID numbers are indistinguishable from ordinary
//!   quantities on their own, so they are claimed only when a dialling or identity cue
//!   ("חייגו", "מוקד", "תעודת זהות"…) sits just before them.
//! * Digits are read with [`crate::numerals::digits`], i.e. the feminine forms — that is
//!   how a phone number is dictated in Hebrew regardless of what it refers to.

use fancy_regex::{Captures, Regex};
use once_cell::sync::Lazy;

use crate::config::Config;
use crate::numerals::digits;
use crate::scanner::{Rule, priority};

/// Characters that may sit between groups of a phone number, and are never spoken.
const SEP: &str = r"[-–—. ]";

static NOT_DIGITS: Lazy<Regex> = Lazy::new(|| Regex::new(r"\D+").expect("NOT_DIGITS"));

/// Words that make a bare three-digit code a number to dial rather than a quantity.
static DIAL_CUE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(concat!(
        r"(?:חייג|חייגו|חייגי|חייגה|התקשר|התקשרו|התקשרי|טלפון|טל[׳']?|נייד|פקס|",
        r"מוקד|למוקד|משטרה|למשטרה|מד״א|כבאות|אמבולנס|חירום|שלוחה|מספר)",
        r"[\s:,\-–]*$"
    ))
    .expect("DIAL_CUE")
});

/// Words that make a nine-digit run an identity number rather than a quantity.
static ID_CUE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?:תעודת\s+זהות|מספר\s+זהות|ת״ז|ת\.ז\.?|ח״פ|ח\.פ\.?|ע״ר|ע״מ|עוסק\s+מורשה)[\s:,\-–]*$",
    )
    .expect("ID_CUE")
});

/// How far back to look for a cue, in characters. Long enough for "תעודת זהות", short
/// enough to stay local.
const CUE_WINDOW: usize = 24;

/// Digit-by-digit reading of a phone group, separators dropped.
fn spoken(raw: &str) -> String {
    digits(&NOT_DIGITS.replace_all(raw, ""))
}

/// The last [`CUE_WINDOW`] characters before the match, for a cue lookup.
fn before(text: &str, start: usize) -> &str {
    let head = &text[..start];
    let cut = head
        .char_indices()
        .rev()
        .take(CUE_WINDOW)
        .last()
        .map_or(head.len(), |(i, _)| i);
    &head[cut..]
}

fn has_cue(m: &Captures<'_>, text: &str, cue: &Regex) -> bool {
    let start = match m.get(0) {
        Some(g) => g.start(),
        None => return false,
    };
    cue.is_match(before(text, start)).unwrap_or(false)
}

/// Boilerplate shared by every rule here: a name, a compiled pattern, and a renderer.
struct PhoneRule {
    name: &'static str,
    re: Regex,
    render: fn(&Captures<'_>, &str, &Config) -> Option<String>,
}

impl PhoneRule {
    fn boxed(
        name: &'static str,
        pattern: &str,
        render: fn(&Captures<'_>, &str, &Config) -> Option<String>,
    ) -> Box<dyn Rule> {
        Box::new(PhoneRule {
            name,
            re: Regex::new(pattern).expect("phone pattern"),
            render,
        })
    }
}

impl Rule for PhoneRule {
    fn name(&self) -> &'static str {
        self.name
    }

    fn priority(&self) -> i32 {
        priority::PHONE
    }

    fn pattern(&self) -> &Regex {
        &self.re
    }

    fn render(&self, m: &Captures<'_>, text: &str, cfg: &Config) -> Option<String> {
        (self.render)(m, text, cfg)
    }
}

fn whole(m: &Captures<'_>) -> Option<String> {
    Some(spoken(m.get(0)?.as_str()))
}

/// `+972-50-1234567`. The country code is read as פלוס plus its digits.
fn render_intl(m: &Captures<'_>, _text: &str, _cfg: &Config) -> Option<String> {
    let all = NOT_DIGITS.replace_all(m.get(0)?.as_str(), "").into_owned();
    let national = &all[3..];
    // "+972-050-..." is a common mistake; the trunk 0 is not dialled, so it is not read.
    let national = national.strip_prefix('0').unwrap_or(national);
    Some(format!("פלוס {} {}", digits("972"), digits(national)))
}

/// `*6555` — the short commercial numbers, spoken as כוכבית then the digits.
fn render_star(m: &Captures<'_>, _text: &str, _cfg: &Config) -> Option<String> {
    Some(format!("כוכבית {}", digits(m.get(1)?.as_str())))
}

/// `100`/`101`/`102` — claimed only after a dialling cue, never as a quantity.
fn render_emergency(m: &Captures<'_>, text: &str, _cfg: &Config) -> Option<String> {
    if !has_cue(m, text, &DIAL_CUE) {
        return None;
    }
    whole(m)
}

/// Nine-digit national ID / company numbers, claimed only after an identity cue.
fn render_id(m: &Captures<'_>, text: &str, _cfg: &Config) -> Option<String> {
    if !has_cue(m, text, &ID_CUE) {
        return None;
    }
    whole(m)
}

/// `192.168.1.1` — digit groups with the dots spoken, unlike a phone separator.
fn render_ip(m: &Captures<'_>, _text: &str, cfg: &Config) -> Option<String> {
    let mut parts = Vec::with_capacity(4);
    for i in 1..=4 {
        let octet = m.get(i)?.as_str();
        // A dotted quad with an out-of-range part is a version string or worse; leave it.
        if octet.parse::<u32>().ok()? > 255 {
            return None;
        }
        parts.push(digits(octet));
    }
    Some(parts.join(&format!(" {} ", cfg.decimal_word)))
}

fn render_plain(m: &Captures<'_>, _text: &str, _cfg: &Config) -> Option<String> {
    whole(m)
}

pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        PhoneRule::boxed(
            "phone-intl",
            &format!(r"\+972{SEP}?0?(?:5\d|7\d|[23489]){SEP}?\d{{3}}{SEP}?\d{{4}}(?!\d)"),
            render_intl,
        ),
        // `050-1234567` / `03-1234567` / `0501234567` — a leading 0 plus an area code.
        PhoneRule::boxed(
            "phone-local",
            // mobile / VoIP is 10 digits, landline 9.
            &format!(
                r"(?<!\d)0(?:(?:5\d|7\d){SEP}?\d{{3}}{SEP}?\d{{4}}|[23489]{SEP}?\d{{3}}{SEP}?\d{{4}})(?!\d)"
            ),
            render_plain,
        ),
        // `1-800-123-456` and friends: the 1-800/1-700/1-900/1-599 service ranges.
        PhoneRule::boxed(
            "phone-service",
            &format!(r"(?<!\d)1{SEP}?(?:800|700|900|599){SEP}?\d{{3}}{SEP}?\d{{3}}(?!\d)"),
            render_plain,
        ),
        PhoneRule::boxed("phone-star", r"\*(\d{3,5})(?!\d)", render_star),
        // Credit-card shaped groups. The grouping itself is the evidence, so it is required.
        PhoneRule::boxed(
            "phone-card",
            concat!(
                r"(?<!\d)(?:",
                r"\d{4}[-– ]\d{4}[-– ]\d{4}[-– ]\d{4}", // Visa / Mastercard
                r"|\d{4}[-– ]\d{6}[-– ]\d{5}",          // American Express
                r")(?!\d)"
            ),
            render_plain,
        ),
        PhoneRule::boxed(
            "phone-ip",
            r"(?<!\d)(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})(?!\d)",
            render_ip,
        ),
        PhoneRule::boxed("phone-id", r"(?<!\d)\d{9}(?!\d)", render_id),
        PhoneRule::boxed("phone-emergency", r"(?<!\d)1[01]\d(?!\d)", render_emergency),
    ]
}
