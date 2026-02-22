// Suggestion generation module -- Phase 3-C
//
// Produces spelling correction candidates for misspelled words by applying
// various edit operations (deletion, insertion, replacement, swap, etc.)
// and validating them through the speller.
//
// Architecture:
//   - `generators`: individual edit-operation generators (SuggestionGenerator trait)
//   - `status`: tracking object for abort conditions, cost budget, deduplication
//   - `strategy`: orchestrator that composes generators into typing / OCR pipelines
//
// Origin: spellchecker/suggestion/

pub mod generators;
pub mod status;
pub mod strategy;

// Re-export key types for convenient access.
pub use generators::SuggestionGenerator;
pub use status::{Suggestion, SuggestionStatus};
pub use strategy::{
    default_ocr_strategy, default_typing_strategy, ocr_strategy, typing_strategy,
    SuggestionStrategy,
};
