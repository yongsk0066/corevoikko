# FST Engine -- Technical Analysis for Rust WASM Port

This document is a byte-level specification of the libvoikko VFST (Voikko Finite State Transducer) engine, derived from reading every line of the C++ implementation. It covers the binary format, traversal algorithm, flag diacritic handling, data structures, module interfaces, and Rust mapping considerations.

Source files analyzed (all paths relative to `libvoikko/src/`):

| File | Lines | Role |
|------|-------|------|
| `fst/Transducer.hpp` | 83 | Base class: flag diacritics, mmap, byte-order detection |
| `fst/Transducer.cpp` | 197 | Base implementation |
| `fst/Transition.hpp` | 54 | Unweighted transition + overflow cell structs |
| `fst/UnweightedTransducer.hpp` | 61 | Unweighted transducer class |
| `fst/UnweightedTransducer.cpp` | 372 | Unweighted loading, traversal, byte-swap |
| `fst/WeightedTransition.hpp` | 53 | Weighted transition + overflow cell structs |
| `fst/WeightedTransducer.hpp` | 64 | Weighted transducer class |
| `fst/WeightedTransducer.cpp` | 430 | Weighted loading, traversal, byte-swap, backtrack |
| `fst/Configuration.hpp` | 58 | Unweighted traversal state |
| `fst/Configuration.cpp` | 66 | Unweighted configuration allocation |
| `fst/WeightedConfiguration.hpp` | 56 | Weighted traversal state |
| `fst/WeightedConfiguration.cpp` | 62 | Weighted configuration allocation |
| `tools/voikkovfstc.cpp` | 508 | VFST compiler (ATT format to binary) |

---

## 1. VFST Binary Format Specification

### 1.1 Header (16 bytes)

```
Offset  Size   Field                Description
------  ----   -----                -----------
0x00    4      cookie1              Magic number: 0x00013A6E (LE native)
0x04    4      cookie2              Magic number: 0x000351FA (LE native)
0x08    1      weighted_flag        0x00 = unweighted, 0x01 = weighted
0x09    7      reserved             Must be zero (future extensions)
```

**Source**: `Transducer.cpp:163-178` (reading), `voikkovfstc.cpp:451-460` (writing).

**Byte-order detection** (`Transducer.cpp:163-178`): The reader checks the cookie pair in native byte order first. If the cookies match reversed (`0x6E3A0100`, `0xFA510300`), the file needs byte-swapping. If neither matches, a `DictionaryException` is thrown.

**Weighted-vs-unweighted** (`Transducer.cpp:181-183`): Determined by byte at offset 8. The reader class validates this against its own type:
- `UnweightedTransducer` constructor rejects `weighted_flag == 0x01`
- `WeightedTransducer` constructor rejects `weighted_flag == 0x00`

### 1.2 Symbol Table

Immediately after the 16-byte header:

```
Offset  Size       Field           Description
------  ----       -----           -----------
0x10    2          symbol_count    uint16_t, number of symbols
0x12    variable   symbols[]       Null-terminated UTF-8 strings, concatenated
```

**Symbol ordering** (enforced by `voikkovfstc.cpp:196-224`):

Symbols are sorted in a specific order before writing:
1. **Epsilon** (index 0): empty string `""`
2. **Flag diacritics**: strings starting with `@` (e.g., `@P.FEATURE.VALUE@`)
3. **Normal single-character symbols**: regular characters (e.g., `a`, `b`, ...)
4. **Multi-character symbols**: strings starting with `[` (e.g., `[Ln]`, `[Bc]`)

This ordering is critical because the reader derives `firstNormalChar` and `firstMultiChar` boundaries from index positions during loading (`UnweightedTransducer.cpp:144-178`, `WeightedTransducer.cpp:150-184`).

**Symbol index 0** is always epsilon (empty string, single null byte in file). It receives special treatment: the reader skips it for `stringToSymbol` mapping and assigns it zero length (`UnweightedTransducer.cpp:154-158`).

**Final transition sentinel**: In weighted transducers, final transitions use `symIn = 0xFFFFFFFF` (uint32_t max). In unweighted transducers, `symIn = 0xFFFF` (uint16_t max). These are not stored in the symbol table.

### 1.3 Padding

After the symbol table, padding bytes are inserted to align the transition table:

- **Unweighted**: align to `sizeof(Transition)` = 8-byte boundary
- **Weighted**: align to `sizeof(WeightedTransition)` = 16-byte boundary

**Source**: `voikkovfstc.cpp:472-478` (writing), `UnweightedTransducer.cpp:181-187` (reading), `WeightedTransducer.cpp:186-192` (reading).

### 1.4 Transition Table

#### Unweighted Transition (8 bytes)

