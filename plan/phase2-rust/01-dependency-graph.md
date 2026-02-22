# Libvoikko C++ Module Dependency Graph

> Comprehensive dependency analysis for Rust WASM porting.
> Generated from source analysis of `libvoikko/src/`.

---

## 1. Module Directory Structure

```
src/
  voikko.h                    # Public C API
  voikko_defines.h            # Constants, option IDs, spell result codes
  voikko_enums.h              # voikko_token_type, voikko_sentence_type
  voikko_structs.h            # voikko_grammar_error (legacy struct)
  porting.h                   # VOIKKOEXPORT macro, platform portability

  utf8/                       # Third-party UTF-8 library (header-only)
  character/                  # Character classification & case conversion
  utils/                      # String conversion (UTF-8 <-> UCS4), case utils
  fst/                        # Finite State Transducer engine (VFST)
  morphology/                 # Morphological analysis
  spellchecker/               # Spell checking
    suggestion/               # Suggestion generation strategies
  hyphenator/                 # Hyphenation
  tokenizer/                  # Word/token boundary detection
  sentence/                   # Sentence boundary detection
  grammar/                    # Grammar checking
    FinnishRuleEngine/        # Finnish-specific grammar rules
  setup/                      # Dictionary loading, VoikkoHandle, initialization
  compatibility/              # Deprecated API wrappers
  tools/                      # CLI tools (voikkospell, voikkohyphenate, etc.)
```

---

## 2. ASCII Dependency Graph

The graph below shows `#include` dependencies between modules. Arrows point
from dependent to dependency (A --> B means "A depends on B").

```
                          +------------------+
                          |   Public C API   |
                          |    voikko.h      |
                          | voikko_defines.h |
                          |  voikko_enums.h  |
                          | voikko_structs.h |
                          +--------+---------+
                                   |
                          +--------v---------+
                          |   compatibility/ |
                          | (deprecated API  |
                          |    wrappers)     |
                          +--------+---------+
                                   |
               +-------------------v--------------------+
               |             setup/setup.hpp            |
               |           (VoikkoHandle struct)        |
               | Includes: morphology/Analyzer.hpp      |
               |           spellchecker/Speller.hpp     |
               |           spellchecker/SpellerCache.hpp|
               |           grammar/GrammarChecker.hpp   |
               |           suggestion/SuggestionGen.hpp |
               |           hyphenator/Hyphenator.hpp    |
               |           setup/Dictionary.hpp         |
               +--+---+---+---+---+---+---+---+--------+
                  |   |   |   |   |   |   |   |
    +-------------+   |   |   |   |   |   |   +------------------+
    |                 |   |   |   |   |   |                      |
    v                 v   |   v   |   v   v                      v
+--------+     +------+   | +-----+  | +--------+       +--------+
|setup/  |     |morph/|   | |spell|  | |grammar/|       |hyphen/ |
|Diction-|     |Analy-|   | |check|  | |Grammar-|       |Hyphen- |
|aryFact.|     |zerFa.|   | |Fact.|  | |CheckFa.|       |atorFac.|
+---+----+     +--+---+   | +--+--+  | +---+----+       +---+----+
    |             |        |    |     |     |                |
    v             v        v    v     v     v                v
+--------+   +--------+ +--------+ +--------+         +--------+
|setup/  |   |morph/  | |spell/  | |grammar/|         |hyphen/ |
|Diction-|   |Finnish-| |Finnish | |Finnish-|         |Analyzer|
|aryLoad.|   |VfstAna.| |Speller-| |Grammar-|         |ToFinni.|
|V5Dict. |   |VfstAna.| |Tweaks  | |Checker |         |HyphAda.|
+--------+   |NullAna.| |Vfst    | |Finnish-|         +---+----+
              +---+----+ |Speller | |RuleEng.|             |
                  |      |Analyzer| |Finnish-|             |
                  |      |ToSpell.| |Analysis|             |
                  |      +---+----+ +---+----+             |
                  |          |          |                   |
                  v          v          v                   v
             +--------+ +--------+ +--------+       +----------+
             | fst/   | | fst/   | |grammar/|       |morphology|
             |Unweigh-| |Weigh-  | |Sentence|       |/Analyzer |
             |tedTran.| |tedTran.| |Token   |       +-----+----+
             +---+----+ +---+----+ |Paragr. |             |
                 |          |      |GcCache |             |
                 v          v      |CacheEnt|             v
             +--------+ +--------+ |RuleEng.|       +----------+
             | fst/   | | fst/   | |VoikkoGr|       |morphology|
             |Transduc| |Weigh-  | |amError |       |/Analysis |
             |er (base| |tedConf.| +---+----+       +----------+
             +---+----+ +--------+     |
                 |                     v
                 v              +-----------+
            +--------+          | voikko_   |
            | fst/   |          | enums.h   |
            |Transit.|          | structs.h |
            |Config. |          +-----------+
            +--------+


    +----------+    +----------+    +----------+
    | character|    |  utils/  |    |   utf8/  |
    | /charset |    |StringUtil|    |(header-  |
    | /Simple  |    |  /utils  |    | only lib)|
    | Char     |    |          |    |          |
    +----------+    +----+-----+    +----------+
         ^               ^               ^
         |               |               |
         +-------+-------+-------+-------+
                 |               |
    Used by: spell.cpp, tokenizer/, sentence/,
    hyphenator/, grammar/, morphology/interface,
    suggestions/
```

