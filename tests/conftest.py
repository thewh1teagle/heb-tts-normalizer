"""Corpus loading for the TSV test data.

Every ``tests/data/*.tsv`` is discovered at collection time — no hardcoded list, so a
rule author dropping in a new topic file gets it tested without touching the harness.

A row carries its own provenance (file + line number) because the only thing worse than
a failing Hebrew assertion is a failing Hebrew assertion you cannot locate.
"""

from __future__ import annotations

import enum
from dataclasses import dataclass, fields
from pathlib import Path
from typing import Any

import pytest

from heb_tts_normalizer import Config

DATA_DIR = Path(__file__).parent / "data"

HEADER = ("input", "expected", "config", "note")

#: Fields a TSV ``config`` column may set, mapped to their declared type.
_CONFIG_FIELDS = {f.name: f.type for f in fields(Config)}

_TRUE = {"true", "yes", "on", "1"}
_FALSE = {"false", "no", "off", "0"}


class CorpusError(Exception):
    """A TSV is malformed. Always names the file and line, never just 'bad row'."""


@dataclass(frozen=True, slots=True)
class Row:
    """One corpus case, with enough provenance to find it again."""

    source: Path
    lineno: int
    input: str
    expected: str
    config: Config
    note: str

    @property
    def id(self) -> str:
        """Pytest test id, e.g. ``dates.tsv:42``."""
        return f"{self.source.name}:{self.lineno}"

    def describe(self, actual: str) -> str:
        """A failure message a human can actually read.

        Hebrew is RTL and niqqud is invisible at a glance, so the three strings go on
        their own lines with ASCII labels, repr'd so trailing spaces and stray
        control characters show up.
        """
        lines = [
            f"{self.id}: normalize() output does not match the corpus.",
            f"  file     {self.source}:{self.lineno}",
        ]
        if self.note:
            lines.append(f"  note     {self.note}")
        if self.config != Config():
            lines.append(f"  config   {_config_summary(self.config)}")
        lines += [
            f"  input    {self.input}",
            f"  expected {self.expected}",
            f"  actual   {actual}",
            "",
            f"  expected(repr) {self.expected!r}",
            f"  actual  (repr) {actual!r}",
        ]
        if self.expected != actual:
            lines.append(f"  first difference at index {_first_diff(self.expected, actual)}")
        return "\n".join(lines)


def _first_diff(a: str, b: str) -> int:
    for i, (x, y) in enumerate(zip(a, b, strict=False)):
        if x != y:
            return i
    return min(len(a), len(b))


def _config_summary(cfg: Config) -> str:
    default = Config()
    changed = [
        f"{name}={getattr(cfg, name)!r}"
        for name in _CONFIG_FIELDS
        if getattr(cfg, name) != getattr(default, name)
    ]
    return ";".join(changed) or "(defaults)"


def _coerce(name: str, raw: str, where: str) -> Any:
    """Turn a TSV string into the type the Config field actually wants."""
    declared = _CONFIG_FIELDS[name]
    # Config uses `from __future__ import annotations`, so field types are strings.
    kind = declared if isinstance(declared, str) else getattr(declared, "__name__", str(declared))
    current = getattr(Config(), name)

    if isinstance(current, enum.Enum):
        enum_cls = type(current)
        try:
            return enum_cls(raw)
        except ValueError:
            allowed = ", ".join(sorted(m.value for m in enum_cls))
            raise CorpusError(f"{where}: {name}={raw!r} is not one of: {allowed}") from None

    if isinstance(current, bool) or kind == "bool":
        lowered = raw.strip().lower()
        if lowered in _TRUE:
            return True
        if lowered in _FALSE:
            return False
        raise CorpusError(f"{where}: {name}={raw!r} is not a boolean (true/false)")

    if isinstance(current, int) and not isinstance(current, bool):
        try:
            return int(raw)
        except ValueError:
            raise CorpusError(f"{where}: {name}={raw!r} is not an integer") from None

    if isinstance(current, str):
        return raw

    raise CorpusError(f"{where}: config key {name!r} has unsupported type {kind!r} for a TSV")


def parse_config(spec: str, where: str = "<inline>") -> Config:
    """``clock=12;expand_units=false`` -> a real Config.

    An unknown key is an error, not a shrug: a typo like ``clok=12`` would otherwise
    silently test the default config and pass for the wrong reason.
    """
    spec = spec.strip()
    if not spec:
        return Config()

    overrides: dict[str, Any] = {}
    for part in spec.split(";"):
        part = part.strip()
        if not part:
            continue
        if "=" not in part:
            raise CorpusError(f"{where}: config entry {part!r} is not key=value")
        key, _, raw = part.partition("=")
        key, raw = key.strip(), raw.strip()
        if key not in _CONFIG_FIELDS:
            known = ", ".join(sorted(_CONFIG_FIELDS))
            raise CorpusError(f"{where}: unknown config key {key!r}. Known keys: {known}")
        if key in overrides:
            raise CorpusError(f"{where}: config key {key!r} given twice")
        overrides[key] = _coerce(key, raw, where)

    return Config(**overrides)


def load_tsv(path: Path) -> list[Row]:
    """Parse one TSV. Blank lines and ``#`` comments are skipped."""
    rows: list[Row] = []
    header_seen = False

    for lineno, line in enumerate(path.read_text(encoding="utf-8").split("\n"), start=1):
        line = line.rstrip("\r")
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue

        cells = line.split("\t")
        where = f"{path.name}:{lineno}"

        if not header_seen:
            got = tuple(c.strip() for c in cells[: len(HEADER)])
            if got != HEADER:
                raise CorpusError(
                    f"{where}: first non-comment line must be the header {HEADER!r}, got {got!r}"
                )
            header_seen = True
            continue

        if len(cells) < 2:
            raise CorpusError(
                f"{where}: expected tab-separated input/expected/config/note, "
                f"got {len(cells)} column(s). Is the separator a real tab?"
            )
        cells += [""] * (len(HEADER) - len(cells))
        if len(cells) > len(HEADER):
            raise CorpusError(f"{where}: {len(cells)} columns, expected at most {len(HEADER)}")

        text_in, expected, config_spec, note = (c.strip() for c in cells)
        if not text_in:
            raise CorpusError(f"{where}: empty input column")

        rows.append(
            Row(
                source=path,
                lineno=lineno,
                input=text_in,
                expected=expected,
                config=parse_config(config_spec, where),
                note=note,
            )
        )

    if not header_seen:
        raise CorpusError(f"{path.name}: no header line found")
    return rows


def corpus_files() -> list[Path]:
    """Every TSV under tests/data, sorted so ids are stable across machines."""
    return sorted(DATA_DIR.glob("*.tsv"))


def load_corpus() -> list[Row]:
    """All rows from all discovered TSVs."""
    rows: list[Row] = []
    for path in corpus_files():
        rows.extend(load_tsv(path))
    return rows


def pytest_generate_tests(metafunc: pytest.Metafunc) -> None:
    """Fan a test with a ``row`` parameter out over the whole corpus."""
    if "row" not in metafunc.fixturenames:
        return
    rows = load_corpus()
    metafunc.parametrize("row", rows, ids=[r.id for r in rows])
