"""Scanner semantics, exercised with rules defined here rather than the real ones.

The scanner's contract — priority, longest-match, declining, frozen spans — has to hold
independently of what any particular rule does, so these tests deliberately use toy
rules. A change in the dates rule must never be able to break this file.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass, field

import regex as re

from heb_tts_normalizer import Config, Rule, scan

CFG = Config()


@dataclass
class FakeRule:
    """A rule whose behaviour is spelled out at the call site."""

    name: str
    priority: int
    pattern: re.Pattern[str]
    #: Rendered output. ``None`` declines every match; a callable decides per match.
    output: str | None = None
    decline_when: Callable[[re.Match[str]], bool] | None = None
    #: Every match text this rule was offered, in order — for span-freezing assertions.
    seen: list[str] = field(default_factory=list)

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        self.seen.append(m.group(0))
        if self.decline_when is not None and self.decline_when(m):
            return None
        return self.output


def rule(
    name: str,
    priority: int,
    pattern: str,
    output: str | None,
    decline_when: Callable[[re.Match[str]], bool] | None = None,
) -> FakeRule:
    return FakeRule(
        name=name,
        priority=priority,
        pattern=re.compile(pattern),
        output=output,
        decline_when=decline_when,
    )


def test_fake_rule_satisfies_the_protocol() -> None:
    assert isinstance(rule("x", 10, r"x", "X"), Rule)


def test_untouched_text_is_byte_identical() -> None:
    r = rule("digits", 10, r"\d+", "N")
    assert scan("שלום עולם", [r], CFG) == "שלום עולם"
    assert r.seen == []


def test_single_match_is_replaced() -> None:
    r = rule("digits", 10, r"\d+", "מספר")
    assert scan("יש 42 כאן", [r], CFG) == "יש מספר כאן"


def test_higher_priority_wins_at_the_same_position() -> None:
    low = rule("low", 10, r"\d+", "LOW")
    high = rule("high", 90, r"\d+", "HIGH")
    assert scan("12", [low, high], CFG) == "HIGH"
    assert scan("12", [high, low], CFG) == "HIGH", "registration order must not matter"
    assert low.seen == [], "a lower-priority rule is never consulted once a higher one renders"


def test_higher_priority_wins_even_with_a_shorter_match() -> None:
    """Priority beats length across priorities — that is what the scheme is for."""
    short_high = rule("short-high", 90, r"\d\d", "HH")
    long_low = rule("long-low", 10, r"\d+", "LLLL")
    assert scan("123456", [short_high, long_low], CFG) == "HHHHHH"


def test_longest_match_wins_within_a_priority() -> None:
    two = rule("two", 50, r"\d\d", "TWO")
    four = rule("four", 50, r"\d\d\d\d", "FOUR")
    assert scan("1234", [two, four], CFG) == "FOUR"
    assert two.seen == []


def test_declining_falls_through_to_the_next_candidate() -> None:
    """A rule that matches but cannot parse hands the span back."""
    picky = rule(
        "picky",
        90,
        r"\d+",
        "PICKY",
        decline_when=lambda m: int(m.group(0)) > 12,
    )
    fallback = rule("fallback", 10, r"\d+", "FALLBACK")

    assert scan("7", [picky, fallback], CFG) == "PICKY"
    assert scan("99", [picky, fallback], CFG) == "FALLBACK"
    assert picky.seen == ["7", "99"], "the declining rule was offered both spans"


def test_declining_within_one_priority_falls_to_the_shorter_match() -> None:
    long_decliner = rule("long", 50, r"\d\d\d\d", None)
    short = rule("short", 50, r"\d\d", "SS")
    assert scan("1234", [long_decliner, short], CFG) == "SSSS"


def test_a_rule_that_declines_everywhere_leaves_text_untouched() -> None:
    never = rule("never", 90, r"\d+", None)
    text = "יש לי 3 ילדים ו-25 שקלים"
    assert scan(text, [never], CFG) == text
    # After a decline the scanner emits one character and retries at the next position,
    # so the tail of a declined span is offered again ("25" declines, then "5" is tried).
    assert never.seen == ["3", "25", "5"], "it was tried, it just never accepted"


def test_matched_spans_are_frozen() -> None:
    """A rule's output is never re-scanned, even when it looks like another rule's input."""
    producer = rule("producer", 90, r"A", "B")
    consumer = rule("consumer", 90, r"B", "C")
    assert scan("A", [producer, consumer], CFG) == "B"
    assert consumer.seen == []


def test_a_rule_does_not_re_match_its_own_output() -> None:
    """The obvious infinite-loop shape: a rule that emits what it matches."""
    r = rule("echo", 50, r"\d+", "12")
    assert scan("999", [r], CFG) == "12"
    assert r.seen == ["999"]


def test_unmatched_text_between_spans_is_preserved_exactly() -> None:
    r = rule("digits", 50, r"\d+", "#")
    assert scan("  1, 2 -- 3  ", [r], CFG) == "  #, # -- #  "


def test_word_boundary_anchors_mid_string() -> None:
    r"""``\b`` must see the character before the scan position, not a fresh string start.

    The scanner matches with ``pattern.match(text, pos)``, which keeps the preceding
    character in view. Slicing the string instead would make ``\bcat`` match inside
    ``bobcat``.
    """
    r = rule("cat", 50, r"\bcat\b", "CAT")
    assert scan("a cat here", [r], CFG) == "a CAT here"
    assert scan("bobcat", [r], CFG) == "bobcat"
    assert r.seen == ["cat"]


def test_lookbehind_sees_preceding_text() -> None:
    r = rule("after-dash", 50, r"(?<=-)\d+", "N")
    assert scan("-25 and 25", [r], CFG) == "-N and 25"


def test_zero_width_match_is_ignored() -> None:
    """An empty match would advance nowhere; the scanner must not accept it."""
    empty = rule("empty", 90, r"\d*", "X")
    assert scan("abc", [empty, rule("noop", 10, r"zzz", "Z")], CFG) == "abc"


def test_zero_width_match_does_not_block_a_real_one() -> None:
    empty = rule("empty", 90, r"\d*", "X")
    real = rule("real", 50, r"[a-z]+", "W")
    assert scan("abc", [empty, real], CFG) == "W"


def test_empty_rule_list_is_a_no_op() -> None:
    assert scan("14:30 ו-3 ק״מ", [], CFG) == "14:30 ו-3 ק״מ"


def test_empty_text() -> None:
    assert scan("", [rule("digits", 50, r"\d+", "N")], CFG) == ""


def test_config_is_passed_through_to_render() -> None:
    seen: list[Config] = []

    class Recorder:
        name = "recorder"
        priority = 50
        pattern = re.compile(r"\d+")

        def render(self, m: re.Match[str], cfg: Config) -> str | None:
            seen.append(cfg)
            return "N"

    cfg = Config(decimal_word="פסיק")
    scan("5", [Recorder()], cfg)
    assert seen == [cfg]


def test_rules_run_left_to_right() -> None:
    r = rule("digits", 50, r"\d+", "N")
    scan("1 22 333", [r], CFG)
    assert r.seen == ["1", "22", "333"]
