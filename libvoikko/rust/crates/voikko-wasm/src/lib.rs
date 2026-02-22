// WASM bindings for Voikko Finnish NLP.
//
// Provides a `WasmVoikko` class exported via wasm-bindgen that wraps
// the `VoikkoHandle` from voikko-fi. All complex return types (Analysis,
// GrammarError, Token, Sentence) are serialized to JavaScript values
// using serde-wasm-bindgen.
//
// Usage from JavaScript:
//
//   const voikko = new WasmVoikko(morVfstBytes, autocorrVfstBytes);
//   voikko.spell("koira");       // => true
//   voikko.suggest("koirra");    // => ["koira", ...]
//   voikko.analyze("koira");     // => [{ CLASS: "nimisana", ... }, ...]
//   voikko.hyphenate("koira");   // => "   - "
//   voikko.grammarErrors("...");  // => [{ errorCode: 2, ... }, ...]
//   voikko.tokens("Koira.");     // => [{ tokenType: "Word", ... }, ...]
//   voikko.sentences("A. B.");    // => [{ sentenceType: "Probable", ... }, ...]
//   voikko.terminate();           // optional cleanup

use serde::Serialize;
use wasm_bindgen::prelude::*;

use voikko_fi::handle::{VoikkoError, VoikkoHandle};

// ============================================================================
// Serde-serializable DTO types for JS interop
// ============================================================================

/// Serializable representation of a grammar error.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsGrammarError {
    error_code: i32,
    start_pos: usize,
    error_len: usize,
    suggestions: Vec<String>,
    short_description: String,
}

/// Serializable representation of a token.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsToken {
    token_type: String,
    text: String,
    token_len: usize,
    pos: usize,
}

/// Serializable representation of a sentence boundary.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsSentence {
    sentence_type: String,
    sentence_len: usize,
}

// ============================================================================
// Conversion helpers
// ============================================================================

fn token_type_to_string(tt: voikko_core::enums::TokenType) -> String {
    match tt {
        voikko_core::enums::TokenType::None => "None".to_string(),
        voikko_core::enums::TokenType::Word => "Word".to_string(),
        voikko_core::enums::TokenType::Punctuation => "Punctuation".to_string(),
        voikko_core::enums::TokenType::Whitespace => "Whitespace".to_string(),
        voikko_core::enums::TokenType::Unknown => "Unknown".to_string(),
    }
}

fn sentence_type_to_string(st: voikko_core::enums::SentenceType) -> String {
    match st {
        voikko_core::enums::SentenceType::None => "None".to_string(),
        voikko_core::enums::SentenceType::NoStart => "NoStart".to_string(),
        voikko_core::enums::SentenceType::Probable => "Probable".to_string(),
        voikko_core::enums::SentenceType::Possible => "Possible".to_string(),
    }
}

fn voikko_error_to_js(e: VoikkoError) -> JsError {
    JsError::new(&e.to_string())
}

// ============================================================================
// WasmVoikko
// ============================================================================

/// Finnish NLP engine for WebAssembly.
///
/// Provides spell checking, morphological analysis, hyphenation, grammar
/// checking, suggestion generation, and tokenization for Finnish text.
#[wasm_bindgen]
pub struct WasmVoikko {
    handle: VoikkoHandle,
}

#[wasm_bindgen]
impl WasmVoikko {
    /// Create a new WasmVoikko instance from raw dictionary data.
    ///
    /// - `mor_data`: contents of `mor.vfst` (morphology transducer, required)
    /// - `autocorr_data`: contents of `autocorr.vfst` (autocorrect transducer, optional)
    #[wasm_bindgen(constructor)]
    pub fn new(mor_data: &[u8], autocorr_data: Option<Vec<u8>>) -> Result<WasmVoikko, JsError> {
        let handle = VoikkoHandle::from_bytes(
            mor_data,
            autocorr_data.as_deref(),
            "fi",
        )
        .map_err(voikko_error_to_js)?;
        Ok(WasmVoikko { handle })
    }

    /// Check whether a word is correctly spelled.
    pub fn spell(&self, word: &str) -> bool {
        self.handle.spell(word)
    }

    /// Generate spelling suggestions for a misspelled word.
    ///
    /// Returns an array of suggested corrections, sorted by priority (best first).
    pub fn suggest(&self, word: &str) -> Vec<String> {
        self.handle.suggest(word)
    }

    /// Perform morphological analysis on a word.
    ///
    /// Returns a JavaScript array of analysis objects. Each object contains
    /// string key-value pairs for morphological attributes (CLASS, BASEFORM,
    /// STRUCTURE, etc.).
    pub fn analyze(&self, word: &str) -> Result<JsValue, JsError> {
        let analyses = self.handle.analyze(word);
        let arr = js_sys::Array::new();
        for a in &analyses {
            let obj = js_sys::Object::new();
            for (k, v) in a.attributes() {
                js_sys::Reflect::set(&obj, &JsValue::from_str(k), &JsValue::from_str(v))
                    .map_err(|e| JsError::new(&format!("{e:?}")))?;
            }
            arr.push(&obj);
        }
        Ok(arr.into())
    }

