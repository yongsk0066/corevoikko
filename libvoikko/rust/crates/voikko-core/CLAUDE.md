# voikko-core

Shared types and utilities for all Voikko crates. Every other crate in the workspace depends on this one.

## Purpose

voikko-core defines the public API types that flow between crates: morphological analysis results, tokens, sentences, grammar errors, and enums. It also provides character classification and case detection utilities needed by the Finnish language module.

## Key types

- `Analysis` -- morphological analysis result, wraps `HashMap<String, String>` with typed attribute key constants (`ATTR_BASEFORM`, `ATTR_CLASS`, `ATTR_STRUCTURE`, etc.)
- `Token` -- a text token with `TokenType`, text content, length, and position
- `Sentence` -- a sentence boundary with `SentenceType` and character length
- `GrammarError` -- a grammar error with error code, position, length, suggestions, and bilingual descriptions (Finnish/English)
- `TokenType` -- enum: None, Word, Punctuation, Whitespace, Unknown
- `SentenceType` -- enum: None, NoStart, Probable, Possible
- `SpellResult` -- enum: Ok, CapitalizeFirst, CapitalizationError, Failed

## Module structure

```
src/
  lib.rs           # re-exports all modules
  enums.rs         # TokenType, SentenceType, SpellResult, option constants
  analysis.rs      # Analysis struct + 21 attribute key constants
  token.rs         # Token + Sentence structs
  grammar_error.rs # GrammarError struct + 18 error codes + description functions
  character.rs     # character classification, Finnish character handling
  case.rs          # case type detection (uppercase, lowercase, mixed), conversion
```

## Design decisions

- **Single external dependency**: only `thiserror` for error derives.
- **Derive-heavy types**: all types implement `Debug, Clone, PartialEq, Eq` at minimum.
- **String-based analysis**: `Analysis` uses `HashMap<String, String>` rather than an enum-keyed map. This matches the C++ design where attribute keys are strings, and allows forward compatibility with new attributes.
- **Bilingual grammar descriptions**: `error_code_description_lang()` supports Finnish (default) and English, matching the C++ `voikko_error_message_cstr` output.

## Build and test

```bash
cargo test -p voikko-core              # 68 tests
cargo clippy -p voikko-core -- -D warnings
```
