//! Markdown removal — a whole-string pre-pass, not a span rule.
//!
//! Markdown is punctuation for the eye. A g2p reading `**חשוב**` aloud has to say
//! something about the asterisks, so they are dropped before any rule runs. This module
//! therefore exports [`strip_markdown`] (called from [`crate::text::clean`]) instead of
//! `rules()`, and it is deliberately absent from the rule registry.
//!
//! # Judgment calls
//!
//! * A fenced code block is dropped whole. Read aloud it is noise; inline `` `code` ``
//!   keeps its text because it is usually a word in the sentence.
//! * An image contributes nothing to speech, so `![alt](url)` disappears entirely, while
//!   `[text](url)` keeps `text` — the URL itself is unspeakable.
//! * Removing a list bullet or a table pipe would run two items into one breath, so a
//!   full stop is left behind when the item does not already end in punctuation.
//! * Hebrew text is full of characters Markdown also uses. `*` and `_` are stripped only
//!   as a *matched pair* around non-space text, and `_` additionally only at a word
//!   boundary, so `*6555` (a star phone number) and `קו_תחתון` survive intact. Geresh and
//!   gershayim (`׳` `״`) are Hebrew punctuation and are never touched.

use std::borrow::Cow;

use fancy_regex::{Captures, Regex};
use once_cell::sync::Lazy;

use crate::text::HEB;

/// Fenced code blocks, ``` or ~~~, dropped with their contents.
static FENCE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?ms)^[ \t]*(?<f>```|~~~)[^\n]*\n.*?^[ \t]*\k<f>[^\n]*$\n?").expect("FENCE")
});
/// A dangling fence marker on its own line, once the pairs above are gone.
static LONE_FENCE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?m)^[ \t]*(?:```|~~~)[^\n]*$\n?").expect("LONE_FENCE"));

/// `<https://example.com>` — an autolink says nothing out loud.
static AUTOLINK: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<(?:https?|mailto|tel):[^>\s]*>").expect("AUTOLINK"));

static IMAGE: Lazy<Regex> = Lazy::new(|| Regex::new(r"!\[[^\]\n]*\]\([^)\n]*\)").expect("IMAGE"));
static INLINE_LINK: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\[([^\]\n]*)\]\([^)\n]*\)").expect("INLINE_LINK"));
static REF_LINK: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\[([^\]\n]*)\]\[[^\]\n]*\]").expect("REF_LINK"));
/// A link definition line, `[ref]: https://…`, is metadata and never spoken.
static LINK_DEF: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?m)^[ \t]*\[[^\]\n]+\]:[^\n]*$\n?").expect("LINK_DEF"));

static INLINE_CODE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?<!`)(`+)(?!`)([^`\n]+?)\1(?!`)").expect("INLINE_CODE"));

static HEADING: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[ \t]{0,3}#{1,6}[ \t]+(?<text>[^\n]*?)(?:[ \t]+#+)?[ \t]*$").expect("HEADING")
});
static THEMATIC_BREAK: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[ \t]{0,3}(?<c>[-*_])(?:[ \t]*\k<c>){2,}[ \t]*$").expect("THEMATIC_BREAK")
});
static BLOCKQUOTE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[ \t]{0,3}>+[ \t]?").expect("BLOCKQUOTE"));
static LIST_MARKER: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[ \t]*(?:[-*+]|\d{1,9}[.)])[ \t]+").expect("LIST_MARKER"));
static TABLE_DIVIDER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[ \t]*\|?[ \t]*:?-{2,}:?[ \t]*(?:\|[ \t]*:?-+:?[ \t]*)*\|?[ \t]*$")
        .expect("TABLE_DIVIDER")
});

static STRIKE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)~~(?=\S)(.+?)(?<=\S)~~").expect("STRIKE"));
static BOLD_STAR: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)\*\*(?=\S)(.+?)(?<=\S)\*\*").expect("BOLD_STAR"));
static BOLD_UNDER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!(
        r"(?s)(?<![\w{HEB}])__(?=\S)(.+?)(?<=\S)__(?![\w{HEB}])"
    ))
    .expect("BOLD_UNDER")
});
static ITALIC_STAR: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\*(?=[^\s*])([^*\n]+?)(?<=\S)\*").expect("ITALIC_STAR"));
static ITALIC_UNDER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!(
        r"(?<![\w{HEB}])_(?=\S)([^_\n]+?)(?<=\S)_(?![\w{HEB}])"
    ))
    .expect("ITALIC_UNDER")
});

static ESCAPED: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\\([\\`*_{}\[\]()#+\-.!~>|])").expect("ESCAPED"));

/// Escaped punctuation is parked in the private-use area so the passes below cannot
/// mistake `\*` for emphasis; [`unprotect_escapes`] puts the character back at the end.
const PUA: u32 = 0xE000;
static PROTECTED: Lazy<Regex> = Lazy::new(|| Regex::new("[\u{e000}-\u{e07f}]").expect("PROTECTED"));

/// Punctuation that already ends a phrase, so no full stop needs adding.
const ENDS_PHRASE: [char; 9] = ['.', ',', ';', ':', '!', '?', '…', '׃', '־'];

fn protect_escapes(text: &str) -> Cow<'_, str> {
    ESCAPED.replace_all(text, |m: &Captures<'_>| {
        m.get(1)
            .and_then(|g| g.as_str().chars().next())
            .and_then(|c| char::from_u32(PUA + c as u32))
            .map_or(String::new(), |c| c.to_string())
    })
}