---

## 3. Module Dependency Matrix

Each module's direct dependencies on other internal modules:

| Module | Depends On |
|--------|-----------|
| **utf8/** | (none -- third-party header-only lib) |
| **character/** | `<cwchar>` only -- no internal deps |
| **utils/utils** | `<cstddef>` only -- no internal deps |
| **utils/StringUtils** | `utf8/` (for UTF-8 <-> UCS4 conversion) |
| **fst/Transition** | (none -- pure data structs) |
| **fst/WeightedTransition** | (none -- pure data structs) |
| **fst/Configuration** | (none -- pure data struct) |
| **fst/WeightedConfiguration** | (none -- pure data struct) |
| **fst/Transducer** | (none -- base class, self-contained) |
| **fst/UnweightedTransducer** | `fst/Transducer`, `fst/Transition`, `fst/Configuration` |
| **fst/WeightedTransducer** | `fst/Transducer`, `fst/WeightedTransition`, `fst/WeightedConfiguration` |
| **setup/LanguageTag** | (none -- self-contained) |
| **setup/BackendProperties** | (none -- self-contained) |
| **setup/DictionaryException** | (none -- self-contained) |
| **setup/Dictionary** | `setup/LanguageTag`, `setup/BackendProperties` |
| **setup/DictionaryLoader** | `setup/Dictionary`, `setup/DictionaryException` |
| **setup/V5DictionaryLoader** | `setup/DictionaryLoader` |
| **setup/DictionaryFactory** | `setup/Dictionary`, `setup/DictionaryException` |
| **morphology/Analysis** | (none -- self-contained data class, typedef'd as `voikko_mor_analysis`) |
| **morphology/Analyzer** | `morphology/Analysis` |
| **morphology/NullAnalyzer** | `morphology/Analyzer` |
| **morphology/FinnishVfstAnalyzer** | `morphology/Analyzer`, `fst/UnweightedTransducer`, `fst/Configuration` |
| **morphology/VfstAnalyzer** | `morphology/Analyzer`, `fst/WeightedTransducer`, `fst/Configuration` |
| **morphology/AnalyzerFactory** | `morphology/Analyzer`, `setup/Dictionary`, `setup/DictionaryException` |
| **spellchecker/Speller** | (none -- pure abstract interface) |
| **spellchecker/SpellerCache** | `spellchecker/Speller` (for `spellresult` enum) |
| **spellchecker/FixedResultSpeller** | `spellchecker/Speller` |
| **spellchecker/AnalyzerToSpellerAdapter** | `spellchecker/Speller`, `morphology/Analyzer` |
| **spellchecker/VfstSpeller** | `spellchecker/Speller`, `fst/WeightedTransducer`, `fst/Configuration` |
| **spellchecker/FinnishSpellerTweaksWrapper** | `spellchecker/Speller`, `morphology/Analyzer`, `setup/setup.hpp`, `hyphenator/AnalyzerToFinnishHyphenatorAdapter` |
| **spellchecker/SpellUtils** | `spellchecker/Speller` (for `spellresult`) |
| **spellchecker/SpellWithPriority** | `setup/setup.hpp` (for `VoikkoHandle`), `morphology/Analyzer` |
| **spellchecker/SpellerFactory** | `spellchecker/Speller`, `setup/Dictionary`, `setup/setup.hpp` |
| **spellchecker/spell.cpp** | `utils/utils`, `utils/StringUtils`, `character/charset`, `character/SimpleChar`, `spellchecker/Speller`, `setup/setup.hpp` |
| **suggestion/Suggestion** | (none -- pure data class) |
| **suggestion/SuggestionType** | (none -- enum only) |
| **suggestion/SuggestionStatus** | `suggestion/Suggestion` |
| **suggestion/SuggestionGenerator** | `suggestion/SuggestionStatus` |
| **suggestion/SuggestionStrategy** | `suggestion/SuggestionGenerator`, `suggestion/SuggestionStatus` |
| **suggestion/SuggestionGeneratorFactory** | `suggestion/SuggestionType`, `suggestion/SuggestionGenerator`, `setup/setup.hpp` |
| **suggestion/SuggestionGeneratorCaseChange** | `suggestion/SuggestionGenerator`, `morphology/Analyzer` |
| **suggestion/SuggestionGenerator{Deletion,Insertion,...}** | `suggestion/SuggestionGenerator`, `morphology/Analyzer` |
| **spellchecker/VfstSuggestion** | `fst/WeightedTransducer`, `suggestion/SuggestionStatus`, `suggestion/SuggestionGenerator`, `setup/setup.hpp` |
| **hyphenator/Hyphenator** | (none -- pure abstract interface) |
| **hyphenator/AnalyzerToFinnishHyphenatorAdapter** | `hyphenator/Hyphenator`, `morphology/Analyzer` |
| **hyphenator/HyphenatorFactory** | `hyphenator/Hyphenator`, `setup/Dictionary`, `setup/setup.hpp` |
| **tokenizer/Tokenizer** | `voikko_enums.h`, `setup/setup.hpp` |
| **sentence/Sentence** | `voikko_enums.h`, `setup/setup.hpp` |
| **grammar/Token** | `morphology/Analysis`, `voikko_enums.h` |
| **grammar/Sentence** | `grammar/Token` |
| **grammar/Paragraph** | `grammar/Sentence` |
| **grammar/CacheEntry** | `grammar/VoikkoGrammarError`, `grammar/error.hpp` |
| **grammar/GcCache** | `grammar/CacheEntry` |
| **grammar/VoikkoGrammarError** | `grammar/error.hpp`, `grammar/Sentence`, `voikko_structs.h` |
| **grammar/error.hpp** | `voikko_structs.h`, `grammar/VoikkoGrammarError` (circular -- fwd-decl) |
| **grammar/Analysis** (gc) | `morphology/Analyzer`, `grammar/Paragraph` |
| **grammar/RuleEngine** | `grammar/GcCache`, `grammar/Paragraph` |
| **grammar/GrammarChecker** | `grammar/RuleEngine`, `grammar/GcCache`, `grammar/Analysis`, `morphology/Analyzer` |
| **grammar/FinnishAnalysis** | `grammar/Analysis`, `grammar/Paragraph`, `setup/setup.hpp` |
| **grammar/FinnishGrammarChecker** | `setup/setup.hpp`, `grammar/GrammarChecker` |
| **grammar/FinnishRuleEngine** | `setup/setup.hpp`, `grammar/RuleEngine`, `grammar/FinnishRuleEngine/*` |
| **grammar/GrammarCheckerFactory** | `grammar/GrammarChecker`, `setup/Dictionary`, `setup/setup.hpp` |
| **grammar/FinnishRuleEngine/SentenceCheck** | `grammar/Sentence`, `setup/setup.hpp` |
| **grammar/FinnishRuleEngine/ParagraphCheck** | `grammar/Paragraph`, `setup/setup.hpp` |
| **grammar/FinnishRuleEngine/CapitalizationCheck** | `FinnishRuleEngine/ParagraphCheck` |
| **grammar/FinnishRuleEngine/checks** | `grammar/Analysis`, `setup/setup.hpp` |
| **setup/setup.hpp** | `morphology/Analyzer`, `spellchecker/Speller`, `spellchecker/SpellerCache`, `grammar/GrammarChecker`, `suggestion/SuggestionGenerator`, `hyphenator/Hyphenator`, `setup/Dictionary` |
| **compatibility/** | `setup/setup.hpp` (wraps all deprecated API calls) |

---

## 4. Shared Types & Interfaces

Types that cross module boundaries -- these must be defined early in the
Rust port as they form the "contract" between modules.

### 4.1 Abstract Interfaces (Trait candidates in Rust)

| C++ Class | Defined In | Implemented By | Used By |
|-----------|-----------|---------------|---------|
| `Analyzer` | `morphology/Analyzer.hpp` | `FinnishVfstAnalyzer`, `VfstAnalyzer`, `NullAnalyzer`, `HfstAnalyzer`, `LttoolboxAnalyzer` | `setup.hpp` (VoikkoHandle), `spellchecker/AnalyzerToSpellerAdapter`, `spellchecker/FinnishSpellerTweaksWrapper`, `spellchecker/SpellWithPriority`, `hyphenator/AnalyzerToFinnishHyphenatorAdapter`, `grammar/GrammarChecker`, `suggestion/*` generators |
| `Speller` | `spellchecker/Speller.hpp` | `AnalyzerToSpellerAdapter`, `FinnishSpellerTweaksWrapper`, `VfstSpeller`, `FixedResultSpeller`, `HfstSpeller` | `setup.hpp`, `spell.cpp`, `SpellerCache` |
| `Hyphenator` | `hyphenator/Hyphenator.hpp` | `AnalyzerToFinnishHyphenatorAdapter`, `HfstHyphenator` | `setup.hpp`, `hyphenator/interface.cpp` |
| `GrammarChecker` | `grammar/GrammarChecker.hpp` | `FinnishGrammarChecker`, `NullGrammarChecker`, `CgGrammarChecker` | `setup.hpp`, `grammar/interface.cpp` |
| `SuggestionGenerator` | `suggestion/SuggestionGenerator.hpp` | `SuggestionStrategy*`, `VfstSuggestion`, `SuggestionGeneratorNull`, 12+ strategy classes | `setup.hpp`, `spellchecker/suggestions.cpp` |
| `RuleEngine` | `grammar/RuleEngine.hpp` | `FinnishRuleEngine`, `CgRuleEngine` | `GrammarChecker` |
| `grammar::Analysis` | `grammar/Analysis.hpp` | `FinnishAnalysis`, `HfstAnalysis` | `GrammarChecker` |
| `SentenceCheck` | `FinnishRuleEngine/SentenceCheck.hpp` | `CompoundVerbCheck`, `MissingVerbCheck`, `NegativeVerbCheck`, `RelativePronounCheck`, `SidesanaCheck`, `VfstAutocorrectCheck` | `FinnishRuleEngine` |
| `ParagraphCheck` | `FinnishRuleEngine/ParagraphCheck.hpp` | `CapitalizationCheck` | `FinnishRuleEngine` |
| `Transducer` | `fst/Transducer.hpp` | `UnweightedTransducer`, `WeightedTransducer` | `FinnishVfstAnalyzer`, `VfstAnalyzer`, `VfstSpeller`, `VfstSuggestion` |
| `DictionaryLoader` | `setup/DictionaryLoader.hpp` | `V3DictionaryLoader`, `V4DictionaryLoader`, `V5DictionaryLoader` | `DictionaryFactory` |

### 4.2 Enums Used Across Modules

| Enum | Defined In | Used By |
|------|-----------|---------|
| `voikko_token_type` | `voikko_enums.h` | `tokenizer/`, `grammar/Token`, `grammar/FinnishAnalysis` |
| `voikko_sentence_type` | `voikko_enums.h` | `sentence/`, `grammar/Sentence` |
| `spellresult` | `spellchecker/Speller.hpp` | `spell.cpp`, `SpellerCache`, `SpellUtils`, `SpellWithPriority`, all Speller impls |
| `casetype` | `utils/utils.hpp` | `spell.cpp`, grammar checks |
| `char_type` | `character/charset.hpp` | `tokenizer/`, grammar checks |
| `SuggestionType` | `suggestion/SuggestionType.hpp` | `SuggestionGeneratorFactory`, `setup.cpp` |
| `FollowingVerbType` | `grammar/Token.hpp` | Grammar rule engine checks |
| `Operation` (flag diacritics) | `fst/Transducer.hpp` | FST internals only |
| `Analysis::Key` | `morphology/Analysis.hpp` | All morphology consumers |

### 4.3 Key Data Structures

| Struct/Class | Defined In | Description | Crossing Points |
|-------------|-----------|-------------|-----------------|
| `VoikkoHandle` | `setup/setup.hpp` | Central handle holding all subsystem pointers | Every public API function, every Factory |
| `Dictionary` | `setup/Dictionary.hpp` | Describes loaded dictionary, 6 BackendProperties | All Factories, DictionaryFactory |
| `BackendProperties` | `setup/BackendProperties.hpp` | Backend name + path | Dictionary |
| `LanguageTag` | `setup/LanguageTag.hpp` | BCP 47 tag (language, script, privateUse) | Dictionary, DictionaryFactory |
| `DictionaryException` | `setup/DictionaryException.hpp` | Error during dict loading | All Factories |
| `Analysis` (`voikko_mor_analysis`) | `morphology/Analysis.hpp` | Map of Key -> wchar_t* values | morphology, spellchecker (via Adapter), grammar/Token, hyphenator (via Adapter) |
| `grammar::Token` | `grammar/Token.hpp` | Annotated token with analyses | grammar/Sentence, all rule checks |
| `grammar::Sentence` | `grammar/Sentence.hpp` | Array of Tokens | grammar/Paragraph, rule checks |
| `grammar::Paragraph` | `grammar/Paragraph.hpp` | Array of Sentences | GrammarChecker, RuleEngine |
| `VoikkoGrammarError` | `grammar/VoikkoGrammarError.hpp` | Error description + suggestions | grammar/interface.cpp, GcCache |
| `voikko_grammar_error` | `voikko_structs.h` | Legacy C struct (inside VoikkoGrammarError) | Deprecated API |
| `Suggestion` | `suggestion/Suggestion.hpp` | word + priority | SuggestionStatus |
| `SuggestionStatus` | `suggestion/SuggestionStatus.hpp` | Accumulator for suggestions | All SuggestionGenerators |
| `Configuration` | `fst/Configuration.hpp` | Transducer traversal state (stacks) | UnweightedTransducer |
| `WeightedConfiguration` | `fst/WeightedConfiguration.hpp` | Weighted traversal state | WeightedTransducer |
| `Transition` | `fst/Transition.hpp` | 8-byte unweighted transition | UnweightedTransducer |
| `WeightedTransition` | `fst/WeightedTransition.hpp` | 16-byte weighted transition | WeightedTransducer |

---

## 5. Call Flow Diagrams

### 5.1 voikkoInit -- Initialization

```
voikkoInit(error, langcode, path)
  |
  +-> new VoikkoHandle()
  |     (set all boolean option defaults)
  |
  +-> DictionaryFactory::load(langcode, path)
  |     +-> getDefaultLocations()            [setup/DictionaryFactory]
  |     +-> addAllVersionVariantsFromPath()
  |     |     +-> V5DictionaryLoader::findDictionaries(path)
  |     |     +-> V3DictionaryLoader::findDictionaries(path)  [if HFST enabled]
  |     |     +-> V4DictionaryLoader::findDictionaries(path)  [if lttoolbox enabled]
  |     +-> LanguageTag::setBcp47(langcode)
  |     +-> Match dictionary by language tag
  |     +-> return Dictionary (with 6 BackendProperties)
  |
  +-> AnalyzerFactory::getAnalyzer(dict)
  |     switch(dict.getMorBackend().getBackend())
  |       "FinnishVfst" -> new FinnishVfstAnalyzer(path)
  |                          +-> new UnweightedTransducer("mor.vfst")
  |                          +-> new Configuration(...)
  |       "Vfst"        -> new VfstAnalyzer(path)
  |                          +-> new WeightedTransducer("mor.vfst")
  |       "null"        -> new NullAnalyzer()
  |       "hfst"        -> new HfstAnalyzer(...)         [if HFST enabled]
  |
  +-> SpellerFactory::getSpeller(handle, dict)
  |     switch(dict.getSpellBackend().getBackend())
  |       "FinnishSpellerTweaksWrapper(AnalyzerToSpellerAdapter)"
  |         -> new AnalyzerToSpellerAdapter(handle->morAnalyzer)
  |         -> new FinnishSpellerTweaksWrapper(adapter, morAnalyzer, handle)
  |       "VfstSpeller"
  |         -> new VfstSpeller(path)
  |              +-> new WeightedTransducer("spell.vfst")
  |       "FixedResultSpeller(SPELL_FAILED)"
  |         -> new FixedResultSpeller(SPELL_FAILED)
  |       "hfst"
  |         -> new HfstSpeller(...)                      [if HFST enabled]
  |
  +-> SuggestionGeneratorFactory::getSuggestionGenerator(handle, STD)
  |     switch(dict.getSuggestionBackend().getBackend())
  |       "FinnishSuggestionStrategy" -> SuggestionStrategyTyping (with sub-generators)
  |       "VfstSuggestion"           -> VfstSuggestion(speller->transducer, path)
  |       "null"                     -> SuggestionGeneratorNull
  |
  +-> HyphenatorFactory::getHyphenator(handle, dict)
  |     switch(dict.getHyphenatorBackend().getBackend())
  |       "AnalyzerToFinnishHyphenatorAdapter"
  |         -> new AnalyzerToFinnishHyphenatorAdapter(handle->morAnalyzer)
  |       "null" or default
  |         -> (returns a null-like hyphenator)
  |
  +-> GrammarCheckerFactory::getGrammarChecker(handle, dict)
  |     switch(dict.getGrammarBackend().getBackend())
  |       "FinnishGrammarChecker"
  |         -> new FinnishGrammarChecker(handle)
  |              analyser = handle->morAnalyzer (or separate gramMor analyzer)
  |              paragraphAnalyser = new FinnishAnalysis(handle)
  |              ruleEngine = new FinnishRuleEngine(handle)
  |       "null"
  |         -> new NullGrammarChecker()
  |
  +-> new SpellerCache(0)
  +-> return VoikkoHandle*
```

### 5.2 voikkoSpellCstr -- Spell Check

```
voikkoSpellCstr(handle, word)
  |
  +-> StringUtils::ucs4FromUtf8(word, len)     [utils/StringUtils -- uses utf8/]
  +-> voikkoSpellUcs4(handle, word_ucs4)
        |
        +-> voikko_normalise(word, nchars)        [character/charset]
        +-> SimpleChar::isDigit(c)                [character/SimpleChar]
        +-> voikko_casetype(nword, nchars)        [utils/utils]
        +-> voikko_is_nonword(nword, nchars)      [utils/utils]
        +-> SimpleChar::lower(c)                  [character/SimpleChar]
        |
        +-> voikko_cached_spell(handle, buffer, len)
        |     +-> SpellerCache::isInCache()        [spellchecker/SpellerCache]
        |     +-> SpellerCache::getSpellResult()
        |     +-> hyphenAwareSpell(handle, word, len)
        |           +-> handle->speller->spell(word, len)
        |           |     -- dispatches to concrete Speller impl --
        |           |     FinnishSpellerTweaksWrapper::spell()
        |           |       +-> AnalyzerToSpellerAdapter::spell()
        |           |             +-> Analyzer::analyze(word, wlen, false)
        |           |             +-> SpellUtils::matchWordAndAnalysis()
        |           |     -- OR --
        |           |     VfstSpeller::spell()
        |           |       +-> WeightedTransducer::prepare()
        |           |       +-> WeightedTransducer::next()
        |           +-> SpellerCache::setSpellResult()
        |
        +-> return VOIKKO_SPELL_OK / VOIKKO_SPELL_FAILED
```

### 5.3 voikkoAnalyzeWordCstr -- Morphological Analysis

```
voikkoAnalyzeWordCstr(handle, word)
  |
  +-> StringUtils::ucs4FromUtf8(word, len)
  +-> voikkoAnalyzeWordUcs4(handle, word_ucs4)
        |
        +-> handle->morAnalyzer->analyze(word, wlen, true)
        |     -- dispatches to concrete Analyzer --
        |     FinnishVfstAnalyzer::analyze()
        |       +-> UnweightedTransducer::prepare(config, word, wlen)
        |       +-> loop: UnweightedTransducer::next(config, outputBuffer)
        |       +-> parseBasicAttributes(analysis, fstOutput)
        |       +-> parseDebugAttributes(analysis, fstOutput)
        |       +-> return list<Analysis*>
        |
        +-> for each analysis: analysis->seal()
        +-> return voikko_mor_analysis** array
```

### 5.4 voikkoNextGrammarErrorCstr -- Grammar Check

```
voikkoNextGrammarErrorCstr(handle, text, textlen, startpos, skiperrors)
  |
  +-> StringUtils::ucs4FromUtf8(text, textlen)
  +-> voikkoNextGrammarErrorUcs4(handle, text_ucs4, ...)
        |
        +-> grammarChecker->errorFromCache(text, startpos, skiperrors)
        |     +-> GcCache: check paragraph hash, walk error linked list
        |
        +-> (if not cached) grammarChecker->paragraphToCache(text, textlen)
        |     +-> paragraphAnalyser->analyseParagraph(text, textlen)
        |     |     FinnishAnalysis::analyseParagraph()
        |     |       +-> loop: tokenizer::Tokenizer::nextToken()  [tokenizer/]
        |     |       +-> sentence::Sentence::next()                [sentence/]
        |     |       +-> for each word token:
        |     |       |     morAnalyzer->analyze(word, wlen, true)  [morphology/]
        |     |       |     classify token properties from Analysis
        |     |       +-> return Paragraph (Sentences -> Tokens)
        |     |
        |     +-> ruleEngine->check(paragraph)
        |           FinnishRuleEngine::check()
        |             +-> for each sentence:
        |             |     gc_local_punctuation()        [checks.cpp]
        |             |     gc_punctuation_of_quotations()
        |             |     gc_repeating_words()
        |             |     CompoundVerbCheck::check()
        |             |     MissingVerbCheck::check()
        |             |     NegativeVerbCheck::check()
        |             |     RelativePronounCheck::check()
        |             |     SidesanaCheck::check()
        |             |     VfstAutocorrectCheck::check()
        |             +-> capitalizationCheck.check(paragraph)
        |             +-> gc_end_punctuation(paragraph)
        |             +-> errors added to GcCache
        |
        +-> grammarChecker->errorFromCache(text, startpos, skiperrors)
        +-> return deep copy of VoikkoGrammarError
```

### 5.5 voikkoHyphenateCstr -- Hyphenation

```
voikkoHyphenateCstr(handle, word)
  |
  +-> StringUtils::ucs4FromUtf8(word, len)
  +-> voikkoHyphenateUcs4(handle, word_ucs4)
        |
        +-> handle->hyphenator->hyphenate(word, wlen)
              -- dispatches to concrete Hyphenator --
              AnalyzerToFinnishHyphenatorAdapter::hyphenate()
                +-> analyzer->analyze(word, wlen, true)     [morphology/]
                +-> splitCompounds() using STRUCTURE attribute
                +-> for each compound part:
                |     ruleHyphenation(word, buffer, nchars)
                |       +-> Finnish vowel/consonant rules
                +-> intersectHyphenations()
                +-> return char* pattern (" ", "-", "=")
```

### 5.6 voikkoNextTokenCstr -- Tokenization

```
voikkoNextTokenCstr(handle, text, textlen, tokenlen)
  |
  +-> StringUtils::ucs4FromUtf8(text, textlen, maxChars)
  +-> voikkoNextTokenUcs4(handle, text_ucs4, ...)
        |
        +-> Tokenizer::nextToken(options, text, textlen, tokenlen)
              +-> get_char_type(c)                   [character/charset]
              +-> SimpleChar::isUpper/isLower/isDigit [character/SimpleChar]
              +-> voikko_is_nonword()                [utils/utils]
              +-> return voikko_token_type
```

---

## 6. Cross-Cutting Concerns

### 6.1 wchar_t Usage

`wchar_t` is the **universal internal string type** across all modules. Every
interface boundary uses `wchar_t*`:

- `Analyzer::analyze(const wchar_t*, size_t, bool)`
- `Speller::spell(const wchar_t*, size_t)`
- `Hyphenator::hyphenate(const wchar_t*, size_t)`
- `Tokenizer::nextToken(..., const wchar_t*, ...)`
- `SuggestionStatus` stores `const wchar_t* word`
- `Analysis` stores `map<Key, wchar_t*>`
- `grammar::Token` stores `wchar_t* str`
- FST transducers operate on `wchar_t` input/output

**Porting implication**: In Rust, all internal strings should be `Vec<char>` or
`Vec<u32>` (Unicode codepoints). The public WASM API will accept/return UTF-8
(`&str`/`String`). The `StringUtils::ucs4FromUtf8` / `utf8FromUcs4` conversions
happen only at the C API boundary (`interface.cpp` files) -- in Rust this
becomes the natural boundary between `&str` and `Vec<char>`.

### 6.2 Memory Ownership Patterns

| Pattern | Where | Rust Equivalent |
|---------|-------|----------------|
| `new[]` / `delete[]` for wchar_t buffers | spell.cpp, interface files, FST output buffers | `Vec<char>` owned by caller |
| `new Analysis` -> caller must `delete` | `Analyzer::analyze()` returns `list<Analysis*>` | Return `Vec<Analysis>` by value |
| `deleteAnalyses()` static helper | `Analyzer::deleteAnalyses()` | Not needed -- Rust `Vec` drops automatically |
| `malloc`/`free` at C boundary | `StringUtils::convertCStringToMalloc()` | `CString` for FFI export |
| Deep copy for cache retrieval | `grammar/interface.cpp` copies cached errors | `Clone` trait on `GrammarError` |
| Pointer ownership via VoikkoHandle | setup.cpp creates, voikkoTerminate destroys | `struct VoikkoHandle { analyzer: Box<dyn Analyzer>, ... }` |
| FST file `mmap` / `munmap` | `Transducer::vfstMmap()` / `vfstMunmap()` | `memmap2` crate or `Vec<u8>` from `fetch()` in WASM |

### 6.3 VoikkoHandle / Dictionary Abstraction

`VoikkoHandle` is a **god struct** that holds pointers to all subsystems:

```cpp
struct VoikkoHandle {
    int ignore_dot;                           // boolean options (12 fields)
    grammar::GrammarChecker * grammarChecker; // owned
    morphology::Analyzer * morAnalyzer;       // owned
    spellchecker::Speller * speller;          // owned
    spellchecker::SpellerCache * spellerCache;// owned
    spellchecker::suggestion::SuggestionGenerator * suggestionGenerator; // owned
    hyphenator::Hyphenator * hyphenator;      // owned
    setup::Dictionary dictionary;             // value
    hfst_ospell::ZHfstOspeller* hfst;        // optional, owned
};
```

In Rust:
```rust
pub struct VoikkoHandle {
    pub options: VoikkoOptions,       // all boolean/int options
    pub analyzer: Box<dyn Analyzer>,
    pub speller: Box<dyn Speller>,
    pub speller_cache: Option<SpellerCache>,
    pub suggestion_generator: Box<dyn SuggestionGenerator>,
    pub hyphenator: Box<dyn Hyphenator>,
    pub grammar_checker: Box<dyn GrammarChecker>,
    pub dictionary: Dictionary,
}
```

**Note**: Several components take `VoikkoHandle*` as parameter to read option
flags. The grammar checker reads `accept_titles_in_gc`, etc. The Finnish rule
engine reads options via handle. The `FinnishSpellerTweaksWrapper` reads
`accept_missing_hyphens`, etc. This means options must be accessible from
all subsystems -- consider using `Arc<VoikkoOptions>` or passing option
references directly.

---

## 7. Porting Order Recommendation

### 7.1 Dependency Tiers

```
Tier 0 (zero internal deps -- port first, in parallel):
  +------------------+  +------------------+  +------------------+
  | utf8/            |  | character/       |  | utils/utils.hpp  |
  | (SKIP: use Rust  |  |   charset.hpp    |  | casetype enum    |
  |  native UTF-8)   |  |   SimpleChar.hpp |  | voikko_casetype  |
  +------------------+  +------------------+  | voikko_set_case  |
                                              | voikko_is_nonword|
                                              +------------------+
  +------------------+  +------------------+  +------------------+
  | voikko_enums.h   |  | voikko_defines.h |  | setup/           |
  | token_type       |  | option constants |  | LanguageTag      |
  | sentence_type    |  | spell result     |  | BackendProperties|
  +------------------+  | codes            |  | DictionaryExcep. |
                        +------------------+  +------------------+
  +------------------+
  | morphology/      |
  | Analysis.hpp     |  (self-contained data class)
  +------------------+

Tier 1 (depends only on Tier 0):
  +------------------+  +------------------+  +------------------+
  | utils/           |  | setup/           |  | fst/             |
  | StringUtils      |  | Dictionary       |  | Transition       |
  | (UTF-8 <-> char) |  | (BackendProps +  |  | WeightedTransi.  |
  | SKIP in WASM:    |  |  LanguageTag)    |  | Configuration    |
  | Rust uses UTF-8  |  +------------------+  | WeightedConfig.  |
  +------------------+                        | Transducer (base)|
                                              +------------------+

Tier 2 (core engine -- port together):
  +------------------+  +------------------+
  | fst/             |  | fst/             |
  | UnweightedTrans. |  | WeightedTrans.   |
  | (VFST engine)    |  | (VFST engine)    |
  +------------------+  +------------------+

  +------------------------------------------+
  | morphology/Analyzer (trait)              |
  | morphology/FinnishVfstAnalyzer           |
  | morphology/VfstAnalyzer                  |
  | morphology/NullAnalyzer                  |
  +------------------------------------------+

  +------------------------------------------+
  | spellchecker/Speller (trait)             |
  | spellchecker/SpellerCache                |
  | spellchecker/SpellUtils                  |
  | spellchecker/AnalyzerToSpellerAdapter    |
  | spellchecker/VfstSpeller                 |
  | spellchecker/FixedResultSpeller          |
  | spellchecker/FinnishSpellerTweaksWrapper |
  +------------------------------------------+

Tier 3 (features on top of Tier 2):
  +------------------------------------------+
  | hyphenator/Hyphenator (trait)            |
  | hyphenator/AnalyzerToFinnishHyph.Adapter |
  +------------------------------------------+

  +------------------------------------------+
  | tokenizer/Tokenizer                      |
  | sentence/Sentence                        |
  +------------------------------------------+

  +------------------------------------------+
  | suggestion/Suggestion, SuggestionStatus  |
  | suggestion/SuggestionGenerator (trait)    |
  | suggestion/SuggestionStrategy            |
  | suggestion/SuggestionStrategyTyping      |
  | suggestion/SuggestionStrategyOcr         |
  | suggestion/12 SuggestionGenerator impls  |
  | spellchecker/VfstSuggestion              |
  +------------------------------------------+

Tier 4 (most complex, most deps):
  +------------------------------------------+
  | grammar/Token, Sentence, Paragraph       |
  | grammar/GcCache, CacheEntry              |
  | grammar/VoikkoGrammarError               |
  | grammar/Analysis (abstract gc analysis)  |
  | grammar/FinnishAnalysis                  |
  | grammar/RuleEngine (trait)               |
  | grammar/FinnishRuleEngine + 8 checks     |
  | grammar/GrammarChecker                   |
  | grammar/FinnishGrammarChecker            |
  +------------------------------------------+

Tier 5 (orchestration):
  +------------------------------------------+
  | setup/DictionaryLoader, V5DictLoader     |
  | setup/DictionaryFactory                  |
  | setup/setup.hpp (VoikkoHandle)           |
  | All Factory classes                      |
  | Public API (interface.cpp files)         |
  | compatibility/ (SKIP for WASM)           |
  +------------------------------------------+
```

### 7.2 Modules with Zero External Dependencies (Port First)

These modules have no internal dependencies and can be ported immediately.
They can also be ported **in parallel** since they are independent:

1. **character/SimpleChar** -- `upper()`, `lower()`, `isUpper()`, `isLower()`, `isDigit()`, `isWhitespace()`. Pure functions on `wchar_t`. In Rust: functions on `char`.
2. **character/charset** -- `get_char_type()`, `voikko_normalise()`, `isFinnishQuotationMark()`. Pure functions.
3. **utils/utils** -- `casetype` enum, `voikko_casetype()`, `voikko_set_case()`, `voikko_is_nonword()`. Pure functions.
4. **morphology/Analysis** -- Self-contained data class. Key enum + map storage.
5. **voikko_enums.h** -- Two enums: `voikko_token_type`, `voikko_sentence_type`.
6. **voikko_defines.h** -- Option constants. Becomes `const` values.
7. **setup/LanguageTag** -- BCP 47 parser. Self-contained.
8. **setup/BackendProperties** -- Three fields: path, backend, advertised.
9. **setup/DictionaryException** -- Simple error type.
10. **fst/Transition, WeightedTransition** -- Pure data structs.
11. **fst/Configuration, WeightedConfiguration** -- Stack-based config structs.

### 7.3 Minimum Viable Product: Spell Check Only

For a "spell check only" MVP, you need:

```
Required modules (in porting order):
1. character/          (SimpleChar, charset)
2. utils/              (utils, StringUtils -- partial)
3. voikko_enums.h      (token_type for API compat)
4. voikko_defines.h    (spell result codes, option constants)
5. morphology/Analysis (data class)
6. fst/                (Transducer, UnweightedTransducer OR WeightedTransducer,
                        Configuration/WeightedConfiguration, Transition types)
7. morphology/Analyzer + FinnishVfstAnalyzer (or VfstAnalyzer)
8. spellchecker/Speller + SpellerCache + SpellUtils
9. spellchecker/AnalyzerToSpellerAdapter (or VfstSpeller)
10. spellchecker/FinnishSpellerTweaksWrapper
11. spell.cpp logic (the spell dispatch with casing/dot/cache)

NOT needed for spell-only MVP:
- grammar/          (entire module)
- hyphenator/       (entire module)
- tokenizer/        (entire module)
- sentence/         (entire module)
- suggestion/       (entire module -- adds suggest feature)
- setup/            (replace with hardcoded WASM init)
- compatibility/    (deprecated C API)
```

**Estimated scope**: ~25 source files for spell-check MVP.

### 7.4 Parallel vs. Sequential Porting

**Can be ported in parallel** (no dependency between each other):
- `character/` and `utils/` and `fst/` (all Tier 0-2 modules)
- `tokenizer/` and `sentence/` and `hyphenator/` (Tier 3, all depend on Tier 2 but not each other)
- All `SuggestionGenerator*` implementations (all depend on the same interface)

**Must be ported sequentially**:
- `fst/Transducer` (base) before `fst/UnweightedTransducer` / `fst/WeightedTransducer`
- `morphology/Analysis` before `morphology/Analyzer` before `morphology/FinnishVfstAnalyzer`
- `spellchecker/Speller` before `AnalyzerToSpellerAdapter` before `FinnishSpellerTweaksWrapper`
- `grammar/Token` -> `Sentence` -> `Paragraph` -> `GcCache` -> `RuleEngine` -> `GrammarChecker`
- All Factories depend on their respective module being complete

### 7.5 HFST Backend Considerations

The following files are **HFST-only** and should be **excluded** from the
WASM port (they require the external `hfstospell` library):

- `morphology/HfstAnalyzer.hpp/.cpp`
- `spellchecker/HfstSpeller.hpp/.cpp`
- `spellchecker/HfstSuggestion.hpp/.cpp`
- `hyphenator/HfstHyphenator.hpp/.cpp`
- `grammar/HfstAnalysis.hpp/.cpp`
- `grammar/CgGrammarChecker.hpp/.cpp`
- `grammar/CgRuleEngine.hpp/.cpp`
- `setup/V3DictionaryLoader.hpp/.cpp`
- `morphology/LttoolboxAnalyzer.hpp/.cpp`
- `setup/V4DictionaryLoader.hpp/.cpp`

This leaves the VFST backend as the sole target:
- `FinnishVfstAnalyzer` (unweighted transducer, voikko-fi dictionary)
- `VfstSpeller` (weighted transducer)
- `VfstSuggestion` (weighted transducers)

---

## 8. Summary Statistics

| Metric | Count |
|--------|-------|
| Total modules (directories) | 11 (+ 1 sub: FinnishRuleEngine, 1 sub: suggestion) |
| Header files | ~75 |
| Source files (.cpp) | ~70 |
| Abstract interfaces (trait candidates) | 10 |
| Factory classes | 6 (Analyzer, Speller, Suggestion, Hyphenator, GrammarChecker, Dictionary) |
| Shared enums | 9 |
| Key data structures | 16 |
| Files needed for spell-check MVP | ~25 |
| Files excluded (HFST/lttoolbox) | ~12 |
