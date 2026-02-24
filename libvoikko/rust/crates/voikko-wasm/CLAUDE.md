# voikko-wasm

WASM bindings for Voikko, exported via `wasm-bindgen`. A thin wrapper around `VoikkoHandle` that exposes Finnish NLP to JavaScript.

## Purpose

This crate compiles to a WebAssembly binary (189KB after `wasm-opt -Oz`) that JavaScript can import. It translates between Rust types and JS values, handling the serialization of complex return types like analysis results, grammar errors, tokens, and sentences.

## Key types

- `WasmVoikko` -- the `#[wasm_bindgen]` exported struct wrapping `VoikkoHandle`
- `JsGrammarError`, `JsToken`, `JsSentence` -- serde-serializable DTOs for JS interop (internal, not exported directly)

## Public API surface

The `WasmVoikko` class exposes:

- **Constructor**: `new(mor_data, autocorr_data?)` -- creates an instance from raw `.vfst` bytes
- **Core methods** (15): `spell`, `suggest`, `analyze`, `hyphenate`, `grammarErrors`, `tokens`, `sentences`, `insertHyphens`, `attributeValues`, `grammarErrorsFromText`, `getVersion`, `setSpellerCacheSize`, `terminate`, `setMinHyphenatedWordLength`, `setMaxSuggestions`
- **Boolean option setters** (14): `setIgnoreDot`, `setIgnoreNumbers`, `setIgnoreUppercase`, `setNoUglyHyphenation`, `setAcceptFirstUppercase`, `setAcceptAllUppercase`, `setOcrSuggestions`, `setIgnoreNonwords`, `setAcceptExtraHyphens`, `setAcceptMissingHyphens`, `setAcceptTitlesInGc`, `setAcceptUnfinishedParagraphsInGc`, `setHyphenateUnknownWords`, `setAcceptBulletedListsInGc`

## Serialization strategy

- Simple types (`bool`, `String`, `Vec<String>`) pass through wasm-bindgen directly.
- `analyze()` builds JS objects manually via `js_sys::Object` and `js_sys::Reflect::set` for maximum compatibility.
- `grammarErrors()`, `tokens()`, `sentences()` use `serde-wasm-bindgen` to serialize DTO structs to `JsValue`.

## Dependencies

- `wasm-bindgen` -- WASM/JS bridge
- `js-sys` -- JS standard library bindings
- `serde` + `serde-wasm-bindgen` -- complex type serialization
- `voikko-fi` with `handle` feature -- the actual NLP engine

## crate-type

`cdylib` + `rlib` -- `cdylib` produces the WASM binary, `rlib` allows the crate to be used as a Rust dependency (for tests).

## Build

```bash
# Build WASM binary
cargo build --target wasm32-unknown-unknown --release -p voikko-wasm

# Generate JS bindings + TypeScript declarations
wasm-bindgen target/wasm32-unknown-unknown/release/voikko_wasm.wasm \
  --out-dir ../js/wasm --target web --typescript

# Optimize WASM binary size
wasm-opt ../js/wasm/voikko_wasm_bg.wasm -Oz --enable-bulk-memory \
  -o ../js/wasm/voikko_wasm_bg.wasm
```

## Test

```bash
cargo test -p voikko-wasm              # 4 tests (compile-time checks)
```
