# Rust WASM Ecosystem Research

> Research on existing Rust WASM projects, patterns, and tooling to inform the libvoikko Rust WASM port.
> Covers divvunspell architecture, wasm-bindgen patterns, build toolchain, binary parsing, NLP crates,
> integration strategy, and testing.

---

## 1. divvunspell Architecture Analysis

### 1.1 Overview

divvunspell is a Rust reimplementation of hfst-ospell for Finno-Ugric language spellchecking. It supports
ZHFST (ZIP-compressed HFST) and BHFST (Box HFST) archive formats with memory-mapped transducer loading,
parallel suggestion generation, Unicode-aware tokenization, and cross-platform support (macOS, Linux,
Windows, iOS, Android).

- Repository: https://github.com/divvun/divvunspell
- License: Apache-2.0 / MIT (library), GPL-3.0 (CLI tools)
- Rust edition: 2024, workspace with resolver v2

### 1.2 Module Structure

```
divvunspell/
  src/
    archive/          # Dictionary archive loading
      boxf.rs         #   BHFST (Box format) handler
      zip.rs          #   ZHFST (ZIP format) handler
      meta.rs         #   Archive metadata (XML -> JSON)
      error.rs        #   Archive error types
      mod.rs
    transducer/       # FST engine
      hfst/           #   HFST-format transducer reader
      thfst/          #   THFST (Tromso-Helsinki FST) reader
        chunked.rs    #     Chunked data loading
        index_table.rs#     State index lookup
        transition_table.rs # Transition storage
        mod.rs
      alphabet.rs     #   Symbol table / alphabet management
      symbol_transition.rs # State transition with symbols
      tree_node.rs    #   Tree-based traversal nodes
      convert.rs      #   Format conversion (HFST -> THFST)
      mod.rs
    speller/          # Spelling engine
      suggestion.rs   #   Suggestion generation (parallel)
      worker.rs       #   Worker for parallel processing
      mod.rs
    tokenizer/        # Word boundary detection
    ffi/              # C FFI bindings
    vfs.rs            # Virtual filesystem abstraction
    constants.rs
    paths.rs
    types.rs
    lib.rs
  cli/                # CLI tool (divvunspell)
  crates/
    accuracy/         # Accuracy testing framework
    thfst-tools/      # Format conversion CLI
```

### 1.3 Key Architecture Patterns

**Trait-based VFS abstraction** (`vfs.rs`): divvunspell defines `Filesystem` and `File` traits that
abstract over native filesystem (with `memmap2`) and archive-based access (BHFST box format). This
enables the same transducer code to work with memory-mapped files on native platforms and in-memory
buffers for other contexts. Key methods:

- `Filesystem::open_file()` / `copy_to_temp_dir()`
- `File::len()` / `read_at()` / `memory_map()` / `partial_memory_map()`

**Two transducer formats**: HFST (standard Helsinki FST) and THFST (Tromso-Helsinki FST, a
byte-aligned format optimized for ARM and memory mapping). THFST uses `index_table.rs` and
`transition_table.rs` for efficient state/transition lookup -- directly analogous to libvoikko's
VFST format.

**Archive abstraction**: ZHFST archives (ZIP containing acceptor.default.hfst, errmodel.default.hfst,
index.xml) can be loaded directly or converted to BHFST (Box format wrapping THFST files with JSON
metadata) for faster loading.

**Speller API pattern**:

```rust
let archive = ZipSpellerArchive::open("language.zhfst")?;
let speller = archive.speller();
let is_correct = speller.is_correct("word");
let suggestions = speller.suggest("wordd");
```

### 1.4 Key Dependencies

| Crate | Purpose | Relevance to libvoikko |
|-------|---------|----------------------|
| `memmap2` | Memory-mapped file I/O | Native builds (not WASM) |
| `unic-segment`, `unic-char-range`, `unic-ucd-category` | Unicode text processing | Equivalent to libvoikko's character/ module |
| `smol_str` | Small string optimization | Useful for symbol table strings |
| `strsim` | String similarity metrics | Suggestion ranking |
| `hashbrown` | Fast hash maps | Symbol table lookups |
| `zip` (with deflate) | ZHFST archive reading | Not needed for VFST (voikko-fi uses flat files) |
| `flatbuffers` | Efficient serialization | THFST format |
| `serde` / `serde_json` | Metadata handling | Dictionary index.txt parsing |
| `parking_lot` | Concurrency primitives | Parallel suggestions (not for WASM) |

### 1.5 Lessons for libvoikko Port

1. **VFS abstraction is essential**: divvunspell's `vfs.rs` pattern of abstracting filesystem access
   enables the same transducer code to work with mmap (native) and `Vec<u8>` (WASM). Libvoikko
   should adopt the same pattern.

2. **Separate format from engine**: divvunspell cleanly separates format reading (`transducer/thfst/`)
   from traversal logic (`transducer/mod.rs`). This makes it possible to support multiple formats
   without changing the speller.

