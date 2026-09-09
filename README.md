# heb-tts-normalizer

Hebrew text in, speakable Hebrew words out. Numbers, money, dates, times and units
become what a person would actually say — so your g2p never has to guess.

```
יש לי 3 ילדים, והכרטיס עלה ₪25 ב-12/03/2026 בשעה 14:30
יש לי שלושה ילדים, והכרטיס עלה עשרים וחמישה שקלים בשנים עשר במרץ אלפיים עשרים ושש בשעה שתיים וחצי אחר הצהריים
```

Output is plain Hebrew, no niqqud — this runs *before* a diacritizer or straight into a
g2p. One Rust core, with bindings for Python, TypeScript and C.

## Install

```sh
cargo add heb-tts-normalizer     # Rust
pip install heb-tts-normalizer   # Python, ctypes over the C ABI
npm install heb-tts-normalizer   # TypeScript, wasm — browser and Node
```

For C, take the tarball for your platform from the [releases](../../releases): it
carries the `cdylib`, the static library and `heb_tts_normalizer.h`.

## Use

```rust
use heb_tts_normalizer::{normalize, Config};
normalize("3 ילדים", &Config::default());   // שלושה ילדים
```

```python
from heb_tts_normalizer import normalize, Config, Clock
normalize("3 ילדות")                        # שלוש ילדות
normalize("14:30", Config(clock=Clock.H24)) # ארבע עשרה שלושים
```

```ts
import { init, normalize } from "heb-tts-normalizer";
await init();                  // browser only needs this; Node resolves immediately
normalize("המחיר 25.50 ₪");    // המחיר עשרים וחמישה שקלים וחמישים אגורות
```

## What it reads

| | | |
|---|---|---|
| Numbers, agreeing in gender | `3 ילדים` · `3 ילדות` | שלושה ילדים · שלוש ילדות |
| Currency | `₪25` | עשרים וחמישה שקלים |
| Percent | `12%` | שנים עשר אחוזים |
| Decimals | `3.5` | שלוש נקודה חמש |
| Dates | `12/03/2026` | שנים עשר במרץ אלפיים עשרים ושש |
| Hebrew-calendar dates | `כ״ז באלול` · `ט״ו בשבט` | כ׳ ז׳ באלול · טו בשבט |
| Times | `14:30` | שתיים וחצי אחר הצהריים |
| Units | `5 ק״מ` | חמישה קילומטרים |
| Ranges and signs | `3–5` · `-5` | שלוש עד חמש · מינוס חמש |
| Phones, digit by digit | `03-1234567` | אפס שלוש אחת שתיים… |
| Abbreviations | `ד״ר` · `מס׳` | דוקטור · מספר |

It also strips Markdown, zero-width characters and bidi controls, and collapses
whitespace — keeping the punctuation that shapes prosody.

Just as important is what it refuses to touch. URLs, emails, filenames and `GPT-4` come
back byte-identical, and a date that cannot be a date (`32/13/2026`) is left alone rather
than guessed at.

## Reading style

Every binding takes the same options; over the C ABI and wasm they cross as one JSON
object, so there is a single config representation for the whole project.

| Option | Default | |
|---|---|---|
| `clock` | `12` | `14:30` as שתיים וחצי or ארבע עשרה שלושים |
| `date_order` | `dmy` | how to read `03/04/2026` |
| `hebrew_date_style` | `letters` | `כ״ז באלול` as כ׳ ז׳ or עשרים ושבעה |
| `read_minor_currency` | `true` | agorot and cents |
| `expand_units` | `true` | `ק״מ` → קילומטרים |
| `expand_abbreviations` | `true` | `ד״ר` → דוקטור |
| `strip_markdown` | `true` | drop syntax, keep the text |
| `gender_overrides` | `{}` | teach it a loanword no lexicon will carry |

## How it works

One left-to-right pass. At each position the highest-priority rule that matches wins, and
the span it claims is never looked at again — so the numbers rule cannot eat the digits
inside a date. A rule returns `None` to decline, which is how an impossible date falls
through instead of being read as arithmetic.

Gender agreement is the part that is actually hard: `אחוז` is masculine, `שעה` is
feminine, and `נשים` is feminine despite the `־ים` ending, so the numeral has to follow
the noun rather than the suffix.

[`docs/DESIGN.md`](docs/DESIGN.md) covers the scanner, the priority table, the numeral
engine and the judgment calls. `crates/heb-tts-normalizer/examples/` has three runnable ones.

## Layout

```
crates/heb-tts-normalizer/        the library — scanner, numerals, rules, and the corpus
crates/heb-tts-normalizer-c/      C ABI: cdylib, staticlib, hand-written header
crates/heb-tts-normalizer-wasm/   wasm-bindgen
bindings/python/                  ctypes over the C ABI
bindings/typescript/              wasm, browser and Node
```

## Develop

```sh
chore check      # fmt, clippy -D warnings, and the full Rust test suite
chore examples   # run all three
chore py-test    # Python smoke tests against a locally built C library
chore ts-test    # TypeScript smoke tests against a locally built wasm module
```

`chore list` shows the rest. Every case lives in a TSV under
`crates/heb-tts-normalizer/tests/data/` — `input`, `expected`, and the config it applies
to. Each row runs through the public `normalize()`, which is what catches one rule
stealing another's text, and is also checked for idempotence and for digits or niqqud
surviving into the output.

## Release

CI is only a cross-compilation farm: a `v*` tag builds the C ABI for six targets plus the
wasm package and attaches them to the GitHub release. Bundling and publishing happen
locally, so no registry token ever reaches CI.

```sh
chore bundle 0.1.0   # fetch those artifacts, verify checksums, build wheels into dist/
chore publish        # upload to PyPI and npm — this ships for real
```

Because the Python binding is ctypes rather than a compiled extension, it has no Python
ABI dependency: that is six wheels, one per platform, instead of one per Python version
per platform.

## License

MIT
