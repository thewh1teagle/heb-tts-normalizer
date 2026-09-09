//! Invariants over real Hebrew text nobody wrote for us.
//!
//! The TSV corpus next door is a specification: every row says what the right answer
//! is, and every row was written by whoever wrote the rules. That provenance is the
//! problem — a case nobody thought of never gets a row, and the tests agree with the
//! code because they came from the same head.
//!
//! This file takes the opposite trade. The sentences under `tests/data/soak/` are
//! mined from OPUS (see `scripts/mine_opus.py`) and nobody has reviewed them, so there
//! is no expected output to compare against. What is asserted instead are properties
//! that must hold whatever the correct reading turns out to be:
//!
//! * `normalize` returns, on every one of them. A panic is a slice at a non-character
//!   boundary or a rule looping, and either is a bug outright.
//! * It is idempotent — callers do re-run it over already-clean text.
//! * It introduces no niqqud. (It cannot *remove* the niqqud real text arrives with,
//!   so the invariant is about what the rules add, not what survives.)
//! * Prose with no trigger in it goes through the rule scan untouched.
//! * The output is finalized: no double space, no stray space before punctuation.
//!
//! Two things that are *not* invariants are measured and ratcheted instead: how often
//! a digit survives into the output, and how often a sentence with a trigger comes back
//! unchanged. Neither is necessarily wrong — `32/13/2026` is not a date and a version
//! number is not a quantity — but both can only get better, so the counts are pinned
//! and a regression fails. `cargo test --test soak -- --ignored --nocapture` prints the
//! per-category breakdown and the sentences behind those numbers.

use std::collections::BTreeMap;
use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::OnceLock;

use heb_tts_normalizer::rules::protect;
use heb_tts_normalizer::text::{clean, finalize};
use heb_tts_normalizer::{Clock, Config, normalize};

/// Sentences whose output still keeps a digit, as of the last mining run.
///
/// Every one is a case some rule looked at and declined, and most of them are right to:
/// `31.9.1975` is not a date, `40:25` is a track length rather than a clock time,
/// `מתי 24:36` is a Bible verse. The largest single group is the fraction — `1/2`,
/// `24/7`, `120/80` — which no rule reads at all. The number may only go down; if a
/// change makes it go up, look at what changed before raising it: sometimes going up is
/// right. Refusing to read the milliseconds in an SRT timecode leaves more digits in the
/// output than reading them did, and is the better answer. `chore soak-report` prints
/// the list.
const MAX_DIGIT_SURVIVORS: usize = 146;

/// Sentences that carry a trigger and still come back completely unchanged: 58 of
/// 4772, a bit over one percent.
///
/// This is the coverage metric the mining is really for — it estimates how much of real
/// Hebrew the rule set does not touch, measured against text nobody wrote for us rather
/// than against our own imagination. Nearly all of what is left is the fraction and the
/// ratio: `50/50`, `24/7`, `לחץ דם 120/80`. Same ratchet, same direction.
const MAX_UNTOUCHED: usize = 58;

fn soak_dir() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/soak"))
}

/// One mined sentence, with enough provenance to find it in the file again.
struct Line {
    file: String,
    lineno: usize,
    category: String,
    text: String,
}

impl Line {
    fn id(&self) -> String {
        format!("soak/{}:{}", self.file, self.lineno)
    }
}

/// Every `tests/data/soak/*.txt`: comment lines out, `# category: x (n)` remembered.
fn load() -> Vec<Line> {
    let dir = soak_dir();
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "txt"))
        .collect();
    files.sort();
    assert!(
        !files.is_empty(),
        "no *.txt under {} — run `chore mine`",
        dir.display()
    );

    let mut lines = Vec::new();
    for path in files {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("file name")
            .to_string();
        let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
        let mut category = String::from("unknown");
        for (idx, raw) in text.lines().enumerate() {
            let raw = raw.trim();
            if let Some(rest) = raw.strip_prefix("# category:") {
                category = rest
                    .split_whitespace()
                    .next()
                    .unwrap_or("unknown")
                    .to_string();
                continue;
            }
            if raw.is_empty() || raw.starts_with('#') {
                continue;
            }
            lines.push(Line {
                file: name.clone(),
                lineno: idx + 1,
                category: category.clone(),
                text: raw.to_string(),
            });
        }
    }
    lines
}