3. **Parallel suggestions are native-only**: Parallel processing via `parking_lot` is only relevant
   for native builds. In WASM, JavaScript's single-threaded model means suggestions run sequentially.
   Use `#[cfg(not(target_arch = "wasm32"))]` to gate parallel code.

4. **Unicode crates over custom character handling**: divvunspell uses the `unic-*` family rather
   than hand-rolled Unicode tables. Libvoikko's `character/` module could be similarly replaced,
   though Finnish-specific character classification may need custom logic.

5. **Release profile optimizations**: divvunspell uses fat LTO, single codegen unit, and debug
   symbol retention in release builds.

---

## 2. wasm-bindgen API Patterns

### 2.1 Exporting Structs and Methods

The fundamental pattern for exposing a Rust struct to JavaScript:

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Voikko {
    // Fields are private to JS (only pub(crate) or private)
    handle: VoikkoHandle,
}

#[wasm_bindgen]
impl Voikko {
    /// Constructor: callable as `new Voikko(dictData)` in JS
    #[wasm_bindgen(constructor)]
    pub fn new(dict_data: &[u8]) -> Result<Voikko, JsError> {
        let handle = VoikkoHandle::from_bytes(dict_data)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(Voikko { handle })
    }

    /// Method: callable as `voikko.spell("word")` in JS
    pub fn spell(&self, word: &str) -> bool {
        self.handle.spell(word)
    }

    /// Returns Vec<String> -> JS Array<string> (automatic conversion)
    pub fn suggest(&self, word: &str) -> Vec<String> {
        self.handle.suggest(word)
    }

    /// Explicit cleanup (Rust Drop runs when JS calls .free())
    pub fn terminate(&mut self) {
        // Release internal resources if needed
    }
}
```

**Key rules:**
- Struct fields cannot be `pub` if they are non-Copy types (String, Vec, etc.)
- `#[wasm_bindgen(constructor)]` enables `new ClassName()` in JS
- Static methods (no `self`) become static functions on the JS class
- `&self` methods become regular methods; `&mut self` methods get runtime borrow checking

### 2.2 String Passing (Rust UTF-8 <-> JS UTF-16)

wasm-bindgen handles string conversion automatically:

```rust
#[wasm_bindgen]
pub fn process_text(input: &str) -> String {
    // `input` is already a valid Rust &str (UTF-8)
    // wasm-bindgen decoded from JS UTF-16 automatically
    let result = format!("Processed: {}", input);
    // Return String -> wasm-bindgen encodes to JS string (UTF-16)
    result
}
```

**Performance considerations:**
- Each string crossing the boundary requires a copy and encoding conversion
- For hot paths (e.g., spell-checking individual words), this overhead is small
- For bulk operations, prefer passing a single large string and processing in Rust
- `&str` parameters are borrowed (no allocation); `String` parameters take ownership

### 2.3 Returning Complex Types with serde-wasm-bindgen

For complex return types like `Analysis` objects, use `serde-wasm-bindgen`:

```rust
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[derive(Serialize)]
pub struct AnalysisResult {
    pub baseform: String,
    pub word_class: String,
    pub case: Option<String>,
    pub number: Option<String>,
}

#[wasm_bindgen]
impl Voikko {
    /// Returns complex objects via serde serialization
    pub fn analyze(&self, word: &str) -> Result<JsValue, JsError> {
        let analyses: Vec<AnalysisResult> = self.handle.analyze(word);
        serde_wasm_bindgen::to_value(&analyses)
            .map_err(|e| JsError::new(&e.to_string()))
    }
}
```

**serde-wasm-bindgen vs. JSON:**
- `serde-wasm-bindgen` converts directly to JS native types (Map, Array, etc.)
- Smaller code size overhead than JSON serialization
- HashMap -> JS Map, Vec -> JS Array, Option::None -> undefined
- Use `Serializer::json_compatible()` if JSON semantics are required

### 2.4 Error Handling (Result -> JsValue)

```rust
use wasm_bindgen::prelude::*;

// Custom error type
#[derive(Debug)]
pub enum VoikkoError {
    DictionaryNotFound(String),
    InvalidFormat(String),
    AnalysisFailed(String),
}

impl std::fmt::Display for VoikkoError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            VoikkoError::DictionaryNotFound(p) => write!(f, "Dictionary not found: {}", p),
            VoikkoError::InvalidFormat(m) => write!(f, "Invalid format: {}", m),
            VoikkoError::AnalysisFailed(m) => write!(f, "Analysis failed: {}", m),
        }
    }
}

impl std::error::Error for VoikkoError {}

// JsError converts any std::error::Error automatically
#[wasm_bindgen]
impl Voikko {
    #[wasm_bindgen(constructor)]
    pub fn new(dict_data: &[u8]) -> Result<Voikko, JsError> {
        // ? operator propagates errors as JS exceptions
        let handle = VoikkoHandle::from_bytes(dict_data)?;
        Ok(Voikko { handle })
    }
}
```

