//! Hebrew number and noun-gender word tables.
//!
//! Everything here is data only — no logic — so the shapes stay easy to audit against a
//! grammar book. Two Hebrew quirks drive the layout:
//!
//! * Cardinals have two gender series, and the *feminine* series is the unmarked one used
//!   for bare counting ("שלוש נקודה חמש"), which is the opposite of what the morphology
//!   suggests: the feminine numeral is the short form and the masculine carries the ־ה.
//! * Small numerals have a bound (construct) form used before a definite noun — but only
//!   in the masculine (שלושת הילדים); the feminine reuses its absolute form (שלוש הילדות).

use std::collections::{HashMap, HashSet};

use once_cell::sync::Lazy;

use crate::config::Gender;

pub const ZERO: &str = "אפס";
pub const MINUS: &str = "מינוס";

/// 0-9, feminine — also the bare-counting and digit-by-digit series.
pub const ONES_FEM: [&str; 10] = [
    "אפס",
    "אחת",
    "שתיים",
    "שלוש",
    "ארבע",
    "חמש",
    "שש",
    "שבע",
    "שמונה",
    "תשע",
];

/// 0-9, masculine.
pub const ONES_MASC: [&str; 10] = [
    "אפס",
    "אחד",
    "שניים",
    "שלושה",
    "ארבעה",
    "חמישה",
    "שישה",
    "שבעה",
    "שמונה",
    "תשעה",
];

/// 10-19, feminine. Indexed by `n - 10`.
pub const TEENS_FEM: [&str; 10] = [
    "עשר",
    "אחת עשרה",
    "שתים עשרה",
    "שלוש עשרה",
    "ארבע עשרה",
    "חמש עשרה",
    "שש עשרה",
    "שבע עשרה",
    "שמונה עשרה",
    "תשע עשרה",
];

/// 10-19, masculine. Indexed by `n - 10`.
pub const TEENS_MASC: [&str; 10] = [
    "עשרה",
    "אחד עשר",
    "שנים עשר",
    "שלושה עשר",
    "ארבעה עשר",
    "חמישה עשר",
    "שישה עשר",
    "שבעה עשר",
    "שמונה עשר",
    "תשעה עשר",
];

/// The 0-9 series for a gender.
pub fn ones(gender: Gender) -> &'static [&'static str; 10] {
    match gender {
        Gender::Fem => &ONES_FEM,
        Gender::Masc => &ONES_MASC,
    }
}

/// The 10-19 series for a gender.
pub fn teens(gender: Gender) -> &'static [&'static str; 10] {
    match gender {
        Gender::Fem => &TEENS_FEM,
        Gender::Masc => &TEENS_MASC,
    }
}

/// 20-90, indexed by the tens digit minus 2. No gender.
pub const TENS: [&str; 8] = [
    "עשרים",
    "שלושים",
    "ארבעים",
    "חמישים",
    "שישים",
    "שבעים",
    "שמונים",
    "תשעים",
];

/// 100-900, indexed by the hundreds digit minus 1. 300+ is feminine-ones + מאות.
pub const HUNDREDS: [&str; 9] = [
    "מאה",
    "מאתיים",
    "שלוש מאות",
    "ארבע מאות",
    "חמש מאות",
    "שש מאות",
    "שבע מאות",
    "שמונה מאות",
    "תשע מאות",
];

/// Masculine construct (bound) forms, 2-10. Used before a definite noun and before
/// אלפים: שלושת הילדים, חמשת אלפים.
pub static CONSTRUCT_MASC: Lazy<HashMap<i64, &'static str>> = Lazy::new(|| {
    HashMap::from([
        (2, "שני"),
        (3, "שלושת"),
        (4, "ארבעת"),
        (5, "חמשת"),
        (6, "ששת"),
        (7, "שבעת"),
        (8, "שמונת"),
        (9, "תשעת"),
        (10, "עשרת"),
    ])
});

