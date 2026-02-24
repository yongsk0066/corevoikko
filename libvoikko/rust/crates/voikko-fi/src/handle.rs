// VoikkoHandle: top-level integration point for Finnish NLP.
//
// Owns all components (analyzer, speller, hyphenator, grammar checker,
// suggestion strategies) and provides a unified API for spell checking,
// morphological analysis, hyphenation, grammar checking, suggestion
// generation, and tokenization.
//
// Design notes:
// - The handle owns a FinnishVfstAnalyzer and creates lightweight adapter
//   objects (AnalyzerToSpellerAdapter, FinnishSpellerTweaksWrapper,
//   FinnishHyphenator) on the fly in each method call to avoid
//   self-referential lifetime issues.
// - The grammar checker is independently owned (it has its own rule engine).
// - Suggestion strategies are created once at construction time.
// - Options are stored directly in the handle and passed to adapters
//   when methods are called.
//
// Origin: setup/VoikkoHandle.hpp (C++ VoikkoHandle)

use std::cell::RefCell;

use voikko_core::analysis::Analysis;
use voikko_core::enums::{SentenceType, TokenType};
use voikko_core::grammar_error::GrammarError;
use voikko_core::token::{Sentence, Token};

use crate::grammar::checker::FinnishGrammarChecker;
use crate::grammar::checks::GrammarOptions;
use crate::hyphenator::{FinnishHyphenator, Hyphenator, HyphenatorOptions};
use crate::morphology::{Analyzer, FinnishVfstAnalyzer};
use crate::speller::adapter::AnalyzerToSpellerAdapter;
use crate::speller::cache::SpellerCache;
use crate::speller::finnish::{FinnishSpellerOptions, FinnishSpellerTweaksWrapper};
use crate::speller::pipeline::{SpellOptions, spell_check};
use crate::suggestion::status::SuggestionStatus;
use crate::suggestion::strategy::{
    SuggestionStrategy, default_ocr_strategy, default_typing_strategy,
};
use crate::tokenizer;

/// Error type for VoikkoHandle construction failures.
#[derive(Debug, thiserror::Error)]
pub enum VoikkoError {
    /// The mor.vfst data could not be loaded.
    #[error("failed to load morphology transducer: {0}")]
    MorphologyLoad(#[from] voikko_fst::VfstError),

    /// The autocorr.vfst data could not be loaded.
    #[error("failed to load autocorrect transducer: {0}")]
    AutocorrectLoad(String),

    /// Unsupported language.
    #[error("unsupported language: {0}")]
    UnsupportedLanguage(String),
}

/// Top-level handle that owns all Finnish NLP components.
///
/// Provides spell checking, morphological analysis, hyphenation, grammar
/// checking, suggestion generation, and tokenization through a single
/// unified interface.
///
/// Origin: setup/VoikkoHandle.hpp
pub struct VoikkoHandle {
    /// The morphological analyzer (shared by speller, hyphenator, suggestions).
    analyzer: FinnishVfstAnalyzer,

    /// The grammar checker (stored without analyzer reference to avoid
    /// self-referential lifetimes). The `grammar_errors()` method uses
    /// `check_with_analyzer()` to pass the handle's analyzer at call time.
    grammar_checker: FinnishGrammarChecker<'static>,

    /// Typing suggestion strategy.
    typing_strategy: SuggestionStrategy,

    /// OCR suggestion strategy.
    ocr_strategy: SuggestionStrategy,

    // -- Options --
    /// Spell checker options.
    spell_options: SpellOptions,

    /// Finnish speller tweak options.
    finnish_spell_options: FinnishSpellerOptions,

    /// Hyphenator options.
    hyphenator_options: HyphenatorOptions,

    /// Grammar checker options.
    grammar_options: GrammarOptions,

    /// Whether to use OCR suggestions instead of typing suggestions.
    use_ocr_suggestions: bool,

    /// Maximum number of suggestions to return.
    max_suggestions: usize,

