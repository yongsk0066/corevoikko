# CLAUDE.md -- C# Binding

C# P/Invoke wrapper for the Rust `voikko-ffi` cdylib. Single file: `VoikkoRust.cs` (~740 lines). Targets .NET 6+ / C# 10+.

## Native Library

Loads `voikko_ffi` via P/Invoke `DllImport`:
- macOS: `libvoikko_ffi.dylib`
- Linux: `libvoikko_ffi.so`
- Windows: `voikko_ffi.dll`

Build the native library first: `cargo build --release -p voikko-ffi`

## Usage

```csharp
using var v = new Voikko("/path/to/vvfst");   // dir with mor.vfst
bool ok = v.Spell("koira");                    // true
List<string> sugg = v.Suggest("koirra");       // ["koira", "koiraa", ...]
v.Hyphenate("koirarata");                       // "koi-ra-ra-ta"
```

The `Voikko` class implements `IDisposable`. Use `using` to ensure native resources are released.

## File Structure

`VoikkoRust.cs` contains everything in the `VoikkoRust` namespace:

**Enums**: `TokenType` (None, Word, Punctuation, Whitespace, Unknown), `SentenceType` (None, NoStart, Probable, Possible).

**Result records**: `Analysis` (extends `Dictionary<string, string>`), `GrammarError`, `Token`, `Sentence`.

**Native structs** (internal, blittable): `NativeAnalysis`, `NativeAnalysisArray`, `NativeGrammarError`, `NativeGrammarErrorArray`, `NativeToken`, `NativeTokenArray`, `NativeSentence`, `NativeSentenceArray`. These mirror the `repr(C)` structs from the Rust FFI.

**P/Invoke declarations** (`Native` static class): all `voikko_*` functions with `CallingConvention.Cdecl`.

**Public API** (`Voikko` class, `IDisposable`): thread-safe via `lock(_lock)`.

## API

### Constructor

`new Voikko(dictPath)` -- reads `mor.vfst` (and optionally `autocorr.vfst`) from `dictPath`. Auto-detects flat layout and V5 structure (`{path}/5/mor-standard/`). Throws `FileNotFoundException` or `VoikkoException` on failure.

### Methods

- `Spell(word)` -- `bool`
- `Suggest(word)` -- `List<string>`
- `Analyze(word)` -- `List<Analysis>`
- `Hyphenate(word, separator="-", allowContextChanges=true)` -- `string`
- `GetHyphenationPattern(word)` -- `string`
- `GrammarErrors(text, language="fi")` -- `List<GrammarError>`
- `Tokens(text)` -- `List<Token>`
- `Sentences(text)` -- `List<Sentence>`
- `Dispose()` -- releases native handle

### Properties (write-only option setters)

Boolean: `IgnoreDot`, `IgnoreNumbers`, `IgnoreUppercase`, `NoUglyHyphenation`, `AcceptFirstUppercase`, `AcceptAllUppercase`, `OcrSuggestions`, `IgnoreNonwords`, `AcceptExtraHyphens`, `AcceptMissingHyphens`, `AcceptTitlesInGc`, `AcceptUnfinishedParagraphsInGc`, `HyphenateUnknownWords`, `AcceptBulletedListsInGc`.

Integer: `MinHyphenatedWordLength`, `MaxSuggestions`, `SpellerCacheSize`.

### Static Methods

- `Voikko.Version()` -- `string`
- `Voikko.AttributeValues(name)` -- `List<string>?`

## Notes

- All strings are passed to native code as null-terminated UTF-8 byte arrays via the internal `Enc()` helper.
- Native pointers are read via `Marshal.PtrToStringUTF8` and `Marshal.ReadIntPtr`.
- This binding was written specifically for the Rust FFI (not the C++ API). It uses the `voikko_new` / `voikko_free` lifecycle and struct-returning functions.
- `VoikkoException` is the binding-specific exception type.