// ---------------------------------------------------------------------------
// character classes
// ---------------------------------------------------------------------------

fn is_digit(c: char) -> bool {
    c.is_ascii_digit() || ('\u{0660}'..='\u{0669}').contains(&c)
}

/// Vowel points and cantillation. Geresh, gershayim, maqaf and the sentence marks are
/// punctuation and are not niqqud.
fn is_niqqud(c: char) -> bool {
    matches!(c as u32,
        0x0591..=0x05BD | 0x05BF | 0x05C1..=0x05C2 | 0x05C4..=0x05C5 | 0x05C7)
}

/// Anything that could make a rule fire. Every rule needs a digit, a currency or
/// percent sign, or a geresh/gershayim, so text with none of these must come out of the
/// scan exactly as it went in.
fn has_trigger(s: &str) -> bool {
    s.chars().any(|c| {
        is_digit(c)
            || matches!(c, '₪' | '$' | '€' | '£' | '%' | '‰' | '°' | '׳' | '״' | '\'' | '"')
            // protect claims Latin-plus-digit names, URLs, handles and filenames
            || c.is_ascii_alphabetic()
            || matches!(c, '#' | '@')
    })
}

/// The spans `protect` claimed in the input — a URL, an email, `GPT-4`, `report.pdf`.
/// A digit inside one of these is meant to survive.
fn protected_spans(text: &str) -> Vec<String> {
    let rules = protect::rules();
    let pattern = rules[0].pattern();
    let mut spans = Vec::new();
    let mut at = 0usize;
    while at <= text.len() {
        match pattern.captures_from_pos(text, at) {
            Ok(Some(caps)) => {
                let m = caps.get(0).expect("group 0");
                spans.push(m.as_str().to_string());
                at = if m.end() > m.start() {
                    m.end()
                } else {
                    m.end() + 1
                };
            }
            _ => break,
        }
    }
    spans
}

/// Whitespace-delimited tokens of `out` that still hold a digit and are not part of
/// anything `protect` claimed.
fn unexplained_digit_tokens(input: &str, out: &str) -> Vec<String> {
    let tokens: Vec<&str> = out
        .split_whitespace()
        .filter(|t| t.chars().any(is_digit))
        .collect();
    if tokens.is_empty() {
        return Vec::new();
    }
    let spans = protected_spans(input);
    tokens
        .into_iter()
        .filter(|t| {
            let bare = t.trim_matches(|c: char| !c.is_alphanumeric());
            !spans.iter().any(|s| s.contains(bare) || bare.contains(s))
        })
        .map(str::to_string)
        .collect()
}

/// Run `normalize` and turn a panic into a message instead of an aborted test run.
fn normalize_or_report(line: &Line, cfg: &Config, failures: &mut Vec<String>) -> Option<String> {
    let hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let out = panic::catch_unwind(AssertUnwindSafe(|| normalize(&line.text, cfg)));
    panic::set_hook(hook);
    match out {
        Ok(s) => Some(s),
        Err(e) => {
            let what = e
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "<non-string panic>".to_string());
            failures.push(format!(
                "{}: normalize() panicked: {what}\n  input {}",
                line.id(),
                line.text
            ));
            None
        }
    }
}

// ---------------------------------------------------------------------------
// invariants
// ---------------------------------------------------------------------------

