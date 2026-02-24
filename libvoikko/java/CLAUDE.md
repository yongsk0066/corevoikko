# CLAUDE.md -- Java Binding

Java JNA wrapper for the Voikko Finnish NLP library. Contains two binding classes: the original `Voikko` (targets the C++ libvoikko API) and the newer `VoikkoRust` (targets the Rust `voikko-ffi` cdylib directly).

## Build

```bash
cd libvoikko/java
mvn compile          # requires JNA 5.17.0 (from pom.xml)
mvn test             # requires native library on java.library.path
```

Maven coordinates: `org.puimula.voikko:libvoikko:4.3.3`. Dependency: `net.java.dev.jna:jna:5.17.0`.

## Native Library

**VoikkoRust** loads `voikko_ffi` (the Rust cdylib):
- macOS: `libvoikko_ffi.dylib`
- Linux: `libvoikko_ffi.so`
- Windows: `voikko_ffi.dll`

Build the native library first: `cargo build --release -p voikko-ffi`

**Voikko** (legacy) loads the original C++ library (`libvoikko.so.1` / `voikko-1` / `voikko`).

## Directory Layout

```
java/
├── src/main/java/org/puimula/libvoikko/
│   ├── VoikkoRust.java       # Rust FFI binding (JNA, ~1020 lines)
│   ├── Voikko.java            # Legacy C++ binding (JNA)
│   ├── Libvoikko.java         # JNA interface for legacy C++ API
│   ├── Analysis.java          # HashMap<String, String> subclass
│   ├── GrammarError.java      # Grammar error data class
│   ├── Token.java             # Token data class
│   ├── Sentence.java          # Sentence data class
│   ├── Dictionary.java        # Dictionary variant info
│   ├── TokenType.java         # Enum: NONE, WORD, PUNCTUATION, WHITESPACE, UNKNOWN
│   ├── SentenceStartType.java # Enum: NONE, NO_START, PROBABLE, POSSIBLE
│   ├── SuggestionStrategy.java# Enum: TYPO, OCR
│   ├── VoikkoException.java   # Runtime exception
│   ├── ByteArray.java         # JNA helper for byte array marshalling
│   ├── SizeT.java             # JNA size_t type
│   ├── SizeTByReference.java  # JNA size_t pointer
│   └── package-info.java
└── pom.xml
```

## VoikkoRust (Rust FFI)

The primary binding for the Rust implementation. Uses JNA to call `voikko-ffi` C functions directly.

### Initialization

The constructor reads dictionary bytes from the filesystem and passes them to `voikko_new`:

```java
VoikkoRust v = new VoikkoRust("/path/to/vvfst");  // dir with mor.vfst
```

Implements `AutoCloseable` for try-with-resources:

```java
try (VoikkoRust v = new VoikkoRust(dictPath)) {
    v.spell("koira"); // true
}
```

### Key Differences from Legacy Voikko

- Takes a dictionary directory path (reads `mor.vfst` bytes directly) instead of a language code
- JNA interface maps to the Rust FFI functions (`voikko_new`, `voikko_spell`, `voikko_suggest`, etc.)
- Struct-by-value returns for analysis, grammar error, token, and sentence arrays
- `AutoCloseable` interface (legacy uses `terminate()` / `finalize()`)
- All methods are `synchronized` for thread safety

### API Methods

- `spell(word)` -- `boolean`
- `suggest(word)` -- `List<String>`
- `analyze(word)` -- `List<Analysis>`
- `hyphenate(word)` / `hyphenate(word, separator, allowContextChanges)` -- `String`
- `getHyphenationPattern(word)` -- `String`
- `grammarErrors(text)` / `grammarErrors(text, language)` -- `List<GrammarError>`
- `tokens(text)` -- `List<Token>`
- `sentences(text)` -- `List<Sentence>`
- `attributeValues(name)` -- `List<String>` or `null`
- `close()` / `terminate()` -- release native resources

Static: `version()`, `addLibraryPath(path)`.

Boolean setters: `setIgnoreDot`, `setIgnoreNumbers`, `setIgnoreUppercase`, `setNoUglyHyphenation`, `setAcceptFirstUppercase`, `setAcceptAllUppercase`, `setOcrSuggestions`, `setSuggestionStrategy`, `setIgnoreNonwords`, `setAcceptExtraHyphens`, `setAcceptMissingHyphens`, `setAcceptTitlesInGc`, `setAcceptUnfinishedParagraphsInGc`, `setHyphenateUnknownWords`, `setAcceptBulletedListsInGc`.

Integer setters: `setMinHyphenatedWordLength`, `setMaxSuggestions`, `setSpellerCacheSize`.

### JNA Struct Mappings

`VoikkoRust` defines inner classes that mirror the C structs from `voikko-ffi`:
- `NativeVoikkoAnalysis` / `NativeVoikkoAnalysisArray`
- `NativeVoikkoGrammarError` / `NativeVoikkoGrammarErrorArray`
- `NativeVoikkoToken` / `NativeVoikkoTokenArray`
- `NativeVoikkoSentence` / `NativeVoikkoSentenceArray`

## Voikko (Legacy C++ Binding)

The original binding that targets C++ libvoikko. Uses `Libvoikko.java` as its JNA interface. Initializes via `voikkoInit(language, path)` rather than reading dictionary bytes directly. Includes `listDicts()` and language listing methods not available in VoikkoRust.

## Notes

- Both bindings exist side-by-side. New code should use `VoikkoRust`.
- The legacy `Voikko` class will not work unless the C++ libvoikko is installed on the system.
- JNA loads the library via `Native.load()`. Use `VoikkoRust.addLibraryPath()` or set `jna.library.path` if the library is not on the default search path.
