//! Adding a rule of your own.
//!
//! A rule is anything implementing [`Rule`]: a name, a priority, a pattern and a
//! `render`. Here: sports scores, which the built-in rules deliberately leave alone —
//! the times rule requires two-digit minutes precisely so that `3:1` is never mistaken
//! for a time.
//!
//! ```text
//! cargo run --example custom_rule
//! ```

use fancy_regex::{Captures, Regex};
use heb_tts_normalizer::numerals::numeral;
use heb_tts_normalizer::text::{clean, finalize};
use heb_tts_normalizer::{Config, Gender, Rule, priority, rules, scan};

/// `ניצחון 3:1` — two small numbers around a colon, read as a score.
struct ScoreRule {
    pattern: Regex,
}

impl ScoreRule {
    fn new() -> Self {
        Self {
            pattern: Regex::new(r"(?<![\d:])(\d{1,2}):(\d{1,2})(?![\d:])").expect("score pattern"),
        }
    }
}

impl Rule for ScoreRule {
    fn name(&self) -> &'static str {
        "score"
    }

    /// Below TIME on purpose: a real time wins, and this rule only ever sees what the
    /// times rule declined — `3:1` is not a time because minutes must be two digits.
    fn priority(&self) -> i32 {
        priority::TIME - 1
    }

    fn pattern(&self) -> &Regex {
        &self.pattern
    }

    fn render(&self, m: &Captures<'_>, _text: &str, _cfg: &Config) -> Option<String> {
        let home: i64 = m.get(1)?.as_str().parse().ok()?;
        let away: i64 = m.get(2)?.as_str().parse().ok()?;
        if home > 30 || away > 30 {
            return None; // decline, and let the other rules have it
        }
        Some(format!(
            "{} {}",
            numeral(home, Gender::Fem, false),
            numeral(away, Gender::Fem, false)
        ))
    }
}

/// The built-in set is a `&'static` slice of boxes, which cannot be cloned into a Vec.
/// A one-line forwarder puts each of them back into a box, so a rule of your own can
/// sit alongside them.
struct Builtin(&'static dyn Rule);

impl Rule for Builtin {
    fn name(&self) -> &'static str {
        self.0.name()
    }
    fn priority(&self) -> i32 {
        self.0.priority()
    }
    fn pattern(&self) -> &Regex {
        self.0.pattern()
    }
    fn render(&self, m: &Captures<'_>, text: &str, cfg: &Config) -> Option<String> {
        self.0.render(m, text, cfg)
    }
}

/// The built-in rules plus one of ours, sorted the way the registry sorts.
fn rule_set() -> Vec<Box<dyn Rule>> {
    let mut all: Vec<Box<dyn Rule>> = rules::all()
        .iter()
        .map(|r| Box::new(Builtin(r.as_ref())) as Box<dyn Rule>)
        .collect();
    all.push(Box::new(ScoreRule::new()));
    all.sort_by_key(|r| -r.priority());
    all
}

/// `normalize()` with a rule set of your choosing: the same clean / scan / finalize
/// sandwich the public function performs.
fn normalize_with(text: &str, rules: &[Box<dyn Rule>], cfg: &Config) -> String {
    finalize(&scan(&clean(text, cfg), rules, cfg), cfg)
}

fn main() {
    let cfg = Config::default();
    let rules = rule_set();

    for text in ["מכבי ניצחה 3:1", "המשחק הסתיים 2:2", "הפגישה ב-14:30"]
    {
        println!("  {text}");
        println!("→ {}\n", normalize_with(text, &rules, &cfg));
    }
}
