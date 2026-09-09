//! Clock times: `14:30`, `9:05`, `20:00`, `14:30:15` and `9 בבוקר`.
//!
//! Judgment calls made here:
//!
//! - **Hours are feminine** (שעה is feminine): 14 is "ארבע עשרה", 2 is "שתיים". Minutes
//!   are counted the same way, for the same reason (דקה is feminine).
//! - **`cfg.clock` picks the register.** `Clock::H24` reads the digits as they are
//!   written — "ארבע עשרה שלושים", "עשרים אפס אפס" — the way a timetable or a radio
//!   bulletin is read. `Clock::H12` (the default) converts and adds the daypart, the
//!   way a person answers "what time is it".
//! - **A minute under ten keeps its zero in H24** ("תשע אפס חמש"), because that is what
//!   the written form says; in H12 it becomes a plain conjunction ("תשע וחמש").
//! - **Dayparts** by the 24-hour hour: 05:00-11:59 בבוקר, 12:00-16:59 אחר הצהריים,
//!   17:00-20:59 בערב, 21:00-04:59 בלילה. Two overrides: the whole 12th hour is
//!   בצהריים ("שתים עשרה וחצי בצהריים" is what people say, not "אחר הצהריים"), and
//!   00:00 exactly is חצות — nobody says "שתים עשרה בלילה" for midnight on the dot.
//! - **The quarter-to idiom keeps the daypart of the hour it is spoken in**, not of the
//!   hour it counts towards: 16:45 is "רבע לחמש אחר הצהריים", because at 16:45 it is
//!   still the afternoon.
//! - **Seconds break every idiom**, so `h:mm:ss` is always read plainly, in both clock
//!   configs — a timestamp is a timestamp, not a time of day.
//! - Declines anything outside a real clock (`25:70`), which is also what keeps scores
//!   and ratios out: those are written `2:1`, and a two-digit minute is required.

use fancy_regex::{Captures, Regex};
use once_cell::sync::Lazy;

use crate::config::{Clock, Config, Gender};
use crate::numerals::{digits, numeral};
use crate::scanner::{Rule, priority};

const MORNING: &str = "בבוקר";
const AFTERNOON: &str = "אחר הצהריים";
const NOON: &str = "בצהריים";
const EVENING: &str = "בערב";
const NIGHT: &str = "בלילה";
const MIDNIGHT: &str = "חצות";

/// Daypart words a written time may spell out itself ("9 בבוקר"). Longest first, so
/// nothing shadows a spelling that starts the same way.
const SPOKEN_DAYPARTS: [&str; 5] = [AFTERNOON, MORNING, NOON, EVENING, NIGHT];

/// The same, as they appear *after* a clock time ("9:00 בבוקר"), plus the abbreviation
/// the abbrev rule has not reached yet at this point in the scan. Anchored, because it
/// is tested against the tail of the text starting at the end of the match.
static WRITTEN_DAYPART: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!(
        r"^\s+(?:{}|אחה״צ|לפנות בוקר)(?![א-ת])",
        SPOKEN_DAYPARTS.join("|")
    ))
    .expect("WRITTEN_DAYPART")
});

fn daypart(hour24: i64) -> &'static str {
    match hour24 {
        12 => NOON,
        5..=11 => MORNING,
        13..=16 => AFTERNOON,
        17..=20 => EVENING,
        _ => NIGHT,
    }
}

/// The 12-hour face value, feminine. Both 0 and 12 read as שתים עשרה.
fn hour_word(hour24: i64) -> String {
    let face = hour24 % 12;
    numeral(if face == 0 { 12 } else { face }, Gender::Fem, false)
}

/// H24 minute/second reading: keep the leading zero of anything under ten.
fn minutes_plain(value: i64, raw: &str) -> String {
    if value < 10 {
        digits(raw)
    } else {
        numeral(value, Gender::Fem, false)
    }
}

fn render_h24(hour: i64, minute: i64, raw_minute: &str) -> String {
    format!(
        "{} {}",
        numeral(hour, Gender::Fem, false),
        minutes_plain(minute, raw_minute)
    )
}

