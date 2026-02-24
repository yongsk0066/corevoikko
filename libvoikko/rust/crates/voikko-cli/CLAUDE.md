# voikko-cli

Command-line tools for Finnish NLP. Eight binaries for testing and demonstrating Voikko's capabilities.

## Purpose

This crate provides standalone CLI tools that read Finnish text from stdin and produce NLP results on stdout. They are useful for manual testing, scripting, and demonstrating the library's capabilities. These tools are not part of the library's public API.

## Binaries

| Binary | Description | Output format |
|--------|-------------|---------------|
| `voikko-spell` | Spell check words | `C: word` (correct) / `W: word` (wrong), optional `-s` for suggestions |
| `voikko-suggest` | Generate suggestions | one suggestion per line |
| `voikko-analyze` | Morphological analysis | key-value attribute pairs per word |
| `voikko-hyphenate` | Hyphenate words | hyphenation pattern string |
| `voikko-tokenize` | Tokenize text | token type + text per token |
| `voikko-gc-pretty` | Grammar check with formatting | highlighted errors with suggestions |
| `voikko-baseform` | Extract base forms | base form of each word |
| `voikko-readability` | Compute readability metrics | readability statistics |

## Common options

All binaries share these options via the `lib.rs` helper module:

- `-d PATH` / `--dict-path PATH` -- dictionary directory containing `mor.vfst`
- `-h` / `--help` -- print usage information

## Dictionary search order

When no explicit path is given, the tools search for `mor.vfst` in this order:

1. `-d` argument
2. `VOIKKO_DICT_PATH` environment variable
3. `~/.voikko/5/mor-standard`
4. macOS: `~/Library/Spelling/voikko/5/mor-standard`
5. `/etc/voikko/5/mor-standard`, `/usr/lib/voikko/5/mor-standard`, `/usr/share/voikko/5/mor-standard`
6. Current working directory

## Shared library code

`src/lib.rs` provides common utilities used by all binaries:

- `load_handle(dict_path)` -- searches for dictionary files and creates a `VoikkoHandle`
- `parse_dict_path(args)` -- parses `-d`/`--dict-path` from CLI arguments
- `fatal(msg)` -- prints error and exits
- `wants_help(args)` -- checks for `-h`/`--help`

## Build and run

```bash
# Build all CLI tools
cargo build -p voikko-cli

# Run a specific tool
VOIKKO_DICT_PATH=/path/to/dict cargo run -p voikko-cli --bin voikko-spell

# Or with explicit dict path
echo "koira" | cargo run -p voikko-cli --bin voikko-spell -- -d /path/to/dict
```
