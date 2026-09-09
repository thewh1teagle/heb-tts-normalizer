"""String hygiene that runs before and after the rule scan.

These are the passes that must not be rules: they rewrite the whole string rather
than one span, and they have to happen before any rule sees the text.
"""

from __future__ import annotations

import unicodedata

import regex as re

from .config import Config

HEB = r"֐-׾"
"""Hebrew letters plus niqqud and the punctuation in the Hebrew block."""

#: Zero-width characters and bidi controls. Invisible to a reader, noise to a g2p.
_INVISIBLE = re.compile("[­​-‏‪-‮⁠-⁤⁦-⁩﻿]")

#: Non-breaking and exotic spaces that should just be a space.
_ODD_SPACE = re.compile("[  -   　]")

_WHITESPACE = re.compile(r"[ \t]{2,}")
_BLANK_LINES = re.compile(r"\n{3,}")
_SPACE_BEFORE_PUNCT = re.compile(r"\s+([,.;:!?])")

#: ASCII quotes used as Hebrew geresh/gershayim: ד"ר, מס'. Canonicalised so a rule
#: can match one spelling instead of four.
_ASCII_GERSHAYIM = re.compile(f'(?<=[{HEB}])"(?=[{HEB}])')
_ASCII_GERESH = re.compile(f"(?<=[{HEB}])'(?![{HEB}])")

GERESH = "׳"
GERSHAYIM = "״"


def clean(text: str, cfg: Config) -> str:
    """Prepare raw text for the scan: strip invisibles, unify quotes, drop Markdown."""
    text = unicodedata.normalize("NFC", text)
    text = _INVISIBLE.sub("", text)
    text = _ODD_SPACE.sub(" ", text)
    text = _ASCII_GERSHAYIM.sub(GERSHAYIM, text)
    text = _ASCII_GERESH.sub(GERESH, text)
    if cfg.strip_markdown:
        from .rules.cleanup.rule import strip_markdown

        text = strip_markdown(text)
    return text


def finalize(text: str, cfg: Config) -> str:
    """Tidy the spacing the rules left behind."""
    if not cfg.clean_whitespace:
        return text
    text = _WHITESPACE.sub(" ", text)
    text = _SPACE_BEFORE_PUNCT.sub(r"\1", text)
    text = _BLANK_LINES.sub("\n\n", text)
    return "\n".join(line.strip() for line in text.split("\n")).strip()
