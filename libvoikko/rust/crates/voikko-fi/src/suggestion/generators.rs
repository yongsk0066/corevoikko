// Individual suggestion generators: each applies one class of edit operation
// to produce candidate words, then validates them via the speller.
//
// Origin: spellchecker/suggestion/SuggestionGenerator*.cpp

use voikko_core::analysis::ATTR_STRUCTURE;
use voikko_core::character::{is_upper, simple_lower, simple_upper};
use voikko_core::enums::SpellResult;

use crate::morphology::Analyzer;
use crate::speller::Speller;
use super::status::SuggestionStatus;

use crate::finnish::constants::{BACK_VOWELS, FRONT_VOWELS};

/// Soft hyphen character (U+00AD).
const SOFT_HYPHEN: char = '\u{00AD}';

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Trait for individual suggestion generators.
///
/// Each generator produces candidate words by applying one class of edit
/// operation to the misspelled word, then validates each candidate through
/// the speller.
///
/// Origin: spellchecker/suggestion/SuggestionGenerator.hpp
pub trait SuggestionGenerator {
    /// Generate suggestions for the word tracked by `status`, using `speller`
    /// to validate candidates.
    fn generate(&self, speller: &dyn Speller, status: &mut SuggestionStatus<'_>);
}

// ---------------------------------------------------------------------------
// Shared helper: suggest_for_buffer (CaseChange logic)
// ---------------------------------------------------------------------------

/// Check a candidate buffer against the speller and, if it passes, add
/// it to the suggestion status with appropriate case corrections.
///
/// This is the backward-compatible variant that does not use morphological
/// analysis. For `CapitalizationError` results, the word is added as-is
/// since no STRUCTURE data is available to determine correct case.
///
/// Origin: SuggestionGeneratorCaseChange.cpp:49-106
pub fn suggest_for_buffer(
    speller: &dyn Speller,
    status: &mut SuggestionStatus<'_>,
    buffer: &[char],
    buf_len: usize,
) {
    suggest_for_buffer_with_analyzer(speller, status, buffer, buf_len, None);
}

/// Check a candidate buffer against the speller and, if it passes, add
/// it to the suggestion status with appropriate case corrections.
///
/// When an `Analyzer` is provided and the spell result is
/// `CapitalizationError`, the analyzer is called to retrieve the
/// STRUCTURE attribute. The STRUCTURE is then used to correct the
/// capitalization of each letter:
/// - `i` / `j` in STRUCTURE => uppercase
/// - `p` / `q` in STRUCTURE => lowercase
/// - `=` markers are skipped (compound boundaries)
///
/// This matches the C++ `SuggestionGeneratorCaseChange::suggestForBuffer`
/// logic for the `SPELL_CAP_ERROR` case.
///
/// Origin: SuggestionGeneratorCaseChange.cpp:49-106
pub fn suggest_for_buffer_with_analyzer(
    speller: &dyn Speller,
    status: &mut SuggestionStatus<'_>,
    buffer: &[char],
    buf_len: usize,
    analyzer: Option<&dyn Analyzer>,
) {
    if status.should_abort() {
        return;
    }
    let word = &buffer[..buf_len];
    let result = speller.spell(word, buf_len);
    status.charge();
    match result {
        SpellResult::Failed => {}
        SpellResult::Ok => {
            let prio = compute_priority(analyzer, word, buf_len, result);
            let s: String = word.iter().collect();
            status.add_suggestion(s, prio);
        }
        SpellResult::CapitalizeFirst => {
            let prio = compute_priority(analyzer, word, buf_len, result);
            let mut corrected: Vec<char> = word.to_vec();
            corrected[0] = simple_upper(corrected[0]);
            let s: String = corrected.iter().collect();
            status.add_suggestion(s, prio);
        }
        SpellResult::CapitalizationError => {
            // Use morphological analysis to determine correct capitalization
            // from the STRUCTURE attribute when an analyzer is available.
            //
            // Origin: SuggestionGeneratorCaseChange.cpp:75-104
            if let Some(analyzer) = analyzer {
                let analyses = analyzer.analyze(word, buf_len);
                status.charge();
                if analyses.is_empty() {
                    return;
                }
                let prio = best_priority_from_analyses(&analyses, result);
                // Use the STRUCTURE from the first analysis.
                if let Some(structure) = analyses[0].get(ATTR_STRUCTURE) {
                    let corrected = apply_structure_case(word, structure);
                    let s: String = corrected.iter().collect();
                    status.add_suggestion(s, prio);
                } else {
                    // No STRUCTURE attribute; add word as-is.
                    let s: String = word.iter().collect();
                    status.add_suggestion(s, prio);
                }
            } else {
                // No analyzer available; add the word unchanged.
                let s: String = word.iter().collect();
                status.add_suggestion(s, priority_from_result(result));
            }
        }
    }
}

/// Compute priority, using rich analysis-based priority when an analyzer
/// is available, or falling back to simple spell-result-based priority.
fn compute_priority(
    analyzer: Option<&dyn Analyzer>,
    word: &[char],
    word_len: usize,
    result: SpellResult,
) -> i32 {
    if let Some(analyzer) = analyzer {
        let analyses = analyzer.analyze(word, word_len);
        if !analyses.is_empty() {
            return best_priority_from_analyses(&analyses, result);
        }
    }
    priority_from_result(result)
}

/// Apply case corrections to a word based on its STRUCTURE attribute.
///
/// The STRUCTURE attribute encodes the expected case for each letter:
/// - `i` / `j` => the corresponding letter should be uppercase
/// - `p` / `q` => the corresponding letter should be lowercase
/// - `=` => compound boundary marker (skipped; does not consume a word char)
///
/// Origin: SuggestionGeneratorCaseChange.cpp:86-101
fn apply_structure_case(word: &[char], structure: &str) -> Vec<char> {
    let mut result: Vec<char> = word.to_vec();
    let struct_chars: Vec<char> = structure.chars().collect();
    let mut j = 0;

    for ch in &mut result {
        // Skip compound boundary markers.
        while j < struct_chars.len() && struct_chars[j] == '=' {
            j += 1;
        }
        if j >= struct_chars.len() {
            break;
        }
        match struct_chars[j] {
            'i' | 'j' => {
                *ch = simple_upper(*ch);
            }
            'p' | 'q' => {
                *ch = simple_lower(*ch);
            }
            _ => {}
        }
        j += 1;
    }

    result
}

/// Map a `SpellResult` to a base priority value.
///
/// Lower values are better. These roughly mirror the C++ behavior in
/// `SpellWithPriority::spellWithPriority` for the simplest case (single
/// word part, no inflection priority).
///
/// Origin: SpellWithPriority.cpp:132-144
fn priority_from_result(result: SpellResult) -> i32 {
    match result {
        SpellResult::Ok => 1,
        SpellResult::CapitalizeFirst => 2,
        SpellResult::CapitalizationError => 3,
        SpellResult::Failed => i32::MAX,
    }
}

// =========================================================================
// Rich priority calculation (with morphological analysis)
// =========================================================================

/// Compute a priority value from a noun's inflection form (SIJAMUOTO).
///
/// Nominative and genitive forms get the best priority because they are
/// the most commonly used forms. Rarer cases get progressively higher
/// (worse) priority values.
///
/// Origin: SpellWithPriority.cpp:38-90 (getPriorityFromNounInflection)
fn priority_from_noun_inflection(sijamuoto: Option<&str>) -> i32 {
    match sijamuoto {
        None => 4,
        Some("nimento") => 2,          // nominative
        Some("omanto") => 3,            // genitive
        Some("osanto") => 5,            // partitive
        Some("sisaolento") => 8,        // inessive
        Some("sisaeronto") => 12,       // elative
        Some("sisatulento") => 8,       // illative
        Some("ulkoolento") => 12,       // adessive
        Some("ulkoeronto") => 30,       // ablative
        Some("ulkotulento") => 20,      // allative
        Some("olento") => 20,           // essive
        Some("tulento") => 20,          // translative
        Some("vajanto") => 60,          // abessive
        Some("seuranto") => 60,         // comitative
        Some("keinonto") => 20,         // instructive
        Some(_) => 4,
    }
}

/// Compute a priority value from a word's CLASS and inflection.
///
/// Nouns, adjectives, pronouns, and proper names use inflection-based
/// priority. Other word classes get a default priority of 4.
///
/// Origin: SpellWithPriority.cpp:92-112 (getPriorityFromWordClassAndInflection)
fn priority_from_word_class_and_inflection(
    word_class: Option<&str>,
    sijamuoto: Option<&str>,
) -> i32 {
    match word_class {
        Some("nimisana")
        | Some("laatusana")
        | Some("nimisana_laatusana")
        | Some("asemosana")
        | Some("etunimi")
        | Some("sukunimi")
        | Some("paikannimi")
        | Some("nimi") => priority_from_noun_inflection(sijamuoto),
        _ => 4,
    }
}

/// Compute a priority penalty based on the number of compound word parts
/// in the STRUCTURE attribute.
///
/// Non-compound words (1 part) get priority 1 (best). Each additional
/// compound part multiplies the priority by 8 (i.e., `1 << (3 * (parts - 1))`).
///
/// Origin: SpellWithPriority.cpp:114-130 (getPriorityFromStructure)
fn priority_from_structure(structure: &str) -> i32 {
    let count_parts = structure
        .chars()
        .filter(|&c| c == '=')
        .take(5)
        .count();
    if count_parts == 0 {
        return 1; // won't happen with a valid dictionary
    }
    1 << (3 * (count_parts - 1))
}

/// Compute a rich priority for a single morphological analysis.
///
/// Combines word class/inflection priority, compound structure penalty,
/// and spell result priority: `class_prio * structure_prio * result_prio`.
///
/// Origin: SpellWithPriority.cpp:146-154 (handleAnalysis)
pub(crate) fn priority_from_analysis(
    analysis: &voikko_core::analysis::Analysis,
    result: SpellResult,
) -> i32 {
    let word_class = analysis.get(voikko_core::analysis::ATTR_CLASS);
    let sijamuoto = analysis.get(voikko_core::analysis::ATTR_SIJAMUOTO);
    let structure = analysis
        .get(ATTR_STRUCTURE)
        .unwrap_or("=p");

    let class_prio = priority_from_word_class_and_inflection(word_class, sijamuoto);
    let struct_prio = priority_from_structure(structure);
    let result_prio = priority_from_result(result);

    class_prio * struct_prio * result_prio
}

/// Compute the best priority across all analyses of a word.
///
/// Iterates through all analyses, picking the best (lowest) priority
/// for the best spell result. This matches the C++ `spellWithPriority`
/// behavior.
///
/// Origin: SpellWithPriority.cpp:156-187
pub(crate) fn best_priority_from_analyses(
    analyses: &[voikko_core::analysis::Analysis],
    result: SpellResult,
) -> i32 {
    if analyses.is_empty() {
        return priority_from_result(result);
    }
    analyses
        .iter()
        .map(|a| priority_from_analysis(a, result))
        .min()
        .unwrap_or(priority_from_result(result))
}

// =========================================================================
// Individual generators
// =========================================================================

// ---------------------------------------------------------------------------
// CaseChange
// ---------------------------------------------------------------------------

/// Try the word as-is to see if it only needs a case correction.
///
/// This is a primary generator -- it is cheap (1 spell check) and catches
/// the common case where the user typed the right word with wrong caps.
///
/// Origin: SuggestionGeneratorCaseChange.cpp
pub struct CaseChange;

impl SuggestionGenerator for CaseChange {
    /// Origin: SuggestionGeneratorCaseChange.cpp:45-47
    fn generate(&self, speller: &dyn Speller, status: &mut SuggestionStatus<'_>) {
        let word = status.word().to_vec();
        let len = status.word_len();
        suggest_for_buffer(speller, status, &word, len);
    }
}

// ---------------------------------------------------------------------------
// SoftHyphens
// ---------------------------------------------------------------------------

/// Try removing all soft hyphens (U+00AD) from the word.
///
/// This is a primary generator -- it handles words that were pasted from
/// hyphenated text.
///
/// Origin: SuggestionGeneratorSoftHyphens.cpp
pub struct SoftHyphens;

impl SuggestionGenerator for SoftHyphens {
    /// Origin: SuggestionGeneratorSoftHyphens.cpp:41-57
    fn generate(&self, speller: &dyn Speller, status: &mut SuggestionStatus<'_>) {
        let word = status.word().to_vec();
        if !word.contains(&SOFT_HYPHEN) {
            return;
        }
        let buffer: Vec<char> = word.iter().copied().filter(|&c| c != SOFT_HYPHEN).collect();
        let len = buffer.len();
        suggest_for_buffer(speller, status, &buffer, len);
    }
}

// ---------------------------------------------------------------------------
// Deletion
// ---------------------------------------------------------------------------

/// Try deleting one character at each position.
///
/// Skips positions where the deleted character is the same as its predecessor
/// (case-insensitive), since that would produce the same candidate as a
/// previous iteration.
///
/// Origin: SuggestionGeneratorDeletion.cpp
pub struct Deletion;

impl SuggestionGenerator for Deletion {
    /// Origin: SuggestionGeneratorDeletion.cpp:41-52
    fn generate(&self, speller: &dyn Speller, status: &mut SuggestionStatus<'_>) {
        let word = status.word().to_vec();
        let wlen = status.word_len();
        if wlen < 2 {
            return;
        }
        let new_len = wlen - 1;
        let mut buffer = Vec::with_capacity(new_len);

        for i in 0..wlen {
            if status.should_abort() {
                break;
            }
            // Skip if same as predecessor (case-insensitive)
            if i > 0 && simple_lower(word[i]) == simple_lower(word[i - 1]) {
                continue;
            }
            buffer.clear();
            buffer.extend_from_slice(&word[..i]);
            buffer.extend_from_slice(&word[i + 1..]);
            suggest_for_buffer(speller, status, &buffer, new_len);
        }
    }
}

// ---------------------------------------------------------------------------
// Insertion
// ---------------------------------------------------------------------------

/// Try inserting each character from a set at every position in the word.
///
/// The character set is ordered by frequency for the target language
/// (Finnish), so more common insertions are tested first.
///
/// Origin: SuggestionGeneratorInsertion.cpp
pub struct Insertion {
    /// Characters to try inserting, ordered by frequency.
    pub characters: Vec<char>,
}

impl SuggestionGenerator for Insertion {
    /// Origin: SuggestionGeneratorInsertion.cpp:43-75
    fn generate(&self, speller: &dyn Speller, status: &mut SuggestionStatus<'_>) {
        let word = status.word().to_vec();
        let wlen = status.word_len();
        if wlen == 0 {
            return;
        }
        let new_len = wlen + 1;
        let mut buffer = vec!['\0'; new_len];

        for &ins in &self.characters {
            // Insert at positions 0..wlen-1 (before each character)
            // Initialize: buffer = word[0] + word[0..wlen]
            buffer[0] = word[0];
            buffer[1..=wlen].copy_from_slice(&word);

            for j in 0..wlen {
                if status.should_abort() {
                    break;
                }
                if j != 0 {
                    buffer[j - 1] = word[j - 1];
                }
                // Avoid duplicates: skip if inserted char equals the char at position j
                if ins == simple_lower(word[j]) {
                    continue;
                }
                // Avoid duplicates: skip if inserted char equals the char before position j
                if j > 0 && ins == simple_lower(word[j - 1]) {
                    continue;
                }
                buffer[j] = ins;
                suggest_for_buffer(speller, status, &buffer, new_len);
            }
            if status.should_abort() {
                break;
            }
            // Insert at end: skip if same as last character
            if ins == word[wlen - 1] {
                continue;
            }
            buffer[wlen - 1] = word[wlen - 1];
            buffer[wlen] = ins;
            suggest_for_buffer(speller, status, &buffer, new_len);
        }
    }
}

// ---------------------------------------------------------------------------
// InsertSpecial
// ---------------------------------------------------------------------------

/// Try inserting hyphens and duplicating characters.
///
/// Two strategies:
/// 1. Insert '-' at positions 2..len-2 (avoiding adjacent hyphens).
/// 2. Duplicate each character at its position (avoiding already-doubled
///    characters and hyphens/apostrophes).
///
/// Origin: SuggestionGeneratorInsertSpecial.cpp
pub struct InsertSpecial;

impl SuggestionGenerator for InsertSpecial {
    /// Origin: SuggestionGeneratorInsertSpecial.cpp:38-69
    fn generate(&self, speller: &dyn Speller, status: &mut SuggestionStatus<'_>) {
        let word = status.word().to_vec();
        let wlen = status.word_len();
        if wlen < 4 {
            return;
        }
        let new_len = wlen + 1;
        let mut buffer = vec!['\0'; new_len];

        // Strategy 1: suggest adding '-'
        for j in 2..=wlen.saturating_sub(2) {
            if status.should_abort() {
                break;
            }
            // Do not add hyphen if there is another hyphen nearby
            if (j >= 2 && word[j - 2] == '-')
                || word[j - 1] == '-'
                || word[j] == '-'
                || (j + 1 < wlen && word[j + 1] == '-')
            {
                continue;
            }
            buffer[..j].copy_from_slice(&word[..j]);
            buffer[j] = '-';
            buffer[j + 1..new_len].copy_from_slice(&word[j..]);
            suggest_for_buffer(speller, status, &buffer, new_len);
        }

        // Strategy 2: suggest character duplication
        // Build buffer as: word[0] word[0] word[1] word[2] ... word[wlen-1]
        buffer[0] = word[0];
        buffer[1..=wlen].copy_from_slice(&word);

        let mut j = 0;
        while j < wlen {
            if status.should_abort() {
                break;
            }
            buffer[j] = word[j];
            // Do not duplicate if there already are two same letters
            if j + 1 < wlen && word[j] == word[j + 1] {
                j += 2;
                if j < wlen {
                    buffer[j] = word[j];
                }
                continue;
            }
            // These should not be duplicated
            if word[j] == '-' || word[j] == '\'' {
                j += 1;
                continue;
            }
            suggest_for_buffer(speller, status, &buffer, new_len);
            j += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// Replacement
// ---------------------------------------------------------------------------

/// Try replacing characters according to a replacement table.
///
/// The replacement table is a flat list of char pairs: `[from1, to1, from2, to2, ...]`.
/// For each pair, every occurrence of `from` in the word is replaced with `to`
/// and the result is checked. Uppercase variants are handled automatically.
///
/// Origin: SuggestionGeneratorReplacement.cpp
pub struct Replacement {
    /// Flat replacement pairs: `[from1, to1, from2, to2, ...]`.
    pub replacements: Vec<char>,
}

impl SuggestionGenerator for Replacement {
    /// Origin: SuggestionGeneratorReplacement.cpp:42-74
    fn generate(&self, speller: &dyn Speller, status: &mut SuggestionStatus<'_>) {
        let word = status.word().to_vec();
        let wlen = status.word_len();
        if self.replacements.len() < 2 {
            return;
        }
        let mut buffer: Vec<char> = word.to_vec();

        let mut i = 0;
        while i + 1 < self.replacements.len() {
            let from = self.replacements[i];
            let to = self.replacements[i + 1];
            i += 2;

            // Lowercase replacements
            for pos in 0..wlen {
                if buffer[pos] != from {
                    continue;
                }
                buffer[pos] = to;
                suggest_for_buffer(speller, status, &buffer, wlen);
                if status.should_abort() {
                    return;
                }
                buffer[pos] = from;
            }

            // Uppercase replacements (only if upper differs from lower)
            let upper_from = simple_upper(from);
            if upper_from == from {
                continue;
            }
            for pos in 0..wlen {
                if buffer[pos] != upper_from {
                    continue;
                }
                buffer[pos] = simple_upper(to);
                suggest_for_buffer(speller, status, &buffer, wlen);
                if status.should_abort() {
                    return;
                }
                buffer[pos] = upper_from;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ReplaceTwo
// ---------------------------------------------------------------------------

/// Try replacing doubled characters according to a replacement table.
///
/// Finds positions where two identical adjacent characters occur (e.g., `ss`)
/// and replaces both with the mapped character (e.g., `dd`). The word is
/// lowercased before matching.
///
/// Origin: SuggestionGeneratorReplaceTwo.cpp
pub struct ReplaceTwo {
    /// Flat replacement pairs: `[from1, to1, from2, to2, ...]`.
    pub replacements: Vec<char>,
}

impl SuggestionGenerator for ReplaceTwo {
    /// Origin: SuggestionGeneratorReplaceTwo.cpp:42-76
    fn generate(&self, speller: &dyn Speller, status: &mut SuggestionStatus<'_>) {
        let word = status.word().to_vec();
        let wlen = status.word_len();
        if wlen < 2 || self.replacements.len() < 2 {
            return;
        }
        let mut buffer: Vec<char> = word.iter().map(|&c| simple_lower(c)).collect();

        let mut i = 0;
        while i < wlen - 1 {
            let replaced = buffer[i];
            if replaced != buffer[i + 1] {
                i += 1;
                continue;
            }
            let mut j = 0;
            while j + 1 < self.replacements.len() {
                let from = self.replacements[j];
                let to = self.replacements[j + 1];
                j += 2;
                if from != replaced {
                    continue;
                }
                buffer[i] = to;
                buffer[i + 1] = to;
                suggest_for_buffer(speller, status, &buffer, wlen);
                if status.should_abort() {
                    return;
                }
            }
            buffer[i] = replaced;
            buffer[i + 1] = replaced;
            if status.should_abort() {
                return;
            }
            // Skip ahead past the pair (matching C++ `++i` in the inner increment)
            i += 2;
        }
    }
}

// ---------------------------------------------------------------------------
// MultiReplacement (used by OCR strategy)
// ---------------------------------------------------------------------------

/// Recursively apply up to N replacements from a table.
///
/// Used by the OCR strategy where multiple characters may be misrecognized
/// simultaneously.
///
/// Origin: SuggestionGeneratorMultiReplacement.cpp
pub struct MultiReplacement {
    /// Flat replacement pairs: `[from1, to1, from2, to2, ...]`.
    pub replacements: Vec<char>,
    /// Maximum number of simultaneous replacements.
    pub replace_count: usize,
}

impl MultiReplacement {
    /// Recursive replacement engine.
    ///
    /// Origin: SuggestionGeneratorMultiReplacement.cpp:50-70
    fn do_generate(
        &self,
        speller: &dyn Speller,
        status: &mut SuggestionStatus<'_>,
        buffer: &mut [char],
        start: usize,
        remaining: usize,
    ) {
        let wlen = status.word_len();
        let mut i = 0;
        while i + 1 < self.replacements.len() {
            let from = self.replacements[i];
            let to = self.replacements[i + 1];
            i += 2;

            for pos in start..wlen {
                if buffer[pos] != from {
                    continue;
                }
                buffer[pos] = to;
                if remaining == 1 {
                    suggest_for_buffer(speller, status, buffer, wlen);
                } else {
                    self.do_generate(speller, status, buffer, pos, remaining - 1);
                }
                if status.should_abort() {
                    return;
                }
                buffer[pos] = from;
            }
        }
    }
}

impl SuggestionGenerator for MultiReplacement {
    /// Origin: SuggestionGeneratorMultiReplacement.cpp:42-48
    fn generate(&self, speller: &dyn Speller, status: &mut SuggestionStatus<'_>) {
        let word = status.word().to_vec();
        let mut buffer: Vec<char> = word.to_vec();
        self.do_generate(speller, status, &mut buffer, 0, self.replace_count);
    }
}

// ---------------------------------------------------------------------------
// Swap
// ---------------------------------------------------------------------------

/// Try swapping pairs of characters within a distance limit.
///
/// The maximum swap distance depends on word length:
/// - Words <= 8 chars: max distance 10 (effectively all pairs)
/// - Longer words: `50 / word_len`
///
/// Skips swaps of identical characters and front/back vowel swaps
/// (already handled by VowelChange).
///
/// Origin: SuggestionGeneratorSwap.cpp
pub struct Swap;

impl SuggestionGenerator for Swap {
    /// Origin: SuggestionGeneratorSwap.cpp:44-77
    fn generate(&self, speller: &dyn Speller, status: &mut SuggestionStatus<'_>) {
        let word = status.word().to_vec();
        let wlen = status.word_len();
        if wlen < 2 {
            return;
        }
        let max_distance = if wlen <= 8 { 10 } else { 50 / wlen };
        if max_distance == 0 {
            return;
        }
        let mut buffer: Vec<char> = word.to_vec();

        for i in 0..wlen {
            if status.should_abort() {
                break;
            }
            for j in (i + 1)..wlen {
                if status.should_abort() {
                    break;
                }
                if j - i > max_distance {
                    break;
                }
                // Do not suggest the same word
                if simple_lower(buffer[i]) == simple_lower(buffer[j]) {
                    continue;
                }
                // Do not suggest swapping front and back vowels
                // (already tested by VowelChange)
                let skip = (0..3).any(|k| {
                    (simple_lower(buffer[i]) == BACK_VOWELS[k]
                        && simple_lower(buffer[j]) == FRONT_VOWELS[k])
                        || (simple_lower(buffer[i]) == FRONT_VOWELS[k]
                            && simple_lower(buffer[j]) == BACK_VOWELS[k])
                });
                if skip {
                    continue;
                }
                buffer[i] = word[j];
                buffer[j] = word[i];
                suggest_for_buffer(speller, status, &buffer, wlen);
                buffer[i] = word[i];
                buffer[j] = word[j];
            }
        }
    }
}

// ---------------------------------------------------------------------------
// SplitWord
// ---------------------------------------------------------------------------

/// Try splitting the word into two words separated by a space.
///
/// Both parts must pass spell check. Handles dots between parts
/// and hyphen-separated words (e.g., "suuntaa-antava" -> "suuntaa antava").
///
/// Origin: SuggestionGeneratorSplitWord.cpp
pub struct SplitWord;

impl SplitWord {
    /// Check if a word is correctly spelled, capitalizing the first letter
    /// if needed (matching the C++ `spellOk` helper).
    ///
    /// Returns `(is_ok, priority)`.
    ///
    /// Origin: SuggestionGeneratorSplitWord.cpp:45-56
    fn spell_ok(
        speller: &dyn Speller,
        status: &mut SuggestionStatus<'_>,
        word: &mut [char],
    ) -> (bool, i32) {
        let len = word.len();
        let first_upper = is_upper(word[0]);
        if first_upper {
            word[0] = simple_lower(word[0]);
        }
        let result = speller.spell(word, len);
        status.charge();
        if first_upper || result == SpellResult::CapitalizeFirst {
            word[0] = simple_upper(word[0]);
        }
        let ok = result == SpellResult::Ok || result == SpellResult::CapitalizeFirst;
        (ok, priority_from_result(result))
    }
}

impl SuggestionGenerator for SplitWord {
    /// Origin: SuggestionGeneratorSplitWord.cpp:68-103
    fn generate(&self, speller: &dyn Speller, status: &mut SuggestionStatus<'_>) {
        let word = status.word().to_vec();
        let wlen = status.word_len();
        if wlen < 4 {
            return;
        }

        let mut part1: Vec<char> = word.to_vec();

        // Iterate split positions from right to left (matching C++ `splitind >= 2`)
        let start = if wlen >= 2 { wlen - 2 } else { return };
        for splitind in (2..=start).rev() {
            // Do not split near hyphens
            if (splitind >= 2 && word[splitind - 2] == '-')
                || word[splitind - 1] == '-'
                || (splitind + 1 < wlen && word[splitind + 1] == '-')
            {
                continue;
            }
            // "suuntaa-antava" -> "suuntaa antava"
            let strip_lead2 = if word[splitind] == '-' { 1usize } else { 0 };

            // Check part 1
            part1.truncate(splitind);
            part1.extend_from_slice(&word[splitind..splitind]); // no-op, just for clarity
            let (mut ok1, mut prio_total) =
                SplitWord::spell_ok(speller, status, &mut part1[..splitind]);

            // If part1 fails and ends with '.', try without the dot
            if !ok1 && splitind > 0 && part1[splitind - 1] == '.' {
                let (ok_nodot, prio_nodot) =
                    SplitWord::spell_ok(speller, status, &mut part1[..splitind - 1]);
                if ok_nodot {
                    ok1 = true;
                    prio_total = prio_nodot;
                }
            }

            if ok1 {
                // Build part 2
                let w2start = splitind + strip_lead2;
                let w2len = wlen - w2start;
                if w2len == 0 {
                    // Restore part1 for next iteration
                    part1.clear();
                    part1.extend_from_slice(&word);
                    continue;
                }
                let mut part2: Vec<char> = word[w2start..w2start + w2len].to_vec();
                let (ok2, prio_part) = SplitWord::spell_ok(speller, status, &mut part2);
                let combined_prio = (prio_total + prio_part) * (1 + strip_lead2 as i32 * 5);

                if ok2 {
                    // Build "part1 part2" suggestion
                    let mut suggestion: Vec<char> = part1[..splitind].to_vec();
                    suggestion.push(' ');
                    suggestion.extend_from_slice(&part2);
                    let s: String = suggestion.iter().collect();
                    status.add_suggestion(s, combined_prio);
                }
            }

            if status.should_abort() {
                break;
            }

            // Restore part1 for next iteration
            part1.clear();
            part1.extend_from_slice(&word);
        }
    }
}

// ---------------------------------------------------------------------------
// VowelChange
// ---------------------------------------------------------------------------

/// Try all combinations of swapping front/back vowels (Finnish vowel harmony).
///
/// Finnish has vowel harmony: back vowels (a, o, u) correspond to front
/// vowels (ae, oe, y). This generator enumerates all 2^n - 1 combinations
/// of flipping vowels (up to 7 vowels in the word).
///
/// Origin: SuggestionGeneratorVowelChange.cpp
pub struct VowelChange;

impl VowelChange {
    /// Check if a character is a back or front vowel (case-insensitive),
    /// returning its index in the vowel arrays (0-5) or None.
    fn vowel_index(c: char) -> Option<usize> {
        BACK_VOWELS
            .iter()
            .position(|&v| v == c)
            .or_else(|| FRONT_VOWELS.iter().position(|&v| v == c))
    }
}

impl SuggestionGenerator for VowelChange {
    /// Origin: SuggestionGeneratorVowelChange.cpp:41-85
    fn generate(&self, speller: &dyn Speller, status: &mut SuggestionStatus<'_>) {
        let word = status.word().to_vec();
        let wlen = status.word_len();

        // Count vowels and build a mask
        let mut vcount: usize = 0;
        let mut mask: u32 = 0;
        for &c in &word {
            if VowelChange::vowel_index(c).is_some() {
                vcount += 1;
                mask = (mask << 1) | 1;
            }
        }
        if vcount == 0 || vcount > 7 {
            return;
        }

        let mut buffer: Vec<char> = word.to_vec();
        let mut pat: u32 = 1;

        while (pat & mask) != 0 {
            // Reset buffer to original word
            buffer.copy_from_slice(&word);

            let mut vowel_idx = 0;
            for i in 0..wlen {
                if VowelChange::vowel_index(word[i]).is_some() {
                    if (pat & (1 << vowel_idx)) != 0 {
                        // Flip the vowel
                        if let Some(pos) = BACK_VOWELS.iter().position(|&v| v == buffer[i]) {
                            buffer[i] = FRONT_VOWELS[pos];
                        } else if let Some(pos) =
                            FRONT_VOWELS.iter().position(|&v| v == buffer[i])
                        {
                            buffer[i] = BACK_VOWELS[pos];
                        }
                    }
                    vowel_idx += 1;
                }
            }

            if status.should_abort() {
                return;
            }
            suggest_for_buffer(speller, status, &buffer, wlen);
            pat += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// DeleteTwo
// ---------------------------------------------------------------------------

/// Try deleting two adjacent characters where a 2-char sequence repeats.
///
/// For words >= 6 characters, finds positions where `word[i..i+2] == word[i+2..i+4]`
/// and removes the duplicate pair.
///
/// Origin: SuggestionGeneratorDeleteTwo.cpp
pub struct DeleteTwo;

impl SuggestionGenerator for DeleteTwo {
    /// Origin: SuggestionGeneratorDeleteTwo.cpp:43-62
    fn generate(&self, speller: &dyn Speller, status: &mut SuggestionStatus<'_>) {
        let word = status.word().to_vec();
        let wlen = status.word_len();
        if wlen < 6 {
            return;
        }
        let new_len = wlen - 2;
        let mut seen: std::collections::HashSet<Vec<char>> = std::collections::HashSet::new();

        for i in 0..wlen.saturating_sub(3) {
            if status.should_abort() {
                break;
            }
            // Check if 2-char sequence at i matches 2-char sequence at i+2
            if word[i] == word[i + 2] && word[i + 1] == word[i + 3] {
                let mut buffer: Vec<char> = Vec::with_capacity(new_len);
                buffer.extend_from_slice(&word[..i]);
                buffer.extend_from_slice(&word[i + 2..]);
                if seen.insert(buffer.clone()) {
                    suggest_for_buffer(speller, status, &buffer, new_len);
                }
            }
        }
    }
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

    // --- CaseChange ---

    #[test]
    fn case_change_finds_correct_word() {
        let speller = MockSpeller::new(&["koira"]);
        let word = chars("koira");
        let mut status = SuggestionStatus::new(&word, 5);
        status.set_max_cost(100);
        CaseChange.generate(&speller, &mut status);
        assert_eq!(status.suggestion_count(), 1);
        assert_eq!(status.suggestions()[0].word, "koira");
    }

    #[test]
    fn case_change_no_match() {
        let speller = MockSpeller::new(&["kissa"]);
        let word = chars("koira");
        let mut status = SuggestionStatus::new(&word, 5);
        status.set_max_cost(100);
        CaseChange.generate(&speller, &mut status);
        assert_eq!(status.suggestion_count(), 0);
    }

    // --- SoftHyphens ---

    #[test]
    fn soft_hyphens_strips_and_checks() {
        let speller = MockSpeller::new(&["koira"]);
        let word = chars("koi\u{00AD}ra");
        let mut status = SuggestionStatus::new(&word, 5);
        status.set_max_cost(100);
        SoftHyphens.generate(&speller, &mut status);
        assert_eq!(status.suggestion_count(), 1);
        assert_eq!(status.suggestions()[0].word, "koira");
    }

    #[test]
    fn soft_hyphens_no_soft_hyphens_noop() {
        let speller = MockSpeller::new(&["koira"]);
        let word = chars("koira");
        let mut status = SuggestionStatus::new(&word, 5);
        status.set_max_cost(100);
        SoftHyphens.generate(&speller, &mut status);
        // No soft hyphens in word, generator should not produce anything
        assert_eq!(status.suggestion_count(), 0);
    }

    // --- Deletion ---

    #[test]
    fn deletion_finds_suggestion() {
        let speller = MockSpeller::new(&["koira"]);
        let word = chars("koiraa"); // extra 'a' at end
        let mut status = SuggestionStatus::new(&word, 5);
        status.set_max_cost(100);
        Deletion.generate(&speller, &mut status);
        assert!(status.suggestion_count() >= 1);
        assert!(status.suggestions().iter().any(|s| s.word == "koira"));
    }

    // --- Insertion ---

    #[test]
    fn insertion_finds_suggestion() {
        let speller = MockSpeller::new(&["koira"]);
        let word = chars("kira"); // missing 'o'
        let mut status = SuggestionStatus::new(&word, 5);
        status.set_max_cost(200);
        let sg =Insertion {
            characters: vec!['o'],
        };
        sg.generate(&speller, &mut status);
        assert!(status.suggestion_count() >= 1);
        assert!(status.suggestions().iter().any(|s| s.word == "koira"));
    }

    // --- Replacement ---

    #[test]
    fn replacement_finds_suggestion() {
        let speller = MockSpeller::new(&["koira"]);
        let word = chars("koiru"); // 'u' instead of 'a'
        let mut status = SuggestionStatus::new(&word, 5);
        status.set_max_cost(100);
        let sg =Replacement {
            replacements: vec!['u', 'a'],
        };
        sg.generate(&speller, &mut status);
        assert!(status.suggestion_count() >= 1);
        assert!(status.suggestions().iter().any(|s| s.word == "koira"));
    }

    // --- Swap ---

    #[test]
    fn swap_finds_suggestion() {
        let speller = MockSpeller::new(&["koira"]);
        let word = chars("kiora"); // 'o' and 'i' swapped
        let mut status = SuggestionStatus::new(&word, 5);
        status.set_max_cost(100);
        Swap.generate(&speller, &mut status);
        assert!(status.suggestion_count() >= 1);
        assert!(status.suggestions().iter().any(|s| s.word == "koira"));
    }

    // --- SplitWord ---

    #[test]
    fn split_word_finds_suggestion() {
        let speller = MockSpeller::new(&["koira", "kissa"]);
        let word = chars("koirakissa"); // two words joined
        let mut status = SuggestionStatus::new(&word, 5);
        status.set_max_cost(200);
        SplitWord.generate(&speller, &mut status);
        assert!(status.suggestion_count() >= 1);
        assert!(status
            .suggestions()
            .iter()
            .any(|s| s.word == "koira kissa"));
    }

    // --- VowelChange ---

    #[test]
    fn vowel_change_finds_suggestion() {
        // "koira" with back vowels -> try "k\u{00F6}ir\u{00E4}" (front vowels)
        let speller = MockSpeller::new(&["k\u{00F6}ir\u{00E4}"]);
        let word = chars("koira");
        let mut status = SuggestionStatus::new(&word, 5);
        status.set_max_cost(200);
        VowelChange.generate(&speller, &mut status);
        assert!(status.suggestion_count() >= 1);
    }

    #[test]
    fn vowel_change_no_vowels_noop() {
        let speller = MockSpeller::new(&["brk"]);
        let word = chars("brk");
        let mut status = SuggestionStatus::new(&word, 5);
        status.set_max_cost(100);
        VowelChange.generate(&speller, &mut status);
        assert_eq!(status.suggestion_count(), 0);
    }

    // --- DeleteTwo ---

    #[test]
    fn delete_two_finds_suggestion() {
        let speller = MockSpeller::new(&["koiraa"]);
        let word = chars("koiraraa"); // "ra" at [3..5] == "ra" at [5..7] -> delete -> "koiraa"
        let mut status = SuggestionStatus::new(&word, 5);
        status.set_max_cost(100);
        DeleteTwo.generate(&speller, &mut status);
        // "koiraara" has "ra" at positions 3-4 and 5-6, so deleting the pair gives "koiraa"
        assert!(status.suggestion_count() >= 1);
    }

    // --- ReplaceTwo ---

    #[test]
    fn replace_two_finds_suggestion() {
        let speller = MockSpeller::new(&["kissa"]);
        let word = chars("kitta"); // 'tt' -> 'ss' with replacement table 't','s'
        let mut status = SuggestionStatus::new(&word, 5);
        status.set_max_cost(100);
        let sg =ReplaceTwo {
            replacements: vec!['t', 's'],
        };
        sg.generate(&speller, &mut status);
        assert!(status.suggestion_count() >= 1);
        assert!(status.suggestions().iter().any(|s| s.word == "kissa"));
    }

    // --- InsertSpecial ---

    #[test]
    fn insert_special_hyphen() {
        let speller = MockSpeller::new(&["koi-ra"]);
        let word = chars("koira");
        let mut status = SuggestionStatus::new(&word, 5);
        status.set_max_cost(100);
        InsertSpecial.generate(&speller, &mut status);
        assert!(status.suggestions().iter().any(|s| s.word == "koi-ra"));
    }

    #[test]
    fn insert_special_duplication() {
        let speller = MockSpeller::new(&["kiissa"]);
        let word = chars("kissa");
        let mut status = SuggestionStatus::new(&word, 5);
        status.set_max_cost(100);
        InsertSpecial.generate(&speller, &mut status);
        // Should try duplicating 'i' -> "kiissa"
        assert!(status.suggestions().iter().any(|s| s.word == "kiissa"));
    }

    // --- MultiReplacement ---

    #[test]
    fn multi_replacement_single() {
        let speller = MockSpeller::new(&["koira"]);
        let word = chars("koiru");
        let mut status = SuggestionStatus::new(&word, 5);
        status.set_max_cost(100);
        let sg =MultiReplacement {
            replacements: vec!['u', 'a'],
            replace_count: 1,
        };
        sg.generate(&speller, &mut status);
        assert!(status.suggestion_count() >= 1);
        assert!(status.suggestions().iter().any(|s| s.word == "koira"));
    }

    #[test]
    fn multi_replacement_two() {
        let speller = MockSpeller::new(&["koira"]);
        let word = chars("loiru"); // 'l'->'k' and 'u'->'a'
        let mut status = SuggestionStatus::new(&word, 5);
        status.set_max_cost(500);
        let sg =MultiReplacement {
            replacements: vec!['l', 'k', 'u', 'a'],
            replace_count: 2,
        };
        sg.generate(&speller, &mut status);
        assert!(status.suggestion_count() >= 1);
        assert!(status.suggestions().iter().any(|s| s.word == "koira"));
    }

    // --- Abort behavior ---

    #[test]
    fn generators_respect_abort() {
        let speller = MockSpeller::new(&["a", "b", "c", "d", "e"]);
        let word = chars("x");
        let mut status = SuggestionStatus::new(&word, 1);
        status.set_max_cost(0); // will abort immediately
        Deletion.generate(&speller, &mut status);
        // Should not panic, even if budget is 0
    }

    // --- apply_structure_case ---

    #[test]
    fn apply_structure_case_all_lowercase() {
        let word = chars("koira");
        let result = apply_structure_case(&word, "=ppppp");
        let s: String = result.iter().collect();
        assert_eq!(s, "koira");
    }

    #[test]
    fn apply_structure_case_first_uppercase() {
        let word = chars("helsinki");
        let result = apply_structure_case(&word, "=ippppppp");
        let s: String = result.iter().collect();
        assert_eq!(s, "Helsinki");
    }

    #[test]
    fn apply_structure_case_mixed() {
        // "abc" with structure "=ipq" -> "Abc"
        let word = chars("abc");
        let result = apply_structure_case(&word, "=ipq");
        let s: String = result.iter().collect();
        assert_eq!(s, "Abc");
    }

    #[test]
    fn apply_structure_case_compound() {
        // Compound word: skip '=' markers
        let word = chars("koiratalo");
        let result = apply_structure_case(&word, "=ppppp=pppp");
        let s: String = result.iter().collect();
        assert_eq!(s, "koiratalo");
    }

    #[test]
    fn apply_structure_case_uppercase_to_lowercase() {
        // "KOIRA" with structure "=ppppp" -> "koira"
        let word = chars("KOIRA");
        let result = apply_structure_case(&word, "=ppppp");
        let s: String = result.iter().collect();
        assert_eq!(s, "koira");
    }

    // --- suggest_for_buffer_with_analyzer ---

    /// A mock speller that returns a specific SpellResult for specific words.
    struct CapErrorSpeller {
        cap_error_words: Vec<String>,
    }

    impl CapErrorSpeller {
        fn new(words: &[&str]) -> Self {
            Self {
                cap_error_words: words.iter().map(|s| s.to_string()).collect(),
            }
        }
    }

    impl Speller for CapErrorSpeller {
        fn spell(&self, word: &[char], word_len: usize) -> SpellResult {
            let s: String = word[..word_len].iter().collect();
            if self.cap_error_words.contains(&s) {
                SpellResult::CapitalizationError
            } else {
                SpellResult::Failed
            }
        }
    }

    /// A mock analyzer that returns pre-configured analyses.
    struct MockAnalyzer {
        entries: Vec<(String, Vec<voikko_core::analysis::Analysis>)>,
    }

    impl MockAnalyzer {
        fn new() -> Self {
            Self {
                entries: Vec::new(),
            }
        }

        fn add(&mut self, word: &str, analyses: Vec<voikko_core::analysis::Analysis>) {
            self.entries.push((word.to_string(), analyses));
        }
    }

    impl Analyzer for MockAnalyzer {
        fn analyze(&self, word: &[char], _word_len: usize) -> Vec<voikko_core::analysis::Analysis> {
            let word_str: String = word.iter().collect();
            for (w, analyses) in &self.entries {
                if *w == word_str {
                    return analyses.clone();
                }
            }
            Vec::new()
        }
    }

    fn make_analysis(pairs: &[(&str, &str)]) -> voikko_core::analysis::Analysis {
        let mut a = voikko_core::analysis::Analysis::new();
        for &(k, v) in pairs {
            a.set(k, v);
        }
        a
    }

    #[test]
    fn suggest_for_buffer_with_analyzer_fixes_case() {
        // "helsinki" -> speller returns CapitalizationError
        // analyzer says STRUCTURE "=ippppppp" -> should produce "Helsinki"
        let speller = CapErrorSpeller::new(&["helsinki"]);
        let mut analyzer = MockAnalyzer::new();
        analyzer.add(
            "helsinki",
            vec![make_analysis(&[(ATTR_STRUCTURE, "=ippppppp")])],
        );

        let word = chars("helsinki");
        let mut status = SuggestionStatus::new(&word, 5);
        status.set_max_cost(100);

        suggest_for_buffer_with_analyzer(
            &speller,
            &mut status,
            &word,
            word.len(),
            Some(&analyzer),
        );

        assert_eq!(status.suggestion_count(), 1);
        assert_eq!(status.suggestions()[0].word, "Helsinki");
    }

    #[test]
    fn suggest_for_buffer_without_analyzer_adds_unchanged() {
        // Without analyzer, CapitalizationError adds word as-is.
        let speller = CapErrorSpeller::new(&["helsinki"]);
        let word = chars("helsinki");
        let mut status = SuggestionStatus::new(&word, 5);
        status.set_max_cost(100);

        suggest_for_buffer_with_analyzer(&speller, &mut status, &word, word.len(), None);

        assert_eq!(status.suggestion_count(), 1);
        assert_eq!(status.suggestions()[0].word, "helsinki");
    }

    #[test]
    fn suggest_for_buffer_with_analyzer_no_analyses_returns_nothing() {
        // If the analyzer returns no analyses, no suggestion is added.
        let speller = CapErrorSpeller::new(&["xyz"]);
        let analyzer = MockAnalyzer::new(); // empty

        let word = chars("xyz");
        let mut status = SuggestionStatus::new(&word, 5);
        status.set_max_cost(100);

        suggest_for_buffer_with_analyzer(
            &speller,
            &mut status,
            &word,
            word.len(),
            Some(&analyzer),
        );

        assert_eq!(status.suggestion_count(), 0);
    }

    // --- Rich priority tests ---

    #[test]
    fn priority_from_noun_inflection_nominative_best() {
        assert_eq!(priority_from_noun_inflection(Some("nimento")), 2);
    }

    #[test]
    fn priority_from_noun_inflection_genitive() {
        assert_eq!(priority_from_noun_inflection(Some("omanto")), 3);
    }

    #[test]
    fn priority_from_noun_inflection_abessive_worst() {
        assert_eq!(priority_from_noun_inflection(Some("vajanto")), 60);
    }

    #[test]
    fn priority_from_noun_inflection_unknown() {
        assert_eq!(priority_from_noun_inflection(None), 4);
        assert_eq!(priority_from_noun_inflection(Some("tuntematon")), 4);
    }

    #[test]
    fn priority_noun_nominative_is_better_than_verb() {
        // Noun in nominative: class=2, struct=1, result=1 => 2
        // Verb (default class=4): class=4, struct=1, result=1 => 4
        let noun = make_analysis(&[
            (ATTR_STRUCTURE, "=ppppp"),
            ("CLASS", "nimisana"),
            ("SIJAMUOTO", "nimento"),
        ]);
        let verb = make_analysis(&[
            (ATTR_STRUCTURE, "=pppppp"),
            ("CLASS", "teonsana"),
        ]);
        let noun_prio = priority_from_analysis(&noun, SpellResult::Ok);
        let verb_prio = priority_from_analysis(&verb, SpellResult::Ok);
        assert!(noun_prio < verb_prio);
    }

    #[test]
    fn priority_compound_word_penalty() {
        // Single part: structure "=ppppp" (1 '=') => struct_prio = 1
        // Two parts: structure "=ppp=ppppp" (2 '=') => struct_prio = 8
        assert_eq!(priority_from_structure("=ppppp"), 1);
        assert_eq!(priority_from_structure("=ppp=ppppp"), 8);
        assert_eq!(priority_from_structure("=pp=pp=pp"), 64);
    }

    #[test]
    fn priority_compound_word_worse_than_simple() {
        let simple = make_analysis(&[
            (ATTR_STRUCTURE, "=ppppp"),
            ("CLASS", "nimisana"),
            ("SIJAMUOTO", "nimento"),
        ]);
        let compound = make_analysis(&[
            (ATTR_STRUCTURE, "=ppppp=pppp"),
            ("CLASS", "nimisana"),
            ("SIJAMUOTO", "nimento"),
        ]);
        let simple_prio = priority_from_analysis(&simple, SpellResult::Ok);
        let compound_prio = priority_from_analysis(&compound, SpellResult::Ok);
        assert!(simple_prio < compound_prio);
    }

    #[test]
    fn priority_spell_ok_better_than_cap_first() {
        let analysis = make_analysis(&[
            (ATTR_STRUCTURE, "=ppppp"),
            ("CLASS", "nimisana"),
            ("SIJAMUOTO", "nimento"),
        ]);
        let ok_prio = priority_from_analysis(&analysis, SpellResult::Ok);
        let cap_prio = priority_from_analysis(&analysis, SpellResult::CapitalizeFirst);
        assert!(ok_prio < cap_prio);
    }

    #[test]
    fn best_priority_picks_lowest() {
        let analyses = vec![
            make_analysis(&[
                (ATTR_STRUCTURE, "=ppppp=pppp"),
                ("CLASS", "nimisana"),
                ("SIJAMUOTO", "ulkoeronto"),
            ]),
            make_analysis(&[
                (ATTR_STRUCTURE, "=ppppp"),
                ("CLASS", "nimisana"),
                ("SIJAMUOTO", "nimento"),
            ]),
        ];
        let best = best_priority_from_analyses(&analyses, SpellResult::Ok);
        // The second analysis (simple noun, nominative) should win.
        let expected = priority_from_analysis(&analyses[1], SpellResult::Ok);
        assert_eq!(best, expected);
    }

    #[test]
    fn best_priority_empty_analyses_uses_flat() {
        let empty: Vec<voikko_core::analysis::Analysis> = Vec::new();
        let prio = best_priority_from_analyses(&empty, SpellResult::Ok);
        assert_eq!(prio, priority_from_result(SpellResult::Ok));
    }
}
