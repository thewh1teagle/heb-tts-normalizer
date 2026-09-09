//! Spans that must survive untouched.
//!
//! A URL is the clearest case: `example.com/12/03/2026` contains a perfectly valid
//! date, and without this rule the dates rule would happily rewrite the middle of the
//! link. Every other rule sits below PROTECT, so claiming a span here and handing it
//! back verbatim is what keeps them off it.
//!
//! Scope is deliberately narrow — text that is *not addressed to a listener*. Things a
//! speaker would actually read out (a version number, an IP address) are left to the
//! rules that know how to say them.

use fancy_regex::{Captures, Regex};

use crate::config::Config;
use crate::scanner::{Rule, priority};

/// A URL, up to the first whitespace or Hebrew quotation mark. Trailing sentence
/// punctuation is excluded so "בקרו ב-example.com." keeps its full stop.
const URL: &str = r#"(?:https?://|www\.)[^\s<>"'׳״]*[^\s<>"'׳״.,;:!?)\]]"#;

const EMAIL: &str = r"[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)+";

/// A filename with a recognised extension. Restricted to a known extension list on
/// purpose: a bare `a.b` pattern would swallow decimals and sentence boundaries.
const EXT: &str = concat!(
    "py|js|ts|tsx|jsx|rs|go|java|rb|sh|c|cpp|h|json|toml|yaml|yml|xml|csv|tsv|md|txt",
    "|html?|css|pdf|docx?|xlsx?|pptx?|png|jpe?g|gif|svg|webp|mp3|mp4|wav|mov|zip|tar|gz"
);

/// Hashtags and handles, Hebrew or Latin.
const HANDLE: &str = r"[#@][A-Za-z0-9_\u{0590}-\u{05FF}]+";

/// A Latin word fused to digits: GPT-4, COVID-19, iPhone15, v2, MP3. The digits are
/// part of a name, not a quantity, so reading them in Hebrew would be wrong. Note the
/// digits must touch the letters — "5 km" is a measurement and belongs to the units rule.
///
/// The trailing group matters: a designation like `Tu-16K-10-26` has to be claimed
/// whole. Claiming only `Tu-16K` leaves `-10-26` behind, and the number rule then reads
/// it as minus ten — inside an aircraft's name.
const ALPHANUM: &str = r"\b[A-Za-z]+-?\d+[A-Za-z0-9]*(?:-[A-Za-z0-9]+)*\b";

/// Match it, then give it straight back.
struct ProtectRule {
    pattern: Regex,
}

impl ProtectRule {
    fn new() -> Self {
        let filename = format!(r"[A-Za-z0-9_\-.]+\.(?:{EXT})\b");
        Self {
            pattern: Regex::new(&format!(
                "(?:{URL})|(?:{EMAIL})|(?:{filename})|(?:{HANDLE})|(?:{ALPHANUM})"
            ))
            .expect("protect pattern"),
        }
    }
}

impl Rule for ProtectRule {
    fn name(&self) -> &'static str {
        "protect"
    }

    fn priority(&self) -> i32 {
        priority::PROTECT
    }

    fn pattern(&self) -> &Regex {
        &self.pattern
    }

    fn render(&self, m: &Captures<'_>, _text: &str, _cfg: &Config) -> Option<String> {
        Some(m.get(0)?.as_str().to_string())
    }
}

pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![Box::new(ProtectRule::new())]
}