```
struct Transition {                      // Transition.hpp:41-45
    uint16_t symIn;                      // offset 0, 2 bytes
    uint16_t symOut;                     // offset 2, 2 bytes
    transinfo_t transInfo;               // offset 4, 4 bytes
};

struct transinfo_t {                     // Transition.hpp:36-39
    unsigned int targetState : 24;       // bits 0-23: target state index
    unsigned int moreTransitions : 8;    // bits 24-31: extra transition count
};
```

Static assertion: `sizeof(Transition) == 8` (`UnweightedTransducer.hpp:40`).

**Bitfield layout of transinfo_t** (4 bytes total):
- On little-endian machines: bytes [0..2] contain `targetState` (24 bits), byte [3] contains `moreTransitions` (8 bits).
- The byte-swap logic (`UnweightedTransducer.cpp:112-113`) swaps the 3 bytes of `targetState` individually while keeping `moreTransitions` in place:
  ```
  uint32_t ts = t.transInfo.targetState;
  t.transInfo.targetState = ((ts<<16) & 0x00FF0000) | (ts & 0x0000FF00) | ((ts>>16) & 0x000000FF);
  ```

#### Unweighted Overflow Cell (8 bytes)

```
struct OverflowCell {                    // Transition.hpp:47-50
    uint32_t moreTransitions;            // offset 0, 4 bytes
    uint32_t padding;                    // offset 4, 4 bytes
};
```

#### Weighted Transition (16 bytes)

```
struct WeightedTransition {              // WeightedTransition.hpp:36-43
    uint32_t symIn;                      // offset 0, 4 bytes
    uint32_t symOut;                     // offset 4, 4 bytes
    uint32_t targetState;                // offset 8, 4 bytes
    int16_t weight;                      // offset 12, 2 bytes (signed)
    uint8_t moreTransitions;             // offset 14, 1 byte
    uint8_t reserved;                    // offset 15, 1 byte
};
```

Note: No `static_assert` for `sizeof(WeightedTransition)`, but `sizeof` = 16 is implied by the alignment code.

#### Weighted Overflow Cell (16 bytes)

```
struct WeightedOverflowCell {            // WeightedTransition.hpp:45-49
    uint32_t moreTransitions;            // offset 0, 4 bytes
    uint32_t shortPadding;               // offset 4, 4 bytes
    uint64_t padding;                    // offset 8, 8 bytes
};
```

### 1.5 State Layout in Transition Table

Each state occupies a contiguous block of transitions. The first transition of a state carries the `moreTransitions` field:

- If `moreTransitions < 255`: the state has `moreTransitions + 1` total transitions (including the first one).
- If `moreTransitions == 255`: an overflow cell follows immediately after the first transition, and `overflow.moreTransitions + 1` gives the total number of transitions (the overflow cell itself takes one slot).

**Source**: `getMaxTc()` in `UnweightedTransducer.cpp:219-226` and `WeightedTransducer.cpp:221-228`.

**Transition ordering within a state** (from `voikkovfstc.cpp:226-244`):
1. Epsilon transitions (symIn == 0) come first
2. Final transitions (symIn == 0xFFFF/0xFFFFFFFF) come next
3. Flag diacritics (symIn < firstNormalChar) come next
4. Normal character transitions, sorted by symIn ascending
5. Multi-character symbol transitions

This ordering enables the binary search optimization in `WeightedTransducer.cpp:366-381`.

### 1.6 State Addressing

`targetState` values are **transition table indices** (not byte offsets). The transducer accesses state N as `transitionStart[N]`. For unweighted, this means byte offset = `N * 8` from transition table start. For weighted, byte offset = `N * 16`.

---

## 2. Loading and Initialization

### 2.1 Memory Mapping

**Source**: `Transducer.cpp:125-161`.

Files are loaded via:
- **POSIX**: `open()` + `fstat()` + `mmap(PROT_READ, MAP_SHARED)` + `close(fd)`. The file descriptor is closed immediately after mmap.
- **Win32**: `CreateFile()` + `CreateFileMapping()` + `MapViewOfFile(FILE_MAP_READ)` + `CloseHandle()`.

Unmapping:
- **POSIX**: `munmap(map, fileLength)`
- **Win32**: `UnmapViewOfFile(map)`

**Byte-swap path**: If the file needs byte-swapping, a new `char[]` buffer is allocated, the entire file is decoded into it, and the original mmap is released (`UnweightedTransducer.cpp:71-123`, `WeightedTransducer.cpp:76-128`). The `byteSwapped` flag is stored so `terminate()` knows whether to `delete[]` or `munmap`.

### 2.2 Symbol Table Parsing

**Source**: `UnweightedTransducer.cpp:125-189`, `WeightedTransducer.cpp:130-194`.

During construction, the following data structures are built:

