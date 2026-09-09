/**
 * The reading-style options, mirroring the Rust `Config`.
 *
 * TypeScript users write camelCase; the JSON handed to Rust is snake_case. The mapping
 * lives in {@link configToJson} and is the only place the two spellings meet.
 */

/** Grammatical gender of a counted noun. */
export type Gender = "m" | "f";

/** How to read a time of day. */
export type Clock = "12" | "24";

/** How to disambiguate `03/04/2026`. */
export type DateOrder = "dmy" | "mdy";

/** How to read the gematria in a Hebrew-calendar date. */
export type HebrewDateStyle = "letters" | "numbers";

export interface Config {
  /** `14:30` as שתיים וחצי אחר הצהריים (`"12"`, default) or ארבע עשרה שלושים (`"24"`). */
  clock?: Clock;
  /** Day-month-year is the Israeli convention and the default. */
  dateOrder?: DateOrder;
  /**
   * `"letters"` (default) leaves `כ״ז באלול` as `כ׳ ז׳ באלול` for the g2p to voice;
   * `"numbers"` reads it as עשרים ושבעה באלול. Fixed names (ט״ו בשבט, תשעה באב) ignore
   * this — they are names, not readings.
   */
  hebrewDateStyle?: HebrewDateStyle;
  /** Currency assumed for a bare amount that is clearly money but carries no symbol. Default `"ILS"`. */
  defaultCurrency?: string;
  /** Read agorot/cents after the major unit. Default `true`. */
  readMinorCurrency?: boolean;
  /** Word between the integer and fractional part of a decimal. Default `"נקודה"`. */
  decimalWord?: string;
  /** Spell out unit abbreviations. Off leaves them untouched. Default `true`. */
  expandUnits?: boolean;
  /** Expand common abbreviations (`ד״ר` -> דוקטור). Default `true`. */
  expandAbbreviations?: boolean;
  /** Drop Markdown syntax, keeping the text it wraps. Default `true`. */
  stripMarkdown?: boolean;
  /** Collapse whitespace and remove zero-width / bidi control characters. Default `true`. */
  cleanWhitespace?: boolean;
  /**
   * Extra noun genders, consulted before the built-in lexicon and the heuristics. Also
   * makes a word count as a noun, which is the escape hatch for jargon.
   */
  genderOverrides?: Record<string, Gender>;
}

/** camelCase key -> the snake_case name serde expects. */
const WIRE_KEYS: { readonly [K in keyof Required<Config>]: string } = {
  clock: "clock",
  dateOrder: "date_order",
  hebrewDateStyle: "hebrew_date_style",
  defaultCurrency: "default_currency",
  readMinorCurrency: "read_minor_currency",
  decimalWord: "decimal_word",
  expandUnits: "expand_units",
  expandAbbreviations: "expand_abbreviations",
  stripMarkdown: "strip_markdown",
  cleanWhitespace: "clean_whitespace",
  genderOverrides: "gender_overrides",
};

/**
 * Render a config as the JSON string the wasm boundary takes, or `undefined` for the
 * Rust defaults. Unset keys are omitted rather than sent as `null`: the Rust struct is
 * `#[serde(default)]`, so an absent key is the default, but a `null` is a type error.
 *
 * An unknown key is rejected here rather than by serde's `deny_unknown_fields`, so the
 * error names the camelCase spelling the caller actually wrote.
 */
export function configToJson(cfg?: Config): string | undefined {
  if (cfg === undefined) return undefined;
  const wire: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(cfg)) {
    if (value === undefined) continue;
    const wireKey = (WIRE_KEYS as Record<string, string | undefined>)[key];
    if (wireKey === undefined) {
      throw new TypeError(
        `heb-tts-normalizer: unknown config option ${JSON.stringify(key)}`,
      );
    }
    wire[wireKey] = value;
  }
  return JSON.stringify(wire);
}
