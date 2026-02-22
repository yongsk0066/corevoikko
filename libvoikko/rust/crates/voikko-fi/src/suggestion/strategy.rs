// Suggestion strategy orchestration: typing and OCR strategies
// Origin: spellchecker/suggestion/SuggestionStrategy.cpp,
//         SuggestionStrategyTyping.cpp, SuggestionStrategyOcr.cpp

use crate::speller::Speller;
use super::generators::*;
use super::status::SuggestionStatus;

// =========================================================================
// Replacement tables (Finnish keyboard adjacency)
// =========================================================================

// Origin: SuggestionStrategyTyping.cpp:48-101

/// Highest-frequency keyboard-neighbor replacements.
/// Origin: SuggestionStrategyTyping.cpp:48 (REPLACEMENTS_1)
const REPLACEMENTS_1: &[char] = &[
    '.', ',', 'a', 's', 'i', 'u', 'i', 'o', 't', 'r',
    't', 'd', 'e', 'r', 's', '\u{0161}', 's', 'a', 'n', 'm',
    'u', 'i', 'l', 'k', 'k', 'l', 'k', 'g', 'o', 'i',
    '\u{00E4}', '\u{00F6}', 'm', 'n', 'r', 'e', 'r', 't', 'v', 'b',
    'p', 'b', 'p', 'o', 'y', 't', 'h', 'j', 'j', 'h',
    'j', 'k', 'd', 't', 'd', 's', 'd', 'f', '\u{00F6}', '\u{00E4}',
    'g', 'f', 'g', 'h', 'g', 'k', 'f', 'g', 'f', 'd',
    'b', 'p', 'b', 'n', 'c', 'v', 'c', 's', 'w', 'e',
    'w', 'v', 'x', 'c', 'z', '\u{017E}', 'z', 'x', 'q', 'a',
    '\u{00E5}', 'o', '\u{00E5}', 'p', '\u{00E5}', '\u{00E4}', '\u{00E5}', '\u{00F6}', 'a', 'e',
    'i', 'k', 't', 'y', 'e', 'a',
];

/// Number-row replacements.
/// Origin: SuggestionStrategyTyping.cpp:61 (REPLACEMENTS_2)
const REPLACEMENTS_2: &[char] = &[
    '1', 'q', '2', 'q', '2', 'w', '3', 'w', '3', 'e',
    '4', 'e', '4', 'r', '5', 'r', '5', 't', '6', 't',
    '6', 'y', '7', 'y', '7', 'u', '8', 'u', '8', 'i',
    '9', 'i', '9', 'o', '0', 'o', '0', 'p', '+', 'p',
    'i', 'e',
];

/// Origin: SuggestionStrategyTyping.cpp:68 (REPLACEMENTS_3)
const REPLACEMENTS_3: &[char] = &[
    'e', 's', 's', 'd', 'n', 'h', 'u', 'j', 'l', '\u{00F6}',
    'k', 'j', 'o', 'p', '\u{00E4}', 'p', 'm', 'k', 'r', 'd',
    'v', 'g', 'p', 'l', 'y', 'h', 'h', 'u', 'j', 'i',
    'd', 'e', '\u{00F6}', 'l', 'g', 't', 'f', 'v', 'b', 'v',
    'c', 'k', 'w', 'a', 'x', 's', 'z', 'a', 'q', 'k',
    '\u{00E5}', 'a', 'a', '\u{00E5}', 'e', '\u{00E9}', 'a', '\u{00E2}', 'k', 'c',
    's', 'c', 'i', 'j', 'x', 'z',
];

