"""Adding a rule of your own.

A rule is anything with a name, a priority, a pattern and a ``render``. Here: sports
scores, which the built-in rules deliberately leave alone — the times rule requires
two-digit minutes precisely so that "3:1" is never mistaken for a time.

    uv run examples/custom_rule.py
"""

import regex as re

from heb_tts_normalizer import ALL_RULES, PRIORITY, Config, Gender, normalize
from heb_tts_normalizer.numerals import numeral


class ScoreRule:
    """``ניצחון 3:1`` — two small numbers around a colon, read as a score."""

    name = "score"
    # Below TIME on purpose: a real time wins, and this rule only ever sees what the
    # times rule declined — "3:1" is not a time because minutes must be two digits.
    priority = PRIORITY.TIME - 1
    pattern = re.compile(r"(?<![\d:])(\d{1,2}):(\d{1,2})(?![\d:])")

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        home, away = int(m[1]), int(m[2])
        if home > 30 or away > 30:
            return None  # decline, and let the other rules have it
        return f"{numeral(home, Gender.FEM)} {numeral(away, Gender.FEM)}"


RULES = [*ALL_RULES, ScoreRule()]

for text in ["מכבי ניצחה 3:1", "המשחק הסתיים 2:2", "הפגישה ב-14:30"]:
    print(f"  {text}")
    print(f"→ {normalize(text, rules=RULES)}\n")