1. **`symbolToString: vector<wchar_t*>`** -- maps symbol index to its UCS-4 string representation.
2. **`symbolStringLength: vector<size_t>`** -- length of each symbol's string (via `wcslen`).
3. **`stringToSymbol: map<wchar_t, uint16_t>`** -- maps single wchar_t characters to their symbol indices. Only populated for normal single-character symbols (between `firstNormalChar` and `firstMultiChar`).
4. **`symbolToDiacritic: vector<OpFeatureValue>`** -- maps symbol indices (below `firstNormalChar`) to their flag diacritic operations.
5. **`firstNormalChar: uint16_t`** -- index of the first non-flag-diacritic, non-epsilon symbol.
6. **`firstMultiChar: uint16_t`** -- index of the first `[`-prefixed multi-character symbol.
7. **`flagDiacriticFeatureCount: uint16_t`** -- number of distinct flag diacritic features.
8. **`unknownSymbolOrdinal`** (unweighted only): set to `symbolCount`, used as sentinel for unknown input characters.

**Symbol classification** (determines the three zones of the symbol table):
- Index 0: epsilon
- Indices 1 to `firstNormalChar - 1`: flag diacritics (`@P.feat.val@`, `@C.feat@`, etc.)
- Indices `firstNormalChar` to `firstMultiChar - 1`: normal characters
- Indices `firstMultiChar` and above: multi-character symbols (tags like `[Ln]`)

### 2.3 Flag Diacritic Parsing

**Source**: `Transducer.cpp:62-123`.

Flag diacritic symbols have the format `@OP.FEATURE.VALUE@` or `@OP.FEATURE@`.

The parser (`getDiacriticOperation`) extracts:
- **Operation** from `symbol[1]`: `P`, `C`, `U`, `R`, `D`
- **Feature** from `symbol[3..dot]`: mapped to a uint16_t index via a `features` map
- **Value** from after the dot to `symbol.length()-1`: mapped to a uint16_t index via a `values` map

Pre-initialized values map:
- `"" -> FlagValueNeutral (0)` -- the neutral/unset value
- `"@" -> FlagValueAny (1)` -- the "any non-neutral" wildcard

---

## 3. Traversal Algorithm

### 3.1 Configuration (Traversal State)

#### Unweighted Configuration (`Configuration.hpp:38-54`)

```
struct Configuration {
    const int bufferSize;           // maximum stack depth
    int stackDepth;                 // current DFS depth
    int flagDepth;                  // number of flag diacritic saves on stack
    int inputDepth;                 // position in input being consumed
    uint32_t* stateIndexStack;      // [bufferSize] state index at each depth
    uint32_t* currentTransitionStack; // [bufferSize] transition index at each depth
    uint16_t* inputSymbolStack;     // [bufferSize] pre-mapped input symbols
    uint16_t* outputSymbolStack;    // [bufferSize] output symbol at each depth
    uint16_t* currentFlagValues;    // [flagDiacriticFeatureCount] current flag state (mutated in-place)
    uint16_t* updatedFlagValue;     // [bufferSize] previous value before flag update (for undo)
    uint16_t* updatedFlagFeature;   // [bufferSize] which feature was updated (for undo)
    int inputLength;                // total input length
};
```

The unweighted configuration uses a **destructive update + undo** strategy for flag diacritics: `currentFlagValues` is mutated in-place, and the old value is saved in `updatedFlagValue`/`updatedFlagFeature` stacks for backtracking.

#### Weighted Configuration (`WeightedConfiguration.hpp:38-52`)

```
struct WeightedConfiguration {
    const int bufferSize;           // maximum stack depth
    int stackDepth;                 // current DFS depth
    int flagDepth;                  // number of flag diacritic saves on stack
    int inputDepth;                 // position in input being consumed
    uint32_t* stateIndexStack;      // [bufferSize]
    uint32_t* currentTransitionStack; // [bufferSize]
    uint32_t* inputSymbolStack;     // [bufferSize] -- note: uint32_t, not uint16_t
    uint32_t* outputSymbolStack;    // [bufferSize] -- note: uint32_t, not uint16_t
    uint32_t* flagValueStack;       // [featureCount * bufferSize] copy-on-write flag state
    int inputLength;
};
```

The weighted configuration uses a **copy-on-push** strategy for flag diacritics: the entire flag array is copied forward by one slot on each flag diacritic step, allowing instant backtrack by simply decrementing `flagDepth`.

**Key difference**: Unweighted uses `uint16_t` for symbol indices; Weighted uses `uint32_t`.

### 3.2 Prepare Phase

**Source**: `UnweightedTransducer.cpp:197-217`, `WeightedTransducer.cpp:202-219`.

`prepare(configuration, input, inputLen)`:
1. Reset all stack depths to 0
2. Set `stateIndexStack[0] = 0` and `currentTransitionStack[0] = 0` (start state)
3. Map each input character via `stringToSymbol` lookup:
   - **Unweighted**: unknown characters get `unknownSymbolOrdinal`; returns `false` if any unknown but continues
   - **Weighted**: unknown characters cause immediate `return false` (no analysis possible)