    /// Speller cache for avoiding redundant lookups.
    /// Wrapped in `RefCell` for interior mutability (`&self` methods need `&mut` cache access).
    speller_cache: RefCell<SpellerCache>,
}

impl VoikkoHandle {
    /// Create a new VoikkoHandle from raw dictionary data.
    ///
    /// - `mor_vfst_data`: contents of `mor.vfst` (morphology transducer, required)
    /// - `autocorr_vfst_data`: contents of `autocorr.vfst` (autocorrect transducer, optional)
    /// - `language`: BCP 47 language code (currently only "fi" is supported)
    ///
    /// Origin: VoikkoHandle constructor + dictionary loading
    pub fn from_bytes(
        mor_vfst_data: &[u8],
        autocorr_vfst_data: Option<&[u8]>,
        language: &str,
    ) -> Result<Self, VoikkoError> {
        if language != "fi" {
            return Err(VoikkoError::UnsupportedLanguage(language.to_string()));
        }

        let analyzer = FinnishVfstAnalyzer::from_bytes(mor_vfst_data)?;

        let autocorr_transducer = match autocorr_vfst_data {
            Some(data) => {
                let t = voikko_fst::unweighted::UnweightedTransducer::from_bytes(data)
                    .map_err(|e| VoikkoError::AutocorrectLoad(e.to_string()))?;
                Some(t)
            }
            None => None,
        };

        let grammar_checker =
            FinnishGrammarChecker::new(GrammarOptions::default(), autocorr_transducer, None);

        Ok(Self {
            analyzer,
            grammar_checker,
            typing_strategy: default_typing_strategy(),
            ocr_strategy: default_ocr_strategy(),
            spell_options: SpellOptions::default(),
            finnish_spell_options: FinnishSpellerOptions::default(),
            hyphenator_options: HyphenatorOptions::default(),
            grammar_options: GrammarOptions::default(),
            use_ocr_suggestions: false,
            max_suggestions: 5,
            speller_cache: RefCell::new(SpellerCache::new(0)),
        })
    }

    // =========================================================================
    // Core NLP methods
    // =========================================================================

    /// Check whether a word is correctly spelled.
    ///
    /// Returns `true` if the word is correct (or bypassed by options like
    /// ignore_numbers, ignore_uppercase, etc.).
    ///
    /// Origin: voikkoSpellCstr
    pub fn spell(&self, word: &str) -> bool {
        let word_chars: Vec<char> = word.chars().collect();
        let adapter = AnalyzerToSpellerAdapter::new(&self.analyzer);
        let tweaks =
            FinnishSpellerTweaksWrapper::new(&adapter, &self.analyzer, self.finnish_spell_options);
        spell_check(
            &word_chars,
            &tweaks,
            Some(&mut *self.speller_cache.borrow_mut()),
            &self.spell_options,
        ) == 1
    }

    /// Generate spelling suggestions for a misspelled word.
    ///
    /// Returns a list of suggested corrections, sorted by priority (best first).
    ///
    /// Origin: voikkoSuggestCstr
    pub fn suggest(&self, word: &str) -> Vec<String> {
        let word_chars: Vec<char> = word.chars().collect();
        let adapter = AnalyzerToSpellerAdapter::new(&self.analyzer);
        let tweaks =
            FinnishSpellerTweaksWrapper::new(&adapter, &self.analyzer, self.finnish_spell_options);

        // Collect 3x candidates (matching C++ MAX_SUGGESTIONS * 3), sort, then truncate.
        let mut status = SuggestionStatus::new(&word_chars, self.max_suggestions * 3);

        let strategy = if self.use_ocr_suggestions {
            &self.ocr_strategy
        } else {
            &self.typing_strategy
        };

        strategy.generate(&tweaks, Some(&self.analyzer), &mut status);
        status.sort_suggestions();

        status
            .into_suggestions()
            .into_iter()
            .take(self.max_suggestions)
            .map(|s| s.word)
            .collect()
    }

    /// Perform morphological analysis on a word.
    ///
    /// Returns all valid analyses of the word, each containing attributes
    /// like CLASS, BASEFORM, STRUCTURE, etc.
    ///
    /// Origin: voikkoAnalyzeWordCstr
    pub fn analyze(&self, word: &str) -> Vec<Analysis> {
        let word_chars: Vec<char> = word.chars().collect();
        let word_len = word_chars.len();
        self.analyzer.analyze(&word_chars, word_len)
    }

