"""Amounts of money: ``₪25``, ``25 ש״ח``, ``$1,250.50``, ``2 מיליון €``.

Two rules rather than one because the symbol may lead or trail the amount and a
single pattern cannot reuse a group name for both positions. They share all the
rendering.

Judgment calls:

* Two fractional digits are money, not a decimal: ``25.5`` is fifty agorot, not
  "point five". Three or more digits after the point is no longer a price, so it
  falls back to a plain decimal reading. ``cfg.read_minor_currency`` turns the money
  reading off entirely and every fraction is then read as a decimal.
* The amount is glued to the unit's gender, never to a guess from the noun's shape,
  because the lexicon already knows it.
"""

from __future__ import annotations

import regex as re

from ...config import Config, Gender
from ...numerals import decimal, numeral
from ...scanner import PRIORITY
from ...text import HEB
from .data import BY_SPELLING, LEADING, SCALE_ALT, TRAILING, Currency, Unit

#: Optionally comma-grouped, optionally fractional. The comma form is listed first so
#: ``1,250`` is not truncated to ``1``.
_AMOUNT = r"\d{1,3}(?:,\d{3})+(?:\.\d+)?|\d+(?:\.\d+)?"

#: Nothing numeric may touch the amount, so a malformed ``1,25 ₪`` is left to the
#: number rules instead of being half-eaten here.
_NOT_NUMBER_BEFORE = r"(?<![\d,.])"
_NOT_WORD_AFTER = rf"(?![{HEB}0-9A-Za-z׳״])"

#: Beyond this the numeral spelling is noise, not speech.
_MAX_DIGITS = 15


def _count(n: int, unit: Unit) -> str:
    """``n`` of ``unit``, agreeing and correctly ordered."""
    if n == 1:
        # One follows its noun: שקל אחד, אגורה אחת.
        return f"{unit.singular} {numeral(1, unit.gender)}"
    if n == 2:
        # Two is always construct before a noun: שני שקלים, שתי אגורות.
        return f"{numeral(2, unit.gender, construct=True)} {unit.form(2)}"
    return f"{numeral(n, unit.gender)} {unit.form(n)}"


def _scaled(n: int, scale: str, unit: Unit) -> str:
    """``2 מיליון ₪`` -> שני מיליון שקלים. The scale word counts, the unit stays plural."""
    if n == 1:
        head = scale
    elif n == 2:
        head = f"{numeral(2, Gender.MASC, construct=True)} {scale}"
    else:
        head = f"{numeral(n, Gender.MASC)} {scale}"
    return f"{head} {unit.form(2)}"


def _render_amount(raw: str, scale: str | None, cur: Currency, cfg: Config) -> str | None:
    whole_s, _, frac = raw.replace(",", "").partition(".")
    if len(whole_s) > _MAX_DIGITS:
        return None
    whole = int(whole_s)
    major = cur.major

    if scale:
        if not frac:
            return _scaled(whole, scale, major)
        return f"{decimal(whole, frac, cfg, Gender.MASC)} {scale} {major.form(2)}"

    if not frac:
        return _count(whole, major)

    money = cfg.read_minor_currency and cur.minor is not None and len(frac) <= 2
    if not money:
        # Not a price any more — read it as a number, the unit staying plural.
        return f"{decimal(whole, frac, cfg, major.gender)} {major.form(2)}"

    assert cur.minor is not None
    minor_value = int(frac.ljust(2, "0"))
    if minor_value == 0:
        return _count(whole, major)
    minor_words = _count(minor_value, cur.minor)
    if whole == 0:
        return minor_words
    return f"{_count(whole, major)} ו{minor_words}"


def _render(m: re.Match[str], cfg: Config) -> str | None:
    return _render_amount(m["amount"], m["scale"], BY_SPELLING[m["cur"]], cfg)


class CurrencySuffixRule:
    """``25 ₪``, ``25₪``, ``25 שקלים``, ``1,250.50 ש״ח``, ``2 מיליון €``."""

    name = "currency-suffix"
    priority = PRIORITY.CURRENCY
    pattern = re.compile(
        _NOT_NUMBER_BEFORE
        + rf"(?P<amount>{_AMOUNT})"
        + rf"(?:\s(?P<scale>{SCALE_ALT}))?"
        + rf"\s?(?P<cur>{TRAILING})"
        + _NOT_WORD_AFTER
    )

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        return _render(m, cfg)


class CurrencyPrefixRule:
    """``₪25``, ``$1,250``, ``USD 25``, ``€2 מיליון``."""

    name = "currency-prefix"
    priority = PRIORITY.CURRENCY
    pattern = re.compile(
        r"(?<![0-9A-Za-z])"
        + rf"(?P<cur>{LEADING})\s?"
        + rf"(?P<amount>{_AMOUNT})"
        + rf"(?:\s(?P<scale>{SCALE_ALT}))?"
        + _NOT_WORD_AFTER
    )

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        return _render(m, cfg)


RULES = [CurrencySuffixRule(), CurrencyPrefixRule()]
