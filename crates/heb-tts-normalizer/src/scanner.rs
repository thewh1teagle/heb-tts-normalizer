//! One left-to-right pass over the text, highest-priority rule wins at each position.
//!
//! Chained passes would be simpler, and wrong: the numbers rule would eat the digits
//! inside a date the dates rule already consumed. Here a span is matched once, rendered
//! once, and never looked at again.

use fancy_regex::{Captures, Regex};

use crate::config::Config;
use crate::text::{clean, finalize};

/// A span matcher. Every rule module exports its rules into the registry.
pub trait Rule: Send + Sync {
    fn name(&self) -> &'static str;

    /// Higher wins. See [`priority`] for the scheme.
    fn priority(&self) -> i32;

    /// Must be able to match at an arbitrary position; avoid a leading `^`.
    fn pattern(&self) -> &Regex;

    /// Spoken form of the match, or `None` to decline.
    ///
    /// Declining matters: a dates pattern will happily match `32/13/2026`, and only
    /// after the digits are parsed can it be ruled out. A declined match falls through
    /// to the next candidate at this position.
    fn render(&self, m: &Captures<'_>, text: &str, cfg: &Config) -> Option<String>;
}

/// Priority scheme. Rules that consume more context sit above the ones they contain.
pub mod priority {
    /// Claims a span in order to leave it alone: URLs, emails, filenames.
    pub const PROTECT: i32 = 100;
    pub const DATE: i32 = 90;
    pub const TIME: i32 = 85;
    pub const PHONE: i32 = 80;
    pub const CURRENCY: i32 = 70;
    pub const PERCENT: i32 = 65;
    /// Above UNIT: `3–5 ק״מ` is one span, not a range then a stray unit.
    pub const RANGE: i32 = 62;
    pub const UNIT: i32 = 60;
    pub const ABBREV: i32 = 55;
    pub const ORDINAL: i32 = 50;
    pub const NUMBER: i32 = 40;
}

/// Rules that cannot match a string containing no ASCII digit.
///
/// Ordinary prose has no digits, and on such text these 24 rules can only ever fail —
/// but failing still costs a full regex search each, several over large alternations
/// (96 unit forms, 67 abbreviations, the month tables). Skipping them outright is most
/// of the cost of normalizing text that has nothing to normalize.
///
/// Listed by name rather than derived from the pattern, because a pattern *containing*
/// `\d` may still match without one: `protect` matches a bare URL, `abbrev` matches
/// `ד״ר`, and `date-hebrew` matches `כ״ז באלול`. Those three are deliberately absent.
/// A rule missing from this list only loses speed; a rule wrongly on it loses
/// correctness, so the corpus is what keeps it honest.
const DIGIT_REQUIRED: &[&str] = &[
    "date-numeric",
    "date-iso",
    "date-textual",
    "date-month-year",
    "time-clock",
    "time-spoken-hour",
    "phone-local",
    "phone-intl",
    "phone-service",
    "phone-star",
    "phone-card",
    "phone-ip",
    "phone-id",
    "phone-emergency",
    "currency-suffix",
    "currency-prefix",
    "percent-sign",
    "percent-word",
    "range",
    "range-with-tail",
    "unit",
    "degree",
    "ordinal",
    "count",
    "number",
    "number-prefix",
];

/// Rewrite every span a rule claims, leaving everything else byte-identical.
///
/// `rules` must already be sorted by descending priority; the registry does that once.
pub fn scan(text: &str, rules: &[Box<dyn Rule>], cfg: &Config) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pos = 0usize;

    // Each rule's next match, cached. The text never changes, so a rule's leftmost match
    // from `pos` stays its leftmost match until `pos` passes it — searching all 28 rules
    // at every character instead costs a scan of the remaining text per rule per
    // character, which is what made this quadratic.
    let has_digit = text.as_bytes().iter().any(u8::is_ascii_digit);

    let mut next: Vec<Option<Captures<'_>>> = Vec::with_capacity(rules.len());
    let mut skipped = vec![false; rules.len()];
    for (i, rule) in rules.iter().enumerate() {
        if !has_digit && DIGIT_REQUIRED.contains(&rule.name()) {
            skipped[i] = true;
            next.push(None);
            continue;
        }
        next.push(rule.pattern().captures_from_pos(text, 0).ok().flatten());
    }

    while pos < text.len() {
        // Candidates that begin exactly here, best first. Within one priority the
        // longest match wins, which is why this collects before rendering.
        let mut here: Vec<(usize, i32, &dyn Rule, &Captures<'_>)> = Vec::new();
        let mut earliest: Option<usize> = None;

        // Refresh any cached match the scan has already moved past. Separate pass:
        // `here` below borrows from `next`, so the mutation has to finish first.
        for (i, rule) in rules.iter().enumerate() {
            if skipped[i] {
                continue;
            }
            if next[i]
                .as_ref()
                .and_then(|c| c.get(0))
                .is_some_and(|m| m.start() < pos)
            {
                next[i] = rule.pattern().captures_from_pos(text, pos).ok().flatten();
            }
        }

        for (rule, slot) in rules.iter().zip(next.iter()) {
            let Some(caps) = slot.as_ref() else {
                continue;
            };
            let Some(m) = caps.get(0) else { continue };
            if m.start() == pos && m.end() > m.start() {
                here.push((m.end(), rule.priority(), rule.as_ref(), caps));
            } else if m.start() > pos {
                earliest = Some(earliest.map_or(m.start(), |e: usize| e.min(m.start())));
            }
        }

        if here.is_empty() {
            // Nothing starts here. Skip straight to the nearest position where anything
            // could, instead of walking one character at a time.
            let next_pos = earliest.unwrap_or(text.len());
            out.push_str(&text[pos..next_pos]);
            pos = next_pos;
            continue;
        }

        here.sort_by(|a, b| b.1.cmp(&a.1).then(b.0.cmp(&a.0)));

        let mut rendered = None;
        for (end, _, rule, caps) in &here {
            if let Some(s) = rule.render(caps, text, cfg) {
                rendered = Some((s, *end));
                break;
            }
        }

        match rendered {
            Some((s, end)) => {
                out.push_str(&s);
                pos = end;
            }
            None => {
                // Every candidate declined. Advance one character — a rule may still
                // match inside the span that was just refused.
                let ch = text[pos..].chars().next().expect("pos is a char boundary");
                out.push(ch);
                pos += ch.len_utf8();
            }
        }
    }

    out
}

/// Turn Hebrew text into something a g2p can read aloud.
///
/// ```
/// use heb_tts_normalizer::{normalize, Config};
/// assert_eq!(normalize("3 ילדים", &Config::default()), "שלושה ילדים");
/// ```
pub fn normalize(text: &str, cfg: &Config) -> String {
    let cleaned = clean(text, cfg);
    let scanned = scan(&cleaned, crate::rules::all(), cfg);
    finalize(&scanned, cfg)
}