4. Store mapped symbol indices in `inputSymbolStack`

### 3.3 Next (DFS Traversal with Backtracking)

This is the core algorithm. Both `next()` and `nextPrefix()` (unweighted) and the three `next()` overloads (weighted) use the same pattern. The algorithm is an **iterative DFS with explicit stack and backtracking**, yielding one result per call (coroutine-like).

#### Pseudocode (Unweighted `nextPrefix`, `UnweightedTransducer.cpp:289-370`)

```
function nextPrefix(config, outputBuffer, bufferLen, prefixLength):
    loopCounter = 0
    while loopCounter < MAX_LOOP_COUNT (100,000):
        stateHead = transitionStart[config.stateIndexStack[config.stackDepth]]
        currentTrans = transitionStart[config.currentTransitionStack[config.stackDepth]]
        startIndex = currentTrans - stateHead
        maxTc = getMaxTc(stateHead)

        for tc = startIndex to maxTc:
            if tc == 1 AND maxTc >= 255:
                tc++; currentTrans++    // skip overflow cell

            if currentTrans.symIn == 0xFFFF:    // FINAL STATE
                if config.inputDepth == config.inputLength OR prefixLength != null:
                    // BUILD OUTPUT: concatenate symbolToString for each
                    // outputSymbolStack[0..stackDepth-1]
                    assemble output string into outputBuffer
                    config.currentTransitionStack[stackDepth] = currentTrans + 1
                    if prefixLength: *prefixLength = config.inputDepth
                    return true

            else if (input not exhausted AND inputSymbol matches currentTrans.symIn)
                 OR (currentTrans.symIn < firstNormalChar AND flagDiacriticCheck passes):
                // PUSH (go down)
                if stackDepth + 2 == bufferSize: return false   // stack overflow
                save output symbol (0 if flag/epsilon, symOut if normal)
                save current transition index
                stackDepth++
                set new state = targetState
                if symIn >= firstNormalChar: inputDepth++
                goto nextInMainLoop

            currentTrans++

        // All transitions exhausted at this level
        if config.stackDepth == 0:
            return false    // no more results

        // POP (backtrack up)
        stackDepth--
        previousSymIn = transition at saved position's symIn
        if previousSymIn >= firstNormalChar:
            inputDepth--
        else if flag diacritic:
            flagDepth--
            restore previous flag value from updatedFlagFeature/updatedFlagValue

        currentTransitionStack[stackDepth]++    // advance to next sibling

        nextInMainLoop:
        loopCounter++

    return false    // loop limit reached
```

#### Weighted Variant Differences (`WeightedTransducer.cpp:298-406`)

1. **Sorted transitions enable binary search**: When `currentTrans.symIn >= firstNormalChar` and `currentTrans.symIn < inputSym`, a binary search skips ahead to find the matching input symbol (`WeightedTransducer.cpp:366-381`).

2. **Early break on exhausted input**: When input is exhausted (`inputSym == 0`) and the current transition's symIn is a normal character, the inner loop breaks immediately (`WeightedTransducer.cpp:337-339`).

3. **Early break on overshoot**: When `currentTrans.symIn > inputSym`, the inner loop breaks (`WeightedTransducer.cpp:363-364`).

4. **Weight computation**: Upon reaching a final state, the total weight is the sum of the final transition's weight plus all weights along the path:
   ```cpp
   *weight = currentTransition->weight;
   for (int i = 0; i < configuration->stackDepth; i++) {
       *weight += (transitionStart + configuration->currentTransitionStack[i])->weight;
   }
   ```

5. **`firstNotReachedPosition` tracking**: Tracks the deepest input position reached across all branches. Used by `VfstSuggestion` to prune the error model's search space.

6. **`backtrackToOutputDepth`** (`WeightedTransducer.cpp:408-428`): Allows the suggestion generator to rewind the traversal to a specific output depth, popping stack frames and restoring input depth accordingly.

7. **Flag diacritics use copy-on-push**: The entire flag array for the current depth is copied forward before modification (`WeightedTransducer.cpp:279`), so backtracking is just `flagDepth--`.

### 3.4 Output Assembly

When a final state is reached, the output string is built by iterating `outputSymbolStack[0..stackDepth-1]`, looking up each index in `symbolToString[]`, and concatenating. Symbols with index < `firstNormalChar` (epsilon, flags) were stored as 0 in `outputSymbolStack` and thus contribute nothing (epsilon has zero length).

### 3.5 Loop Limit

Both variants use `MAX_LOOP_COUNT = 100,000` (`Transducer.hpp:57`) as a safety limit on the outer while loop iterations. If exceeded, `next()` returns false.

---

## 4. Flag Diacritic Handling

### 4.1 Operations Supported

**Source**: `Transducer.hpp:41-47`, `UnweightedTransducer.cpp:228-283`, `WeightedTransducer.cpp:230-286`.

