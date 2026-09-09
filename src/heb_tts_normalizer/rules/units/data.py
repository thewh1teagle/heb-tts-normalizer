"""Unit lexicon: written form -> spoken singular/plural plus grammatical gender.

The gender is the whole reason this table exists. Hebrew numerals agree with the
noun they count, and the unit *is* the noun: ``5 ק״מ`` is חמישה קילומטרים (masculine)
while ``5 דקות`` is חמש דקות (feminine). A unit table without genders would get half
of its output wrong, so every entry carries one.

Each unit is registered under every spelling it plausibly appears in — the Hebrew
abbreviation with its gershayim/geresh (``ק״מ``, ``מ׳``), the word spelled out in both
numbers (``קילומטר``/``קילומטרים``) and the Latin symbol (``km``). They all map to the
same :class:`Unit`, so the rule only ever reasons about one object.

Note on the abbreviations: ``text.clean`` has already turned ASCII ``"`` and ``'``
between Hebrew letters into ``״``/``׳``, so the keys here use the real Hebrew
punctuation only.

Other modules (the ranges rule) import ``UNITS`` and read ``.singular``, ``.plural``
and ``.gender`` off the entries; treat those three attribute names as public API.
"""

from __future__ import annotations

from dataclasses import dataclass

from ...config import Gender

M = Gender.MASC
F = Gender.FEM


@dataclass(frozen=True, slots=True)
class Unit:
    """One unit of measure, in the two forms a Hebrew numeral can demand."""

    #: Form used after 1 and after a decimal ("קילוגרם").
    singular: str
    #: Form used from 2 upwards ("קילוגרמים").
    plural: str
    #: Gender the counting numeral must agree with.
    gender: Gender


#: (spellings, singular, plural, gender). The first spelling is the canonical
#: abbreviation; the rest are alternate ways the same unit gets written.
_TABLE: list[tuple[tuple[str, ...], str, str, Gender]] = [
    # -- length ---------------------------------------------------------------
    (("מ״מ", "מילימטר", "מילימטרים", "mm"), "מילימטר", "מילימטרים", M),
    (("ס״מ", "סנטימטר", "סנטימטרים", "cm"), "סנטימטר", "סנטימטרים", M),
    (("מ׳", "מטר", "מטרים", "m"), "מטר", "מטרים", M),
    (("ק״מ", "קילומטר", "קילומטרים", "km"), "קילומטר", "קילומטרים", M),
    # -- weight ---------------------------------------------------------------
    (("מ״ג", "מיליגרם", "מיליגרמים", "mg"), "מיליגרם", "מיליגרמים", M),
    (("ג׳", "גרם", "גרמים", "gr", "g"), "גרם", "גרמים", M),
    (("ק״ג", "קילוגרם", "קילוגרמים", "kg"), "קילוגרם", "קילוגרמים", M),
    # קילו is invariant in speech: "שלושה קילו", never "קילואים".
    (("קילו",), "קילו", "קילו", M),
    (("טון", "טונות", "ton"), "טון", "טונות", M),
    # -- volume ---------------------------------------------------------------
    (("מ״ל", "מיליליטר", "מיליליטרים", "ml"), "מיליליטר", "מיליליטרים", M),
    (("ליטר", "ליטרים", "lt"), "ליטר", "ליטרים", M),
    # קוב (cubic metre) is invariant too: "חמישה קוב".
    (("קוב",), "קוב", "קוב", M),
    (("מ״ק", "מטר מעוקב", "מטרים מעוקבים"), "מטר מעוקב", "מטרים מעוקבים", M),
    # -- time -----------------------------------------------------------------
    (("שנ׳", "שנייה", "שניה", "שניות"), "שנייה", "שניות", F),
    (("דק׳", "דקה", "דקות"), "דקה", "דקות", F),
    (("שע׳", "שעה", "שעות"), "שעה", "שעות", F),
    (("יום", "ימים"), "יום", "ימים", M),
    (("שבוע", "שבועות"), "שבוע", "שבועות", M),
    (("חודש", "חודשים"), "חודש", "חודשים", M),
    # plural of שנה is שנים (not שניים) — the numeral still takes the feminine form.
    (("שנה", "שנים"), "שנה", "שנים", F),
    # -- data -----------------------------------------------------------------
    (("ק״ב", "קילובייט", "KB", "kb"), "קילובייט", "קילובייטים", M),
    (("מ״ב", "מגהבייט", "מגה בייט", "MB", "mb"), "מגהבייט", "מגהבייטים", M),
    (("ג״ב", "גיגהבייט", "גיגה בייט", "GB", "gb"), "גיגהבייט", "גיגהבייטים", M),
    (("ט״ב", "טרהבייט", "טרה בייט", "TB", "tb"), "טרהבייט", "טרהבייטים", M),
    # -- speed ----------------------------------------------------------------
    (("קמ״ש", "קילומטר לשעה", "km/h", "kmh"), "קילומטר לשעה", "קילומטרים לשעה", M),
    # -- temperature ----------------------------------------------------------
    (("מעלות", "מעלה"), "מעלה", "מעלות", F),
    # -- area -----------------------------------------------------------------
    (("מ״ר", "מטר רבוע", "מטרים רבועים"), "מטר רבוע", "מטרים רבועים", M),
    (("ס״ר", "סנטימטר רבוע"), "סנטימטר רבוע", "סנטימטרים רבועים", M),
    (("ק״ר", "קילומטר רבוע"), "קילומטר רבוע", "קילומטרים רבועים", M),
    (("דונם", "דונמים"), "דונם", "דונמים", M),
]

#: Every written form of every unit, keyed by the exact string as it appears in text.
UNITS: dict[str, Unit] = {
    form: Unit(singular, plural, gender)
    for forms, singular, plural, gender in _TABLE
    for form in forms
}

#: Written forms, longest first — so an alternation prefers ``ק״מ`` over ``מ׳``
#: and ``מגה בייט`` over ``מגה``.
UNIT_FORMS: list[str] = sorted(UNITS, key=len, reverse=True)

#: Temperature scales that may follow a degree sign.
SCALES: dict[str, str] = {
    "C": "צלזיוס",
    "c": "צלזיוס",
    "צ": "צלזיוס",
    "F": "פרנהייט",
    "f": "פרנהייט",
    "פ": "פרנהייט",
}

__all__ = ["SCALES", "UNITS", "UNIT_FORMS", "Unit"]
