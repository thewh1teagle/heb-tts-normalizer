//! Every corpus row through the full public `normalize()`.
//!
//! This is the test that catches cross-rule interference. A unit test of the dates rule
//! cannot notice that its pattern started stealing `14:30` from times; a whole-pipeline
//! test on a sentence containing both does.
//!
//! Every `tests/data/*.tsv` beside this file is discovered at runtime — no hardcoded
//! list, so a rule author dropping in a new topic file gets it tested for free. A row
//! carries its own provenance (file + line) because the only thing worse than a failing
//! Hebrew assertion is a failing Hebrew assertion you cannot locate.

use std::fs;
use std::path::{Path, PathBuf};

use heb_tts_normalizer::{Config, normalize};
use serde_json::{Map, Value};

const HEADER: [&str; 4] = ["input", "expected", "config", "note"];

/// Config keys whose value is an enum, with the spellings serde accepts.
const ENUM_KEYS: [(&str, &[&str]); 3] = [
    ("clock", &["12", "24"]),
    ("date_order", &["dmy", "mdy"]),
    ("hebrew_date_style", &["letters", "numbers"]),
];

const BOOL_KEYS: [&str; 5] = [
    "read_minor_currency",
    "expand_units",
    "expand_abbreviations",
    "strip_markdown",
    "clean_whitespace",
];

const STRING_KEYS: [&str; 2] = ["default_currency", "decimal_word"];

/// `gender_overrides=סטוריז:m,ריל:f` — the one key whose value is a mapping.
const MAP_KEYS: [&str; 1] = ["gender_overrides"];

/// A note beginning with this says the row keeps a digit on purpose, and why.
///
/// The usual case — input a rule declines — is already covered by the passthrough skip
/// below, where `expected` equals `input`. This is for the rest: a sentence where some
/// rules fired and one span was declined, so the output is part words and part digits.
/// `tests/data/opus.tsv` is full of them, since real text is full of numbers that are
/// not quantities. The reason lives in the note beside the row rather than in a table
/// keyed by line number, which would silently point at the wrong row the moment anyone
/// inserted a line.
const KEEPS_DIGIT: &str = "keeps-digit:";

fn data_dir() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data"))
}

/// One corpus case, with enough provenance to find it again.
struct Row {
    file: String,
    lineno: usize,
    input: String,
    expected: String,
    config: Config,
    config_spec: String,
    note: String,
}

impl Row {
    /// `dates.tsv:42`
    fn id(&self) -> String {
        format!("{}:{}", self.file, self.lineno)
    }

    /// A failure message a human can actually read.
    ///
    /// Hebrew is RTL and a stray control character is invisible at a glance, so the
    /// three strings go on their own lines with ASCII labels, and again escaped so
    /// trailing spaces and invisibles show up.
    fn describe(&self, actual: &str) -> String {
        let mut lines = vec![
            format!(
                "{}: normalize() output does not match the corpus.",
                self.id()
            ),
            format!("  file     tests/data/{}:{}", self.file, self.lineno),
        ];
        if !self.note.is_empty() {
            lines.push(format!("  note     {}", self.note));
        }
        if !self.config_spec.is_empty() {
            lines.push(format!("  config   {}", self.config_spec));
        }
        lines.push(format!("  input    {}", self.input));
        lines.push(format!("  expected {}", self.expected));
        lines.push(format!("  actual   {actual}"));
        lines.push(String::new());
        lines.push(format!("  expected(esc) {}", escape(&self.expected)));
        lines.push(format!("  actual  (esc) {}", escape(actual)));
        if self.expected != actual {
            lines.push(format!(
                "  first difference at char {}",
                first_diff(&self.expected, actual)
            ));
        }
        lines.join("\n")
    }
}