/// The feminine bound form exists only for 2; 3-10 reuse the absolute form.
pub static CONSTRUCT_FEM: Lazy<HashMap<i64, &'static str>> =
    Lazy::new(|| HashMap::from([(2, "שתי")]));

/// The construct table for a gender.
pub fn construct(gender: Gender) -> &'static HashMap<i64, &'static str> {
    match gender {
        Gender::Fem => &CONSTRUCT_FEM,
        Gender::Masc => &CONSTRUCT_MASC,
    }
}

pub const THOUSAND: &str = "אלף";
pub const TWO_THOUSAND: &str = "אלפיים";
pub const THOUSANDS_PLURAL: &str = "אלפים";
pub const MILLION: &str = "מיליון";
pub const MILLIARD: &str = "מיליארד";

/// Ordinals 1-10, masculine. Above 10 Hebrew simply reuses the cardinal.
pub static ORDINALS_MASC: Lazy<HashMap<i64, &'static str>> = Lazy::new(|| {
    HashMap::from([
        (1, "ראשון"),
        (2, "שני"),
        (3, "שלישי"),
        (4, "רביעי"),
        (5, "חמישי"),
        (6, "שישי"),
        (7, "שביעי"),
        (8, "שמיני"),
        (9, "תשיעי"),
        (10, "עשירי"),
    ])
});

/// Ordinals 1-10, feminine.
pub static ORDINALS_FEM: Lazy<HashMap<i64, &'static str>> = Lazy::new(|| {
    HashMap::from([
        (1, "ראשונה"),
        (2, "שנייה"),
        (3, "שלישית"),
        (4, "רביעית"),
        (5, "חמישית"),
        (6, "שישית"),
        (7, "שביעית"),
        (8, "שמינית"),
        (9, "תשיעית"),
        (10, "עשירית"),
    ])
});

/// The ordinal table for a gender.
pub fn ordinals(gender: Gender) -> &'static HashMap<i64, &'static str> {
    match gender {
        Gender::Fem => &ORDINALS_FEM,
        Gender::Masc => &ORDINALS_MASC,
    }
}

/// Single-letter clitics that may precede a noun. Stripped only when the bare
/// remainder is itself a word we know, so מים (water) is never read as מ + ים.
pub const CLITICS: [char; 7] = ['ה', 'ו', 'ב', 'ל', 'כ', 'מ', 'ש'];

