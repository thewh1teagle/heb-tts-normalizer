"""Numbers followed by a unit of measure: ``5 ק״מ`` -> חמישה קילומטרים.

Two rules live here, both at ``PRIORITY.UNIT``:

``UnitRule``
    A number plus one of the spellings in :data:`data.UNITS`. The unit's gender
    drives the numeral, and the count picks the noun form:

    ==========  ==========================  ===========================
    count       shape                       example
    ==========  ==========================  ===========================
    1           noun then numeral           ``1 ק״מ`` -> קילומטר אחד
    2           construct numeral, plural   ``2 ק״מ`` -> שני קילומטרים
    0, 3+       agreeing numeral, plural    ``5 ק״מ`` -> חמישה קילומטרים
    decimal     numeral, **singular**       ``2.5 ק״ג`` -> ... קילוגרם
    ==========  ==========================  ===========================

    ``1`` follows its noun because Hebrew treats אחד/אחת as an adjective, not a
    count. ``2`` takes the bound form שני/שתי, since שניים/שתיים cannot stand
    before a noun.

    The decimal row is the one judgment call. A fraction is not a plural count —
    Hebrew keeps the singular after it, exactly as with a measurement read off an
    instrument: "שתיים נקודה חמש קילוגרם", never "…קילוגרמים". The number itself is
    read as a bare quantity, so it takes the feminine (the citation form used for
    counting) regardless of the unit's gender.

``DegreeRule``
    The ``°`` sign, with or without a scale letter: ``25°C`` -> עשרים וחמש מעלות
    צלזיוס, ``25°`` -> עשרים וחמש מעלות. מעלה is feminine. Temperature is the one
    place a decimal keeps the *plural* noun — a fever is "שלושים ושמונה נקודה חמש
    מעלות", never "…מעלה" — so the degree rule does not share the singular rule above.

``cfg.expand_units=False`` makes ``UnitRule`` decline, leaving the span untouched
for a caller who would rather keep the abbreviation. ``DegreeRule`` ignores the
flag: ``°`` is a symbol, not a word, and handing it to a g2p unexpanded produces
nothing readable — leaving it alone is never the more faithful choice.
"""

from __future__ import annotations

import regex as re

from ...config import Config
from ...numerals import decimal, numeral
from ...scanner import PRIORITY
from ...text import HEB
from .data import SCALES, UNIT_FORMS, UNITS, Unit

#: An integer, optionally with thousands commas, optionally with a decimal part.
_NUMBER = r"\d{1,3}(?:,\d{3})+(?:\.\d+)?|\d+(?:\.\d+)?"

#: An optional leading minus, claimed here so the number rule does not grab ``-5``
#: and leave ``°C`` stranded. A hyphen after a Hebrew letter is the maqaf of a
#: proclitic (``מ-5 ק״מ``, "from 5 km"), never a sign, so the lookbehind rules it out.
_SIGN = rf"(?:(?<![{HEB}\d])(?P<sign>[-−]) ?)?"

_UNIT_ALT = "|".join(re.escape(form) for form in UNIT_FORMS)

#: A unit must not run into a following letter or digit: ``5 מ׳`` is metres,
#: ``5 מטרו`` is not. Latin symbols need the same guard (``5 kg`` vs ``5 kgs``).
_TAIL = rf"(?![{HEB}A-Za-z0-9׳״])"


def _say(n: int, unit: Unit) -> str:
    """Numeral and unit, agreeing and in the order Hebrew puts them."""
    if n == 1:
        return f"{unit.singular} {numeral(1, unit.gender)}"
    if n == 2:
        return f"{numeral(2, unit.gender, construct=True)} {unit.plural}"
    return f"{numeral(n, unit.gender)} {unit.plural}"


def _signed(m: re.Match[str], said: str) -> str:
    return f"מינוס {said}" if m.group("sign") else said


class UnitRule:
    """A count followed by a unit of measure."""

    name = "unit"
    priority = PRIORITY.UNIT
    pattern = re.compile(rf"{_SIGN}(?<![\d.,])(?P<num>{_NUMBER})[ \t]?(?P<unit>{_UNIT_ALT}){_TAIL}")

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        if not cfg.expand_units:
            return None
        unit = UNITS[m.group("unit")]
        raw = m.group("num").replace(",", "")
        if "." in raw:
            whole, frac = raw.split(".", 1)
            # A measured fraction is not a count: singular noun, bare feminine number.
            return _signed(m, f"{decimal(int(whole), frac, cfg)} {unit.singular}")
        return _signed(m, _say(int(raw), unit))


class DegreeRule:
    """``25°C`` / ``25°`` — the degree sign spelled out."""

    name = "degree"
    priority = PRIORITY.UNIT
    pattern = re.compile(
        rf"{_SIGN}(?<![\d.,])(?P<num>{_NUMBER})[ \t]?°[ \t]?"
        rf"(?P<scale>{'|'.join(SCALES)})?(?![{HEB}A-Za-z0-9])"
    )

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        raw = m.group("num").replace(",", "")
        scale = SCALES.get(m.group("scale") or "")
        if "." in raw:
            whole, frac = raw.split(".", 1)
            # Temperature keeps the plural after a fraction: "…נקודה חמש מעלות".
            said = f"{decimal(int(whole), frac, cfg)} מעלות"
        else:
            said = _say(int(raw), UNITS["מעלות"])
        said = _signed(m, said)
        return f"{said} {scale}" if scale else said


RULES = [UnitRule(), DegreeRule()]
