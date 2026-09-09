//! C ABI for `heb-tts-normalizer`.
//!
//! The surface is six functions and one opaque handle; see
//! `include/heb_tts_normalizer.h`, which is hand-written and must be kept in sync.
//!
//! Three deliberate choices:
//!
//! * **Config crosses the boundary as a JSON string, not a C struct.** `Config` has
//!   eleven fields and will grow; a struct would make every new option a silent ABI
//!   break for anyone who did not recompile. `Config::from_json` already rejects
//!   unknown keys, so a typo is a loud error rather than a silently ignored field.
//! * **An opaque handle rather than a stateless function**, because it owns the parsed
//!   config — parsing JSON on every call would dominate the cost of normalizing a
//!   short string.
//! * **Strings we return must come back to [`heb_string_free`].** They are Rust
//!   allocations; a caller's `free()` is undefined behaviour on every platform where
//!   the two allocators can differ.
//!
//! No panic may cross the boundary: every entry point runs inside `catch_unwind` and
//! turns an unwind into a null return plus a [`heb_last_error`] message. Errors are
//! kept in a thread-local, so a message is only ever read by the thread that caused it.

use std::cell::RefCell;
use std::ffi::{CStr, CString, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::OnceLock;

use heb_tts_normalizer::{Config, normalize};

thread_local! {
    /// Held for the life of the thread so [`heb_last_error`]'s pointer stays valid
    /// until the same thread produces the next error.
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

fn set_error(msg: impl Into<Vec<u8>>) {
    // A nul inside the message would truncate it; replacing is better than dropping.
    let cleaned: Vec<u8> = msg
        .into()
        .into_iter()
        .map(|b| if b == 0 { b'?' } else { b })
        .collect();
    let c = CString::new(cleaned).unwrap_or_else(|_| c"error".to_owned());
    LAST_ERROR.with(|slot| *slot.borrow_mut() = Some(c));
}

fn clear_error() {
    LAST_ERROR.with(|slot| *slot.borrow_mut() = None);
}

/// Run `f`, converting a panic into an error message and `None`.
fn guard<T>(what: &str, f: impl FnOnce() -> Option<T>) -> Option<T> {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(v) => v,
        Err(payload) => {
            let detail = payload
                .downcast_ref::<&str>()
                .map(|s| (*s).to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown panic".to_string());
            set_error(format!("{what} panicked: {detail}"));
            None
        }
    }
}

/// Read a `const char*` as UTF-8. Invalid UTF-8 is an error, never a panic.
fn borrow_utf8<'a>(ptr: *const c_char, what: &str) -> Option<&'a str> {
    if ptr.is_null() {
        set_error(format!("{what} is NULL"));
        return None;
    }
    // SAFETY: non-null and, by the header's contract, a nul-terminated string that
    // outlives this call.
    let bytes = unsafe { CStr::from_ptr(ptr) };
    match bytes.to_str() {
        Ok(s) => Some(s),
        Err(e) => {
            set_error(format!("{what} is not valid UTF-8: {e}"));
            None
        }
    }
}

/// An opaque normalizer. Holds the parsed [`Config`]; immutable, so sharing one across
/// threads is safe as long as [`heb_normalizer_free`] happens once, after the last use.
pub struct HebNormalizer {
    cfg: Config,
}

/// Create a normalizer from a JSON config. `NULL` or an empty string means defaults.
///
/// Returns `NULL` on failure; call [`heb_last_error`] for the reason.
///
/// # Safety
/// `config_json` must be `NULL` or a nul-terminated string valid for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn heb_new(config_json: *const c_char) -> *mut HebNormalizer {
    clear_error();
    let handle = guard("heb_new", || {
        let json = if config_json.is_null() {
            ""
        } else {
            borrow_utf8(config_json, "config_json")?
        };
        match Config::from_json(json) {
            Ok(cfg) => Some(Box::new(HebNormalizer { cfg })),
            Err(e) => {
                set_error(format!("invalid config JSON: {e}"));
                None
            }
        }
    });
    match handle {
        Some(b) => Box::into_raw(b),
        None => std::ptr::null_mut(),
    }
}

/// Destroy a normalizer. `NULL` is a no-op; passing the same pointer twice is not.
///
/// # Safety
/// `handle` must be `NULL` or a pointer from [`heb_new`] that has not been freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn heb_normalizer_free(handle: *mut HebNormalizer) {
    if handle.is_null() {
        return;
    }
    // SAFETY: the caller guarantees this came from heb_new and is freed once.
    let boxed = unsafe { Box::from_raw(handle) };
    let _ = guard("heb_normalizer_free", || {
        drop(boxed);
        Some(())
    });
}

