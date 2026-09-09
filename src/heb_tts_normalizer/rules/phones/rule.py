"""Telephone-shaped digit runs, read one digit at a time.

Covers the Israeli formats a reader actually meets: mobile and landline numbers with
or without separators (``050-1234567``, ``03-1234567``, ``0501234567``), the
international ``+972`` form, star numbers (``*6555``), ``1-800``/``1-700`` service
lines, emergency short codes, national ID numbers, credit-card groups and IP-like
dotted quads.

Judgment calls
--------------
* This rule sits above dates and numbers, so it must be *stingy*. Every pattern here
  demands a real phone shape — a known prefix (``0``+area code, ``+972``, ``1-800``,
  ``*``) or an explicit grouping. A bare run of seven or nine digits is never claimed;
  that way ``1234567`` still reaches the numbers rule as a quantity.
* Separators (``-``, ``–``, ``.``, space) are silent. Nobody says "מקף" mid-number.
* Emergency codes (100/101/102…) and ID numbers are indistinguishable from ordinary
  quantities on their own, so they are claimed only when a dialling or identity cue
  ("חייגו", "מוקד", "תעודת זהות"…) sits just before them.
* Digits are read with :func:`digits`, i.e. the feminine forms — that is how a phone
  number is dictated in Hebrew regardless of what it refers to.
"""

from __future__ import annotations

import regex as re

from ...config import Config
from ...numerals import digits
from ...scanner import PRIORITY

#: Characters that may sit between groups of a phone number, and are never spoken.
_SEP = r"[-–—. ]"

_NOT_DIGITS = re.compile(r"\D+")

#: Words that make a bare three-digit code a number to dial rather than a quantity.
_DIAL_CUE = re.compile(
    r"(?:חייג|חייגו|חייגי|חייגה|התקשר|התקשרו|התקשרי|טלפון|טל[׳']?|נייד|פקס|"
    r"מוקד|למוקד|משטרה|למשטרה|מד״א|כבאות|אמבולנס|חירום|שלוחה|מספר)"
    r"[\s:,\-–]*$"
)

#: Words that make a nine-digit run an identity number rather than a quantity.
_ID_CUE = re.compile(
    r"(?:תעודת\s+זהות|מספר\s+זהות|ת״ז|ת\.ז\.?|ח״פ|ח\.פ\.?|ע״ר|ע״מ|עוסק\s+מורשה)[\s:,\-–]*$"
)

#: How far back to look for a cue. Long enough for "תעודת זהות", short enough to stay local.
_CUE_WINDOW = 24


def _spoken(raw: str) -> str:
    """Digit-by-digit reading of a phone group, separators dropped."""
    return digits(_NOT_DIGITS.sub("", raw))


def _has_cue(m: re.Match[str], cue: re.Pattern[str]) -> bool:
    before = m.string[max(0, m.start() - _CUE_WINDOW) : m.start()]
    return cue.search(before) is not None


class LocalPhoneRule:
    """``050-1234567`` / ``03-1234567`` / ``0501234567`` — a leading 0 plus an area code."""

    name = "phone-local"
    priority = PRIORITY.PHONE
    pattern = re.compile(
        rf"(?<!\d)0(?:"
        rf"(?:5\d|7\d){_SEP}?\d{{3}}{_SEP}?\d{{4}}"  # mobile / VoIP: 10 digits
        rf"|[23489]{_SEP}?\d{{3}}{_SEP}?\d{{4}}"  # landline: 9 digits
        rf")(?!\d)"
    )

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        return _spoken(m.group(0))


class IntlPhoneRule:
    """``+972-50-1234567``. The country code is read as פלוס plus its digits."""

    name = "phone-intl"
    priority = PRIORITY.PHONE
    pattern = re.compile(rf"\+972{_SEP}?0?(?:5\d|7\d|[23489]){_SEP}?\d{{3}}{_SEP}?\d{{4}}(?!\d)")

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        national = _NOT_DIGITS.sub("", m.group(0))[3:]
        # "+972-050-..." is a common mistake; the trunk 0 is not dialled, so it is not read.
        national = national.removeprefix("0")
        return f"פלוס {digits('972')} {digits(national)}"


class StarPhoneRule:
    """``*6555`` — the short commercial numbers, spoken as כוכבית then the digits."""

    name = "phone-star"
    priority = PRIORITY.PHONE
    pattern = re.compile(r"\*(\d{3,5})(?!\d)")

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        return f"כוכבית {digits(m.group(1))}"


class ServicePhoneRule:
    """``1-800-123-456`` and friends: the 1-800/1-700/1-900/1-599 service ranges."""

    name = "phone-service"
    priority = PRIORITY.PHONE
    pattern = re.compile(rf"(?<!\d)1{_SEP}?(?:800|700|900|599){_SEP}?\d{{3}}{_SEP}?\d{{3}}(?!\d)")

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        return _spoken(m.group(0))


class EmergencyPhoneRule:
    """``100``/``101``/``102`` — claimed only after a dialling cue, never as a quantity."""

    name = "phone-emergency"
    priority = PRIORITY.PHONE
    pattern = re.compile(r"(?<!\d)1[01]\d(?!\d)")

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        if not _has_cue(m, _DIAL_CUE):
            return None
        return _spoken(m.group(0))


class IdNumberRule:
    """Nine-digit national ID / company numbers, claimed only after an identity cue."""

    name = "phone-id"
    priority = PRIORITY.PHONE
    pattern = re.compile(r"(?<!\d)\d{9}(?!\d)")

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        if not _has_cue(m, _ID_CUE):
            return None
        return _spoken(m.group(0))


class CardNumberRule:
    """Credit-card shaped groups. The grouping itself is the evidence, so it is required."""

    name = "phone-card"
    priority = PRIORITY.PHONE
    pattern = re.compile(
        r"(?<!\d)(?:"
        r"\d{4}[-– ]\d{4}[-– ]\d{4}[-– ]\d{4}"  # Visa / Mastercard
        r"|\d{4}[-– ]\d{6}[-– ]\d{5}"  # American Express
        r")(?!\d)"
    )

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        return _spoken(m.group(0))


class IpAddressRule:
    """``192.168.1.1`` — digit groups with the dots spoken, unlike a phone separator."""

    name = "phone-ip"
    priority = PRIORITY.PHONE
    pattern = re.compile(r"(?<!\d)(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})(?!\d)")

    def render(self, m: re.Match[str], cfg: Config) -> str | None:
        octets = m.groups()
        # A dotted quad with an out-of-range part is a version string or worse; leave it.
        if any(int(o) > 255 for o in octets):
            return None
        joiner = f" {cfg.decimal_word} "
        return joiner.join(digits(o) for o in octets)


RULES = [
    IntlPhoneRule(),
    LocalPhoneRule(),
    ServicePhoneRule(),
    StarPhoneRule(),
    CardNumberRule(),
    IpAddressRule(),
    IdNumberRule(),
    EmergencyPhoneRule(),
]
