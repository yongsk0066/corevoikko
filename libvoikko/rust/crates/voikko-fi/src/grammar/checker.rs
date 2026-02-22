// FinnishGrammarChecker: top-level grammar checker for Finnish.
//
// Implements the GrammarChecker trait. Takes text, tokenizes into paragraphs,
// runs the FinnishRuleEngine on each sentence, and collects errors.
//
// Origin: grammar/FinnishGrammarChecker.cpp, grammar/GrammarChecker.hpp

use std::cell::RefCell;

use voikko_core::enums::TokenType;
use voikko_core::grammar_error::GrammarError;

use super::GrammarChecker;
use super::cache::GcCache;
use super::checks::GrammarOptions;
use super::engine::FinnishRuleEngine;
use super::finnish_analysis::analyse_token;
use super::paragraph::{self, GrammarSentence, GrammarToken, Paragraph};
use crate::morphology::Analyzer;
use crate::tokenizer;

/// Top-level Finnish grammar checker.
///
/// Owns the rule engine and the grammar cache. The cache uses `RefCell` for
/// interior mutability so that the `GrammarChecker` trait (`&self`) can
/// read and update the cache.
///
/// Optionally holds a reference to a morphological analyzer. When an analyzer
/// is available, `analyse_paragraph` is used instead of `tokenize_paragraph`,
/// enabling richer grammar checks (verb detection, case checks, etc.).
///
/// Origin: grammar/FinnishGrammarChecker.hpp, FinnishGrammarChecker.cpp
pub(crate) struct FinnishGrammarChecker<'a> {
    /// The rule engine that orchestrates all individual checks.
    engine: FinnishRuleEngine,
    /// Cache for grammar checking results (interior mutability for &self).
    cache: RefCell<GcCache>,
    /// Optional morphological analyzer for enriched grammar analysis.
    analyzer: Option<&'a dyn Analyzer>,
}

impl<'a> FinnishGrammarChecker<'a> {
    /// Create a new FinnishGrammarChecker with the given options.
    ///
    /// The autocorrect transducer is optional; if `None`, autocorrect
    /// checking is skipped. The analyzer is optional; if `None`, only
    /// structural tokenization is used (no morphological annotation).
    ///
    /// Origin: FinnishGrammarChecker.cpp:37-40
    pub(crate) fn new(
        options: GrammarOptions,
        autocorrect_transducer: Option<voikko_fst::unweighted::UnweightedTransducer>,
        analyzer: Option<&'a dyn Analyzer>,
    ) -> Self {
        Self {
            engine: FinnishRuleEngine::new(options, autocorrect_transducer),
            cache: RefCell::new(GcCache::new()),
            analyzer,
        }
    }

    /// Update the grammar checker options.
    pub(crate) fn set_options(&mut self, options: GrammarOptions) {
        self.engine.set_options(options);
    }

    /// Access the cache (for error retrieval).
    pub(crate) fn cache(&self) -> &RefCell<GcCache> {
        &self.cache
    }

    /// Build a `Paragraph` from text, using `analyse_paragraph` with
    /// morphological annotation when an analyzer is available, or falling
    /// back to `tokenize_paragraph` (structural tokenization only).
    fn build_paragraph(&self, text: &[char], text_len: usize) -> Paragraph {
        if let Some(analyzer) = self.analyzer {
            // Use analyse_paragraph with morphological token annotation.
            // Origin: FinnishAnalysis.cpp:analyseParagraph
            let mut analyse_fn = |token: &mut GrammarToken| {
                analyse_token(token, analyzer);
            };
            match paragraph::analyse_paragraph(text, text_len, &mut analyse_fn) {
                Some(p) => p,
                // Sentence too long; fall back to structural tokenization.
                None => Self::tokenize_paragraph(text, text_len),
            }
        } else {
            Self::tokenize_paragraph(text, text_len)
        }
    }

