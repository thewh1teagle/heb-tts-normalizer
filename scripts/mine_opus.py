#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Mine OPUS Hebrew corpora for real sentences the soak test can run over.

The hand-written corpus under `tests/data/*.tsv` was written by whoever wrote the
rules, so it can only ever contain cases somebody thought of. This script goes to
text nobody wrote for us: it downloads the Hebrew side of a few OPUS corpora, keeps
the sentences that carry something this library is supposed to read (a digit, a
currency sign, a gershayim numeral, a unit, a month name), and vendors a small,
deduplicated, stratified sample under `tests/data/soak/`.

Three things matter here and each is a deliberate choice:

* **Stratify before sampling.** Uniform sampling of Hebrew text is roughly 80% plain
  integers. A per-mille sign, a Hebrew-calendar date or a range would never survive
  it. Every sentence is filed under the *rarest* trigger it carries, and the sample is
  drawn round-robin across those categories, so a rare category gets its floor before
  the common ones get their fill.
* **Deduplicate on a skeleton, not on the text.** OpenSubtitles repeats lines heavily,
  and near-duplicates ("יש לי 3 ילדים" / "יש לי 4 ילדים") are worth no more than one
  row. Digits collapse to `#` and punctuation is dropped before the dedup key.
* **Nothing large is committed.** The downloads live in the gitignored `build/opus/`;
  only the sample and a manifest recording each source's hash and byte range land in
  git, so `--verify` can prove the sample came from the bytes it claims.

    uv run scripts/mine_opus.py             # fetch, mine, write tests/data/soak/
    uv run scripts/mine_opus.py --verify    # re-check the manifest against the cache
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import random
import re
import sys
import unicodedata
import urllib.request
from collections import Counter, defaultdict
from datetime import date
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_CACHE = ROOT / "build" / "opus"
DEFAULT_OUT = ROOT / "crates" / "heb-tts-normalizer" / "tests" / "data" / "soak"

OPUS_BASE = "https://object.pouta.csc.fi"

#: The corpora, and why each one is here.
#:
#: TED2020 is the obvious omission. It is a good source — percentages, statistics,
#: spoken large numbers — but it is CC BY-NC-ND, and this sample is committed inside a
#: crate that publishes to crates.io under MIT. Shipping non-commercial data inside an
#: MIT crate would quietly attach a condition nobody reading the licence expects, so it
#: stays out. QED covers most of the same lecture register under CC BY-SA.
#:
#: `cap_bytes` truncates the download. OpenSubtitles is 1.7 GB of gzip and we want a
#: few thousand sentences; a gzip stream decompresses fine up to the truncation point,
#: so a prefix is enough and the manifest records exactly which prefix.
CORPORA = {
    "opensubtitles": {
        "opus": "OpenSubtitles",
        "version": "v2024",
        "title": "OpenSubtitles",
        "licence": "CC0 / public-domain dedication by the OPUS release; subtitles from opensubtitles.org",
        "cite": "P. Lison and J. Tiedemann, OpenSubtitles2016 (LREC 2016)",
        "why": "spoken register: prices, counts, casual number use",
        # OPUS's sentence splitter treats a colon as a boundary, so this corpus has no
        # `HH:MM` in it at all — every time got cut in half. Times come from the other
        # corpora; what is left here is the informal number use nothing else has.
        "caveat": "no times: OPUS split every HH:MM at the colon",
        "cap_bytes": 96 * 1024 * 1024,
        "target": 2000,
    },
    "wikimedia": {
        "opus": "wikimedia",
        "version": "v20230407",
        "title": "wikimedia (Wikipedia/Wikinews translations)",
        "licence": "CC BY-SA 3.0",
        "cite": "J. Tiedemann, Parallel Data, Tools and Interfaces in OPUS (LREC 2012)",
        "why": "dates in both calendars, large numbers, units, gershayim acronyms",
        "cap_bytes": None,
        "target": 1400,
    },
    "qed": {
        "opus": "QED",
        "version": "v2.0a",
        "title": "QED (educational subtitles)",
        "licence": "CC BY-SA 3.0",
        "cite": "A. Abdelali et al., The AMARA Corpus (LREC 2014)",
        "why": "lecture register: measurements, percentages, arithmetic read aloud",
        "cap_bytes": None,
        "target": 800,
    },
    "tatoeba": {
        "opus": "Tatoeba",
        "version": "v2023-04-12",
        "title": "Tatoeba",
        "licence": "CC BY 2.0 FR",
        "cite": "Tatoeba Project, https://tatoeba.org",
        "why": "short, clean, well-formed sentences — gender agreement on ordinary nouns",
        "cap_bytes": None,
        "target": 400,
    },
    "globalvoices": {
        "opus": "GlobalVoices",
        "version": "v2018q4",
        "title": "GlobalVoices",
        "licence": "CC BY 3.0",
        "cite": "Global Voices, https://globalvoices.org",
        "why": "news register: dates, statistics, percentages",
        "cap_bytes": None,
        "target": 200,
    },
}

