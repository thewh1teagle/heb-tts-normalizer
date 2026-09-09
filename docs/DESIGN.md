# Design

## One pass, not a chain

The obvious implementation is a chain of `re.sub` passes: a dates pass, then currency,
then numbers. It breaks immediately. The numbers pass eats the digits inside a date the
dates pass already consumed, `₪25` gets normalized twice, and the order of the passes
becomes load-bearing in a way nobody can see from any single file.

Instead there is one left-to-right scan. At each position the scanner asks the rules, in
priority order, whether they match *here*. The first rule whose `render` returns a string
wins, its span is appended to the output, and the scan resumes after it. **A span is
matched once, rendered once, and never looked at again** — so no rule can see another
rule's output, and rewriting one rule cannot silently change what another one sees.

Ties within a priority go to the longest match.

## Declining

`render` returns `str | None`. Returning `None` means *this looked like mine but isn't*,
and the scanner falls through to the next candidate at the same position.

This exists because validity is often only knowable after parsing. The dates pattern
happily matches `32/13/2026`; only once the numbers are in hand can it be ruled out.
Ordinary regex alternation cannot express that.

Declining is also how config is honoured: with `expand_units=False` the units rule
declines, so the span survives untouched instead of being half-rewritten.

## Priorities

Higher wins. The scheme is: a rule that consumes more context sits above the rules whose
patterns it contains.

| | rules |
|---:|---|
| 100 `PROTECT` | `protect` |
| 90 `DATE` | `date-numeric`, `date-iso`, `date-textual`, `date-month-year` |
| 85 `TIME` | `time-clock`, `time-spoken-hour` |
| 80 `PHONE` | `phone-intl`, `phone-local`, `phone-service`, `phone-star`, `phone-card`, `phone-ip`, `phone-id`, `phone-emergency` |
| 70 `CURRENCY` | `currency-suffix`, `currency-prefix` |
| 65 `PERCENT` | `percent-sign`, `percent-word` |
| 62 `RANGE` | `range-with-tail`, `range` |
| 60 `UNIT` | `unit`, `degree` |
| 55 `ABBREV` | `abbrev` |
| 50 `ORDINAL` | `ordinal` |
| 40 `NUMBER` | `count`, `number`, `number-prefix` |

`RANGE` sits above `UNIT` deliberately: `3–5 ק״מ` is one span reading
*שלושה עד חמישה קילומטרים*, not a range followed by an orphaned unit.

## Two things that are not rules

**`clean` / `finalize`** (`text.py`) rewrite the whole string rather than one span, so
they run before and after the scan. `clean` does NFC, strips zero-width and bidi
controls, and canonicalises ASCII `"` and `'` between Hebrew letters into `״` and `׳` —
which is why every rule can match one spelling instead of four. `finalize` collapses the
spacing the rules left behind.

**`strip_markdown`** (`rules/cleanup/`) is a pre-pass for the same reason. It drops
syntax and keeps text, discards fenced code blocks outright, and leaves the punctuation
that shapes prosody.

## Leaving things alone

Two decisions point the same way, and they are the ones most likely to be revisited:

- **`protect`** claims URLs, emails, filenames, handles, and Latin words fused to digits
  (`GPT-4`), then hands the span straight back. Without it the dates rule would rewrite
  the middle of `example.com/12/03/2026`.
- **A date-shaped span that fails validation is returned unchanged**, not declined.
  `32/13/2026` is far likelier to be a serial number than arithmetic, and letting the
  lower rules loose on it produced *אלפיים עשרים ושש עד שלוש עשרה*.

Not understanding something is a reason to leave it alone. The cost is that a digit can
reach the g2p; the alternative is corrupting text that was already correct.

## The Hebrew calendar

`כ״ז באלול` becomes `כ׳ ז׳ באלול`: the numeral is split into individually marked letters
and the voicing is left to the g2p, which is the right division of labour — a Hebrew date
is said by its letters (`ה׳ באייר` is /hei be-iyar/, not *חמישה באייר*), and the g2p is
where pronunciation belongs. What the normalizer owes it is the disambiguation: written
with gershayim, `כ״ז` is indistinguishable from an acronym. `hebrew_date_style=numbers`
reads them as a number instead. Three things make this more than a lookup:

- **The month is required.** A bare `כ״ז` is indistinguishable from a gershayim acronym,
  and the abbreviations rule has the better claim on it. This also forces the rule to sit
  at `DATE` priority: below `ABBREV`, `א׳ בתשרי` was read *אלף בתשרי*.
- **Some dates are words, not readings.** `ט״ו בשבט` is /tu bishvat/ to every speaker, so
  spelling it out as *חמישה עשר בשבט* is correct and wrong at once.
- **The fast days go the other way.** `ט׳ באב` is *תשעה באב* and `י״ז בתמוז` is
  *שבעה עשר בתמוז* — lexicalised as numerals, never as letters. Both kinds live in
  `HEBREW_DATE_NAMES`, and a fixed name beats whatever `hebrew_date_style` says, because
  it is a name rather than a reading.

Years are said as words (`תשפ״ו` → *תשפו*), so the marks come off and the g2p reads what
is left.

## Numerals

`numerals/` is where the Hebrew lives, and it is the only part that is genuinely hard.

```python
numeral(n, gender=FEM, *, construct=False)   # שלושה / שלוש / שלושת
decimal(whole, frac, cfg, gender=FEM)        # 3.5 -> שלוש נקודה חמש
digits(s)                                    # "050" -> אפס חמש אפס
ordinal(n, gender=FEM, *, definite=False)    # השלישית
count_phrase(n, noun, cfg)                   # numeral + noun, agreeing
noun_gender(word, cfg)
```

Points worth knowing:

- Bare counting is **feminine** (`3.5` → *שלוש נקודה חמש*), agreement is by the counted
  noun.
- **1 follows its noun** — *ילד אחד*, not *אחד ילד*. This is a reordering, which is the
  most destructive thing the library does, so it is gated hardest (see below).
- **2 binds**: *שני ילדים*, *שתי ילדות*.
- **3–10 masculine bind before a definite noun**: *שלושת הילדים*, but *שלוש הילדות*.
- Hundreds take the feminine (*שלוש מאות*); thousands take the masculine construct
  (*שלושת אלפים*, *עשרת אלפים*).
- The conjunction ו attaches to the final component: *אלף מאתיים שלושים וארבע*.

### Gender resolution

In order: `cfg.gender_overrides`, then a lexicon of ~190 common nouns, then exception
sets, then suffix morphology (`־ות` feminine, `־ים` masculine). The exception sets are
the point — `נשים`, `שנים`, `מילים`, `ערים` and the body-part duals (`עיניים`, `כנפיים`)
are feminine despite `־ים`; `מקומות`, `שולחנות`, `לילות` are masculine despite `־ות`.

### When is a number counting something?

Only if the next word is plausibly the noun it counts. The test is deliberately strict:
a plural ending (`־ים`, `־ות`), a construct plural (`־י`, as in *61 חברי כנסת*), a scale
word (*2 מיליון*), or a word the caller declared in `gender_overrides`. For `n == 1` it
must be in the lexicon outright, because that is the case that reorders words.

A false negative costs the bare feminine form, which is the right reading for a number
that isn't counting anything. A false positive mangles the sentence — it turns
*כביש 6 סמוך* into *כביש שישה* and moves words around. So the failure is aimed one way
on purpose.

## Testing

Cases live in TSVs under `tests/data/`, one row per case: `input`, `expected`, the
`config` it applies to, and a `note`. A typo in the config column raises rather than
silently testing the defaults.

Every row runs through the full public `normalize()`, which is what catches one rule
stealing another's text — a dates pattern that starts claiming `14:30` fails here and
nowhere else. On top of that, every row is checked for idempotence and for digits or
niqqud surviving into the output, and `tests/data/passthrough.tsv` asserts the things
that must come back byte-identical.

`tests/test_scanner.py` tests the scanner against fake rules only, so the machinery is
verified independently of any Hebrew.

## Adding a rule

Add a folder under `rules/` with the pattern, its lexicon, and nothing else; export
`RULES`; add it to the registry in `rules/__init__.py`. `examples/custom_rule.py` does it
from outside the package, without a fork.