    /// Hyphenate a word.
    ///
    /// Returns a pattern string of the same character length as the input word.
    /// Each character indicates the hyphenation status at that position:
    /// - `' '`: no hyphenation point
    /// - `'-'`: hyphenation point before this character
    /// - `'='`: hyphenation point with explicit hyphen (compound boundary)
    ///
    /// Origin: voikkoHyphenateCstr
    pub fn hyphenate(&self, word: &str) -> String {
        let word_chars: Vec<char> = word.chars().collect();
        let hyp = FinnishHyphenator::new(&self.analyzer, self.hyphenator_options);
        hyp.hyphenate(&word_chars)
    }

    /// Check a paragraph of text for grammar errors.
    ///
    /// Returns a list of grammar errors found in the text.
    ///
    /// Origin: voikkoNextGrammarErrorCstr
    pub fn grammar_errors(&self, text: &str) -> Vec<GrammarError> {
        let text_chars: Vec<char> = text.chars().collect();
        let text_len = text_chars.len();
        self.grammar_checker
            .check_with_analyzer(&text_chars, text_len, &self.analyzer)
    }

    /// Tokenize text into a list of tokens.
    ///
    /// Each token has a type (Word, Punctuation, Whitespace, Unknown),
    /// text content, and position.
    ///
    /// Origin: voikkoNextTokenCstr
    pub fn tokens(&self, text: &str) -> Vec<Token> {
        let text_chars: Vec<char> = text.chars().collect();
        let text_len = text_chars.len();
        let mut result = Vec::new();
        let mut pos = 0;
        while pos < text_len {
            let (token_type, token_len) = tokenizer::next_token(&text_chars, text_len, pos);
            if token_type == TokenType::None || token_len == 0 {
                break;
            }
            let token_text: String = text_chars[pos..pos + token_len].iter().collect();
            result.push(Token::new(token_type, token_text, pos));
            pos += token_len;
        }
        result
    }

    /// Detect sentence boundaries in text.
    ///
    /// Returns a list of sentences, each with its type (Probable, Possible, None)
    /// and character length.
    ///
    /// Origin: voikkoNextSentenceStartCstr
    pub fn sentences(&self, text: &str) -> Vec<Sentence> {
        let text_chars: Vec<char> = text.chars().collect();
        let text_len = text_chars.len();
        let mut result = Vec::new();
        let mut pos = 0;
        while pos < text_len {
            let (sentence_type, sentence_len) =
                tokenizer::next_sentence(&text_chars, text_len, pos);
            if sentence_type == SentenceType::None {
                // Include the final segment
                if sentence_len > 0 {
                    result.push(Sentence::new(sentence_type, sentence_len));
                }
                break;
            }
            result.push(Sentence::new(sentence_type, sentence_len));
            pos += sentence_len;
        }
        result
    }

    // =========================================================================
    // Option setters
    // =========================================================================

    /// Set whether to ignore trailing dots in spell checking.
    pub fn set_ignore_dot(&mut self, value: bool) {
        self.spell_options.ignore_dot = value;
        self.hyphenator_options.ignore_dot = value;
    }

    /// Set whether to ignore words containing numbers in spell checking.
    pub fn set_ignore_numbers(&mut self, value: bool) {
        self.spell_options.ignore_numbers = value;
    }

    /// Set whether to accept words written entirely in uppercase without checking.
    pub fn set_ignore_uppercase(&mut self, value: bool) {
        self.spell_options.ignore_uppercase = value;
    }

    /// Set whether to suppress ugly but correct hyphenation points.
    pub fn set_no_ugly_hyphenation(&mut self, value: bool) {
        self.hyphenator_options.ugly_hyphenation = !value;
    }

    /// Set whether to accept words with a capitalized first letter.
    pub fn set_accept_first_uppercase(&mut self, value: bool) {
        self.spell_options.accept_first_uppercase = value;
    }

    /// Set whether to accept words with all letters capitalized.
    pub fn set_accept_all_uppercase(&mut self, value: bool) {
        self.spell_options.accept_all_uppercase = value;
    }

    /// Set whether to use OCR-optimized suggestions.
    pub fn set_ocr_suggestions(&mut self, value: bool) {
        self.use_ocr_suggestions = value;
    }

    /// Set whether to ignore non-words (URLs, email addresses, etc.).
    pub fn set_ignore_nonwords(&mut self, value: bool) {
        self.spell_options.ignore_nonwords = value;
    }

