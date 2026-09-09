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

**`clean` / `finalize`** (`text.rs`) rewrite the whole string rather than one span, so
they run before and after the scan. `clean` does NFC, strips zero-width and bidi
controls, and canonicalises ASCII `"` and `'` between Hebrew letters into `״` and `׳` —
which is why every rule can match one spelling instead of four. `finalize` collapses the
spacing the rules left behind.

**`strip_markdown`** (`rules/cleanup.rs`) is a pre-pass for the same reason. It drops
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

```rust
numeral(n, gender, construct)        // שלושה / שלוש / שלושת
decimal(whole, frac, cfg, gender)    // 3.5 -> שלוש נקודה חמש
digits(s)                            // "050" -> אפס חמש אפס
ordinal(n, gender, definite)         // השלישית
count_phrase(n, noun, cfg)           // numeral + noun, agreeing
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

Cases live in TSVs under `crates/heb-tts-normalizer/tests/data/`, one row per case:
`input`, `expected`, the `config` it applies to, and a `note`. A typo in the config
column fails the test naming `file:line`, rather than silently testing the defaults.
They live inside the crate so it is self-contained and publishable — at the repo root,
`cargo publish` would ship a crate whose own tests could not run.

Every row runs through the full public `normalize()`, which is what catches one rule
stealing another's text — a dates pattern that starts claiming `14:30` fails here and
nowhere else. On top of that, every row is checked for idempotence and for digits or
niqqud surviving into the output, and `tests/data/passthrough.tsv` asserts the things
that must come back byte-identical.

`tests/scanner.rs` tests the scanner against fake rules only, so the machinery is
verified independently of any Hebrew — including the multi-byte cases that only matter in
Rust: a decline must advance a whole character rather than a byte, and the skip-ahead must
land on a char boundary.

### Text nobody wrote for us

Every TSV row was written by whoever wrote the rules, which is the wrong provenance for
finding blind spots: a case nobody thought of never gets a row, and the tests agree with
the code because they came from the same head. So `tests/data/soak/` holds a few
thousand real Hebrew sentences mined from [OPUS](https://opus.nlpl.eu/) — subtitles,
Wikipedia, lecture transcripts, Tatoeba — by `scripts/mine_opus.py`, sampled stratified
by trigger type so a Hebrew-calendar date or a per-mille sign is not drowned by plain
integers. Corpus choice is constrained by licence as much as by content: the sample is
committed inside a crate published under MIT, so a non-commercial corpus stays out of it
however good its numbers are.

None of it is reviewed, so none of it says what the right answer is. `tests/soak.rs`
asserts only what must hold whatever the right answer is: `normalize` returns rather
than panicking, it is idempotent, it invents no niqqud, prose with no trigger in it goes
through the scan untouched, and the output is properly finalized. That needs no expected
values, so it scales to as much text as we care to run — and it is where a real crash or
a mangled sentence turns up.

Two numbers are measured rather than asserted, because neither is wrong by itself:
how often a digit survives into the output, and how often a sentence with a trigger comes
back unchanged. `32/13/2026` is not a date and a catalogue number is not a quantity, so
both are legitimately non-zero. They are pinned as budgets that may only go down;
`chore soak-report` prints the sentences behind them, which is where the next golden TSV
row comes from.

`tests/data/opus.tsv` is where those sentences end up once somebody has decided what they
should sound like. Each row started in the soak sample and was read by two independent
reviewers; where they disagreed the sentence stayed out, on the grounds that a row nobody
is sure about is worse than no row. These are ordinary corpus rows and run with the rest,
so a rule change that alters a real sentence fails by name instead of moving a percentage.

Real text is full of numbers that are not quantities, so a row there can legitimately
keep a digit — an unread fraction, an impossible date, a track length. Such a row says so
in its `note`, which starts `keeps-digit:` and gives the reason. The reason sits beside
the row rather than in a table keyed by line number, which would point at the wrong row
the moment anyone inserted a line.

## Adding a rule

Add a module under `src/rules/` with the pattern, its lexicon, and nothing else; expose
`pub fn rules() -> Vec<Box<dyn Rule>>`; add it to the registry in `rules/mod.rs`.
`examples/custom_rule.rs` does it from outside the crate, without a fork.

## The port, and what it cost

The library began as Python; that implementation is in the history at `a31cd38`, and its
corpus is why the Rust port is verifiable rather than a reinterpretation — every rule was
checked row-for-row against it before the Python was deleted.

Two things the port forced:

- **`fancy-regex`, not `regex`.** RE2 — the `regex` crate, and Go's `regexp` — has no
  lookaround at all, and the rules lean on it constantly. `fancy-regex` keeps the patterns
  identical to the originals. It costs binary size and speed, and it is why the wasm is
  1.2 MiB and `normalize` runs at ~6 µs/char. Converting the lookarounds into explicit
  position checks in `render` would let us drop to `regex`; the scanner already hands each
  rule the full text and its offset, so the door is open.
- **A match cache in the scanner.** Asking 28 rules for a match at every position, when
  each search scans forward through the remaining text, is quadratic — it measured 820 µs
  on a 19-character string. The text is immutable, so a rule's leftmost match from `pos`
  stays its leftmost match until `pos` passes it; caching that made it linear.

## Bindings

One core, three surfaces. Config crosses every boundary as the same JSON object, so
there is a single config representation for the whole project rather than one per binding.

- **C ABI** — an opaque handle (regex construction is amortised across calls), a
  JSON config string rather than a struct (a struct makes every added option a silent ABI
  break), and a mandatory `heb_string_free`, because a caller must never `free()` a Rust
  allocation.
- **Python** — ctypes over that ABI. `heb_normalize`'s `restype` is `c_void_p`, not
  `c_char_p`: ctypes converts a `c_char_p` result to `bytes` and throws the address away,
  which would leak on every call.
- **wasm** — `web` and `nodejs` builds behind an `exports` map. Init is async in the
  browser and the API says so, rather than pretending otherwise.