Five operations are supported (no `N` -- Negative). The `Operation` enum:

| Enum Value | Symbol | Semantics |
|------------|--------|-----------|
| `Operation_P` | `@P.FEAT.VAL@` | **Positive Set**: unconditionally set feature to value |
| `Operation_C` | `@C.FEAT@` | **Clear**: reset feature to neutral (0) |
| `Operation_U` | `@U.FEAT.VAL@` | **Unification**: if feature is neutral, set it; if already set to same value, pass; if set to different value, fail |
| `Operation_R` | `@R.FEAT.VAL@` or `@R.FEAT@` | **Require**: if val=Any, require feature is non-neutral; if val=specific, require feature equals that value |
| `Operation_D` | `@D.FEAT.VAL@` or `@D.FEAT@` | **Disallow**: if val=Any, require feature IS neutral; if val=specific, require feature does NOT equal that value |

### 4.2 Check Algorithm Detail

```
function flagDiacriticCheck(config, transducer, symbol):
    if no flag features or symbol == 0 (epsilon):
        return true

    ofv = transducer.symbolToDiacritic[symbol]
    currentValue = currentFlagArray[ofv.feature]

    switch ofv.op:
        P (Positive Set):
            mark for update
        C (Clear):
            set ofv.value = FlagValueNeutral(0), mark for update
        U (Unification):
            if currentValue != 0:
                if currentValue != ofv.value: return false  // conflict
                // else: already unified, no update needed
            else:
                mark for update  // set from neutral
        R (Require):
            if ofv.value == FlagValueAny(1):
                if currentValue == FlagValueNeutral(0): return false  // not set
            else:
                if currentValue != ofv.value: return false  // wrong value
        D (Disallow):
            if ofv.value == FlagValueAny(1):
                if currentValue != FlagValueNeutral(0): return false  // is set
            else:
                if currentValue == ofv.value: return false  // has disallowed value

    // Save old value for backtracking (unweighted) or copy array (weighted)
    save/copy flag state
    if marked for update:
        set currentFlagArray[ofv.feature] = ofv.value
    flagDepth++
    return true
```

### 4.3 Backtracking Flag State

**Unweighted** (destructive + undo):
- On push: save `(feature, oldValue)` at `flagDepth` position
- On pop: `currentFlagValues[updatedFlagFeature[flagDepth]] = updatedFlagValue[flagDepth]`

**Weighted** (copy-on-push):
- On push: `memcpy(currentArray + featureCount, currentArray, featureCount * sizeof(uint32_t))`, then modify the copy
- On pop: just `flagDepth--` (the previous array slice is still intact)

The weighted approach uses more memory (`featureCount * bufferSize * 4` bytes) but has simpler backtracking. The unweighted approach is more memory-efficient.

---

## 5. Key Data Structures Summary

### 5.1 `transinfo_t` Bitfield (4 bytes)

```
Bits 0-23:  targetState (24 bits) -- max 16,777,215 states
Bits 24-31: moreTransitions (8 bits) -- max 254 inline, 255 = overflow
```

**Platform concern**: C++ bitfield layout is implementation-defined. The code relies on `targetState` occupying the low 24 bits on little-endian platforms. The byte-swap code explicitly handles endianness.

### 5.2 Class Hierarchy

```
Transducer (base)
  |-- fileLength: size_t
  |-- map: void*                        // mmap'd region
  |-- byteSwapped: bool
  |-- flagDiacriticFeatureCount: uint16_t
  |-- firstNormalChar: uint16_t
  |-- symbolToDiacritic: vector<OpFeatureValue>
  |
  +-- UnweightedTransducer
  |     |-- transitionStart: Transition*
  |     |-- stringToSymbol: map<wchar_t, uint16_t>
  |     |-- symbolToString: vector<wchar_t*>
  |     |-- symbolStringLength: vector<size_t>
  |     |-- firstMultiChar: uint16_t
  |     |-- unknownSymbolOrdinal: uint16_t
  |
  +-- WeightedTransducer
        |-- transitionStart: WeightedTransition*
        |-- stringToSymbol: map<wchar_t, uint16_t>
        |-- symbolToString: vector<wchar_t*>
        |-- symbolStringLength: vector<size_t>
        |-- firstMultiChar: uint16_t
```

### 5.3 OpFeatureValue

```
struct OpFeatureValue {
    Operation op;       // enum: P, C, U, R, D
    uint16_t feature;   // index into feature array
    uint16_t value;     // value index (0=neutral, 1=any, 2+=specific)
};
```

---

## 6. Interface with Other Modules

### 6.1 Morphology

**FinnishVfstAnalyzer** (`morphology/FinnishVfstAnalyzer.cpp`):
- Uses `UnweightedTransducer` with file `mor.vfst`
- Pattern: `prepare() -> while(next()) { process output }`
- Buffer size: 2000, max 100 analyses
- Lowercases input before analysis
- Output is parsed for Finnish morphological tags (`[Ln]`, `[Bc]`, etc.)

