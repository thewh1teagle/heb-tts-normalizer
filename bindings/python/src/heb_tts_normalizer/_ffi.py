"""Loading the shared library and declaring its ABI.

Everything ctypes-shaped lives here so ``__init__`` stays about the API.

Two rules earn their own note:

* Every function gets ``argtypes`` and ``restype``. Without ``restype`` ctypes assumes
  ``int``, which silently truncates a 64-bit pointer to 32 bits — the resulting crash
  looks like a bug in Rust and is not.
* ``heb_normalize`` returns ``c_void_p``, not ``c_char_p``. ctypes converts a
  ``c_char_p`` result to ``bytes`` and throws the pointer away, which would leak the
  Rust allocation on every call; we keep the address so we can hand it back to
  ``heb_string_free``.
"""

from __future__ import annotations

import ctypes
import os
import sys
from ctypes import c_char_p, c_void_p
from pathlib import Path

__all__ = ["LIB", "LIB_PATH", "LibraryNotFound", "load_library"]


class LibraryNotFound(OSError):
    """The native library could not be found or could not be loaded."""


def _candidate_names() -> list[str]:
    if sys.platform == "darwin":
        return ["libheb_tts_normalizer_c.dylib"]
    if sys.platform == "win32":
        return ["heb_tts_normalizer_c.dll", "libheb_tts_normalizer_c.dll"]
    return ["libheb_tts_normalizer_c.so"]


def _candidate_dirs() -> list[Path]:
    """Where to look, in order.

    Next to the package first: that is where the wheel bundles the library, and an
    installed package must never pick up a stale build from a checkout that happens to
    be on the path. Then the workspace ``target`` directories, so a developer can
    ``cargo build`` and import without installing anything; ``release`` is preferred
    over ``debug`` because a debug build of the regex engine is very slow, and a
    checkout that has both almost always means the release one is the intended artifact.
    """
    here = Path(__file__).resolve().parent
    dirs = [here, here / "lib"]

    # bindings/python/src/heb_tts_normalizer -> ... -> repo root. Stop at the first
    # ancestor that looks like the workspace.
    for parent in here.parents:
        target = parent / "target"
        if (parent / "Cargo.toml").is_file() and target.is_dir():
            dirs += [target / "release", target / "debug"]
            break

    return dirs


def _find_library() -> str | None:
    override = os.environ.get("HEB_TTS_NORMALIZER_LIB")
    if override:
        # An explicit path is a promise, not a hint: if it is wrong, say so rather than
        # quietly loading some other copy.
        return override

    names = _candidate_names()
    for directory in _candidate_dirs():
        for name in names:
            candidate = directory / name
            if candidate.is_file():
                return str(candidate)

    # Last resort: the platform loader's own search path (LD_LIBRARY_PATH, /usr/lib).
    from ctypes.util import find_library

    return find_library("heb_tts_normalizer_c")


def load_library() -> tuple[ctypes.CDLL, str]:
    """Locate, load and declare the native library. Raises `LibraryNotFound`."""
    path = _find_library()
    if path is None:
        searched = "\n  ".join(str(d) for d in _candidate_dirs())
        raise LibraryNotFound(
            "could not find the heb-tts-normalizer native library "
            f"({' / '.join(_candidate_names())}). Looked in:\n  {searched}\n"
            "Build it with `cargo build --release -p heb-tts-normalizer-c`, or set "
            "HEB_TTS_NORMALIZER_LIB to the library's path."
        )

    try:
        lib = ctypes.CDLL(path)
    except OSError as exc:
        raise LibraryNotFound(f"failed to load {path}: {exc}") from exc

    _declare(lib)
    return lib, path


def _declare(lib: ctypes.CDLL) -> None:
    lib.heb_new.argtypes = [c_char_p]
    lib.heb_new.restype = c_void_p

    lib.heb_normalizer_free.argtypes = [c_void_p]
    lib.heb_normalizer_free.restype = None

    # c_void_p, so the returned pointer survives for heb_string_free.
    lib.heb_normalize.argtypes = [c_void_p, c_char_p]
    lib.heb_normalize.restype = c_void_p

    lib.heb_string_free.argtypes = [c_void_p]
    lib.heb_string_free.restype = None

    # Static, library-owned: c_char_p is right here, nothing to free.
    lib.heb_last_error.argtypes = []
    lib.heb_last_error.restype = c_char_p

    lib.heb_version.argtypes = []
    lib.heb_version.restype = c_char_p


LIB, LIB_PATH = load_library()
