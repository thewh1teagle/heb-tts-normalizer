"""Hebrew number and noun-gender word tables.

Everything here is data only — no logic — so the shapes stay easy to audit against a
grammar book. Two Hebrew quirks drive the layout:

* Cardinals have two gender series, and the *feminine* series is the unmarked one used
  for bare counting ("שלוש נקודה חמש"), which is the opposite of what the morphology
  suggests: the feminine numeral is the short form and the masculine carries the ־ה.
* Small numerals have a bound (construct) form used before a definite noun — but only
  in the masculine (שלושת הילדים); the feminine reuses its absolute form (שלוש הילדות).
"""

from __future__ import annotations

from ..config import Gender

ZERO = "אפס"
MINUS = "מינוס"

#: 0-9, feminine — also the bare-counting and digit-by-digit series.
ONES_FEM: tuple[str, ...] = (
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
)

#: 0-9, masculine.
ONES_MASC: tuple[str, ...] = (
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
)

ONES: dict[Gender, tuple[str, ...]] = {Gender.FEM: ONES_FEM, Gender.MASC: ONES_MASC}

#: 10-19 by gender. Indexed by ``n - 10``.
TEENS_FEM: tuple[str, ...] = (
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
)

TEENS_MASC: tuple[str, ...] = (
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
)

TEENS: dict[Gender, tuple[str, ...]] = {Gender.FEM: TEENS_FEM, Gender.MASC: TEENS_MASC}

#: 20-90, indexed by the tens digit. No gender.
TENS: dict[int, str] = {
    2: "עשרים",
    3: "שלושים",
    4: "ארבעים",
    5: "חמישים",
    6: "שישים",
    7: "שבעים",
    8: "שמונים",
    9: "תשעים",
}

#: 100-900, indexed by the hundreds digit. 300+ is feminine-ones + מאות.
HUNDREDS: dict[int, str] = {
    1: "מאה",
    2: "מאתיים",
    3: "שלוש מאות",
    4: "ארבע מאות",
    5: "חמש מאות",
    6: "שש מאות",
    7: "שבע מאות",
    8: "שמונה מאות",
    9: "תשע מאות",
}

#: Masculine construct (bound) forms, 2-10. Used before a definite noun and before
#: אלפים: שלושת הילדים, חמשת אלפים.
CONSTRUCT_MASC: dict[int, str] = {
    2: "שני",
    3: "שלושת",
    4: "ארבעת",
    5: "חמשת",
    6: "ששת",
    7: "שבעת",
    8: "שמונת",
    9: "תשעת",
    10: "עשרת",
}

#: The feminine bound form exists only for 2; 3-10 reuse the absolute form.
CONSTRUCT_FEM: dict[int, str] = {2: "שתי"}

CONSTRUCT: dict[Gender, dict[int, str]] = {
    Gender.FEM: CONSTRUCT_FEM,
    Gender.MASC: CONSTRUCT_MASC,
}

THOUSAND = "אלף"
TWO_THOUSAND = "אלפיים"
THOUSANDS_PLURAL = "אלפים"
MILLION = "מיליון"
MILLIARD = "מיליארד"

#: Ordinals 1-10. Above 10 Hebrew simply reuses the cardinal.
ORDINALS_MASC: dict[int, str] = {
    1: "ראשון",
    2: "שני",
    3: "שלישי",
    4: "רביעי",
    5: "חמישי",
    6: "שישי",
    7: "שביעי",
    8: "שמיני",
    9: "תשיעי",
    10: "עשירי",
}

ORDINALS_FEM: dict[int, str] = {
    1: "ראשונה",
    2: "שנייה",
    3: "שלישית",
    4: "רביעית",
    5: "חמישית",
    6: "שישית",
    7: "שביעית",
    8: "שמינית",
    9: "תשיעית",
    10: "עשירית",
}

ORDINALS: dict[Gender, dict[int, str]] = {
    Gender.FEM: ORDINALS_FEM,
    Gender.MASC: ORDINALS_MASC,
}

