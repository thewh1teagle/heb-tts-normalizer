/**
 * The environment-independent half of the package: the typed surface, plus the slot the
 * per-environment entry point (`browser.ts` / `node.ts`) drops its loaded wasm module
 * into.
 */

import { type Config, configToJson } from "./config.js";

/** The shape `wasm-bindgen` generates. Structural, so both builds satisfy it. */
export interface WasmExports {
  normalize(text: string, config_json?: string): string;
  version(): string;
  Normalizer: new (config_json?: string) => {
    normalize(text: string): string;
    free(): void;
  };
}

let wasm: WasmExports | undefined;

/** Called by the entry point once its build is loaded. Not part of the public API. */
export function setWasm(exports: WasmExports): void {
  wasm = exports;
}

/** Whether the wasm module is loaded and the synchronous API is usable. */
export function isReady(): boolean {
  return wasm !== undefined;
}

function required(): WasmExports {
  if (wasm === undefined) {
    throw new Error(
      "heb-tts-normalizer: wasm is not loaded yet — `await init()` before calling normalize()",
    );
  }
  return wasm;
}

/**
 * Normalize a single string. Parses the config on every call; construct a
 * {@link Normalizer} instead when normalizing many strings with one config.
 */
export function normalize(text: string, cfg?: Config): string {
  return required().normalize(text, configToJson(cfg));
}

/** The version of the underlying Rust crate. */
export function version(): string {
  return required().version();
}

/**
 * A normalizer holding one parsed config.
 *
 * The wasm object it owns lives in linear memory. It is small, and modern runtimes
 * reclaim it through `FinalizationRegistry`, but {@link free} releases it immediately if
 * you are creating many.
 */
export class Normalizer {
  readonly #inner: InstanceType<WasmExports["Normalizer"]>;

  constructor(cfg?: Config) {
    const ctor = required().Normalizer;
    this.#inner = new ctor(configToJson(cfg));
  }

  normalize(text: string): string {
    return this.#inner.normalize(text);
  }

  /** Release the wasm-side allocation. The instance is unusable afterwards. */
  free(): void {
    this.#inner.free();
  }
}
