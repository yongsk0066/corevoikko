# voikko-fi

Finnish-specific NLP module. Provides morphological analysis, spell checking, hyphenation, suggestion generation, grammar checking, and tokenization.

## Purpose

This is the largest crate in the workspace and where most Finnish language logic lives. It wraps the FST engine from `voikko-fst` with Finnish-specific rules for analyzing word structure, checking spelling, generating suggestions, and detecting grammar errors. The `VoikkoHandle` in `handle.rs` ties all modules together into a single unified API.

## Key types

- `VoikkoHandle` -- top-level entry point that owns all components. All public methods (spell, suggest, analyze, hyphenate, grammar_errors, tokens, sentences) live here.
- `VoikkoError` -- construction error enum (MorphologyLoad, AutocorrectLoad, UnsupportedLanguage)
- `FinnishVfstAnalyzer` -- walks the FST and parses output tags into `Analysis` structs
- `AnalyzerToSpellerAdapter` -- adapts the analyzer into a speller interface
- `SpellerCache` -- LRU cache for spell check results (wrapped in `RefCell` for interior mutability)
- `FinnishGrammarChecker` -- paragraph-level grammar error detection with 18 rule types
- `SuggestionStrategy` -- configurable chain of suggestion generators (typing vs OCR)

## Module structure

```
src/
  lib.rs                    # feature-gated module declarations
  handle.rs                 # VoikkoHandle (unified API, "handle" feature)
  finnish/
    constants.rs            # Finnish vowel/consonant tables
  morphology/
    mod.rs                  # Analyzer trait
    vfst.rs                 # VfstAnalyzer (generic weighted FST traversal)
    finnish.rs              # FinnishVfstAnalyzer (tag parsing, highest complexity)
    tag_parser.rs           # FST output tag parser
  speller/
    mod.rs                  # Speller trait
    adapter.rs              # AnalyzerToSpellerAdapter
    cache.rs                # SpellerCache (with invalidation on resize)
    finnish.rs              # FinnishSpellerTweaks (Finnish-specific rules)
    pipeline.rs             # normalize -> cache -> spell pipeline
    utils.rs                # STRUCTURE pattern matching
  hyphenator/
    mod.rs                  # FinnishHyphenator + Hyphenator trait
  suggestion/
    mod.rs                  # suggestion module root
    strategy.rs             # SuggestionStrategy (generator chain)
    generators.rs           # individual generators (edit distance, split, etc.)
    vfst.rs                 # FST-based suggestion generation
    status.rs               # SuggestionStatus (priority queue)
  grammar/
    mod.rs                  # grammar module root
    checker.rs              # FinnishGrammarChecker
    engine.rs               # rule evaluation engine
    checks.rs               # GrammarOptions + individual check functions
    paragraph.rs            # paragraph splitting
    finnish_analysis.rs     # grammar-specific analysis helpers
    cache.rs                # grammar check caching
    autocorrect.rs          # autocorrect transducer integration
  tokenizer/
    mod.rs                  # next_token() + next_sentence() (always enabled)
```

## Feature flags

| Feature | Default | Enables | Dependencies |
|---------|---------|---------|-------------|
| `analyze` | yes | morphology module | -- |
| `spell` | yes | speller module | analyze |
| `suggest` | no | suggestion module | spell |
| `hyphenate` | no | hyphenator module | analyze |
| `grammar` | no | grammar module | analyze |
| `tokenize` | no | (tokenizer is always compiled, this flag is for explicitness) | -- |
| `handle` | no | VoikkoHandle + all modules | all above |

The `handle` feature enables everything and is used by voikko-wasm, voikko-ffi, and voikko-cli.

## Design decisions

- **Interior mutability for caching**: `VoikkoHandle` methods take `&self`, but the `SpellerCache` needs mutation. It uses `RefCell<SpellerCache>` to avoid requiring `&mut self` on every spell check call.
- **No self-referential lifetimes**: adapter objects (AnalyzerToSpellerAdapter, FinnishHyphenator) are created on-demand in each method call rather than stored as fields. This avoids self-referential struct issues.
- **Feature gating for binary size**: modules behind feature flags keep the WASM binary small when only spell checking is needed.

## Build and test

```bash
cargo test -p voikko-fi                       # 494 tests + 10 ignored
cargo test -p voikko-fi --all-features        # includes handle tests
cargo clippy -p voikko-fi --all-features -- -D warnings

# Integration tests (require dictionary files)
VOIKKO_DICT_PATH=/path/to/dict cargo test -p voikko-fi --all-features

# Benchmarks (7 benchmarks, requires dictionary)
VOIKKO_DICT_PATH=/path/to/dict cargo bench -p voikko-fi --features handle
```
