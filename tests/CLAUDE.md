# tests

Integration test data for Voikko. These are plain-text input/expected-output files consumed by the `voikkotest` tool (`tools/bin/voikkotest`) and by the C++ legacy test suite.

The Rust unit tests live inside each crate (`libvoikko/rust/crates/*/`), not here.

## voikkotest/ -- Main Test Data

Organized by dictionary variant. Each variant directory contains text files with expected results.

### fi-x-vfst/ (primary, 8 files)

The main Finnish VFST test suite:

- `spell.txt` -- Spell checking assertions. Lines starting with `!` are expected misspellings; bare words are expected correct. `#` lines are comments, `[]` lines are section headers.
- `morpho.txt` -- Morphological analysis assertions (word + expected analysis attributes).
- `grammar.txt` -- Grammar checker assertions (input paragraph + expected error codes, positions, suggestions).
- `hyphen.txt` -- Hyphenation assertions.
- `suggest.txt` -- Spelling suggestion assertions.
- `sentence.txt` -- Sentence detection assertions.
- `tokenizer.txt` -- Tokenization assertions.
- `config.txt` -- Test configuration (dictionary path, language variant settings).

### fi-x-svfst/ -- Sukija variant tests (spell, morpho, suggest, config)

### fi-x-murre/ -- Dialect variant tests (spell, morpho, config)

## testdicts/

Minimal test dictionaries for unit testing dictionary loading.

- `fi-x-svfst/` -- Contains a small hand-built VFST dictionary with `Makefile`, `all.lexc`, weighted ATT files, and `index.txt`. Used to test dictionary discovery and loading without the full 3.8MB production dictionary.

## hyphenation/

Reference hyphenation data from Finnish literature (Juhani Aho's "Rautatie"):

- `Aho_Rautatie-hyphenated.txt` -- Text with correct hyphenation marks.
- `Aho_Rautatie-unhyphenated.txt` -- Same text without hyphenation.

## ooovoikkotest/

- `hyphen.txt` -- Hyphenation test cases for OpenOffice/LibreOffice integration testing, with configuration parameters (`HyphMinWordLength`, `HyphMinLeading`, `HyphMinTrailing`).

## Other Files

- `vanhat-sanat.txt` -- Old Finnish word forms (dialects, archaic forms) for testing `VANHAT_MUODOT` mode.
