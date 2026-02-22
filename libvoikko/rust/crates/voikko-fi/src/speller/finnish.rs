// Finnish-specific spell checker adjustments
// Origin: spellchecker/FinnishSpellerTweaksWrapper.cpp

use voikko_core::analysis::{ATTR_MALAGA_VAPAA_JALKIOSA, ATTR_STRUCTURE};
use voikko_core::character::{is_consonant, is_vowel, simple_lower};
use voikko_core::enums::SpellResult;

use crate::morphology::Analyzer;
use crate::speller::utils::match_word_and_analysis;
use crate::speller::Speller;

/// Options controlling Finnish spelling tweaks.
#[derive(Debug, Clone, Default)]
pub struct FinnishSpellerOptions {
    /// Accept extra hyphens in compound words.
    /// Origin: VoikkoHandle::accept_extra_hyphens
    pub accept_extra_hyphens: bool,
}

/// Wraps a base `Speller` with Finnish-specific adjustments.
///
/// Handles:
/// 1. Soft hyphen (U+00AD) validation
/// 2. Optional hyphen in compound words
/// 3. Vowel-consonant overlap patterns ("pop-opisto")
/// 4. Free suffix parts ("ja-sana")
/// 5. Ambiguous compound boundaries ("syy-silta" / "syys-ilta")
///
/// Note: Soft hyphen validation requires a hyphenator. Since the hyphenator
/// is not yet implemented (Phase 3-A), soft hyphen handling returns a basic
/// result by stripping soft hyphens and checking the stripped word. Full
/// hyphen position validation will be added when the hyphenator is available.
///
/// Origin: FinnishSpellerTweaksWrapper.cpp:42-223
pub struct FinnishSpellerTweaksWrapper<'a> {
    inner: &'a dyn Speller,
    analyzer: &'a dyn Analyzer,
    options: FinnishSpellerOptions,
}

impl<'a> FinnishSpellerTweaksWrapper<'a> {
    /// Create a new Finnish speller tweaks wrapper.
    ///
    /// Origin: FinnishSpellerTweaksWrapper.cpp:42-51
    pub fn new(
        inner: &'a dyn Speller,
        analyzer: &'a dyn Analyzer,
        options: FinnishSpellerOptions,
    ) -> Self {
        Self {
            inner,
            analyzer,
            options,
        }
    }

