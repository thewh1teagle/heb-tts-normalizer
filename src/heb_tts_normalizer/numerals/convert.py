"""Turn numbers into Hebrew words: cardinals, decimals, digit strings, ordinals.

Judgment calls made here:

* A cardinal is built as a list of components (milliards, millions, thousands, hundreds,
  tens, ones) and the conjunction ו־ is attached to the *last* component whenever there
  is more than one — this is what a speaker does: מאה ועשרים, אלף מאתיים שלושים וארבע.
* The thousands group changes shape with its count: 1 אלף, 2 אלפיים, 3-10 masculine
  construct + אלפים (חמשת אלפים), and 11+ plain masculine numeral + the *singular* אלף
  (עשרים אלף) — Hebrew stops pluralizing the unit past ten.
* מיליון / מיליארד behave the same way except they have no dual, so 2 takes the bare
  construct שני מיליון.
* Fraction digits are read one by one in the feminine, the unmarked counting series.
"""

from __future__ import annotations

from ..config import Config, Gender
from . import words as w

_MILLIARD = 1_000_000_000
_MILLION = 1_000_000
_THOUSAND = 1_000


def numeral(n: int, gender: Gender = Gender.FEM, *, construct: bool = False) -> str:
    """Cardinal in words.

    ``construct=True`` gives the bound form used before a definite noun: 2 -> שני/שתי,
    3-10 masculine -> שלושת/ארבעת/חמשת… The feminine has no construct above 2, so it
    falls back to the absolute form, which is what Hebrew actually says (שלוש הילדות).
    """
    if n < 0:
        return f"{w.MINUS} {numeral(-n, gender, construct=construct)}"
    if construct:
        bound = w.CONSTRUCT[gender].get(n)
        if bound is not None:
            return bound
    return _join(_components(n, gender))


def decimal(whole: int, frac: str, cfg: Config, gender: Gender = Gender.FEM) -> str:
    """``3, '5'`` -> ``שלוש נקודה חמש``. The fractional part is read digit by digit."""
    return f"{numeral(whole, gender)} {cfg.decimal_word} {digits(frac)}"


def digits(s: str) -> str:
    """``'050'`` -> ``אפס חמש אפס``. Leading zeros are kept; non-digits are dropped."""
    return " ".join(w.ONES_FEM[int(ch)] for ch in s if ch.isdigit())


def ordinal(n: int, gender: Gender = Gender.FEM, *, definite: bool = False) -> str:
    """1-10 -> ראשון..עשירי (feminine ראשונה..עשירית). Above 10 Hebrew uses the cardinal."""
    word = w.ORDINALS[gender].get(n) or numeral(n, gender)
    return f"ה{word}" if definite else word


def _components(n: int, gender: Gender) -> list[str]:
    """The spoken chunks of ``n``, largest first, without any conjunction."""
    if n == 0:
        return [w.ZERO]

    parts: list[str] = []
    rest = n

    for value, name in ((_MILLIARD, w.MILLIARD), (_MILLION, w.MILLION)):
        count, rest = divmod(rest, value)
        if count:
            parts.append(_big_scale(count, name))

    count, rest = divmod(rest, _THOUSAND)
    if count:
        parts.append(_thousands(count))

    parts.extend(_under_thousand(rest, gender))
    return parts


def _under_thousand(n: int, gender: Gender) -> list[str]:
    if n == 0:
        return []
    parts: list[str] = []
    hundreds, rest = divmod(n, 100)
    if hundreds:
        parts.append(w.HUNDREDS[hundreds])
    if 10 <= rest < 20:
        parts.append(w.TEENS[gender][rest - 10])
    elif rest:
        tens, ones = divmod(rest, 10)
        if tens:
            parts.append(w.TENS[tens])
        if ones:
            parts.append(w.ONES[gender][ones])
    return parts


def _thousands(count: int) -> str:
    if count == 1:
        return w.THOUSAND
    if count == 2:
        return w.TWO_THOUSAND
    if count <= 10:
        # 3-10 take the masculine construct with the plural unit: חמשת אלפים.
        return f"{w.CONSTRUCT_MASC[count]} {w.THOUSANDS_PLURAL}"
    return f"{numeral(count, Gender.MASC)} {w.THOUSAND}"


def _big_scale(count: int, name: str) -> str:
    if count == 1:
        return name
    if count == 2:
        return f"{w.CONSTRUCT_MASC[2]} {name}"
    return f"{numeral(count, Gender.MASC)} {name}"


def _join(parts: list[str]) -> str:
    """Space-separate the components, with ו־ on the last one when there are several."""
    if len(parts) == 1:
        return parts[0]
    return f"{' '.join(parts[:-1])} ו{parts[-1]}"