/// Normalize `utf8_in`. Returns a freshly allocated UTF-8 string that the caller must
/// release with [`heb_string_free`], or `NULL` on failure.
///
/// # Safety
/// `handle` must come from [`heb_new`] and still be live; `utf8_in` must be a
/// nul-terminated string valid for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn heb_normalize(
    handle: *const HebNormalizer,
    utf8_in: *const c_char,
) -> *mut c_char {
    clear_error();
    let out = guard("heb_normalize", || {
        if handle.is_null() {
            set_error("handle is NULL");
            return None;
        }
        // SAFETY: non-null, and the caller guarantees it is a live heb_new pointer.
        let norm = unsafe { &*handle };
        let input = borrow_utf8(utf8_in, "utf8_in")?;
        let result = normalize(input, &norm.cfg);
        // Normalized Hebrew never contains a nul, but a caller's input could carry one
        // through a rule, and a nul would truncate the result for the caller.
        match CString::new(result) {
            Ok(c) => Some(c),
            Err(_) => {
                set_error("result contains an interior NUL byte");
                None
            }
        }
    });
    match out {
        Some(c) => c.into_raw(),
        None => std::ptr::null_mut(),
    }
}

/// Free a string returned by [`heb_normalize`]. `NULL` is a no-op.
///
/// # Safety
/// `s` must be `NULL` or a pointer returned by [`heb_normalize`] and not yet freed.
/// Never pass it to `free()` instead — the allocator may not be the C one.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn heb_string_free(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    // SAFETY: the caller guarantees this came from heb_normalize and is freed once.
    let owned = unsafe { CString::from_raw(s) };
    drop(owned);
}

/// The last error on *this* thread, or `""` if the last call succeeded.
///
/// The pointer is owned by the library and stays valid until the same thread makes
/// another call that fails; never free it.
///
/// # Safety
/// The returned pointer must not outlive the next failing call on this thread.
#[unsafe(no_mangle)]
pub extern "C" fn heb_last_error() -> *const c_char {
    LAST_ERROR.with(|slot| match slot.borrow().as_ref() {
        Some(c) => c.as_ptr(),
        None => c"".as_ptr(),
    })
}

/// The library version, as a static nul-terminated string. Never free it.
#[unsafe(no_mangle)]
pub extern "C" fn heb_version() -> *const c_char {
    static VERSION: OnceLock<CString> = OnceLock::new();
    VERSION
        .get_or_init(|| {
            CString::new(heb_tts_normalizer::VERSION).unwrap_or_else(|_| c"unknown".to_owned())
        })
        .as_ptr()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(h: *const HebNormalizer, s: &str) -> Option<String> {
        let input = CString::new(s).unwrap();
        let out = unsafe { heb_normalize(h, input.as_ptr()) };
        if out.is_null() {
            return None;
        }
        let owned = unsafe { CStr::from_ptr(out) }.to_str().unwrap().to_string();
        unsafe { heb_string_free(out) };
        Some(owned)
    }

    fn last_error() -> String {
        unsafe { CStr::from_ptr(heb_last_error()) }
            .to_str()
            .unwrap()
            .to_string()
    }

    #[test]
    fn round_trip_with_default_config() {
        let h = unsafe { heb_new(std::ptr::null()) };
        assert!(!h.is_null());
        assert!(call(h, "3 ילדים").is_some());
        unsafe { heb_normalizer_free(h) };
    }

    #[test]
    fn config_json_is_parsed_and_typos_are_loud() {
        let ok = CString::new(r#"{"clock":"24"}"#).unwrap();
        let h = unsafe { heb_new(ok.as_ptr()) };
        assert!(!h.is_null());
        unsafe { heb_normalizer_free(h) };

        let bad = CString::new(r#"{"clokc":"24"}"#).unwrap();
        let h = unsafe { heb_new(bad.as_ptr()) };
        assert!(h.is_null());
        assert!(last_error().contains("invalid config JSON"));
    }

    #[test]
    fn null_pointers_are_errors_not_crashes() {
        assert!(unsafe { heb_normalize(std::ptr::null(), c"x".as_ptr()) }.is_null());
        assert_eq!(last_error(), "handle is NULL");

        let h = unsafe { heb_new(std::ptr::null()) };
        assert!(unsafe { heb_normalize(h, std::ptr::null()) }.is_null());
        assert_eq!(last_error(), "utf8_in is NULL");
        unsafe { heb_normalizer_free(h) };

        unsafe { heb_normalizer_free(std::ptr::null_mut()) };
        unsafe { heb_string_free(std::ptr::null_mut()) };
    }

    #[test]
    fn invalid_utf8_is_an_error() {
        let h = unsafe { heb_new(std::ptr::null()) };
        // c_char is signed on x86_64 and unsigned on aarch64, so cast rather than
        // write the byte as a literal.
        let bad: [c_char; 3] = [0x61u8 as c_char, 0xFFu8 as c_char, 0];
        assert!(unsafe { heb_normalize(h, bad.as_ptr()) }.is_null());
        assert!(last_error().contains("not valid UTF-8"));
        unsafe { heb_normalizer_free(h) };
    }

    #[test]
    fn version_is_reported() {
        let v = unsafe { CStr::from_ptr(heb_version()) }.to_str().unwrap();
        assert_eq!(v, heb_tts_normalizer::VERSION);
    }
}