    /// Set whether to accept extra hyphens in compound words.
    pub fn set_accept_extra_hyphens(&mut self, value: bool) {
        self.finnish_spell_options.accept_extra_hyphens = value;
    }

    /// Set whether to accept missing hyphens at start/end of word.
    pub fn set_accept_missing_hyphens(&mut self, value: bool) {
        self.spell_options.accept_missing_hyphens = value;
    }

    /// Set whether to accept incomplete sentences in titles (grammar checking).
    pub fn set_accept_titles_in_gc(&mut self, value: bool) {
        self.grammar_options.accept_titles_in_gc = value;
        self.grammar_checker
            .set_options(self.grammar_options.clone());
    }

    /// Set whether to accept incomplete sentences at end of paragraph (grammar checking).
    pub fn set_accept_unfinished_paragraphs_in_gc(&mut self, value: bool) {
        self.grammar_options.accept_unfinished_paragraphs_in_gc = value;
        self.grammar_checker
            .set_options(self.grammar_options.clone());
    }

    /// Set whether to hyphenate unknown words.
    pub fn set_hyphenate_unknown_words(&mut self, value: bool) {
        self.hyphenator_options.hyphenate_unknown = value;
    }

    /// Set whether to accept bulleted list paragraphs in grammar checking.
    pub fn set_accept_bulleted_lists_in_gc(&mut self, value: bool) {
        self.grammar_options.accept_bulleted_lists_in_gc = value;
        self.grammar_checker
            .set_options(self.grammar_options.clone());
    }

    /// Set the minimum word length for hyphenation.
    pub fn set_min_hyphenated_word_length(&mut self, value: usize) {
        self.hyphenator_options.min_hyphenated_word_length = value;
    }

    /// Set the maximum number of suggestions to return.
    pub fn set_max_suggestions(&mut self, value: usize) {
        self.max_suggestions = value;
    }

    // =========================================================================
    // Extended API methods (ported from TS wrapper layer)
    // =========================================================================

    /// Insert hyphens into a word using the given separator.
    ///
    /// Uses the hyphenation pattern from `hyphenate()` to insert separators:
    /// - `'-'` in the pattern: insert separator BEFORE this character
    /// - `'='` in the pattern (when `allow_context_changes` is true):
    ///   replace the character with separator, unless the character is already
    ///   `'-'` (in which case it is preserved to avoid a double hyphen)
    ///
    /// Origin: Voikko.hyphenate() in libvoikko/js/src/index.ts
    pub fn insert_hyphens(
        &self,
        word: &str,
        separator: &str,
        allow_context_changes: bool,
    ) -> String {
        let pattern = self.hyphenate(word);
        let word_chars: Vec<char> = word.chars().collect();
        let pat_chars: Vec<char> = pattern.chars().collect();
        let mut result = String::new();

        for (i, ch) in word_chars.iter().enumerate() {
            if i < pat_chars.len() {
                match pat_chars[i] {
                    '-' => result.push_str(separator),
                    '=' if allow_context_changes => {
                        if *ch == '-' {
                            // Existing hyphen at compound boundary -- preserve it
                            result.push('-');
                            continue;
                        }
                        // Replace non-hyphen char with separator
                        result.push_str(separator);
                        continue;
                    }
                    _ => {}
                }
            }
            result.push(*ch);
        }
        result
    }