**Pattern:** Use `JsError` (not `JsValue`) for error returns. `JsError` implements
`From<E> where E: std::error::Error`, so `?` works with any standard Rust error type.
On the JS side, this becomes a thrown `Error` with the message from `Display`.

### 2.5 Memory Management

**Ownership model:**
- Rust structs marked `#[wasm_bindgen]` are stored in a slab on the WASM side
- JS gets an opaque handle (index into the slab)
- Calling `.free()` on the JS object deallocates the Rust struct and nulls the handle
- Subsequent use after `.free()` panics in Rust (converted to JS exception)
- If `.free()` is never called, the Rust struct is leaked (no GC integration)

**Best practices:**
- Always provide a `terminate()` or `free()` method for resource cleanup
- Document that the object must not be used after calling free/terminate
- For long-lived objects (like `Voikko`), consider a `using`/`try-finally` pattern in JS
- Avoid holding `&mut self` across async boundaries -- this causes runtime borrow panics

**FinalizationRegistry pattern** (for automatic cleanup):

```typescript
// TypeScript wrapper can add automatic cleanup
const registry = new FinalizationRegistry((ptr: number) => {
  // Call .free() on the Rust object when JS GC collects the wrapper
});

class Voikko {
  private inner: WasmVoikko;
  constructor(dictData: Uint8Array) {
    this.inner = new WasmVoikko(dictData);
    registry.register(this, this.inner.__wbg_ptr);
  }
}
```

### 2.6 Async Patterns

Rust async functions can be exported to JS as Promise-returning functions:

```rust
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::Response;

#[wasm_bindgen]
pub async fn load_dictionary(url: &str) -> Result<Voikko, JsError> {
    let window = web_sys::window().unwrap();
    let resp: Response = JsFuture::from(window.fetch_with_str(url))
        .await?
        .dyn_into()?;
    let buf: js_sys::ArrayBuffer = JsFuture::from(resp.array_buffer()?)
        .await?
        .dyn_into()?;
    let data = js_sys::Uint8Array::new(&buf).to_vec();
    Voikko::new(&data)
}
```

**Caveats:**
- `wasm-bindgen-futures` bridges JS Promises and Rust Futures via `spawn_local`
- WASM is single-threaded: async does NOT mean parallel execution
- Avoid holding `&mut self` references across `.await` points -- this causes
  "recursive use of an object" panics if JS calls another method concurrently
- Prefer: fetch dict in JS, pass bytes to Rust constructor (simpler, fewer deps)

---

## 3. Build Toolchain Recommendation

### 3.1 Post-wasm-pack Landscape

The rustwasm working group and wasm-pack were officially archived in July 2025. The `wasm-bindgen`
repository was transferred to a new `wasm-bindgen` organization with maintainers @daxpedda and
@guybededd (Cloudflare). wasm-bindgen itself continues to be actively maintained.

**Recommended approach: manual pipeline** (no wasm-pack dependency):

```bash
# Step 1: Build the WASM binary
cargo build --target wasm32-unknown-unknown --release

# Step 2: Generate JS/TS bindings
wasm-bindgen target/wasm32-unknown-unknown/release/voikko_wasm.wasm \
  --out-dir pkg \
  --target bundler \
  --typescript

# Step 3: Optimize the WASM binary
wasm-opt pkg/voikko_wasm_bg.wasm -Oz -o pkg/voikko_wasm_bg.wasm

# Step 4 (optional): Strip debug names for production
wasm-strip pkg/voikko_wasm_bg.wasm
```

**Why not wasm-pack:**
- Archived, no longer maintained
- Serial pipeline (cargo build + wasm-bindgen + wasm-opt) cannot be customized
- Version pinning of wasm-bindgen-cli is fragile
- Manual pipeline gives full control and better IDE integration

**Tool installation:**

```bash
# wasm-bindgen-cli: MUST match the wasm-bindgen crate version exactly
cargo install wasm-bindgen-cli --version "0.2.100"

# wasm-opt: from binaryen
brew install binaryen    # macOS
# or: cargo install wasm-opt

# wasm-strip: from WABT (optional)
brew install wabt
```

### 3.2 Recommended Cargo.toml

