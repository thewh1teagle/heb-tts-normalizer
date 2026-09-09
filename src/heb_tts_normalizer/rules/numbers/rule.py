"""Bare numbers written in digits: counts, decimals, thousands separators, signs, ordinals.

Covered here, from most specific to least:

- ``1,234`` — comma is a thousands separator, so this is one number. A bare ``1.234``
  is *not*: in Hebrew typography the dot is the decimal point, so ``1.234`` reads as
  "אחת נקודה שתיים שלוש ארבע". There is no comma-decimal form to support.
- ``3.5`` — decimals, fraction digits read one by one ("שלוש נקודה חמש", "שלוש נקודה אחת ארבע").
- ``-5`` / ``+5`` — a leading sign reads as מינוס / פלוס.
- ``3 ילדים`` — a plain integer agrees with the noun that follows it, which means the
  match has to *include* the noun (``count_phrase`` consumes it and orders it correctly:
  ``1 ילד`` becomes "ילד אחד", ``3 הילדים`` becomes "שלושת הילדים"). With nothing to agree
  with, a bare number is read in the feminine — that is how Hebrew counts in the abstract.
- ``ה-3`` — an ordinal, at its own priority so it beats the cardinal reading.
- ``ב-5`` / ``ל-3`` — a one-letter prefix glued to a number with a hyphen. This rule eats
  *only* the prefix and the hyphen, handing the digits straight back to the scanner, so
  currency and units still get their shot: "ו-25₪" comes out "ועשרים וחמישה שקלים".

Judgment calls:

- **What counts as a countable noun.** Anything else would need a lexicon we do not
  have, so the test is deliberately crude and conservative: a Hebrew word of two or
  more letters that is not in ``_STOPWORDS`` (particles, conjunctions, pronouns and
  the handful of adverbs that plausibly follow a number). "3 עד 5" must not become
  "שלושה עד 5", and a wrong *decline* only costs us the feminine default, while a
  wrong *accept* mangles the sentence — so when in doubt the rule declines.
- **``ה-3`` is masculine.** "ה-3" is read השלישי, not השלישית; without knowing the
  elided noun there is no better guess, and the masculine is the citation form.
- **Only ``ה`` + hyphen + digits is an ordinal.** "פרק 3" stays a cardinal ("פרק שלוש"),
  because Hebrew writes an ordinal after a noun that way just as often as a cardinal.
- **Left alone on purpose:** leading zeros (``007`` is a phone/serial matter, not ours),
  digit groups the higher-priority rules already own, and ``1,5`` — a comma decimal is
  not Hebrew usage, so it stays two numbers around a comma.
"""

from __future__ import annotations

import regex as re

from ...config import Config, Gender
from ...numerals import count_phrase, decimal, numeral, ordinal
from ...numerals.words import LEXICON
from ...scanner import PRIORITY

#: Hebrew letters only — ``text.HEB`` also spans niqqud and punctuation, which a
#: countable noun never contains once ``clean`` has run.
_LETTER = "א-ת"

#: ASCII hyphen, non-breaking hyphen, maqaf, en dash, em dash.
_HYPHEN = "\\-\u2011\u05be\u2013\u2014"

#: Words that may follow a number without being counted by it. Particles, conjunctions,
#: pronouns and comparatives — the things that would otherwise be mistaken for a noun.
_STOPWORDS = frozenset(
    {
        "של",
        "את",
        "עד",
        "או",
        "אבל",
        "כי",
        "גם",
        "לא",
        "הוא",
        "היא",
        "הם",
        "הן",
        "זה",
        "זו",
        "אם",
        "כדי",
        "מתוך",
        "על",
        "אל",
        "מן",
        "כמו",
        "יש",
        "אין",
        "היה",
        "היו",
        "רק",
        "כבר",
        "אך",
        "אז",
        "לפני",
        "אחרי",
        "ועד",
        "בלבד",
        "בערך",
        "יותר",
        "פחות",
        "מעל",
        "מתחת",
        "כולל",
        "כמעט",
        "אולי",
        "אפילו",
        "אשר",
        "כדאי",
        "צריך",
        "אפשר",
    }
)

#: A number, optionally with thousands separators, optionally with a decimal part.
#: The grouped alternative comes first so ``1,234`` is never read as a bare ``1``.
_NUMBER = r"(?P<num>\d{1,3}(?:,\d{3})+|\d+)(?:\.(?P<frac>\d+))?"
_INTEGER = r"(?P<num>\d{1,3}(?:,\d{3})+|\d+)"

#: Never start mid-number, and never read one limb of a date or a time. If the dates
#: rule declined ``32/13/2026`` it is malformed, not three numbers, so these guards keep
#: the whole thing in digits rather than reading half of it aloud.
_BEFORE = r"(?<!\d)(?<![:/])"
_AFTER = r"(?!\d)(?![:/]\d)"
_NOUN = rf"(?P<noun>[{_LETTER}]{{2,}})(?![{_LETTER}])"