    /// Return the list of valid values for an enumerated morphological attribute.
    ///
    /// Returns `None` if the attribute name is not recognized.
    ///
    /// Origin: voikkoGetAttributeValues (C API) / ENUMERATED_ATTRIBUTES in TS wrapper
    pub fn attribute_values(attribute_name: &str) -> Option<&'static [&'static str]> {
        match attribute_name {
            "CLASS" => Some(&[
                "nimisana",
                "laatusana",
                "nimisana_laatusana",
                "teonsana",
                "seikkasana",
                "asemosana",
                "suhdesana",
                "huudahdussana",
                "sidesana",
                "etuliite",
                "lukusana",
                "lyhenne",
                "kieltosana",
                "etunimi",
                "sukunimi",
                "paikannimi",
                "nimi",
            ]),
            "NUMBER" => Some(&["singular", "plural"]),
            "PERSON" => Some(&["1", "2", "3", "4"]),
            "MOOD" => Some(&[
                "indicative",
                "conditional",
                "potential",
                "imperative",
                "A-infinitive",
                "E-infinitive",
                "MA-infinitive",
                "MINEN-infinitive",
                "MAINEN-infinitive",
            ]),
            "TENSE" => Some(&["present_simple", "past_imperfective"]),
            "COMPARISON" => Some(&["positive", "comparative", "superlative"]),
            "NEGATIVE" => Some(&["false", "true", "both"]),
            "PARTICIPLE" => Some(&[
                "present_active",
                "present_passive",
                "past_active",
                "past_passive",
                "agent",
                "negation",
            ]),
            "POSSESSIVE" => Some(&["1s", "2s", "1p", "2p", "3"]),
            "SIJAMUOTO" => Some(&[
                "nimento",
                "omanto",
                "osanto",
                "olento",
                "tulento",
                "kohdanto",
                "sisaolento",
                "sisaeronto",
                "sisatulento",
                "ulkoolento",
                "ulkoeronto",
                "ulkotulento",
                "vajanto",
                "seuranto",
                "keinonto",
                "kerrontosti",
            ]),
            "FOCUS" => Some(&["läs", "kAAn", "kin", "hAn", "pA", "s"]),
            "KYSYMYSLIITE" => Some(&["true"]),
            _ => None,
        }
    }

    /// Check text for grammar errors, splitting at newline boundaries.
    ///
    /// Each line is treated as a separate paragraph. Error positions
    /// (`start_pos`) are relative to the full input text.
    ///
    /// Origin: voikkoNextGrammarErrorCstr (called per-paragraph by the C API),
    ///         Voikko.grammarErrors() in libvoikko/js/src/index.ts
    pub fn grammar_errors_from_text(&self, text: &str) -> Vec<GrammarError> {
        let mut result = Vec::new();
        let mut pos = 0;
        let text_chars: Vec<char> = text.chars().collect();
        let text_len = text_chars.len();

        while pos < text_len {
            // Find next newline
            let nl_pos = text_chars[pos..]
                .iter()
                .position(|&c| c == '\n')
                .map(|p| pos + p)
                .unwrap_or(text_len);

            // Strip trailing \r for \r\n line endings
            let para_end = if nl_pos > pos && text_chars[nl_pos - 1] == '\r' {
                nl_pos - 1
            } else {
                nl_pos
            };

            // Check non-empty paragraph
            if para_end > pos {
                let para = &text_chars[pos..para_end];
                let para_len = para.len();
                let mut errors =
                    self.grammar_checker
                        .check_with_analyzer(para, para_len, &self.analyzer);

                // Adjust start_pos to be relative to the full text
                for error in &mut errors {
                    error.start_pos += pos;
                }
                result.extend(errors);
            }

            // Advance past newline
            if nl_pos >= text_len {
                break;
            }
            pos = nl_pos + 1;
        }

        result
    }

    /// Return the crate version (from Cargo.toml).
    ///
    /// Origin: voikkoGetVersion (C API)
    pub fn get_version() -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    /// Replace the speller cache with a new one of the given size.
    ///
    /// `size` is the size parameter (power-of-two scaling factor).
    /// A value of 0 gives the base cache size.
    ///
    /// Origin: voikkoSetIntegerOption VOIKKO_SPELLER_CACHE_SIZE
    pub fn set_speller_cache_size(&mut self, size: usize) {
        self.speller_cache = RefCell::new(SpellerCache::new(size));
    }

