/**
 * Node entry point.
 *
 * Node's `wasm-bindgen` build instantiates the module synchronously at import time from
 * the filesystem, so there is nothing to wait for. `init()` still exists and still
 * returns a promise, so the same code runs unchanged in both environments.
 */

import * as wasm from "../pkg/node/heb_tts_normalizer_wasm.js";
import { setWasm, type WasmExports } from "./api.js";

setWasm(wasm as unknown as WasmExports);

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

/** Already loaded under Node; present so browser and Node code look the same. */
export function init(_input?: unknown): Promise<void> {
  return Promise.resolve();
}