    /// Spell-check a word after soft hyphens have been stripped.
    ///
    /// This handles optional hyphens, VC overlap, free suffix, and ambiguous
    /// compound logic.
    ///
    /// Origin: FinnishSpellerTweaksWrapper.cpp:53-174
    fn spell_without_soft_hyphen(&self, word: &[char], wlen: usize) -> SpellResult {
        let result = self.inner.spell(word, wlen);

        // Look for a hyphen to process optional/compound hyphen logic
        let hyphen_pos = if result != SpellResult::Ok && wlen > 3 {
            word[1..wlen - 1].iter().position(|&c| c == '-').map(|p| p + 1)
        } else {
            None
        };

        let Some(hyphen_idx) = hyphen_pos else {
            return result;
        };

        // Build a buffer with the hyphen at hyphen_idx removed
        let leading_len = hyphen_idx;
        let mut buffer: Vec<char> = Vec::with_capacity(wlen - 1);
        buffer.extend_from_slice(&word[..leading_len]);
        buffer.extend_from_slice(&word[hyphen_idx + 1..wlen]);

        // --- Optional hyphens (accept_extra_hyphens) ---
        // Origin: FinnishSpellerTweaksWrapper.cpp:73-82
        if self.options.accept_extra_hyphens
            && leading_len > 1
            && buffer.get(leading_len).copied() != Some('-')
        {
            let spres = self.spell_without_soft_hyphen(&buffer, buffer.len());
            if spres == SpellResult::Ok {
                return spres;
            }
        }

        // --- Vowel-consonant overlap: "pop-opisto" pattern ---
        // Leading part ends with VC, trailing part starts with same VC pair.
        // Origin: FinnishSpellerTweaksWrapper.cpp:85-98
        if leading_len >= 2 && wlen - leading_len >= 3 {
            let vc1 = simple_lower(word[leading_len - 2]);
            let vc2 = simple_lower(word[leading_len - 1]);
            if is_vowel(vc1)
                && is_consonant(vc2)
                && simple_lower(word[leading_len + 1]) == vc1
                && simple_lower(word[leading_len + 2]) == vc2
            {
                let spres = self.inner.spell(&buffer, buffer.len());
                if spres != SpellResult::Failed
                    && (result == SpellResult::Failed || result > spres)
                {
                    return spres;
                }
            }
        }

        // --- Free suffix part: "ja-sana" pattern ---
        // If leading part (before last hyphen) is valid, and trailing part
        // has MALAGA_VAPAA_JALKIOSA=true.
        // Origin: FinnishSpellerTweaksWrapper.cpp:101-126
        for i in (1..wlen - 1).rev() {
            if word[i] == '-' {
                let leading_result = self.spell(&word[..i], i);
                if leading_result != SpellResult::Failed {
                    let trailing_word: Vec<char> =
                        word[i + 1..wlen].to_vec();
                    let trailing_analyses =
                        self.analyzer.analyze(&trailing_word, trailing_word.len());
                    let is_trailing_acceptable = trailing_analyses.iter().any(|a| {
                        a.get(ATTR_MALAGA_VAPAA_JALKIOSA)
                            .is_some_and(|v| v == "true")
                    });
                    if is_trailing_acceptable {
                        return leading_result;
                    }
                }
                break;
            }
        }

        // --- Ambiguous compound: "syy-silta" / "syys-ilta" ---
        // Remove the hyphen, analyze the result, and check if any analysis
        // has a compound boundary at the hyphen position.
        // Origin: FinnishSpellerTweaksWrapper.cpp:129-171
        let analyses = self.analyzer.analyze(&buffer, buffer.len());

        if analyses.is_empty() {
            return result;
        }

        let mut result_with_border = SpellResult::Failed;
        let mut result_without_border = SpellResult::Failed;

        for analysis in &analyses {
            let structure = match analysis.get(ATTR_STRUCTURE) {
                Some(s) => s,
                None => continue,
            };

            // Walk through the STRUCTURE to find where the hyphen position
            // falls, skipping '=' boundary markers.
            let structure_chars: Vec<char> = structure.chars().collect();
            let mut j = 0;
            let mut i = 0;
            while i < leading_len {
                while j < structure_chars.len() && structure_chars[j] == '=' {
                    j += 1;
                }
                if j >= structure_chars.len() {
                    break;
                }
                j += 1;
                i += 1;
            }

            if i == leading_len {
                let spres = match_word_and_analysis(&buffer, structure);
                if j < structure_chars.len() && structure_chars[j] == '=' {
                    if result_with_border == SpellResult::Failed
                        || result_with_border > spres
                    {
                        result_with_border = spres;
                    }
                } else if result_without_border == SpellResult::Failed
                    || result_without_border > spres
                {
                    result_without_border = spres;
                }
            }
        }

        // Accept only if both "with border" and "without border" analyses exist
        if result_with_border != SpellResult::Failed
            && result_without_border != SpellResult::Failed
            && (result == SpellResult::Failed || result > result_with_border)
        {
            return result_with_border;
        }

        result
    }
}

