# heb-tts-normalizer

Hebrew text in, speakable Hebrew words out. Numbers, money, dates, times and units
become what a person would actually say — so your g2p never has to guess.

```
יש לי 3 ילדים, והכרטיס עלה ₪25 ב-12/03/2026 בשעה 14:30
יש לי שלושה ילדים, והכרטיס עלה עשרים וחמישה שקלים בשנים עשר במרץ אלפיים עשרים ושש בשעה שתיים וחצי אחר הצהריים
```

Output is plain Hebrew, no niqqud — this runs *before* a diacritizer or straight into a
g2p.

## Install

```sh
uv add heb-tts-normalizer
```

## Use

```python
from heb_tts_normalizer import normalize

normalize("3 ילדים")  # שלושה ילדים
normalize("3 ילדות")  # שלוש ילדות
normalize("המחיר 25.50 ₪")  # המחיר עשרים וחמישה שקלים וחמישים אגורות
```

## What it reads

| | | |
|---|---|---|
| Numbers, agreeing in gender | `3 ילדים` · `3 ילדות` | שלושה ילדים · שלוש ילדות |
| Currency | `₪25` | עשרים וחמישה שקלים |
| Percent | `12%` | שנים עשר אחוזים |
| Decimals | `3.5` | שלוש נקודה חמש |
| Dates | `12/03/2026` | שנים עשר במרץ אלפיים עשרים ושש |
| Hebrew-calendar dates | `כ״ז באלול` · `ט״ו בשבט` | כף זין באלול · טו בשבט |
| Times | `14:30` | שתיים וחצי אחר הצהריים |
| Units | `5 ק״מ` | חמישה קילומטרים |
| Ranges and signs | `3–5` · `-5` | שלוש עד חמש · מינוס חמש |
| Phones, digit by digit | `03-1234567` | אפס שלוש אחת שתיים… |
| Abbreviations | `ד״ר` · `מס׳` | דוקטור · מספר |

It also strips Markdown, zero-width characters and bidi controls, and collapses
whitespace — keeping the punctuation that shapes prosody.

Just as important is what it refuses to touch. URLs, emails, filenames and `GPT-4` come
back byte-identical, and a date that can't be a date (`32/13/2026`) is left alone rather
than guessed at.

## Reading style

```python
from heb_tts_normalizer import Config, Clock, normalize

normalize("14:30", Config(clock=Clock.H24))  # ארבע עשרה שלושים
```

| Option | Default | |
|---|---|---|
| `clock` | `H12` | `14:30` as שתיים וחצי or ארבע עשרה שלושים |
| `date_order` | `DMY` | how to read `03/04/2026` |
| `hebrew_date_style` | `LETTERS` | `כ״ז באלול` as כף זין or עשרים ושבעה |
| `read_minor_currency` | `True` | agorot and cents |
| `expand_units` | `True` | `ק״מ` → קילומטרים |
| `expand_abbreviations` | `True` | `ד״ר` → דוקטור |
| `strip_markdown` | `True` | drop syntax, keep the text |
| `gender_overrides` | `{}` | teach it a loanword no lexicon will carry |

## How it works

One left-to-right pass. At each position the highest-priority rule that matches wins, and
the span it claims is never looked at again — so the numbers rule cannot eat the digits
inside a date. A rule returns `None` to decline, which is how an impossible date falls
through instead of being read as arithmetic.

Each rule is a folder under `rules/` holding its pattern and its lexicon. Adding one is a
folder and a line in the registry — or, from outside the package, a class and a list.

Gender agreement is the part that is actually hard: `אחוז` is masculine, `שעה` is
feminine, and `נשים` is feminine despite the `־ים` ending, so the numeral has to follow
the noun rather than the suffix.

[`docs/DESIGN.md`](docs/DESIGN.md) covers the scanner, the priority table, the numeral
engine and the judgment calls. [`examples/`](examples) has three runnable ones.

## Develop

```sh
uv sync
uv run pytest
uv run ruff check . && uv run ty check
```

Cases live in TSVs under `tests/data/` — `input`, `expected`, and the config it applies
to. Every row runs through the full public `normalize()`, which is what catches one rule
stealing another's text.

## License

MIT
