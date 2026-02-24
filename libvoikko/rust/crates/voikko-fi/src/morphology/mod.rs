// Morphological analysis module
// Origin: morphology/

mod finnish;
mod tag_parser;
mod vfst;

pub use finnish::FinnishVfstAnalyzer;
pub use vfst::VfstAnalyzer;

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

/// Blanket implementation: a shared reference to an analyzer also
/// implements `Analyzer`. This allows code that holds `&T` (e.g.,
/// `VoikkoHandle` lending a reference to its owned analyzer) to use
/// it in generic contexts requiring `A: Analyzer`.
impl<T: Analyzer + ?Sized> Analyzer for &T {
    fn analyze(&self, word: &[char], word_len: usize) -> Vec<Analysis> {
        (**self).analyze(word, word_len)
    }
}
