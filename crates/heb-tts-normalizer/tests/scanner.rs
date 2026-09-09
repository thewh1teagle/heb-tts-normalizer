//! Scanner semantics, exercised with rules defined here rather than the real ones.
//!
//! The scanner's contract — priority, longest-match, declining, frozen spans — has to
//! hold independently of what any particular rule does, so these tests deliberately use
//! toy rules. A change in the dates rule must never be able to break this file.
//!
//! The Rust port adds one concern Python never had: the scanner walks *byte* offsets
//! over UTF-8, and every Hebrew letter is two bytes. A slice at a non-boundary is a
//! panic, not a wrong answer, so the multi-byte section at the bottom is not optional.

use std::sync::{Arc, Mutex};

use fancy_regex::{Captures, Regex};
use heb_tts_normalizer::{Config, Rule, scan};

/// Every match text a rule was offered, in order — for span-freezing assertions.
///
/// Shared out of the rule before it is boxed, since `scan` takes ownership of the
/// boxes and a `Rule` is `&self` everywhere.
#[derive(Clone, Default)]
struct Seen(Arc<Mutex<Vec<String>>>);

impl Seen {
    fn get(&self) -> Vec<String> {
        self.0.lock().expect("seen lock").clone()
    }
}

type DeclineFn = Box<dyn Fn(&str) -> bool + Send + Sync>;

/// A rule whose behaviour is spelled out at the call site.
struct FakeRule {
    name: &'static str,
    priority: i32,
    pattern: Regex,
    /// Rendered output. `None` declines every match; `decline_when` decides per match.
    output: Option<String>,
    decline_when: Option<DeclineFn>,
    seen: Seen,
}

impl FakeRule {
    fn new(name: &'static str, priority: i32, pattern: &str, output: Option<&str>) -> Self {
        Self {
            name,
            priority,
            pattern: Regex::new(pattern).expect("test pattern"),
            output: output.map(str::to_string),
            decline_when: None,
            seen: Seen::default(),
        }
    }

    fn declining_when(mut self, f: impl Fn(&str) -> bool + Send + Sync + 'static) -> Self {
        self.decline_when = Some(Box::new(f));
        self
    }

    fn seen(&self) -> Seen {
        self.seen.clone()
    }
}

impl Rule for FakeRule {
    fn name(&self) -> &'static str {
        self.name
    }

    fn priority(&self) -> i32 {
        self.priority
    }

    fn pattern(&self) -> &Regex {
        &self.pattern
    }

    fn render(&self, m: &Captures<'_>, _text: &str, _cfg: &Config) -> Option<String> {
        let whole = m.get(0).expect("group 0").as_str();
        self.seen
            .0
            .lock()
            .expect("seen lock")
            .push(whole.to_string());
        if let Some(decline) = &self.decline_when {
            if decline(whole) {
                return None;
            }
        }
        self.output.clone()
    }
}

/// `vec![Box::new(a), Box::new(b)]` without the ceremony.
macro_rules! rules {
    ($($r:expr),* $(,)?) => {
        {
            let v: Vec<Box<dyn Rule>> = vec![$(Box::new($r)),*];
            v
        }
    };
}

fn cfg() -> Config {
    Config::default()
}

// ---------------------------------------------------------------------------
// the basics
// ---------------------------------------------------------------------------

#[test]
fn untouched_text_is_byte_identical() {
    let r = FakeRule::new("digits", 10, r"\d+", Some("N"));
    let seen = r.seen();
    assert_eq!(scan("שלום עולם", &rules![r], &cfg()), "שלום עולם");
    assert!(seen.get().is_empty());
}

#[test]
fn single_match_is_replaced() {
    let r = FakeRule::new("digits", 10, r"\d+", Some("מספר"));
    assert_eq!(scan("יש 42 כאן", &rules![r], &cfg()), "יש מספר כאן");
}

#[test]
fn empty_rule_list_is_a_no_op() {
    assert_eq!(scan("14:30 ו-3 ק״מ", &[], &cfg()), "14:30 ו-3 ק״מ");
}

#[test]
fn empty_text() {
    let r = FakeRule::new("digits", 50, r"\d+", Some("N"));
    assert_eq!(scan("", &rules![r], &cfg()), "");
}

