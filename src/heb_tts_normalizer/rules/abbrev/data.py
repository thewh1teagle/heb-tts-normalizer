"""Hebrew abbreviations, split by how a reader actually pronounces them.

There are two kinds, and conflating them is the classic way to get this wrong.

**Spelled out** (:data:`EXPANSIONS`) — the gershayim stand in for words nobody says
letter by letter. ``ד״ר`` is pronounced "doktor", not "dalet resh"; ``וכו׳`` is
"vechuli"; ``ארה״ב`` is "artsot ha-brit". These must become their full wording, or
the g2p reads a nonsense letter salad.

**Read as a word** (:data:`READ_AS_WORD`) — the acronym has been lexicalised. Nobody
says "tsva hagana le-yisrael" when they see ``צה״ל``; they say /tsahal/. Expanding
these would be actively wrong — it puts words in the speaker's mouth and changes the
register. The right move is the smallest one: drop the gershayim so the g2p sees a
plain Hebrew word (``צה״ל`` -> ``צהל``) and pronounces it the way the letters read.
Values are given explicitly rather than computed, so a spelling that needs help can
get it without touching the rule.

Membership is a judgment call and two entries deserve a note:

* ``ביה״ס`` is *not* read as a word — "beit ha-sefer" is what a speaker says — so it
  expands, even though it looks like the lexicalised acronyms around it.
* ``שליט״א`` is the reverse: it looks like a formula to spell out, but it is read
  /shlita/ as a single word, so it belongs with ``צה״ל``.

Both dicts are keyed by the form as it appears *after* ``text.clean``, i.e. with real
gershayim ``״`` and geresh ``׳``, never the ASCII quotes.
"""

from __future__ import annotations

#: Abbreviation -> the words a reader says in its place.
EXPANSIONS: dict[str, str] = {
    # titles and people
    "ד״ר": "דוקטור",
    "פרופ׳": "פרופסור",
    "גב׳": "גברת",
    "עו״ד": "עורך דין",
    "רו״ח": "רואה חשבון",
    "יו״ר": "יושב ראש",
    "ח״כ": "חבר כנסת",
    "ז״ל": "זכרונו לברכה",
    "הי״ד": "השם ייקום דמו",
    # addresses and references
    "רח׳": "רחוב",
    "שד׳": "שדרות",
    "ת״ד": "תא דואר",
    "מס׳": "מספר",
    "עמ׳": "עמוד",
    "ר״ת": "ראשי תיבות",
    "בע״מ": "בערבון מוגבל",
    # places
    "ת״א": "תל אביב",
    "ארה״ב": "ארצות הברית",
    "בריה״מ": "ברית המועצות",
    "א״י": "ארץ ישראל",
    "ביה״ס": "בית הספר",
    "בי״ס": "בית ספר",
    "ביה״ח": "בית החולים",
    "ביהמ״ש": "בית המשפט",
    # time
    "אחה״צ": "אחר הצהריים",
    "לפנה״צ": "לפני הצהריים",
    "לפנה״ס": "לפני הספירה",
    "לספה״נ": "לספירת הנוצרים",
    "אח״כ": "אחר כך",
    # connectives and stock phrases
    "וכו׳": "וכולי",
    "וגו׳": "וגומר",
    "וכד׳": "וכדומה",
    "כ״א": "כל אחד",
    "כ״כ": "כל כך",
    "ע״י": "על ידי",
    "ע״פ": "על פי",
    "עפ״י": "על פי",
    "ע״ש": "על שם",
    "בד״כ": "בדרך כלל",
    "סה״כ": "סך הכל",
    "כנ״ל": "כנזכר לעיל",
    "א״כ": "אם כן",
    "צ״ל": "צריך להיות",
    "ד״ש": "דרישת שלום",
    "מו״ל": "מוציא לאור",
    "מו״מ": "משא ומתן",
    "בס״ד": "בסיעתא דשמיא",
    "ב״ה": "ברוך השם",
    # single letters used as enumerators: "סעיף א׳", "יום ג׳"
    "א׳": "אלף",
    "ב׳": "בית",
    "ג׳": "גימל",
}

#: Acronym -> the same acronym as a plain word, for the ones a reader pronounces
#: rather than unpacks. Removing the gershayim is the entire transformation.
READ_AS_WORD: dict[str, str] = {
    "צה״ל": "צהל",
    "אמ״ן": "אמן",
    "שב״כ": "שבכ",
    "מד״א": "מדא",
    "רמטכ״ל": "רמטכל",
    "קק״ל": "קקל",
    "תנ״ך": "תנך",
    "ש״ס": "שס",
    "בג״ץ": "בגץ",
    "מנכ״ל": "מנכל",
    "סמנכ״ל": "סמנכל",
    "נגמ״ש": "נגמש",
    "חו״ל": "חול",
    "דו״ח": "דוח",
    "דו״חות": "דוחות",
    "תפו״ז": "תפוז",
    "צל״ש": "צלש",
    "שליט״א": "שליטא",
}

#: Everything the rule can match, longest first so ``עפ״י`` beats ``ע״פ`` and
#: ``ביהמ״ש`` beats ``ביה״ס``-shaped prefixes.
ABBREVIATIONS: list[str] = sorted(EXPANSIONS | READ_AS_WORD, key=len, reverse=True)

#: Single-letter proclitics that glue onto an abbreviation: בת״א, לד״ר, וכו׳'s ו.
#: Matched separately and re-attached, so the lexicon stays free of every prefixed form.
PREFIXES = "ובלכמשה"

__all__ = ["ABBREVIATIONS", "EXPANSIONS", "PREFIXES", "READ_AS_WORD"]
