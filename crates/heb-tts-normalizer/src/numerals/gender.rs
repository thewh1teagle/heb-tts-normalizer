//! Noun gender resolution and number-noun agreement.
//!
//! Resolution order: caller overrides, then a built-in lexicon, then explicit exception
//! sets, then suffix heuristics. The exceptions come before the heuristics because they
//! exist precisely to contradict them (נשים looks masculine, מקומות looks feminine).
//!
//! A leading one-letter clitic (ה/ו/ב/ל/כ/מ/ש) is stripped only as a *fallback* lookup: we
//! try the word as written first, so מים stays water and never becomes מ + ים.

use crate::config::{Config, Gender};
use crate::numerals::convert::numeral;
use crate::numerals::words as w;

/// Best guess at the grammatical gender of `word`.
pub fn noun_gender(word: &str, cfg: &Config) -> Gender {
    let forms = lookup_forms(word);
    for form in &forms {
        if let Some(&over) = cfg.gender_overrides.get(*form) {
            return over;
        }
    }
    for form in &forms {
        if let Some(&known) = w::LEXICON.get(*form) {
            return known;
        }
    }
    for form in &forms {
        if w::FEM_DESPITE_IM.contains(*form) || w::FEM_DESPITE_NO_SUFFIX.contains(*form) {
            return Gender::Fem;
        }
        if w::MASC_DESPITE_OT.contains(*form) || w::MASC_DESPITE_FEM_SUFFIX.contains(*form) {
            return Gender::Masc;
        }
    }
    // The suffix heuristic always runs on the original word, never a peeled form.
    by_suffix(word)
}

/// Numeral + noun, agreeing in gender and correctly ordered.
///
/// Two Hebrew rules drive the shape: the numeral 1 *follows* its noun (ילד אחד), and a
/// definite noun takes the numeral's construct form (שלושת הילדים).
pub fn count_phrase(n: i64, noun: &str, cfg: &Config) -> String {
    let gender = noun_gender(noun, cfg);
    let definite = is_definite(noun);

    if n == 1 {
        return format!("{} {}", noun, numeral(1, gender, false));
    }
    // 2 is always bound, definite or not: שני ילדים, שתי ילדות, שני הילדים.
    if n == 2 || (definite && (3..=10).contains(&n)) {
        return format!("{} {}", numeral(n, gender, true), noun);
    }
    format!("{} {}", numeral(n, gender, false), noun)
}

fn is_definite(noun: &str) -> bool {
    noun.chars().count() > 2 && noun.starts_with('ה')
}

/// The word as written, then with one and two leading clitics peeled off.
fn lookup_forms(word: &str) -> Vec<&str> {
    let mut forms = vec![word];
    let mut current = word;
    for _ in 0..2 {
        let mut chars = current.chars();
        match chars.next() {
            Some(first) if current.chars().count() > 2 && w::CLITICS.contains(&first) => {
                current = chars.as_str();
                forms.push(current);
            }
            _ => break,
        }
    }
    forms
}

fn by_suffix(word: &str) -> Gender {
    if word.ends_with("ות") {
        return Gender::Fem;
    }
    if word.ends_with("ים") || word.ends_with("יים") {
        return Gender::Masc;
    }
    if word.ends_with('ה') || word.ends_with('ת') {
        return Gender::Fem;
    }
    Gender::Masc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> Config {
        Config::default()
    }

    #[test]
    fn count_phrase_examples() {
        let c = cfg();
        assert_eq!(count_phrase(3, "ילדים", &c), "שלושה ילדים");
        assert_eq!(count_phrase(3, "ילדות", &c), "שלוש ילדות");
        assert_eq!(count_phrase(2, "ילדים", &c), "שני ילדים");
        assert_eq!(count_phrase(3, "הילדים", &c), "שלושת הילדים");
        assert_eq!(count_phrase(1, "ילד", &c), "ילד אחד");
        assert_eq!(count_phrase(3, "נשים", &c), "שלוש נשים");
    }

    #[test]
    fn definite_feminine_reuses_absolute_form() {
        let c = cfg();
        assert_eq!(count_phrase(3, "הילדות", &c), "שלוש הילדות");
        assert_eq!(count_phrase(2, "הילדות", &c), "שתי הילדות");
        // Past ten a definite noun no longer takes a bound numeral.
        assert_eq!(count_phrase(11, "הילדים", &c), "אחד עשר הילדים");
    }

    #[test]
    fn lexicon_and_exceptions() {
        let c = cfg();
        assert_eq!(noun_gender("נשים", &c), Gender::Fem);
        assert_eq!(noun_gender("מקומות", &c), Gender::Masc);
        assert_eq!(noun_gender("לילה", &c), Gender::Masc);
        assert_eq!(noun_gender("עיר", &c), Gender::Fem);
        assert_eq!(noun_gender("שנים", &c), Gender::Fem);
        assert_eq!(noun_gender("כוכבית", &c), Gender::Masc);
    }

    #[test]
    fn clitics_are_a_fallback_only() {
        let c = cfg();
        // מים is in the lexicon as written, so the מ is never taken for a clitic.
        assert_eq!(noun_gender("מים", &c), Gender::Masc);
        assert_eq!(noun_gender("הילדות", &c), Gender::Fem);
        assert_eq!(noun_gender("והילדים", &c), Gender::Masc);
    }

    #[test]
    fn suffix_heuristic() {
        let c = cfg();
        assert_eq!(noun_gender("מכוניות", &c), Gender::Fem);
        assert_eq!(noun_gender("קווים", &c), Gender::Masc);
        assert_eq!(noun_gender("גלידה", &c), Gender::Fem);
        assert_eq!(noun_gender("זרזיר", &c), Gender::Masc);
    }

    #[test]
    fn overrides_win() {
        let mut c = Config::default();
        c.gender_overrides.insert("ילדים".into(), Gender::Fem);
        assert_eq!(noun_gender("ילדים", &c), Gender::Fem);
    }
}
