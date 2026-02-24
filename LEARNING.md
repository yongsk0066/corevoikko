# Learning Guide

This document helps newcomers build the background knowledge needed to work on Corevoikko. Read this before diving into the code.

The topics are ordered so that each section motivates the next: first the problem, then the solution, then the implementation.


## 1. Agglutinative Languages and the Spell Checking Problem

Before looking at any code, understand the problem this project solves.

### What you need to know

Most English spell checkers use a word list: look up the word, if it's in the list, it's correct. This works because English has relatively few forms per word (run, runs, running, ran — maybe 4-5 forms).

Finnish is an **agglutinative language**. Words are built by chaining a root with suffixes — case markers, plural markers, possessive suffixes, clitics — in sequence. A single noun like "talo" (house) has over 2,000 valid inflected forms. Verbs have even more. Compound words multiply this further: "talonrakentaja" = talo + n + rakentaja (house builder), and each part inflects independently.

A word list approach would need tens of millions of entries and still miss valid compound words. Instead, Voikko uses **morphological analysis as spell checking**: a word is correct if it can be decomposed into a valid root + valid suffixes according to the rules of Finnish grammar. This decomposition is performed by a Finite State Transducer.

Other agglutinative languages (Turkish, Hungarian, Korean, Japanese) face the same challenge. The techniques in this project apply broadly, though the specific rules are Finnish.

### Why it matters

This is the core insight of the entire project: **spell checking = morphological analysis**. Every feature builds on this:

- `spell("kissoja")` → can the FST decompose this word? → yes → correct
- `analyze("kissoja")` → what are all valid decompositions? → noun "kissa", partitive, plural
- `suggest("kisssa")` → mutate characters, check which mutations the FST accepts
- `hyphenate("kissanpentu")` → find morpheme boundaries via analysis, insert hyphens there
- `grammarErrors(text)` → analyze each word, check if the sequence makes grammatical sense

### How to learn

```plaintext
What is an agglutinative language? How does it differ from isolating
languages (like English or Chinese) and fusional languages (like
Russian or Latin)? Give examples of each type. Why do agglutinative
languages make dictionary-based spell checking impractical?
```

```plaintext
How can morphological analysis be used as a spell checker? If I have a
system that can decompose a word into root + suffixes according to
grammar rules, how does that tell me whether the word is spelled
correctly? What are the advantages over a simple word list lookup?
```