#[test]
fn unmatched_text_between_spans_is_preserved_exactly() {
    let r = FakeRule::new("digits", 50, r"\d+", Some("#"));
    assert_eq!(scan("  1, 2 -- 3  ", &rules![r], &cfg()), "  #, # -- #  ");
}

#[test]
fn rules_run_left_to_right() {
    let r = FakeRule::new("digits", 50, r"\d+", Some("N"));
    let seen = r.seen();
    scan("1 22 333", &rules![r], &cfg());
    assert_eq!(seen.get(), ["1", "22", "333"]);
}

// ---------------------------------------------------------------------------
// priority
// ---------------------------------------------------------------------------

#[test]
fn higher_priority_wins_at_the_same_position() {
    let low = FakeRule::new("low", 10, r"\d+", Some("LOW"));
    let low_seen = low.seen();
    let high = FakeRule::new("high", 90, r"\d+", Some("HIGH"));
    assert_eq!(scan("12", &rules![low, high], &cfg()), "HIGH");
    assert!(
        low_seen.get().is_empty(),
        "a lower-priority rule is never consulted once a higher one renders"
    );

    // Registration order must not matter; the registry sorts, and so does the scanner.
    let low = FakeRule::new("low", 10, r"\d+", Some("LOW"));
    let high = FakeRule::new("high", 90, r"\d+", Some("HIGH"));
    assert_eq!(scan("12", &rules![high, low], &cfg()), "HIGH");
}

#[test]
fn higher_priority_wins_even_with_a_shorter_match() {
    // Priority beats length across priorities — that is what the scheme is for.
    let short_high = FakeRule::new("short-high", 90, r"\d\d", Some("HH"));
    let long_low = FakeRule::new("long-low", 10, r"\d+", Some("LLLL"));
    assert_eq!(
        scan("123456", &rules![short_high, long_low], &cfg()),
        "HHHHHH"
    );
}

#[test]
fn longest_match_wins_within_a_priority() {
    let two = FakeRule::new("two", 50, r"\d\d", Some("TWO"));
    let two_seen = two.seen();
    let four = FakeRule::new("four", 50, r"\d\d\d\d", Some("FOUR"));
    assert_eq!(scan("1234", &rules![two, four], &cfg()), "FOUR");
    assert!(two_seen.get().is_empty());
}

// ---------------------------------------------------------------------------
// declining
// ---------------------------------------------------------------------------

#[test]
fn declining_falls_through_to_the_next_candidate() {
    // A rule that matches but cannot parse hands the span back.
    let build = || {
        let picky = FakeRule::new("picky", 90, r"\d+", Some("PICKY"))
            .declining_when(|s| s.parse::<i64>().unwrap_or(0) > 12);
        let seen = picky.seen();
        let fallback = FakeRule::new("fallback", 10, r"\d+", Some("FALLBACK"));
        (rules![picky, fallback], seen)
    };

    let (rs, seen_a) = build();
    assert_eq!(scan("7", &rs, &cfg()), "PICKY");
    assert_eq!(seen_a.get(), ["7"]);

    let (rs, seen_b) = build();
    assert_eq!(scan("99", &rs, &cfg()), "FALLBACK");
    assert_eq!(
        seen_b.get(),
        ["99"],
        "the declining rule was offered the span"
    );
}

#[test]
fn declining_within_one_priority_falls_to_the_shorter_match() {
    let long_decliner = FakeRule::new("long", 50, r"\d\d\d\d", None);
    let short = FakeRule::new("short", 50, r"\d\d", Some("SS"));
    assert_eq!(scan("1234", &rules![long_decliner, short], &cfg()), "SSSS");
}

#[test]
fn a_rule_that_declines_everywhere_leaves_text_untouched() {
    let never = FakeRule::new("never", 90, r"\d+", None);
    let seen = never.seen();
    let text = "יש לי 3 ילדים ו-25 שקלים";
    assert_eq!(scan(text, &rules![never], &cfg()), text);
    // After a decline the scanner emits one character and retries at the next position,
    // so the tail of a declined span is offered again ("25" declines, then "5" is tried).
    assert_eq!(
        seen.get(),
        ["3", "25", "5"],
        "it was tried, it just never accepted"
    );
}