/// Show the invisibles. `\u{...}` for anything a terminal will not render honestly.
fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 || c == '\u{7f}' || is_invisible(c) => {
                out.push_str(&format!("\\u{{{:04X}}}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn is_invisible(c: char) -> bool {
    matches!(c as u32,
        0x00A0 | 0x00AD | 0x200B..=0x200F | 0x202A..=0x202E
        | 0x2060..=0x2064 | 0x2066..=0x2069 | 0xFEFF)
}

fn first_diff(a: &str, b: &str) -> usize {
    a.chars()
        .zip(b.chars())
        .position(|(x, y)| x != y)
        .unwrap_or_else(|| a.chars().count().min(b.chars().count()))
}

// ---------------------------------------------------------------------------
// config column
// ---------------------------------------------------------------------------

/// `clock=24;expand_units=false` -> a real `Config`.
///
/// An unknown key is an error, not a shrug: a typo like `clok=24` would otherwise
/// silently test the default config and pass for the wrong reason. Same for a bad enum
/// value — `clock=25` must be loud, not "well, the default is 12".
fn parse_config(spec: &str, where_: &str) -> Result<Config, String> {
    let spec = spec.trim();
    if spec.is_empty() {
        return Ok(Config::default());
    }

    let mut obj: Map<String, Value> = Map::new();

    for part in spec.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (key, raw) = match part.split_once('=') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => return Err(format!("{where_}: config entry {part:?} is not key=value")),
        };
        if obj.contains_key(key) {
            return Err(format!("{where_}: config key {key:?} given twice"));
        }

        let value = if let Some((_, allowed)) = ENUM_KEYS.iter().find(|(k, _)| *k == key) {
            if !allowed.contains(&raw) {
                return Err(format!(
                    "{where_}: {key}={raw:?} is not one of: {}",
                    allowed.join(", ")
                ));
            }
            Value::String(raw.to_string())
        } else if BOOL_KEYS.contains(&key) {
            match raw.to_ascii_lowercase().as_str() {
                "true" | "yes" | "on" | "1" => Value::Bool(true),
                "false" | "no" | "off" | "0" => Value::Bool(false),
                _ => {
                    return Err(format!(
                        "{where_}: {key}={raw:?} is not a boolean (true/false)"
                    ));
                }
            }
        } else if STRING_KEYS.contains(&key) {
            Value::String(raw.to_string())
        } else if MAP_KEYS.contains(&key) {
            parse_gender_map(key, raw, where_)?
        } else {
            let mut known: Vec<&str> = ENUM_KEYS.iter().map(|(k, _)| *k).collect();
            known.extend(BOOL_KEYS);
            known.extend(STRING_KEYS);
            known.extend(MAP_KEYS);
            known.sort_unstable();
            return Err(format!(
                "{where_}: unknown config key {key:?}. Known keys: {}",
                known.join(", ")
            ));
        };

        obj.insert(key.to_string(), value);
    }

    let json = Value::Object(obj).to_string();
    Config::from_json(&json)
        .map_err(|e| format!("{where_}: config {spec:?} is not a valid Config: {e}"))
}

/// `word:m,other:f` — the TSV spelling of a `HashMap<String, Gender>`.
fn parse_gender_map(key: &str, raw: &str, where_: &str) -> Result<Value, String> {
    let mut map: Map<String, Value> = Map::new();
    for entry in raw.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let (word, gender) = entry.split_once(':').ok_or_else(|| {
            format!("{where_}: {key} entry {entry:?} is not word:gender (m or f)")
        })?;
        let gender = gender.trim();
        if gender != "m" && gender != "f" {
            return Err(format!(
                "{where_}: {key} entry {entry:?} has gender {gender:?}, expected m or f"
            ));
        }
        map.insert(word.trim().to_string(), Value::String(gender.to_string()));
    }
    Ok(Value::Object(map))
}

// ---------------------------------------------------------------------------
// loading
// ---------------------------------------------------------------------------