**Recommended reading:**
- The Wikipedia article on [agglutinative languages](https://en.wikipedia.org/wiki/Agglutinative_language)
- Kenneth Beesley's overview of computational morphology


## 2. Finnish Morphology

Now that you know *why* morphological analysis matters, here's *what* Finnish morphology looks like.

### What you need to know

**Cases.** Finnish has 15 grammatical cases. A single noun like "talo" (house) can appear as: talo, talon, taloa, talossa, talosta, taloon, talolla, talolta, talolle, talona, taloksi, talotta, taloineen... Each form conveys a different grammatical meaning (location, direction, possession, etc.).

**Compound words.** Finnish freely forms compound words by joining roots: "lentokonesuihkuturbiinimoottori" = jet turbine engine. There is no predefined list of valid compounds — any semantically sensible combination is allowed.

**Stacking.** A single word can have: root + derivation suffix + plural marker + case suffix + possessive suffix + clitic particle. For example, "talossanikinko" = talo (house) + ssa (in) + ni (my) + kin (also) + ko (question) — "in my house too?"

**Voikko's morphological attributes.** When the analyzer decomposes a word, it produces key-value attributes:

- `BASEFORM` — dictionary form (e.g., "kissa" from "kissoilla")
- `CLASS` — part of speech: nimisana (noun), teonsana (verb), laatusana (adjective), etc.
- `SIJAMUOTO` — grammatical case: nimento (nominative), osanto (partitive), sisaolento (inessive), etc.
- `NUMBER` — singular or plural
- `STRUCTURE` — how the word is composed from morphemes

### How to learn

```plaintext
Explain Finnish noun inflection. Finnish has 15 grammatical cases —
list them with their Finnish names (nimento, omanto, osanto, etc.),
their linguistic names (nominative, genitive, partitive, etc.), and a
brief explanation of what each one expresses. Use "talo" (house) as the
example word and show the inflected form for each case in singular.
```

```plaintext
What are compound words in Finnish? How are they formed? Give 5 examples
of compound words, break each one into its component parts, and explain
the meaning. How would a spell checker need to handle compound words
differently from simple words?
```

```plaintext
Explain the difference between inflection and derivation in Finnish
morphology. Give examples of both. How do they interact — can a word
have both derivational and inflectional suffixes? In what order?
```

**Key files to read:**
- `libvoikko/rust/crates/voikko-fi/src/morphology/tag_parser.rs` — how FST output tags are parsed
- `libvoikko/rust/crates/voikko-core/src/analysis.rs` — the 21 attribute key constants
- `libvoikko/doc/morphological-analysis.txt` — full attribute reference


## 3. Finite State Transducers (FST)

This is the data structure that makes morphological analysis fast enough for real-time spell checking.

### What you need to know

A **Finite State Automaton (FSA)** is a directed graph that recognizes strings. You start at an initial state, follow edges labeled with characters, and check if you end up at an accepting state. This is the foundation of regular expressions.

A **Finite State Transducer (FST)** extends the FSA by adding an *output* label to each edge alongside the input label. When you traverse the graph, you collect output symbols along the way. So an FST doesn't just say "yes, this string is valid" — it also produces a result string describing *how* it's valid.

In Voikko:

```plaintext
Input:  k-i-s-s-o-j-a
Output: [Ln][Xp]kissa[X]kissoja[Spar][Nm]
```

This output tells us: it's a noun (`Ln`), the base form is "kissa" (`[Xp]kissa[X]`), in partitive case (`Spar`), plural (`Nm`). One input can have multiple valid paths, meaning a word can have multiple analyses.

The key property: an FST can encode millions of word forms in a compact graph (3.8MB for all of Finnish) because shared prefixes and suffixes share graph nodes. This is far smaller and faster than a word list.

**Flag diacritics** are a special mechanism: control symbols on edges that constrain which paths are valid. For example, a flag can enforce "this path is only valid if CASE=NOM was set earlier," preventing impossible suffix combinations. The five types are P (push), C (clear), U (unify), R (require), D (disallow).

### How to learn

```plaintext
Explain finite state automata (FSA) and finite state transducers (FST)
in simple terms. How do they differ from regular expressions? Give a
small example of an FST that maps input strings to output strings,
showing the state transitions step by step.
```

```plaintext
What does it mean for a language to be "regular"? Why can finite state
transducers handle morphological analysis of natural languages, even
though natural language is not strictly regular? What role do flag
diacritics play in extending FST power beyond strictly regular languages?
```

```plaintext
Explain the weighted vs unweighted variants of finite state transducers.
When would you use weights? How does adding weights change the traversal
algorithm (e.g., need for backtracking or priority queues)?
```

```plaintext
What are flag diacritics in finite state transducers? Explain the five
operations: P (positive set), C (clear), U (unification), R (require),
D (disallow). Give an example of how they constrain valid paths, such as
enforcing agreement between a noun's case and a suffix's case requirement.
```

**Recommended reading:**
- Beesley & Karttunen, *Finite State Morphology* (2003) — the definitive reference
- The [foma documentation](https://fomafst.github.io/) — the tool that compiles FST graphs
- `plan/phase2-rust/02-fst-engine.md` in this repo — details on the VFST binary format


## 4. Spelling Suggestions

Spell checking answers "is this word correct?" Suggestion generation answers "what did the user probably mean?" This is a separate and complex problem.

### What you need to know

When a word fails spell checking, Voikko generates correction candidates. The `suggestion/generators.rs` file (1,600 lines) implements multiple strategies:

- **Character-level edits** — substitute, insert, delete, or transpose adjacent characters, then check if the FST recognizes the result. This catches typos.
- **Word splitting** — try inserting a space at every position to see if the input is two valid words joined together ("taloauto" → "talo auto").
- **Keyboard layout awareness** — characters near each other on the keyboard are more likely typo substitutions.
- **OCR correction** — similar-looking characters (l/1, O/0) are likely OCR misreads.

Candidates are ranked by a priority system. The `SuggestionStrategy` type chains multiple generators together, and results are deduplicated and sorted before being returned.

There are two preset strategies: one optimized for typing errors (the default) and one optimized for OCR correction.

### How to learn

```plaintext
How do spell checkers generate spelling suggestions? Explain the concept
of edit distance (Levenshtein distance) and how it's used to find
candidate corrections. What is the typical approach: generate all
candidates within edit distance 1-2, then filter by dictionary lookup?
How do you rank multiple candidates?
```

```plaintext
What strategies exist for spelling suggestion beyond simple edit
distance? Explain keyboard-aware suggestions (using key proximity),
phonetic similarity, word splitting/joining, and context-aware ranking.
How do these complement each other?
```

**Key files to read:**
- `libvoikko/rust/crates/voikko-fi/src/suggestion/generators.rs` — all generator implementations
- `libvoikko/rust/crates/voikko-fi/src/suggestion/strategy.rs` — how generators are chained


## 5. The VFST Binary Format

Voikko uses its own binary format to store precompiled FST graphs. This is what `.vfst` files contain.

### What you need to know

A `.vfst` file is a compact binary dump of an FST graph:

1. A 16-byte **header** with magic bytes and a flag indicating weighted/unweighted
2. A **symbol table** mapping numeric indices to UTF-8 strings (characters, tags like `[Ln]`, flag diacritics like `@P.CASE.NOM@`)
3. A **transition table** — an array of fixed-size entries, each describing one edge (input symbol, output symbol, target state)

The `voikko-fst` crate loads this binary into memory and provides a traversal API. Transitions are mapped via zero-copy `bytemuck` casting for performance.

### When you need this

Only if you're working on the FST engine (`voikko-fst`). For higher-level work, treat `.vfst` files as opaque input.

### How to learn

```plaintext
I'm studying a custom binary format for finite state transducers called
VFST. It has a 16-byte header (8-byte magic, 1-byte weighted flag,
7 reserved), followed by a symbol table (2-byte count + null-terminated
UTF-8 strings), padding for alignment, and then an array of transition
entries (8 bytes each for unweighted: input symbol u16, output symbol
u16, target info u32).

Help me understand: how would you write a parser for this format in Rust?
What are the tradeoffs of zero-copy parsing (casting raw bytes to
structs via bytemuck) vs copying into new structs?
```

**Key file to read:** `libvoikko/rust/crates/voikko-fst/src/format.rs`


## 6. Dictionary Compilation (foma)

The `.vfst` files are compiled from linguistic source data using **foma**, a finite-state toolkit.

### What you need to know

The `voikko-fi/` directory at the repo root contains Finnish linguistic source data:

- **joukahainen.xml** (8.7MB) — a database of Finnish words with morphological classifications
- **Python generators** — convert the XML into `.lexc` format (a standard lexicon format)
- **handwritten .lexc files** — inflection patterns, numerals, special cases
- **main.foma.in** — the foma script that compiles the final FST

The pipeline: XML → Python → `.lexc` → foma → `.att` → `.vfst`.

### When you need this

Only if you're modifying the dictionary (adding words, fixing morphological rules). For code work, treat `.vfst` files as given.

### How to learn

```plaintext
What is the LEXC format used in finite-state morphology tools like foma
and HFST? Explain the structure of a .lexc file: LEXICON declarations,
continuation classes, multichar symbols. Give a small example that
defines a few Finnish nouns with nominative and partitive forms.
```

```plaintext
I'm looking at a foma script that compiles a Finnish morphological
transducer. It uses commands like "define", "regex", "compose",
"minimize", and "save stack". Explain what each of these foma commands
does and how they work together to build a transducer from lexicon files.
```

**Key files to read:**
- `voikko-fi/vvfst/main.foma.in` — the master foma script
- `voikko-fi/vocabulary/flags.txt` — vocabulary flag documentation


## 7. WebAssembly and wasm-bindgen

The Rust code is compiled to WebAssembly so it can run in browsers and Node.js.

### What you need to know

**WebAssembly (WASM)** is a binary instruction format that runs in web browsers at near-native speed. Rust compiles to WASM via the `wasm32-unknown-unknown` target.

**wasm-bindgen** generates JS glue code that handles type conversion between Rust and JavaScript. The `#[wasm_bindgen]` attribute marks functions and types for export. For complex return types, this project uses **serde-wasm-bindgen** to serialize Rust structs to JS objects.

The compiled binary is 189KB after `wasm-opt -Oz`. The JS wrapper (`libvoikko/js/`) handles WASM loading, dictionary fetching, CDN fallback, caching, and TypeScript types.

### When you need this

Only if you're working on the JS/TS package or adding new public API methods.

### How to learn

```plaintext
Explain how wasm-bindgen works in Rust. How does the #[wasm_bindgen]
attribute transform Rust functions for JavaScript consumption? What
happens to Rust types like String, Vec<String>, and custom structs when
they cross the WASM boundary? What are the limitations?
```

```plaintext
What is serde-wasm-bindgen and when would you use it instead of plain
wasm-bindgen? Compare the two approaches for returning a Vec<MyStruct>
from Rust to JavaScript. What are the performance implications?
```

**Key file to read:** `libvoikko/rust/crates/voikko-wasm/src/lib.rs`


## 8. C FFI and Language Bindings

The Rust code is also compiled to a native shared library for use from Python, Java, C#, and Common Lisp.

### What you need to know

The `voikko-ffi` crate exposes `extern "C"` functions using the opaque handle pattern: callers create a handle, pass it to every function, and free it when done.

The key challenge is memory ownership. Every function that returns allocated data has a corresponding `voikko_free_*()` function. Each language binding (Python ctypes, Java JNA, C# P/Invoke, Common Lisp CFFI) wraps these C functions in idiomatic APIs.

### When you need this

Only if you're adding a new public API function or working on a language binding.

### How to learn

```plaintext
Explain Rust's extern "C" FFI. How do you expose a Rust function to C
callers? What types can cross the FFI boundary safely? How do you handle
Rust's String type — what's the pattern for returning C strings and who
is responsible for freeing them?
```

```plaintext
I'm building a Rust shared library that returns complex data to C
callers (arrays of structs, strings inside structs). Explain the opaque
handle pattern: how to create handles, pass them to functions, and free
them. What are the common pitfalls (double free, use after free, memory
leaks)?
```

**Key file to read:** `libvoikko/rust/crates/voikko-ffi/src/lib.rs`


## Learning Path

Follow this order. Stop when you have enough context for your task.

1. **The problem** — why word lists don't work for Finnish (Section 1)
2. **Finnish morphology** — what the language looks like (Section 2)
3. **FST** — the data structure that solves it (Section 3)
4. **Read the code** — `handle.rs` → `morphology/` → `voikko-fst/` (see [ARCHITECTURE.md](libvoikko/rust/ARCHITECTURE.md))
5. **Suggestions** — if working on the suggestion module (Section 4)
6. **VFST format** — if working on the FST engine (Section 5)
7. **foma** — if modifying the dictionary (Section 6)
8. **WASM or FFI** — if working on bindings (Section 7 or 8)

Most contributors need Sections 1-3 and then the code. The rest is on-demand.