// ---------------------------------------------------------------------------
// frozen spans
// ---------------------------------------------------------------------------

#[test]
fn matched_spans_are_frozen() {
    // A rule's output is never re-scanned, even when it looks like another rule's input.
    let producer = FakeRule::new("producer", 90, r"A", Some("B"));
    let consumer = FakeRule::new("consumer", 90, r"B", Some("C"));
    let consumer_seen = consumer.seen();
    assert_eq!(scan("A", &rules![producer, consumer], &cfg()), "B");
    assert!(consumer_seen.get().is_empty());
}

#[test]
fn a_rule_does_not_re_match_its_own_output() {
    // The obvious infinite-loop shape: a rule that emits what it matches.
    let r = FakeRule::new("echo", 50, r"\d+", Some("12"));
    let seen = r.seen();
    assert_eq!(scan("999", &rules![r], &cfg()), "12");
    assert_eq!(seen.get(), ["999"]);
}

// ---------------------------------------------------------------------------
// matching at a position, not on a slice
// ---------------------------------------------------------------------------

#[test]
fn word_boundary_anchors_mid_string() {
    // `\b` must see the character before the scan position, not a fresh string start.
    // Slicing the text at `pos` instead of matching from it would make `\bcat` match
    // inside `bobcat`.
    let r = FakeRule::new("cat", 50, r"\bcat\b", Some("CAT"));
    let seen = r.seen();
    let rs = rules![r];
    assert_eq!(scan("a cat here", &rs, &cfg()), "a CAT here");
    assert_eq!(scan("bobcat", &rs, &cfg()), "bobcat");
    assert_eq!(seen.get(), ["cat"]);
}

#[test]
fn lookbehind_sees_preceding_text() {
    let r = FakeRule::new("after-dash", 50, r"(?<=-)\d+", Some("N"));
    assert_eq!(scan("-25 and 25", &rules![r], &cfg()), "-N and 25");
}

#[test]
fn zero_width_match_is_ignored() {
    // An empty match would advance nowhere; the scanner must not accept it.
    let empty = FakeRule::new("empty", 90, r"\d*", Some("X"));
    let noop = FakeRule::new("noop", 10, r"zzz", Some("Z"));
    assert_eq!(scan("abc", &rules![empty, noop], &cfg()), "abc");
}

#[test]
fn zero_width_match_does_not_block_a_real_one() {
    let empty = FakeRule::new("empty", 90, r"\d*", Some("X"));
    let real = FakeRule::new("real", 50, r"[a-z]+", Some("W"));
    assert_eq!(scan("abc", &rules![empty, real], &cfg()), "W");
}

#[test]
fn config_reaches_render() {
    struct Recorder {
        pattern: Regex,
        saw: Arc<Mutex<Vec<String>>>,
    }
    impl Rule for Recorder {
        fn name(&self) -> &'static str {
            "recorder"
        }
        fn priority(&self) -> i32 {
            50
        }
        fn pattern(&self) -> &Regex {
            &self.pattern
        }
        fn render(&self, _m: &Captures<'_>, _text: &str, cfg: &Config) -> Option<String> {
            self.saw
                .lock()
                .expect("lock")
                .push(cfg.decimal_word.clone());
            Some("N".to_string())
        }
    }

    let saw: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let rules: Vec<Box<dyn Rule>> = vec![Box::new(Recorder {
        pattern: Regex::new(r"\d+").expect("pattern"),
        saw: Arc::clone(&saw),
    })];

    let c = Config {
        decimal_word: "פסיק".to_string(),
        ..Config::default()
    };
    assert_eq!(scan("5", &rules, &c), "N");
    assert_eq!(saw.lock().expect("lock").as_slice(), ["פסיק"]);
}

