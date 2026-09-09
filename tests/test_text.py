"""The pre- and post-passes in ``text.py``.

These run outside the scanner, so they get tested outside it too. Markdown stripping is
switched off throughout: it lives in the cleanup rule, and this file is about hygiene
that ``clean``/``finalize`` own themselves.
"""

from __future__ import annotations

import unicodedata

import pytest

from heb_tts_normalizer import Config
from heb_tts_normalizer.text import GERESH, GERSHAYIM, clean, finalize

#: Markdown stripping belongs to the cleanup rule; keep it out of these assertions.
CFG = Config(strip_markdown=False)


def _clean(text: str) -> str:
    return clean(text, CFG)


def _finalize(text: str) -> str:
    return finalize(text, CFG)


# --- NFC ------------------------------------------------------------------------


def test_clean_output_is_nfc() -> None:
    text = _clean("café שלום")
    assert text == unicodedata.normalize("NFC", text)
    assert text.startswith("café")


def test_hebrew_presentation_form_is_decomposed_by_nfc() -> None:
    """U+FB1D (yod with hiriq) is a composition exclusion, so NFC leaves it split."""
    assert _clean("יִ") == "יִ"


def test_clean_does_not_disturb_plain_hebrew() -> None:
    assert _clean("שלום עולם") == "שלום עולם"


# --- invisibles -----------------------------------------------------------------


@pytest.mark.parametrize(
    "char",
    [
        "​",  # zero width space
        "‌",  # ZWNJ
        "‍",  # ZWJ
        "­",  # soft hyphen
        "﻿",  # BOM / zero width no-break space
        "⁠",  # word joiner
    ],
)
def test_zero_width_characters_are_removed(char: str) -> None:
    assert _clean(f"של{char}ום") == "שלום"


@pytest.mark.parametrize(
    "char",
    [
        "‎",  # LRM
        "‏",  # RLM
        "‪",  # LRE
        "‫",  # RLE
        "‬",  # PDF
        "‭",  # LRO
        "‮",  # RLO
        "⁦",  # LRI
        "⁩",  # PDI
    ],
)
def test_bidi_controls_are_removed(char: str) -> None:
    assert _clean(f"{char}שלום 5{char}") == "שלום 5"


def test_bidi_wrapped_number_survives_as_a_bare_number() -> None:
    """Copy-paste from a browser wraps digits in isolates; a rule must still see them."""
    assert _clean("יש ⁦123⁩ שקלים") == "יש 123 שקלים"


# --- spaces ---------------------------------------------------------------------


@pytest.mark.parametrize(
    "char",
    [
        " ",  # NBSP
        " ",  # figure space
        " ",  # narrow NBSP
        " ",  # thin space
        "　",  # ideographic space
    ],
)
def test_exotic_spaces_become_plain_spaces(char: str) -> None:
    assert _clean(f"3{char}ק״מ") == "3 ק״מ"


def test_nbsp_between_number_and_unit_is_a_single_plain_space() -> None:
    assert _clean("25 ₪") == "25 ₪"


# --- geresh / gershayim ---------------------------------------------------------


def test_ascii_double_quote_between_hebrew_letters_becomes_gershayim() -> None:
    assert _clean('ד"ר כהן') == f"ד{GERSHAYIM}ר כהן"
    assert _clean('3 ק"מ') == f"3 ק{GERSHAYIM}מ"


def test_ascii_apostrophe_after_a_hebrew_letter_becomes_geresh() -> None:
    assert _clean("מס' 5") == f"מס{GERESH} 5"


def test_apostrophe_between_hebrew_letters_is_left_alone() -> None:
    """``ג'ירפה`` — the geresh rule only fires when no Hebrew letter follows."""
    assert _clean("ג'ירפה") == "ג'ירפה"


@pytest.mark.parametrize("text", ["don't", "it's fine", "the 90's"])
def test_english_contractions_are_not_touched(text: str) -> None:
    assert _clean(text) == text


def test_quote_around_a_whole_hebrew_phrase_is_not_converted() -> None:
    """A quotation is not an abbreviation: neither quote has Hebrew on both sides."""
    assert _clean('אמר "שלום עולם" ויצא') == 'אמר "שלום עולם" ויצא'


def test_quote_around_a_single_hebrew_word_is_not_converted() -> None:
    assert _clean('הוא "צדק"') == 'הוא "צדק"'


def test_existing_gershayim_is_left_as_is() -> None:
    assert _clean(f"ד{GERSHAYIM}ר") == f"ד{GERSHAYIM}ר"


# --- finalize -------------------------------------------------------------------


def test_finalize_collapses_runs_of_spaces() -> None:
    assert _finalize("שלום    עולם") == "שלום עולם"


def test_finalize_collapses_tabs() -> None:
    assert _finalize("שלום\t\tעולם") == "שלום עולם"


def test_finalize_strips_surrounding_whitespace() -> None:
    assert _finalize("   שלום עולם   ") == "שלום עולם"


def test_finalize_removes_space_before_punctuation() -> None:
    assert _finalize("שלום , עולם !") == "שלום, עולם!"


@pytest.mark.parametrize("punct", [",", ".", ";", ":", "!", "?"])
def test_space_before_each_punctuation_mark_is_removed(punct: str) -> None:
    assert _finalize(f"מילה {punct}") == f"מילה{punct}"


def test_finalize_keeps_the_space_after_punctuation() -> None:
    assert _finalize("שלום , עולם . מה נשמע ?") == "שלום, עולם. מה נשמע?"


def test_finalize_collapses_blank_lines() -> None:
    assert _finalize("שורה\n\n\n\n\nשורה") == "שורה\n\nשורה"


def test_finalize_keeps_a_single_blank_line() -> None:
    assert _finalize("שורה\n\nשורה") == "שורה\n\nשורה"


def test_finalize_strips_each_line() -> None:
    assert _finalize("  שורה אחת  \n   שורה שתיים  ") == "שורה אחת\nשורה שתיים"


def test_finalize_of_empty_string() -> None:
    assert _finalize("") == ""


def test_finalize_of_whitespace_only() -> None:
    assert _finalize("   \n\n   ") == ""


def test_finalize_is_idempotent() -> None:
    once = _finalize("  שלום  ,  עולם  \n\n\n\n  סוף  ")
    assert _finalize(once) == once


def test_finalize_respects_clean_whitespace_false() -> None:
    """An opt-out means genuinely untouched, spacing warts and all."""
    text = "  שלום  ,  עולם  "
    assert finalize(text, Config(clean_whitespace=False)) == text
