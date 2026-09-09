/*
 * heb-tts-normalizer — C ABI.
 *
 * Normalize Hebrew text for TTS: numbers, currency, dates, times and units become
 * spoken words. Output is plain Hebrew, never niqqud. All strings are UTF-8.
 *
 * This header is hand-written and is the authority on the ABI; keep it in sync with
 * crates/heb-tts-normalizer-c/src/lib.rs.
 *
 * Memory rules, in short:
 *   - heb_normalize returns an owned string; release it with heb_string_free, never
 *     with free() — the allocator is Rust's and need not be the C one.
 *   - heb_last_error and heb_version return static / library-owned strings; never free.
 *   - a handle from heb_new is released once with heb_normalizer_free.
 *
 * Errors: a function that can fail returns NULL and leaves a message in
 * heb_last_error(). The message is thread-local and stays valid until the next failing
 * call on the same thread. No panic crosses this boundary.
 *
 * Threads: a handle is immutable after creation, so several threads may normalize
 * through the same handle concurrently. Freeing it must happen once, after the last use.
 */

#ifndef HEB_TTS_NORMALIZER_H
#define HEB_TTS_NORMALIZER_H

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque normalizer; owns the parsed config. */
typedef struct HebNormalizer HebNormalizer;

/*
 * Create a normalizer.
 *
 * config_json: a JSON object of reading-style options, or NULL / "" for the defaults.
 *              Unknown keys are rejected, so a typo is loud. Fields:
 *
 *   clock              "12" | "24"                (default "12")
 *   date_order         "dmy" | "mdy"              (default "dmy")
 *   hebrew_date_style  "letters" | "numbers"      (default "letters")
 *   default_currency   string                     (default "ILS")
 *   read_minor_currency bool                      (default true)
 *   decimal_word       string                     (default "נקודה")
 *   expand_units       bool                       (default true)
 *   expand_abbreviations bool                     (default true)
 *   strip_markdown     bool                       (default true)
 *   clean_whitespace   bool                       (default true)
 *   gender_overrides   object of word -> "m"|"f"  (default {})
 *
 * Returns NULL on failure; see heb_last_error().
 */
HebNormalizer *heb_new(const char *config_json);

/* Release a normalizer. NULL is a no-op. */
void heb_normalizer_free(HebNormalizer *handle);

/*
 * Normalize utf8_in. Returns an owned, nul-terminated UTF-8 string that the caller
 * releases with heb_string_free, or NULL on failure (NULL handle or input, invalid
 * UTF-8). See heb_last_error().
 */
char *heb_normalize(const HebNormalizer *handle, const char *utf8_in);

/* Release a string returned by heb_normalize. NULL is a no-op. */
void heb_string_free(char *s);

/*
 * The last error on this thread, or "" if the last call succeeded. Library-owned;
 * never free it. Valid until the next failing call on this thread — copy it if you
 * need to keep it.
 */
const char *heb_last_error(void);

/* The library version, e.g. "0.1.0". Static; never free it. */
const char *heb_version(void);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* HEB_TTS_NORMALIZER_H */