#[test]
fn render_sees_the_whole_text_and_its_own_offset() {
    // How a rule inspects what precedes it — the `פי` idiom does exactly this.
    struct Offsets {
        pattern: Regex,
    }
    impl Rule for Offsets {
        fn name(&self) -> &'static str {
            "offsets"
        }
        fn priority(&self) -> i32 {
            50
        }
        fn pattern(&self) -> &Regex {
            &self.pattern
        }
        fn render(&self, m: &Captures<'_>, text: &str, _cfg: &Config) -> Option<String> {
            let start = m.get(0).expect("group 0").start();
            // Slicing at the match start must always be a char boundary.
            let before = &text[..start];
            Some(
                if before.ends_with("פי ") {
                    "IDIOM"
                } else {
                    "PLAIN"
                }
                .to_string(),
            )
        }
    }

    let rules: Vec<Box<dyn Rule>> = vec![Box::new(Offsets {
        pattern: Regex::new(r"\d+").expect("pattern"),
    })];
    assert_eq!(scan("גדל פי 3", &rules, &cfg()), "גדל פי IDIOM");
    assert_eq!(scan("יש 3", &rules, &cfg()), "יש PLAIN");
}

// ---------------------------------------------------------------------------
// multi-byte offsets — the Rust-specific hazard
// ---------------------------------------------------------------------------

#[test]
fn a_rule_matching_hebrew_does_not_slice_mid_character() {
    // Every letter here is two bytes, so a scanner counting characters would slice
    // halfway through one and panic.
    let r = FakeRule::new("shalom", 50, r"שלום", Some("היי"));
    let seen = r.seen();
    assert_eq!(
        scan("אמרתי שלום לכולם", &rules![r], &cfg()),
        "אמרתי היי לכולם"
    );
    assert_eq!(seen.get(), ["שלום"]);
}

#[test]
fn skip_ahead_lands_on_a_char_boundary() {
    // Nothing matches until deep inside a run of two-byte letters, so the scanner takes
    // its "jump to the earliest possible match" path across multi-byte text. The jump
    // target comes from the regex, which reports byte offsets on boundaries — and the
    // copied prefix must come out byte-identical.
    let r = FakeRule::new("digits", 50, r"\d+", Some("מספר"));
    let text = "פסקה ארוכה של עברית בלבד עד שמופיע 42 ואז שוב עברית";
    assert_eq!(
        scan(text, &rules![r], &cfg()),
        "פסקה ארוכה של עברית בלבד עד שמופיע מספר ואז שוב עברית"
    );
}

#[test]
fn declining_over_hebrew_advances_by_a_whole_character() {
    // The decline path advances one *character*, not one byte. With two-byte letters a
    // byte-wise advance would panic on the next slice, or hand a rule half a letter.
    let never = FakeRule::new(
        "never",
        50,
        &format!(r"[{}]+", heb_tts_normalizer::text::LETTER),
        None,
    );
    let seen = never.seen();
    assert_eq!(scan("שלום", &rules![never], &cfg()), "שלום");
    assert_eq!(seen.get(), ["שלום", "לום", "ום", "ם"]);
}

#[test]
fn hebrew_lookbehind_across_a_multi_byte_prefix() {
    // Variable-width lookbehind over two-byte letters: the reason this crate uses
    // fancy-regex at all, and the place a byte/char mix-up would surface first.
    let r = FakeRule::new("after-bet", 50, r"(?<=ב-)\d+", Some("מספר"));
    assert_eq!(
        scan("עלה ב-25 אחוז ואז 25", &rules![r], &cfg()),
        "עלה ב-מספר אחוז ואז 25"
    );
}

#[test]
fn mixed_script_text_round_trips() {
    // Latin (1 byte), Hebrew (2 bytes), an emoji (4 bytes) and a digit run, so every
    // UTF-8 width appears on both sides of a replacement.
    let r = FakeRule::new("digits", 50, r"\d+", Some("N"));
    let text = "hello שלום 🎉 42 שלום world";
    assert_eq!(scan(text, &rules![r], &cfg()), "hello שלום 🎉 N שלום world");
}

#[test]
fn a_match_ending_at_the_last_byte_is_fine() {
    let rs = rules![FakeRule::new("tail", 50, r"\d+", Some("סוף"))];
    assert_eq!(scan("המחיר 25", &rs, &cfg()), "המחיר סוף");
    assert_eq!(scan("25", &rs, &cfg()), "סוף");
}
