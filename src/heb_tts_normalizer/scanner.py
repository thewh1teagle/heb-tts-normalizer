"""One left-to-right pass over the text, highest-priority rule wins at each position.

Chained regex passes would be simpler, and wrong: the numbers pass would eat the
digits inside a date the dates pass already consumed. Here a span is matched once,
rendered once, and never looked at again.
"""

from __future__ import annotations

from collections.abc import Iterator, Sequence
from itertools import groupby
from typing import Protocol, runtime_checkable

import regex as re

from .config import Config
from .text import clean, finalize


@runtime_checkable
class Rule(Protocol):
    """A span matcher. Every rule module exports a list of these as ``RULES``."""

    name: str
    #: Higher wins. See PRIORITY below for the scheme.
    priority: int
    #: Must be able to match at an arbitrary position; avoid a leading ``^``.
    pattern: re.Pattern[str]

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        """Spoken form of the match, or None to decline.

        Declining matters: a dates pattern will happily match ``32/13/2026``, and
        the only place that can be ruled out is after the digits are parsed. A
        declined match falls through to the next candidate at this position.
        """


class PRIORITY:
    """Priority scheme. Rules that consume more context sit above the ones they contain."""

    PROTECT = 100  # claims a span in order to leave it alone: URLs, emails, filenames
    DATE = 90
    TIME = 85
    PHONE = 80
    CURRENCY = 70
    PERCENT = 65
    RANGE = 62  # above UNIT: "3–5 ק״מ" is one span, not a range then a stray unit
    UNIT = 60
    ABBREV = 55
    ORDINAL = 50
    NUMBER = 40


def _candidates(
    text: str, pos: int, groups: Sequence[tuple[int, list[Rule]]]
) -> Iterator[tuple[re.Match[str], Rule]]:
    """Matches at ``pos``, best first: by priority, then by length within a priority."""
    for _priority, rules in groups:
        matches = []
        for rule in rules:
            m = rule.pattern.match(text, pos)
            if m is not None and m.end() > m.start():
                matches.append((m, rule))
        matches.sort(key=lambda pair: -pair[0].end())
        yield from matches


def scan(text: str, rules: Sequence[Rule], cfg: Config) -> str:
    """Rewrite every span a rule claims, leaving everything else byte-identical."""
    ordered = sorted(rules, key=lambda r: -r.priority)
    groups = [(p, list(g)) for p, g in groupby(ordered, key=lambda r: r.priority)]

    out: list[str] = []
    pos, end = 0, len(text)
    while pos < end:
        for m, rule in _candidates(text, pos, groups):
            rendered = rule.render(m, cfg)
            if rendered is not None:
                out.append(rendered)
                pos = m.end()
                break
        else:
            out.append(text[pos])
            pos += 1
    return "".join(out)


def normalize(text: str, cfg: Config | None = None, rules: Sequence[Rule] | None = None) -> str:
    """Turn Hebrew text into something a g2p can read aloud.

    >>> normalize("יש לי 3 ילדים ו-25₪")
    'יש לי שלושה ילדים ועשרים וחמישה שקלים'

    Pass ``rules`` to use a set other than the built-in one — usually
    ``[*ALL_RULES, MyRule()]``. See ``examples/custom_rule.py``.
    """
    from .rules import ALL_RULES

    cfg = cfg or Config()
    return finalize(scan(clean(text, cfg), ALL_RULES if rules is None else rules, cfg), cfg)
