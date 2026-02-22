// Morphological analysis module
// Origin: morphology/

mod vfst;
mod finnish;
mod tag_parser;

pub use vfst::VfstAnalyzer;
pub use finnish::FinnishVfstAnalyzer;

use voikko_core::analysis::Analysis;

/// Trait for morphological analyzers.
///
/// Abstracts over different analyzer backends (VFST, HFST, etc.).
/// In practice, only VFST is used for Finnish.
///
/// Origin: morphology/Analyzer.hpp
pub trait Analyzer {
    /// Analyze a word and return all valid analyses.
    ///
    /// The word is provided as a char slice for random-access indexing
    /// (needed by FinnishVfstAnalyzer's STRUCTURE parsing).
    fn analyze(&self, word: &[char], word_len: usize) -> Vec<Analysis>;
}