```toml
[package]
name = "voikko-wasm"
version = "0.1.0"
edition = "2024"
rust-version = "1.93"

[lib]
crate-type = ["cdylib", "rlib"]
# cdylib: produces .wasm for wasm-bindgen
# rlib:   allows native tests and benchmarks

[dependencies]
wasm-bindgen = "0.2"
serde = { version = "1", features = ["derive"] }
serde-wasm-bindgen = "0.6"
# js-sys and web-sys only if browser APIs are needed:
# js-sys = "0.3"
# web-sys = { version = "0.3", features = ["console"] }

[dev-dependencies]
wasm-bindgen-test = "0.3"

[features]
default = ["spell", "suggest", "analyze", "hyphenate", "grammar"]
spell = []
suggest = ["spell"]
analyze = []
hyphenate = ["analyze"]
grammar = ["analyze"]
tokenize = []

[profile.release]
opt-level = "z"         # Optimize for size (try "s" too, measure both)
lto = true              # Fat LTO: aggressive cross-crate optimization
codegen-units = 1       # Single codegen unit: slower build, smaller binary
strip = true            # Strip debug symbols
panic = "abort"         # No unwinding: smaller binary (WASM has no catch)

[profile.release.package."*"]
opt-level = "z"         # Also optimize dependencies for size
```

**Feature flags enable modular builds:**

```bash
# Spell-check only (smallest binary)
cargo build --target wasm32-unknown-unknown --release --no-default-features --features spell

# Spell + suggest
cargo build --target wasm32-unknown-unknown --release --no-default-features --features suggest

# Full build (all features)
cargo build --target wasm32-unknown-unknown --release
```

### 3.3 Bundle Size Optimization

**Build-time optimizations (in Cargo.toml above):**
- `opt-level = "z"`: optimize for code size
- `lto = true`: link-time optimization enables aggressive dead code elimination
- `codegen-units = 1`: single compilation unit allows more inlining opportunities
- `panic = "abort"`: removes unwinding machinery (~10-20 KB savings)
- `strip = true`: removes debug info from the binary

**Post-build optimizations:**
- `wasm-opt -Oz`: binaryen optimizer, typically 15-20% additional size reduction
- `wasm-strip`: removes remaining name sections
- `wasm-snip`: replace known-unused functions with `unreachable` (use with `twiggy` analysis)

**Code-level optimizations:**
- Avoid monomorphization bloat: use `dyn Trait` for rarely-called paths
- Use `#[inline(never)]` on cold error-handling paths
- Prefer concrete types over generics for infrequently-used functions
- Minimize `format!()` usage (pulls in formatting machinery)

**Expected sizes (based on divvunspell and similar projects):**

| Component | Estimated .wasm | gzipped |
|-----------|----------------|---------|
| Spell-check only | 150-250 KB | 60-100 KB |
| Spell + suggest | 200-350 KB | 80-140 KB |
| Full (all features) | 300-500 KB | 120-200 KB |
| Dictionary data | ~3.9 MB | ~1.5 MB |

### 3.4 wasm-bindgen Target Modes

| Target | Use Case | Output |
|--------|----------|--------|
| `--target bundler` | Webpack/Vite/Rollup | ESM with import for .wasm |
| `--target web` | Direct `<script type="module">` | ESM with manual init() |
| `--target nodejs` | Node.js server-side | CJS with require() |
| `--target no-modules` | Legacy browsers | Global variable |

**Recommendation:** Use `--target bundler` for the primary build (works with all modern bundlers),
and `--target web` for CDN/direct browser use. The existing TypeScript wrapper (`libvoikko/js/`)
can abstract over the initialization differences.

---

## 4. Binary Parsing Approach

### 4.1 VFST Format Requirements

The VFST binary format (documented in `02-fst-engine.md`) has these characteristics:
- Fixed-size header (16 bytes)
- Variable-length symbol table (null-terminated UTF-8 strings)
- Alignment padding
- Dense array of fixed-size transitions (8 bytes unweighted, 16 bytes weighted)
- Little-endian by default (WASM is always LE)

### 4.2 Library Comparison

| Library | Approach | Pros | Cons | Fit for VFST |
|---------|----------|------|------|--------------|
| `zerocopy` | Zero-copy cast `&[u8]` -> `&Struct` | Fastest access, no parsing overhead | Requires fixed layout, no padding/alignment flexibility | Good for transitions |
| `bytemuck` | Similar to zerocopy, simpler API | Easy `#[repr(C)]` struct casting | Less validation than zerocopy | Good for transitions |
| `nom` | Parser combinators | Flexible, composable, zero-copy borrows | Overkill for fixed-format binary | Over-engineered |
| `binrw` (successor to binread) | Declarative derive macro | Very ergonomic for complex formats | Runtime overhead from validation | Reasonable |
| `byteorder` | Read/write with explicit endianness | Simple, well-known | Manual, verbose | Fine for header |
| Manual `&[u8]` slicing | Direct byte manipulation | Full control, zero overhead | Verbose, error-prone | Pragmatic choice |

### 4.3 Recommended Approach: Hybrid

For VFST parsing, use a layered strategy:

**Header + Symbol Table** (parsed once at load time): Manual `&[u8]` slicing with `byteorder` or
Rust's native `u16::from_le_bytes()` / `u32::from_le_bytes()`. The header is only 16 bytes and the
symbol table is variable-length, so zero-copy casting is not applicable.

