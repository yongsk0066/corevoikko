#![allow(dead_code)]
// FinnishRuleEngine: orchestrates all individual grammar checks on a paragraph.
//
// This module runs all sentence-level and paragraph-level checks in order
// and collects all grammar errors.
//
// Origin: grammar/FinnishRuleEngine.cpp, grammar/FinnishRuleEngine.hpp

use voikko_core::grammar_error::GrammarError;
use voikko_fst::unweighted::UnweightedTransducer;

use super::checks::{
    GrammarOptions, GrammarParagraph,
    gc_capitalization, gc_compound_verb, gc_end_punctuation, gc_local_punctuation,
    gc_missing_verb, gc_negative_verb_mismatch, gc_punctuation_of_quotations,
    gc_repeating_words, gc_sidesana,
};
use super::autocorrect::gc_autocorrect;

/// Finnish rule engine that orchestrates all grammar checks on a paragraph.
///
/// Origin: grammar/FinnishRuleEngine.hpp, FinnishRuleEngine.cpp
pub(crate) struct FinnishRuleEngine {
    /// Grammar checker options.
    options: GrammarOptions,
    /// Optional autocorrect transducer (loaded from autocorr.vfst).
    autocorrect_transducer: Option<UnweightedTransducer>,
}

impl FinnishRuleEngine {
    /// Create a new FinnishRuleEngine.
    ///
    /// The `autocorrect_transducer` is loaded from `autocorr.vfst` if available.
    /// If `None`, autocorrect checking is skipped.
    ///
    /// Origin: FinnishRuleEngine.cpp:47-59
    pub(crate) fn new(
        options: GrammarOptions,
        autocorrect_transducer: Option<UnweightedTransducer>,
    ) -> Self {
        Self {
            options,
            autocorrect_transducer,
        }
    }

    /// Update the grammar checker options.
    pub(crate) fn set_options(&mut self, options: GrammarOptions) {
        self.options = options;
    }

    /// Access the current options.
    pub(crate) fn options(&self) -> &GrammarOptions {
        &self.options
    }