# ---------------------------------------------------------------------------
# triggers
# ---------------------------------------------------------------------------

HEB = r"֐-׿"
GERESH = r"'׳"
GERSHAYIM = r"\"״"

HEB_MONTHS = (
    "תשרי|חשוון|חשון|כסלו|טבת|שבט|אדר|אדר א|אדר ב|ניסן|אייר|סיוון|סיון|תמוז|אב|אלול"
)
GREG_MONTHS = (
    "ינואר|פברואר|מרץ|מרס|אפריל|מאי|יוני|יולי|אוגוסט|ספטמבר|אוקטובר|נובמבר|דצמבר"
)
UNITS = (
    "מ״מ|ס״מ|ק״מ|מ״ג|ק״ג|מ״ל|מ״ר|ס״ר|ק״ר|מ״ק|ק״ב|מ״ב|ג״ב|ט״ב|קמ״ש"
    "|מילימטר|סנטימטר|קילומטר|מיליגרם|קילוגרם|מיליליטר|ליטר|גרם|טון|קילו|קוב|דונם"
    "|מעלות|מעלה|שניות|שנייה|דקות|דקה|שעות|שעה|ימים|יום|שבועות|שבוע|חודשים|חודש"
    "|שנים|שנה|מטרים|מטר|קילובייט|מגהבייט|גיגהבייט|טרהבייט"
)
ABBREVS = (
    "ד״ר|פרופ׳|מס׳|עו״ד|רו״ח|ח״כ|ת״ד|רח׳|כת׳|וכו׳|וגו׳|בע״מ|צה״ל|ארה״ב|ע״י|ע״פ"
)
CURRENCY_WORDS = "ש״ח|שקל|שקלים|אגורות|אגורה|דולר|דולרים|יורו|אירו|פאונד|סנט"

#: Trigger categories, ordered by how much a sample of one is worth. A sentence is
#: filed under the *first* one it matches, so a line carrying both a per-mille sign and
#: a plain integer counts as per-mille — which is the whole point of stratifying.
#: Everything a digit can trigger comes before the two Hebrew-letter categories, since
#: a gershayim token appears in a large fraction of all Hebrew sentences and would
#: otherwise swallow every time and price on its way past.
CATEGORIES: list[tuple[str, re.Pattern[str]]] = [
    ("permille", re.compile("‰")),
    ("degree", re.compile(r"\d\s*°|°[CF]")),
    (
        "hebrew_date",
        re.compile(rf"[{HEB}]{{1,2}}[{GERSHAYIM}][{HEB}]\s*ב?({HEB_MONTHS})\b"),
    ),
    ("phone", re.compile(r"(?<!\d)0\d{1,2}[- ]?\d{7}(?!\d)|\*\d{3,4}(?!\d)")),
    ("time", re.compile(r"(?<!\d)\d{1,2}:\d{2}(?!\d)")),
    ("date_numeric", re.compile(r"(?<!\d)\d{1,2}[./-]\d{1,2}([./-]\d{2,4})?(?!\d)")),
    ("month_name", re.compile(rf"\d[^\n]{{0,12}}\b({GREG_MONTHS})\b|\b({GREG_MONTHS})\b[^\n]{{0,12}}\d")),
    (
        "currency",
        re.compile(rf"[₪$€£]|\d[\s\-]*({CURRENCY_WORDS})|({CURRENCY_WORDS})[\s\-]*\d"),
    ),
    ("percent", re.compile(r"\d\s*%|%\s*\d")),
    ("unit", re.compile(rf"\d\s*({UNITS})(?![{HEB}])")),
    ("range", re.compile(r"(?<!\d)\d+\s*[-–—]\s*\d+(?!\d)|\d+\s+עד\s+\d+")),
    ("large_number", re.compile(r"\d{1,3}(,\d{3})+")),
    ("fraction", re.compile(r"(?<!\d)\d{1,3}/\d{1,3}(?!\d)(?!\s*/)")),
    ("decimal", re.compile(r"(?<!\d)\d+\.\d+(?!\d)")),
    # A minus sign, not a subtitle speaker dash: something must precede it.
    ("signed", re.compile(r"(?<=[\s(\[])[-−]\d")),
    ("abbrev", re.compile(rf"(?<![{HEB}])({ABBREVS})(?![{HEB}])")),
    # Any other gershayim or geresh token: a gematria numeral or an acronym the
    # abbreviation lexicon does not know. Common enough that it sits at the bottom,
    # above only the bare integer.
    (
        "gershayim",
        re.compile(rf"(?<![{HEB}])[{HEB}]{{1,3}}[{GERSHAYIM}][{HEB}](?![{HEB}])"),
    ),
    ("integer", re.compile(r"\d")),
]

