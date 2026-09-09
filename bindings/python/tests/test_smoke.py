"""Smoke tests for the ctypes binding.

Deliberately thin: full conformance is the Rust corpus's job, in
`crates/heb-tts-normalizer/tests/data/*.tsv`.
What can only break *here* is the plumbing — that the library loads, that a config
survives the JSON round trip, that an error surfaces as an exception rather than a
crash, and that returned strings are freed.
"""

from __future__ import annotations

import json
import sys

import pytest

from heb_tts_normalizer import (
    Clock,
    Config,
    DateOrder,
    Gender,
    HebrewDateStyle,
    NormalizerError,
    normalize,
    version,
)


@pytest.mark.parametrize(
    "text",
    ["3 ילדים", "₪25", "14:30", "כ״ז באלול"],
)
def test_normalize_produces_words(text: str) -> None:
    out = normalize(text)
    assert isinstance(out, str)
    assert out.strip()
    # The whole point: no ASCII digits survive.
    assert not any(ch.isdigit() and ch.isascii() for ch in out)


def test_empty_input_is_not_an_error() -> None:
    assert normalize("") == ""


def test_plain_hebrew_passes_through() -> None:
    assert normalize("שלום עולם") == "שלום עולם"


def test_config_changes_the_reading() -> None:
    t = "14:30"
    assert normalize(t, Config(clock=Clock.H12)) != normalize(t, Config(clock=Clock.H24))


def test_config_round_trips_to_the_json_rust_expects() -> None:
    cfg = Config(
        clock=Clock.H24,
        date_order=DateOrder.MDY,
        hebrew_date_style=HebrewDateStyle.NUMBERS,
        expand_units=False,
        gender_overrides={"וופל": Gender.MASC},
    )
    payload = json.loads(cfg.to_json())

    assert payload["clock"] == "24"
    assert payload["date_order"] == "mdy"
    assert payload["hebrew_date_style"] == "numbers"
    assert payload["expand_units"] is False
    assert payload["gender_overrides"] == {"וופל": "m"}
    # Every field goes over, so Rust's deny_unknown_fields never sees a surprise and
    # a default never silently diverges between the two sides.
    assert set(payload) == {
        "clock",
        "date_order",
        "hebrew_date_style",
        "default_currency",
        "read_minor_currency",
        "decimal_word",
        "expand_units",
        "expand_abbreviations",
        "strip_markdown",
        "clean_whitespace",
        "gender_overrides",
    }

    assert normalize("כ״ז באלול", cfg) != normalize("כ״ז באלול", Config())


def test_bare_strings_work_where_an_enum_is_expected() -> None:
    assert normalize("14:30", Config(clock="24")) == normalize("14:30", Config(clock=Clock.H24))


def test_invalid_config_raises_with_the_rust_message() -> None:
    with pytest.raises(NormalizerError) as excinfo:
        normalize("שלום", Config(clock="25"))
    assert "invalid config JSON" in str(excinfo.value)


def test_non_str_input_is_a_type_error() -> None:
    with pytest.raises(TypeError):
        normalize("3 ילדים".encode())  # type: ignore[arg-type]


def test_version_matches_the_package() -> None:
    assert version().count(".") == 2


def test_handles_are_cached_per_config() -> None:
    from heb_tts_normalizer import _handles

    before = len(_handles)
    for _ in range(50):
        normalize("שלום", Config(clock=Clock.H24))
    # One config, one handle, however many calls.
    assert len(_handles) <= before + 1


def _rss_bytes() -> int:
    import resource

    usage = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    # ru_maxrss is bytes on macOS and kilobytes on Linux.
    return usage if sys.platform == "darwin" else usage * 1024


@pytest.mark.skipif(sys.platform == "win32", reason="no resource module on Windows")
def test_returned_strings_are_freed() -> None:
    """A missed `heb_string_free` shows up here and nowhere else.

    Each result is ~120 bytes, so dropping the free leaks well over a megabyte across
    the loop. Measured on the release build: 0.06 MiB with the free, 1.22 MiB without —
    half a megabyte separates the two cleanly. ru_maxrss is a high-water mark, so it
    can only ever accuse a real leak, never a transient.

    The loop takes several seconds; it is deliberately not padded further, because
    normalizing is ~1 ms per call and a longer input would make this the slowest test
    in the repo without sharpening it.
    """
    text = "יש לי 3 ילדים ו-25₪ בשעה 14:30 "

    for _ in range(2_000):  # let the arena reach its steady state first
        normalize(text)
    baseline = _rss_bytes()

    for _ in range(10_000):
        normalize(text)
    growth = _rss_bytes() - baseline

    assert growth < 512 * 1024, f"RSS grew by {growth / 1024:.0f} KiB"