fn check(line: &Line, cfg: &Config, failures: &mut Vec<String>) {
    let Some(out) = normalize_or_report(line, cfg, failures) else {
        return;
    };

    let twice = normalize(&out, cfg);
    if twice != out {
        failures.push(format!(
            "{}: not idempotent\n  input {}\n  once  {}\n  twice {}",
            line.id(),
            line.text,
            out,
            twice
        ));
    }

    let added: Vec<char> = out
        .chars()
        .filter(|c| is_niqqud(*c) && !line.text.contains(*c))
        .collect();
    if !added.is_empty() {
        let points: Vec<String> = added
            .iter()
            .map(|c| format!("U+{:04X}", *c as u32))
            .collect();
        failures.push(format!(
            "{}: output has niqqud the input did not: {}\n  input {}\n  out   {}",
            line.id(),
            points.join(", "),
            line.text,
            out
        ));
    }

    if cfg.clean_whitespace {
        if out.contains("  ") {
            failures.push(format!(
                "{}: output has a double space\n  out {out:?}",
                line.id()
            ));
        }
        if out != out.trim() {
            failures.push(format!(
                "{}: output has surrounding whitespace\n  out {out:?}",
                line.id()
            ));
        }
        // finalize() removes these, so one in the output means a rule emitted it after
        // finalize ran, or finalize missed it.
        for bad in [" ,", " .", " ;", " :", " !", " ?"] {
            if out.contains(bad) {
                failures.push(format!(
                    "{}: output has a space before punctuation ({bad:?})\n  out {out}",
                    line.id()
                ));
            }
        }
    }
}

#[test]
fn invariants_hold_on_real_text() {
    let lines = load();
    assert!(
        lines.len() > 3000,
        "only {} soak sentences loaded — is tests/data/soak/ populated?",
        lines.len()
    );

    // Two configs, because a rule can be well-behaved on one branch and not the other,
    // and the non-default branches never see real text otherwise.
    let configs = [
        ("default", Config::default()),
        (
            "clock=24;expand_units=false",
            Config {
                clock: Clock::H24,
                expand_units: false,
                ..Config::default()
            },
        ),
    ];

    let mut failures: Vec<String> = Vec::new();
    for (name, cfg) in &configs {
        let before = failures.len();
        for line in &lines {
            check(line, cfg, &mut failures);
        }
        if failures.len() > before {
            failures.insert(before, format!("--- under config: {name} ---"));
        }
    }

    report(&failures, lines.len());
}

#[test]
fn prose_without_a_trigger_is_untouched() {
    // Real Hebrew prose, taken from the same sentences with every trigger-bearing token
    // dropped. Compared against clean()+finalize() rather than the input itself: those
    // two passes are allowed to touch the string (NFC, invisibles, quote shapes), the
    // rule scan is not.
    let cfg = Config::default();
    let mut failures: Vec<String> = Vec::new();
    let mut checked = 0usize;

    for line in load() {
        let prose: String = line
            .text
            .split_whitespace()
            .filter(|t| !has_trigger(t))
            .collect::<Vec<_>>()
            .join(" ");
        if prose.split_whitespace().count() < 3 {
            continue;
        }
        checked += 1;
        let expected = finalize(&clean(&prose, &cfg), &cfg);
        let actual = normalize(&prose, &cfg);
        if actual != expected {
            failures.push(format!(
                "{}: a rule fired on text with no trigger in it\n  prose    {prose}\n  \
                 expected {expected}\n  actual   {actual}",
                line.id()
            ));
        }
    }

    assert!(checked > 1000, "only {checked} prose lines derived");
    report(&failures, checked);
}

// ---------------------------------------------------------------------------
// ratcheted metrics
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Tally {
    total: usize,
    untouched: Vec<String>,
    digits: Vec<String>,
}

/// The measurement, computed once. Three tests read it and it is the expensive part
/// of this file; `cargo test` runs them in three threads of one process.
fn measured() -> &'static BTreeMap<String, Tally> {
    static ONCE: OnceLock<BTreeMap<String, Tally>> = OnceLock::new();
    ONCE.get_or_init(measure)
}