HEB_LETTER = re.compile(rf"[{HEB}]")
LATIN = re.compile(r"[A-Za-z]")
#: Subtitle furniture, markup leftovers and lines that are not Hebrew prose.
REJECT = re.compile(r"[♪♫<>{}\[\]|]|https?://|www\.|@[A-Za-z]|\.(?:com|net|org|co\.il)\b")
TRUNCATED = re.compile(r"\s[בלמכשהו]$")
LEADING_DASH = re.compile(r"^[-–—]\s*")
DIGIT_RUN = re.compile(r"\d+")
NON_WORD = re.compile(rf"[^{HEB}A-Za-z#]+")

MIN_CHARS = 14
MAX_CHARS = 180
MIN_HEB_LETTERS = 6


def die(msg: str) -> None:
    sys.exit(f"mine_opus: {msg}")


# ---------------------------------------------------------------------------
# fetch
# ---------------------------------------------------------------------------


def url_for(spec: dict) -> str:
    return f"{OPUS_BASE}/OPUS-{spec['opus']}/{spec['version']}/mono/he.txt.gz"


def fetch(key: str, spec: dict, cache: Path, force: bool) -> Path:
    """Download the Hebrew side, truncated to `cap_bytes` if it is set."""
    cache.mkdir(parents=True, exist_ok=True)
    path = cache / f"{key}.he.txt.gz"
    if path.exists() and not force:
        return path

    url = url_for(spec)
    cap = spec["cap_bytes"]
    req = urllib.request.Request(url)
    if cap is not None:
        req.add_header("Range", f"bytes=0-{cap - 1}")
    print(f"  fetching {url}" + (f" (first {cap // 1024 // 1024} MiB)" if cap else ""))
    tmp = path.with_suffix(".part")
    with urllib.request.urlopen(req) as resp, tmp.open("wb") as out:
        written = 0
        while chunk := resp.read(1 << 20):
            out.write(chunk)
            written += len(chunk)
            if cap is not None and written >= cap:
                break
    tmp.replace(path)
    return path


def digest(path: Path) -> tuple[str, int]:
    h = hashlib.sha256()
    size = 0
    with path.open("rb") as f:
        while chunk := f.read(1 << 20):
            h.update(chunk)
            size += len(chunk)
    return h.hexdigest(), size


def read_lines(path: Path):
    """Every line of a possibly-truncated gzip stream.

    A prefix of a gzip file decompresses cleanly right up to the cut, and then raises.
    That is expected, not a failure, so the tail error is swallowed — but only after
    some lines came out, otherwise the file is genuinely broken.
    """
    seen = 0
    with gzip.open(path, "rt", encoding="utf-8", errors="replace") as f:
        try:
            for line in f:
                seen += 1
                yield line
        except (EOFError, OSError, gzip.BadGzipFile) as e:
            if seen == 0:
                die(f"{path.name}: not readable as gzip: {e}")


# ---------------------------------------------------------------------------
# filtering
# ---------------------------------------------------------------------------


def tidy(line: str) -> str:
    """One line of source text, whitespace-normalized and stripped of its speaker dash.

    Nothing else is touched. Niqqud, bidi controls and odd quotation marks stay: they
    are part of what real text throws at the normalizer, and removing them here would
    hide exactly the inputs worth soaking over. The line-leading hyphen goes because it
    is the subtitle convention for "a different speaker talks now" — never prose and
    never a minus sign, and left in it would be most of the `signed` bucket.
    """
    line = line.replace("\u00a0", " ")
    line = "".join(
        c for c in line if c == " " or unicodedata.category(c) not in ("Cc", "Zl", "Zp")
    )
    return LEADING_DASH.sub("", " ".join(line.split()))


