# Module-Level Architecture Analysis for Rust WASM Port

## Table of Contents

1. [Spellchecker Module](#1-spellchecker-module)
2. [Suggestion Strategies](#2-suggestion-strategies)
3. [Morphology Module](#3-morphology-module)
4. [Grammar Module](#4-grammar-module)
5. [Hyphenation Module](#5-hyphenation-module)
6. [Tokenizer Module](#6-tokenizer-module)
7. [Cross-Cutting Analysis](#7-cross-cutting-analysis)
8. [Porting Risk Assessment](#8-porting-risk-assessment)

---

## 1. Spellchecker Module

**Source:** `libvoikko/src/spellchecker/`

### Architecture Overview

The spellchecker has a layered architecture with multiple backend implementations behind an abstract `Speller` interface.

```
voikkoSpellUcs4 (public C API)
  |
  +-- normalization, case detection, option handling
  |
  +-- voikko_cached_spell
  |     |
  |     +-- SpellerCache (hash-based, words <= 10 chars)
  |     |
  |     +-- hyphenAwareSpell
  |           |
  |           +-- Speller::spell (virtual)
  |                 |
  |                 +-- FinnishSpellerTweaksWrapper (Finnish)
  |                 |     |-- AnalyzerToSpellerAdapter
  |                 |     |-- soft hyphen validation
  |                 |     |-- optional hyphen handling
  |                 |     |-- ambiguous compound resolution
  |                 |
  |                 +-- VfstSpeller (experimental, direct FST)
  |                 +-- HfstSpeller (HFST backend)
```

### spell.cpp -- Top-Level Spell Check Pipeline

The public entry point `voikkoSpellUcs4` performs these steps:

```
function voikkoSpellUcs4(word):
  1. Normalize Unicode (voikko_normalise)
  2. If ignore_numbers and word contains digit -> OK
  3. Detect case type: ALL_LOWER, FIRST_UPPER, ALL_UPPER, COMPLEX, NO_LETTERS
  4. If ignore_uppercase and ALL_UPPER -> OK
  5. If ignore_nonwords and is_nonword -> OK
  6. Convert word to all-lowercase buffer
  7. Handle trailing dot (if ignore_dot)

  8. For COMPLEX/NO_LETTERS case:
     - Keep original case except lowercase first char
     - Call hyphenAwareSpell directly (no cache)
     - Check SPELL_CAP_FIRST against accept_first_uppercase

  9. For simple cases (ALL_LOWER, FIRST_UPPER, ALL_UPPER):
     - Call voikko_cached_spell with lowercase buffer
     - Map spellresult to VOIKKO_SPELL_OK/FAILED based on case type
     - If failed, retry with trailing dot
```

**Key insight:** The word is always lowercased before FST lookup. Case correctness is validated separately via the STRUCTURE attribute from morphological analysis.

### spellresult Enum

The internal `spellresult` has four values with an ordering:
- `SPELL_OK` (best) -- word is correctly spelled as-is
- `SPELL_CAP_FIRST` -- word requires first letter capitalized
- `SPELL_CAP_ERROR` -- capitalization does not match any valid form
- `SPELL_FAILED` (worst) -- word not found in dictionary

### VfstSpeller (Direct FST Speller)

Simple implementation for the experimental `vfst` backend:

```
function spell(word, wlen):
  result = transducer.prepare(word) && transducer.next()
  if result: return SPELL_OK

  if failed and first char is lowercase:
    try with first char uppercased
    if succeeds: return SPELL_CAP_FIRST

  return SPELL_FAILED
```

Uses `spl.vfst` -- a dedicated spelling transducer (separate from morphology transducer `mor.vfst`).

### AnalyzerToSpellerAdapter

Bridges the morphological analyzer to the speller interface. This is the main production path for Finnish.

```
function spell(word, wlen):
  analyses = analyzer.analyze(word, wlen, fullMorphology=false)
  if empty: return SPELL_FAILED

  best_result = SPELL_FAILED
  for each analysis:
    structure = analysis.getValue(STRUCTURE)
    result = SpellUtils::matchWordAndAnalysis(word, wlen, structure)
    best_result = min(best_result, result)  // SPELL_OK < CAP_FIRST < CAP_ERROR
    if best_result == SPELL_OK: break early

  return best_result
```

### SpellUtils::matchWordAndAnalysis

The STRUCTURE attribute is a character-by-character encoding of expected case:
- `i` / `j` = uppercase letter expected (j = abbreviation context)
- `p` / `q` = lowercase letter expected (q = abbreviation context)
- `=` = compound boundary (skipped)
- `-` = literal hyphen
- `:` = literal colon

```
function matchWordAndAnalysis(word, len, structure):
  result = SPELL_OK
  j = 0  // structure position
  for i in 0..len:
    skip '=' in structure

    captype = classify(word[i])  // 'i'=upper, 'p'=lower, 'v'=punctuation

    if captype=='p' and structure[j] in {'i','j'}:
      if i==0: result = SPELL_CAP_FIRST
      else: result = SPELL_CAP_ERROR; break

    if captype=='i' and structure[j] in {'p','q'}:
      result = SPELL_CAP_ERROR; break

    j++
  return result
```

### FinnishSpellerTweaksWrapper (Finnish-Specific)

Wraps a base speller with Finnish-specific rules:

1. **Soft hyphen handling:** Strips U+00AD characters, validates the stripped word, then verifies that soft hyphen positions match valid hyphenation points using the Finnish hyphenator.

2. **Optional hyphen handling:** For words containing `-`, tries removing the hyphen if `accept_extra_hyphens` is set. Recurses for multiple hyphens (potential deep recursion -- noted as FIXME).

3. **Vowel-consonant overlap pattern:** For `pop-opisto` style words where the leading part ends with VC and the trailing part starts with the same VC pair.

4. **Free suffix part:** For `ja-sana` patterns, checks if the leading part is any valid word and the trailing part has `MALAGA_VAPAA_JALKIOSA=true`.

5. **Ambiguous compound:** For `syy-silta` / `syys-ilta` patterns, removes the hyphen, analyzes the result, and checks if any analysis has a compound boundary (`=` in STRUCTURE) at the hyphen position.

### SpellerCache

A fixed-size, hash-based cache for words up to 10 characters:

- Hash function: `hash = (hash * 37 + char) % (1 << order)`
- Cache organized by word length (1-10), each length bucket has power-of-2 slots
- Total size: `sizeof(wchar_t) * 6544 * (1 << sizeParam)` for word storage
- Only caches `SPELL_OK` (stored as `'p'`) and `SPELL_CAP_FIRST` (stored as `'i'`)
- `SPELL_FAILED` results are NOT cached
- Simple replacement strategy: hash collisions overwrite silently

### SpellWithPriority

Used by the suggestion system to rank suggestions. Computes a priority score from:

1. **Word class and inflection** (sijamuoto): nominative=2, genitive=3, partitive=5, etc. up to 60
2. **Compound structure**: `1 << (3 * (partCount - 1))` -- exponentially penalizes compounds with many parts
3. **Spell result**: OK=1, CAP_FIRST=2, CAP_ERROR=3

Final priority = class_priority * structure_priority * spell_priority

### Memory Ownership

- `spell.cpp`: Allocates `nword` (normalized) and `buffer` (lowercase) on heap, deletes both before return
- `SpellerCache`: Owns word array and result array; no external ownership transfer
- `FinnishSpellerTweaksWrapper`: Owns its inner `speller` and `hyphenator`; deletes both in `terminate()`
- `AnalyzerToSpellerAdapter`: Does NOT own its analyzer; just holds a pointer

---

## 2. Suggestion Strategies

**Source:** `libvoikko/src/spellchecker/suggestion/`

### Architecture

```
SuggestionGenerator (interface)
  |
  +-- SuggestionStrategy (abstract, contains generator lists)
  |     |
  |     +-- SuggestionStrategyTyping (Finnish keyboard errors)
  |     +-- SuggestionStrategyOcr (OCR misrecognition)
  |
  +-- VfstSuggestion (VFST error-model transducer)
  +-- HfstSuggestion (HFST speller's built-in suggestions)
  +-- SuggestionGeneratorNull (no-op)
  |
  +-- Individual generators (used by SuggestionStrategy):
      +-- SuggestionGeneratorCaseChange
      +-- SuggestionGeneratorSoftHyphens
      +-- SuggestionGeneratorVowelChange      [FINNISH-SPECIFIC]
      +-- SuggestionGeneratorReplacement
      +-- SuggestionGeneratorReplaceTwo
      +-- SuggestionGeneratorMultiReplacement
      +-- SuggestionGeneratorDeletion
      +-- SuggestionGeneratorDeleteTwo
      +-- SuggestionGeneratorInsertion
      +-- SuggestionGeneratorInsertSpecial
      +-- SuggestionGeneratorSplitWord
      +-- SuggestionGeneratorSwap
```

### SuggestionStrategy Orchestration

`SuggestionStrategy` holds two lists: `primaryGenerators` and `generators`.

```
function generate(status):
  status.setMaxCost(this.maxCost)

  for gen in primaryGenerators:
    if status.shouldAbort(): break
    gen.generate(status)

  if status.suggestionCount > 0: return  // primary found something

  for gen in generators:
    if status.shouldAbort(): break
    gen.generate(status)
```

### SuggestionStatus -- Abort Logic

```
function shouldAbort():
  if suggestionCount == maxSuggestions: return true
  if currentCost < maxCost: return false
  if suggestionCount == 0 and currentCost < 2 * maxCost: return false  // double budget if nothing found
  return true
```

Every spell check attempt increments `currentCost` by 1 via `charge()`. The typing strategy has maxCost=800, OCR has maxCost=2000.

### Suggestion Priority System

When a suggestion is added: `finalPriority = priority * (suggestionCount + 5)`

This penalizes later-found suggestions, biasing toward strategies executed earlier in the pipeline. After all generators run, suggestions are sorted by priority (insertion sort).

The public API returns at most 5 suggestions (`MAX_SUGGESTIONS = 5`), then applies case adjustment to match the original word's case pattern.

### Suggestion Generator Catalog

#### SuggestionGeneratorCaseChange (Primary)
- **Finnish-specific:** No
- **Algorithm:** Checks the word as-is against the analyzer. If `SPELL_CAP_FIRST`, uppercases first letter. If `SPELL_CAP_ERROR`, reads STRUCTURE to fix all letter cases.
- **Complexity:** O(1) spell checks (1-2 calls)

#### SuggestionGeneratorSoftHyphens (Primary)
- **Finnish-specific:** No
- **Algorithm:** Strips all U+00AD soft hyphens and checks the result.
- **Complexity:** O(1)

#### SuggestionGeneratorVowelChange (FINNISH-SPECIFIC)
- **Algorithm:** Finnish has vowel harmony (back vowels a/o/u vs. front vowels ae/oe/y). Enumerates all 2^n - 1 combinations of swapping front/back vowels (up to 7 vowels). Handles both lowercase and uppercase.
- **Complexity:** O(2^min(vowelCount, 7))
- **Finnish knowledge:** Back vowels (a, o, u) and front vowels (ae, oe, y) pairs

#### SuggestionGeneratorReplacement
- **Finnish-specific:** The replacement tables are Finnish keyboard-specific, but the algorithm is generic.
- **Algorithm:** For each replacement pair (from, to) in a table, replaces every occurrence of `from` with `to` in the word and checks. Also handles uppercase variants.
- **Complexity:** O(|replacements| * wordLen)

#### SuggestionGeneratorReplaceTwo
- **Finnish-specific:** No (algorithm is generic)
- **Algorithm:** Finds doubled characters (e.g., `ss`) and replaces both with the target character from the replacement table (e.g., `ss` -> `dd`).
- **Complexity:** O(|replacements| * wordLen)

#### SuggestionGeneratorMultiReplacement (OCR only)
- **Algorithm:** Recursive: applies up to N replacements from the table simultaneously. Used for OCR where multiple characters may be misrecognized.
- **Complexity:** O(|replacements|^N * wordLen^N) -- exponential, but N is typically 2

#### SuggestionGeneratorDeletion
- **Finnish-specific:** No
- **Algorithm:** Tries deleting each character (skipping consecutive duplicates). Uses `SuggestionGeneratorCaseChange::suggestForBuffer` to validate.
- **Complexity:** O(wordLen)

#### SuggestionGeneratorDeleteTwo
- **Finnish-specific:** No
- **Algorithm:** For words >= 6 chars, finds positions where 2-char substrings repeat (e.g., `abab` -> try `ab`). Removes the duplicate 2-char sequence.
- **Complexity:** O(wordLen)

#### SuggestionGeneratorInsertion
- **Finnish-specific:** The character lists are Finnish-frequency-ordered, but algorithm is generic.
- **Algorithm:** Tries inserting each character from a priority-ordered list at every position. Skips duplicates.
- **Complexity:** O(|charList| * wordLen)

#### SuggestionGeneratorInsertSpecial
- **Finnish-specific:** No
- **Algorithm:** Two strategies:
  1. Insert `-` at positions 2..len-2 (avoiding adjacent hyphens)
  2. Duplicate each character (avoiding already-doubled chars)
- **Complexity:** O(wordLen)

#### SuggestionGeneratorSplitWord
- **Finnish-specific:** No
- **Algorithm:** Tries splitting the word into two parts at every position. Both parts must pass spell check. Handles dots between parts and hyphen-separated words (`suuntaa-antava` -> `suuntaa antava`).
- **Complexity:** O(wordLen) spell checks

#### SuggestionGeneratorSwap
- **Finnish-specific:** Partially (skips front/back vowel swaps already tested by VowelChange)
- **Algorithm:** Swaps pairs of characters within a distance limit (50/wordLen for long words, up to 10 for words <= 8 chars).
- **Complexity:** O(wordLen * maxDistance)

### VfstSuggestion (Transducer-Based)

Uses a dedicated error model transducer (`err.vfst`) composed with the acceptor:

```
function generate(status):
  suggestionWeights = {}

  errorModel.prepare(word)
  while not abort and errorModel.next() -> (errorOutput, errorWeight):
    acceptor.prepare(errorOutput)
    if acceptor.next() -> (_, acceptorWeight):
      weight = acceptorWeight + errorWeight
      suggestionWeights[errorOutput] = min(existing, weight)
    else:
      errorModel.backtrackToOutputDepth(firstNotReachedPosition)

  sort by weight, add to status
```

This is the most sophisticated suggestion mechanism -- the error model FST encodes common misspellings and the acceptor validates the result.

### Typing Strategy Generator Order (Finnish)

| Order | Generator | Replacement Table |
|-------|-----------|-------------------|
| Primary 1 | CaseChange | -- |
| Primary 2 | SoftHyphens | -- |
| 1 | VowelChange | -- |
| 2 | Replacement | REPLACEMENTS_1 (highest-freq keyboard neighbors) |
| 3 | Deletion | -- |
| 4 | InsertSpecial | -- |
| 5 | SplitWord | -- |
| 6 | ReplaceTwo | REPLACEMENTS_1 |
| 7 | Replacement | REPLACEMENTS_2 (number row) |
| 8 | Insertion | "aitesn" (most common Finnish letters) |
| 9 | Swap | -- |
| 10 | Replacement | REPLACEMENTS_3 |
| 11 | Insertion | remaining letters |
| 12 | Replacement | REPLACEMENTS_4 |
| 13-15 | ReplaceTwo | REPLACEMENTS_2, 3, 4 |
| 16 | DeleteTwo | -- |
| 17 | Replacement | REPLACEMENTS_5 |

The replacement tables encode Finnish keyboard adjacency. For example, REPLACEMENTS_1 includes `ae`->`oe` proximity, `s`->`sh` (s -> accent-s), etc.

---

## 3. Morphology Module

**Source:** `libvoikko/src/morphology/`

### Architecture

```
Analyzer (abstract interface)
  |
  +-- FinnishVfstAnalyzer  [Finnish VFST, main production path]
  +-- VfstAnalyzer          [Generic VFST, minimal parsing]
  +-- HfstAnalyzer          [HFST backend]
  +-- LttoolboxAnalyzer     [Apertium backend]
  +-- NullAnalyzer          [no-op]
```

### VfstAnalyzer (Generic)

Simple: lowercases input, runs through `mor.vfst` weighted transducer, returns raw FST output and weight.

```
function analyze(word, wlen, fullMorphology):
  wordLower = lowercase(word)
  prepare(wordLower)
  while next() -> (output, weight):
    analysis = new Analysis()
    if fullMorphology:
      analysis[FSTOUTPUT] = output
    analysis[WEIGHT] = exp(-0.01 * weight)
    add to list
  return list
```

### FinnishVfstAnalyzer (1,179 lines -- Why So Complex?)

This is the heart of Finnish morphological analysis. It is complex because it:

1. **Parses FST output tags** into structured morphological attributes
2. **Validates compound word structure** (hyphenation rules at morpheme boundaries)
3. **Constructs STRUCTURE strings** that encode case expectations for each character
4. **Derives base forms** from inflected forms using FST output markup
5. **Handles Finnish-specific morphological categories** (15 cases, 6 moods, participles, etc.)
6. **Generates organizational name variants** (`duplicateOrgName`)
7. **Parses numeral base forms** with special rules

#### FST Output Tag Format

The FST output contains bracketed tags interleaved with surface characters:

```
[Ln]koira[Sg][Ny]  -> class=nimisana, sijamuoto=omanto, number=singular
[Lt]luke[Tn1][Sn]  -> class=teonsana, mood=A-infinitive, sijamuoto=nimento
```

Tag categories:
- `[Lx]` -- Word class (L=lexical): n=noun, l=adjective, t=verb, etc.
- `[Sx]` -- Case (sijamuoto): n=nominative, g=genitive, p=partitive, etc.
- `[Nx]` -- Number: y=singular, m=plural
- `[Tx]` -- Mood: t=indicative, e=conditional, k=imperative, etc.
- `[Px]` -- Person: 1, 2, 3, 4 (passive)
- `[Ax]` -- Tense: p=present, i=past
- `[Cx]` -- Comparison: c=comparative, s=superlative
- `[Rx]` -- Participle: v=present_active, t=past_passive, etc.
- `[Ex]` -- Negative: t=true, f=false, b=both
- `[Fx]` -- Focus particle: kin, kaan, ko (question clitic)
- `[Ox]` -- Possessive suffix: 1y=1st singular, 3=3rd person, etc.
- `[Bx]` -- Boundary: c=compound, h=hyphen
- `[Ix]` -- Info flags: sf=suffix, cu=conditional hyphen required, ca/vj/ra/rm=various
- `[Dx]` -- Derivation: e=place name inflection, g=generic noun
- `[Xp]...[X]` -- Base form override
- `[Xr]...[X]` -- Replacement (ignored in baseform)
- `[Xs]...[X]` -- Word ID
- `[Xj]...[X]` -- Joined base form component

#### parseStructure -- Building the STRUCTURE String

Encodes each character's expected case:

```
function parseStructure(fstOutput, wlen):
  structure starts with '='
  for each character in fstOutput:
    if [Bx] boundary tag (not [Bh]):
      add '=' to structure
    if [Le] (proper noun class):
      defaultTitleCase = true, isAbbr = false
    if [La] (abbreviation class):
      isAbbr = true
    if [Xr]...[X] (explicit structure override):
      copy structure characters directly
    if regular character:
      if defaultTitleCase: add 'i' or 'j' (abbr)
      else: add 'p' or 'q' (abbr)
      decrement charsMissing
```

#### isValidAnalysis -- Compound Validation

Validates hyphenation at compound boundaries:

```
function isValidAnalysis(fstOutput):
  Track: boundaryPassed, hyphenPresent, hyphenRequired

  Rules:
  1. At [Bh] boundary: if previous boundary required hyphen and none found -> reject
  2. After boundary, at first non-tag char:
     - If same vowel on both sides of boundary -> hyphen required
     - If digit before boundary -> hyphen required
     - If [Isf] flag set -> hyphen unconditionally allowed
     - Compare hyphenRequired with hyphenPresent -> mismatch = reject
  3. Proper noun starting word + non-Ica noun ending = reject
```

#### parseBasicAttributes -- Tag Extraction

Scans FST output **backwards** from the end, extracting the last (most specific) value for each attribute category. This is because suffixes determine the final inflection.

Special handling:
- `nimisana_laatusana` class: converted to `laatusana` if comparative/superlative or if -sti adverb suffix
- Past passive participle: forces class to `laatusana`
- Prefix class (`etuliite`): set when compound ends with hyphen before [Bc]
- `kerrontosti` case: removes NUMBER attribute

#### parseBaseform -- Base Form Derivation

Complex algorithm that:
1. Extracts text from `[Xp]` and `[Xj]` tags as the "true" base form
2. For non-tagged text, uses STRUCTURE to determine capitalization
3. Special handling for compound place names (e.g., "Isolla-Britannialla" -> "Iso-Britannia")
4. Special path for numerals via `parseNumeralBaseform`

#### duplicateOrgName

For compound nouns with `[Ion]` tag (organizational name), creates a duplicate analysis with class=`nimi` and first letter forced uppercase in STRUCTURE.

#### Attribute Maps

All Finnish morphological terms are mapped from short FST codes to full Finnish names:

| Category | Code Examples | Finnish Name Examples |
|----------|--------------|----------------------|
| Class | n, l, t, ee, es, ep | nimisana, laatusana, teonsana, etunimi, sukunimi, paikannimi |
| Case | n, g, p, es, tr, ine, ela, ill, ade, abl, all, ab, ko, in, sti, ak | nimento, omanto, osanto, olento, tulento, ... |
| Mood | n1, n2, n3, n4, n5, t, e, k, m | A-infinitive through potential |
| Participle | v, a, u, t, m, e | present_active through negation |

---

## 4. Grammar Module

**Source:** `libvoikko/src/grammar/`

### Architecture

```
GrammarChecker (abstract)
  |
  +-- FinnishGrammarChecker
  |     |-- FinnishAnalysis (paragraph/sentence analyzer)
  |     |-- FinnishRuleEngine
  |           |-- CapitalizationCheck (paragraph-level)
  |           |-- gc_local_punctuation (sentence-level, free function)
  |           |-- gc_punctuation_of_quotations (sentence-level)
  |           |-- gc_repeating_words (sentence-level)
  |           |-- gc_end_punctuation (paragraph-level)
  |           |-- MissingVerbCheck (sentence-level)
  |           |-- NegativeVerbCheck (sentence-level)
  |           |-- CompoundVerbCheck (sentence-level)
  |           |-- SidesanaCheck (sentence-level)
  |           |-- RelativePronounCheck (sentence-level, TODO/empty)
  |           |-- VfstAutocorrectCheck (sentence-level, VFST-dependent)
  |
  +-- CgGrammarChecker (constraint grammar, experimental)
  +-- NullGrammarChecker
```

### Pipeline

```
1. FinnishAnalysis::analyseParagraph(text)
   -> Split text into sentences (using Sentence::next)
   -> For each sentence: tokenize, then analyseToken for each word token

2. FinnishRuleEngine::check(paragraph)
   -> For each sentence:
      a. gc_local_punctuation       -- whitespace and punctuation errors
      b. gc_punctuation_of_quotations -- quotation mark punctuation
      c. gc_repeating_words         -- duplicate adjacent words
      d. MissingVerbCheck::check    -- sentence without main verb
      e. NegativeVerbCheck::check   -- negative verb + positive verb mismatch
      f. CompoundVerbCheck::check   -- A/MA-infinitive mismatch
      g. SidesanaCheck::check       -- conjunction at end of sentence
      h. VfstAutocorrectCheck::check -- FST-based autocorrection
   -> For whole paragraph:
      i. CapitalizationCheck::check -- capitalization state machine
      j. gc_end_punctuation         -- missing period at end
```

### Grammar Error Codes

| Code | ID | Description | Finnish-Specific? |
|------|----|-------------|-------------------|
| 1 | GCERR_INVALID_SPELLING | Incorrect spelling (autocorrect) | No |
| 2 | GCERR_EXTRA_WHITESPACE | Multiple spaces | No |
| 3 | GCERR_SPACE_BEFORE_PUNCTUATION | Space before comma | No |
| 4 | GCERR_EXTRA_COMMA | Duplicate comma | No |
| 5 | GCERR_INVALID_SENTENCE_STARTER | Bad sentence-starting character | Partially (Finnish quotation marks) |
| 6 | GCERR_WRITE_FIRST_LOWERCASE | Should be lowercase | No |
| 7 | GCERR_WRITE_FIRST_UPPERCASE | Should be uppercase | No |
| 8 | GCERR_REPEATING_WORD | Word repeated | Partially (Finnish exceptions: ollut, olleet, silla) |
| 9 | GCERR_TERMINATING_PUNCTUATION_MISSING | No period at end | No |
| 10 | GCERR_INVALID_PUNCTUATION_AT_END_OF_QUOTATION | Wrong punctuation in quotes | Finnish punctuation rules |
| 11 | GCERR_FOREIGN_QUOTATION_MARK | Non-Finnish quotation mark (U+201C) | Yes |
| 12 | GCERR_MISPLACED_CLOSING_PARENTHESIS | Unmatched close paren | No |
| 13 | GCERR_NEGATIVE_VERB_MISMATCH | Negative + positive verb | Yes |
| 14 | GCERR_A_INFINITIVE_REQUIRED | Wrong infinitive form | Yes |
| 15 | GCERR_MA_INFINITIVE_REQUIRED | Wrong infinitive form | Yes |
| 16 | GCERR_MISPLACED_SIDESANA | Conjunction at sentence end | Yes |
| 17 | GCERR_MISSING_MAIN_VERB | No verb in sentence | Yes |
| 18 | GCERR_EXTRA_MAIN_VERB | Too many verbs / missing comma | Yes |

### Rule Details

#### FinnishAnalysis::analyseToken (Token Annotation)

For each word token, performs morphological analysis and sets boolean flags:
- `isValidWord` -- at least one analysis exists
- `firstLetterLcase` -- STRUCTURE says first letter should be lowercase
- `possibleSentenceStart` -- follows sentence-ending punctuation
- `isGeographicalNameInGenitive` -- place name in genitive case
- `possibleGeographicalName` -- has POSSIBLE_GEOGRAPHICAL_NAME attribute
- `possibleMainVerb` -- could be a main verb
- `isMainVerb` -- is indicative mood verb (all analyses agree)
- `isVerbNegative` -- is a negative verb (kieltosana)
- `isPositiveVerb` -- positive verb form (negative=false in 3rd person conditional)
- `isConjunction` -- is a conjunction (sidesana) or negative verb ending in -ka
- `possibleConjunction` -- at least one analysis is conjunction
- `requireFollowingVerb` -- verb requires A-infinitive or MA-infinitive next
- `verbFollowerType` -- this verb could be an A/MA-infinitive follower

#### CapitalizationCheck (State Machine)

A 5-state finite automaton operating over the paragraph:

```
States: INITIAL, UPPER, LOWER, DONT_CARE, QUOTED
Transitions driven by: word tokens, punctuation, quotes, tabs

INITIAL -> UPPER (default) | QUOTED (if inside quotes) | DONT_CARE (if bullet list)
UPPER   -> error if word starts lowercase (suggest uppercase)
         -> LOWER (normal continuation) | QUOTED | DONT_CARE (tab/title/chapter)
LOWER   -> error if word starts uppercase when firstLetterLcase=true
         -> UPPER (after sentence-ending punctuation)
DONT_CARE -> LOWER (normal) | UPPER (after sentence end)
QUOTED  -> DONT_CARE (after closing quote) | UPPER (after sentence end in quote)
```

Quote tracking uses a stack: Finnish quotation marks and parentheses are paired.

#### MissingVerbCheck (FINNISH-SPECIFIC)

Flags sentences ending with `.` or `?` that contain >= 2 words but no verb:
- A word counts as "verb found" if it is: unknown word, possibleMainVerb, or isVerbNegative
- Resets `foundVerbInCurrentClause` at conjunctions and commas
- Special cases: "siina missa" and "kavi miten" can separate clauses without comma
- Extra verb check: if two indicative verbs found in same clause, flags GCERR_EXTRA_MAIN_VERB

#### NegativeVerbCheck (FINNISH-SPECIFIC)

Checks for `[negative_verb] [space] [positive_verb]` patterns. In Finnish, the negative verb (en, et, ei, etc.) should be followed by a connegative verb form, not a regular positive form.

#### CompoundVerbCheck (FINNISH-SPECIFIC)

Checks verb pairs: if verb A requires A-infinitive follower but verb B is MA-infinitive (or vice versa), flags an error.

#### SidesanaCheck (FINNISH-SPECIFIC)

Flags sentences where the last word before `.` is a conjunction (ja, tai, mutta, etc.), excluding "vaan" (which can end a sentence as "mita vaan").

#### VfstAutocorrectCheck

Uses `autocorr.vfst` transducer to find known misspelling patterns spanning multiple words. Tries both original case and lowered-first-letter variants.

```
function check(sentence):
  Build continuous input buffer from tokens (normalize whitespace to spaces)
  For each word-start position:
    Run transducer with nextPrefix()
    If match found and ends at word boundary:
      Create GCERR_INVALID_SPELLING error with transducer output as suggestion
```

### Memory Ownership

- `FinnishAnalysis` holds a pointer to `VoikkoHandle` (does not own it)
- `Sentence` and `Paragraph` are allocated by `FinnishAnalysis`, freed by `GrammarChecker`
- Token strings are heap-allocated wchar_t arrays, freed when Sentence is destroyed
- `CacheEntry` errors are allocated by checks, transferred to `GcCache` ownership

---

## 5. Hyphenation Module

**Source:** `libvoikko/src/hyphenator/`

### Architecture

```
Hyphenator (abstract)
  |
  +-- AnalyzerToFinnishHyphenatorAdapter  [Main Finnish path]
  +-- HfstHyphenator                      [HFST backend]
```

### AnalyzerToFinnishHyphenatorAdapter Algorithm

The Finnish hyphenation algorithm operates in two phases:

#### Phase 1: Compound Splitting (splitCompounds)

```
function splitCompounds(word, len):
  analyses = analyzer.analyze(word)
  if empty and ignoreDot and ends with '.':
    strip dot, re-analyze

  for each analysis:
    interpretAnalysis -> extract compound structure from STRUCTURE attribute

  if no analyses:
    if hyphenateUnknown: allow rule hyphenation
    else: forbid all hyphenation ('X' markers)
    mark explicit '-' as '='

  removeExtraHyphenations(results, len)
```

The `interpretAnalysis` function reads the STRUCTURE attribute:
- `=` followed by letter -> `-` (compound boundary, hyphenation point)
- `-=` -> `=` (explicit hyphen at boundary, always break here)
- `j` or `q` -> `X` (abbreviation context, forbid hyphenation)

#### Phase 2: Rule-Based Syllable Hyphenation (ruleHyphenation)

Applied to each compound component separately. All Finnish-specific rules:

```
function ruleHyphenation(word, hyphenation, nchars):
  1. Skip leading consonants until first vowel

  2. -CV rule: Before consonant-vowel, add hyphen
     (e.g., "kis-sa", "koi-ra")
     Exception: not after special chars (/.:&%')

  3. 'V rule: After apostrophe before vowel, add compound break '='

  4. VV split: Before/after long vowels (aa, ee, etc.),
     split surrounding vowels

  5. V-V: For specific vowel pairs (ae, ao, ea, eo, ia, io, oa, oe,
     ua, ue, ye, eae, eoe, iae, ioe, yae, aee, oee), insert hyphen

  6. Long consonants: "shtsh", "sh-t-sh", "tsh", "t-sh", "zh"
     Move hyphen before the long consonant cluster

  7. If not uglyHyphenation:
     - Forbid hyphen at position 1 and last position
     - Forbid splitting consecutive vowels

  8. If uglyHyphenation and nchars >= 3:
     - VV-V split: after "ie" or "ai" followed by vowel
```

#### Intersection Logic

When multiple analyses exist, hyphenation points are intersected -- only positions where ALL analyses agree to hyphenate are kept. For `allPossibleHyphenPositions` (used by soft-hyphen validation), the union is taken instead.

### Finnish Phonological Constants

- `SPLIT_VOWELS`: 18 pairs of vowels that can be split (ae, ao, ea, eo, ia, io, ...)
- `LONG_CONSONANTS`: 5 sequences treated as units (shtsh, sh-t-sh, tsh, t-sh, zh)
- `SPLIT_AFTER`: 2 patterns (ie, ai) allowing VV-V split
- `VOIKKO_VOWELS` and `VOIKKO_CONSONANTS`: defined in character/charset.hpp

---

## 6. Tokenizer Module

**Source:** `libvoikko/src/tokenizer/`

### Architecture

Stateless module with one static method: `Tokenizer::nextToken`.

### Token Types

- `TOKEN_NONE` -- end of input
- `TOKEN_WORD` -- word (letters, digits, embedded punctuation)
- `TOKEN_WHITESPACE` -- one or more whitespace characters
- `TOKEN_PUNCTUATION` -- single punctuation character (or `...`)
- `TOKEN_UNKNOWN` -- unrecognized character

### Word Boundary Algorithm

```
function word_length(text, textlen, options):
  // Check for URL/email first
  urlLen = findUrlOrEmail(text, textlen)
  if urlLen != 0: return urlLen

  processing_number = false
  seenLetters = false

  for each character:
    LETTER -> continue, seenLetters = true
    DIGIT  -> continue, processing_number = true
    WHITESPACE/UNKNOWN -> end of word
    PUNCTUATION:
      ' or RIGHT_SINGLE_QUOTATION_MARK or ':' -> continue if followed by letter
      '-' / SOFT_HYPHEN / HYPHEN / NON-BREAKING_HYPHEN:
        -> include if followed by letter/digit
        -> include if at end of text or followed by whitespace
        -> end if followed by comma
      '.' -> include if followed by letter; include digits only if no letters seen
      ',' -> include only if processing_number and followed by digit
      other -> end of word
```

### nextToken Entry Point

```
function nextToken(text, textlen):
  first char type:
    LETTER/DIGIT -> return (TOKEN_WORD, word_length)
    WHITESPACE   -> scan consecutive whitespace, return (TOKEN_WHITESPACE, count)
    PUNCTUATION:
      '-'/HYPHEN -> if followed by word, include as TOKEN_WORD
      '...'      -> return (TOKEN_PUNCTUATION, 3)
      other      -> return (TOKEN_PUNCTUATION, 1)
    UNKNOWN      -> return (TOKEN_UNKNOWN, 1)
```

### URL/Email Detection

`findUrlOrEmail` handles:
- HTTP/HTTPS URLs: starts with `http://` or `https://`, includes letters/digits/punctuation until whitespace
- Email addresses: detects `@`, requires `.` after `@`, terminates at whitespace or invalid chars

### Character Classification

Uses `get_char_type()` from `character/charset.hpp` which classifies Unicode code points into CHAR_LETTER, CHAR_DIGIT, CHAR_WHITESPACE, CHAR_PUNCTUATION, CHAR_UNKNOWN.

`isFinnishQuotationMark()` recognizes Finnish-style quotes (not the LEFT DOUBLE QUOTATION MARK U+201C which is flagged as foreign).

### Finnish-Specific Elements

The tokenizer itself has minimal Finnish-specific logic:
- `isFinnishQuotationMark` used in hyphen-word boundary detection
- `ignore_dot` option affects word boundary at trailing dot

---

## 7. Cross-Cutting Analysis

### Finnish-Specific vs. Language-Agnostic Logic

| Module | Finnish-Specific | Language-Agnostic |
|--------|-----------------|-------------------|
| **Tokenizer** | Quotation mark detection | URL/email detection, character classification, word boundaries |
| **Spellchecker** | FinnishSpellerTweaksWrapper (soft hyphens, optional hyphens, ambiguous compounds) | spell.cpp (case handling, caching, normalization), SpellUtils, VfstSpeller, AnalyzerToSpellerAdapter |
| **Suggestions** | VowelChange, replacement tables (keyboard layout), insertion character order | All generator algorithms, SuggestionStrategy orchestration, SuggestionStatus |
| **Morphology** | FinnishVfstAnalyzer (all 1179 lines: tag parsing, compound validation, baseform derivation, attribute maps) | VfstAnalyzer (generic FST output), Analysis class, Analyzer interface |
| **Grammar** | MissingVerbCheck, NegativeVerbCheck, CompoundVerbCheck, SidesanaCheck, quotation rules, repeating word exceptions, error messages | CapitalizationCheck (mostly), whitespace/punctuation checks, paragraph/sentence structure |
| **Hyphenation** | AnalyzerToFinnishHyphenatorAdapter (all syllable rules, vowel/consonant tables, compound splitting) | Hyphenator interface, intersection/union logic |

### Complexity Assessment

| Module | Lines (approx) | Complexity | Key Challenge |
|--------|----------------|------------|---------------|
| FinnishVfstAnalyzer | ~1180 | **Very High** | FST output parsing, compound validation, baseform derivation with numerous edge cases |
| AnalyzerToFinnishHyphenatorAdapter | ~540 | **High** | Finnish phonological rules, compound structure interaction |
| CapitalizationCheck | ~380 | **Medium-High** | State machine with quote/parenthesis tracking |
| FinnishSpellerTweaksWrapper | ~220 | **Medium** | Recursive optional hyphen handling, compound analysis |
| SuggestionStrategyTyping | ~145 (config) + generators | **Medium** | Large replacement tables, generator ordering |
| spell.cpp | ~255 | **Medium** | Case handling state machine |
| Tokenizer | ~260 | **Medium** | URL/email detection, Unicode punctuation handling |
| SpellerCache | ~115 | **Low** | Simple hash-based cache |
| Grammar rule checks | ~50 each | **Low** | Simple pattern matching per check |

### Memory Ownership Patterns

**Dominant pattern: Manual new/delete with ownership transfer.**

Key patterns to translate to Rust:

1. **Analysis lists:** `list<Analysis *> *` is heap-allocated, caller responsible for calling `Analyzer::deleteAnalyses`. In Rust: `Vec<Analysis>` with ownership.

2. **wchar_t buffers:** Pervasive `new wchar_t[n]` / `delete[]` for temporary buffers. In Rust: `Vec<u32>` or `String` with automatic cleanup.

3. **Configuration/Transducer lifecycle:** Created in constructor, destroyed in `terminate()`. In Rust: `Drop` trait.

4. **Suggestion ownership transfer:** `SuggestionStatus::addSuggestion` takes ownership of the `wchar_t *`. If maxSuggestions exceeded, the pointer is deleted by the callee. In Rust: `Vec<String>` with push semantics.

5. **Const vs. owned attributes:** `Analysis` distinguishes between owned (`addAttribute`) and borrowed (`addConstAttribute`) string values via a bitset. In Rust: use `Cow<'static, str>` or separate types.

6. **Shared analyzer references:** Multiple modules (speller, suggestion generators, hyphenator, grammar) hold raw pointers to the same `Analyzer`. In Rust: `Arc<dyn Analyzer>` or lifetime-bounded references.

---

## 8. Porting Risk Assessment

### Critical Risks

#### Risk 1: FinnishVfstAnalyzer FST Output Parsing (HIGH)

**Impact:** If FST tag parsing has any off-by-one or logic errors, all downstream modules (spelling, suggestions, grammar, hyphenation) will malfunction.

**Specific concerns:**
- The `parseStructure` function has complex index tracking with `charsMissing`, `charsSeen`, `charsFromDefault` that must be exactly right
- `isValidAnalysis` has intricate compound boundary validation with multiple boolean flags
- `parseBaseform` handles numerals, compound place names, and joined components via different code paths
- Backward scanning in `parseBasicAttributes` with class override logic

**Mitigation:** Extensive golden-file testing with real FST output. Port the tag parser as a separate, independently testable unit. Capture test cases from the C++ implementation using instrumented builds.

#### Risk 2: Unicode/wchar_t to Rust String Handling (HIGH)

**Impact:** The entire codebase operates on `wchar_t` arrays (4 bytes on Linux/macOS). Rust strings are UTF-8. Every character index operation must be correctly translated.

**Specific concerns:**
- `wmemchr`, `wcsncmp`, `wcschr` operate on fixed-width characters; Rust str indexing is byte-based
- Many algorithms index by character position (e.g., STRUCTURE parsing, hyphenation points)
- The speller cache uses `wcsncpy` for fixed-width storage

**Mitigation:** Use `Vec<char>` (Rust char = Unicode scalar value = 4 bytes) internally for character-indexed algorithms, converting to/from UTF-8 at API boundaries. Alternatively, use a `Vec<u32>` representation matching wchar_t semantics.

#### Risk 3: Finnish Phonological Rules in Hyphenation (MEDIUM)

**Impact:** Incorrect hyphenation breaks soft-hyphen validation in spelling, which breaks spell checking for words with soft hyphens.

**Specific concerns:**
- The vowel/consonant tables and split rules encode Finnish phonology
- The interaction between compound structure (from morphology) and syllable rules is subtle
- `isGoodHyphenPosition` checks backward and forward for vowels bounded by existing hyphens

**Mitigation:** Port the hyphenation tables as static data. Create a comprehensive test suite for Finnish hyphenation patterns.

#### Risk 4: Suggestion Generator Cost Model (MEDIUM)

**Impact:** If the cost/abort model behaves differently, suggestions may be worse quality or significantly slower.

**Specific concerns:**
- The `shouldAbort` logic allows 2x budget when no suggestions found
- Priority = `priority * (suggestionCount + 5)` biases toward early finds
- Generator ordering is tuned for Finnish typing patterns

**Mitigation:** Port the exact abort and priority logic. Validate with known misspelling -> suggestion pairs.

### Lower Risks

#### Risk 5: Grammar Rule Check Correctness (LOW-MEDIUM)

Most grammar checks are simple pattern matching. The main complexity is in `CapitalizationCheck`'s state machine and `FinnishAnalysis::analyseToken`'s flag computation. Both are well-structured and testable.

#### Risk 6: Tokenizer Edge Cases (LOW)

The tokenizer is straightforward but has many special cases (URL detection, number formatting, various Unicode dashes). These are well-defined and testable.

#### Risk 7: SpellerCache Hash Compatibility (LOW)

The cache uses a simple hash function. For a Rust port, this can be reimplemented exactly or replaced with a standard HashMap. The cache is purely an optimization -- incorrect behavior just means slower operation, not wrong results.

### Recommended Porting Order

1. **Character utilities** (SimpleChar, charset) -- foundation for everything
2. **Analysis data model** (Analysis, Key enum) -- needed by all modules
3. **FST engine** (WeightedTransducer, UnweightedTransducer) -- core dependency
4. **Tokenizer** -- simple, self-contained, needed by grammar
5. **VfstAnalyzer** (generic) -- simpler morphology path
6. **FinnishVfstAnalyzer** -- most complex, most critical
7. **AnalyzerToSpellerAdapter + SpellUtils** -- basic spell check
8. **spell.cpp pipeline** (case handling, caching) -- public spell API
9. **FinnishSpellerTweaksWrapper** -- Finnish spelling tweaks
10. **Hyphenator** -- needed by FinnishSpellerTweaksWrapper for soft hyphens
11. **Suggestion generators** -- can be ported individually
12. **Grammar checks** -- each check is independent
13. **VfstSuggestion** -- transducer-based suggestions (if err.vfst available)

### Data Tables to Extract as Constants

The following should become `const` / `static` data in Rust:

1. `classMap`, `sijamuotoMap`, `moodMap`, etc. in FinnishVfstAnalyzer (~100 entries total)
2. `REPLACEMENTS_1` through `REPLACEMENTS_5` in SuggestionStrategyTyping
3. OCR `REPLACEMENTS` table
4. `SPLIT_VOWELS`, `LONG_CONSONANTS`, `SPLIT_AFTER` in hyphenator
5. `VOIKKO_VOWELS`, `VOIKKO_CONSONANTS` character sets
6. `VOIKKO_HASH_ORDERS`, `VOIKKO_CACHE_OFFSETS`, `VOIKKO_META_OFFSETS` for SpellerCache
7. Grammar error codes and bilingual error messages
8. Repeating word exceptions ("ollut", "olleet", "silla")