```rust
fn parse_header(data: &[u8]) -> Result<VfstHeader, VfstError> {
    if data.len() < 16 {
        return Err(VfstError::TooShort);
    }
    let cookie1 = u32::from_le_bytes(data[0..4].try_into().unwrap());
    let cookie2 = u32::from_le_bytes(data[4..8].try_into().unwrap());
    if cookie1 != VFST_COOKIE1 || cookie2 != VFST_COOKIE2 {
        return Err(VfstError::InvalidMagic);
    }
    let weighted = data[8] == 0x01;
    Ok(VfstHeader { weighted })
}
```

**Transition table** (accessed millions of times during traversal): Zero-copy casting with
`bytemuck` or `zerocopy`. The transition structs are `#[repr(C)]` with fixed sizes (8 or 16 bytes),
making them ideal candidates for zero-copy reinterpretation.

```rust
use bytemuck::{Pod, Zeroable};

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct Transition {
    sym_in: u16,
    sym_out: u16,
    trans_info: u32,
}

impl Transition {
    fn target_state(&self) -> u32 { self.trans_info & 0x00FF_FFFF }
    fn more_transitions(&self) -> u8 { (self.trans_info >> 24) as u8 }
}

// Zero-copy access to transition table
fn transitions(data: &[u8], offset: usize) -> &[Transition] {
    let slice = &data[offset..];
    bytemuck::cast_slice(slice)
}
```

### 4.4 mmap Alternatives in WASM

| Context | Strategy |
|---------|----------|
| WASM (browser) | `fetch()` -> `ArrayBuffer` -> `Uint8Array` passed to Rust as `&[u8]` or `Vec<u8>` |
| WASM (Node.js) | `fs.readFileSync()` -> `Buffer` -> `Uint8Array` -> Rust `Vec<u8>` |
| Native (benchmark/test) | `memmap2::Mmap` for OS-level memory mapping |

For WASM, the entire dictionary file lives in WASM linear memory as a `Vec<u8>`. This is equivalent
to mmap in terms of access patterns -- the transition table can be accessed via pointer/offset
arithmetic with zero-copy casting.

**Platform abstraction:**

```rust
pub enum TransducerStorage {
    Owned(Vec<u8>),
    #[cfg(not(target_arch = "wasm32"))]
    Mapped(memmap2::Mmap),
}

impl AsRef<[u8]> for TransducerStorage {
    fn as_ref(&self) -> &[u8] {
        match self {
            TransducerStorage::Owned(v) => v,
            #[cfg(not(target_arch = "wasm32"))]
            TransducerStorage::Mapped(m) => m,
        }
    }
}
```

### 4.5 Endianness Handling

WASM is always little-endian. The VFST format is natively little-endian. Therefore:

- **WASM builds**: No byte-swapping needed. Use `zerocopy`/`bytemuck` for direct struct casting.
- **Native builds**: Check endianness at load time. If big-endian, swap bytes during loading (as the
  C++ code already does). Gate with `#[cfg(target_endian = "big")]`.
- **Simplification**: If the Rust port targets WASM as primary, consider dropping big-endian support
  entirely and document that dictionaries must be LE. This removes ~80 lines of byte-swap code.

---

## 5. Rust FST and NLP Crates

### 5.1 BurntSushi `fst` Crate

The `fst` crate provides compact ordered sets and maps using finite state automata. It supports
regular expression and Levenshtein automaton queries over sorted byte-string keys with u64 values.

**Why it does NOT fit VFST:**
- Designed for immutable key-value sets, not transducers with input/output symbol pairs
- Keys are byte strings, not symbol-indexed transitions
- No flag diacritic support
- No weighted transitions
- Different binary format entirely

**Potential use:** Could be useful as an auxiliary data structure for dictionary lookups or
autocomplete, but not as a replacement for the VFST engine.

### 5.2 `rustfst` Crate

`rustfst` is a Rust reimplementation of OpenFST for constructing, combining, optimizing, and
searching weighted finite-state transducers. It supports determinization, minimization, composition,
and shortest-path algorithms.

**Why it does NOT fit VFST:**
- General-purpose weighted transducer library with a different binary format
- Much heavier than what libvoikko needs (full FST algebra operations)
- Not designed for the specific VFST binary format

**Potential use:** Reference implementation for understanding weighted transducer algorithms.
The composition and shortest-path algorithms could inform the suggestion engine design.

### 5.3 Recommended NLP/Unicode Crates

| Crate | Purpose | Use in libvoikko |
|-------|---------|-----------------|
| `unicode-segmentation` | UAX#29 grapheme/word/sentence boundaries | Replace tokenizer/ character boundary detection |
| `unicode-normalization` | NFC/NFD/NFKC/NFKD normalization | Input normalization (voikko_normalise) |
| `unicode-categories` or `unicode-general-category` | General category lookup | Replace character/charset.hpp char_type detection |
| `smol_str` | Inline small strings (up to 22 bytes) | Symbol table storage (most symbols are short) |
| `hashbrown` | Fast HashMap (Robin Hood hashing) | char-to-symbol lookup table |
| `tinyvec` | Stack-allocated small vectors | Suggestion lists, analysis results |
| `compact_str` | Compact string representation | Alternative to smol_str |

