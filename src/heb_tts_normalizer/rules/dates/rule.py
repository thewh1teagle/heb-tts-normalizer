"""Dates: ``12/03/2026``, ``12.03.2026``, ``12-03-2026``, ``12/3/26``, ``2026-03-12``,
``12 במרץ 2026`` and the bare month/year ``3/2026``.

Judgment calls made here:

- **Gender.** The day of the month is masculine (יום is masculine) — ``12/03`` is
  "שנים עשר במרץ", not "שתים עשרה". The year is feminine (שנה is feminine) —
  2026 is "אלפיים עשרים ושש". Getting these backwards is the single most audible
  mistake a Hebrew date reader can make, so they are wired in, not inferred.
- **The first of the month is an ordinal**: "הראשון במרץ", never "אחד במרץ".
  Days 2-31 are plain cardinals, which is how Israelis actually say them.
- **Validation is a real calendar check**, via ``datetime.date``. That is what lets
  the rule decline ``32/13/2026`` and ``00/05/2026`` (and ``30/02/2026``) so the
  span falls through to the numbers rule and gets read as digits.
- **The configured order is a preference, not a straitjacket.** With
  ``date_order=MDY`` the text ``25/12/2026`` has no month 25, so rather than
  declining a perfectly readable date we fall back to the other order. Only a date
  that is impossible in *both* orders is declined.
- **Two-digit years** follow the usual window: 00-29 -> 2000s, 30-99 -> 1900s.
- ``2026-03-12`` (ISO) is order-independent and is read regardless of ``date_order``.

A date-shaped span that fails validation is claimed and returned **unchanged**, not
declined. ``32/13/2026`` is far more likely to be a serial number than arithmetic, and
letting the ranges and numbers rules loose on it produced "אלפיים עשרים ושש עד שלוש
עשרה". Not understanding something is a reason to leave it alone.
"""

from __future__ import annotations

from datetime import date

import regex as re

from ...config import Config, DateOrder, Gender
from ...numerals import numeral, ordinal
from ...scanner import PRIORITY
from .data import (
    HEBREW_DATE_NAMES,
    HEBREW_MONTH_VARIANTS,
    HEBREW_MONTHS,
    MONTH_NUMBERS,
    MONTHS,
    OMER,
    gematria,
)

#: Longest first, so "מרץ" cannot shadow a longer spelling that starts the same way.
_MONTH_ALT = "|".join(sorted(MONTH_NUMBERS, key=len, reverse=True))

#: A date must not start in the middle of a number, and must not be the head of a
#: longer numeric run. A preceding "-" is fine: "ב-12/03/2026" is ordinary Hebrew.
_BEFORE = r"(?<![\d./])"
_AFTER = r"(?!\d)(?![./-]\d)"


def _expand_year(raw: str) -> int:
    """Two-digit years: 00-29 are this century, 30-99 the last one."""
    if len(raw) == 4:
        return int(raw)
    two = int(raw)
    return 2000 + two if two <= 29 else 1900 + two


def _day_words(day: int) -> str:
    # The 1st is the only day said as an ordinal; the rest are cardinals.
    if day == 1:
        return ordinal(1, Gender.MASC, definite=True)
    return numeral(day, Gender.MASC)


def _spell(day: int | None, month: int, year: int | None) -> str:
    parts: list[str] = []
    if day is not None:
        parts.append(f"{_day_words(day)} ב{MONTHS[month]}")
    else:
        parts.append(MONTHS[month])
    if year is not None:
        parts.append(numeral(year, Gender.FEM))
    return " ".join(parts)


def _valid(day: int, month: int, year: int) -> bool:
    try:
        date(year, month, day)
    except ValueError:
        return False
    return True


def _resolve(first: int, second: int, year: int, cfg: Config) -> tuple[int, int] | None:
    """Map the two leading numbers onto (day, month) using the configured order.

    Falls back to the opposite order when the configured one is impossible, which is
    what makes ``25/12/2026`` readable even under MDY.
    """
    if cfg.date_order is DateOrder.MDY:
        orders = ((second, first), (first, second))
    else:
        orders = ((first, second), (second, first))
    for day, month in orders:
        if _valid(day, month, year):
            return day, month
    return None


