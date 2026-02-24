# voikko-fst

Language-agnostic VFST (Voikko Finite State Transducer) engine. Reads `.vfst` binary files and traverses the graph to produce output strings from input words.

## Purpose

This crate loads and traverses FST graphs stored in the `.vfst` binary format. It knows nothing about Finnish -- it just follows graph edges and returns symbol sequences. The `voikko-fi` crate uses it to perform morphological analysis, spell checking, and other NLP tasks.

## Key types and traits

- `Transducer` trait -- the core abstraction with `prepare(&[char]) -> bool` and `next(&mut String) -> bool` methods. Uses a coroutine-style pattern: call `prepare` once, then `next` repeatedly until it returns `false`.
- `UnweightedTransducer` -- loads and traverses unweighted `.vfst` files (8-byte transitions)
- `WeightedTransducer` -- loads and traverses weighted `.vfst` files (16-byte transitions with weight)
- `VfstError` -- typed error enum for parsing failures (InvalidMagic, TooShort, TypeMismatch, InvalidSymbolTable, InvalidFlagDiacritic, AlignmentError)
- `Configuration` / `WeightedConfiguration` -- explicit DFS stack for traversal state

## Module structure

```
src/
  lib.rs         # Transducer trait, VfstError, MAX_LOOP_COUNT
  format.rs      # 16-byte header parsing and validation
  transition.rs  # #[repr(C)] transition structs + bytemuck zero-copy
  symbols.rs     # symbol table (HashMap<char, u16> + Vec<String>)
  flags.rs       # flag diacritic operations (P, C, U, R, D)
  config.rs      # traversal configuration (explicit DFS stack)
  unweighted.rs  # UnweightedTransducer loading + traversal
  weighted.rs    # WeightedTransducer loading + traversal (with backtracking)
```

## VFST binary format

The `.vfst` file layout:

1. **16-byte header**: 8-byte magic + 1-byte weighted flag + 7 reserved bytes
2. **Symbol table**: 2-byte count + null-terminated UTF-8 strings
3. **Padding**: aligned to 8 bytes (unweighted) or 16 bytes (weighted)
4. **Transition table**: array of fixed-size entries -- 8 bytes each for unweighted (symIn, symOut, transInfo), 16 bytes for weighted (+weight field)

## Design decisions

- **No byte-swap**: WASM is always little-endian, and dictionaries are written in LE. Byte-swap logic from C++ is removed.
- **No mmap**: data is loaded as `Vec<u8>`. Native mmap support can be added later via `memmap2`.
- **Zero-copy transitions**: transition tables are cast directly from bytes using `bytemuck::cast_slice`, avoiding per-transition allocation.
- **Explicit DFS stack**: traversal uses `continue 'outer` labeled loops instead of the C++ goto pattern. No recursion, keeping memory usage predictable.

## Performance

`Transducer::next()` is the hottest function in the codebase -- it is called roughly 10,000 times per word during morphological analysis. The zero-allocation goal in the traversal inner loop is critical for performance.

## Build and test

```bash
cargo test -p voikko-fst              # 71 tests
cargo clippy -p voikko-fst -- -D warnings
```