fn load_tsv(path: &Path) -> Result<Vec<Row>, String> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("<unnamed>")
        .to_string();
    let text =
        fs::read_to_string(path).map_err(|e| format!("{}: cannot read: {e}", path.display()))?;

    let mut rows = Vec::new();
    let mut header_seen = false;

    for (idx, line) in text.split('\n').enumerate() {
        let lineno = idx + 1;
        let line = line.strip_suffix('\r').unwrap_or(line);
        let stripped = line.trim();
        if stripped.is_empty() || stripped.starts_with('#') {
            continue;
        }

        let cells: Vec<&str> = line.split('\t').collect();
        let where_ = format!("{name}:{lineno}");

        if !header_seen {
            let got: Vec<&str> = cells.iter().take(HEADER.len()).map(|c| c.trim()).collect();
            if got.as_slice() != HEADER.as_slice() {
                return Err(format!(
                    "{where_}: first non-comment line must be the header {HEADER:?}, got {got:?}"
                ));
            }
            header_seen = true;
            continue;
        }

        if cells.len() < 2 {
            return Err(format!(
                "{where_}: expected tab-separated input/expected/config/note, got {} column(s). \
                 Is the separator a real tab?",
                cells.len()
            ));
        }
        if cells.len() > HEADER.len() {
            return Err(format!(
                "{where_}: {} columns, expected at most {}",
                cells.len(),
                HEADER.len()
            ));
        }

        let cell = |i: usize| cells.get(i).map(|c| c.trim()).unwrap_or("").to_string();
        let input = cell(0);
        if input.is_empty() {
            return Err(format!("{where_}: empty input column"));
        }
        let config_spec = cell(2);

        rows.push(Row {
            config: parse_config(&config_spec, &where_)?,
            file: name.clone(),
            lineno,
            input,
            expected: cell(1),
            config_spec,
            note: cell(3),
        });
    }

    if !header_seen {
        return Err(format!("{name}: no header line found"));
    }
    Ok(rows)
}

/// Every TSV under `tests/data`, sorted so ids are stable across machines.
fn corpus_files() -> Result<Vec<PathBuf>, String> {
    let dir = data_dir();
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .map_err(|e| format!("cannot read {}: {e}", dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "tsv"))
        .collect();
    files.sort();
    if files.is_empty() {
        return Err(format!("no *.tsv found under {}", dir.display()));
    }
    Ok(files)
}

fn load_corpus() -> Vec<Row> {
    let mut rows = Vec::new();
    for path in corpus_files().unwrap_or_else(|e| panic!("corpus discovery failed: {e}")) {
        match load_tsv(&path) {
            Ok(mut r) => rows.append(&mut r),
            Err(e) => panic!("corpus is malformed: {e}"),
        }
    }
    rows
}

// ---------------------------------------------------------------------------
// per-row checks
// ---------------------------------------------------------------------------

/// Digits the library exists to remove: ASCII and Arabic-Indic.
fn digits_in(s: &str) -> Vec<char> {
    s.chars()
        .filter(|c| c.is_ascii_digit() || ('\u{0660}'..='\u{0669}').contains(c))
        .collect()
}

/// Vowel points and cantillation only. Maqaf (05BE), paseq (05C0), sof pasuq (05C3),
/// nun hafukha (05C6), geresh (05F3) and gershayim (05F4) are punctuation and are
/// allowed to survive into the output.
fn niqqud_in(s: &str) -> Vec<char> {
    s.chars()
        .filter(|c| {
            matches!(*c as u32,
                0x0591..=0x05BD | 0x05BF | 0x05C1..=0x05C2 | 0x05C4..=0x05C5 | 0x05C7)
        })
        .collect()
}