class NumericDateRule:
    """``12/03/2026``, ``12.03.2026``, ``12-03-2026``, ``12/3/26``."""

    name = "date-numeric"
    priority = PRIORITY.DATE
    pattern = re.compile(rf"{_BEFORE}(\d{{1,2}})([./-])(\d{{1,2}})\2(\d{{4}}|\d{{2}}){_AFTER}")

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        year = _expand_year(m[4])
        resolved = _resolve(int(m[1]), int(m[3]), year, cfg)
        if resolved is None:
            return m[0]  # date-shaped but impossible: keep it, don't let it be read
        day, month = resolved
        return _spell(day, month, year)


class IsoDateRule:
    """``2026-03-12``. Unambiguous, so ``date_order`` does not apply."""

    name = "date-iso"
    priority = PRIORITY.DATE
    pattern = re.compile(rf"{_BEFORE}(\d{{4}})-(\d{{2}})-(\d{{2}}){_AFTER}")

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        year, month, day = int(m[1]), int(m[2]), int(m[3])
        if not _valid(day, month, year):
            return m[0]
        return _spell(day, month, year)


class TextualDateRule:
    """``12 במרץ 2026`` and the year-less ``12 במרץ``."""

    name = "date-textual"
    priority = PRIORITY.DATE
    pattern = re.compile(
        rf"{_BEFORE}(\d{{1,2}})\s+ב[־-]?({_MONTH_ALT})(?:\s+(\d{{4}})(?!\d))?",
    )

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        day = int(m[1])
        month = MONTH_NUMBERS[m[2]]
        year = int(m[3]) if m[3] else None
        # No year means no leap-year question; a February 29th still has to be allowed.
        if not _valid(day, month, year if year is not None else 2024):
            return m[0]
        return _spell(day, month, year)


class MonthYearRule:
    """A bare ``3/2026`` is month/year — never a day, in either order."""

    name = "date-month-year"
    priority = PRIORITY.DATE
    pattern = re.compile(rf"{_BEFORE}(\d{{1,2}})/(\d{{4}}){_AFTER}")

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        month, year = int(m[1]), int(m[2])
        if not 1 <= month <= 12:
            return m[0]
        return _spell(None, month, year)


#: A Hebrew numeral: two letters around a gershayim (י״ד, ט״ו) or one with a geresh (א׳).
_GEMATRIA = r"(?:[א-ת]״[א-ת]|[א-ת]׳)"

#: Months plus their defective spellings, and the Omer, which counts days the same way.
_HEB_MONTH_ALT = "|".join(
    sorted({*HEBREW_MONTHS, *HEBREW_MONTH_VARIANTS, OMER}, key=len, reverse=True)
)

#: A Hebrew year: תשפ״ו, optionally with the millennium ה׳. Read as the word it is.
_HEB_YEAR = r"(?:ה[׳'])?[א-ת]{1,3}״[א-ת]"


class HebrewDateRule:
    """``כ״ז באלול``, ``א׳ בתשרי``, ``ט״ו בשבט``, ``י״ד בתשרי תשפ״ו``.

    The month is required. A bare ``כ״ז`` is indistinguishable from a gershayim acronym,
    and the abbreviations rule has the better claim on it — which is also why this rule
    has to sit at DATE priority: without it ``א׳ בתשרי`` was read "אלף בתשרי".
    """

    name = "date-hebrew"
    priority = PRIORITY.DATE
    pattern = re.compile(
        rf"(?<![א-ת])(?P<day>{_GEMATRIA})\s+(?P<prep>[בל]?)(?P<month>{_HEB_MONTH_ALT})"
        rf"(?![א-ת])(?:\s+(?P<year>{_HEB_YEAR}))?"
    )

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        day = gematria(m["day"])
        month = HEBREW_MONTH_VARIANTS.get(m["month"], m["month"])
        limit = 49 if month == OMER else 30
        if not 1 <= day <= limit:
            return m[0]  # numeral-shaped but not a day: leave it alone

        named = HEBREW_DATE_NAMES.get((day, month))
        # The day of a Hebrew month is masculine, as with a Gregorian date.
        spoken = named or f"{numeral(day, Gender.MASC)} {m['prep'] or 'ב'}{month}"
        if m["year"]:
            # A Hebrew year is said as a word (תשפ״ו is /tashpav/), so drop the marks
            # and let the g2p read what is left.
            spoken += " " + m["year"].replace("״", "").replace("׳", "").replace("'", "")
        return spoken


RULES = [
    NumericDateRule(),
    IsoDateRule(),
    TextualDateRule(),
    MonthYearRule(),
    HebrewDateRule(),
]