**VfstAnalyzer** (`morphology/VfstAnalyzer.cpp`):
- Uses `WeightedTransducer` with file `mor.vfst`
- Same prepare/next pattern but also extracts `weight`
- Weight is converted: `prob = exp(-0.01 * weight)`

### 6.2 Spellchecker

**VfstSpeller** (`spellchecker/VfstSpeller.cpp`):
- Uses `WeightedTransducer` with file `spl.vfst`
- Pattern: `prepare() -> next()` -- just checks if any result exists
- Also tries title-case variant for `SPELL_CAP_FIRST`

**VfstSuggestion** (`spellchecker/VfstSuggestion.cpp`):
- Uses **two** `WeightedTransducer`s: acceptor (`spl.vfst`) and error model (`err.vfst`)
- Pattern: feed misspelled word through error model, feed each error model output through acceptor
- Uses `next(config, buf, len, &weight, &firstNotReachedPosition)` variant
- Uses `backtrackToOutputDepth()` on the error model when acceptor fails, to prune search
- Collects weighted suggestions into a priority queue

### 6.3 Grammar (Autocorrect)

**VfstAutocorrectCheck** (`grammar/FinnishRuleEngine/VfstAutocorrectCheck.cpp`):
- Uses `UnweightedTransducer` with file `autocorr.vfst` (loaded by filename)
- Uses `nextPrefix()` -- finds the longest prefix match
- Pattern: tries each word-starting position in a sentence; if prefix match aligns with a word boundary, it's an autocorrect suggestion

### 6.4 Public API Surface of the FST Engine

The FST engine exposes these methods to consumers:

```
// Transducer (base)
uint16_t getFlagDiacriticFeatureCount() const;
void terminate();

// UnweightedTransducer
explicit UnweightedTransducer(const char* filePath);
bool prepare(Configuration*, const wchar_t* input, size_t inputLen) const;
bool next(Configuration*, wchar_t* outputBuffer, size_t bufferLen) const;
bool nextPrefix(Configuration*, wchar_t* outputBuffer, size_t bufferLen, size_t* prefixLength) const;

// WeightedTransducer
explicit WeightedTransducer(const char* filePath);
bool prepare(WeightedConfiguration*, const wchar_t* input, size_t inputLen) const;
bool next(WeightedConfiguration*, wchar_t* outputBuffer, size_t bufferLen) const;
bool next(WeightedConfiguration*, wchar_t* outputBuffer, size_t bufferLen, int16_t* weight) const;
bool next(WeightedConfiguration*, wchar_t* outputBuffer, size_t bufferLen, int16_t* weight,
          int* firstNotReachedPosition) const;
void backtrackToOutputDepth(WeightedConfiguration*, int depth);
```

All consumers follow the same lifecycle:
1. Construct transducer (loads and parses file)
2. Create Configuration/WeightedConfiguration with `featureCount` and `bufferSize`
3. For each word: `prepare()` then loop `next()` / `nextPrefix()`
4. At shutdown: `terminate()` (releases mmap/memory)

---

## 7. Rust Type Mapping Proposals

### 7.1 Binary Format Structures

```rust
// --- Unweighted ---

/// 8 bytes, matching C++ Transition exactly
#[repr(C)]
struct Transition {
    sym_in: u16,
    sym_out: u16,
    // transinfo_t as raw u32 to avoid bitfield portability issues
    trans_info: u32,
}

impl Transition {
    fn target_state(&self) -> u32 {
        self.trans_info & 0x00FF_FFFF
    }
    fn more_transitions(&self) -> u8 {
        (self.trans_info >> 24) as u8
    }
}

/// 8 bytes
#[repr(C)]
struct OverflowCell {
    more_transitions: u32,
    _padding: u32,
}

// --- Weighted ---

/// 16 bytes
#[repr(C)]
struct WeightedTransition {
    sym_in: u32,
    sym_out: u32,
    target_state: u32,
    weight: i16,
    more_transitions: u8,
    _reserved: u8,
}

/// 16 bytes
#[repr(C)]
struct WeightedOverflowCell {
    more_transitions: u32,
    _short_padding: u32,
    _padding: u64,
}
```

**Rationale for `trans_info` as raw u32**: C/C++ bitfield layout is implementation-defined. By treating it as a raw u32 and extracting fields with bit masks, we get portable, predictable behavior across all Rust targets.

### 7.2 Symbol Table