    /// Tokenize text into a `Paragraph` (sentences with annotated tokens).
    ///
    /// This is a simplified tokenization that creates `GrammarToken` values
    /// from the tokenizer output. In the full pipeline, `FinnishAnalysis`
    /// (Phase 4-A) would also run morphological analysis to set the grammar
    /// flags (is_valid_word, is_main_verb, etc.). This method provides the
    /// structural tokenization only.
    ///
    /// Origin: grammar/FinnishAnalysis.cpp (simplified)
    fn tokenize_paragraph(text: &[char], text_len: usize) -> Paragraph {
        let mut sentences = Vec::new();
        let mut para_pos: usize = 0;

        while para_pos < text_len {
            let (sentence_type, sentence_len) =
                tokenizer::next_sentence(text, text_len, para_pos);

            if sentence_type == voikko_core::enums::SentenceType::None && sentence_len == 0 {
                break;
            }

            // Tokenize the sentence span into grammar tokens
            let sentence_end = para_pos + sentence_len;
            let mut tokens = Vec::new();
            let mut tok_pos = para_pos;

            while tok_pos < sentence_end {
                let (token_type, token_len) =
                    tokenizer::next_token(text, text_len, tok_pos);

                if token_type == TokenType::None || token_len == 0 {
                    break;
                }

                let token_text: Vec<char> = text[tok_pos..tok_pos + token_len].to_vec();
                let token = GrammarToken::new(token_type, token_text, tok_pos);
                tokens.push(token);
                tok_pos += token_len;
            }

            if !tokens.is_empty() {
                let mut s = GrammarSentence::new(para_pos);
                s.tokens = tokens;
                sentences.push(s);
            }

            if sentence_len == 0 {
                break;
            }
            para_pos += sentence_len;
        }

        // If no sentence boundary was found, treat the entire text as one sentence
        if sentences.is_empty() && text_len > 0 {
            let mut tokens = Vec::new();
            let mut tok_pos = 0;
            while tok_pos < text_len {
                let (token_type, token_len) =
                    tokenizer::next_token(text, text_len, tok_pos);
                if token_type == TokenType::None || token_len == 0 {
                    break;
                }
                let token_text: Vec<char> = text[tok_pos..tok_pos + token_len].to_vec();
                tokens.push(GrammarToken::new(token_type, token_text, tok_pos));
                tok_pos += token_len;
            }
            if !tokens.is_empty() {
                let mut s = GrammarSentence::new(0);
                s.tokens = tokens;
                sentences.push(s);
            }
        }

        Paragraph { sentences }
    }

    /// Check a paragraph for grammar errors using an externally-provided analyzer.
    ///
    /// This allows the caller (e.g., VoikkoHandle) to pass its own analyzer
    /// without requiring the checker to hold a lifetime-bound reference.
    /// The checker's cache and autocorrect transducer are still used.
    ///
    /// Origin: grammar/GrammarChecker.cpp:paragraphToCache (with external analyzer)
    pub(crate) fn check_with_analyzer(
        &self,
        text: &[char],
        text_len: usize,
        analyzer: &dyn Analyzer,
    ) -> Vec<GrammarError> {
        // Check cache first
        {
            let cache = self.cache.borrow();
            if let Some(cached) = cache.check_cache(text) {
                return cached.to_vec();
            }
        }

        // Build paragraph with morphological analysis
        let mut analyse_fn = |token: &mut GrammarToken| {
            analyse_token(token, analyzer);
        };
        let paragraph = match paragraph::analyse_paragraph(text, text_len, &mut analyse_fn) {
            Some(p) => p,
            None => Self::tokenize_paragraph(text, text_len),
        };
        let errors = self.engine.check(&paragraph);

        // Store in cache
        self.cache.borrow_mut().store_cache(text, errors.clone());

        errors
    }
}

