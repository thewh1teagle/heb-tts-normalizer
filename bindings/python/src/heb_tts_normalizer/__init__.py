"""Normalize Hebrew text for TTS, so a g2p only ever sees words.

    >>> from heb_tts_normalizer import normalize
    >>> normalize("יש לי 3 ילדים ו-25₪")
    'יש לי שלושה ילדים ועשרים וחמישה שקלים'

The rules live in Rust now; this package is a thin ctypes binding over the C ABI. The
call signature and the `Config` fields are unchanged from the pure-Python release, so
existing callers do not have to notice the swap.
"""

from __future__ import annotations

import json
import threading
from ctypes import c_char_p, c_void_p, cast
from dataclasses import asdict, dataclass, field
from enum import Enum
from typing import Dict, Optional, Union

from ._ffi import LIB, LIB_PATH, LibraryNotFound

__all__ = [
    "Clock",
    "Config",
    "DateOrder",
    "Gender",
    "HebrewDateStyle",
    "LibraryNotFound",
    "NormalizerError",
    "normalize",
    "version",
]


class NormalizerError(RuntimeError):
    """The native library refused a call. Carries its `heb_last_error` message."""


# `str, Enum` rather than `StrEnum`: this binding supports Python 3.9, and StrEnum
# arrived in 3.11. Comparison against a plain string still works, which is what
# callers of the old package relied on.
class Clock(str, Enum):
    """How to read a time of day."""

    H12 = "12"  # 14:30 -> שתיים וחצי אחר הצהריים
    H24 = "24"  # 14:30 -> ארבע עשרה שלושים


class DateOrder(str, Enum):
    """How to disambiguate 03/04/2026."""

    DMY = "dmy"  # Israeli convention
    MDY = "mdy"


class HebrewDateStyle(str, Enum):
    """How to read the gematria in a Hebrew-calendar date."""

    LETTERS = "letters"  # כ״ז באלול -> כ׳ ז׳ באלול, left for the g2p to voice
    NUMBERS = "numbers"  # כ״ז באלול -> עשרים ושבעה באלול


class Gender(str, Enum):
    """Grammatical gender of a counted noun."""

    MASC = "m"
    FEM = "f"


@dataclass(frozen=True)
class Config:
    """Reading style. The defaults are the ones an Israeli speaker would use."""

    clock: Union[Clock, str] = Clock.H12
    date_order: Union[DateOrder, str] = DateOrder.DMY
    #: Letter names are how a Hebrew date is normally said. Dates whose name is fixed
    #: (תשעה באב, ט״ו בשבט) ignore this — they are names, not readings.
    hebrew_date_style: Union[HebrewDateStyle, str] = HebrewDateStyle.LETTERS

    #: Currency assumed for a bare number that is clearly money but carries no symbol.
    default_currency: str = "ILS"
    #: Read agorot/cents after the major unit ("25.50 ₪" -> "... וחמישים אגורות").
    read_minor_currency: bool = True

    #: Word between the integer and fractional part of a decimal.
    decimal_word: str = "נקודה"

    #: Spell out unit abbreviations ("ק״מ" -> "קילומטרים"). Off leaves them untouched.
    expand_units: bool = True
    #: Expand common abbreviations ("ד״ר" -> "דוקטור").
    expand_abbreviations: bool = True

    #: Drop Markdown syntax, keeping the text it wraps.
    strip_markdown: bool = True
    #: Collapse runs of whitespace and remove zero-width / bidi control characters.
    clean_whitespace: bool = True

    #: Extra noun genders, consulted before the built-in lexicon and the suffix
    #: heuristics. Lets a caller fix a word we get wrong without a release.
    gender_overrides: Dict[str, Union[Gender, str]] = field(default_factory=dict)

    def to_json(self) -> str:
        """The JSON the C API takes. Field names match the Rust `Config` exactly."""
        return json.dumps(_plain(asdict(self)), ensure_ascii=False, sort_keys=True)


def _plain(value: object) -> object:
    """Enums down to their values, so `json` sees only primitives."""
    if isinstance(value, Enum):
        return value.value
    if isinstance(value, dict):
        return {_plain(k): _plain(v) for k, v in value.items()}
    if isinstance(value, (list, tuple)):
        return [_plain(v) for v in value]
    return value


_DEFAULT = Config()

# One native handle per distinct config, kept for the life of the process: parsing the
# config JSON costs more than normalizing a short string, so paying it per call would
# dominate. Handles are immutable after creation, so sharing one across threads is
# safe; the lock only guards the dict.
_handles: Dict[str, int] = {}
_handles_lock = threading.Lock()


def _last_error(fallback: str) -> str:
    msg = LIB.heb_last_error()
    if not msg:
        return fallback
    return msg.decode("utf-8", "replace") or fallback


def _handle_for(cfg: Config) -> int:
    key = cfg.to_json()
    handle = _handles.get(key)
    if handle is not None:
        return handle

    with _handles_lock:
        handle = _handles.get(key)
        if handle is not None:
            return handle
        created = LIB.heb_new(key.encode("utf-8"))
        if not created:
            raise NormalizerError(_last_error("heb_new failed"))
        _handles[key] = created
        return created


def normalize(text: str, cfg: Optional[Config] = None) -> str:
    """Turn Hebrew text into something a g2p can read aloud.

    >>> normalize("יש לי 3 ילדים ו-25₪")
    'יש לי שלושה ילדים ועשרים וחמישה שקלים'

    Pass `cfg` to change the reading style; see `Config`.
    """
    if not isinstance(text, str):
        raise TypeError(f"text must be str, not {type(text).__name__}")

    handle = _handle_for(_DEFAULT if cfg is None else cfg)
    out = LIB.heb_normalize(handle, text.encode("utf-8"))
    if not out:
        raise NormalizerError(_last_error("heb_normalize failed"))
    try:
        # Copy into a Python str while the Rust allocation is still alive.
        return cast(c_void_p(out), c_char_p).value.decode("utf-8")  # type: ignore[union-attr]
    finally:
        LIB.heb_string_free(c_void_p(out))


def version() -> str:
    """The version of the native library actually loaded."""
    return LIB.heb_version().decode("utf-8")


#: Path of the shared library this process loaded. Useful when a dev checkout and an
#: installed wheel disagree about which build is in play.
library_path = LIB_PATH

__version__ = "0.2.0"
