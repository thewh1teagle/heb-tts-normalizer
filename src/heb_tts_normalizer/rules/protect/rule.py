"""Spans that must survive untouched.

A URL is the clearest case: ``example.com/12/03/2026`` contains a perfectly valid
date, and without this rule the dates rule would happily rewrite the middle of the
link. Every other rule sits below PROTECT, so claiming a span here and handing it
back verbatim is what keeps them off it.

Scope is deliberately narrow — text that is *not addressed to a listener*. Things a
speaker would actually read out (a version number, an IP address) are left to the
rules that know how to say them.
"""

from __future__ import annotations

import regex as re

from ...config import Config
from ...scanner import PRIORITY

#: A URL, up to the first whitespace or Hebrew quotation mark. Trailing sentence
#: punctuation is excluded so "בקרו ב-example.com." keeps its full stop.
URL = r"(?:https?://|www\.)[^\s<>\"'׳״]*[^\s<>\"'׳״.,;:!?)\]]"

EMAIL = r"[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)+"

#: A filename with a recognised extension. Restricted to a known extension list on
#: purpose: a bare ``a.b`` pattern would swallow decimals and sentence boundaries.
EXT = (
    "py|js|ts|tsx|jsx|rs|go|java|rb|sh|c|cpp|h|json|toml|yaml|yml|xml|csv|tsv|md|txt"
    "|html?|css|pdf|docx?|xlsx?|pptx?|png|jpe?g|gif|svg|webp|mp3|mp4|wav|mov|zip|tar|gz"
)
FILENAME = rf"[A-Za-z0-9_\-.]+\.(?:{EXT})\b"

#: Hashtags and handles, Hebrew or Latin.
HANDLE = r"[#@][A-Za-z0-9_֐-׿]+"

#: A Latin word fused to digits: GPT-4, COVID-19, iPhone15, v2, MP3. The digits are
#: part of a name, not a quantity, so reading them in Hebrew would be wrong. Note the
#: digits must touch the letters — "5 km" is a measurement and belongs to the units rule.
ALPHANUM = r"\b[A-Za-z]+-?\d+[A-Za-z0-9]*\b"

_PATTERN = re.compile(f"(?:{URL})|(?:{EMAIL})|(?:{FILENAME})|(?:{HANDLE})|(?:{ALPHANUM})")


class ProtectRule:
    """Match it, then give it straight back."""

    name = "protect"
    priority = PRIORITY.PROTECT
    pattern = _PATTERN

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        return m[0]


RULES = [ProtectRule()]