fn measure() -> BTreeMap<String, Tally> {
    let cfg = Config::default();
    let mut by_category: BTreeMap<String, Tally> = BTreeMap::new();
    for line in load() {
        let out = normalize(&line.text, &cfg);
        let tally = by_category.entry(line.category.clone()).or_default();
        tally.total += 1;
        if out == line.text {
            tally
                .untouched
                .push(format!("{}\t{}", line.id(), line.text));
        }
        let stray = unexplained_digit_tokens(&line.text, &out);
        if !stray.is_empty() {
            tally
                .digits
                .push(format!("{}\t{:?}\t{}", line.id(), stray, out));
        }
    }
    by_category
}

#[test]
fn digit_survival_does_not_regress() {
    let tallies = measured();
    let found: usize = tallies.values().map(|t| t.digits.len()).sum();
    assert!(
        found <= MAX_DIGIT_SURVIVORS,
        "{found} sentences keep a digit in the output, budget is {MAX_DIGIT_SURVIVORS}.\n\
         A rule stopped firing on real text. Run:\n  \
         cargo test --test soak -- --ignored --nocapture\n\
         to see them. If the increase is genuinely correct, raise MAX_DIGIT_SURVIVORS \
         in tests/soak.rs and say why."
    );
}

#[test]
fn coverage_does_not_regress() {
    let tallies = measured();
    let found: usize = tallies.values().map(|t| t.untouched.len()).sum();
    assert!(
        found <= MAX_UNTOUCHED,
        "{found} sentences carry a trigger and come back unchanged, budget is \
         {MAX_UNTOUCHED}.\nCoverage against real text went down. Run:\n  \
         cargo test --test soak -- --ignored --nocapture"
    );
}

/// The mining report: what fired, what did not, and the sentences behind both numbers.
///
/// Ignored by default because its output is a page of Hebrew, not a pass/fail. This is
/// the thing to read when picking sentences to promote into golden TSV rows.
#[test]
#[ignore = "prints a report; run with --nocapture"]
fn report_by_category() {
    let tallies = measured();
    let total: usize = tallies.values().map(|t| t.total).sum();
    let untouched: usize = tallies.values().map(|t| t.untouched.len()).sum();
    let digits: usize = tallies.values().map(|t| t.digits.len()).sum();

    println!("\nsoak coverage over {total} mined sentences\n");
    println!(
        "{:<16} {:>7} {:>11} {:>9}",
        "category", "total", "untouched", "digits"
    );
    println!("{}", "-".repeat(46));
    for (category, t) in tallies {
        println!(
            "{:<16} {:>7} {:>10.0}% {:>9}",
            category,
            t.total,
            100.0 * t.untouched.len() as f64 / t.total.max(1) as f64,
            t.digits.len(),
        );
    }
    println!("{}", "-".repeat(46));
    println!(
        "{:<16} {:>7} {:>10.0}% {:>9}",
        "all",
        total,
        100.0 * untouched as f64 / total.max(1) as f64,
        digits
    );

    for (category, t) in tallies {
        if t.untouched.is_empty() && t.digits.is_empty() {
            continue;
        }
        println!("\n=== {category} ===");
        for row in &t.untouched {
            println!("  untouched  {row}");
        }
        for row in &t.digits {
            println!("  digit-kept {row}");
        }
    }
}

/// One readable panic instead of a thousand.
fn report(failures: &[String], checked: usize) {
    if failures.is_empty() {
        return;
    }
    const SHOWN: usize = 40;
    let mut out = String::new();
    for f in failures.iter().take(SHOWN) {
        out.push('\n');
        out.push_str(&"-".repeat(72));
        out.push('\n');
        out.push_str(f);
        out.push('\n');
    }
    out.push_str(&"-".repeat(72));
    if failures.len() > SHOWN {
        out.push_str(&format!("\n… and {} more.", failures.len() - SHOWN));
    }
    panic!(
        "{out}\n\n{} failure(s) over {checked} real sentences.",
        failures.len()
    );
}