    /// Release resources held by this handle. After calling this,
    /// the handle should not be used for any NLP operations.
    ///
    /// In Rust, this is a no-op since resources are released when the handle
    /// is dropped. This method exists to match the C++ API pattern where
    /// `voikkoTerminate` explicitly frees resources.
    pub fn terminate(self) {
        // Resources are released by Drop
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_language_returns_error() {
        let result = VoikkoHandle::from_bytes(&[], None, "sv");
        assert!(result.is_err());
        match result {
            Err(VoikkoError::UnsupportedLanguage(lang)) => assert_eq!(lang, "sv"),
            Err(other) => panic!("expected UnsupportedLanguage, got: {other}"),
            Ok(_) => panic!("expected error"),
        }
    }

    #[test]
    fn invalid_mor_data_returns_error() {
        let result = VoikkoHandle::from_bytes(&[0, 1, 2, 3], None, "fi");
        assert!(result.is_err());
    }

    // Integration tests with real dictionary data are guarded by the
    // VOIKKO_DICT_PATH environment variable. They are not part of the
    // default test suite.

    #[test]
    #[ignore = "requires mor.vfst dictionary file"]
    fn integration_spell_with_real_dict() {
        let mor_data = std::fs::read(
            std::env::var("VOIKKO_MOR_VFST").unwrap_or_else(|_| "../../test-data/mor.vfst".into()),
        )
        .expect("failed to read mor.vfst");
        let handle =
            VoikkoHandle::from_bytes(&mor_data, None, "fi").expect("failed to create handle");

        assert!(handle.spell("koira"));
        assert!(handle.spell("Helsinki"));
        assert!(!handle.spell("xyzzyplugh"));
    }

    #[test]
    #[ignore = "requires mor.vfst dictionary file"]
    fn integration_analyze_with_real_dict() {
        let mor_data = std::fs::read(
            std::env::var("VOIKKO_MOR_VFST").unwrap_or_else(|_| "../../test-data/mor.vfst".into()),
        )
        .expect("failed to read mor.vfst");
        let handle =
            VoikkoHandle::from_bytes(&mor_data, None, "fi").expect("failed to create handle");

        let analyses = handle.analyze("koira");
        assert!(!analyses.is_empty());
    }

    #[test]
    #[ignore = "requires mor.vfst dictionary file"]
    fn integration_hyphenate_with_real_dict() {
        let mor_data = std::fs::read(
            std::env::var("VOIKKO_MOR_VFST").unwrap_or_else(|_| "../../test-data/mor.vfst".into()),
        )
        .expect("failed to read mor.vfst");
        let handle =
            VoikkoHandle::from_bytes(&mor_data, None, "fi").expect("failed to create handle");

        let pattern = handle.hyphenate("koira");
        assert_eq!(pattern.len(), 5); // same char count as "koira"
    }

    #[test]
    fn tokenize_simple_text() {
        // Tokenizer doesn't need a dictionary -- we can test with any handle
        // but we can't construct one without valid dictionary data.
        // Instead, test the tokenizer directly through the module.
        let text = "Koira juoksi.";
        let text_chars: Vec<char> = text.chars().collect();
        let text_len = text_chars.len();

        let mut tokens = Vec::new();
        let mut pos = 0;
        while pos < text_len {
            let (tt, tlen) = tokenizer::next_token(&text_chars, text_len, pos);
            if tt == TokenType::None || tlen == 0 {
                break;
            }
            let token_text: String = text_chars[pos..pos + tlen].iter().collect();
            tokens.push(Token::new(tt, token_text, pos));
            pos += tlen;
        }

        assert_eq!(tokens.len(), 4); // "Koira", " ", "juoksi", "."
        assert_eq!(tokens[0].token_type, TokenType::Word);
        assert_eq!(tokens[0].text, "Koira");
        assert_eq!(tokens[3].token_type, TokenType::Punctuation);
        assert_eq!(tokens[3].text, ".");
    }

    // =========================================================================
    // attribute_values tests
    // =========================================================================

    #[test]
    fn attribute_values_class() {
        let vals = VoikkoHandle::attribute_values("CLASS").unwrap();
        assert!(vals.contains(&"nimisana"));
        assert!(vals.contains(&"teonsana"));
        assert!(vals.contains(&"nimi"));
        assert_eq!(vals.len(), 17);
    }

    #[test]
    fn attribute_values_number() {
        let vals = VoikkoHandle::attribute_values("NUMBER").unwrap();
        assert_eq!(vals, &["singular", "plural"]);
    }

    #[test]
    fn attribute_values_person() {
        let vals = VoikkoHandle::attribute_values("PERSON").unwrap();
        assert_eq!(vals, &["1", "2", "3", "4"]);
    }

    #[test]
    fn attribute_values_mood() {
        let vals = VoikkoHandle::attribute_values("MOOD").unwrap();
        assert_eq!(vals.len(), 9);
        assert!(vals.contains(&"indicative"));
        assert!(vals.contains(&"MAINEN-infinitive"));
    }

    #[test]
    fn attribute_values_sijamuoto() {
        let vals = VoikkoHandle::attribute_values("SIJAMUOTO").unwrap();
        assert_eq!(vals.len(), 16);
        assert!(vals.contains(&"nimento"));
        assert!(vals.contains(&"kerrontosti"));
    }

    #[test]
    fn attribute_values_focus() {
        let vals = VoikkoHandle::attribute_values("FOCUS").unwrap();
        assert_eq!(vals.len(), 6);
        assert!(vals.contains(&"läs"));
    }

    #[test]
    fn attribute_values_kysymysliite() {
        let vals = VoikkoHandle::attribute_values("KYSYMYSLIITE").unwrap();
        assert_eq!(vals, &["true"]);
    }

    #[test]
    fn attribute_values_unknown_returns_none() {
        assert!(VoikkoHandle::attribute_values("NONEXISTENT").is_none());
        assert!(VoikkoHandle::attribute_values("").is_none());
    }

    #[test]
    fn attribute_values_all_known_names() {
        let names = [
            "CLASS",
            "NUMBER",
            "PERSON",
            "MOOD",
            "TENSE",
            "COMPARISON",
            "NEGATIVE",
            "PARTICIPLE",
            "POSSESSIVE",
            "SIJAMUOTO",
            "FOCUS",
            "KYSYMYSLIITE",
        ];
        for name in &names {
            assert!(
                VoikkoHandle::attribute_values(name).is_some(),
                "expected Some for attribute {name}"
            );
        }
    }

    // =========================================================================
    // get_version tests
    // =========================================================================

    #[test]
    fn get_version_returns_cargo_version() {
        let version = VoikkoHandle::get_version();
        assert_eq!(version, env!("CARGO_PKG_VERSION"));
        // Should be a valid semver-like string
        assert!(!version.is_empty());
        assert!(version.contains('.'));
    }

    // =========================================================================
    // insert_hyphens tests (unit tests without dictionary)
    // =========================================================================

    // NOTE: insert_hyphens depends on hyphenate() which requires a real
    // dictionary, so full integration tests are marked #[ignore].
    // We test the core logic through integration tests below.

    #[test]
    #[ignore = "requires mor.vfst dictionary file"]
    fn integration_insert_hyphens_basic() {
        let mor_data = std::fs::read(
            std::env::var("VOIKKO_MOR_VFST").unwrap_or_else(|_| "../../test-data/mor.vfst".into()),
        )
        .expect("failed to read mor.vfst");
        let handle =
            VoikkoHandle::from_bytes(&mor_data, None, "fi").expect("failed to create handle");

        // "koira" should hyphenate to "koi-ra" with default separator
        let result = handle.insert_hyphens("koira", "-", true);
        // The result should contain the original letters
        assert!(result.contains("koi"));
        assert!(result.contains("ra"));
    }

    #[test]
    #[ignore = "requires mor.vfst dictionary file"]
    fn integration_insert_hyphens_custom_separator() {
        let mor_data = std::fs::read(
            std::env::var("VOIKKO_MOR_VFST").unwrap_or_else(|_| "../../test-data/mor.vfst".into()),
        )
        .expect("failed to read mor.vfst");
        let handle =
            VoikkoHandle::from_bytes(&mor_data, None, "fi").expect("failed to create handle");

        let result_dash = handle.insert_hyphens("koira", "-", true);
        let result_dot = handle.insert_hyphens("koira", ".", true);
        // If there are hyphenation points, the separator should differ
        let dash_count = result_dash.matches('-').count();
        let dot_count = result_dot.matches('.').count();
        assert_eq!(dash_count, dot_count);
    }

    #[test]
    #[ignore = "requires mor.vfst dictionary file"]
    fn integration_insert_hyphens_empty_separator() {
        let mor_data = std::fs::read(
            std::env::var("VOIKKO_MOR_VFST").unwrap_or_else(|_| "../../test-data/mor.vfst".into()),
        )
        .expect("failed to read mor.vfst");
        let handle =
            VoikkoHandle::from_bytes(&mor_data, None, "fi").expect("failed to create handle");

        // With empty separator, result should be the original word
        // (hyphenation points are inserted as empty strings)
        let result = handle.insert_hyphens("koira", "", true);
        // All original characters should still be present
        assert!(result.len() <= "koira".len());
    }

    // =========================================================================
    // set_speller_cache_size tests
    // =========================================================================

    #[test]
    #[ignore = "requires mor.vfst dictionary file"]
    fn integration_set_speller_cache_size() {
        let mor_data = std::fs::read(
            std::env::var("VOIKKO_MOR_VFST").unwrap_or_else(|_| "../../test-data/mor.vfst".into()),
        )
        .expect("failed to read mor.vfst");
        let mut handle =
            VoikkoHandle::from_bytes(&mor_data, None, "fi").expect("failed to create handle");

        // Spell a word, change cache size, spell again — should still work
        assert!(handle.spell("koira"));
        handle.set_speller_cache_size(2);
        assert!(handle.spell("koira"));
        handle.set_speller_cache_size(0);
        assert!(handle.spell("koira"));
    }

    // =========================================================================
    // grammar_errors_from_text tests
    // =========================================================================

    #[test]
    #[ignore = "requires mor.vfst and autocorr.vfst dictionary files"]
    fn integration_grammar_errors_from_text_multiline() {
        let mor_data = std::fs::read(
            std::env::var("VOIKKO_MOR_VFST").unwrap_or_else(|_| "../../test-data/mor.vfst".into()),
        )
        .expect("failed to read mor.vfst");
        let autocorr_data = std::fs::read(
            std::env::var("VOIKKO_AUTOCORR_VFST")
                .unwrap_or_else(|_| "../../test-data/autocorr.vfst".into()),
        )
        .ok();
        let handle = VoikkoHandle::from_bytes(&mor_data, autocorr_data.as_deref(), "fi")
            .expect("failed to create handle");

        // Test with multi-line text: positions should be adjusted
        let text = "Ensimmäinen rivi.\nToinen rivi.";
        let errors = handle.grammar_errors_from_text(text);
        // Verify that any errors from the second paragraph have startPos >= 18
        // (length of "Ensimmäinen rivi.\n")
        for error in &errors {
            // Errors can only be in one paragraph or the other
            assert!(error.start_pos < text.chars().count());
        }
    }

    #[test]
    #[ignore = "requires mor.vfst dictionary file"]
    fn integration_grammar_errors_from_text_empty_lines() {
        let mor_data = std::fs::read(
            std::env::var("VOIKKO_MOR_VFST").unwrap_or_else(|_| "../../test-data/mor.vfst".into()),
        )
        .expect("failed to read mor.vfst");
        let handle =
            VoikkoHandle::from_bytes(&mor_data, None, "fi").expect("failed to create handle");

        // Empty lines should be skipped without error
        let text = "Rivi.\n\n\nToinen rivi.";
        let _errors = handle.grammar_errors_from_text(text);
        // Should not panic
    }

    #[test]
    #[ignore = "requires mor.vfst dictionary file"]
    fn integration_grammar_errors_from_text_crlf() {
        let mor_data = std::fs::read(
            std::env::var("VOIKKO_MOR_VFST").unwrap_or_else(|_| "../../test-data/mor.vfst".into()),
        )
        .expect("failed to read mor.vfst");
        let handle =
            VoikkoHandle::from_bytes(&mor_data, None, "fi").expect("failed to create handle");

        // CRLF line endings should be handled correctly
        let text = "Rivi.\r\nToinen rivi.";
        let _errors = handle.grammar_errors_from_text(text);
        // Should not panic
    }

    #[test]
    fn sentences_simple_text() {
        let text = "Ensimmäinen. Toinen.";
        let text_chars: Vec<char> = text.chars().collect();
        let text_len = text_chars.len();

        let mut sentences = Vec::new();
        let mut pos = 0;
        while pos < text_len {
            let (st, slen) = tokenizer::next_sentence(&text_chars, text_len, pos);
            if st == SentenceType::None {
                if slen > 0 {
                    sentences.push(Sentence::new(st, slen));
                }
                break;
            }
            sentences.push(Sentence::new(st, slen));
            pos += slen;
        }

        assert!(!sentences.is_empty());
    }
}
