//! String hygiene that runs before and after the rule scan.
//!
//! These are the passes that must not be rules: they rewrite the whole string rather
//! than one span, and they have to happen before any rule sees the text.

use fancy_regex::Regex;
use once_cell::sync::Lazy;
use unicode_normalization::UnicodeNormalization;

use crate::config::Config;

/// Hebrew letters plus niqqud and the punctuation in the Hebrew block.
pub const HEB: &str = r"\u{0590}-\u{05FE}";

/// Hebrew letters only, the class rules use for word boundaries.
pub const LETTER: &str = r"\u{05D0}-\u{05EA}";

pub const GERESH: char = '׳';
pub const GERSHAYIM: char = '״';

/// Zero-width characters and bidi controls: invisible to a reader, noise to a g2p.
static INVISIBLE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        "[\u{00AD}\u{200B}-\u{200F}\u{202A}-\u{202E}\u{2060}-\u{2064}\u{2066}-\u{2069}\u{FEFF}]",
    )
    .expect("INVISIBLE")
});

/// Non-breaking and exotic spaces that should just be a space.
static ODD_SPACE: Lazy<Regex> = Lazy::new(|| {
    Regex::new("[\u{00A0}\u{2000}-\u{200A}\u{202F}\u{205F}\u{3000}]").expect("ODD_SPACE")
});

static WHITESPACE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[ \t]{2,}").expect("WHITESPACE"));
static BLANK_LINES: Lazy<Regex> = Lazy::new(|| Regex::new(r"\n{3,}").expect("BLANK_LINES"));
static SPACE_BEFORE_PUNCT: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\s+([,.;:!?])").expect("SPACE_BEFORE_PUNCT"));

/// ASCII quotes used as Hebrew geresh/gershayim: `ד"ר`, `מס'`. Canonicalised so a rule
/// can match one spelling instead of four.
static ASCII_GERSHAYIM: Lazy<Regex> =
    Lazy::new(|| Regex::new(&format!(r#"(?<=[{HEB}])"(?=[{HEB}])"#)).expect("ASCII_GERSHAYIM"));
static ASCII_GERESH: Lazy<Regex> =
    Lazy::new(|| Regex::new(&format!(r"(?<=[{HEB}])'(?![{HEB}])")).expect("ASCII_GERESH"));

/// ASCII `"` and `'` used as Hebrew punctuation, given their Hebrew shapes.
fn canonical_quotes(text: &str) -> String {
    let s = ASCII_GERSHAYIM.replace_all(text, GERSHAYIM.to_string());
    ASCII_GERESH
        .replace_all(&s, GERESH.to_string())
        .into_owned()
}

/// Prepare raw text for the scan: strip invisibles, unify quotes, drop Markdown.
pub fn clean(text: &str, cfg: &Config) -> String {
    let mut s: String = text.nfc().collect();
    s = INVISIBLE.replace_all(&s, "").into_owned();
    s = ODD_SPACE.replace_all(&s, " ").into_owned();
    s = canonical_quotes(&s);
    if cfg.strip_markdown {
        s = crate::rules::cleanup::strip_markdown(&s);
    }
    s
}

/// Tidy up after the rules: quote shapes first, then the spacing they left behind.
///
/// The quotes are done twice — once here and once in [`clean`] — because a rule turns
/// digits into Hebrew words, and that can put an ASCII quote next to a Hebrew letter
/// that was not next to one before the scan: `74'` comes out as `שבעים וארבע'`. Without
/// this pass the shape would change on a *second* call to `normalize`, and idempotence
/// is a promise this library makes.
pub fn finalize(text: &str, cfg: &Config) -> String {
    let text = canonical_quotes(text);
    if !cfg.clean_whitespace {
        return text;
    }
    let mut s = WHITESPACE.replace_all(&text, " ").into_owned();
    s = SPACE_BEFORE_PUNCT.replace_all(&s, "$1").into_owned();
    s = BLANK_LINES.replace_all(&s, "\n\n").into_owned();
    s.lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}