fn unprotect_escapes(text: &str) -> Cow<'_, str> {
    PROTECTED.replace_all(text, |m: &Captures<'_>| {
        let ch = match m.get(0).and_then(|g| g.as_str().chars().next()) {
            Some(c) => c,
            None => return String::new(),
        };
        let orig = char::from_u32(ch as u32 - PUA);
        // Escaped emphasis markers are dropped rather than restored. A literal "*" says
        // nothing aloud, and restoring one is not idempotent: the next pass would read
        // it as emphasis again.
        match orig {
            Some('*') | Some('_') | None => String::new(),
            Some(c) => c.to_string(),
        }
    })
}

/// Give a bullet or table row a sentence boundary so the TTS breathes between items.
fn terminate(text: &str) -> String {
    let text = text.trim();
    match text.chars().last() {
        None => String::new(),
        Some(c) if ENDS_PHRASE.contains(&c) => text.to_string(),
        _ => format!("{text}."),
    }
}

/// Cell text of a Markdown table row, or `None` if the line is not one.
fn table_row(line: &str) -> Option<String> {
    let stripped = line.trim();
    if !stripped.contains('|') {
        return None;
    }
    if !(stripped.starts_with('|') || stripped.ends_with('|')) {
        return None;
    }
    if TABLE_DIVIDER.is_match(stripped).unwrap_or(false) {
        return Some(String::new());
    }
    let cells: Vec<&str> = stripped
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .collect();
    Some(terminate(&cells.join(", ")))
}

/// Line-level Markdown: headings, rules, quotes, bullets, tables.
fn strip_block_syntax(text: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    for raw in text.split('\n') {
        if THEMATIC_BREAK.is_match(raw).unwrap_or(false) {
            out.push(String::new());
            continue;
        }
        if let Ok(Some(caps)) = HEADING.captures(raw) {
            let body = caps.name("text").map_or("", |g| g.as_str());
            out.push(body.trim().to_string());
            continue;
        }
        if let Some(row) = table_row(raw) {
            out.push(row);
            continue;
        }
        let line = BLOCKQUOTE.replace_all(raw, "");
        match LIST_MARKER.find(&line) {
            Ok(Some(item)) => out.push(terminate(&line[item.end()..])),
            _ => out.push(line.into_owned()),
        }
    }
    out.join("\n")
}

/// Drop bold/italic/strikethrough markers, keeping the words they wrap.
pub fn strip_emphasis(text: &str) -> String {
    let s = STRIKE.replace_all(text, "${1}").into_owned();
    // Bold first: `***x***` is then left as `*x*` for the italic pass.
    let s = BOLD_STAR.replace_all(&s, "${1}").into_owned();
    let s = BOLD_UNDER.replace_all(&s, "${1}").into_owned();
    let s = ITALIC_STAR.replace_all(&s, "${1}").into_owned();
    ITALIC_UNDER.replace_all(&s, "${1}").into_owned()
}

/// `[text](url)` -> `text`; images, autolinks and link definitions -> nothing.
pub fn strip_links(text: &str) -> String {
    let s = LINK_DEF.replace_all(text, "").into_owned();
    let s = IMAGE.replace_all(&s, "").into_owned();
    let s = INLINE_LINK.replace_all(&s, "${1}").into_owned();
    let s = REF_LINK.replace_all(&s, "${1}").into_owned();
    AUTOLINK.replace_all(&s, "").into_owned()
}

/// Drop fenced blocks entirely; keep the text inside inline code spans.
pub fn strip_code(text: &str) -> String {
    let s = FENCE.replace_all(text, "").into_owned();
    let s = LONE_FENCE.replace_all(&s, "").into_owned();
    INLINE_CODE.replace_all(&s, "${2}").into_owned()
}

/// Remove Markdown syntax, leaving only the words and the prosodic punctuation.
pub fn strip_markdown(text: &str) -> String {
    let s = protect_escapes(text).into_owned();
    let s = strip_code(&s);
    let s = strip_links(&s);
    let s = strip_block_syntax(&s);
    let s = strip_emphasis(&s);
    // An escaped `\*` was never markup, so it comes back as itself, unspoken as syntax.
    unprotect_escapes(&s).into_owned()
}