**Finnish-specific character handling**: The `character/SimpleChar.hpp` module has Finnish-specific
case conversion rules (e.g., dotted I handling). Standard `char::to_uppercase()` in Rust handles
Unicode correctly, but some libvoikko-specific rules (like `voikko_normalise`) may need custom
logic. Evaluate whether `unicode-normalization`'s NFC/NFKC covers the needed normalization, and
add Finnish-specific fixups as needed.

---

## 6. Integration with Existing TypeScript Wrapper

### 6.1 Current Architecture

The existing JS/WASM package (`libvoikko/js/`) has this structure:

```
TypeScript Wrapper (Voikko class)
    |
    v
RawVoikkoInstance (from libvoikko_api.js cwrap)
    |
    v
Emscripten WASM Module (C++ compiled)
    |
    v
VFST Dictionary (mounted via Emscripten VFS)
```

The `Voikko` class in `src/index.ts` is a thin wrapper over `RawVoikkoInstance`. The actual API
surface is defined in `types.ts` with 15 methods (spell, suggest, analyze, tokens, sentences,
grammarErrors, getHyphenationPattern, hyphenate, attributeValues, plus option setters).

### 6.2 Migration Strategy: Transparent Replacement

The ideal approach is a **transparent replacement** of the Emscripten WASM module with a Rust WASM
module, keeping the TypeScript `Voikko` class API unchanged.

**Phase 1: Rust WASM module exposes the same interface as RawVoikkoInstance**

```rust
#[wasm_bindgen]
pub struct WasmVoikko { /* ... */ }

#[wasm_bindgen]
impl WasmVoikko {
    #[wasm_bindgen(constructor)]
    pub fn new(dict_data: &[u8], language: &str) -> Result<WasmVoikko, JsError> { ... }

    pub fn spell(&self, word: &str) -> bool { ... }
    pub fn suggest(&self, word: &str) -> Vec<String> { ... }
    pub fn analyze(&self, word: &str) -> JsValue { ... }  // serde-wasm-bindgen
    pub fn tokens(&self, text: &str) -> JsValue { ... }
    pub fn sentences(&self, text: &str) -> JsValue { ... }
    pub fn grammar_errors(&self, text: &str, language: &str) -> JsValue { ... }
    pub fn get_hyphenation_pattern(&self, word: &str) -> String { ... }
    pub fn hyphenate(&self, word: &str, separator: Option<String>,
                     allow_context_changes: Option<bool>) -> String { ... }
    pub fn attribute_values(&self, attr: &str) -> JsValue { ... }

    // Option setters
    pub fn set_ignore_dot(&mut self, value: bool) { ... }
    pub fn set_ignore_numbers(&mut self, value: bool) { ... }
    // ... (14 total option setters)

    pub fn terminate(&mut self) { ... }
}
```

**Phase 2: Update wasm-loader.ts to load Rust WASM instead of Emscripten**

The `wasm-loader.ts` currently handles:
1. `loadWasm()`: loads Emscripten module factory
2. `loadDict()`: fetches dictionary files
3. `mountDict()`: writes dict to Emscripten VFS

With Rust WASM, this simplifies to:
1. `loadWasm()`: load the wasm-bindgen output module
2. `loadDict()`: fetch dictionary files (same as before, returns `Uint8Array`)
3. Dictionary is passed directly to `WasmVoikko` constructor (no VFS mount needed)

**Phase 3: Voikko class adapts minimally**

```typescript
// Updated init() -- dictionary bytes passed to Rust constructor
static async init(language: string = 'fi', options: VoikkoInitOptions = {}): Promise<Voikko> {
    const [wasmModule, dictData] = await Promise.all([
        loadRustWasm(options.locateFile),
        loadDict(options),
    ]);
    await wasmModule.default(); // Initialize WASM
    const raw = new wasmModule.WasmVoikko(dictData, language);
    return new Voikko(raw);
}
```

### 6.3 API Boundary Design

**Recommended boundary: thin TS wrapper over Rust WASM**

| Layer | Responsibility |
|-------|---------------|
| TypeScript wrapper (`Voikko` class) | API ergonomics, type safety, dictionary loading, WASM init |
| Rust WASM (`WasmVoikko`) | All NLP logic: FST traversal, spell check, suggest, analyze, etc. |
| Dictionary data | Passed as `Uint8Array` from TS to Rust constructor |

**What stays in TypeScript:**
- Dictionary fetching (browser `fetch()` / Node.js `fs.readFile`)
- WASM module loading and initialization
- API type definitions (for IDE support)
- FinalizationRegistry for automatic cleanup (optional)