def acceptable(text: str) -> bool:
    if not (MIN_CHARS <= len(text) <= MAX_CHARS):
        return False
    if REJECT.search(text):
        return False
    # OPUS splits sentences at a colon, which cuts subtitles mid-clause and leaves a
    # dangling one-letter preposition at the end. The fragment before it is fine; the
    # truncation marker is not text anyone would ever be asked to read.
    if TRUNCATED.search(text):
        return False
    heb = len(HEB_LETTER.findall(text))
    if heb < MIN_HEB_LETTERS:
        return False
    # Mostly-Latin lines slip through the Hebrew-side files; they are somebody else's
    # normalizer's problem.
    return len(LATIN.findall(text)) <= heb


def categorize(text: str) -> str | None:
    for name, pattern in CATEGORIES:
        if pattern.search(text):
            return name
    return None


def skeleton(text: str) -> str:
    """Dedup key: the sentence with every number collapsed and punctuation dropped."""
    return NON_WORD.sub("", DIGIT_RUN.sub("#", text)).lower()


# ---------------------------------------------------------------------------
# mining
# ---------------------------------------------------------------------------


def mine(path: Path, seen_skeletons: set[str]) -> dict[str, list[str]]:
    """Every accepted sentence from one corpus, filed under its rarest trigger."""
    buckets: dict[str, list[str]] = defaultdict(list)
    for raw in read_lines(path):
        text = tidy(raw)
        if not acceptable(text):
            continue
        category = categorize(text)
        if category is None:
            continue
        key = skeleton(text)
        if key in seen_skeletons:
            continue
        seen_skeletons.add(key)
        buckets[category].append(text)
    return buckets


def sample(buckets: dict[str, list[str]], target: int, rng: random.Random) -> list[tuple[str, str]]:
    """Round-robin across categories, so the rare ones are not drowned by integers."""
    pools = {}
    for category, lines in buckets.items():
        shuffled = sorted(set(lines))
        rng.shuffle(shuffled)
        pools[category] = shuffled

    picked: list[tuple[str, str]] = []
    order = sorted(pools)
    while len(picked) < target and any(pools[c] for c in order):
        for category in order:
            if not pools[category]:
                continue
            picked.append((category, pools[category].pop()))
            if len(picked) >= target:
                break
    picked.sort(key=lambda p: (p[0], p[1]))
    return picked


def write_sample(out: Path, key: str, spec: dict, picked: list[tuple[str, str]]) -> None:
    counts = Counter(c for c, _ in picked)
    lines = [
        f"# {spec['title']} — soak sample for heb-tts-normalizer",
        "#",
        f"# source:  {url_for(spec)}",
        f"# licence: {spec['licence']}",
        f"# cite:    {spec['cite']}",
        f"# why:     {spec['why']}",
        *([f"# caveat:  {spec['caveat']}"] if "caveat" in spec else []),
        "#",
        "# Real sentences, unreviewed: nothing here says what the correct reading is,",
        "# only that normalize() must not misbehave on it. Regenerate with",
        "# `chore mine`; see tests/data/soak/ATTRIBUTION.md.",
        "#",
        f"# {len(picked)} sentences across {len(counts)} trigger categories.",
    ]
    current = None
    for category, text in picked:
        if category != current:
            current = category
            lines += ["", f"# category: {category} ({counts[category]})"]
        lines.append(text)
    (out / f"{key}.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")