    /// Check a paragraph for grammar errors.
    ///
    /// Runs all individual checks on each sentence, then paragraph-level
    /// checks (capitalization, end punctuation). Returns a collected list
    /// of all errors.
    ///
    /// The order of checks matches the C++ FinnishRuleEngine::check:
    /// 1. Per-sentence: local punctuation, quotation punctuation, repeating words
    /// 2. Per-sentence: verb checks (missing verb, negative verb mismatch,
    ///    compound verb, sidesana, autocorrect)
    /// 3. Paragraph-level: capitalization, end punctuation
    ///
    /// Origin: FinnishRuleEngine.cpp:69-86
    pub(crate) fn check(&self, paragraph: &GrammarParagraph) -> Vec<GrammarError> {
        let mut errors = Vec::new();

        // Per-sentence checks
        for sentence in &paragraph.sentences {
            // Punctuation and whitespace checks
            // Origin: FinnishRuleEngine.cpp:72
            errors.extend(gc_local_punctuation(sentence));

            // Quotation punctuation check
            // Origin: FinnishRuleEngine.cpp:73
            errors.extend(gc_punctuation_of_quotations(sentence));

            // Repeating word check
            // Origin: FinnishRuleEngine.cpp:74
            errors.extend(gc_repeating_words(sentence));

            // Missing verb and extra main verb check
            // Origin: FinnishRuleEngine.cpp:49 (MissingVerbCheck)
            // Note: MissingVerbCheck.cpp handles both missing and extra main verb
            errors.extend(gc_missing_verb(sentence, &self.options));

            // Negative verb mismatch check
            // Origin: FinnishRuleEngine.cpp:50 (NegativeVerbCheck)
            errors.extend(gc_negative_verb_mismatch(sentence));

            // Compound verb infinitive type check
            // Origin: FinnishRuleEngine.cpp:51 (CompoundVerbCheck)
            errors.extend(gc_compound_verb(sentence));

            // Misplaced conjunction check
            // Origin: FinnishRuleEngine.cpp:52 (SidesanaCheck)
            errors.extend(gc_sidesana(sentence));

            // Autocorrect check (if transducer available)
            // Origin: FinnishRuleEngine.cpp:54-58
            if let Some(ref transducer) = self.autocorrect_transducer {
                errors.extend(gc_autocorrect(sentence, transducer));
            }
        }

        // Paragraph-level checks

        // Capitalization check (operates across sentences)
        // Origin: FinnishRuleEngine.cpp:83
        errors.extend(gc_capitalization(paragraph, &self.options));

        // End punctuation check
        // Origin: FinnishRuleEngine.cpp:84
        errors.extend(gc_end_punctuation(paragraph, &self.options));

        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voikko_core::enums::TokenType;
    use voikko_core::grammar_error::{
        GCERR_EXTRA_WHITESPACE, GCERR_REPEATING_WORD, GCERR_TERMINATING_PUNCTUATION_MISSING,
        GCERR_WRITE_FIRST_UPPERCASE,
    };

    use super::super::checks::GrammarOptions;
    use super::super::paragraph::{GrammarSentence, GrammarToken, Paragraph};

    fn word(text: &str, pos: usize) -> GrammarToken {
        GrammarToken::new(TokenType::Word, text.chars().collect(), pos)
    }

    fn ws(text: &str, pos: usize) -> GrammarToken {
        GrammarToken::new(TokenType::Whitespace, text.chars().collect(), pos)
    }

    fn punct(text: &str, pos: usize) -> GrammarToken {
        GrammarToken::new(TokenType::Punctuation, text.chars().collect(), pos)
    }

    fn sentence(tokens: Vec<GrammarToken>, pos: usize) -> GrammarSentence {
        let mut s = GrammarSentence::new(pos);
        s.tokens = tokens;
        s
    }

    type GrammarParagraph = Paragraph;

    #[test]
    fn engine_detects_extra_whitespace() {
        let s = sentence(
            vec![word("Koira", 0), ws("  ", 5), word("kissa", 7), punct(".", 12)],
            0,
        );
        let p = GrammarParagraph {
            sentences: vec![s],
        };
        let engine = FinnishRuleEngine::new(GrammarOptions::default(), None);
        let errs = engine.check(&p);
        assert!(errs.iter().any(|e| e.error_code == GCERR_EXTRA_WHITESPACE));
    }

    #[test]
    fn engine_detects_repeating_word() {
        let s = sentence(
            vec![
                word("Koira", 0),
                ws(" ", 5),
                word("koira", 6),
                punct(".", 11),
            ],
            0,
        );
        let p = GrammarParagraph {
            sentences: vec![s],
        };
        let engine = FinnishRuleEngine::new(GrammarOptions::default(), None);
        let errs = engine.check(&p);
        assert!(errs.iter().any(|e| e.error_code == GCERR_REPEATING_WORD));
    }

    #[test]
    fn engine_detects_missing_end_punctuation() {
        let s1 = sentence(
            vec![word("Koira", 0), ws(" ", 5), word("juoksee", 6), punct(".", 13)],
            0,
        );
        let s2 = sentence(vec![word("Kissa", 15), ws(" ", 20), word("nukkuu", 21)], 15);
        let p = GrammarParagraph {
            sentences: vec![s1, s2],
        };
        let engine = FinnishRuleEngine::new(GrammarOptions::default(), None);
        let errs = engine.check(&p);
        assert!(errs
            .iter()
            .any(|e| e.error_code == GCERR_TERMINATING_PUNCTUATION_MISSING));
    }

    #[test]
    fn engine_detects_capitalization_error() {
        let mut w1 = word("koira", 0);
        w1.is_valid_word = true;
        w1.first_letter_lcase = true;
        let s = sentence(vec![w1, punct(".", 5)], 0);
        let p = GrammarParagraph {
            sentences: vec![s],
        };
        let engine = FinnishRuleEngine::new(GrammarOptions::default(), None);
        let errs = engine.check(&p);
        assert!(errs
            .iter()
            .any(|e| e.error_code == GCERR_WRITE_FIRST_UPPERCASE));
    }

    #[test]
    fn engine_empty_paragraph_no_errors() {
        let p = GrammarParagraph {
            sentences: vec![],
        };
        let engine = FinnishRuleEngine::new(GrammarOptions::default(), None);
        let errs = engine.check(&p);
        assert!(errs.is_empty());
    }

    #[test]
    fn engine_options_suppress_end_punctuation() {
        let s = sentence(vec![word("Otsikko", 0)], 0);
        let p = GrammarParagraph {
            sentences: vec![s],
        };
        let opts = GrammarOptions {
            accept_titles_in_gc: true,
            ..Default::default()
        };
        let engine = FinnishRuleEngine::new(opts, None);
        let errs = engine.check(&p);
        assert!(!errs
            .iter()
            .any(|e| e.error_code == GCERR_TERMINATING_PUNCTUATION_MISSING));
    }

    #[test]
    fn engine_no_autocorrect_without_transducer() {
        let s = sentence(
            vec![word("Koira", 0), punct(".", 5)],
            0,
        );
        let p = GrammarParagraph {
            sentences: vec![s],
        };
        let engine = FinnishRuleEngine::new(GrammarOptions::default(), None);
        let errs = engine.check(&p);
        // No GCERR_INVALID_SPELLING expected without a transducer
        assert!(!errs
            .iter()
            .any(|e| e.error_code == voikko_core::grammar_error::GCERR_INVALID_SPELLING));
    }
}
