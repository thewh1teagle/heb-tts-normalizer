//! Rule registry. A rule module is self-contained; adding one means adding it here.

use once_cell::sync::Lazy;

use crate::scanner::Rule;

pub mod abbrev;
pub mod cleanup;
pub mod currency;
pub mod dates;
pub mod numbers;
pub mod percent;
pub mod phones;
pub mod protect;
pub mod ranges;
pub mod times;
pub mod units;

/// Every span rule, sorted by descending priority once at startup.
///
/// `cleanup` is deliberately absent: it rewrites the whole string, so it runs as a
/// pre-pass in [`crate::text::clean`] rather than as a span rule.
static ALL: Lazy<Vec<Box<dyn Rule>>> = Lazy::new(|| {
    let mut rules: Vec<Box<dyn Rule>> = Vec::new();
    rules.extend(protect::rules());
    rules.extend(dates::rules());
    rules.extend(times::rules());
    rules.extend(phones::rules());
    rules.extend(currency::rules());
    rules.extend(percent::rules());
    rules.extend(ranges::rules());
    rules.extend(units::rules());
    rules.extend(abbrev::rules());
    rules.extend(numbers::rules());
    rules.sort_by_key(|r| -r.priority());
    rules
});

/// The built-in rule set.
pub fn all() -> &'static [Box<dyn Rule>] {
    &ALL
}