#: Single-letter clitics that may precede a noun. Stripped only when the bare
#: remainder is itself a word we know, so מים (water) is never read as מ + ים.
CLITICS = "הובלכמש"

_M = Gender.MASC
_F = Gender.FEM

#: Nouns whose gender we assert rather than guess. Weighted toward what actually
#: shows up next to a number: people, time, money, places, body, household, media.
LEXICON: dict[str, Gender] = {
    # people
    "איש": _M,
    "אנשים": _M,
    "אישה": _F,
    "נשים": _F,
    "אדם": _M,
    "גבר": _M,
    "ילד": _M,
    "ילדים": _M,
    "ילדה": _F,
    "ילדות": _F,
    "תינוק": _M,
    "נער": _M,
    "נערה": _F,
    "בחור": _M,
    "בחורה": _F,
    "בן": _M,
    "בנים": _M,
    "בת": _F,
    "בנות": _F,
    "אח": _M,
    "אחים": _M,
    "אחות": _F,
    "אחיות": _F,
    "אב": _M,
    "אבא": _M,
    "אם": _F,
    "אמא": _F,
    "הורים": _M,
    "סבא": _M,
    "סבתא": _F,
    "משפחה": _F,
    "זוג": _M,
    "חבר": _M,
    "חברים": _M,
    "חברה": _F,
    "שכן": _M,
    "אורח": _M,
    "תלמיד": _M,
    "תלמידה": _F,
    "מורה": _M,
    "סטודנט": _M,
    "עובד": _M,
    "עובדת": _F,
    "מנהל": _M,
    "נהג": _M,
    "רופא": _M,
    "רופאה": _F,
    "אחיין": _M,
    "שוטר": _M,
    "חייל": _M,
    "חיילת": _F,
    "לקוח": _M,
    "משתמש": _M,
    "קבוצה": _F,
    "צוות": _M,
    "כיתה": _F,
    "עם": _M,
    # time
    "שנה": _F,
    "שנים": _F,
    "חודש": _M,
    "חודשים": _M,
    "שבוע": _M,
    "שבועות": _M,
    "יום": _M,
    "ימים": _M,
    "שעה": _F,
    "שעות": _F,
    "דקה": _F,
    "דקות": _F,
    "שנייה": _F,
    "שניות": _F,
    "רגע": _M,
    "לילה": _M,
    "לילות": _M,
    "בוקר": _M,
    "ערב": _M,
    "עונה": _F,
    "עשור": _M,
    "דור": _M,
    "פעם": _F,
    "פעמים": _F,
    "זמן": _M,
    "תאריך": _M,
    # money
    "שקל": _M,
    "שקלים": _M,
    "אגורה": _F,
    "אגורות": _F,
    "דולר": _M,
    "אירו": _M,
    "יורו": _M,
    "לירה": _F,
    "סנט": _M,
    "מטבע": _M,
    "מטבעות": _M,
    "כסף": _M,
    "מחיר": _M,
    "תשלום": _M,
    "חשבון": _M,
    "הנחה": _F,
    "אחוז": _M,
    "אחוזים": _M,
    "מס": _M,
    "משכורת": _F,
    "עלות": _F,
    "הכנסה": _F,
    "הלוואה": _F,
    "חוב": _M,
    # places
    "מקום": _M,
    "מקומות": _M,
    "עיר": _F,
    "ערים": _F,
    "כפר": _M,
    "מדינה": _F,
    "ארץ": _F,
    "ארצות": _F,
    "רחוב": _M,
    "רחובות": _M,
    "שכונה": _F,
    "בית": _M,
    "בתים": _M,
    "דירה": _F,
    "חדר": _M,
    "חדרים": _M,
    "קומה": _F,
    "בניין": _M,
    "משרד": _M,
    "חנות": _F,
    "חנויות": _F,
    "גן": _M,
    "פארק": _M,
    "שדה": _M,
    "הר": _M,
    "ים": _M,
    "נהר": _M,
    "נהרות": _M,
    "כביש": _M,
    "דרך": _F,
    "תחנה": _F,
    # body
    "יד": _F,
    "ידיים": _F,
    "רגל": _F,
    "רגליים": _F,
    "עין": _F,
    "עיניים": _F,
    "אוזן": _F,
    "אוזניים": _F,
    "אצבע": _F,
    "אצבעות": _F,
    "ראש": _M,
    "שיער": _M,
    "שן": _F,
    "שיניים": _F,
    "לב": _M,
    "פה": _M,
    "אף": _M,
    "גב": _M,
    "בטן": _F,
    "כתף": _F,
    "ברך": _F,
    "עצם": _F,
    "דם": _M,
    # household
    "שולחן": _M,
    "שולחנות": _M,
    "כיסא": _M,
    "כיסאות": _M,
    "מיטה": _F,
    "ארון": _M,
    "ארונות": _M,
    "דלת": _F,
    "דלתות": _F,
    "חלון": _M,
    "חלונות": _M,
    "קיר": _M,
    "קירות": _M,
    "רצפה": _F,
    "מנורה": _F,
    "מקרר": _M,
    "תנור": _M,
    "סיר": _M,
    "כוס": _F,
    "כוסות": _F,
    "צלחת": _F,
    "כף": _F,
    "סכין": _F,
    "מזלג": _M,
    "בקבוק": _M,
    "שקית": _F,
    "קופסה": _F,
    "מפתח": _M,
    "מגבת": _F,
    "שטיח": _M,
    "ספה": _F,
    "מטבח": _M,
    # media, objects, abstractions
    "ספר": _M,
    "ספרים": _M,
    "עמוד": _M,
    "דף": _M,
    "מילה": _F,
    "מילים": _F,
    "משפט": _M,
    "אות": _F,
    "אותיות": _F,
    "שורה": _F,
    "פרק": _M,
    "סרט": _M,
    "סרטון": _M,
    "שיר": _M,
    "תמונה": _F,
    "הודעה": _F,
    "הודעות": _F,
    "מייל": _M,
    "אתר": _M,
    "מחשב": _M,
    "טלפון": _M,
    "מכשיר": _M,
    "מסך": _M,
    "מקלדת": _F,
    "עכבר": _M,
    "קובץ": _M,
    "קבצים": _M,
    "תיקייה": _F,
    "תוכנית": _F,
    "אפליקציה": _F,
    "שאלה": _F,
    "תשובה": _F,
    "בעיה": _F,
    "פתרון": _M,
    "רעיון": _M,
    "דוגמה": _F,
    "נושא": _M,
    "עניין": _M,
    "שלב": _M,
    "חלק": _M,
    "סוג": _M,
    "דבר": _M,
    "דברים": _M,
    "קול": _M,
    "קולות": _M,
    "שם": _M,
    "שמות": _M,
    "חלום": _M,
    "חלומות": _M,
    "לוח": _M,
    "לוחות": _M,
    # food and transport
    "ביצה": _F,
    "ביצים": _F,
    "לחם": _M,
    "חלב": _M,
    "מים": _M,
    "עוגה": _F,
    "תפוח": _M,
    "עגבנייה": _F,
    "מכונית": _F,
    "רכב": _M,
    "אוטובוס": _M,
    "רכבת": _F,
    "מטוס": _M,
    "אופניים": _M,
    "נסיעה": _F,
    "טיסה": _F,
    "כרטיס": _M,
}

#: Feminine in spite of the masculine-looking ־ים plural.
FEM_DESPITE_IM: frozenset[str] = frozenset(
    {
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
    }
)

#: Masculine in spite of the feminine-looking ־ות plural.
MASC_DESPITE_OT: frozenset[str] = frozenset(
    {
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
    }
)

#: Singular nouns the ־ה/־ת/־ית heuristic would misread.
MASC_DESPITE_FEM_SUFFIX: frozenset[str] = frozenset(
    {
        "לילה",
        "אבא",
        "סבא",
        "מורה",
        "שדה",
        "מקרה",
        "מנהיג",
        "כוכבית",
    }
)

#: Singular nouns with no feminine marker that are feminine anyway.
FEM_DESPITE_NO_SUFFIX: frozenset[str] = frozenset(
    {
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
    }
)
