"""Currency lexicon: how each currency is spelled, and how its units are counted.

A currency is two counted nouns (major and minor) plus the set of spellings that
select it. The nouns carry their own gender because Hebrew agreement is a property
of the word, not of the amount: ``25 ₪`` is *עשרים וחמישה שקלים* (masculine) while
the same 25 agorot is *עשרים וחמש אגורות* (feminine).

Indeclinability is modelled as a first-class property of a unit rather than a
per-currency exception. אירו and פני simply have no plural — *עשרים וחמישה אירו*,
never *אירואים* — and any unit may be like that, so the rule never special-cases a
currency code.
"""

from __future__ import annotations

from dataclasses import dataclass

import regex as re

from ...config import Gender
from ...text import HEB


@dataclass(frozen=True, slots=True)
class Unit:
    """One counted noun of a currency (a shekel, an agora, a cent)."""

    singular: str
    #: The counting plural, or ``None`` for an indeclinable noun (אירו, פני).
    plural: str | None
    gender: Gender

    @property
    def indeclinable(self) -> bool:
        """True when the noun keeps its singular shape after any numeral."""
        return self.plural is None

    def form(self, n: int) -> str:
        """The shape the noun takes when counting ``n`` of it."""
        if self.indeclinable or n == 1:
            return self.singular
        # indeclinable already returned above, so plural is present; the fallback
        # keeps the type honest without an ignore comment.
        return self.plural or self.singular


@dataclass(frozen=True, slots=True)
class Currency:
    """A currency and every written form that selects it."""

    code: str
    major: Unit
    #: Hundredth part, or ``None`` where we never read one aloud.
    minor: Unit | None
    spellings: tuple[str, ...]


CURRENCIES: tuple[Currency, ...] = (
    Currency(
        code="ILS",
        major=Unit("שקל", "שקלים", Gender.MASC),
        minor=Unit("אגורה", "אגורות", Gender.FEM),
        spellings=("₪", "ש״ח", 'ש"ח', "שקלים", "שקל", "ILS", "NIS"),
    ),
    Currency(
        code="USD",
        major=Unit("דולר", "דולרים", Gender.MASC),
        minor=Unit("סנט", "סנטים", Gender.MASC),
        spellings=("$", "דולרים", "דולר", "USD"),
    ),
    Currency(
        code="EUR",
        # אירו is indeclinable: the plural *אירואים does not exist in speech.
        major=Unit("אירו", None, Gender.MASC),
        minor=Unit("סנט", "סנטים", Gender.MASC),
        spellings=("€", "אירו", "יורו", "EUR"),
    ),
    Currency(
        code="GBP",
        major=Unit("פאונד", "פאונדים", Gender.MASC),
        # פני is borrowed whole and never counted as *פנים.
        minor=Unit("פני", None, Gender.MASC),
        spellings=("£", "פאונדים", "פאונד", "שטרלינג", "GBP"),
    ),
)

BY_SPELLING: dict[str, Currency] = {s: c for c in CURRENCIES for s in c.spellings}

_HEBREW = re.compile(f"[{HEB}]")


def _alternation(spellings: list[str]) -> str:
    """Regex alternation, longest first so שקלים wins over שקל."""
    return "|".join(re.escape(s) for s in sorted(spellings, key=len, reverse=True))


#: Every spelling, usable after the amount: ``25 ₪``, ``25 שקלים``.
TRAILING = _alternation(list(BY_SPELLING))

#: Spellings that may also precede the amount (``$25``, ``USD 25``). Hebrew currency
#: names never do — nobody writes *שקל 25* — so they are excluded here.
LEADING = _alternation([s for s in BY_SPELLING if not _HEBREW.search(s)])

#: Scale words that stay words in the output: ``2 מיליון ₪`` -> שני מיליון שקלים.
SCALES: tuple[str, ...] = ("מיליון", "מיליארד", "טריליון")
SCALE_ALT = _alternation(list(SCALES))
