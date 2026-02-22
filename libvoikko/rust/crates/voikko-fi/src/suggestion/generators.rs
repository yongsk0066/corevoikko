// Individual suggestion generators: each applies one class of edit operation
// to produce candidate words, then validates them via the speller.
//
// Origin: spellchecker/suggestion/SuggestionGenerator*.cpp

use voikko_core::character::{is_upper, simple_lower, simple_upper};
use voikko_core::enums::SpellResult;

use crate::speller::Speller;
use super::status::SuggestionStatus;

/// Back vowels used in Finnish vowel harmony (lowercase + uppercase).
///
/// Origin: SuggestionGeneratorVowelChange.cpp:35, SuggestionGeneratorSwap.cpp:38
const BACK_VOWELS: &[char] = &['a', 'o', 'u', 'A', 'O', 'U'];

/// Front vowels corresponding to back vowels (same index order).
///
/// Origin: SuggestionGeneratorVowelChange.cpp:36, SuggestionGeneratorSwap.cpp:39
const FRONT_VOWELS: &[char] = &[
    '\u{00E4}', '\u{00F6}', 'y',
    '\u{00C4}', '\u{00D6}', 'Y',
];

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
/// This is the Rust equivalent of the static
/// `SuggestionGeneratorCaseChange::suggestForBuffer` which every generator
/// calls after constructing a candidate.
///
/// Origin: SuggestionGeneratorCaseChange.cpp:49-106
pub fn suggest_for_buffer(
    speller: &dyn Speller,
    status: &mut SuggestionStatus<'_>,
    buffer: &[char],
    buf_len: usize,
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
            let s: String = word.iter().collect();
            status.add_suggestion(s, priority_from_result(result));
        }
        SpellResult::CapitalizeFirst => {
            let mut corrected: Vec<char> = word.to_vec();
            corrected[0] = simple_upper(corrected[0]);
            let s: String = corrected.iter().collect();
            status.add_suggestion(s, priority_from_result(result));
        }
        SpellResult::CapitalizationError => {
            // The speller already told us the word exists but with
            // different capitalization. We would need analysis STRUCTURE
            // data to fix it properly. For now, just add the word as-is
            // with a high priority penalty, matching the C++ behavior
            // where the full analysis path is available.
            //
            // In the C++ code this calls morAnalyzer->analyze() to read
            // the STRUCTURE attribute and fix case. We approximate by
            // adding the word unchanged.
            let s: String = word.iter().collect();
            status.add_suggestion(s, priority_from_result(result));
        }
    }
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
        let word = status.word();
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
        let word = status.word();
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
        let word = status.word();
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
            buffer[1..=wlen].copy_from_slice(word);

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
        let word = status.word();
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
        buffer[1..=wlen].copy_from_slice(word);

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
        let word = status.word();
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
        let word = status.word();
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
        let word = status.word();
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
        let word = status.word();
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
        let word = status.word();
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
                    part1.extend_from_slice(word);
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
            part1.extend_from_slice(word);
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
        let word = status.word();
        let wlen = status.word_len();

        // Count vowels and build a mask
        let mut vcount: usize = 0;
        let mut mask: u32 = 0;
        for &c in word {
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
            buffer.copy_from_slice(word);

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
        let word = status.word();
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
        let gen = Insertion {
            characters: vec!['o'],
        };
        gen.generate(&speller, &mut status);
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
        let gen = Replacement {
            replacements: vec!['u', 'a'],
        };
        gen.generate(&speller, &mut status);
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
        let word = chars("koiraara"); // "ra" repeated -> "koiraa"
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
        let gen = ReplaceTwo {
            replacements: vec!['t', 's'],
        };
        gen.generate(&speller, &mut status);
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
        let gen = MultiReplacement {
            replacements: vec!['u', 'a'],
            replace_count: 1,
        };
        gen.generate(&speller, &mut status);
        assert!(status.suggestion_count() >= 1);
        assert!(status.suggestions().iter().any(|s| s.word == "koira"));
    }

    #[test]
    fn multi_replacement_two() {
        let speller = MockSpeller::new(&["koira"]);
        let word = chars("loiru"); // 'l'->'k' and 'u'->'a'
        let mut status = SuggestionStatus::new(&word, 5);
        status.set_max_cost(500);
        let gen = MultiReplacement {
            replacements: vec!['l', 'k', 'u', 'a'],
            replace_count: 2,
        };
        gen.generate(&speller, &mut status);
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
}
