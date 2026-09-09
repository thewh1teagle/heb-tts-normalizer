"""Calendar vocabulary: Gregorian months, Hebrew-calendar months and weekday names.

Only the Gregorian table drives rendering today; the Hebrew months and the weekday
names are here because Hebrew text names them constantly ("יום שלישי, 12 במרץ",
"ראש חודש כסלו") and a rule that wants to recognise them should not re-invent the
spellings.

Spelling notes:
- March is written both מרץ and מרס; both are accepted on input, מרץ is emitted.
- חשוון / כסלו / סיוון are given in their full (plene) spelling, with the defective
  variants accepted on input, because that is how they are typed in practice.
- The preposition is a bound ב־ ("במרץ", "באפריל"), never a standalone word.
"""

from __future__ import annotations

#: Gregorian month number -> the spelling we emit.
MONTHS: dict[int, str] = {
    1: "ינואר",
    2: "פברואר",
    3: "מרץ",
    4: "אפריל",
    5: "מאי",
    6: "יוני",
    7: "יולי",
    8: "אוגוסט",
    9: "ספטמבר",
    10: "אוקטובר",
    11: "נובמבר",
    12: "דצמבר",
}

#: Every spelling we accept on input -> month number. Includes the emitted forms.
MONTH_NUMBERS: dict[str, int] = {
    **{name: number for number, name in MONTHS.items()},
    "מרס": 3,
}

#: Hebrew-calendar months, in calendar order starting at תשרי. אדר א׳/אדר ב׳ in a
#: leap year are spelled out rather than abbreviated, since a g2p cannot read ׳.
HEBREW_MONTHS: tuple[str, ...] = (
    "תשרי",
    "חשוון",
    "כסלו",
    "טבת",
    "שבט",
    "אדר",
    "ניסן",
    "אייר",
    "סיוון",
    "תמוז",
    "אב",
    "אלול",
)

#: Defective spellings that turn up in real text -> the plene form above.
HEBREW_MONTH_VARIANTS: dict[str, str] = {
    "חשון": "חשוון",
    "מרחשוון": "חשוון",
    "מרחשון": "חשוון",
    "כסליו": "כסלו",
    "סיון": "סיוון",
    "מנחם אב": "אב",
    "אדר א": "אדר ראשון",
    "אדר ב": "אדר שני",
}

#: Letter values for reading a Hebrew-calendar date. Final forms carry their ordinary
#: value; they are vanishingly rare in a date but cost nothing to accept.
GEMATRIA: dict[str, int] = {
    "א": 1,
    "ב": 2,
    "ג": 3,
    "ד": 4,
    "ה": 5,
    "ו": 6,
    "ז": 7,
    "ח": 8,
    "ט": 9,
    "י": 10,
    "כ": 20,
    "ך": 20,
    "ל": 30,
    "מ": 40,
    "ם": 40,
    "נ": 50,
    "ן": 50,
    "ס": 60,
    "ע": 70,
    "פ": 80,
    "ף": 80,
    "צ": 90,
    "ץ": 90,
    "ק": 100,
    "ר": 200,
    "ש": 300,
    "ת": 400,
}

#: Dates whose name is a word, not a reading. "ט״ו בשבט" is /tu bishvat/ to every
#: speaker; spelling it out as חמישה עשר בשבט is correct and wrong at the same time.
#: (day, month) -> what to say. Note ט׳ באב needs no entry: תשעה באב IS the cardinal.
HEBREW_DATE_NAMES: dict[tuple[int, str], str] = {
    # Read as a word.
    (15, "שבט"): "טו בשבט",
    (15, "אב"): "טו באב",
    (33, "עומר"): "לג בעומר",
    # Read as a number. These are the exceptions to the letter-name default: the fast
    # days are lexicalised as numerals, and "טית באב" is not a thing anyone says.
    (9, "אב"): "תשעה באב",
    (10, "טבת"): "עשרה בטבת",
    (17, "תמוז"): "שבעה עשר בתמוז",
}

#: Not a month, but it counts days the same way and appears in the same shape.
OMER = "עומר"

#: Letter names, for reading a Hebrew date the way it is actually said: כ״ז באלול is
#: "כף זין באלול", not "עשרים ושבעה באלול". Only א-ל can appear in a day (1-30).
LETTER_NAMES: dict[str, str] = {
    "א": "אלף",
    "ב": "בית",
    "ג": "גימל",
    "ד": "דלת",
    "ה": "הא",
    "ו": "וו",
    "ז": "זין",
    "ח": "חית",
    "ט": "טית",
    "י": "יוד",
    "כ": "כף",
    "ל": "למד",
}


def letter_names(numeral: str) -> str:
    """Spell a Hebrew numeral out as its letter names, dropping geresh/gershayim."""
    return " ".join(LETTER_NAMES[ch] for ch in numeral if ch in LETTER_NAMES)


def gematria(letters: str) -> int:
    """Numeric value of a Hebrew numeral, ignoring its geresh/gershayim."""
    return sum(GEMATRIA.get(ch, 0) for ch in letters if ch in GEMATRIA)


#: Weekday number (Sunday = 1, as Hebrew counts) -> full name.
WEEKDAYS: dict[int, str] = {
    1: "יום ראשון",
    2: "יום שני",
    3: "יום שלישי",
    4: "יום רביעי",
    5: "יום חמישי",
    6: "יום שישי",
    7: "יום שבת",
}

#: שבת stands alone as a day name; ערב שבת / מוצאי שבת are the usual compounds.
SABBATH: str = "שבת"
SABBATH_COMPOUNDS: tuple[str, ...] = ("ערב שבת", "מוצאי שבת")

__all__ = [
    "GEMATRIA",
    "HEBREW_DATE_NAMES",
    "HEBREW_MONTHS",
    "HEBREW_MONTH_VARIANTS",
    "LETTER_NAMES",
    "MONTHS",
    "MONTH_NUMBERS",
    "OMER",
    "SABBATH",
    "SABBATH_COMPOUNDS",
    "WEEKDAYS",
    "gematria",
    "letter_names",
]