/// Nouns whose gender we assert rather than guess. Weighted toward what actually
/// shows up next to a number: people, time, money, places, body, household, media.
pub static LEXICON: Lazy<HashMap<&'static str, Gender>> = Lazy::new(|| {
    use Gender::Fem as F;
    use Gender::Masc as M;
    HashMap::from([
        // people
        ("איש", M),
        ("אנשים", M),
        ("אישה", F),
        ("נשים", F),
        ("אדם", M),
        ("גבר", M),
        ("ילד", M),
        ("ילדים", M),
        ("ילדה", F),
        ("ילדות", F),
        ("תינוק", M),
        ("נער", M),
        ("נערה", F),
        ("בחור", M),
        ("בחורה", F),
        ("בן", M),
        ("בנים", M),
        ("בת", F),
        ("בנות", F),
        ("אח", M),
        ("אחים", M),
        ("אחות", F),
        ("אחיות", F),
        ("אב", M),
        ("אבא", M),
        ("אם", F),
        ("אמא", F),
        ("הורים", M),
        ("סבא", M),
        ("סבתא", F),
        ("משפחה", F),
        ("זוג", M),
        ("חבר", M),
        ("חברים", M),
        ("חברה", F),
        ("שכן", M),
        ("אורח", M),
        ("תלמיד", M),
        ("תלמידה", F),
        ("מורה", M),
        ("סטודנט", M),
        ("עובד", M),
        ("עובדת", F),
        ("מנהל", M),
        ("נהג", M),
        ("רופא", M),
        ("רופאה", F),
        ("אחיין", M),
        ("שוטר", M),
        ("חייל", M),
        ("חיילת", F),
        ("לקוח", M),
        ("משתמש", M),
        ("קבוצה", F),
        ("צוות", M),
        ("כיתה", F),
        ("עם", M),
        // time
        ("שנה", F),
        ("שנים", F),
        ("חודש", M),
        ("חודשים", M),
        ("שבוע", M),
        ("שבועות", M),
        ("יום", M),
        ("ימים", M),
        ("שעה", F),
        ("שעות", F),
        ("דקה", F),
        ("דקות", F),
        ("שנייה", F),
        ("שניות", F),
        ("רגע", M),
        ("לילה", M),
        ("לילות", M),
        ("בוקר", M),
        ("ערב", M),
        ("עונה", F),
        ("עשור", M),
        ("דור", M),
        ("פעם", F),
        ("פעמים", F),
        ("זמן", M),
        ("תאריך", M),
        // money
        ("שקל", M),
        ("שקלים", M),
        ("אגורה", F),
        ("אגורות", F),
        ("דולר", M),
        ("אירו", M),
        ("יורו", M),
        ("לירה", F),
        ("סנט", M),
        ("מטבע", M),
        ("מטבעות", M),
        ("כסף", M),
        ("מחיר", M),
        ("תשלום", M),
        ("חשבון", M),
        ("הנחה", F),
        ("אחוז", M),
        ("אחוזים", M),
        ("מס", M),
        ("משכורת", F),
        ("עלות", F),
        ("הכנסה", F),
        ("הלוואה", F),
        ("חוב", M),
        // places
        ("מקום", M),
        ("מקומות", M),
        ("עיר", F),
        ("ערים", F),
        ("כפר", M),
        ("מדינה", F),
        ("ארץ", F),
        ("ארצות", F),
        ("רחוב", M),
        ("רחובות", M),
        ("שכונה", F),
        ("בית", M),
        ("בתים", M),
        ("דירה", F),
        ("חדר", M),
        ("חדרים", M),
        ("קומה", F),
        ("בניין", M),
        ("משרד", M),
        ("חנות", F),
        ("חנויות", F),
        ("גן", M),
        ("פארק", M),
        ("שדה", M),
        ("הר", M),
        ("ים", M),
        ("נהר", M),
        ("נהרות", M),
        ("כביש", M),
        ("דרך", F),
        ("תחנה", F),
        // body
        ("יד", F),
        ("ידיים", F),
        ("רגל", F),
        ("רגליים", F),
        ("עין", F),
        ("עיניים", F),
        ("אוזן", F),
        ("אוזניים", F),
        ("אצבע", F),
        ("אצבעות", F),
        ("ראש", M),
        ("שיער", M),
        ("שן", F),
        ("שיניים", F),
        ("לב", M),
        ("פה", M),
        ("אף", M),
        ("גב", M),
        ("בטן", F),
        ("כתף", F),
        ("ברך", F),
        ("עצם", F),
        ("דם", M),
        // household
        ("שולחן", M),
        ("שולחנות", M),
        ("כיסא", M),
        ("כיסאות", M),
        ("מיטה", F),
        ("ארון", M),
        ("ארונות", M),
        ("דלת", F),
        ("דלתות", F),
        ("חלון", M),
        ("חלונות", M),
        ("קיר", M),
        ("קירות", M),
        ("רצפה", F),
        ("מנורה", F),
        ("מקרר", M),
        ("תנור", M),
        ("סיר", M),
        ("כוס", F),
        ("כוסות", F),
        ("צלחת", F),
        ("כף", F),
        ("סכין", F),
        ("מזלג", M),
        ("בקבוק", M),
        ("שקית", F),
        ("קופסה", F),
        ("מפתח", M),
        ("מגבת", F),
        ("שטיח", M),
        ("ספה", F),
        ("מטבח", M),
        // media, objects, abstractions
        ("ספר", M),
        ("ספרים", M),
        ("עמוד", M),
        ("דף", M),
        ("מילה", F),
        ("מילים", F),
        ("משפט", M),
        ("אות", F),
        ("אותיות", F),
        ("שורה", F),
        ("פרק", M),
        ("סרט", M),
        ("סרטון", M),
        ("שיר", M),
        ("תמונה", F),
        ("הודעה", F),
        ("הודעות", F),
        ("מייל", M),
        ("אתר", M),
        ("מחשב", M),
        ("טלפון", M),
        ("מכשיר", M),
        ("מסך", M),
        ("מקלדת", F),
        ("עכבר", M),
        ("קובץ", M),
        ("קבצים", M),
        ("תיקייה", F),
        ("תוכנית", F),
        ("אפליקציה", F),
        ("שאלה", F),
        ("תשובה", F),
        ("בעיה", F),
        ("פתרון", M),
        ("רעיון", M),
        ("דוגמה", F),
        ("נושא", M),
        ("עניין", M),
        ("שלב", M),
        ("חלק", M),
        ("סוג", M),
        ("דבר", M),
        ("דברים", M),
        ("קול", M),
        ("קולות", M),
        ("שם", M),
        ("שמות", M),
        ("חלום", M),
        ("חלומות", M),
        ("לוח", M),
        ("לוחות", M),
        // food and transport
        ("ביצה", F),
        ("ביצים", F),
        ("לחם", M),
        ("חלב", M),
        ("מים", M),
        ("עוגה", F),
        ("תפוח", M),
        ("עגבנייה", F),
        ("מכונית", F),
        ("רכב", M),
        ("אוטובוס", M),
        ("רכבת", F),
        ("מטוס", M),
        ("אופניים", M),
        ("נסיעה", F),
        ("טיסה", F),
        ("כרטיס", M),
    ])
});