    /// Hyphenate a word.
    ///
    /// Returns a pattern string of the same character length as the input word.
    /// Each character indicates the hyphenation status at that position:
    /// - `' '` (space): no hyphenation point
    /// - `'-'`: hyphenation point before this character
    /// - `'='`: hyphenation point with explicit hyphen
    pub fn hyphenate(&self, word: &str) -> String {
        self.handle.hyphenate(word)
    }

    /// Check a paragraph of text for grammar errors.
    ///
    /// Returns a JavaScript array of grammar error objects with fields:
    /// `errorCode`, `startPos`, `errorLen`, `suggestions`.
    #[wasm_bindgen(js_name = "grammarErrors")]
    pub fn grammar_errors(&self, text: &str) -> Result<JsValue, JsError> {
        let errors = self.handle.grammar_errors(text);
        let js_errors: Vec<JsGrammarError> = errors
            .into_iter()
            .map(|e| JsGrammarError {
                error_code: e.error_code,
                start_pos: e.start_pos,
                error_len: e.error_len,
                suggestions: e.suggestions,
                short_description: e.short_description,
            })
            .collect();
        serde_wasm_bindgen::to_value(&js_errors)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Tokenize text into a list of tokens.
    ///
    /// Returns a JavaScript array of token objects with fields:
    /// `tokenType` ("Word", "Punctuation", "Whitespace", "Unknown"),
    /// `text`, `tokenLen`, `pos`.
    pub fn tokens(&self, text: &str) -> Result<JsValue, JsError> {
        let tokens = self.handle.tokens(text);
        let js_tokens: Vec<JsToken> = tokens
            .into_iter()
            .map(|t| JsToken {
                token_type: token_type_to_string(t.token_type),
                text: t.text,
                token_len: t.token_len,
                pos: t.pos,
            })
            .collect();
        serde_wasm_bindgen::to_value(&js_tokens)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Detect sentence boundaries in text.
    ///
    /// Returns a JavaScript array of sentence objects with fields:
    /// `sentenceType` ("Probable", "Possible", "None"), `sentenceLen`.
    pub fn sentences(&self, text: &str) -> Result<JsValue, JsError> {
        let sentences = self.handle.sentences(text);
        let js_sentences: Vec<JsSentence> = sentences
            .into_iter()
            .map(|s| JsSentence {
                sentence_type: sentence_type_to_string(s.sentence_type),
                sentence_len: s.sentence_len,
            })
            .collect();
        serde_wasm_bindgen::to_value(&js_sentences)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Hyphenate a word with the given separator inserted at hyphenation points.
    ///
    /// - `separator`: string to insert at hyphenation points (e.g. "-", "\u{00AD}")
    /// - `allow_context_changes`: if true, handle compound-boundary replacements
    #[wasm_bindgen(js_name = "insertHyphens")]
    pub fn insert_hyphens(&self, word: &str, separator: &str, allow_context_changes: bool) -> String {
        self.handle.insert_hyphens(word, separator, allow_context_changes)
    }

    /// Get possible values for an enumerated morphological attribute.
    ///
    /// Returns null if the attribute name is not recognized.
    #[wasm_bindgen(js_name = "attributeValues")]
    pub fn attribute_values(&self, attribute_name: &str) -> Option<Vec<String>> {
        VoikkoHandle::attribute_values(attribute_name)
            .map(|vals| vals.iter().map(|s| s.to_string()).collect())
    }

    /// Check text for grammar errors, splitting at newline boundaries.
    ///
    /// Each line is treated as a separate paragraph. Error positions are
    /// relative to the full input text.
    #[wasm_bindgen(js_name = "grammarErrorsFromText")]
    pub fn grammar_errors_from_text(&self, text: &str) -> Result<JsValue, JsError> {
        let errors = self.handle.grammar_errors_from_text(text);
        let js_errors: Vec<JsGrammarError> = errors
            .into_iter()
            .map(|e| JsGrammarError {
                error_code: e.error_code,
                start_pos: e.start_pos,
                error_len: e.error_len,
                suggestions: e.suggestions,
                short_description: e.short_description,
            })
            .collect();
        serde_wasm_bindgen::to_value(&js_errors)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Get the library version string.
    #[wasm_bindgen(js_name = "getVersion")]
    pub fn get_version() -> String {
        VoikkoHandle::get_version().to_string()
    }

    /// Replace the speller cache with a new one of the given size parameter.
    #[wasm_bindgen(js_name = "setSpellerCacheSize")]
    pub fn set_speller_cache_size(&mut self, size: usize) {
        self.handle.set_speller_cache_size(size);
    }

    /// Release resources held by this instance.
    ///
    /// After calling this method, the instance should not be used.
    /// In practice, WASM memory is managed by the garbage collector
    /// (or FinalizationRegistry), but this method allows explicit cleanup.
    pub fn terminate(self) {
        // Drop self, releasing all resources.
    }

    // =========================================================================
    // Option setters
    // =========================================================================

    /// Set whether to ignore trailing dots in spell checking.
    #[wasm_bindgen(js_name = "setIgnoreDot")]
    pub fn set_ignore_dot(&mut self, value: bool) {
        self.handle.set_ignore_dot(value);
    }

    /// Set whether to ignore words containing numbers in spell checking.
    #[wasm_bindgen(js_name = "setIgnoreNumbers")]
    pub fn set_ignore_numbers(&mut self, value: bool) {
        self.handle.set_ignore_numbers(value);
    }

    /// Set whether to accept words written entirely in uppercase without checking.
    #[wasm_bindgen(js_name = "setIgnoreUppercase")]
    pub fn set_ignore_uppercase(&mut self, value: bool) {
        self.handle.set_ignore_uppercase(value);
    }

    /// Set whether to suppress ugly but correct hyphenation points.
    #[wasm_bindgen(js_name = "setNoUglyHyphenation")]
    pub fn set_no_ugly_hyphenation(&mut self, value: bool) {
        self.handle.set_no_ugly_hyphenation(value);
    }

    /// Set whether to accept words with a capitalized first letter.
    #[wasm_bindgen(js_name = "setAcceptFirstUppercase")]
    pub fn set_accept_first_uppercase(&mut self, value: bool) {
        self.handle.set_accept_first_uppercase(value);
    }

    /// Set whether to accept words with all letters capitalized.
    #[wasm_bindgen(js_name = "setAcceptAllUppercase")]
    pub fn set_accept_all_uppercase(&mut self, value: bool) {
        self.handle.set_accept_all_uppercase(value);
    }

    /// Set whether to use OCR-optimized suggestions.
    #[wasm_bindgen(js_name = "setOcrSuggestions")]
    pub fn set_ocr_suggestions(&mut self, value: bool) {
        self.handle.set_ocr_suggestions(value);
    }

    /// Set whether to ignore non-words (URLs, email addresses, etc.).
    #[wasm_bindgen(js_name = "setIgnoreNonwords")]
    pub fn set_ignore_nonwords(&mut self, value: bool) {
        self.handle.set_ignore_nonwords(value);
    }

    /// Set whether to accept extra hyphens in compound words.
    #[wasm_bindgen(js_name = "setAcceptExtraHyphens")]
    pub fn set_accept_extra_hyphens(&mut self, value: bool) {
        self.handle.set_accept_extra_hyphens(value);
    }

    /// Set whether to accept missing hyphens at start/end of word.
    #[wasm_bindgen(js_name = "setAcceptMissingHyphens")]
    pub fn set_accept_missing_hyphens(&mut self, value: bool) {
        self.handle.set_accept_missing_hyphens(value);
    }

    /// Set whether to accept incomplete sentences in titles (grammar checking).
    #[wasm_bindgen(js_name = "setAcceptTitlesInGc")]
    pub fn set_accept_titles_in_gc(&mut self, value: bool) {
        self.handle.set_accept_titles_in_gc(value);
    }

    /// Set whether to accept incomplete sentences at end of paragraph.
    #[wasm_bindgen(js_name = "setAcceptUnfinishedParagraphsInGc")]
    pub fn set_accept_unfinished_paragraphs_in_gc(&mut self, value: bool) {
        self.handle.set_accept_unfinished_paragraphs_in_gc(value);
    }

    /// Set whether to hyphenate unknown words.
    #[wasm_bindgen(js_name = "setHyphenateUnknownWords")]
    pub fn set_hyphenate_unknown_words(&mut self, value: bool) {
        self.handle.set_hyphenate_unknown_words(value);
    }

    /// Set whether to accept bulleted list paragraphs in grammar checking.
    #[wasm_bindgen(js_name = "setAcceptBulletedListsInGc")]
    pub fn set_accept_bulleted_lists_in_gc(&mut self, value: bool) {
        self.handle.set_accept_bulleted_lists_in_gc(value);
    }

    /// Set the minimum word length for hyphenation.
    #[wasm_bindgen(js_name = "setMinHyphenatedWordLength")]
    pub fn set_min_hyphenated_word_length(&mut self, value: usize) {
        self.handle.set_min_hyphenated_word_length(value);
    }

    /// Set the maximum number of suggestions to return.
    #[wasm_bindgen(js_name = "setMaxSuggestions")]
    pub fn set_max_suggestions(&mut self, value: usize) {
        self.handle.set_max_suggestions(value);
    }
}
