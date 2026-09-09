/**
 * Browser entry point.
 *
 * `WebAssembly.instantiateStreaming` is asynchronous and there is no honest way around
 * that, so the module is loaded by an `init()` the caller awaits once. Everything after
 * that is synchronous.
 */

import initWasm, {
  type InitInput,
  type InitOutput,
} from "../pkg/web/heb_tts_normalizer_wasm.js";
import * as wasm from "../pkg/web/heb_tts_normalizer_wasm.js";
import { setWasm, type WasmExports } from "./api.js";

export {
  Normalizer,
  isReady,
  normalize,
  version,
  type WasmExports,
} from "./api.js";
export type {
  Clock,
  Config,
  DateOrder,
  Gender,
  HebrewDateStyle,
} from "./config.js";

let pending: Promise<void> | undefined;

/**
 * Load the wasm module. Await this once before anything else.
 *
 * `input` is passed straight through to `wasm-bindgen`'s initializer: leave it out and
 * the `.wasm` file is fetched from next to this module, which is what a bundler or a
 * plain `<script type="module">` both want. Pass a `URL`, `Response`, `ArrayBuffer` or
 * `WebAssembly.Module` to load it from somewhere else.
 *
 * Concurrent calls share one load, and later calls resolve immediately.
 */
export function init(input?: InitInput | Promise<InitInput>): Promise<void> {
  pending ??= (async () => {
    // The object form is the non-deprecated way to pass a location to wasm-bindgen;
    // omitting the argument entirely is what picks up the sibling `.wasm` file.
    const loaded: Promise<InitOutput> =
      input === undefined ? initWasm() : initWasm({ module_or_path: input });
    await loaded;
    setWasm(wasm as unknown as WasmExports);
  })().catch((err: unknown) => {
    // A failed load must not poison the module: let the next call retry.
    pending = undefined;
    throw err;
  });
  return pending;
}

export type { InitInput };
