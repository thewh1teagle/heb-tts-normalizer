"""Clock times: ``14:30``, ``9:05``, ``20:00``, ``14:30:15`` and ``9 בבוקר``.

Judgment calls made here:

- **Hours are feminine** (שעה is feminine): 14 is "ארבע עשרה", 2 is "שתיים". Minutes
  are counted the same way, for the same reason (דקה is feminine).
- **``cfg.clock`` picks the register.** ``Clock.H24`` reads the digits as they are
  written — "ארבע עשרה שלושים", "עשרים אפס אפס" — the way a timetable or a radio
  bulletin is read. ``Clock.H12`` (the default) converts and adds the daypart, the
  way a person answers "what time is it".
- **A minute under ten keeps its zero in H24** ("תשע אפס חמש"), because that is what
  the written form says; in H12 it becomes a plain conjunction ("תשע וחמש").
- **Dayparts** by the 24-hour hour: 05:00-11:59 בבוקר, 12:00-16:59 אחר הצהריים,
  17:00-20:59 בערב, 21:00-04:59 בלילה. Two overrides: the whole 12th hour is
  בצהריים ("שתים עשרה וחצי בצהריים" is what people say, not "אחר הצהריים"), and
  00:00 exactly is חצות — nobody says "שתים עשרה בלילה" for midnight on the dot.
- **The quarter-to idiom keeps the daypart of the hour it is spoken in**, not of the
  hour it counts towards: 16:45 is "רבע לחמש אחר הצהריים", because at 16:45 it is
  still the afternoon.
- **Seconds break every idiom**, so ``h:mm:ss`` is always read plainly, in both clock
  configs — a timestamp is a timestamp, not a time of day.
- Declines anything outside a real clock (``25:70``), which is also what keeps scores
  and ratios out: those are written ``2:1``, and a two-digit minute is required.
"""

from __future__ import annotations

import regex as re

from ...config import Clock, Config, Gender
from ...numerals import digits, numeral
from ...scanner import PRIORITY

MORNING = "בבוקר"
AFTERNOON = "אחר הצהריים"
NOON = "בצהריים"
EVENING = "בערב"
NIGHT = "בלילה"
MIDNIGHT = "חצות"

#: Daypart words a written time may spell out itself ("9 בבוקר").
_SPOKEN_DAYPARTS = (AFTERNOON, MORNING, NOON, EVENING, NIGHT)

#: The same, as they appear *after* a clock time ("9:00 בבוקר"), plus the abbreviation
#: the abbrev rule has not reached yet at this point in the scan.
_WRITTEN_DAYPART = re.compile(rf"\s+(?:{'|'.join(_SPOKEN_DAYPARTS)}|אחה״צ|לפנות בוקר)(?![א-ת])")


def _daypart(hour24: int) -> str:
    if hour24 == 12:
        return NOON
    if 5 <= hour24 <= 11:
        return MORNING
    if 13 <= hour24 <= 16:
        return AFTERNOON
    if 17 <= hour24 <= 20:
        return EVENING
    return NIGHT


def _hour_word(hour24: int) -> str:
    """The 12-hour face value, feminine. Both 0 and 12 read as שתים עשרה."""
    return numeral(hour24 % 12 or 12, Gender.FEM)


def _minutes_plain(value: int, raw: str) -> str:
    """H24 minute/second reading: keep the leading zero of anything under ten."""
    return digits(raw) if value < 10 else numeral(value, Gender.FEM)


def _render_h24(hour: int, minute: int, raw_minute: str) -> str:
    return f"{numeral(hour, Gender.FEM)} {_minutes_plain(minute, raw_minute)}"


def _render_h12(hour: int, minute: int, *, part: str | None = None) -> str:
    """``part=""`` suppresses the daypart, for when the text already spells one out."""
    if hour == 0 and minute == 0:
        return MIDNIGHT
    part = _daypart(hour) if part is None else part
    if minute == 0:
        return f"{_hour_word(hour)} {part}"
    if minute == 15:
        return f"{_hour_word(hour)} ורבע {part}"
    if minute == 30:
        return f"{_hour_word(hour)} וחצי {part}"
    if minute == 45:
        # Counted towards the next hour, but spoken in the current one's daypart.
        return f"רבע ל{_hour_word(hour + 1)} {part}"
    return f"{_hour_word(hour)} ו{numeral(minute, Gender.FEM)} {part}"


class ClockTimeRule:
    """``14:30``, ``9:05``, ``20:00``, ``14:30:15``."""

    name = "time-clock"
    priority = PRIORITY.TIME
    pattern = re.compile(r"(?<![\d:.])(\d{1,2}):(\d{2})(?::(\d{2}))?(?![\d:])")

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        hour, minute = int(m[1]), int(m[2])
        second = int(m[3]) if m[3] else None
        if hour > 23 or minute > 59 or (second is not None and second > 59):
            return None
        if second is not None:
            # Seconds present: read the three groups as written, whatever the config.
            plain = _render_h24(hour, minute, m[2])
            return f"{plain} {_minutes_plain(second, m[3])}"
        if cfg.clock is Clock.H24:
            return _render_h24(hour, minute, m[2])
        # "9:00 בבוקר" already says which part of the day it is; adding ours would
        # read "בתשע בבוקר בבוקר".
        part = "" if _WRITTEN_DAYPART.match(m.string, m.end()) else None
        return _render_h12(hour, minute, part=part).rstrip()


class SpokenHourRule:
    """``9 בבוקר``, ``8 בערב``, ``3 אחר הצהריים`` — the daypart is already a word."""

    name = "time-spoken-hour"
    priority = PRIORITY.TIME
    pattern = re.compile(rf"(?<![\d:.])(\d{{1,2}})\s+({'|'.join(_SPOKEN_DAYPARTS)})(?![א-ת])")

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        hour = int(m[1])
        # Only a clock-face hour can carry a daypart; "24 בערב" is not a time.
        if not 1 <= hour <= 12:
            return None
        return f"{numeral(hour, Gender.FEM)} {m[2]}"


RULES = [ClockTimeRule(), SpokenHourRule()]
