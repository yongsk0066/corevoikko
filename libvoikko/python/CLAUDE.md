# CLAUDE.md -- Python Binding

Python ctypes wrapper for the Voikko C library. Originally written for the C++ libvoikko; now works with the Rust FFI cdylib (`libvoikko_ffi`).

## Overview

Single file: `libvoikko.py` (~920 lines). Uses Python's `ctypes` module to load the native shared library and call its C API functions. No external dependencies beyond the standard library.

Compatible with Python 2.7+ and Python 3.x (uses `from __future__ import unicode_literals`).

## Native Library

This binding loads the **old-style C API** (function names like `voikkoInit`, `voikkoSpellUcs4`, `voikkoSuggestUcs4`). It expects:
- macOS: `libvoikko.1.dylib`
- Linux: `libvoikko.so.1`
- Windows: `libvoikko-1.dll`

To use with the Rust FFI, the Rust cdylib must expose these same symbol names, or a compatibility shim is needed. The Rust `voikko-ffi` crate exposes a different API (`voikko_new`, `voikko_spell`, etc.), so this binding currently targets the original C++ library interface.

## Key Classes

- `Voikko(language, path=None)` -- main class. Creates a handle via `voikkoInit`. All NLP methods are instance methods on this class.
- `VoikkoLibrary(CDLL)` -- low-level ctypes wrapper that sets argument/return types for every C function.
- `Dictionary` -- represents an available dictionary variant (language, script, variant, description).
- `Token` -- tokenizer result with `tokenText` and `tokenType` (integer constants: NONE=0, WORD=1, PUNCTUATION=2, WHITESPACE=3, UNKNOWN=4).
- `Sentence` -- sentence boundary result with `sentenceText` and `nextStartType`.
- `GrammarError` -- grammar error with errorCode, startPos, errorLen, shortDescription, suggestions.
- `SuggestionStrategy` -- enum-like class with TYPO=0 and OCR=1.
- `VoikkoException` -- base exception for all errors.

## API Methods (Voikko class)

- `spell(word)` -- returns `bool`
- `suggest(word)` -- returns `list[str]`
- `analyze(word)` -- returns `list[dict]` (key-value morphological analysis)
- `hyphenate(word, separator="-", allowContextChanges=True)` -- returns hyphenated string
- `getHyphenationPattern(word)` -- returns pattern string
- `grammarErrors(text, language)` -- returns `list[GrammarError]` (handles multi-paragraph splitting)
- `tokens(text)` -- returns `list[Token]`
- `sentences(text)` -- returns `list[Sentence]`
- `attributeValues(attributeName)` -- returns `list[str]` or `None`
- `terminate()` -- releases native resources

Class methods: `listDicts(path)`, `listSupportedSpellingLanguages(path)`, `listSupportedHyphenationLanguages(path)`, `listSupportedGrammarCheckingLanguages(path)`, `getVersion()`, `setLibrarySearchPath(path)`.

Boolean option setters: `setIgnoreDot`, `setIgnoreNumbers`, `setIgnoreUppercase`, `setAcceptFirstUppercase`, `setAcceptAllUppercase`, `setIgnoreNonwords`, `setAcceptExtraHyphens`, `setAcceptMissingHyphens`, `setAcceptTitlesInGc`, `setAcceptUnfinishedParagraphsInGc`, `setAcceptBulletedListsInGc`, `setNoUglyHyphenation`, `setHyphenateUnknownWords`.

Integer option setters: `setMinHyphenatedWordLength`, `setSpellerCacheSize`.

`setSuggestionStrategy(value)` accepts `SuggestionStrategy.TYPO` or `SuggestionStrategy.OCR`.

## Usage

```python
import libvoikko

v = libvoikko.Voikko("fi")
v.spell("kissa")       # True
v.suggest("kisssa")     # ['kissa', 'kissaa', ...]
v.analyze("kissa")      # [{'SIJAMUOTO': 'nimento', 'CLASS': 'nimisana', ...}]
v.hyphenate("kissa")    # 'kis-sa'
v.terminate()
```

## Notes

- Thread safety: a single `Voikko` instance should not be used from multiple threads simultaneously.
- After `terminate()`, the internal `__lib` is replaced by a dummy that raises `VoikkoException` on any method call.
- `grammarErrors` splits text on `\n` and processes each paragraph independently, adjusting error offsets.
- This file predates the Rust rewrite and uses the original C++ function naming convention (`voikkoSpellUcs4`, `voikkoSuggestUcs4`). The Rust `voikko-ffi` crate uses a different API surface.
