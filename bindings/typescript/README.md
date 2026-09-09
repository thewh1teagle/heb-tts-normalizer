# heb-tts-normalizer

Normalize Hebrew text for TTS. Numbers, currency, dates, times and units become spoken
words, so a g2p only ever sees words:

```
יש לי 3 ילדים ו-25₪   ->   יש לי שלושה ילדים ועשרים וחמישה שקלים
```

The Rust core compiled to WebAssembly, with a typed wrapper. One package, browser and
Node. Output is plain Hebrew, never niqqud.

```sh
npm install heb-tts-normalizer
```

## Usage

`init()` loads the WebAssembly module. Await it once, then everything is synchronous.

### Browser

Instantiating WebAssembly in a browser is asynchronous and this package does not pretend
otherwise — `normalize()` throws if you call it before `init()` has resolved.

```ts
import { init, normalize, Normalizer } from "heb-tts-normalizer";

await init();

normalize("יש לי 3 ילדים ו-25₪");
// "יש לי שלושה ילדים ועשרים וחמישה שקלים"
```

With no argument, `init()` fetches `heb_tts_normalizer_wasm_bg.wasm` from next to the
JS module — what Vite, webpack, Rollup and a plain `<script type="module">` all expect.
To serve it from somewhere else, hand `init()` a `URL`, a `Response`, an `ArrayBuffer`
or an already-compiled `WebAssembly.Module`:

```ts
await init(new URL("/assets/heb_tts_normalizer_wasm_bg.wasm", location.origin));
await init(fetch("/assets/heb_tts_normalizer_wasm_bg.wasm")); // a Promise is fine too
```

Calling `init()` again is free, and concurrent calls share a single load — so a module
that may be entered from several places can just `await init()` at the top of each.

### Node

Node instantiates the module synchronously at import time, so there is nothing to wait
for. `init()` still exists and still returns a promise, which means the same code runs
in both environments:

```ts
import { init, normalize } from "heb-tts-normalizer";

await init(); // resolves immediately under Node
console.log(normalize("הפגישה ב-14:30"));
```

Node ≥ 18. Resolution is automatic: `package.json#exports` sends Node to the `nodejs`
wasm build and everything else to the `web` build.

### Normalizing many strings

`normalize()` parses the config on every call. In a loop, construct a `Normalizer`
instead — it parses once and reuses the result:

```ts
const n = new Normalizer({ clock: "24" });
const spoken = lines.map((line) => n.normalize(line));
n.free(); // optional; the allocation is small and GC reclaims it anyway
```

## Config

Every option is optional and falls back to the Rust default — the reading an Israeli
speaker would use. Options are camelCase here and are translated to the snake_case the
Rust core expects; a key that is not an option throws rather than being silently
ignored.

```ts
normalize("14:30 ב-03/04/2026", {
  clock: "24",
  dateOrder: "dmy",
});
```

| Option | Type | Default | Meaning |
| --- | --- | --- | --- |
| `clock` | `"12" \| "24"` | `"12"` | `14:30` as שתיים וחצי אחר הצהריים, or ארבע עשרה שלושים. |
| `dateOrder` | `"dmy" \| "mdy"` | `"dmy"` | How to disambiguate `03/04/2026`. |
| `hebrewDateStyle` | `"letters" \| "numbers"` | `"letters"` | `כ״ז באלול` as `כ׳ ז׳ באלול` for the g2p to voice, or as עשרים ושבעה באלול. Fixed names (ט״ו בשבט, תשעה באב) ignore this — they are names, not readings. |
| `defaultCurrency` | `string` | `"ILS"` | Currency assumed for a bare amount that is clearly money but carries no symbol. |
| `readMinorCurrency` | `boolean` | `true` | Read agorot/cents after the major unit. |
| `decimalWord` | `string` | `"נקודה"` | Word between the integer and fractional part of a decimal. |
| `expandUnits` | `boolean` | `true` | Spell out unit abbreviations. Off leaves them untouched. |
| `expandAbbreviations` | `boolean` | `true` | Expand common abbreviations (`ד״ר` → דוקטור). |
| `stripMarkdown` | `boolean` | `true` | Drop Markdown syntax, keeping the text it wraps. |
| `cleanWhitespace` | `boolean` | `true` | Collapse whitespace, remove zero-width and bidi controls. |
| `genderOverrides` | `Record<string, "m" \| "f">` | `{}` | Extra noun genders, consulted before the built-in lexicon and the heuristics. Also makes a word count as a noun — the escape hatch for jargon. |

`genderOverrides` is how you teach it a word it does not know:

```ts
normalize("2 סושי", { genderOverrides: { סושי: "m" } });
```

## API

```ts
function init(input?: InitInput | Promise<InitInput>): Promise<void>;
function normalize(text: string, cfg?: Config): string;
function version(): string;   // the Rust crate's version
function isReady(): boolean;  // whether init() has resolved

class Normalizer {
  constructor(cfg?: Config);
  normalize(text: string): string;
  free(): void;
}
```

An invalid config throws: `configToJson` catches a misspelled camelCase key, and the
Rust side rejects a bad value (`clock: "13"`) through serde.

## Building from source

Needs a Rust toolchain and [`wasm-pack`](https://rustwasm.github.io/wasm-pack/)
(`cargo install wasm-pack`).

```sh
pnpm install
pnpm build   # wasm-pack (web + nodejs targets) into pkg/, then tsc into dist/
pnpm test
```

Two `wasm-pack` targets ship in the one package. `web` is the browser build: plain ESM
that a bundler, a CDN or a bare `<script type="module">` can all load, which `bundler`
(webpack-only) would not give. `nodejs` is the Node build, which reads the `.wasm` off
disk and instantiates it synchronously. `package.json#exports` picks between them, so
neither the caller nor a bundler ever sees the other one.

The `.wasm` is about 1.2 MB, ~470 KB over the wire once gzipped — mostly the Hebrew
lexicon. It is identical in both builds, so a bundle only ever carries one copy.

## License

MIT
