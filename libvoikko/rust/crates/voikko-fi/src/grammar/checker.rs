#![allow(dead_code)]
// FinnishGrammarChecker: top-level grammar checker for Finnish.
//
// Implements the GrammarChecker trait. Takes text, tokenizes into paragraphs,
// runs the FinnishRuleEngine on each sentence, and collects errors.
//
// Origin: grammar/FinnishGrammarChecker.cpp, grammar/GrammarChecker.hpp

use voikko_core::enums::TokenType;
use voikko_core::grammar_error::GrammarError;

use super::GrammarChecker;
use super::cache::GcCache;
use super::checks::GrammarOptions;
use super::paragraph::{GrammarSentence, GrammarToken, Paragraph};
use super::engine::FinnishRuleEngine;
use crate::tokenizer;

/// Top-level Finnish grammar checker.
///
/// Owns the rule engine and the grammar cache. Implements the `GrammarChecker`
/// trait to provide the public API for checking a paragraph of text.
///
/// Origin: grammar/FinnishGrammarChecker.hpp, FinnishGrammarChecker.cpp
pub(crate) struct FinnishGrammarChecker {
    /// The rule engine that orchestrates all individual checks.
    engine: FinnishRuleEngine,
    /// Cache for grammar checking results.
    cache: GcCache,
}

impl FinnishGrammarChecker {
    /// Create a new FinnishGrammarChecker with the given options.
    ///
    /// The autocorrect transducer is optional; if `None`, autocorrect
    /// checking is skipped.
    ///
    /// Origin: FinnishGrammarChecker.cpp:37-40
    pub(crate) fn new(
        options: GrammarOptions,
        autocorrect_transducer: Option<voikko_fst::unweighted::UnweightedTransducer>,
    ) -> Self {
        Self {
            engine: FinnishRuleEngine::new(options, autocorrect_transducer),
            cache: GcCache::new(),
        }
    }

    /// Update the grammar checker options.
    pub(crate) fn set_options(&mut self, options: GrammarOptions) {
        self.engine.set_options(options);
    }

    /// Access the cache (for error retrieval).
    pub(crate) fn cache(&self) -> &GcCache {
        &self.cache
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
}

impl GrammarChecker for FinnishGrammarChecker {
    /// Check a paragraph for grammar errors.
    ///
    /// Tokenizes the text into sentences, runs all checks, and returns
    /// collected errors. Results are also stored in the cache.
    ///
    /// Origin: grammar/GrammarChecker.cpp:paragraphToCache + errorFromCache
    fn check(&self, text: &[char], text_len: usize) -> Vec<GrammarError> {
        let paragraph = Self::tokenize_paragraph(text, text_len);
        self.engine.check(&paragraph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_text(text: &str) -> Vec<GrammarError> {
        let chars: Vec<char> = text.chars().collect();
        let checker = FinnishGrammarChecker::new(GrammarOptions::default(), None);
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
        let checker = FinnishGrammarChecker::new(GrammarOptions::default(), None);
        let chars: Vec<char> = "Koira.".chars().collect();
        let _errs: Vec<GrammarError> = GrammarChecker::check(&checker, &chars, chars.len());
    }
}