**What moves to Rust:**
- All FST engine logic
- All NLP processing (spell, suggest, analyze, hyphenate, grammar)
- Dictionary format parsing (VFST binary -> in-memory structures)
- Character classification and case conversion
- Tokenization and sentence detection

### 6.4 Backward Compatibility

The `Voikko` class API (`spell()`, `suggest()`, `analyze()`, etc.) remains identical. Consumers
of the npm package see no change. The only breaking change is in initialization:
- Old: Emscripten VFS mounting with separate files
- New: Single `Uint8Array` dictionary bundle (or keep multiple files and concatenate in TS)

To avoid breaking changes in dictionary loading, the TypeScript wrapper can maintain both
`dictionaryUrl` and `dictionaryPath` options, fetching the same files and assembling them
into the byte array format that Rust expects.

---

## 7. Testing Strategy

### 7.1 Test Layers

```
Layer 1: Rust Native Tests (cargo test)
  - Unit tests for each module (FST parser, traversal, spell, etc.)
  - Integration tests with test .vfst files
  - Property-based tests with proptest
  - No WASM involved -- fast iteration

Layer 2: WASM Integration Tests (wasm-bindgen-test)
  - Test that wasm-bindgen exports work correctly
  - Verify string passing, complex return types, error handling
  - Run in Node.js or headless browser

Layer 3: TypeScript Wrapper Tests (vitest)
  - Test the full stack: TS -> WASM -> Rust
  - Reuse existing voikko.test.ts test cases
  - Verify backward compatibility

Layer 4: Differential Tests
  - Compare Rust output vs. C++ output for the same dictionary
  - Automated: run both implementations on word lists, diff results
```

### 7.2 Rust Native Tests (Layer 1)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vfst_header() {
        let data = include_bytes!("../test-data/test.vfst");
        let header = parse_header(data).unwrap();
        assert!(!header.weighted);
    }

    #[test]
    fn test_spell_basic() {
        let dict = include_bytes!("../test-data/mor.vfst");
        let voikko = VoikkoHandle::from_mor_vfst(dict).unwrap();
        assert!(voikko.spell("koira"));
        assert!(!voikko.spell("koirra"));
    }

    #[test]
    fn test_flag_diacritics_all_ops() {
        // Test P (set), C (clear), U (unify), R (require), D (disallow)
        let mut flags = FlagState::new(3); // 3 features
        assert!(check_flag(&mut flags, OpFeatureValue { op: FlagOp::P, feature: 0, value: 2 }));
        assert_eq!(flags.get(0), 2);
        assert!(check_flag(&mut flags, OpFeatureValue { op: FlagOp::R, feature: 0, value: 2 }));
        assert!(!check_flag(&mut flags, OpFeatureValue { op: FlagOp::D, feature: 0, value: 2 }));
    }
}
```

### 7.3 Property-Based Tests with proptest

```rust
use proptest::prelude::*;

proptest! {
    // Any valid Finnish word should produce at least one analysis
    #[test]
    fn analyze_known_words_nonempty(word in known_finnish_words()) {
        let voikko = test_voikko_instance();
        let analyses = voikko.analyze(&word);
        prop_assert!(!analyses.is_empty(), "No analysis for known word: {}", word);
    }

    // spell(word) should be consistent with analyze(word)
    #[test]
    fn spell_consistent_with_analyze(word in any_string_strategy()) {
        let voikko = test_voikko_instance();
        let is_correct = voikko.spell(&word);
        let analyses = voikko.analyze(&word);
        if is_correct {
            prop_assert!(!analyses.is_empty());
        }
    }

    // Transducer traversal should never exceed loop limit silently
    #[test]
    fn traversal_terminates(input in "[a-z]{1,50}") {
        let transducer = test_transducer();
        let mut config = TraversalConfig::new(2000, transducer.flag_feature_count());
        transducer.prepare(&mut config, &input);
        let mut output = String::new();
        let mut count = 0;
        while transducer.next(&mut config, &mut output) {
            count += 1;
            output.clear();
            prop_assert!(count <= 1000, "Too many results for input: {}", input);
        }
    }
}
```

### 7.4 WASM Integration Tests (Layer 2)

```rust
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_wasm_spell() {
    let dict_data: &[u8] = include_bytes!("../test-data/mor.vfst");
    let voikko = WasmVoikko::new(dict_data, "fi").unwrap();
    assert!(voikko.spell("koira"));
    assert!(!voikko.spell("koirra"));
}

#[wasm_bindgen_test]
fn test_wasm_suggest_returns_array() {
    let dict_data: &[u8] = include_bytes!("../test-data/mor.vfst");
    let voikko = WasmVoikko::new(dict_data, "fi").unwrap();
    let suggestions = voikko.suggest("koirra");
    assert!(!suggestions.is_empty());
}

