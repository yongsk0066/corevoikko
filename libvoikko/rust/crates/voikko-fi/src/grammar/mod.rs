// Grammar checking module
// Origin: grammar/

#[allow(dead_code)]
pub mod autocorrect;
#[allow(dead_code)]
pub mod cache;
#[allow(dead_code)]
pub mod checker;
#[allow(dead_code)]
pub mod checks;
#[allow(dead_code)]
pub mod engine;
#[allow(dead_code)]
pub mod finnish_analysis;
#[allow(dead_code)]
pub mod paragraph;

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
