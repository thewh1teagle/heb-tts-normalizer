#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["build>=1.2", "hatchling>=1.25", "wheel>=0.43"]
# ///
"""Turn the release's C-ABI tarballs into one Python wheel per platform.

Called by `chore bundle <version>`, which has already downloaded the assets. This
script verifies them against SHA256SUMS, then for each platform drops the native
library beside `bindings/python/src/heb_tts_normalizer/__init__.py` and builds a wheel
tagged for that platform.

Two things this deliberately does not do:

* **No sdist.** An sdist would need Rust and cargo at install time, so `pip install`
  on an unmatched platform would fall back to it and fail deep inside a cargo log.
  Wheels-only makes pip say "no matching distribution found", which is the truth.
* **No per-Python-version matrix.** The binding is ctypes, so it has no Python ABI
  dependency at all: one wheel per platform, tagged `py3-none-<platform>`. Six wheels,
  not thirty. If this ever starts emitting cp39/cp310/cp311 tags, something is wrong.
"""

from __future__ import annotations

import argparse
import hashlib
import shutil
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PY_PROJECT = ROOT / "bindings" / "python"
PKG_DIR = PY_PROJECT / "src" / "heb_tts_normalizer"

#: rustc target triple -> the wheel platform tag pip matches against.
#:
#: macOS gets two wheels rather than one universal2: they map 1:1 onto the CI
#: artifacts, universal2 only helps single-file app bundling, and it would double
#: every download for no benefit to pip.
TAGS = {
    "x86_64-unknown-linux-gnu": "manylinux_2_17_x86_64.manylinux2014_x86_64",
    "x86_64-unknown-linux-musl": "musllinux_1_2_x86_64",
    "aarch64-unknown-linux-gnu": "manylinux_2_17_aarch64.manylinux2014_aarch64",
    "x86_64-apple-darwin": "macosx_10_12_x86_64",
    "aarch64-apple-darwin": "macosx_11_0_arm64",
    "x86_64-pc-windows-msvc": "win_amd64",
}

LIB_SUFFIXES = (".so", ".dylib", ".dll")


def die(msg: str) -> None:
    sys.exit(f"build_wheels: {msg}")


def _is_library(name: str) -> bool:
    """The shared library, and nothing else the archive happens to carry.

    Matched by name as well as suffix: a macOS tar drops AppleDouble `._` siblings
    beside every file, and those end in `.dylib` too.
    """
    return (
        not name.startswith("._")
        and name.startswith(("libheb_tts_normalizer_c", "heb_tts_normalizer_c"))
        and name.endswith(LIB_SUFFIXES)
    )


def verify_sums(stage: Path) -> None:
    """Check every file listed in SHA256SUMS. A missing file is as bad as a wrong one."""
    sums = stage / "SHA256SUMS"
    if not sums.is_file():
        die(f"{sums} is missing — nothing can be trusted without it")

    checked = 0
    for lineno, line in enumerate(sums.read_text().splitlines(), start=1):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        try:
            digest, name = line.split(maxsplit=1)
        except ValueError:
            die(f"SHA256SUMS:{lineno}: cannot parse {line!r}")
        name = name.lstrip("*").strip()
        target = stage / name
        if not target.is_file():
            die(f"SHA256SUMS lists {name}, which was not downloaded")
        actual = hashlib.sha256(target.read_bytes()).hexdigest()
        if actual != digest:
            die(f"{name}: sha256 mismatch\n  expected {digest}\n  actual   {actual}")
        checked += 1

    if not checked:
        die("SHA256SUMS is empty")
    print(f"verified {checked} asset(s) against SHA256SUMS")


def extract_library(tarball: Path, into: Path) -> Path:
    """Pull the one shared library out of a C-ABI tarball."""
    with tarfile.open(tarball) as tf:
        members = [
            m
            for m in tf.getmembers()
            if m.isfile() and _is_library(Path(m.name).name)
        ]
        if len(members) != 1:
            names = [m.name for m in members] or ["<none>"]
            die(f"{tarball.name}: expected exactly one shared library, found {names}")
        member = members[0]
        # Flatten: the name inside the archive carries a directory we do not want.
        extracted = into / Path(member.name).name
        src = tf.extractfile(member)
        if src is None:
            die(f"{tarball.name}: cannot read {member.name}")
        extracted.write_bytes(src.read())
    return extracted


def build_wheel(platform_tag: str, out: Path) -> Path:
    """Build the Python package and retag the wheel for one platform."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        subprocess.run(
            [
                sys.executable,
                "-m",
                "build",
                "--wheel",
                "--no-isolation",
                "--outdir",
                str(tmp_path),
                str(PY_PROJECT),
            ],
            check=True,
        )
        built = list(tmp_path.glob("*.whl"))
        if len(built) != 1:
            die(f"expected one wheel, got {[p.name for p in built]}")

        # hatchling emits a pure `py3-none-any` wheel; the native library inside makes
        # that a lie, so stamp the real platform on it.
        subprocess.run(
            [
                sys.executable,
                "-m",
                "wheel",
                "tags",
                "--remove",
                "--python-tag",
                "py3",
                "--abi-tag",
                "none",
                "--platform-tag",
                platform_tag,
                str(built[0]),
            ],
            check=True,
        )
        retagged = list(tmp_path.glob("*.whl"))
        if len(retagged) != 1:
            die(f"expected one retagged wheel, got {[p.name for p in retagged]}")
        final = out / retagged[0].name
        shutil.move(str(retagged[0]), final)
        return final


def clean_libs() -> None:
    for suffix in LIB_SUFFIXES:
        for stale in PKG_DIR.glob(f"*{suffix}"):
            stale.unlink()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", required=True, help="release version, without the v")
    parser.add_argument("--stage", required=True, type=Path, help="downloaded assets")
    parser.add_argument("--out", required=True, type=Path, help="where wheels go")
    args = parser.parse_args()

    stage: Path = args.stage
    out: Path = args.out
    out.mkdir(parents=True, exist_ok=True)

    verify_sums(stage)

    # A stale library from a previous run would end up in every wheel.
    clean_libs()

    built: list[Path] = []
    try:
        for target, platform_tag in TAGS.items():
            tarball = stage / f"heb-tts-normalizer-c-{target}.tar.gz"
            if not tarball.is_file():
                die(f"missing asset {tarball.name} — was the release complete?")

            with tempfile.TemporaryDirectory() as tmp:
                lib = extract_library(tarball, Path(tmp))
                shutil.copy2(lib, PKG_DIR / lib.name)
            try:
                wheel = build_wheel(platform_tag, out)
            finally:
                clean_libs()

            if args.version not in wheel.name:
                die(f"{wheel.name} is not version {args.version} — check the tag")
            built.append(wheel)
            print(f"  {target:32} -> {wheel.name}")
    finally:
        clean_libs()

    if len(built) != len(TAGS):
        die(f"built {len(built)} wheels, expected {len(TAGS)}")

    print(f"\n{len(built)} wheels in {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