#: Plural morphology. A counted Hebrew noun is almost always plural ("שלושה ילדים"),
#: so requiring it is what separates a real count from a number that merely happens to
#: sit before a word — "כביש 6 סמוך", "הרצל 25 תל אביב", "פי 3 בעשור".
_PLURAL_SUFFIXES = ("ים", "ות")

#: Singular nouns that are counted anyway. Scale words are the whole list: "2 מיליון"
#: is a count, and there is no plural to look for.
_COUNTABLE_SINGULARS = frozenset({"מיליון", "מיליארד", "טריליון", "אלף", "אלפים"})

#: "פי 3" is a fixed idiom and always masculine — "פי שלושה", never "פי שלוש". Without
#: this it would agree with whatever word happened to follow.
_TIMES_IDIOM = "פי"


def _is_countable(word: str, n: int, cfg: Config) -> bool:
    """Whether ``word`` is plausibly a noun that ``n`` counts.

    Deliberately strict. Getting this wrong does not merely pick the wrong gender: for
    ``n == 1`` the numeral moves behind its noun, so a false positive reorders the
    sentence. A false negative only costs the bare feminine form, which is the correct
    reading for a number that isn't counting anything.
    """
    if len(word) < 2 or word in _STOPWORDS:
        return False
    if word in cfg.gender_overrides:
        # The caller named it and gave it a gender, so it is a noun by declaration.
        # This is the escape hatch for jargon and loanwords no lexicon will carry.
        return True
    if word in _COUNTABLE_SINGULARS:
        return True
    if n == 1:
        # The reordering case: only a noun we actually know about earns it.
        return word in LEXICON
    # A construct plural ends in ־י, not ־ים: "61 חברי כנסת", "3 בתי ספר".
    return word.endswith(_PLURAL_SUFFIXES) or (len(word) >= 3 and word.endswith("י"))


def _read(num: str, frac: str | None, cfg: Config, gender: Gender = Gender.FEM) -> str:
    """Spoken form of one numeric token, decimal or not."""
    whole = int(num.replace(",", ""))
    if frac is None:
        return numeral(whole, gender)
    return decimal(whole, frac, cfg, gender)


_WORD_BEFORE = re.compile(rf"(?P<word>[{_LETTER}]+)\s+$")


def _preceded_by(m: re.Match[str], word: str) -> bool:
    """Whether ``word`` is the token immediately before the match."""
    before = _WORD_BEFORE.search(m.string, 0, m.start())
    return before is not None and before["word"] == word


class CountRule:
    """``3 ילדים`` — an integer agreeing with the noun it counts."""

    name = "count"
    priority = PRIORITY.NUMBER
    pattern = re.compile(rf"{_BEFORE}{_INTEGER}(?!\.?\d){_AFTER}[ ]{_NOUN}")

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        noun = m["noun"]
        n = int(m["num"].replace(",", ""))
        if _preceded_by(m, _TIMES_IDIOM):
            return f"{numeral(n, Gender.MASC)} {noun}"
        if not _is_countable(noun, n, cfg):
            return None
        return count_phrase(n, noun, cfg)


class NumberRule:
    """Any other number: bare, decimal, grouped, or signed."""

    name = "number"
    priority = PRIORITY.NUMBER
    pattern = re.compile(rf"{_BEFORE}(?P<sign>[-+])?{_NUMBER}{_AFTER}")

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        sign = m["sign"]
        if sign is not None:
            # "ב-5" is a prefixed number, not minus five; the hyphen belongs to the letter.
            before = m.string[m.start() - 1] if m.start() else ""
            if before and ("א" <= before <= "ת" or before in _HYPHEN):
                return None
        # "פי 3" is masculine whether or not a noun follows, so the idiom is checked
        # here as well as in CountRule — at the end of a sentence only this rule runs.
        gender = Gender.MASC if _preceded_by(m, _TIMES_IDIOM) else Gender.FEM
        spoken = _read(m["num"], m["frac"], cfg, gender)
        if sign == "-":
            return f"מינוס {spoken}"
        if sign == "+":
            return f"פלוס {spoken}"
        return spoken


class OrdinalRule:
    """``ה-3`` / ``ה־3`` — the one spelling that is unambiguously an ordinal."""

    name = "ordinal"
    priority = PRIORITY.ORDINAL
    pattern = re.compile(rf"ה[{_HYPHEN}](?P<num>\d{{1,3}}){_AFTER}")

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        return ordinal(int(m["num"]), Gender.MASC, definite=True)


class PrefixRule:
    """``ב-5``, ``ל-3``, ``ו-25₪`` — swallow the hyphen, leave the digits to the scanner.

    Consuming only the prefix keeps the number available to currency, units and the
    range rules, all of which key off the digits themselves.
    """

    name = "number-prefix"
    priority = PRIORITY.NUMBER
    # Two-letter prefixes ("וב-", "מל-") are allowed; anything ending in ה is not,
    # so "מה-3" still reaches the ordinal rule as "מ" + "ה-3".
    pattern = re.compile(rf"(?<![{_LETTER}])(?P<pre>[בוכלמש]{{1,2}})[{_HYPHEN}](?=[\d₪$€£])")

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        return m["pre"]


RULES = [CountRule(), NumberRule(), OrdinalRule(), PrefixRule()]
