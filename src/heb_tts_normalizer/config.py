"""Reading-style options. Everything the caller might reasonably disagree about lives here."""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import StrEnum


class Clock(StrEnum):
    """How to read a time of day."""

    H12 = "12"  # 14:30 -> שתיים וחצי אחר הצהריים
    H24 = "24"  # 14:30 -> ארבע עשרה שלושים


class DateOrder(StrEnum):
    """How to disambiguate 03/04/2026."""

    DMY = "dmy"  # Israeli convention
    MDY = "mdy"


class Gender(StrEnum):
    """Grammatical gender of a counted noun."""

    MASC = "m"
    FEM = "f"


@dataclass(frozen=True, slots=True)
class Config:
    """Reading style. The defaults are the ones an Israeli speaker would use."""

    clock: Clock = Clock.H12
    date_order: DateOrder = DateOrder.DMY

    #: Currency assumed for a bare number that is clearly money but carries no symbol.
    default_currency: str = "ILS"
    #: Read agorot/cents after the major unit ("25.50 ₪" -> "... וחמישים אגורות").
    read_minor_currency: bool = True

    #: Word between the integer and fractional part of a decimal.
    decimal_word: str = "נקודה"

    #: Spell out unit abbreviations ("ק״מ" -> "קילומטרים"). Off leaves them untouched.
    expand_units: bool = True
    #: Expand common abbreviations ("ד״ר" -> "דוקטור").
    expand_abbreviations: bool = True

    #: Drop Markdown syntax, keeping the text it wraps.
    strip_markdown: bool = True
    #: Collapse runs of whitespace and remove zero-width / bidi control characters.
    clean_whitespace: bool = True

    #: Extra noun genders, consulted before the built-in lexicon and the suffix
    #: heuristics. Lets a caller fix a word we get wrong without a release.
    gender_overrides: dict[str, Gender] = field(default_factory=dict)
