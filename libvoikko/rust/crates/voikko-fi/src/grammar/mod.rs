// Grammar checking module
// Origin: grammar/

pub mod cache;
pub mod paragraph;
pub mod finnish_analysis;
pub mod checks;
pub mod autocorrect;
pub mod engine;
pub mod checker;

use voikko_core::grammar_error::GrammarError;

/// Trait for grammar checkers.
///
/// Origin: grammar/GrammarChecker.hpp
pub trait GrammarChecker {
    /// Check a paragraph for grammar errors.
    ///
    /// Returns a list of grammar errors found in the paragraph text.
    /// The text is provided as a char slice for random-access indexing.
    fn check(&self, text: &[char], text_len: usize) -> Vec<GrammarError>;
}
