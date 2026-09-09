//! WebAssembly bindings for `heb_tts_normalizer`.
//!
//! The config crosses the boundary as a JSON string rather than a JS object so that
//! there is exactly one config representation in the project — the same one the C API
//! uses, parsed by the same serde derive. `deny_unknown_fields` on the core `Config`
//! means a misspelled key surfaces here as a thrown `Error`, not as a silent default.

use heb_tts_normalizer::{Config, VERSION};
use wasm_bindgen::prelude::*;

/// Install the panic hook once, so a Rust panic shows up as a readable stack trace in
/// the browser console instead of `unreachable executed`.
fn set_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    {
        use std::sync::Once;
        static ONCE: Once = Once::new();
        ONCE.call_once(console_error_panic_hook::set_once);
    }
}

fn parse(config_json: Option<String>) -> Result<Config, JsError> {
    match config_json {
        Some(json) => {
            Config::from_json(&json).map_err(|e| JsError::new(&format!("invalid config JSON: {e}")))
        }
        None => Ok(Config::default()),
    }
}

/// A normalizer holding a parsed config, so a config is parsed once and reused across
/// many calls.
#[wasm_bindgen]
pub struct Normalizer {
    config: Config,
}

#[wasm_bindgen]
impl Normalizer {
    /// `config_json` is a JSON object of `Config` fields; `undefined` means defaults.
    #[wasm_bindgen(constructor)]
    pub fn new(config_json: Option<String>) -> Result<Normalizer, JsError> {
        set_hook();
        Ok(Normalizer {
            config: parse(config_json)?,
        })
    }

    /// Normalize `text` with this normalizer's config.
    #[wasm_bindgen]
    pub fn normalize(&self, text: &str) -> String {
        heb_tts_normalizer::normalize(text, &self.config)
    }
}

/// One-shot normalization. Parses the config on every call; prefer [`Normalizer`] in a
/// loop.
#[wasm_bindgen]
pub fn normalize(text: &str, config_json: Option<String>) -> Result<String, JsError> {
    set_hook();
    Ok(heb_tts_normalizer::normalize(text, &parse(config_json)?))
}

/// The core crate's version.
#[wasm_bindgen]
pub fn version() -> String {
    VERSION.to_string()
}