/// Origin: SuggestionStrategyTyping.cpp:77 (REPLACEMENTS_4)
const REPLACEMENTS_4: &[char] = &[
    'q', 'w', 'q', 's', 'w', 'q', 'w', 's', 'w', 'd',
    'e', 'd', 'e', 'f', 'r', 'f', 'r', 'g', 't', 'f',
    't', 'g', 't', 'h', 'y', 'g', 'y', 'j', 'u', 'h',
    'u', 'k', 'i', 'l', 'o', 'k', 'o', 'l', 'p', '\u{00F6}',
    'p', '\u{00E4}', 's', 'e', 's', 'x', 'd', 'r', 'b', 'g',
    'f', 'e', 'f', 'r', 'f', 't', 'f', 'c', 'g', 'y',
    'g', 'b', 'g', 'v', 'h', 'y', 'h', 'n', 'h', 'b',
    'h', 'g', 'j', 'u', 'j', 'm', 'j', 'n', 'k', 'i',
    'k', 'o', 'k', 'm', 'l', 'o', 'l', 'p', '\u{00F6}', 'p',
    '\u{00F6}', '\u{00E5}', '\u{00E4}', '\u{00E5}', 'z', 's', 'x', 'd', 'c', 'd',
    'c', 'f', 'c', 'x', 'v', 'f', 'b', 'h', 'n', 'j',
    'n', 'b', 'm', 'j', 'e', 'w', 'p', '\u{00E5}', 'a', 'q',
    's', 'w', 's', 'z', 'd', 'w', 'd', 'c', 'd', 'x',
    'v', 'c', 'a', 'w', 'a', 'z', 's', 'q',
];

/// Origin: SuggestionStrategyTyping.cpp:93 (REPLACEMENTS_5)
const REPLACEMENTS_5: &[char] = &[
    'a', 'o', 'o', 'a', 'o', 'u', 't', 'l', 's', 'r',
    'a', 'i', 'e', '\u{00E4}', '\u{00E4}', 'e', 'u', 'v', 'v', 'u',
    'o', 'd', 'd', 'o', 'k', 'q', 'p', 'v', 'v', 'p',
    'q', 'e', 'e', 'q', 'a', 'd', 'd', 'a', 'r', 's',
    'e', 't', 't', 'e', 'r', 'y', 'y', 'r', 't', 'u',
    'u', 't', 'y', 'i', 'i', 'y', 'u', 'o', 'i', 'p',
    'p', 'i', 'o', '\u{00E5}', 'h', 'v', 'v', 'h', 'h', 'm',
    'm', 'h',
];

/// OCR replacement table.
/// Origin: SuggestionStrategyOcr.cpp:38 (REPLACEMENTS)
const OCR_REPLACEMENTS: &[char] = &[
    '0', 'o', 'l', 'i', 'i', 'l', 'u', 'o', 'o', 'u',
    'a', '\u{00E4}', '\u{00E4}', 'a', 'o', '\u{00F6}', '\u{00F6}', 'o', 's', '\u{0161}',
    '\u{0161}', 's', 'z', '\u{017E}', '\u{017E}', 'z', 'e', '\u{00E9}', '\u{00E9}', 'e',
    'a', '\u{00E2}', '\u{00E2}', 'a', 'p', 'b', 'b', 'p', 'e', 'f',
    'f', 'e', 'q', 'o', 'o', 'q', 'n', 'm', 'm', 'n',
    'u', 'v', 'v', 'u', 'o', 'c', 'c', 'o', 'b', 'h',
    'h', 'b', '_', 'a', '_', 'b', '_', 'c', '_', 'd',
    '_', 'e', '_', 'f', '_', 'g', '_', 'h', '_', 'i',
    '_', 'j', '_', 'k', '_', 'l', '_', 'm', '_', 'n',
    '_', 'o', '_', 'p', '_', 'q', '_', 'r', '_', 's',
    '_', 't', '_', 'u', '_', 'v', '_', 'w', '_', 'x',
    '_', 'y', '_', 'z', '_', '\u{00E4}', '_', '\u{00F6}',
];

/// Insertion characters ordered by frequency (first set: most common Finnish letters).
/// Origin: SuggestionStrategyTyping.cpp:123
const INSERTION_CHARS_PRIMARY: &str = "aitesn";

/// Insertion characters: remaining letters.
/// Origin: SuggestionStrategyTyping.cpp:130
const INSERTION_CHARS_SECONDARY: &str = "ulko\u{00E4}mrvpyhjd\u{00F6}gfbcw:xzq\u{00E5}'.";

// =========================================================================
// SuggestionStrategy
// =========================================================================

/// A suggestion strategy holds primary and secondary generator lists
/// and orchestrates them with a cost budget.
///
/// Origin: SuggestionStrategy.hpp, SuggestionStrategy.cpp
pub struct SuggestionStrategy {
    /// Maximum computational cost budget.
    max_cost: usize,
    /// Primary generators -- run first; if any produce suggestions, secondaries are skipped.
    primary_generators: Vec<Box<dyn SuggestionGenerator>>,
    /// Secondary generators -- run only if primaries produced nothing.
    generators: Vec<Box<dyn SuggestionGenerator>>,
}