```rust
struct SymbolTable {
    /// Symbol index -> string (stored as String, not wchar_t*)
    symbol_strings: Vec<String>,
    /// Symbol index -> string length in chars (for output assembly)
    symbol_lengths: Vec<usize>,
    /// Single character -> symbol index (for input mapping)
    char_to_symbol: HashMap<char, u16>,  // or BTreeMap for cache friendliness
    /// Symbol index -> flag diacritic operation (only for indices < first_normal_char)
    flag_diacritics: Vec<OpFeatureValue>,
    /// Boundary indices
    first_normal_char: u16,
    first_multi_char: u16,
    /// Feature count for flag diacritics
    flag_feature_count: u16,
}
```

**wchar_t elimination**: The C++ code uses `wchar_t` (which is 4 bytes on Linux/macOS, 2 bytes on Windows). Rust's `char` is always a Unicode scalar value (4 bytes). For WASM, we should use Rust `char`/`String` internally and convert to/from UTF-8 at the API boundary. This eliminates the `wchar_t` portability concern entirely.

### 7.3 Flag Diacritics

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
enum FlagOp {
    P,  // Positive set
    C,  // Clear
    U,  // Unification
    R,  // Require
    D,  // Disallow
}

const FLAG_VALUE_NEUTRAL: u16 = 0;
const FLAG_VALUE_ANY: u16 = 1;

#[derive(Clone, Copy, Debug)]
struct OpFeatureValue {
    op: FlagOp,
    feature: u16,
    value: u16,
}

/// Flag state for traversal
struct FlagState {
    /// Current values indexed by feature
    values: Vec<u16>,
}
```

### 7.4 Configuration (Traversal State)

```rust
struct TraversalConfig {
    buffer_size: usize,
    stack_depth: usize,
    flag_depth: usize,
    input_depth: usize,
    input_length: usize,

    state_index_stack: Vec<u32>,
    current_transition_stack: Vec<u32>,
    input_symbol_stack: Vec<u32>,      // u16 for unweighted, u32 for weighted
    output_symbol_stack: Vec<u32>,

    // Flag state -- use one of these strategies:
    flag_values: Vec<u16>,             // unweighted: single mutable array + undo stack
    flag_undo_feature: Vec<u16>,       // unweighted: which feature was changed
    flag_undo_value: Vec<u16>,         // unweighted: what it was before
    // OR for weighted:
    // flag_value_stack: Vec<u32>,     // flattened 2D: [depth * feature_count + feature]
}
```

For the Rust port, consider unifying unweighted and weighted into a single generic implementation parameterized by the transition type, since the traversal algorithm is nearly identical.

### 7.5 Transducer

```rust
/// Trait for both weighted and unweighted transducers
trait Transducer {
    type Config;

    fn prepare(&self, config: &mut Self::Config, input: &str) -> bool;
    fn next(&self, config: &mut Self::Config, output: &mut String) -> bool;
}

struct VfstTransducer<T: TransitionType> {
    /// The raw binary data (owned Vec<u8> for WASM, or mmap for native)
    data: TransducerData,
    /// Pointer/offset to start of transition table within data
    transition_offset: usize,
    /// Symbol table
    symbols: SymbolTable,
}

enum TransducerData {
    Owned(Vec<u8>),     // For WASM: loaded via fetch, no mmap
    // Mmap(MmapHandle), // For native: could add later
}
```

### 7.6 WASM-Specific Considerations

For WASM, `mmap` is unavailable. The binary data should be loaded as a `Vec<u8>` (e.g., via `fetch` on the JS side, passed as `Uint8Array`). The transition table can be accessed via safe slice operations with bounds checking, or via `unsafe` pointer casting for performance-critical paths.

---

## 8. Complexity Estimates

### 8.1 Component Breakdown

| Component | C++ Lines | Rust Difficulty | Estimated Rust Lines | Notes |
|-----------|-----------|-----------------|---------------------|-------|
| Binary format parser (header, symbols) | ~120 | Low | ~150 | Straightforward byte parsing. No mmap needed for WASM. |
| Transition table access | ~40 | Low-Medium | ~60 | Bitfield extraction for unweighted `transinfo_t` needs care. Use explicit bit masks. |
| Byte-swap support | ~100 | Low | ~80 | Could simplify: always store LE, use `u16::from_le_bytes()` etc. Or skip entirely if we control the build pipeline. |
| Symbol table construction | ~80 | Low | ~100 | Replace `wchar_t*` with `String`. Use `HashMap<char, u16>` instead of `map<wchar_t, uint16_t>`. |
| Flag diacritic parsing | ~60 | Low | ~60 | Direct port of string parsing logic. |
| Flag diacritic checking | ~55 | Low | ~50 | Simple match/switch on 5 operations. |
| Unweighted traversal (`next`/`nextPrefix`) | ~85 | Medium | ~120 | The `goto nextInMainLoop` needs restructuring. The coroutine-like yield-on-final pattern is idiomatic in Rust with explicit state. |
| Weighted traversal (`next` with weight) | ~110 | Medium | ~140 | Same as unweighted plus binary search, weight accumulation, `firstNotReachedPosition`. |
| `backtrackToOutputDepth` | ~20 | Low | ~25 | Simple stack unwinding. |
| Configuration structs | ~60 | Low | ~40 | Replace raw arrays with `Vec`. |
| Memory management (mmap/munmap) | ~50 | N/A for WASM | ~10 | WASM: just `Vec<u8>`. Native: could use `memmap2` crate. |
| **Total** | **~780** | | **~835** | |

### 8.2 Tricky Areas for Rust Port

1. **Bitfield `transinfo_t`**: C++ uses a compiler-dependent bitfield. Rust solution: store as raw `u32`, extract with bit masks. This is actually more portable.

2. **`goto nextInMainLoop`**: The C++ uses `goto` to skip the backtracking/exhaustion check when pushing down. In Rust, use `continue` with a labeled outer loop, or restructure into a state machine with explicit enum states.

3. **`wchar_t` (4 bytes on Unix, 2 bytes on Windows)**: Eliminated entirely in Rust. Use `char` (always 4 bytes, Unicode scalar value). For WASM, all I/O is UTF-8 at the boundary.

4. **mmap**: Not available in WASM. Use `Vec<u8>` loaded from `ArrayBuffer`. For native builds, the `memmap2` crate provides cross-platform mmap.

5. **Coroutine-like `next()`**: The C++ `next()` function resumes from where it left off (saved in `currentTransitionStack`). This pattern maps naturally to Rust -- the configuration struct IS the coroutine state. No async/generators needed.

6. **Unsafe for performance**: Transition table access could use `unsafe` pointer arithmetic for zero-copy access to the mmap'd/loaded data, similar to C++. Alternative: safe `bytemuck` or `zerocopy` crate for `#[repr(C)]` struct casting. For a first pass, safe bounds-checked indexing is fine and can be optimized later.