/// Feminine in spite of the masculine-looking ־ים plural.
pub static FEM_DESPITE_IM: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from([
        "כנפיים",
        "לחיים",
        "ציפורניים",
        "מותניים",
        "נשים",
        "שנים",
        "מילים",
        "ביצים",
        "פעמים",
        "אבנים",
        "ערים",
        "נמלים",
        "דבורים",
        "חיטים",
        "ידיים",
        "רגליים",
        "עיניים",
        "אוזניים",
        "שיניים",
        "ברכיים",
        "כתפיים",
    ])
});

/// Masculine in spite of the feminine-looking ־ות plural.
pub static MASC_DESPITE_OT: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from([
        "מקומות",
        "שולחנות",
        "קירות",
        "חלונות",
        "לילות",
        "רחובות",
        "אבות",
        "שמות",
        "קולות",
        "כיסאות",
        "לוחות",
        "ארונות",
        "חלומות",
        "זנבות",
        "מטבחות",
        "רהיטים",
        "שבועות",
        "נהרות",
        "פירות",
        "כוחות",
        "מטבעות",
        "גגות",
    ])
});

/// Singular nouns the ־ה/־ת/־ית heuristic would misread.
pub static MASC_DESPITE_FEM_SUFFIX: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from([
        "לילה",
        "אבא",
        "סבא",
        "מורה",
        "שדה",
        "מקרה",
        "מנהיג",
        "כוכבית",
    ])
});

/// Singular nouns with no feminine marker that are feminine anyway.
pub static FEM_DESPITE_NO_SUFFIX: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from([
        "עיר",
        "ארץ",
        "אבן",
        "דרך",
        "יד",
        "רגל",
        "עין",
        "אוזן",
        "נפש",
        "שמש",
        "רוח",
        "אש",
        "כוס",
        "שן",
        "בטן",
        "ברך",
        "כתף",
        "אצבע",
        "סכין",
        "כף",
        "בת",
        "אם",
        "עצם",
        "ציפורן",
    ])
});