impl SuggestionStrategy {
    /// Run the strategy: execute primary generators, then secondary if no
    /// suggestions were found by primaries.
    ///
    /// Origin: SuggestionStrategy.cpp:49-65
    pub fn generate(&self, speller: &dyn Speller, status: &mut SuggestionStatus<'_>) {
        status.set_max_cost(self.max_cost);

        for generator in &self.primary_generators {
            if status.should_abort() {
                break;
            }
            generator.generate(speller, status);
        }
        if status.suggestion_count() > 0 {
            // Primary generator found something; skip secondaries.
            return;
        }

        for generator in &self.generators {
            if status.should_abort() {
                break;
            }
            generator.generate(speller, status);
        }
    }
}

// =========================================================================
// Factory functions
// =========================================================================

/// Create the typing strategy for Finnish keyboard errors.
///
/// The generator order and replacement tables match the C++ `SuggestionStrategyTyping`
/// constructor exactly.
///
/// Origin: SuggestionStrategyTyping.cpp:103-143
pub fn typing_strategy(max_cost: usize) -> SuggestionStrategy {
    let primary_generators: Vec<Box<dyn SuggestionGenerator>> = vec![
        Box::new(CaseChange),
        Box::new(SoftHyphens),
    ];

    let generators: Vec<Box<dyn SuggestionGenerator>> = vec![
        Box::new(VowelChange),
        Box::new(Replacement { replacements: REPLACEMENTS_1.to_vec() }),
        Box::new(Deletion),
        Box::new(InsertSpecial),
        Box::new(SplitWord),
        Box::new(ReplaceTwo { replacements: REPLACEMENTS_1.to_vec() }),
        Box::new(Replacement { replacements: REPLACEMENTS_2.to_vec() }),
        Box::new(Insertion { characters: INSERTION_CHARS_PRIMARY.chars().collect() }),
        Box::new(Swap),
        Box::new(Replacement { replacements: REPLACEMENTS_3.to_vec() }),
        Box::new(Insertion { characters: INSERTION_CHARS_SECONDARY.chars().collect() }),
        Box::new(Replacement { replacements: REPLACEMENTS_4.to_vec() }),
        Box::new(ReplaceTwo { replacements: REPLACEMENTS_2.to_vec() }),
        Box::new(ReplaceTwo { replacements: REPLACEMENTS_3.to_vec() }),
        Box::new(ReplaceTwo { replacements: REPLACEMENTS_4.to_vec() }),
        Box::new(DeleteTwo),
        Box::new(Replacement { replacements: REPLACEMENTS_5.to_vec() }),
    ];

    SuggestionStrategy {
        max_cost,
        primary_generators,
        generators,
    }
}

/// Create the OCR strategy for optical character recognition errors.
///
/// Origin: SuggestionStrategyOcr.cpp:53-62
pub fn ocr_strategy(max_cost: usize) -> SuggestionStrategy {
    let primary_generators: Vec<Box<dyn SuggestionGenerator>> = vec![
        Box::new(CaseChange),
    ];

    let generators: Vec<Box<dyn SuggestionGenerator>> = vec![
        Box::new(Replacement { replacements: OCR_REPLACEMENTS.to_vec() }),
        Box::new(MultiReplacement {
            replacements: OCR_REPLACEMENTS.to_vec(),
            replace_count: 2,
        }),
    ];

    SuggestionStrategy {
        max_cost,
        primary_generators,
        generators,
    }
}

/// Default typing strategy with the standard C++ budget (800).
///
/// Origin: SuggestionGeneratorFactory.cpp:59
pub fn default_typing_strategy() -> SuggestionStrategy {
    typing_strategy(800)
}

