"""Hebrew abbreviations and acronyms, at ``PRIORITY.ABBREV``.

One rule, two behaviours, because Hebrew abbreviations split in two (see
:mod:`.data` for the full argument):

* a **spelled-out** abbreviation becomes its wording — ``ד״ר כהן`` -> דוקטור כהן;
* a **lexicalised acronym** only loses its gershayim — ``צה״ל`` -> ``צהל`` — because
  a reader says /tsahal/ and expanding it would substitute words for a word.

Attached one-letter proclitics are matched alongside and put back: ``בת״א`` ->
בתל אביב, ``מצה״ל`` -> מצהל. Keeping them out of the lexicon means each entry is
written once instead of once per possible prefix.

``cfg.expand_abbreviations=False`` turns off the first behaviour only. Dropping the
gershayim off ``צה״ל`` is not an expansion — it is the difference between a word the
g2p can read and a string of letters with punctuation in the middle — so it always
happens. This rule sits below units on purpose: ``ג׳`` is grams after a number and
the letter gimel everywhere else, and ``5 ג׳`` should reach the unit rule first.
"""

from __future__ import annotations

import regex as re

from ...config import Config
from ...scanner import PRIORITY
from ...text import HEB
from .data import ABBREVIATIONS, EXPANSIONS, PREFIXES, READ_AS_WORD

#: An abbreviation may not start or end in the middle of a Hebrew word, and the
#: geresh/gershayim count as part of one: ``ג׳ינס`` must not yield "גימל".
_EDGE = rf"[{HEB}׳״]"


class AbbrevRule:
    """A known abbreviation, with an optional attached prefix letter."""

    name = "abbrev"
    priority = PRIORITY.ABBREV
    pattern = re.compile(
        rf"(?<!{_EDGE})([{PREFIXES}]?)({'|'.join(re.escape(a) for a in ABBREVIATIONS)})(?!{_EDGE})"
    )

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        prefix, abbr = m.group(1), m.group(2)
        word = READ_AS_WORD.get(abbr)
        if word is None:
            if not cfg.expand_abbreviations:
                return None
            word = EXPANSIONS[abbr]
        return prefix + word


RULES = [AbbrevRule()]
