"""Numeric ranges: ``3-5``, ``3–5 ק״מ``, ``2020-2024``.

A range is read with עד between its ends: "3-5" is "שלוש עד חמש". Two things make it
more than a search-and-replace on the dash:

- **A range that carries a unit or a noun is one span.** "3-5 ק״מ" has to come out
  "שלושה עד חמישה קילומטרים" — both numerals take the *unit's* gender, so the unit
  cannot be left for the units rule to pick up afterwards. That is why this rule sits
  above ``PRIORITY.UNIT``: it claims the whole phrase, unit included.
- **The dash is not a minus.** "-5" alone is מינוס חמש and belongs to the numbers rule;
  the pattern here therefore demands digits on *both* sides of the dash.

Judgment calls:

- **Order is not checked.** "5-3" is still read as a range; refusing it would leave the
  digits to be read separately around a stray dash, which is worse than reading it.
- **An unrecognised tail is left alone.** If the word after the range is a stopword, or a
  unit abbreviation we cannot expand (``expand_units=false``, or one missing from the
  lexicon), the tail rule declines and the bare range matches instead — the word survives
  untouched for whatever rule wants it. The cost is that with ``expand_units=false`` the
  numerals fall back to the feminine, since the unit's gender is exactly what we chose
  not to look up.
- **Years need nothing special:** 2020-2024 is just two cardinals, "אלפיים ועשרים עד
  אלפיים עשרים וארבע".
- Decimal ends are supported ("2.5-3.5"); fraction digits are read one by one as usual.
"""

from __future__ import annotations

from functools import cache
from typing import Any

import regex as re

from ...config import Config, Gender
from ...numerals import decimal, noun_gender, numeral
from ...scanner import PRIORITY

_LETTER = "א-ת"
_HYPHEN = "\\-\u2011\u05be\u2013\u2014"

#: Words that may follow a range without belonging to it.
_STOPWORDS = frozenset(
    {"של", "את", "או", "עד", "אבל", "כי", "גם", "לא", "הוא", "היא", "זה", "אם", "מתוך",
     "על", "אל", "יותר", "פחות", "בלבד", "בערך", "לפחות", "כמעט", "מהם", "לכל"}
)  # fmt: skip

_END = r"\d+(?:\.\d+)?"
#: Digits on *both* sides — that is the whole difference between a range and a minus
#: sign. The guards keep us out of the middle of a date or a time the way the numbers
#: rule does; a trailing full stop is fine, a trailing digit is not.
_RANGE = rf"(?<!\d)(?<![:/])(?P<a>{_END})[ ]?[{_HYPHEN}][ ]?(?P<b>{_END})(?!\d)(?![:/]\d)"
#: A unit abbreviation ("ק״מ", "מ׳") or a plain noun.
_TAIL = rf"(?P<tail>[{_LETTER}]+(?:[״׳][{_LETTER}]*)?)(?![{_LETTER}])"


@cache
def _units() -> dict[str, Any]:
    """The units lexicon, or an empty mapping if it is not importable.

    Another rule folder owns that module; a range should still be readable if the
    import ever changes shape under us, so every failure degrades to "no units known".
    """
    try:
        from ..units import data as unit_data
    except Exception:
        return {}
    for name in ("UNITS", "UNIT_LEXICON", "LEXICON", "ABBREVIATIONS", "ABBREVS"):
        found = getattr(unit_data, name, None)
        if isinstance(found, dict) and found:
            return found
    for found in vars(unit_data).values():
        if isinstance(found, dict) and found and all(isinstance(k, str) for k in found):
            first = next(iter(found.values()))
            if hasattr(first, "plural") and hasattr(first, "gender"):
                return found
    return {}


def _read(token: str, gender: Gender, cfg: Config) -> str:
    """One end of the range, decimal or integer."""
    whole, _, frac = token.partition(".")
    if frac:
        return decimal(int(whole), frac, cfg, gender)
    return numeral(int(whole), gender)


def _span(m: re.Match[str], gender: Gender, cfg: Config) -> str:
    return f"{_read(m['a'], gender, cfg)} עד {_read(m['b'], gender, cfg)}"


class RangeWithTailRule:
    """``3-5 ק״מ`` / ``3-5 ילדים`` — the range and what it measures, agreeing."""

    name = "range-with-tail"
    priority = PRIORITY.RANGE
    pattern = re.compile(rf"{_RANGE}[ ]{_TAIL}")

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        tail = m["tail"]
        unit = _units().get(tail)
        if unit is not None:
            if not cfg.expand_units:
                return None  # leave the abbreviation alone; the bare range still reads
            gender = getattr(unit, "gender", Gender.FEM)
            return f"{_span(m, gender, cfg)} {unit.plural}"
        # An abbreviation we do not know is not a noun either — hand the span back.
        if "״" in tail or "׳" in tail or len(tail) < 2 or tail in _STOPWORDS:
            return None
        return f"{_span(m, noun_gender(tail, cfg), cfg)} {tail}"


class RangeRule:
    """``3-5`` on its own, read in the feminine like any uncounted number."""

    name = "range"
    priority = PRIORITY.RANGE
    pattern = re.compile(_RANGE)

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        return _span(m, Gender.FEM, cfg)


RULES = [RangeWithTailRule(), RangeRule()]