impl Speller for FinnishSpellerTweaksWrapper<'_> {
    /// Spell-check a word with Finnish-specific adjustments.
    ///
    /// Handles soft hyphens (U+00AD) by stripping them and validating
    /// the stripped word. Full soft-hyphen position validation requires
    /// the hyphenator (Phase 3-A).
    ///
    /// Origin: FinnishSpellerTweaksWrapper.cpp:176-216
    fn spell(&self, word: &[char], wlen: usize) -> SpellResult {
        let has_soft_hyphen = word[..wlen].contains(&'\u{00AD}');

        if has_soft_hyphen {
            // Strip soft hyphens and collect their positions
            let mut buffer = Vec::with_capacity(wlen);
            let mut shy_positions = Vec::new();
            for (i, &ch) in word[..wlen].iter().enumerate() {
                if ch != '\u{00AD}' {
                    buffer.push(ch);
                } else {
                    // Soft hyphen at start, end, or duplicate position -> fail
                    if buffer.is_empty()
                        || i + 1 == wlen
                        || (!shy_positions.is_empty()
                            && *shy_positions.last().unwrap() == buffer.len())
                    {
                        return SpellResult::Failed;
                    }
                    shy_positions.push(buffer.len());
                }
            }

            let result_wo_shy = self.spell_without_soft_hyphen(&buffer, buffer.len());

            if result_wo_shy != SpellResult::Failed {
                // TODO: When the hyphenator is implemented (Phase 3-A),
                // validate that all soft hyphen positions are at valid
                // hyphenation points. For now, we accept the word if the
                // stripped version is valid.
                //
                // Origin: FinnishSpellerTweaksWrapper.cpp:197-208
                // (hyphenator->allPossibleHyphenPositions check)
                let _ = shy_positions; // Will be used by hyphenator validation
            }

            result_wo_shy
        } else {
            self.spell_without_soft_hyphen(word, wlen)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voikko_core::analysis::Analysis;
    use crate::morphology::Analyzer;
    use crate::speller::adapter::AnalyzerToSpellerAdapter;

    /// A mock analyzer for testing Finnish speller tweaks.
    struct MockFinnishAnalyzer;

    impl MockFinnishAnalyzer {
        fn make_analysis(structure: &str) -> Analysis {
            let mut a = Analysis::new();
            a.set(ATTR_STRUCTURE, structure);
            a
        }

        fn make_analysis_with_free_suffix(structure: &str) -> Analysis {
            let mut a = Analysis::new();
            a.set(ATTR_STRUCTURE, structure);
            a.set(ATTR_MALAGA_VAPAA_JALKIOSA, "true");
            a
        }
    }

    impl Analyzer for MockFinnishAnalyzer {
        fn analyze(&self, word: &[char], _word_len: usize) -> Vec<Analysis> {
            let s: String = word.iter().collect();
            match s.as_str() {
                "koira" => vec![Self::make_analysis("=ppppp")],
                "kissa" => vec![Self::make_analysis("=ppppp")],
                "helsinki" => vec![Self::make_analysis("=ippppppp")],
                "ja" => vec![Self::make_analysis("=pp")],
                // "sana" has MALAGA_VAPAA_JALKIOSA=true (free suffix part)
                "sana" => vec![Self::make_analysis_with_free_suffix("=pppp")],
                // "popopisto" = compound of "pop" + "opisto"
                "popopisto" => vec![Self::make_analysis("=ppp=pppppp")],
                // "syysilta" (hyphen removed from "syy-silta") has two analyses:
                // with and without compound boundary
                "syysilta" => vec![
                    Self::make_analysis("=ppp=ppppp"), // syy + silta (boundary at 3)
                    Self::make_analysis("=pppppppp"),  // syysilta (no boundary)
                ],
                _ => vec![],
            }
        }
    }

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    fn make_wrapper<'a>(
        inner: &'a dyn Speller,
        analyzer: &'a dyn Analyzer,
        accept_extra_hyphens: bool,
    ) -> FinnishSpellerTweaksWrapper<'a> {
        FinnishSpellerTweaksWrapper::new(
            inner,
            analyzer,
            FinnishSpellerOptions {
                accept_extra_hyphens,
            },
        )
    }

    #[test]
    fn basic_word_passes_through() {
        let analyzer = MockFinnishAnalyzer;
        let adapter = AnalyzerToSpellerAdapter::new(&analyzer);
        let wrapper = make_wrapper(&adapter, &analyzer, false);

        let word = chars("koira");
        assert_eq!(wrapper.spell(&word, word.len()), SpellResult::Ok);
    }

    #[test]
    fn unknown_word_fails() {
        let analyzer = MockFinnishAnalyzer;
        let adapter = AnalyzerToSpellerAdapter::new(&analyzer);
        let wrapper = make_wrapper(&adapter, &analyzer, false);

        let word = chars("xyzzy");
        assert_eq!(wrapper.spell(&word, word.len()), SpellResult::Failed);
    }

    #[test]
    fn soft_hyphen_at_start_fails() {
        let analyzer = MockFinnishAnalyzer;
        let adapter = AnalyzerToSpellerAdapter::new(&analyzer);
        let wrapper = make_wrapper(&adapter, &analyzer, false);

        let word = chars("\u{00AD}koira");
        assert_eq!(wrapper.spell(&word, word.len()), SpellResult::Failed);
    }

    #[test]
    fn soft_hyphen_at_end_fails() {
        let analyzer = MockFinnishAnalyzer;
        let adapter = AnalyzerToSpellerAdapter::new(&analyzer);
        let wrapper = make_wrapper(&adapter, &analyzer, false);

        let word = chars("koira\u{00AD}");
        assert_eq!(wrapper.spell(&word, word.len()), SpellResult::Failed);
    }

    #[test]
    fn soft_hyphen_stripped_valid_word() {
        let analyzer = MockFinnishAnalyzer;
        let adapter = AnalyzerToSpellerAdapter::new(&analyzer);
        let wrapper = make_wrapper(&adapter, &analyzer, false);

        // "koi\u{00AD}ra" -> strips to "koira" which is valid
        let word = chars("koi\u{00AD}ra");
        assert_eq!(wrapper.spell(&word, word.len()), SpellResult::Ok);
    }

    #[test]
    fn consecutive_soft_hyphens_fail() {
        let analyzer = MockFinnishAnalyzer;
        let adapter = AnalyzerToSpellerAdapter::new(&analyzer);
        let wrapper = make_wrapper(&adapter, &analyzer, false);

        let word = chars("ko\u{00AD}\u{00AD}ira");
        assert_eq!(wrapper.spell(&word, word.len()), SpellResult::Failed);
    }

    #[test]
    fn vc_overlap_pattern() {
        // "pop-opisto" -> strip hyphen -> "popopisto" which is a known compound
        let analyzer = MockFinnishAnalyzer;
        let adapter = AnalyzerToSpellerAdapter::new(&analyzer);
        let wrapper = make_wrapper(&adapter, &analyzer, false);

        let word = chars("pop-opisto");
        assert_eq!(wrapper.spell(&word, word.len()), SpellResult::Ok);
    }

    #[test]
    fn free_suffix_ja_sana() {
        // "ja-sana": "ja" is a valid word, "sana" has MALAGA_VAPAA_JALKIOSA=true
        let analyzer = MockFinnishAnalyzer;
        let adapter = AnalyzerToSpellerAdapter::new(&analyzer);
        let wrapper = make_wrapper(&adapter, &analyzer, false);

        let word = chars("ja-sana");
        assert_eq!(wrapper.spell(&word, word.len()), SpellResult::Ok);
    }

    #[test]
    fn ambiguous_compound_with_hyphen() {
        // "syy-silta": remove hyphen -> "syyssilta" which has analyses
        // both with and without boundary at position 3
        let analyzer = MockFinnishAnalyzer;
        let adapter = AnalyzerToSpellerAdapter::new(&analyzer);
        let wrapper = make_wrapper(&adapter, &analyzer, false);

        let word = chars("syy-silta");
        assert_eq!(wrapper.spell(&word, word.len()), SpellResult::Ok);
    }

    #[test]
    fn optional_hyphen_with_flag() {
        // When accept_extra_hyphens is true, "koi-ra" should work
        // because stripping hyphen gives "koira" which is valid
        let analyzer = MockFinnishAnalyzer;
        let adapter = AnalyzerToSpellerAdapter::new(&analyzer);
        let wrapper = make_wrapper(&adapter, &analyzer, true);

        let word = chars("koi-ra");
        assert_eq!(wrapper.spell(&word, word.len()), SpellResult::Ok);
    }

    #[test]
    fn optional_hyphen_without_flag_fails() {
        // Without accept_extra_hyphens, "koi-ra" is not recognized
        // (not a VC overlap, not free suffix, not ambiguous compound)
        let analyzer = MockFinnishAnalyzer;
        let adapter = AnalyzerToSpellerAdapter::new(&analyzer);
        let wrapper = make_wrapper(&adapter, &analyzer, false);

        let word = chars("koi-ra");
        // "koira" analyzed without hyphen -> analyses exist but no compound boundary
        // at position 3, so ambiguous compound check needs both border and non-border.
        // Only non-border analysis exists -> check fails -> returns original result
        assert_eq!(wrapper.spell(&word, word.len()), SpellResult::Failed);
    }

    #[test]
    fn proper_noun_passes_through() {
        let analyzer = MockFinnishAnalyzer;
        let adapter = AnalyzerToSpellerAdapter::new(&analyzer);
        let wrapper = make_wrapper(&adapter, &analyzer, false);

        let word = chars("helsinki");
        assert_eq!(
            wrapper.spell(&word, word.len()),
            SpellResult::CapitalizeFirst
        );
    }
}
