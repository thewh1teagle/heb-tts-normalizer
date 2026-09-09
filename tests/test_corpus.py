"""Every corpus row through the full public ``normalize()``.

This is the test that catches cross-rule interference. A unit test of the dates rule
cannot notice that its pattern started stealing ``14:30`` from times; a whole-pipeline
test on a sentence containing both does.
"""

from __future__ import annotations

import pytest
import regex as re
from conftest import Row

from heb_tts_normalizer import Config, normalize

#: Digits and Hebrew points/accents. The library exists to remove both: a g2p that
#: sees "14" or a vowel-pointed word has been handed the problem it was meant to
#: be spared.
_DIGIT = re.compile(r"[0-9٠-٩]")
#: Vowel points and cantillation only. Maqaf (05BE), paseq (05C0), sof pasuq (05C3)
#: and 05C6 are punctuation and are allowed to survive into the output.
_NIQQUD = re.compile(r"[֑-ׇֽֿׁׂׅׄ]")

#: Rows that legitimately keep a digit in the output, keyed by test id
#: (``file.tsv:lineno``) with the reason. Malformed input a rule declines is the
#: usual case: ``32/13/2026`` is not a date, so it survives verbatim.
DIGIT_EXEMPT: dict[str, str] = {}


def test_empty_string() -> None:
    assert normalize("") == ""


def test_whitespace_only() -> None:
    assert normalize("   \n  ") == ""


@pytest.mark.parametrize(
    "text",
    [
        "שלום עולם",
        "הוא הלך לבית הספר ולא חזר",
        "מה נשמע? הכל בסדר, תודה.",
    ],
)
def test_plain_hebrew_unchanged(text: str) -> None:
    """Nothing to normalize means nothing changes."""
    assert normalize(text) == text


def test_corpus_row(row: Row) -> None:
    actual = normalize(row.input, row.config)
    assert actual == row.expected, row.describe(actual)


def test_corpus_row_is_idempotent(row: Row) -> None:
    """Normalizing spoken output again must be a no-op.

    A rule that rewrites its own output loops or corrupts on a second pass, and callers
    do re-run this over already-clean text.
    """
    once = normalize(row.input, row.config)
    twice = normalize(once, row.config)
    assert twice == once, (
        f"{row.id}: normalize() is not idempotent\n"
        f"  input  {row.input}\n"
        f"  once   {once}\n"
        f"  twice  {twice}"
    )


def test_corpus_row_has_no_digits(row: Row) -> None:
    actual = normalize(row.input, row.config)
    if row.id in DIGIT_EXEMPT:
        pytest.skip(f"documented exemption: {DIGIT_EXEMPT[row.id]}")
    if row.expected == row.input:
        # The row asserts the text is left alone (a URL, a filename, an unparseable
        # date). Its digits are deliberate, and test_corpus_row already pins the output.
        pytest.skip("row asserts a deliberate passthrough")
    found = _DIGIT.findall(actual)
    assert not found, (
        f"{row.id}: output still contains digits {found}\n"
        f"  input  {row.input}\n"
        f"  actual {actual}\n"
        "  If this row is meant to keep a digit, add it to DIGIT_EXEMPT with a reason."
    )


def test_corpus_row_has_no_niqqud(row: Row) -> None:
    actual = normalize(row.input, row.config)
    found = _NIQQUD.findall(actual)
    assert not found, (
        f"{row.id}: output contains niqqud {[hex(ord(c)) for c in found]}\n"
        f"  input  {row.input}\n"
        f"  actual {actual}"
    )


def test_corpus_row_is_finalized(row: Row) -> None:
    """``expected`` is post-``finalize`` output: stripped, single-spaced."""
    assert row.expected == row.expected.strip(), f"{row.id}: expected has surrounding whitespace"
    assert "  " not in row.expected, f"{row.id}: expected contains a double space"


def test_config_defaults_are_used_when_column_empty() -> None:
    """Guards the loader itself: an empty config column means the real defaults."""
    from conftest import parse_config

    assert parse_config("") == Config()
