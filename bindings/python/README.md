# heb-tts-normalizer (Python)

Normalize Hebrew text for TTS: numbers, currency, dates, times and units become spoken
words, ready for g2p. Output is plain Hebrew, never niqqud.

```python
from heb_tts_normalizer import normalize

normalize("יש לי 3 ילדים ו-25₪")
# 'יש לי שלושה ילדים ועשרים וחמישה שקלים'
```

The rules live in Rust. This package is a `ctypes` binding over the C ABI in
`crates/heb-tts-normalizer-c` — no runtime dependencies, and nothing to compile at
install time.

## Reading style

```python
from heb_tts_normalizer import Clock, Config, Gender, HebrewDateStyle, normalize

cfg = Config(
    clock=Clock.H24,
    hebrew_date_style=HebrewDateStyle.NUMBERS,
    gender_overrides={"וופל": Gender.MASC},
)
normalize("14:30", cfg)
```

`Config` mirrors the Rust struct field for field: `clock`, `date_order`,
`hebrew_date_style`, `default_currency`, `read_minor_currency`, `decimal_word`,
`expand_units`, `expand_abbreviations`, `strip_markdown`, `clean_whitespace`,
`gender_overrides`. Enum fields also accept their bare string value (`clock="24"`).
An unknown field value raises `NormalizerError` carrying the message Rust produced —
the config is validated on the Rust side, so the two can never drift.

Handles are cached per distinct config, so repeated calls with the same `Config` pay
the JSON parse once.

## Finding the native library

`heb_tts_normalizer._ffi` looks for `libheb_tts_normalizer_c.dylib` (macOS),
`libheb_tts_normalizer_c.so` (Linux) or `heb_tts_normalizer_c.dll` (Windows), in this
order:

1. `$HEB_TTS_NORMALIZER_LIB`, if set — a full path to the library. An explicit path is
   treated as a promise: if it is wrong you get an error, not a silent fallback.
2. Next to `__init__.py`, and in a `lib/` subdirectory beside it. This is where the
   wheel bundles it, so an installed package never picks up a stale build from a
   checkout that happens to be on the path.
3. The workspace `target/release` then `target/debug`, found by walking up from the
   package to the first ancestor holding both a `Cargo.toml` and a `target/`. Release
   is preferred: a debug build of the regex engine is slow enough to be misleading.
4. The platform loader's own search path (`ctypes.util.find_library`).

Failing all four raises `LibraryNotFound`, listing the directories it tried.
`heb_tts_normalizer.library_path` reports which file was actually loaded, and
`version()` reports the native library's version — worth checking when a checkout and
an installed wheel disagree.

## Developing

```sh
cargo build --release -p heb-tts-normalizer-c   # step 3 above finds this
cd bindings/python && uv run pytest
```

To build a wheel, copy the compiled library next to `src/heb_tts_normalizer/__init__.py`
first; `pyproject.toml` lists it under `tool.hatch.build.targets.wheel.artifacts` so it
is included when present. Wheels are per-platform and must be tagged accordingly.

## Differences from the pure-Python release

`normalize(text, cfg=None)` and `Config` are unchanged. The rule-authoring surface is
gone — `Rule`, `scan`, `PRIORITY` and `ALL_RULES` were Python-level extension points
and have no equivalent across the C ABI; write a rule in Rust instead. `Config` is a
plain `str`-valued `Enum` rather than `StrEnum`, because this package supports Python
3.9; comparisons against plain strings still work.
