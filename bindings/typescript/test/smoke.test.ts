import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

import { configToJson } from "../src/config.js";
import { Normalizer, init, isReady, normalize, version } from "../src/node.js";

await init();

/** Nothing a g2p cannot pronounce should survive normalization. */
const DIGIT_OR_SYMBOL = /[0-9₪$€:%]/u;

// Read rather than hardcoded: the wasm reports the crate version, and the point of the
// assertion is that the npm package and the crate are released in lockstep.
const pkgVersion = JSON.parse(
  readFileSync(new URL("../package.json", import.meta.url), "utf8"),
).version as string;

describe("wasm loading", () => {
  it("is ready and reports the version the package claims", () => {
    expect(isReady()).toBe(true);
    expect(version()).toBe(pkgVersion);
  });
});

describe("normalize", () => {
  it("matches the crate's documented example", () => {
    expect(normalize("יש לי 3 ילדים ו-25₪")).toBe(
      "יש לי שלושה ילדים ועשרים וחמישה שקלים",
    );
  });

  it.each([
    ["3 ילדים", "שלושה"],
    ["₪25", "שקלים"],
    // The default clock is 12-hour, so 14:30 is read as "half past two, afternoon".
    ["14:30", "וחצי"],
    ["כ״ז באלול", "אלול"],
  ])("spells out %j", (input, expectedFragment) => {
    const out = normalize(input);
    expect(out).toContain(expectedFragment);
    expect(out).not.toMatch(DIGIT_OR_SYMBOL);
  });

  it("leaves plain Hebrew alone", () => {
    expect(normalize("שלום עולם")).toBe("שלום עולם");
  });

  it("handles the empty string", () => {
    expect(normalize("")).toBe("");
  });
});

describe("config", () => {
  it("maps camelCase to the snake_case serde expects", () => {
    expect(
      JSON.parse(
        configToJson({
          clock: "24",
          dateOrder: "mdy",
          hebrewDateStyle: "numbers",
          expandUnits: false,
          genderOverrides: { פיצה: "f" },
        })!,
      ),
    ).toEqual({
      clock: "24",
      date_order: "mdy",
      hebrew_date_style: "numbers",
      expand_units: false,
      gender_overrides: { פיצה: "f" },
    });
  });

  it("omits an unset config entirely, so Rust uses its own defaults", () => {
    expect(configToJson(undefined)).toBeUndefined();
    expect(configToJson({})).toBe("{}");
  });

  it("names the camelCase key when it is not a real option", () => {
    // @ts-expect-error -- the point of the test is the runtime guard
    expect(() => configToJson({ date_order: "dmy" })).toThrow(/date_order/);
  });

  it("round-trips through wasm: the clock option changes the reading", () => {
    const twelve = normalize("14:30", { clock: "12" });
    const twentyFour = normalize("14:30", { clock: "24" });
    expect(twelve).not.toBe(twentyFour);
    expect(twentyFour).toContain("שלושים");
  });

  it("rejects a config Rust cannot parse", () => {
    // `deny_unknown_fields` fires on the Rust side for a valid-JSON-but-wrong shape.
    expect(() => new Normalizer({ clock: "13" as "12" })).toThrow();
  });
});

describe("Normalizer", () => {
  it("reuses one parsed config across calls", () => {
    const n = new Normalizer({ hebrewDateStyle: "numbers" });
    try {
      expect(n.normalize("כ״ז באלול")).toBe(n.normalize("כ״ז באלול"));
      expect(n.normalize("3 ילדים")).toContain("שלושה");
    } finally {
      n.free();
    }
  });

  it("agrees with the free function", () => {
    const n = new Normalizer();
    try {
      for (const s of ["3 ילדים", "₪25", "14:30", "כ״ז באלול"]) {
        expect(n.normalize(s)).toBe(normalize(s));
      }
    } finally {
      n.free();
    }
  });
});