7. **Two traversal variants**: The unweighted and weighted traversal algorithms are ~80% identical. Consider a generic implementation parameterized by a `TransitionAccess` trait:
   ```rust
   trait TransitionAccess {
       fn sym_in(&self, idx: u32) -> u32;
       fn sym_out(&self, idx: u32) -> u32;
       fn target_state(&self, idx: u32) -> u32;
       fn more_transitions(&self, idx: u32) -> u8;
       fn weight(&self, idx: u32) -> i16;  // returns 0 for unweighted
   }
   ```

### 8.3 Simplification Opportunities

1. **Drop byte-swap support**: If we control the dictionary build pipeline for WASM, always emit little-endian (WASM is LE). Saves ~80 lines and complexity.

2. **Unify Configuration types**: Use a single `Config` struct with `u32` symbol stacks (upcast from `u16` for unweighted). Slight memory overhead but simpler code.

3. **Use Rust strings throughout**: No `wchar_t` conversion overhead. Symbols stored as `&str` or `String`, input accepted as `&str`, output produced as `String`.

4. **Iterator-based API**: Instead of the C-style `prepare()/next()` pair, expose a Rust iterator:
   ```rust
   let results: Vec<AnalysisResult> = transducer.analyze("koirille").collect();
   ```

---

## 9. File Map: VFST Binary Layout (Visual)

```
+------------------+  offset 0
|  Header (16 B)   |
|  cookie1 (4B)    |
|  cookie2 (4B)    |
|  weighted (1B)   |
|  reserved (7B)   |
+------------------+  offset 16
|  Symbol Count    |  uint16_t
+------------------+  offset 18
|  Symbol[0] = ""  |  1 byte (just null)
|  Symbol[1] = "@P.|  null-terminated UTF-8
|  ...             |
|  Symbol[N-1]     |
+------------------+  variable
|  Padding (0-7B   |  align to 8B (unweighted)
|   or 0-15B)      |  align to 16B (weighted)
+------------------+  aligned offset
|  Transition[0]   |  State 0, first transition
|  Transition[1]   |  (or OverflowCell if moreTransitions==255)
|  ...             |
|  Transition[M]   |  State 0, last transition
+------------------+
|  Transition[M+1] |  State 1, first transition
|  ...             |
+------------------+
|  ...             |  remaining states
+------------------+  EOF
```

---

## 10. Test Strategy Notes

The existing test infrastructure provides:

- `AllAutomaticTests.py`: runs without dictionary (null component + dictionary info tests)
- `libvoikkoTest.py`: full API tests requiring Finnish dictionary

For the Rust port, additional unit tests should cover:
1. Binary format parsing with crafted test `.vfst` files
2. Flag diacritic operations (all 5 ops with edge cases)
3. Traversal correctness: compare Rust output against C++ output for the same dictionary
4. Byte-order handling (if retained)
5. Buffer overflow handling (stack depth limits, output buffer limits)
6. Loop limit enforcement (`MAX_LOOP_COUNT`)

The `voikkovfstc` tool can generate test `.vfst` files from ATT-format transducers, enabling creation of small, focused test cases.
