"""Percentages: ``12%``, ``12.5%``, ``12 אחוז``, and the per-mille sign ``5‰``.

The one thing worth getting right here: **אחוז is masculine**. ``12%`` is
*שנים עשר אחוזים*, not *שתים עשרה אחוזים* — the feminine bare-counting form that a
naive digit-to-word pass produces is the usual mistake.

פרומיל is a borrowing and is not counted in the plural, so ``5‰`` is *חמישה פרומיל*.

When the noun is already spelled out (``12 אחוז``) it is kept exactly as written and
only the digits are converted; picking a number for the writer would be a rewrite,
not a normalization. Ranges (``3%-5%``) belong to the ranges rule.
"""

from __future__ import annotations

import regex as re

from ...config import Config, Gender
from ...numerals import decimal, numeral
from ...scanner import PRIORITY
from ...text import HEB

_AMOUNT = r"\d{1,3}(?:,\d{3})+(?:\.\d+)?|\d+(?:\.\d+)?"
_NOT_NUMBER_BEFORE = r"(?<![\d,.])"
_NOT_WORD_AFTER = rf"(?![{HEB}0-9A-Za-z׳״])"

_PERCENT_SINGULAR = "אחוז"
_PERCENT_PLURAL = "אחוזים"
#: Indeclinable, hence one form for every count.
_PERMILLE = "פרומיל"

#: Spelled-out nouns we recognise after the digits, longest first.
_SPELLED = "|".join(("אחוזים", "אחוז", "פרומילים", "פרומיל"))

_MAX_DIGITS = 15


def _phrase(n: int, noun: str) -> str:
    if n == 1:
        # One follows its noun, and אחוז is masculine: אחוז אחד.
        return f"{noun} אחד"
    if n == 2:
        return f"{numeral(2, Gender.MASC, construct=True)} {noun}"
    return f"{numeral(n, Gender.MASC)} {noun}"


def _render(raw: str, noun_written: str | None, sign: str | None, cfg: Config) -> str | None:
    whole_s, _, frac = raw.replace(",", "").partition(".")
    if len(whole_s) > _MAX_DIGITS:
        return None
    whole = int(whole_s)

    if noun_written is not None:
        noun = noun_written
    elif sign == "‰":
        noun = _PERMILLE
    else:
        noun = _PERCENT_SINGULAR if whole == 1 and not frac else _PERCENT_PLURAL

    if frac:
        return f"{decimal(whole, frac, cfg, Gender.MASC)} {noun}"
    return _phrase(whole, noun)


class PercentSignRule:
    """``12%``, ``12.5 %``, ``5‰``."""

    name = "percent-sign"
    priority = PRIORITY.PERCENT
    pattern = re.compile(_NOT_NUMBER_BEFORE + rf"(?P<amount>{_AMOUNT})\s?(?P<sign>[%‰])")

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        return _render(m["amount"], None, m["sign"], cfg)


class PercentWordRule:
    """``12 אחוז`` — the noun is already there, only the digits need reading."""

    name = "percent-word"
    priority = PRIORITY.PERCENT
    pattern = re.compile(
        _NOT_NUMBER_BEFORE + rf"(?P<amount>{_AMOUNT})\s(?P<noun>{_SPELLED})" + _NOT_WORD_AFTER
    )

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        return _render(m["amount"], m["noun"], None, cfg)


RULES = [PercentSignRule(), PercentWordRule()]
