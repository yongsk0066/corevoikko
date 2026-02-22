// Spell checking module
// Origin: spellchecker/

pub mod adapter;
pub mod cache;
pub mod finnish;
pub mod pipeline;
pub mod utils;

use voikko_core::enums::SpellResult;

/// Trait for spell checkers.
///
/// All implementations take a word as a `char` slice (for random-access
/// indexing into character positions) and return a `SpellResult`.
///
/// The word passed to `spell` is typically already lowercased; case
/// correctness is validated separately via the STRUCTURE attribute.
///
/// Origin: spellchecker/Speller.hpp:47-56
pub trait Speller {
    /// Check whether the given word is correct (or would be correct
    /// with different capitalization).
    ///
    /// - `word`: the word to check (char slice, not necessarily null-terminated)
    /// - `word_len`: the number of characters to consider
    fn spell(&self, word: &[char], word_len: usize) -> SpellResult;
}