def write_attribution(out: Path, used: dict[str, dict]) -> None:
    body = [
        "# Soak-test corpora",
        "",
        "Real Hebrew sentences mined from [OPUS](https://opus.nlpl.eu/) by",
        "`scripts/mine_opus.py`. They carry no expected output — the soak test asserts",
        "only invariants that must hold whatever the correct reading is, so no row here",
        "was ever reviewed by a human, and none of it is a specification.",
        "",
        "Each file is a sampled, deduplicated subset of the Hebrew side of one corpus,",
        "under that corpus's own licence:",
        "",
        "| File | Corpus | Licence | Source |",
        "|---|---|---|---|",
    ]
    for key, spec in used.items():
        body.append(
            f"| `{key}.txt` | {spec['title']} | {spec['licence']} | [{spec['opus']} {spec['version']}]({url_for(spec)}) |"
        )
    body += [
        "",
        "Citations, as the corpora ask:",
        "",
    ]
    for spec in used.values():
        body.append(f"- {spec['title']} — {spec['cite']}")
    body += [
        "",
        "OPUS itself: J. Tiedemann, *Parallel Data, Tools and Interfaces in OPUS*,",
        "LREC 2012, <https://opus.nlpl.eu/>.",
        "",
        "`manifest.json` records the exact bytes each sample came from, so",
        "`uv run scripts/mine_opus.py --verify` can prove a sample matches its source.",
        "",
    ]
    (out / "ATTRIBUTION.md").write_text("\n".join(body), encoding="utf-8")


# ---------------------------------------------------------------------------
# entry point
# ---------------------------------------------------------------------------


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--corpora", default="all", help="comma-separated keys, or 'all'")
    ap.add_argument("--cache", type=Path, default=DEFAULT_CACHE)
    ap.add_argument("--out", type=Path, default=DEFAULT_OUT)
    ap.add_argument("--seed", type=int, default=20260909, help="sampling seed")
    ap.add_argument("--refetch", action="store_true", help="ignore the download cache")
    ap.add_argument("--verify", action="store_true", help="check the manifest, write nothing")
    args = ap.parse_args()

    keys = list(CORPORA) if args.corpora == "all" else args.corpora.split(",")
    for key in keys:
        if key not in CORPORA:
            die(f"unknown corpus {key!r}. Known: {', '.join(CORPORA)}")

    if args.verify:
        verify(keys, args)
        return

    args.out.mkdir(parents=True, exist_ok=True)
    manifest = {
        "generated": date.today().isoformat(),
        "seed": args.seed,
        "note": "Written by scripts/mine_opus.py. sha256 covers the fetched bytes, "
        "which for a capped corpus are a prefix of the remote file.",
        "corpora": {},
    }
    used = {}
    # One skeleton set across every corpus: OpenSubtitles and QED are both subtitles
    # and share plenty of lines, and a duplicate is no more interesting the second time.
    seen: set[str] = set()

    for key in keys:
        spec = CORPORA[key]
        print(f"{key}:")
        path = fetch(key, spec, args.cache, args.refetch)
        sha, size = digest(path)
        buckets = mine(path, seen)
        found = sum(len(v) for v in buckets.values())
        picked = sample(buckets, spec["target"], random.Random(f"{args.seed}:{key}"))
        write_sample(args.out, key, spec, picked)
        used[key] = spec
        manifest["corpora"][key] = {
            "opus": spec["opus"],
            "version": spec["version"],
            "url": url_for(spec),
            "licence": spec["licence"],
            "fetched_bytes": size,
            "capped": spec["cap_bytes"] is not None,
            "sha256": sha,
            "candidates": found,
            "sampled": len(picked),
            "categories": dict(sorted(Counter(c for c, _ in picked).items())),
        }
        print(f"  {found} candidates -> {len(picked)} sampled into {key}.txt")

    write_attribution(args.out, used)
    (args.out / "manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    total = sum(c["sampled"] for c in manifest["corpora"].values())
    print(f"{total} sentences written to {args.out.relative_to(ROOT)}")


def verify(keys: list[str], args) -> None:
    """Re-hash the cache against the manifest. Nothing is written."""
    path = args.out / "manifest.json"
    if not path.exists():
        die(f"no manifest at {path}")
    manifest = json.loads(path.read_text(encoding="utf-8"))
    bad = 0
    for key in keys:
        recorded = manifest["corpora"].get(key)
        if recorded is None:
            print(f"{key}: not in the manifest")
            bad += 1
            continue
        cached = args.cache / f"{key}.he.txt.gz"
        if not cached.exists():
            cached = fetch(key, CORPORA[key], args.cache, False)
        sha, size = digest(cached)
        if sha == recorded["sha256"] and size == recorded["fetched_bytes"]:
            print(f"{key}: ok ({size} bytes)")
        else:
            print(f"{key}: MISMATCH\n  manifest {recorded['sha256']} ({recorded['fetched_bytes']})\n  cache    {sha} ({size})")
            bad += 1
    if bad:
        die(f"{bad} corpus/corpora do not match the manifest")


if __name__ == "__main__":
    main()
