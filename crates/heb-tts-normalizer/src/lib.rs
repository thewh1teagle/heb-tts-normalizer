//! Normalize Hebrew text for TTS, so a g2p only ever sees words.
//!
//! ```
//! use heb_tts_normalizer::{normalize, Config};
//! assert_eq!(normalize("יש לי 3 ילדים ו-25₪", &Config::default()),
//!            "יש לי שלושה ילדים ועשרים וחמישה שקלים");
//! ```

pub mod config;
pub mod numerals;
pub mod rules;
pub mod scanner;
pub mod text;

pub use config::{Clock, Config, DateOrder, Gender, HebrewDateStyle};
pub use scanner::{Rule, normalize, priority, scan};

/// The crate version, for the C API and the wasm binding to report.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
