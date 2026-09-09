//! The common case: a paragraph of real Hebrew, ready for a g2p.
//!
//! ```text
//! cargo run --example basic
//! ```

use heb_tts_normalizer::{Config, normalize};

const TEXT: &str = "\
ד״ר כהן פרסם היום מחקר על 1,200 נבדקים.
המחיר עלה ב-12% ל-₪25.50, והישיבה נדחתה ל-12/03/2026 בשעה 14:30.
המרחק הוא 5 ק״מ, וזה ייקח בין 3 ל-5 שעות.
לפרטים חייגו 03-1234567 או היכנסו ל-https://example.com/2026.";

fn main() {
    let cfg = Config::default();
    for line in TEXT.lines() {
        println!("  {line}");
        println!("→ {}\n", normalize(line, &cfg));
    }
}