#[wasm_bindgen_test]
fn test_wasm_analyze_returns_valid_js() {
    let dict_data: &[u8] = include_bytes!("../test-data/mor.vfst");
    let voikko = WasmVoikko::new(dict_data, "fi").unwrap();
    let result = voikko.analyze("koiralle");
    // Result is a JsValue (Array of objects)
    assert!(result.is_array());
}
```

**Running WASM tests:**

```bash
# In Node.js (default)
cargo test --target wasm32-unknown-unknown

# In headless Chrome
WASM_BINDGEN_USE_BROWSER=1 cargo test --target wasm32-unknown-unknown

# With specific browser
WASM_BINDGEN_USE_BROWSER=1 CHROMEDRIVER=/usr/bin/chromedriver \
  cargo test --target wasm32-unknown-unknown
```

### 7.5 Differential Testing (Layer 4)

The most critical validation is ensuring the Rust implementation produces identical results to C++.

**Strategy:**
1. Build a word list from Finnish text corpora (10,000+ words)
2. Run each word through both C++ libvoikko and Rust voikko-wasm
3. Compare: spell results, analysis output, hyphenation patterns, suggestions
4. Any differences are logged and investigated

**Implementation:**

```python
# differential_test.py
import libvoikko  # C++ via Python bindings
import subprocess
import json

voikko_cpp = libvoikko.Voikko("fi")

with open("wordlist.txt") as f:
    words = [line.strip() for line in f]

# Run Rust implementation (via Node.js wrapper or native CLI)
rust_results = json.loads(subprocess.check_output(
    ["node", "test-runner.mjs", json.dumps(words)]
))

mismatches = []
for word in words:
    cpp_spell = voikko_cpp.spell(word)
    rust_spell = rust_results[word]["spell"]
    if cpp_spell != rust_spell:
        mismatches.append((word, "spell", cpp_spell, rust_spell))

    cpp_analyze = voikko_cpp.analyze(word)
    rust_analyze = rust_results[word]["analyze"]
    # Compare analysis results...

print(f"Mismatches: {len(mismatches)} / {len(words)}")
for m in mismatches[:20]:
    print(f"  {m}")
```

### 7.6 Test Data Strategy

- Use `voikkovfstc` (from C++ `tools/`) to compile small ATT-format transducers into test .vfst files
- Create minimal test dictionaries that exercise specific features:
  - Flag diacritics (all 5 operations)
  - Multi-character symbols
  - Overflow cells (states with 255+ transitions)
  - Weighted vs. unweighted transitions
  - Edge cases: empty input, very long input, unknown characters
- Include the real `voikko-fi` dictionary for integration tests (guarded by feature flag or env var)

---

## 8. Summary and Recommendations

### 8.1 Architecture Decision Record

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Build tool | Manual pipeline (cargo + wasm-bindgen-cli + wasm-opt) | wasm-pack archived; manual gives full control |
| JS binding | wasm-bindgen | Industry standard, active maintenance, TS type generation |
| Complex types | serde-wasm-bindgen | Native JS types, smaller than JSON, officially recommended |
| Binary parsing | bytemuck for transitions, manual for header/symbols | Zero-copy for hot path, simple code for cold path |
| String handling | Rust `String`/`&str` internally, wasm-bindgen at boundary | UTF-8 everywhere, no wchar_t |
| Dictionary loading | `&[u8]` passed from JS | No VFS needed, simpler than Emscripten mount |
| Feature modularity | Cargo feature flags | Selective builds for different use cases |
| Error handling | `Result<T, JsError>` | Automatic conversion from std::error::Error |
| Testing | 4-layer: native + wasm + TS + differential | Comprehensive coverage at each level |

### 8.2 Risk Mitigation

| Risk | Mitigation |
|------|-----------|
| Behavior divergence from C++ | Differential testing against full word lists |
| WASM bundle too large | Feature flags for modular builds; wasm-opt -Oz; twiggy profiling |
| wasm-bindgen breaking changes | Pin exact version; monitor new org's releases |
| Memory leaks in JS | FinalizationRegistry; document .terminate() requirement |
| Async borrow panics | No &mut self across await; use interior mutability (RefCell) if needed |
| Finnish-specific Unicode issues | Comprehensive test suite with Finnish-specific edge cases |

### 8.3 Implementation Priority

```
Phase 1: FST Engine (Rust native)
  - VFST binary parser
  - Unweighted traversal
  - Weighted traversal
  - Flag diacritics
  -> Validate with differential tests against C++

Phase 2: Spell Check MVP (Rust WASM)
  - character/ and utils/ modules
  - FinnishVfstAnalyzer
  - AnalyzerToSpellerAdapter
  - Basic spell() API via wasm-bindgen
  -> First working WASM binary

Phase 3: Full API
  - suggest(), analyze(), hyphenate(), tokens(), sentences()
  - Grammar checking (most complex)
  - Option setters
  -> Feature-complete replacement

Phase 4: Integration
  - Update TypeScript wrapper
  - Backward compatibility tests
  - npm package release
  -> Drop-in replacement for Emscripten build
```