/// `part = Some("")` suppresses the daypart, for when the text already spells one out.
fn render_h12(hour: i64, minute: i64, part: Option<&str>) -> String {
    if hour == 0 && minute == 0 {
        return MIDNIGHT.to_string();
    }
    let part = part.unwrap_or_else(|| daypart(hour));
    match minute {
        0 => format!("{} {part}", hour_word(hour)),
        15 => format!("{} ורבע {part}", hour_word(hour)),
        30 => format!("{} וחצי {part}", hour_word(hour)),
        // Counted towards the next hour, but spoken in the current one's daypart.
        45 => format!("רבע ל{} {part}", hour_word(hour + 1)),
        _ => format!(
            "{} ו{} {part}",
            hour_word(hour),
            numeral(minute, Gender::Fem, false)
        ),
    }
}

fn capture<'t>(m: &Captures<'t>, index: usize) -> Option<&'t str> {
    m.get(index).map(|g| g.as_str())
}

/// `14:30`, `9:05`, `20:00`, `14:30:15`.
struct ClockTimeRule {
    re: Regex,
}

impl ClockTimeRule {
    fn new() -> Self {
        Self {
            re: Regex::new(r"(?<![\d:.])(\d{1,2}):(\d{2})(?::(\d{2}))?(?![\d:])")
                .expect("time-clock"),
        }
    }
}

impl Rule for ClockTimeRule {
    fn name(&self) -> &'static str {
        "time-clock"
    }

    fn priority(&self) -> i32 {
        priority::TIME
    }

    fn pattern(&self) -> &Regex {
        &self.re
    }

    fn render(&self, m: &Captures<'_>, text: &str, cfg: &Config) -> Option<String> {
        let raw_minute = capture(m, 2)?;
        let hour: i64 = capture(m, 1)?.parse().ok()?;
        let minute: i64 = raw_minute.parse().ok()?;
        let raw_second = capture(m, 3);
        let second: Option<i64> = match raw_second {
            Some(s) => Some(s.parse().ok()?),
            None => None,
        };
        if hour > 23 || minute > 59 || second.is_some_and(|s| s > 59) {
            return None;
        }
        if let (Some(second), Some(raw_second)) = (second, raw_second) {
            // Seconds present: read the three groups as written, whatever the config.
            let plain = render_h24(hour, minute, raw_minute);
            return Some(format!("{plain} {}", minutes_plain(second, raw_second)));
        }
        if cfg.clock == Clock::H24 {
            return Some(render_h24(hour, minute, raw_minute));
        }
        // "9:00 בבוקר" already says which part of the day it is; adding ours would
        // read "בתשע בבוקר בבוקר".
        let tail = &text[m.get(0)?.end()..];
        let part = match WRITTEN_DAYPART.is_match(tail) {
            Ok(true) => Some(""),
            _ => None,
        };
        Some(render_h12(hour, minute, part).trim_end().to_string())
    }
}

/// `9 בבוקר`, `8 בערב`, `3 אחר הצהריים` — the daypart is already a word.
struct SpokenHourRule {
    re: Regex,
}

impl SpokenHourRule {
    fn new() -> Self {
        Self {
            re: Regex::new(&format!(
                r"(?<![\d:.])(\d{{1,2}})\s+({})(?![א-ת])",
                SPOKEN_DAYPARTS.join("|")
            ))
            .expect("time-spoken-hour"),
        }
    }
}

impl Rule for SpokenHourRule {
    fn name(&self) -> &'static str {
        "time-spoken-hour"
    }

    fn priority(&self) -> i32 {
        priority::TIME
    }

    fn pattern(&self) -> &Regex {
        &self.re
    }

    fn render(&self, m: &Captures<'_>, _text: &str, _cfg: &Config) -> Option<String> {
        let hour: i64 = capture(m, 1)?.parse().ok()?;
        // Only a clock-face hour can carry a daypart; "24 בערב" is not a time.
        if !(1..=12).contains(&hour) {
            return None;
        }
        Some(format!(
            "{} {}",
            numeral(hour, Gender::Fem, false),
            capture(m, 2)?
        ))
    }
}

pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(ClockTimeRule::new()),
        Box::new(SpokenHourRule::new()),
    ]
}
