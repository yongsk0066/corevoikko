# Corevoikko

Finnish natural language processing library — spell checking, morphological analysis, hyphenation, grammar checking, and tokenization.

This is a Rust rewrite of the original [Voikko](https://voikko.puimula.org/) C++ library, compiled to native code and WebAssembly. The original C++ source is preserved in `libvoikko/legacy/` for reference.

## Features

- **Spell checking** with compound word and derivation support
- **Spelling suggestions** tuned for common typing errors and OCR correction
- **Morphological analysis** with full inflection details
- **Hyphenation** with compound-aware splitting
- **Grammar checking** with context-sensitive paragraph analysis
- **Tokenization** and sentence splitting

## Repository Structure

```
corevoikko/
├── libvoikko/
│   ├── rust/                  # Rust implementation (6 crates, 637 tests)
│   │   └── crates/
│   │       ├── voikko-core/   # Shared types and enums
│   │       ├── voikko-fst/    # VFST finite state transducer engine
│   │       ├── voikko-fi/     # Finnish language module
│   │       ├── voikko-wasm/   # WebAssembly build (wasm-bindgen)
│   │       ├── voikko-ffi/    # C FFI shared library
│   │       └── voikko-cli/    # CLI tools (8 binaries)
│   ├── js/                    # npm package (@yongsk0066/voikko)
│   ├── python/                # Python bindings (ctypes → voikko-ffi)
│   ├── java/                  # Java bindings (JNA → voikko-ffi)
│   ├── cs/                    # C# bindings (P/Invoke → voikko-ffi)
│   ├── cl/                    # Common Lisp bindings (CFFI → voikko-ffi)
│   └── legacy/                # Original C++ source (preserved)
├── voikko-fi/                 # Finnish dictionary data (VFST format)
├── plan/                      # Porting design documents
├── data/                      # Grammar help XML
├── tests/                     # Integration test data
└── tools/                     # Developer utilities
```

## Quick Start

### npm (Browser / Node.js)

```bash
npm install @yongsk0066/voikko
```

```typescript
import { Voikko } from '@yongsk0066/voikko';

// Node.js — dictionary is bundled, zero config
const voikko = await Voikko.init();

// Browser — serve dictionary files via HTTP
const voikko = await Voikko.init('fi', { dictionaryUrl: '/dict/' });

voikko.spell('koira');        // true
voikko.suggest('koirra');     // ['koira', ...]
voikko.analyze('koirien');    // [{ STRUCTURE: '=pppppp=p', ... }]
voikko.hyphenate('kissa');    // 'kis-sa'
voikko.terminate();
```

Finnish dictionary files are **bundled** in the npm package. Node.js users need no additional setup. For browser usage, copy the dictionary files from `node_modules/@yongsk0066/voikko/dict/` to your public directory and pass `dictionaryUrl`.

### Rust

```bash
cd libvoikko/rust
cargo test --all-features     # 637 tests
cargo clippy --all-features -- -D warnings
```

### CLI Tools

```bash
cd libvoikko/rust
VOIKKO_DICT_PATH=/path/to/dict cargo run -p voikko-cli --bin voikko-spell
```

Available: `voikko-spell`, `voikko-suggest`, `voikko-analyze`, `voikko-hyphenate`, `voikko-tokenize`, `voikko-gc-pretty`, `voikko-baseform`, `voikko-readability`

### Native Library (FFI)

```bash
cd libvoikko/rust
cargo build --release -p voikko-ffi
# → target/release/libvoikko_ffi.{dylib,so,dll}
```

Bindings available for Python (ctypes), Java (JNA), C# (P/Invoke), and Common Lisp (CFFI). See `libvoikko/python/`, `libvoikko/java/`, `libvoikko/cs/`, `libvoikko/cl/`.

### WASM Build

```bash
cd libvoikko/rust
cargo build --target wasm32-unknown-unknown --release -p voikko-wasm
wasm-bindgen target/wasm32-unknown-unknown/release/voikko_wasm.wasm \
  --out-dir ../js/wasm --target web --typescript
wasm-opt ../js/wasm/voikko_wasm_bg.wasm -Oz --enable-bulk-memory \
  -o ../js/wasm/voikko_wasm_bg.wasm
```

### Finnish Dictionary

```bash
cd voikko-fi
make vvfst                    # Requires foma, Python 3, GNU make
make vvfst-install DESTDIR=~/.voikko
```

## Language Bindings

| Language | Location | Mechanism | Status |
|----------|----------|-----------|--------|
| JS/TS | `libvoikko/js/` | voikko-wasm (wasm-bindgen) | 37 vitest |
| Python | `libvoikko/python/` | ctypes → voikko-ffi | Verified |
| Java | `libvoikko/java/` | JNA → voikko-ffi | Scaffold |
| C# | `libvoikko/cs/` | P/Invoke → voikko-ffi | Scaffold |
| Common Lisp | `libvoikko/cl/` | CFFI → voikko-ffi | Scaffold |

## License

Tri-licensed under [MPL 1.1](libvoikko/LICENSE.CORE) / [GPL 2+](libvoikko/COPYING) / [LGPL 2.1+](libvoikko/LICENSE.CORE).

See [LICENSE](LICENSE) for full details.

## Credits

This project is a Rust rewrite of [Voikko](https://voikko.puimula.org/), originally created by Harri Pitkanen and contributors. The original C++ implementation and the linguistic data in `voikko-fi/` are the work of the Voikko project contributors.

## Links

- [npm package](https://www.npmjs.com/package/@yongsk0066/voikko)
- [Original Voikko project](https://voikko.puimula.org/)