fn check_row(row: &Row, failures: &mut Vec<String>) {
    let actual = normalize(&row.input, &row.config);

    if actual != row.expected {
        failures.push(row.describe(&actual));
    }

    // Normalizing spoken output again must be a no-op: a rule that rewrites its own
    // output loops or corrupts on a second pass, and callers do re-run this over
    // already-clean text.
    let twice = normalize(&actual, &row.config);
    if twice != actual {
        failures.push(format!(
            "{}: normalize() is not idempotent\n  input  {}\n  once   {}\n  twice  {}",
            row.id(),
            row.input,
            actual,
            twice
        ));
    }

    let id = row.id();
    // A row whose expected equals its input asserts the text is left alone (a URL, a
    // filename, an unparseable date). Its digits are deliberate.
    if row.expected != row.input && !row.note.starts_with(KEEPS_DIGIT) {
        let found = digits_in(&actual);
        if !found.is_empty() {
            failures.push(format!(
                "{id}: output still contains digits {found:?}\n  input  {}\n  actual {actual}\n  \
                 If this row is meant to keep a digit, start its note with \
                 \"keeps-digit:\" and say why.",
                row.input
            ));
        }
    }

    // Niqqud the *rules* introduced. The library does not strip the niqqud a text
    // arrives with, and the OPUS rows do arrive with some, so the assertion is about
    // what comes out that did not go in.
    let niqqud: Vec<char> = niqqud_in(&actual)
        .into_iter()
        .filter(|c| !row.input.contains(*c))
        .collect();
    if !niqqud.is_empty() {
        let points: Vec<String> = niqqud
            .iter()
            .map(|c| format!("U+{:04X}", *c as u32))
            .collect();
        failures.push(format!(
            "{id}: output has niqqud the input did not: {}\n  input  {}\n  actual {actual}",
            points.join(", "),
            row.input
        ));
    }

    // `expected` is post-finalize output: stripped and single-spaced.
    if row.expected != row.expected.trim() {
        failures.push(format!("{id}: expected column has surrounding whitespace"));
    }
    if row.expected.contains("  ") {
        failures.push(format!("{id}: expected column contains a double space"));
    }
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[test]
fn corpus() {
    let rows = load_corpus();
    assert!(
        rows.len() > 1000,
        "only {} rows loaded — corpus discovery is broken",
        rows.len()
    );

    let mut failures: Vec<String> = Vec::new();
    for row in &rows {
        check_row(row, &mut failures);
    }

    if !failures.is_empty() {
        let mut report = String::new();
        for f in &failures {
            report.push('\n');
            report.push_str(&"-".repeat(72));
            report.push('\n');
            report.push_str(f);
            report.push('\n');
        }
        report.push_str(&"-".repeat(72));
        panic!(
            "{report}\n\n{} failure(s) across {} corpus rows.",
            failures.len(),
            rows.len()
        );
    }
}

#[test]
fn every_tsv_parses() {
    // Separate from `corpus` so a malformed TSV fails with its own message rather than
    // being buried under a thousand Hebrew diffs.
    for path in corpus_files().expect("corpus files") {
        load_tsv(&path).unwrap_or_else(|e| panic!("{e}"));
    }
}

#[test]
fn empty_string() {
    assert_eq!(normalize("", &Config::default()), "");
}

#[test]
fn whitespace_only() {
    assert_eq!(normalize("   \n  ", &Config::default()), "");
}

#[test]
fn plain_hebrew_unchanged() {
    // Nothing to normalize means nothing changes.
    for text in [
        "שלום עולם",
        "הוא הלך לבית הספר ולא חזר",
        "מה נשמע? הכל בסדר, תודה.",
    ] {
        assert_eq!(normalize(text, &Config::default()), text);
    }
}

#[test]
fn empty_config_column_is_defaults() {
    // Guards the loader itself: an empty config column means the real defaults.
    assert_eq!(
        parse_config("", "<inline>").expect("empty spec"),
        Config::default()
    );
}

#[test]
fn config_column_is_parsed() {
    let cfg = parse_config("clock=24;expand_units=false", "<inline>").expect("spec");
    assert_eq!(cfg.clock, heb_tts_normalizer::Clock::H24);
    assert!(!cfg.expand_units);
    assert_eq!(
        cfg.date_order,
        Config::default().date_order,
        "untouched keys keep defaults"
    );
}

#[test]
fn unknown_config_key_is_an_error() {
    // A typo must never silently test the defaults.
    let err = parse_config("clok=24", "dates.tsv:7").expect_err("typo must fail");
    assert!(err.contains("dates.tsv:7"), "{err}");
    assert!(err.contains("unknown config key"), "{err}");
}

#[test]
fn bad_enum_value_is_an_error() {
    let err = parse_config("clock=25", "times.tsv:9").expect_err("bad enum must fail");
    assert!(err.contains("times.tsv:9"), "{err}");
    assert!(err.contains("12, 24"), "{err}");
}

#[test]
fn bad_bool_value_is_an_error() {
    let err = parse_config("expand_units=maybe", "units.tsv:3").expect_err("bad bool must fail");
    assert!(err.contains("units.tsv:3"), "{err}");
}

#[test]
fn gender_overrides_parse() {
    let cfg = parse_config("gender_overrides=סטוריז:m", "<inline>").expect("spec");
    assert_eq!(
        cfg.gender_overrides.get("סטוריז"),
        Some(&heb_tts_normalizer::Gender::Masc)
    );
}
