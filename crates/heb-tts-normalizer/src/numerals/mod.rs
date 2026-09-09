//! Hebrew numerals: the words, the agreement, and the noun gender behind it.

pub mod convert;
pub mod gender;
pub mod words;

pub use convert::{decimal, digits, numeral, ordinal};
pub use gender::{count_phrase, noun_gender};
