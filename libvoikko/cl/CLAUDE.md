# CLAUDE.md -- Common Lisp Binding

CFFI wrapper for the Rust `voikko-ffi` cdylib. Single file: `voikko-rust.lisp` (~700 lines). Package name: `voikko-rust`.

## Prerequisites

- A Common Lisp implementation (SBCL, CCL, etc.)
- [CFFI](https://github.com/cffi/cffi) (`(ql:quickload :cffi)`)
- The Rust FFI shared library on the library search path

## Native Library

Loads `libvoikko_ffi` via CFFI:
- macOS: `libvoikko_ffi.dylib`
- Linux: `libvoikko_ffi.so`
- Windows: `voikko_ffi.dll`

Build the native library first: `cargo build --release -p voikko-ffi`

If the library is not on the default search path:

```lisp
(pushnew #P"/path/to/target/release/"
         cffi:*foreign-library-directories* :test #'equal)
```

## Usage

```lisp
(ql:quickload :cffi)
(load "voikko-rust.lisp")

(voikko-rust:with-voikko (v "/path/to/voikko-fi/vvfst/")
  (voikko-rust:voikko-spell v "kissa")       ; => T
  (voikko-rust:voikko-suggest v "kisss")      ; => ("kissa" "kissaa" ...)
  (voikko-rust:voikko-analyze v "kissalla"))  ; => ((("BASEFORM" . "kissa") ...))
```

`with-voikko` is an `unwind-protect` macro that guarantees `voikko-free` is called.

## File Structure

`voikko-rust.lisp` contains:

**Package definition** -- exports all public symbols.

**Library loading** -- `define-foreign-library` with platform-specific names. Loaded at `eval-when (:load-toplevel :execute)` with a `skip` restart.

**Condition** -- `voikko-error` (extends `error`), signaled on initialization failure.

**Constants** -- token types (`+token-none+` through `+token-unknown+`) and sentence types (`+sentence-none+` through `+sentence-possible+`).

**C struct definitions** -- `voikko-analysis`, `voikko-analysis-array`, `voikko-grammar-error`, `voikko-grammar-error-array`, `voikko-token`, `voikko-token-array`, `voikko-sentence`, `voikko-sentence-array`. These mirror the `repr(C)` structs from the Rust FFI.

**Low-level FFI** -- `defcfun` declarations for all `voikko_*` functions (prefixed with `%`).

**High-level API** -- Lisp functions that handle memory management, type conversion, and cleanup.

## API

### Lifecycle

- `(voikko-new dict-path)` -- creates a handle. Reads `mor.vfst` (and optionally `autocorr.vfst`) as octet vectors and passes them to `voikko_new`. Auto-detects flat and V5 directory layouts. Signals `voikko-error` on failure.
- `(voikko-free handle)` -- frees a handle. Safe to call with `nil` or null pointer.
- `(with-voikko (var dict-path) &body)` -- macro for scoped handle management.

### Core Functions

- `(voikko-spell handle word)` -- returns `T` or `NIL`
- `(voikko-suggest handle word)` -- returns list of strings
- `(voikko-analyze handle word)` -- returns list of alists `(("KEY" . "value") ...)`
- `(voikko-hyphenate handle word)` -- returns pattern string
- `(voikko-insert-hyphens handle word &key separator allow-context-changes)` -- returns hyphenated string
- `(voikko-grammar-errors handle text &key language)` -- returns list of plists with `:error-code`, `:start-pos`, `:error-len`, `:description`, `:suggestions`
- `(voikko-tokens handle text)` -- returns list of plists with `:token-type`, `:text`, `:position`
- `(voikko-sentences handle text)` -- returns list of plists with `:sentence-type`, `:length`

### Utility

- `(voikko-version)` -- returns version string
- `(voikko-attribute-values name)` -- returns list of strings or `nil`

### Option Setters

All take `(handle value)` where `value` is a generalized boolean (for bool options) or integer:

`voikko-set-ignore-dot`, `voikko-set-ignore-numbers`, `voikko-set-ignore-uppercase`, `voikko-set-no-ugly-hyphenation`, `voikko-set-accept-first-uppercase`, `voikko-set-accept-all-uppercase`, `voikko-set-ocr-suggestions`, `voikko-set-ignore-nonwords`, `voikko-set-accept-extra-hyphens`, `voikko-set-accept-missing-hyphens`, `voikko-set-accept-titles-in-gc`, `voikko-set-accept-unfinished-paragraphs-in-gc`, `voikko-set-hyphenate-unknown-words`, `voikko-set-accept-bulleted-lists-in-gc`, `voikko-set-min-hyphenated-word-length`, `voikko-set-max-suggestions`, `voikko-set-speller-cache-size`.

## Notes

- This binding was written specifically for the Rust FFI. It uses `voikko_new` (which accepts raw dictionary bytes) rather than the C++ `voikkoInit` (which takes a language code).
- Dictionary files are read into Lisp octet vectors and then copied to foreign memory via `with-foreign-array` before calling `voikko_new`.
- All struct-returning FFI functions return CFFI plists. The high-level API converts these to idiomatic Lisp data structures (alists, plists, strings).
- Memory management: every FFI call that allocates memory has a corresponding `unwind-protect` to ensure the free function is called.