impl GrammarChecker for FinnishGrammarChecker<'_> {
    /// Check a paragraph for grammar errors.
    ///
    /// Uses `analyse_paragraph` when a morphological analyzer is available,
    /// falling back to `tokenize_paragraph` for structural-only tokenization.
    /// Runs all checks and returns collected errors. Results are cached.
    ///
    /// Origin: grammar/GrammarChecker.cpp:paragraphToCache + errorFromCache
    fn check(&self, text: &[char], text_len: usize) -> Vec<GrammarError> {
        // Check cache first
        {
            let cache = self.cache.borrow();
            if let Some(cached) = cache.check_cache(text) {
                return cached.to_vec();
            }
        }

        let paragraph = self.build_paragraph(text, text_len);
        let errors = self.engine.check(&paragraph);

        // Store in cache
        self.cache.borrow_mut().store_cache(text, errors.clone());

        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voikko_core::analysis::Analysis;

    fn check_text(text: &str) -> Vec<GrammarError> {
        let chars: Vec<char> = text.chars().collect();
        let checker = FinnishGrammarChecker::new(GrammarOptions::default(), None, None);
        checker.check(&chars, chars.len())
    }

    #[test]
    fn checker_empty_text_no_errors() {
        let errs = check_text("");
        assert!(errs.is_empty());
    }

    #[test]
    fn checker_detects_extra_whitespace() {
        // "Koira  kissa." has extra whitespace between words
        let errs = check_text("Koira  kissa.");
        assert!(errs
            .iter()
            .any(|e| e.error_code == voikko_core::grammar_error::GCERR_EXTRA_WHITESPACE));
    }

    #[test]
    fn checker_detects_repeating_word() {
        // "Koira koira juoksee." has a repeating word
        let errs = check_text("Koira koira juoksee.");
        assert!(errs
            .iter()
            .any(|e| e.error_code == voikko_core::grammar_error::GCERR_REPEATING_WORD));
    }

    #[test]
    fn checker_normal_sentence_minimal_errors() {
        // "Koira juoksee." — a simple valid sentence
        // Without morphological analysis, some verb-related checks may still trigger
        // because is_valid_word defaults to false (treating words as unrecognized).
        let errs = check_text("Koira juoksee.");
        // Should not have extra whitespace or repeating word errors
        assert!(!errs
            .iter()
            .any(|e| e.error_code == voikko_core::grammar_error::GCERR_EXTRA_WHITESPACE));
        assert!(!errs
            .iter()
            .any(|e| e.error_code == voikko_core::grammar_error::GCERR_REPEATING_WORD));
    }

    #[test]
    fn checker_tokenizes_multiple_sentences() {
        let text = "Koira juoksi. Kissa nukkui.";
        let chars: Vec<char> = text.chars().collect();
        let paragraph = FinnishGrammarChecker::tokenize_paragraph(&chars, chars.len());
        // Should detect at least one sentence boundary
        assert!(paragraph.sentences.len() >= 1);
    }

    #[test]
    fn checker_space_before_comma() {
        let errs = check_text("Koira ,kissa.");
        assert!(errs
            .iter()
            .any(|e| e.error_code == voikko_core::grammar_error::GCERR_SPACE_BEFORE_PUNCTUATION));
    }

    #[test]
    fn checker_implements_trait() {
        // Verify that FinnishGrammarChecker implements GrammarChecker
        let checker = FinnishGrammarChecker::new(GrammarOptions::default(), None, None);
        let chars: Vec<char> = "Koira.".chars().collect();
        let _errs: Vec<GrammarError> = GrammarChecker::check(&checker, &chars, chars.len());
    }

    // -- Tests with analyzer --

    /// A mock analyzer that returns pre-configured analyses.
    struct MockAnalyzer {
        entries: Vec<(String, Vec<Analysis>)>,
    }

    impl MockAnalyzer {
        fn new() -> Self {
            Self {
                entries: Vec::new(),
            }
        }

        fn add(&mut self, word: &str, analyses: Vec<Analysis>) {
            self.entries.push((word.to_string(), analyses));
        }
    }

    impl Analyzer for MockAnalyzer {
        fn analyze(&self, word: &[char], _word_len: usize) -> Vec<Analysis> {
            let word_str: String = word.iter().collect();
            for (w, analyses) in &self.entries {
                if *w == word_str {
                    return analyses.clone();
                }
            }
            Vec::new()
        }
    }

    fn make_analysis(pairs: &[(&str, &str)]) -> Analysis {
        let mut a = Analysis::new();
        for &(k, v) in pairs {
            a.set(k, v);
        }
        a
    }

    #[test]
    fn checker_with_analyzer_uses_analyse_paragraph() {
        use voikko_core::analysis::{ATTR_CLASS, ATTR_STRUCTURE};

        let mut analyzer = MockAnalyzer::new();
        analyzer.add(
            "Koira",
            vec![make_analysis(&[
                (ATTR_STRUCTURE, "=ipppp"),
                (ATTR_CLASS, "nimisana"),
            ])],
        );
        analyzer.add(
            "juoksee",
            vec![make_analysis(&[
                (ATTR_STRUCTURE, "=ppppppp"),
                (ATTR_CLASS, "teonsana"),
            ])],
        );

        let checker =
            FinnishGrammarChecker::new(GrammarOptions::default(), None, Some(&analyzer));
        let text = "Koira juoksee.";
        let chars: Vec<char> = text.chars().collect();
        let paragraph = checker.build_paragraph(&chars, chars.len());

        // With the analyzer, the first word should be marked as valid.
        let first_word = paragraph.sentences[0]
            .tokens
            .iter()
            .find(|t| t.token_type == TokenType::Word)
            .unwrap();
        assert!(
            first_word.is_valid_word,
            "Expected 'Koira' to be marked as valid word with analyzer"
        );
    }

    #[test]
    fn checker_without_analyzer_uses_tokenize_paragraph() {
        // Without analyzer, words should NOT be marked as valid
        let checker = FinnishGrammarChecker::new(GrammarOptions::default(), None, None);
        let text = "Koira juoksee.";
        let chars: Vec<char> = text.chars().collect();
        let paragraph = checker.build_paragraph(&chars, chars.len());

        let first_word = paragraph.sentences[0]
            .tokens
            .iter()
            .find(|t| t.token_type == TokenType::Word)
            .unwrap();
        assert!(
            !first_word.is_valid_word,
            "Expected word not to be marked valid without analyzer"
        );
    }
}
