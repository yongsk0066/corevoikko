// Bridges the morphological Analyzer to the Speller interface
// Origin: spellchecker/AnalyzerToSpellerAdapter.cpp

use voikko_core::analysis::ATTR_STRUCTURE;
use voikko_core::enums::SpellResult;

use crate::morphology::Analyzer;
use crate::speller::Speller;
use crate::speller::utils::match_word_and_analysis;

/// Adapts an `Analyzer` to the `Speller` trait by performing morphological
/// analysis and then validating case via the STRUCTURE attribute.
///
/// This is the main production path for Finnish spell checking:
/// 1. Analyze the word to get all possible interpretations.
/// 2. For each analysis, compare the word's actual case against the STRUCTURE.
/// 3. Return the best (least severe) result.
///
/// Origin: AnalyzerToSpellerAdapter.cpp:38-65
pub struct AnalyzerToSpellerAdapter<'a> {
    analyzer: &'a dyn Analyzer,
}

impl<'a> AnalyzerToSpellerAdapter<'a> {
    /// Create a new adapter wrapping the given analyzer.
    pub fn new(analyzer: &'a dyn Analyzer) -> Self {
        Self { analyzer }
    }
}

impl Speller for AnalyzerToSpellerAdapter<'_> {
    /// Spell-check a word by running morphological analysis and checking STRUCTURE.
    ///
    /// The word is expected to be already lowercased (except for characters that
    /// are intentionally uppercase in COMPLEX case patterns).
    ///
    /// Origin: AnalyzerToSpellerAdapter.cpp:41-65
    fn spell(&self, word: &[char], word_len: usize) -> SpellResult {
        let analyses = self.analyzer.analyze(word, word_len);

        if analyses.is_empty() {
            return SpellResult::Failed;
        }

        let mut best = SpellResult::Failed;

        for analysis in &analyses {
            let structure = match analysis.get(ATTR_STRUCTURE) {
                Some(s) => s,
                None => continue,
            };

            let result = match_word_and_analysis(word, structure);

            if best == SpellResult::Failed || best > result {
                best = result;
            }

            if best == SpellResult::Ok {
                break;
            }
        }

        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voikko_core::analysis::Analysis;

    /// A mock analyzer that returns predefined analyses for known words.
    struct MockAnalyzer;

    impl MockAnalyzer {
        fn make_analysis(structure: &str) -> Analysis {
            let mut a = Analysis::new();
            a.set(ATTR_STRUCTURE, structure);
            a
        }
    }

    impl Analyzer for MockAnalyzer {
        fn analyze(&self, word: &[char], _word_len: usize) -> Vec<Analysis> {
            let s: String = word.iter().collect();
            match s.as_str() {
                // "koira" — all lowercase noun
                "koira" => vec![Self::make_analysis("=ppppp")],
                // "helsinki" — proper noun (first letter should be uppercase)
                "helsinki" => vec![Self::make_analysis("=ippppppp")],
                // "iso-britannia" — compound with hyphen, multiple analyses
                "iso-britannia" => vec![
                    Self::make_analysis("=ppp-=ippppppppp"),
                    Self::make_analysis("=ppp-=pppppppppp"),
                ],
                // "abc" — abbreviation context
                "abc" => vec![Self::make_analysis("=jjj")],
                _ => vec![],
            }
        }
    }

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn known_lowercase_word_is_ok() {
        let analyzer = MockAnalyzer;
        let adapter = AnalyzerToSpellerAdapter::new(&analyzer);
        let word = chars("koira");
        assert_eq!(adapter.spell(&word, word.len()), SpellResult::Ok);
    }

    #[test]
    fn unknown_word_returns_failed() {
        let analyzer = MockAnalyzer;
        let adapter = AnalyzerToSpellerAdapter::new(&analyzer);
        let word = chars("xyzzy");
        assert_eq!(adapter.spell(&word, word.len()), SpellResult::Failed);
    }

    #[test]
    fn proper_noun_lowercase_returns_cap_first() {
        let analyzer = MockAnalyzer;
        let adapter = AnalyzerToSpellerAdapter::new(&analyzer);
        let word = chars("helsinki");
        assert_eq!(
            adapter.spell(&word, word.len()),
            SpellResult::CapitalizeFirst
        );
    }

    #[test]
    fn multiple_analyses_picks_best_result() {
        // "iso-britannia" has two analyses: one expects 'i' (uppercase) for 'b',
        // one expects 'p' (lowercase). The 'p' version should yield Ok.
        let analyzer = MockAnalyzer;
        let adapter = AnalyzerToSpellerAdapter::new(&analyzer);
        let word = chars("iso-britannia");
        assert_eq!(adapter.spell(&word, word.len()), SpellResult::Ok);
    }

    #[test]
    fn abbreviation_all_lowercase_where_uppercase_expected() {
        // "abc" with structure "=jjj" expects all uppercase.
        // Position 0: lowercase where j expected -> CapitalizeFirst.
        // Position 1: lowercase where j expected -> CapitalizationError (breaks).
        // Result: CapitalizationError.
        let analyzer = MockAnalyzer;
        let adapter = AnalyzerToSpellerAdapter::new(&analyzer);
        let word = chars("abc");
        assert_eq!(
            adapter.spell(&word, word.len()),
            SpellResult::CapitalizationError
        );
    }

    #[test]
    fn empty_word_with_no_analyses_returns_failed() {
        let analyzer = MockAnalyzer;
        let adapter = AnalyzerToSpellerAdapter::new(&analyzer);
        assert_eq!(adapter.spell(&[], 0), SpellResult::Failed);
    }
}