/// Default OCR strategy with the standard C++ budget (2000).
///
/// Origin: SuggestionGeneratorFactory.cpp:56
pub fn default_ocr_strategy() -> SuggestionStrategy {
    ocr_strategy(2000)
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use voikko_core::enums::SpellResult;

    /// A mock speller that accepts a predefined set of words.
    struct MockSpeller {
        accepted: Vec<String>,
    }

    impl MockSpeller {
        fn new(words: &[&str]) -> Self {
            Self {
                accepted: words.iter().map(|s| s.to_string()).collect(),
            }
        }
    }

    impl Speller for MockSpeller {
        fn spell(&self, word: &[char], word_len: usize) -> SpellResult {
            let s: String = word[..word_len].iter().collect();
            if self.accepted.contains(&s) {
                SpellResult::Ok
            } else {
                SpellResult::Failed
            }
        }
    }

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn typing_strategy_primary_short_circuits() {
        // If CaseChange finds the word, secondaries should not run.
        let speller = MockSpeller::new(&["koira"]);
        let word = chars("koira");
        let mut status = SuggestionStatus::new(&word, 5);
        let strategy = default_typing_strategy();
        strategy.generate(&speller, &mut status);
        assert_eq!(status.suggestion_count(), 1);
        assert_eq!(status.suggestions()[0].word, "koira");
    }

    #[test]
    fn typing_strategy_deletion() {
        // "koiraa" -> delete 'a' -> "koira"
        let speller = MockSpeller::new(&["koira"]);
        let word = chars("koiraa");
        let mut status = SuggestionStatus::new(&word, 5);
        let strategy = default_typing_strategy();
        strategy.generate(&speller, &mut status);
        assert!(status.suggestion_count() >= 1);
        assert!(status.suggestions().iter().any(|s| s.word == "koira"));
    }

    #[test]
    fn typing_strategy_swap() {
        // "kiora" -> swap 'i' and 'o' -> "koira"
        let speller = MockSpeller::new(&["koira"]);
        let word = chars("kiora");
        let mut status = SuggestionStatus::new(&word, 5);
        let strategy = default_typing_strategy();
        strategy.generate(&speller, &mut status);
        assert!(status.suggestion_count() >= 1);
        assert!(status.suggestions().iter().any(|s| s.word == "koira"));
    }

    #[test]
    fn typing_strategy_split_word() {
        let speller = MockSpeller::new(&["koira", "kissa"]);
        let word = chars("koirakissa");
        let mut status = SuggestionStatus::new(&word, 5);
        let strategy = default_typing_strategy();
        strategy.generate(&speller, &mut status);
        assert!(status.suggestion_count() >= 1);
        assert!(status
            .suggestions()
            .iter()
            .any(|s| s.word == "koira kissa"));
    }

    #[test]
    fn ocr_strategy_replacement() {
        // OCR: '0' -> 'o'
        let speller = MockSpeller::new(&["koira"]);
        let word = chars("k0ira");
        let mut status = SuggestionStatus::new(&word, 5);
        let strategy = default_ocr_strategy();
        strategy.generate(&speller, &mut status);
        assert!(status.suggestion_count() >= 1);
        assert!(status.suggestions().iter().any(|s| s.word == "koira"));
    }

    #[test]
    fn strategy_respects_max_suggestions() {
        let speller = MockSpeller::new(&["a", "b", "c", "d", "e"]);
        let word = chars("x");
        let mut status = SuggestionStatus::new(&word, 2);
        let strategy = default_typing_strategy();
        strategy.generate(&speller, &mut status);
        assert!(status.suggestion_count() <= 2);
    }

    #[test]
    fn strategy_cost_budget_limits_work() {
        // With a very small budget, the strategy should abort quickly
        let speller = MockSpeller::new(&["koira"]);
        let word = chars("xyzzyxyzzy"); // long unknown word
        let mut status = SuggestionStatus::new(&word, 5);
        let strategy = typing_strategy(1); // very small budget
        strategy.generate(&speller, &mut status);
        // Should not run forever -- just verify it terminates
    }

    #[test]
    fn typing_strategy_has_correct_generator_counts() {
        let strategy = default_typing_strategy();
        assert_eq!(strategy.primary_generators.len(), 2);
        assert_eq!(strategy.generators.len(), 17);
    }

    #[test]
    fn ocr_strategy_has_correct_generator_counts() {
        let strategy = default_ocr_strategy();
        assert_eq!(strategy.primary_generators.len(), 1);
        assert_eq!(strategy.generators.len(), 2);
    }
}
