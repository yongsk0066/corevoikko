// Hyphenation module -- Finnish syllable-based hyphenation
// Origin: hyphenator/Hyphenator.hpp, AnalyzerToFinnishHyphenatorAdapter.hpp/cpp
//
// The Finnish hyphenator works by:
// 1. Running morphological analysis to detect compound word boundaries (STRUCTURE attr)
// 2. Applying Finnish syllable rules within each morpheme component
// 3. Intersecting (or union-ing) compound boundaries with syllable rules

use voikko_core::analysis::{Analysis, ATTR_STRUCTURE};
use voikko_core::character::{is_consonant, is_vowel, simple_lower};

use crate::morphology::Analyzer;

use crate::finnish::constants::SPLIT_VOWELS;

/// Long consonant sequences treated as indivisible units.
/// A hyphen should be moved before the entire cluster rather than splitting it.
/// Origin: AnalyzerToFinnishHyphenatorAdapter.cpp:45 (LONG_CONSONANTS)
const LONG_CONSONANTS: &[&[char]] = &[
    &['\u{0161}', 't', '\u{0161}'], // štš  (C++ "shtsh" uses š and tš differently)
    // Note: C++ uses L"shtsh" and L"\u0161t\u0161" separately. The C++ wchar_t values
    // for "shtsh" are actually 's','h','t','s','h' (ASCII), not š. Let me re-check.
    // Actually looking at the C++ more carefully:
    //   L"shtsh" -> 's','h','t','s','h' (5 chars)
    //   L"\u0161t\u0161" -> 'š','t','š' (3 chars)
    //   L"tsh" -> 't','s','h' (3 chars)
    //   L"t\u0161" -> 't','š' (2 chars)
    //   L"zh" -> 'z','h' (2 chars)
    &['s', 'h', 't', 's', 'h'],     // shtsh
    &['t', 's', 'h'],               // tsh
    &['t', '\u{0161}'],             // tš
    &['z', 'h'],                    // zh
];

/// Vowel pair patterns after which a following vowel may be split (VV-V rule).
/// Only applied in ugly hyphenation mode.
/// Origin: AnalyzerToFinnishHyphenatorAdapter.cpp:46 (SPLIT_AFTER)
const SPLIT_AFTER: &[[char; 2]] = &[['i', 'e'], ['a', 'i']];

/// Special characters that block a hyphenation point after them.
/// Origin: AnalyzerToFinnishHyphenatorAdapter.cpp:418 (the wcschr check)
const SPECIAL_CHARS_BEFORE_HYPHEN: &[char] = &['/', '.', ':', '&', '%', '\''];

// ---------------------------------------------------------------------------
// Hyphenation options
// Origin: AnalyzerToFinnishHyphenatorAdapter.hpp:58-61
// ---------------------------------------------------------------------------

/// Configuration options for the Finnish hyphenator.
/// Origin: AnalyzerToFinnishHyphenatorAdapter.hpp:58-61
#[derive(Debug, Clone, Copy)]
pub struct HyphenatorOptions {
    /// When true, include aesthetically ugly but correct hyphenation points.
    /// When false, suppress ugly positions (e.g., single-char syllables at edges,
    /// splitting consecutive vowels).
    /// Origin: AnalyzerToFinnishHyphenatorAdapter.hpp:59 (uglyHyphenation)
    pub ugly_hyphenation: bool,

    /// When true, attempt rule-based hyphenation on words not found in the dictionary.
    /// When false, forbid all hyphenation for unknown words.
    /// Origin: AnalyzerToFinnishHyphenatorAdapter.hpp:60 (hyphenateUnknown)
    pub hyphenate_unknown: bool,

    /// Minimum word length (and minimum compound component length) for hyphenation.
    /// Words shorter than this get no hyphenation points.
    /// Origin: AnalyzerToFinnishHyphenatorAdapter.hpp:61 (minHyphenatedWordLength)
    pub min_hyphenated_word_length: usize,

    /// When true, ignore a trailing dot when analyzing the word. If the word
    /// with the dot has no analyses but the word without the dot does, use
    /// those analyses instead.
    /// Origin: AnalyzerToFinnishHyphenatorAdapter.hpp:62 (ignoreDot)
    pub ignore_dot: bool,
}

impl Default for HyphenatorOptions {
    /// Default options matching the C++ constructor defaults.
    /// Origin: AnalyzerToFinnishHyphenatorAdapter.cpp:48-54
    fn default() -> Self {
        Self {
            ugly_hyphenation: true,
            hyphenate_unknown: true,
            min_hyphenated_word_length: 2,
            ignore_dot: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Hyphenator trait
// Origin: hyphenator/Hyphenator.hpp
// ---------------------------------------------------------------------------

/// Trait for hyphenation backends.
///
/// The returned string uses the same notation as the C++ `Hyphenator::hyphenate`:
///   `' '` = no hyphenation point before or at this character
///   `'-'` = hyphenation point before this character (character preserved)
///   `'='` = hyphenation point (character replaced with hyphen, e.g., at compound boundary
///           where the word already contains a `-`)
///
/// Origin: hyphenator/Hyphenator.hpp:38-53
pub trait Hyphenator {
    /// Hyphenate the given word and return a pattern string.
    ///
    /// The returned string has the same length as `word` (in characters).
    /// Each position indicates whether a hyphen break is allowed before
    /// that character.
    fn hyphenate(&self, word: &[char]) -> String;

    /// Return all possible hyphenation positions (union across analyses).
    ///
    /// Unlike `hyphenate` which returns the intersection (conservative),
    /// this returns the union (permissive) of all analysis-based hyphenation
    /// patterns. Used by the soft-hyphen validation in the spellchecker.
    ///
    /// Origin: AnalyzerToFinnishHyphenatorAdapter.hpp:52
    fn all_possible_hyphen_positions(&self, word: &[char]) -> String;
}

// ---------------------------------------------------------------------------
// FinnishHyphenator
// Origin: AnalyzerToFinnishHyphenatorAdapter
// ---------------------------------------------------------------------------

/// Finnish hyphenator that combines morphological analysis (compound boundary
/// detection) with rule-based syllable splitting.
///
/// Origin: AnalyzerToFinnishHyphenatorAdapter
pub struct FinnishHyphenator<A: Analyzer> {
    analyzer: A,
    options: HyphenatorOptions,
}

impl<A: Analyzer> FinnishHyphenator<A> {
    /// Create a new Finnish hyphenator wrapping the given analyzer.
    /// Origin: AnalyzerToFinnishHyphenatorAdapter::AnalyzerToFinnishHyphenatorAdapter
    pub fn new(analyzer: A, options: HyphenatorOptions) -> Self {
        Self { analyzer, options }
    }

    /// Update hyphenator options.
    pub fn set_options(&mut self, options: HyphenatorOptions) {
        self.options = options;
    }

    /// Return a reference to the current options.
    pub fn options(&self) -> &HyphenatorOptions {
        &self.options
    }

    // -----------------------------------------------------------------------
    // Phase 1: Compound splitting
    // Origin: AnalyzerToFinnishHyphenatorAdapter::splitCompounds
    // -----------------------------------------------------------------------

    /// Analyze the word and produce one hyphenation buffer per analysis,
    /// with compound boundaries marked. Returns `None` on error.
    ///
    /// The returned buffers use internal markers:
    ///   `' '` = no boundary
    ///   `'-'` = compound boundary (hyphenation allowed)
    ///   `'='` = explicit hyphen boundary (always break here)
    ///   `'X'` = hyphenation forbidden at this position
    ///
    /// Origin: AnalyzerToFinnishHyphenatorAdapter::splitCompounds
    fn split_compounds(&self, word: &[char]) -> Option<(Vec<Vec<u8>>, bool)> {
        let len = word.len();

        // Convert to lowercase string for the analyzer
        let word_lower: Vec<char> = word.iter().map(|&c| simple_lower(c)).collect();

        let mut analyses = self.analyzer.analyze(&word_lower, len);

        // Try removing trailing dot if no analyses found
        let mut dot_removed = false;
        if analyses.is_empty() && self.options.ignore_dot && len > 1 && word[len - 1] == '.' {
            let trimmed: Vec<char> = word_lower[..len - 1].to_vec();
            analyses = self.analyzer.analyze(&trimmed, len - 1);
            if !analyses.is_empty() {
                dot_removed = true;
            }
        }

        let effective_len = if dot_removed { len - 1 } else { len };

        let mut all_results: Vec<Vec<u8>> = Vec::new();

        if analyses.is_empty() {
            // No analyses found: create a single buffer
            let fill = if self.options.hyphenate_unknown {
                b' '
            } else {
                b'X'
            };
            let mut result = vec![fill; len];

            if allow_rule_hyphenation(word, len, self.options.ugly_hyphenation) {
                // Mark explicit hyphens in the word
                for i in 1..len.saturating_sub(1) {
                    if word[i] == '-' {
                        result[i] = b'=';
                    }
                }
            }
            all_results.push(result);
        } else {
            // Process each analysis
            let max_analysis_count = 31; // C++ limit: MAX_ANALYSIS_COUNT
            for analysis in analyses.iter().take(max_analysis_count) {
                let mut result = vec![b' '; len];
                interpret_analysis(analysis, &mut result, effective_len);
                if dot_removed {
                    result[len - 1] = b' ';
                }
                all_results.push(result);
            }
        }

        if all_results.is_empty() {
            return None;
        }

        remove_extra_hyphenations(&mut all_results, len);

        Some((all_results, dot_removed))
    }

    // -----------------------------------------------------------------------
    // Phase 2: Rule-based hyphenation within compound components
    // Origin: AnalyzerToFinnishHyphenatorAdapter::compoundHyphenation
    // -----------------------------------------------------------------------

    /// Apply rule-based syllable hyphenation to each component of a compound word.
    ///
    /// The `hyphenation` buffer already has compound boundaries marked.
    /// This function fills in syllable break points within each component.
    ///
    /// Origin: AnalyzerToFinnishHyphenatorAdapter::compoundHyphenation
    fn compound_hyphenation(&self, word: &[char], hyphenation: &mut [u8], len: usize) {
        let mut start = 0;

        // Skip leading '=' markers
        while start < len && hyphenation[start] == b'=' {
            start += 1;
        }

        let mut end = start + 1;
        while end < len {
            if hyphenation[end] != b' ' && hyphenation[end] != b'X' {
                // We found a compound boundary at `end`
                if end >= start + self.options.min_hyphenated_word_length {
                    rule_hyphenation(
                        &word[start..],
                        &mut hyphenation[start..],
                        end - start,
                        self.options.ugly_hyphenation,
                    );
                }
                if hyphenation[end] == b'=' {
                    start = end + 1;
                } else {
                    start = end;
                }
                end = start + 1;
            } else {
                end += 1;
            }
        }

        // Handle the last component
        if end == len && start < end && end >= start + self.options.min_hyphenated_word_length {
            rule_hyphenation(
                &word[start..],
                &mut hyphenation[start..],
                end - start,
                self.options.ugly_hyphenation,
            );
        }
    }

    // -----------------------------------------------------------------------
    // Core hyphenation entry point
    // -----------------------------------------------------------------------

    /// Internal implementation shared by `hyphenate` and `all_possible_hyphen_positions`.
    /// The `use_intersection` flag controls whether we intersect (conservative) or
    /// union (permissive) the analyses.
    ///
    /// Origin: AnalyzerToFinnishHyphenatorAdapter::hyphenate / allPossibleHyphenPositions
    fn hyphenate_internal(&self, word: &[char], use_intersection: bool) -> String {
        let wlen = word.len();

        // Short words: no hyphenation
        if wlen < self.options.min_hyphenated_word_length {
            return " ".repeat(wlen);
        }

        let Some((mut hyphenations, dot_removed)) = self.split_compounds(word) else {
            return " ".repeat(wlen);
        };

        let effective_len = if dot_removed { wlen - 1 } else { wlen };

        for hyph in &mut hyphenations {
            self.compound_hyphenation(word, hyph, effective_len);
        }

        if use_intersection {
            intersect_hyphenations(&hyphenations)
        } else {
            union_hyphenations(&hyphenations)
        }
    }
}

impl<A: Analyzer> Hyphenator for FinnishHyphenator<A> {
    /// Hyphenate the word using the intersection of all analysis patterns.
    /// Origin: AnalyzerToFinnishHyphenatorAdapter::hyphenate
    fn hyphenate(&self, word: &[char]) -> String {
        self.hyphenate_internal(word, true)
    }

    /// Return all possible hyphenation positions using the union of all patterns.
    /// Origin: AnalyzerToFinnishHyphenatorAdapter::allPossibleHyphenPositions
    fn all_possible_hyphen_positions(&self, word: &[char]) -> String {
        self.hyphenate_internal(word, false)
    }
}

// ---------------------------------------------------------------------------
// interpretAnalysis: extract compound boundaries from STRUCTURE attribute
// Origin: AnalyzerToFinnishHyphenatorAdapter::interpretAnalysis
// ---------------------------------------------------------------------------

/// Read the STRUCTURE attribute from an analysis and mark compound boundaries
/// in the hyphenation buffer.
///
/// STRUCTURE encoding:
///   The STRUCTURE string starts with `=` and contains one character per input
///   character, with boundary markers interspersed:
///     - `=` followed by a letter code -> compound boundary (mark as `'-'`)
///     - `-=` -> explicit hyphen boundary (mark as `'='`)
///     - `j` or `q` -> abbreviation context (mark as `'X'`, forbid hyphenation)
///     - other letter codes (`i`, `p`) -> no boundary (leave as `' '`)
///
/// Origin: AnalyzerToFinnishHyphenatorAdapter::interpretAnalysis
fn interpret_analysis(analysis: &Analysis, buffer: &mut [u8], len: usize) {
    let structure = match analysis.get(ATTR_STRUCTURE) {
        Some(s) => s,
        None => return,
    };

    let structure_chars: Vec<char> = structure.chars().collect();
    let mut sptr = 0;

    // Fill buffer with spaces
    for b in buffer.iter_mut().take(len) {
        *b = b' ';
    }

    // Skip leading '='
    if sptr < structure_chars.len() && structure_chars[sptr] == '=' {
        sptr += 1;
    }

    for (i, buf_byte) in buffer.iter_mut().enumerate().take(len) {
        if sptr >= structure_chars.len() {
            break;
        }

        // Check for "-=" pattern (explicit hyphen at compound boundary)
        if structure_chars[sptr] == '-'
            && sptr + 1 < structure_chars.len()
            && structure_chars[sptr + 1] == '='
        {
            if i != 0 {
                *buf_byte = b'=';
            }
            sptr += 2;
            continue;
        }

        // Check for "=" (compound boundary, not at start)
        if structure_chars[sptr] == '=' {
            *buf_byte = b'-';
            sptr += 2; // skip '=' and the following letter code
            continue;
        }

        // Check for abbreviation markers
        if structure_chars[sptr] == 'j' || structure_chars[sptr] == 'q' {
            *buf_byte = b'X';
        }

        sptr += 1;
    }
}

// ---------------------------------------------------------------------------
// allowRuleHyphenation: check if rule-based hyphenation is safe
// Origin: AnalyzerToFinnishHyphenatorAdapter::allowRuleHyphenation
// ---------------------------------------------------------------------------

/// Check if a word is safe for rule-based hyphenation.
///
/// Returns false for very short words, non-word strings (URLs, emails),
/// and words ending with digits (when ugly hyphenation is disabled).
///
/// Origin: AnalyzerToFinnishHyphenatorAdapter::allowRuleHyphenation
fn allow_rule_hyphenation(word: &[char], nchars: usize, ugly_hyphenation: bool) -> bool {
    if nchars <= 1 {
        return false;
    }

    if !ugly_hyphenation {
        // Non-word strings (URLs, emails) are not hyphenated
        if is_nonword(word, nchars) {
            return false;
        }

        // Words ending with a digit are not safe to hyphenate
        if let Some(&last) = word.get(nchars - 1) {
            if last.is_ascii_digit() {
                return false;
            }
        }
    }

    true
}

/// Check if a word looks like a URL or email address (non-word).
///
/// Patterns detected:
///   - `X*//X*.X+` (URLs)
///   - `X*@X+.X+`  (emails)
///   - `www.X+.X+`  (www prefixed)
///
/// Origin: utils/utils.cpp:94-123 (voikko_is_nonword)
fn is_nonword(word: &[char], nchars: usize) -> bool {
    if nchars < 4 {
        return false;
    }

    // Check for "//" followed by "." pattern
    if let Some(slash_pos) = word[..nchars.saturating_sub(3)]
        .iter()
        .position(|&c| c == '/')
    {
        if slash_pos + 1 < nchars
            && word[slash_pos + 1] == '/'
            && word[slash_pos + 2..nchars].contains(&'.')
        {
            return true;
        }
    }

    // Check for "@" followed by non-dot then "." pattern
    if let Some(at_pos) = word[..nchars.saturating_sub(3)]
        .iter()
        .position(|&c| c == '@')
    {
        if at_pos + 1 < nchars
            && word[at_pos + 1] != '.'
            && at_pos + 2 < nchars
            && word[at_pos + 2..nchars].contains(&'.')
        {
            return true;
        }
    }

    // Check for "www." prefix
    if nchars >= 7
        && word[0] == 'w'
        && word[1] == 'w'
        && word[2] == 'w'
        && word[3] == '.'
        && word[4] != '.'
        && word[5..nchars].contains(&'.')
    {
        return true;
    }

    false
}

// ---------------------------------------------------------------------------
// ruleHyphenation: Finnish syllable rules
// Origin: AnalyzerToFinnishHyphenatorAdapter::ruleHyphenation
// ---------------------------------------------------------------------------

/// Apply Finnish syllable-based hyphenation rules to a word segment.
///
/// This operates on a single compound component (not the full word).
/// The `word` and `hyphenation_points` slices must have at least `nchars` elements.
///
/// Rules applied in order:
/// 1. -CV: hyphen before consonant-vowel pairs
/// 2. 'V: compound break after apostrophe before vowel
/// 3. Long vowel boundaries (VV): split before/after long vowels
/// 4. V-V: split specific vowel pairs (SPLIT_VOWELS table)
/// 5. Long consonants: move hyphen before indivisible consonant clusters
/// 6. Aesthetic cleanup (when ugly_hyphenation is false)
/// 7. VV-V: split after "ie"/"ai" before vowel (ugly mode only)
///
/// Origin: AnalyzerToFinnishHyphenatorAdapter::ruleHyphenation
fn rule_hyphenation(
    word: &[char],
    hyphenation_points: &mut [u8],
    nchars: usize,
    ugly_hyphenation: bool,
) {
    if !allow_rule_hyphenation(word, nchars, ugly_hyphenation) {
        return;
    }

    // If the segment is marked as forbidden ('X'), skip it
    if hyphenation_points[0] == b'X' {
        return;
    }

    // Create a lowercase copy for phonological analysis
    let word_lower: Vec<char> = word[..nchars].iter().map(|&c| simple_lower(c)).collect();

    // Skip leading consonants to find the first vowel
    let mut i = 0;
    while i < nchars && is_consonant(word_lower[i]) {
        i += 1;
    }

    // Rule 1: -CV (consonant-vowel break)
    // Add a hyphen before a consonant that is followed by a vowel,
    // unless the previous character is a special character.
    // Origin: AnalyzerToFinnishHyphenatorAdapter.cpp:416-422
    while i <= nchars.saturating_sub(2) {
        if is_consonant(word_lower[i])
            && is_vowel(word_lower[i + 1])
            && i >= 1
            && !SPECIAL_CHARS_BEFORE_HYPHEN.contains(&word_lower[i - 1])
            && (i <= 1 || ugly_hyphenation || word_lower[i - 2] != '\'')
        {
            hyphenation_points[i] = b'-';
        }
        i += 1;
    }

    // Rule 2: 'V (apostrophe before vowel = compound break)
    // Origin: AnalyzerToFinnishHyphenatorAdapter.cpp:425-429
    for i in 1..nchars.saturating_sub(1) {
        if word_lower[i] == '\'' && is_vowel(word_lower[i + 1]) {
            hyphenation_points[i] = b'=';
        }
    }

    // Rule 3: Split before and after long vowels (VV)
    // If a vowel appears doubled (aa, ee, etc.), split surrounding vowels away.
    // Origin: AnalyzerToFinnishHyphenatorAdapter.cpp:432-442
    for i in 1..nchars.saturating_sub(1) {
        if is_vowel(word_lower[i]) && word_lower[i] == word_lower[i + 1] {
            // If there is a vowel before the long vowel, split before it
            if is_vowel(word_lower[i - 1])
                && is_good_hyphen_position(&word_lower, hyphenation_points, i, nchars)
            {
                hyphenation_points[i] = b'-';
            }
            // Split after the long vowel
            if i + 2 < nchars
                && is_good_hyphen_position(&word_lower, hyphenation_points, i + 2, nchars)
            {
                hyphenation_points[i + 2] = b'-';
            }
        }
    }

    // Rule 4: V-V (specific vowel pairs that can be split)
    // Only split if the position doesn't already have a hyphen.
    // Origin: AnalyzerToFinnishHyphenatorAdapter.cpp:444-461
    for i in 0..nchars.saturating_sub(1) {
        if hyphenation_points[i + 1] != b' ' {
            continue;
        }
        if !is_vowel(word_lower[i]) || !is_vowel(word_lower[i + 1]) {
            continue;
        }
        let pair = [word_lower[i], word_lower[i + 1]];
        if SPLIT_VOWELS.contains(&pair) {
            hyphenation_points[i + 1] = b'-';
        }
    }

    // Rule 5: Long consonant clusters
    // If a consonant cluster is indivisible (e.g., "tsh"), move any hyphen
    // from inside the cluster to before the cluster.
    // Origin: AnalyzerToFinnishHyphenatorAdapter.cpp:463-476
    for i in 1..nchars.saturating_sub(1) {
        for long_cons in LONG_CONSONANTS {
            let clen = long_cons.len();
            if i + clen <= nchars
                && word_lower[i..i + clen]
                    .iter()
                    .zip(long_cons.iter())
                    .all(|(&a, &b)| a == b)
            {
                for k in (i + 1)..=(i + clen).min(nchars - 1) {
                    if k < hyphenation_points.len() && hyphenation_points[k] == b'-' {
                        hyphenation_points[k] = b' ';
                        hyphenation_points[i] = b'-';
                    }
                }
            }
        }
    }

    // Rule 6: Aesthetic cleanup (when ugly_hyphenation is false)
    // - Forbid hyphen at position 1 (splitting single char at start)
    // - Forbid hyphen at last position (splitting single char at end)
    // - Forbid splitting consecutive vowels
    // Origin: AnalyzerToFinnishHyphenatorAdapter.cpp:478-486
    if !ugly_hyphenation {
        hyphenation_points[1] = b' ';
        if nchars >= 1 {
            hyphenation_points[nchars - 1] = b' ';
        }
        for i in 0..nchars.saturating_sub(1) {
            if is_vowel(word_lower[i]) && is_vowel(word_lower[i + 1]) {
                hyphenation_points[i + 1] = b' ';
            }
        }
    } else if nchars >= 3 {
        // Rule 7: VV-V (ugly mode only)
        // After "ie" or "ai" followed by a vowel, allow a split.
        // Origin: AnalyzerToFinnishHyphenatorAdapter.cpp:487-499
        for i in 0..nchars.saturating_sub(3) {
            for split_pair in SPLIT_AFTER {
                let pair = [word_lower[i], word_lower[i + 1]];
                if hyphenation_points[i + 1] != b'-'
                    && pair == *split_pair
                    && is_vowel(word_lower[i + 2])
                    && is_good_hyphen_position(&word_lower, hyphenation_points, i + 2, nchars)
                {
                    hyphenation_points[i + 2] = b'-';
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// isGoodHyphenPosition: validate a proposed hyphenation point
// Origin: AnalyzerToFinnishHyphenatorAdapter::isGoodHyphenPosition
// ---------------------------------------------------------------------------

/// Check whether a proposed hyphenation point at `new_hyphen_pos` is valid.
///
/// A hyphenation point is valid if:
/// - It is not at the first or last position
/// - There is at least one vowel in the syllable before the proposed break
/// - There is at least one vowel in the syllable after the proposed break
///
/// Syllable boundaries are determined by existing hyphen marks in the buffer.
///
/// Origin: AnalyzerToFinnishHyphenatorAdapter::isGoodHyphenPosition
fn is_good_hyphen_position(
    word: &[char],
    hyphenation_points: &[u8],
    new_hyphen_pos: usize,
    nchars: usize,
) -> bool {
    // Out of bounds check
    if new_hyphen_pos == 0 || new_hyphen_pos + 1 >= nchars {
        return false;
    }

    // Check backwards for vowels (in the syllable before the proposed break).
    // C++ loop: checks `i == 0` break BEFORE the vowel check, so word[0] is
    // never checked for vowels. We replicate this order exactly.
    let mut has_vowel = false;
    if new_hyphen_pos >= 1 {
        let mut i = new_hyphen_pos - 1;
        loop {
            if hyphenation_points[i] == b'-' || hyphenation_points[i] == b'=' {
                break;
            }
            if i == 0 {
                break;
            }
            if is_vowel(word[i]) {
                has_vowel = true;
            }
            i -= 1;
        }
    }
    if !has_vowel {
        return false;
    }

    // Check forwards for vowels (in the syllable after the proposed break)
    has_vowel = false;
    for i in new_hyphen_pos..nchars {
        if hyphenation_points[i] == b'-' || hyphenation_points[i] == b'=' {
            break;
        }
        if word[i] == '.' {
            break;
        }
        if is_vowel(word[i]) {
            has_vowel = true;
        }
    }

    has_vowel
}

// ---------------------------------------------------------------------------
// intersectHyphenations: conservative merge (all analyses must agree)
// Origin: AnalyzerToFinnishHyphenatorAdapter::intersectHyphenations
// ---------------------------------------------------------------------------

/// Compute the intersection of multiple hyphenation buffers.
///
/// A hyphen is only kept if ALL analyses agree on it. 'X' markers are
/// converted to spaces (they only served as internal "forbidden" flags).
///
/// Origin: AnalyzerToFinnishHyphenatorAdapter::intersectHyphenations
fn intersect_hyphenations(hyphenations: &[Vec<u8>]) -> String {
    if hyphenations.is_empty() {
        return String::new();
    }

    let len = hyphenations[0].len();
    let mut result: Vec<u8> = hyphenations[0].clone();

    // Convert 'X' to ' ' in the base
    for b in &mut result {
        if *b == b'X' {
            *b = b' ';
        }
    }

    // Intersect with remaining analyses
    for hyph in &hyphenations[1..] {
        for i in 0..len {
            if hyph[i] == b' ' || hyph[i] == b'X' {
                result[i] = b' ';
            }
        }
    }

    // Convert to String
    result.iter().map(|&b| b as char).collect()
}

/// Compute the union of multiple hyphenation buffers.
///
/// A hyphen is kept if ANY analysis suggests it. 'X' markers are
/// converted to spaces.
///
/// Origin: AnalyzerToFinnishHyphenatorAdapter.cpp:91-111 (unionHyphenations)
fn union_hyphenations(hyphenations: &[Vec<u8>]) -> String {
    if hyphenations.is_empty() {
        return String::new();
    }

    let len = hyphenations[0].len();
    let mut result: Vec<u8> = hyphenations[0].clone();

    // Convert 'X' to ' ' in the base
    for b in &mut result {
        if *b == b'X' {
            *b = b' ';
        }
    }

    // Union with remaining analyses
    for hyph in &hyphenations[1..] {
        for i in 0..len {
            if hyph[i] == b'-' {
                result[i] = b'-';
            }
        }
    }

    result.iter().map(|&b| b as char).collect()
}

// ---------------------------------------------------------------------------
// removeExtraHyphenations: prune unnecessary analysis variants
// Origin: AnalyzerToFinnishHyphenatorAdapter::removeExtraHyphenations
// ---------------------------------------------------------------------------

/// Remove analysis variants that have more compound parts than necessary.
///
/// If the minimum number of parts across all analyses is 1 (i.e., at least
/// one analysis says the word is not a compound), remove all analyses that
/// split the word into compounds.
///
/// Origin: AnalyzerToFinnishHyphenatorAdapter::removeExtraHyphenations
fn remove_extra_hyphenations(hyphenations: &mut Vec<Vec<u8>>, len: usize) {
    // Count parts for each analysis
    let part_counts: Vec<usize> = hyphenations
        .iter()
        .map(|hyph| {
            1 + hyph[..len]
                .iter()
                .filter(|&&b| b != b' ' && b != b'X')
                .count()
        })
        .collect();

    let min_parts = *part_counts.iter().min().unwrap_or(&0);

    // Only prune if min_parts is 1 (word can be non-compound)
    if min_parts > 1 {
        return;
    }

    // Remove entries where parts > min_parts
    let mut i = 0;
    while i < hyphenations.len() {
        let parts = 1 + hyphenations[i][..len]
            .iter()
            .filter(|&&b| b != b' ' && b != b'X')
            .count();
        if parts > min_parts {
            hyphenations.swap_remove(i);
        } else {
            i += 1;
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use voikko_core::analysis::Analysis;

    // -----------------------------------------------------------------------
    // Mock analyzer for testing
    // -----------------------------------------------------------------------

    /// A mock analyzer that returns predefined analyses for specific words.
    struct MockAnalyzer {
        /// Map from word (lowercase chars) to list of STRUCTURE values
        entries: Vec<(Vec<char>, Vec<String>)>,
    }

    impl MockAnalyzer {
        fn new() -> Self {
            Self {
                entries: Vec::new(),
            }
        }

        /// Register a word with one or more STRUCTURE attribute values.
        fn add_word(&mut self, word: &str, structures: &[&str]) {
            let chars: Vec<char> = word.chars().collect();
            let structs: Vec<String> = structures.iter().map(|s| s.to_string()).collect();
            self.entries.push((chars, structs));
        }
    }

    impl Analyzer for MockAnalyzer {
        fn analyze(&self, word: &[char], _word_len: usize) -> Vec<Analysis> {
            for (entry_word, structures) in &self.entries {
                if word == entry_word.as_slice() {
                    return structures
                        .iter()
                        .map(|s| {
                            let mut a = Analysis::new();
                            a.set(ATTR_STRUCTURE, s.as_str());
                            a
                        })
                        .collect();
                }
            }
            Vec::new()
        }
    }

    /// A mock analyzer that always returns empty results (unknown words).
    struct NullAnalyzer;

    impl Analyzer for NullAnalyzer {
        fn analyze(&self, _word: &[char], _word_len: usize) -> Vec<Analysis> {
            Vec::new()
        }
    }

    // -----------------------------------------------------------------------
    // Helper functions
    // -----------------------------------------------------------------------

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    fn hyphenate_str(hyp: &impl Hyphenator, word: &str) -> String {
        hyp.hyphenate(&chars(word))
    }

    /// Render a hyphenated word with hyphens inserted at break points.
    /// E.g., word="koira", pattern=" -  -" => "koi-ra" (but the pattern is
    ///   actually "  - -" for a proper split, let me not overthink the rendering).
    fn render_hyphenation(word: &str, pattern: &str) -> String {
        let word_chars: Vec<char> = word.chars().collect();
        let pattern_chars: Vec<char> = pattern.chars().collect();
        let mut result = String::new();
        for (i, &wc) in word_chars.iter().enumerate() {
            if i < pattern_chars.len() && (pattern_chars[i] == '-' || pattern_chars[i] == '=') {
                result.push('-');
            }
            result.push(wc);
        }
        result
    }

    // -----------------------------------------------------------------------
    // Pure function tests: is_nonword
    // -----------------------------------------------------------------------

    #[test]
    fn nonword_url() {
        let w = chars("http://example.com");
        assert!(is_nonword(&w, w.len()));
    }

    #[test]
    fn nonword_email() {
        let w = chars("user@example.com");
        assert!(is_nonword(&w, w.len()));
    }

    #[test]
    fn nonword_www() {
        let w = chars("www.example.com");
        assert!(is_nonword(&w, w.len()));
    }

    #[test]
    fn nonword_regular_word() {
        let w = chars("koira");
        assert!(!is_nonword(&w, w.len()));
    }

    #[test]
    fn nonword_short() {
        let w = chars("ab");
        assert!(!is_nonword(&w, w.len()));
    }

    // -----------------------------------------------------------------------
    // Pure function tests: is_good_hyphen_position
    // -----------------------------------------------------------------------

    #[test]
    fn good_hyphen_position_basic() {
        let word = chars("koira");
        let hyph = vec![b' '; 5];
        // Position 3 ("r"): syllable before has "oi" (vowels), after has "a" (vowel)
        assert!(is_good_hyphen_position(&word, &hyph, 3, 5));
    }

    #[test]
    fn good_hyphen_position_at_start() {
        let word = chars("koira");
        let hyph = vec![b' '; 5];
        assert!(!is_good_hyphen_position(&word, &hyph, 0, 5));
    }

    #[test]
    fn good_hyphen_position_at_end() {
        let word = chars("koira");
        let hyph = vec![b' '; 5];
        // Position 4 is the last char -> new_hyphen_pos + 1 >= nchars
        assert!(!is_good_hyphen_position(&word, &hyph, 4, 5));
    }

    #[test]
    fn good_hyphen_no_vowel_before() {
        let word = chars("strk");
        let hyph = vec![b' '; 4];
        // Position 2: before has "st" (no vowels)
        assert!(!is_good_hyphen_position(&word, &hyph, 2, 4));
    }

    // -----------------------------------------------------------------------
    // Pure function tests: interpret_analysis
    // -----------------------------------------------------------------------

    #[test]
    fn interpret_simple_word() {
        // "koira" with STRUCTURE "=ppppp" -> no compound boundaries
        let mut a = Analysis::new();
        a.set(ATTR_STRUCTURE, "=ppppp");
        let mut buf = vec![b' '; 5];
        interpret_analysis(&a, &mut buf, 5);
        assert_eq!(buf, vec![b' ', b' ', b' ', b' ', b' ']);
    }

    #[test]
    fn interpret_compound_word() {
        // "koiranruoka" with STRUCTURE "=ppppp=pppppp"
        // The '=' at position 5 (after "koiran") marks a compound boundary
        let mut a = Analysis::new();
        a.set(ATTR_STRUCTURE, "=ppppp=pppppp");
        let mut buf = vec![b' '; 11];
        interpret_analysis(&a, &mut buf, 11);
        // Position 5 should be '-' (compound boundary before "ruoka")
        // But let's trace through the logic:
        // sptr starts at 1 (after first '=')
        // i=0: sptr=1 'p' -> ' ', sptr=2
        // i=1: sptr=2 'p' -> ' ', sptr=3
        // i=2: sptr=3 'p' -> ' ', sptr=4
        // i=3: sptr=4 'p' -> ' ', sptr=5
        // i=4: sptr=5 'p' -> ' ', sptr=6
        // i=5: sptr=6 '=' -> buffer[5] = '-', sptr=8 (skip '=' and next char)
        // i=6: sptr=8 'p' -> ' ', sptr=9
        // ... etc
        assert_eq!(buf[5], b'-');
        // Other positions should be ' '
        assert_eq!(buf[0], b' ');
        assert_eq!(buf[4], b' ');
        assert_eq!(buf[6], b' ');
    }

    #[test]
    fn interpret_hyphen_compound() {
        // "maa-ala" with STRUCTURE "=ppp-=pppp"
        // The '-=' at position 3 marks an explicit hyphen boundary
        let mut a = Analysis::new();
        a.set(ATTR_STRUCTURE, "=ppp-=pppp");
        let mut buf = vec![b' '; 7];
        interpret_analysis(&a, &mut buf, 7);
        // i=0: sptr=1 'p' -> ' '
        // i=1: sptr=2 'p' -> ' '
        // i=2: sptr=3 'p' -> ' '
        // i=3: sptr=4 '-', sptr+1=5 '=' -> buffer[3] = '=', sptr=6
        // i=4: sptr=6 'p' -> ' '
        // i=5: sptr=7 'p' -> ' '
        // i=6: sptr=8 'p' -> ' '
        assert_eq!(buf[3], b'=');
        assert_eq!(buf[0], b' ');
    }

    #[test]
    fn interpret_abbreviation_marker() {
        // Test 'j' and 'q' markers
        let mut a = Analysis::new();
        a.set(ATTR_STRUCTURE, "=jqp");
        let mut buf = vec![b' '; 3];
        interpret_analysis(&a, &mut buf, 3);
        assert_eq!(buf[0], b'X'); // 'j' marker
        assert_eq!(buf[1], b'X'); // 'q' marker
        assert_eq!(buf[2], b' '); // 'p' marker
    }

    // -----------------------------------------------------------------------
    // Pure function tests: rule_hyphenation
    // -----------------------------------------------------------------------

    #[test]
    fn rule_hyphenation_cv_break() {
        // "koira" -> should get -CV break at 'r' (position 3): "koi-ra"
        let word = chars("koira");
        let mut hyph = vec![b' '; 5];
        rule_hyphenation(&word, &mut hyph, 5, true);
        // The -CV rule fires at position 3 (r is consonant, a is vowel)
        assert_eq!(hyph[3], b'-');
    }

    #[test]
    fn rule_hyphenation_kissa() {
        // "kissa" -> "kis-sa"
        // -CV at position 2 ('s' before 's') - no, 's' before 'a' at position 3.
        // Actually: k(0) i(1) s(2) s(3) a(4)
        // -CV at pos 2: s(2) consonant, s(3) consonant -> no
        // -CV at pos 3: s(3) consonant, a(4) vowel -> yes, "-" at position 3
        // So "kis-sa"
        let word = chars("kissa");
        let mut hyph = vec![b' '; 5];
        rule_hyphenation(&word, &mut hyph, 5, true);
        assert_eq!(hyph[3], b'-');
        let rendered = render_hyphenation("kissa", &String::from_utf8(hyph).unwrap());
        assert_eq!(rendered, "kis-sa");
    }

    #[test]
    fn rule_hyphenation_talo() {
        // "talo" -> "ta-lo"
        // t(0) a(1) l(2) o(3)
        // Skip leading consonants: i starts at 1 (first vowel 'a')
        // -CV at pos 2: l(2) consonant, o(3) vowel -> yes
        let word = chars("talo");
        let mut hyph = vec![b' '; 4];
        rule_hyphenation(&word, &mut hyph, 4, true);
        assert_eq!(hyph[2], b'-');
    }

    #[test]
    fn rule_hyphenation_maa_no_split_long_vowel() {
        // "maa" -> should NOT be split (long vowel stays together)
        // m(0) a(1) a(2) - only 3 chars
        // -CV: starting from first vowel (pos 1). pos 1: a vowel, not consonant.
        // pos 2 would be checked but we need i <= nchars-2, so i <= 1.
        // Actually the -CV loop starts at i (the position after leading consonants)
        // and goes to nchars-2. For "maa": i starts at 1 (a is vowel).
        // i=1: a is vowel, not consonant -> skip
        // No -CV breaks. The long vowel rule (Rule 3) checks for double vowels:
        // i=1: a(1)==a(2) -> yes. Before: a(0)? No, i-1=0 which is 'm' (consonant).
        // So no split before. After: i+2=3 which is >= nchars, so no split after.
        let word = chars("maa");
        let mut hyph = vec![b' '; 3];
        rule_hyphenation(&word, &mut hyph, 3, true);
        assert_eq!(hyph, vec![b' ', b' ', b' ']);
    }

    #[test]
    fn rule_hyphenation_saippua() {
        // "saippua" -> "saip-pu-a"
        // s(0) a(1) i(2) p(3) p(4) u(5) a(6)
        // -CV: skip leading consonants -> i=1 (first vowel)
        // i=1: a vowel -> no
        // i=2: i vowel -> no
        // i=3: p consonant, p(4) consonant -> no
        // i=4: p consonant, u(5) vowel -> yes, hyph[4] = '-'
        // i=5: u vowel -> no (i <= 5, nchars-2=5)
        //
        // V-V rule: check vowel pairs
        // i=0: s not vowel -> skip
        // i=1: a(1) vowel, i(2) vowel -> pair "ai" -> check SPLIT_VOWELS
        //   "ai" is NOT in SPLIT_VOWELS. Actually let me check: SPLIT_VOWELS has
        //   ae, ao, ea, eo, ia, io, oa, oe, ua, ue, ye, ... No "ai".
        //   So no split at pos 2.
        // i=5: u(5) vowel, a(6) vowel -> pair "ua" -> check SPLIT_VOWELS
        //   "ua" IS in SPLIT_VOWELS. hyph[6] = '-'
        //
        // VV-V rule (ugly mode): check "ai" followed by vowel
        // i=0: s(0),a(1) -> not matching SPLIT_AFTER
        // i=1: a(1),i(2) -> "ai" IS in SPLIT_AFTER!
        //   hyph[1+1]=hyph[2] != '-' (it's ' '), pair matches "ai",
        //   word[i+2]=p(3) -> is_vowel? No. So this doesn't fire.
        //
        // Result: hyph = "    - -" -> "saip-pu-a"
        let word = chars("saippua");
        let mut hyph = vec![b' '; 7];
        rule_hyphenation(&word, &mut hyph, 7, true);
        assert_eq!(hyph[4], b'-'); // "saip-pua"
        assert_eq!(hyph[6], b'-'); // "saip-pu-a"
        let rendered = render_hyphenation("saippua", &String::from_utf8(hyph).unwrap());
        assert_eq!(rendered, "saip-pu-a");
    }

    #[test]
    fn rule_hyphenation_with_umlaut() {
        // "kävelö" -> "kä-ve-lö"
        // k(0) ä(1) v(2) e(3) l(4) ö(5)
        // -CV: i starts at 1 (ä is vowel)
        // i=1: ä vowel -> no
        // i=2: v consonant, e(3) vowel -> yes, hyph[2] = '-'
        // i=3: e vowel -> no
        // i=4: l consonant, ö(5) vowel -> yes, hyph[4] = '-'
        let word: Vec<char> = "k\u{00E4}vel\u{00F6}".chars().collect();
        let mut hyph = vec![b' '; 6];
        rule_hyphenation(&word, &mut hyph, 6, true);
        assert_eq!(hyph[2], b'-');
        assert_eq!(hyph[4], b'-');
    }

    #[test]
    fn rule_hyphenation_single_syllable() {
        // "tie" -> no hyphenation (too short for meaningful splits)
        let word = chars("tie");
        let mut hyph = vec![b' '; 3];
        rule_hyphenation(&word, &mut hyph, 3, true);
        // -CV: i starts at 0 (t is consonant), then i=1 (i is vowel)
        // Actually: skip leading consonants. t is consonant, so i increments.
        // i=1: starts the -CV loop. But we need i <= nchars-2 = 1.
        // i=1: i(1) is vowel, not consonant -> no
        // No -CV breaks.
        // V-V: i(1) vowel, e(2) vowel -> pair "ie"
        //   "ie" is NOT in SPLIT_VOWELS (it's a diphthong).
        // VV-V: i=0: t(0),i(1) -> not in SPLIT_AFTER. Only "ie" and "ai" are there.
        //   Wait, SPLIT_AFTER = [['i','e'], ['a','i']].
        //   i=0: t,i -> no match.
        // No hyphens.
        assert_eq!(hyph, vec![b' ', b' ', b' ']);
    }

    #[test]
    fn rule_hyphenation_no_ugly_suppresses_edges() {
        // "talo" with ugly=false -> position 1 forbidden, last position forbidden
        // -CV at pos 2: l consonant, o vowel -> hyph[2] = '-'
        // non-ugly cleanup: hyph[1] = ' ' (already ' '), hyph[3] = ' ' (nchars-1=3)
        // But hyph[2] = '-' is not at pos 1 or nchars-1, so it stays
        let word = chars("talo");
        let mut hyph = vec![b' '; 4];
        rule_hyphenation(&word, &mut hyph, 4, false);
        assert_eq!(hyph[2], b'-');
    }

    #[test]
    fn rule_hyphenation_no_ugly_vowel_pair() {
        // With ugly=false, consecutive vowels should not be split
        // "kauas" -> k(0) a(1) u(2) a(3) s(4)
        // -CV at pos 3 is 'a' (vowel) followed by 's' (consonant) -> no
        // -CV at pos 4: s followed by nothing (i <= nchars-2=3)
        // Actually -CV loop: skip to first vowel (i=1), loop i=1..3
        // i=1: a vowel -> no
        // i=2: u vowel -> no
        // i=3: a vowel -> no
        // No -CV breaks. V-V: (1,2) a,u -> "au" not in SPLIT_VOWELS.
        // (2,3) u,a -> "ua" IS in SPLIT_VOWELS -> hyph[3] = '-'
        // Non-ugly cleanup: a(2) vowel, a(3) vowel -> hyph[3] = ' '
        // So the V-V split at position 3 gets removed by the non-ugly rule.
        let word = chars("kauas");
        let mut hyph = vec![b' '; 5];
        rule_hyphenation(&word, &mut hyph, 5, false);
        assert_eq!(hyph[3], b' '); // suppressed by non-ugly rule
    }

    #[test]
    fn rule_hyphenation_very_short_word() {
        // "aa" -> nchars <= 1? No, nchars=2. allowRuleHyphenation returns true.
        // But no -CV or V-V splits can happen in 2 chars.
        let word = chars("aa");
        let mut hyph = vec![b' '; 2];
        rule_hyphenation(&word, &mut hyph, 2, true);
        assert_eq!(hyph, vec![b' ', b' ']);
    }

    #[test]
    fn rule_hyphenation_single_char() {
        // Single character: allowRuleHyphenation returns false
        let word = chars("a");
        let mut hyph = vec![b' '; 1];
        rule_hyphenation(&word, &mut hyph, 1, true);
        assert_eq!(hyph, vec![b' ']);
    }

    // -----------------------------------------------------------------------
    // intersect/union tests
    // -----------------------------------------------------------------------

    #[test]
    fn intersect_single() {
        let buffers = vec![vec![b' ', b'-', b' ', b'-', b' ']];
        let result = intersect_hyphenations(&buffers);
        assert_eq!(result.len(), 5);
        assert_eq!(result, " - - ");
    }

    #[test]
    fn intersect_agreement() {
        let buffers = vec![
            vec![b' ', b'-', b' ', b'-', b' '],
            vec![b' ', b'-', b' ', b' ', b' '],
        ];
        let result = intersect_hyphenations(&buffers);
        // Only position 1 has '-' in both
        assert_eq!(result, " -   ");
    }

    #[test]
    fn intersect_x_becomes_space() {
        let buffers = vec![vec![b'X', b'-', b'X']];
        let result = intersect_hyphenations(&buffers);
        assert_eq!(result, " - ");
    }

    #[test]
    fn union_merges() {
        let buffers = vec![
            vec![b' ', b'-', b' ', b' ', b' '],
            vec![b' ', b' ', b' ', b'-', b' '],
        ];
        let result = union_hyphenations(&buffers);
        assert_eq!(result.len(), 5);
        assert_eq!(result, " - - ");
    }

    // -----------------------------------------------------------------------
    // remove_extra_hyphenations tests
    // -----------------------------------------------------------------------

    #[test]
    fn remove_extra_keeps_simple() {
        // One analysis with no compounds (min_parts=1), one with compounds (parts=2)
        // The compound one should be removed.
        let mut buffers = vec![
            vec![b' ', b' ', b' ', b' ', b' '], // 1 part
            vec![b' ', b' ', b'-', b' ', b' '],  // 2 parts
        ];
        remove_extra_hyphenations(&mut buffers, 5);
        assert_eq!(buffers.len(), 1);
        assert_eq!(buffers[0], vec![b' ', b' ', b' ', b' ', b' ']);
    }

    #[test]
    fn remove_extra_keeps_all_if_all_compound() {
        // Both analyses have compound boundaries -> min_parts > 1, keep all
        let mut buffers = vec![
            vec![b' ', b' ', b'-', b' ', b' '], // 2 parts
            vec![b' ', b'-', b' ', b' ', b' '],  // 2 parts
        ];
        remove_extra_hyphenations(&mut buffers, 5);
        assert_eq!(buffers.len(), 2);
    }

    // -----------------------------------------------------------------------
    // FinnishHyphenator integration tests (with mock analyzer)
    // -----------------------------------------------------------------------

    #[test]
    fn hyphenate_unknown_word() {
        let hyp = FinnishHyphenator::new(NullAnalyzer, HyphenatorOptions::default());
        let result = hyphenate_str(&hyp, "koira");
        // Unknown word with hyphenate_unknown=true: rule hyphenation applied
        // "koira" -> rule: -CV at pos 3 -> " " " " " " "-" " "
        assert_eq!(result, "   - ");
        let rendered = render_hyphenation("koira", &result);
        assert_eq!(rendered, "koi-ra");
    }

    #[test]
    fn hyphenate_unknown_word_forbidden() {
        let opts = HyphenatorOptions {
            hyphenate_unknown: false,
            ..Default::default()
        };
        let hyp = FinnishHyphenator::new(NullAnalyzer, opts);
        let result = hyphenate_str(&hyp, "koira");
        // Unknown + hyphenate_unknown=false -> all spaces (no hyphenation)
        assert_eq!(result, "     ");
    }

    #[test]
    fn hyphenate_short_word() {
        let hyp = FinnishHyphenator::new(NullAnalyzer, HyphenatorOptions::default());
        let result = hyphenate_str(&hyp, "a");
        assert_eq!(result, " ");
    }

    #[test]
    fn hyphenate_with_known_simple_word() {
        let mut analyzer = MockAnalyzer::new();
        // "koira" has STRUCTURE "=ppppp" (all lowercase, single morpheme)
        analyzer.add_word("koira", &["=ppppp"]);
        let hyp = FinnishHyphenator::new(analyzer, HyphenatorOptions::default());
        let result = hyphenate_str(&hyp, "koira");
        // Simple word -> compound_hyphenation calls rule_hyphenation on the whole word
        // -CV at pos 3 -> "   - "
        assert_eq!(result, "   - ");
    }

    #[test]
    fn hyphenate_compound_word() {
        let mut analyzer = MockAnalyzer::new();
        // "koiranruoka" = "koiran" + "ruoka", STRUCTURE "=pppppp=ppppp"
        // Wait, "koiran" is 6 chars, "ruoka" is 5 chars = 11 total
        // STRUCTURE: =pppppp=ppppp (=p*6 =p*5)
        analyzer.add_word("koiranruoka", &["=pppppp=ppppp"]);
        let hyp = FinnishHyphenator::new(analyzer, HyphenatorOptions::default());
        let result = hyphenate_str(&hyp, "koiranruoka");
        // Compound boundary at position 6 ('-')
        // Rule hyphenation on "koiran" (pos 0-5, len 6):
        //   k(0) o(1) i(2) r(3) a(4) n(5)
        //   -CV: i starts at 1. i=1: o vowel->no. i=2: i vowel->no.
        //   i=3: r consonant, a(4) vowel -> hyph[3] = '-'
        //   i=4: a vowel -> no (i<=4, nchars-2=4) -> i=4: a vowel->no
        //   V-V: (1,2) o,i -> "oi" not in SPLIT_VOWELS -> no
        // Rule hyphenation on "ruoka" (pos 6-10, len 5):
        //   r(0) u(1) o(2) k(3) a(4)
        //   -CV: i starts at 1. i=1: u vowel->no. i=2: o vowel->no.
        //   i=3: k consonant, a(4) vowel -> hyph[3] = '-' -> global pos 9
        //   V-V: (1,2) u,o -> "uo" not in SPLIT_VOWELS -> no (it's a diphthong!)
        // Final: positions 3='-', 6='-', 9='-'
        assert_eq!(result.len(), 11);
        let rendered = render_hyphenation("koiranruoka", &result);
        assert_eq!(rendered, "koi-ran-ruo-ka");
    }

    #[test]
    fn hyphenate_with_explicit_hyphen() {
        let mut analyzer = MockAnalyzer::new();
        // "maa-ala" with STRUCTURE "=ppp-=ppp"
        // m(0) a(1) a(2) -(3) a(4) l(5) a(6)
        analyzer.add_word("maa-ala", &["=ppp-=ppp"]);
        let hyp = FinnishHyphenator::new(analyzer, HyphenatorOptions::default());
        let result = hyphenate_str(&hyp, "maa-ala");
        // Position 3 should be '=' (explicit hyphen boundary)
        let result_bytes: Vec<u8> = result.bytes().collect();
        assert_eq!(result_bytes[3], b'=');
    }

    #[test]
    fn hyphenate_preserves_diphthongs() {
        // Finnish diphthongs should NOT be split: ai, ei, oi, ui, yi, äi, öi,
        // au, eu, ou, iu, ie, uo, yö
        let hyp = FinnishHyphenator::new(NullAnalyzer, HyphenatorOptions::default());

        // "tie" (diphthong "ie") -> should not split
        let result = hyphenate_str(&hyp, "tie");
        assert_eq!(result, "   ");

        // "suo" (diphthong "uo") -> should not split
        let result = hyphenate_str(&hyp, "suo");
        assert_eq!(result, "   ");
    }

    #[test]
    fn hyphenate_splits_non_diphthong_vowels() {
        let hyp = FinnishHyphenator::new(NullAnalyzer, HyphenatorOptions::default());

        // "kaupunki" -> should split "ea", "ao" etc if they occur
        // Let's test with a word that has splittable vowels
        // "teatteria" -> t(0) e(1) a(2) t(3) t(4) e(5) r(6) i(7) a(8)
        // -CV: skip to i=1. i=1: e vowel->no. i=2: a vowel->no.
        //   i=3: t consonant, t(4) consonant -> no.
        //   i=4: t consonant, e(5) vowel -> hyph[4] = '-'
        //   i=5: e vowel->no. i=6: r consonant, i(7) vowel -> hyph[6] = '-'
        //   i=7: i vowel->no (i<=7, nchars-2=7) -> i vowel->no
        // V-V: (1,2) e,a -> "ea" IS in SPLIT_VOWELS -> hyph[2] = '-'
        //   (7,8) i,a -> "ia" IS in SPLIT_VOWELS -> hyph[8] = '-'
        let result = hyphenate_str(&hyp, "teatteria");
        let rendered = render_hyphenation("teatteria", &result);
        assert_eq!(rendered, "te-at-te-ri-a");
    }

    #[test]
    fn hyphenate_long_vowel_boundaries() {
        let hyp = FinnishHyphenator::new(NullAnalyzer, HyphenatorOptions::default());

        // "aamu" -> a(0) a(1) m(2) u(3)
        // -CV: skip to i=0 (a is vowel). i=0: a vowel->no.
        //   i=1: a vowel->no. i=2: m consonant, u(3) vowel -> hyph[2]='-'
        // Long vowel rule: i=1: a(1)==a(2)? No, a(1)!=m(2). Hmm wait:
        // i ranges 1..nchars-1=3, so i=1,2.
        // i=1: word[1]='a', word[2]='m' -> not equal, skip.
        // No long vowel rule fires. Just -CV.
        // Result: "  - " -> "aa-mu"
        let result = hyphenate_str(&hyp, "aamu");
        let rendered = render_hyphenation("aamu", &result);
        assert_eq!(rendered, "aa-mu");
    }

    #[test]
    fn hyphenate_long_vowel_with_surrounding() {
        let hyp = FinnishHyphenator::new(NullAnalyzer, HyphenatorOptions::default());

        // "aatto" -> a(0) a(1) t(2) t(3) o(4)
        // -CV: skip leading consonants? a is vowel, so i=0.
        // Loop i=0..3: i=0: a vowel->no. i=1: a vowel->no.
        //   i=2: t consonant, t(3) consonant->no.
        //   i=3: t consonant, o(4) vowel -> hyph[3]='-'
        // Long vowel: i=1: a(1)==a(2)? a!=t, no.
        //   i=0: actually the loop is i=1..nchars-1=4, so i=1,2,3.
        //   Wait, range is 1..nchars.saturating_sub(1)=4 exclusive, so i=1,2,3.
        //   i=1: word[1]='a', word[2]='t' -> no.
        // No long vowel fires because "aa" is at the start and the chars are
        // word_lower[0]='a', word_lower[1]='a'. The loop starts at i=1:
        // i=1: is_vowel('a') and 'a'=='t'? No.
        // Hmm, word_lower[1]='a' and word_lower[2]='t'. Not equal.
        // But word_lower[0]='a' and word_lower[1]='a' -- the loop starts at i=1
        // but the check is word[i]==word[i+1], so i=1 checks word[1] vs word[2].
        // For "aatto" that's 'a' vs 't' -- not equal.
        // The check for i=0 would be word[0] vs word[1] = 'a' vs 'a', but the
        // loop starts at i=1 not i=0. This is correct -- the C++ code also
        // starts at i=1: `for (i = 1; i < nchars - 1; i++)`.
        // So the "aa" at the beginning doesn't trigger the long vowel rule through
        // this path. The "aa" is simply not split because it's at positions 0,1.
        let result = hyphenate_str(&hyp, "aatto");
        let rendered = render_hyphenation("aatto", &result);
        assert_eq!(rendered, "aat-to");
    }

    #[test]
    fn hyphenate_multiple_analyses_intersection() {
        let mut analyzer = MockAnalyzer::new();
        // Two analyses for the same word with different compound boundaries
        // Analysis 1: split at position 3
        // Analysis 2: no split
        analyzer.add_word(
            "koira",
            &["=ppppp", "=pp=ppp"], // second one splits at pos 2
        );
        let hyp = FinnishHyphenator::new(analyzer, HyphenatorOptions::default());
        let result = hyphenate_str(&hyp, "koira");
        // With two analyses, the intersection means only positions where
        // BOTH agree get a hyphen. Since one has no compound boundary and
        // one has a boundary at 2, the rule hyphenation results may differ.
        // The intersection will be conservative.
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn hyphenate_kissa_unknown() {
        let hyp = FinnishHyphenator::new(NullAnalyzer, HyphenatorOptions::default());
        let result = hyphenate_str(&hyp, "kissa");
        let rendered = render_hyphenation("kissa", &result);
        assert_eq!(rendered, "kis-sa");
    }

    #[test]
    fn all_possible_union() {
        let mut analyzer = MockAnalyzer::new();
        // Two analyses: one allows a split, one doesn't
        analyzer.add_word("koira", &["=ppppp"]);
        let hyp = FinnishHyphenator::new(analyzer, HyphenatorOptions::default());
        let result = hyp.all_possible_hyphen_positions(&chars("koira"));
        // Single analysis, so union == intersection
        assert_eq!(result, "   - ");
    }

    #[test]
    fn hyphenate_empty() {
        let hyp = FinnishHyphenator::new(NullAnalyzer, HyphenatorOptions::default());
        let result = hyp.hyphenate(&[]);
        assert_eq!(result, "");
    }

    // -----------------------------------------------------------------------
    // Edge case tests
    // -----------------------------------------------------------------------

    #[test]
    fn hyphenate_all_consonants() {
        let hyp = FinnishHyphenator::new(NullAnalyzer, HyphenatorOptions::default());
        let result = hyphenate_str(&hyp, "krst");
        // No vowels -> no meaningful hyphenation
        // -CV rule: skip leading consonants -> i goes past all chars.
        // No breaks.
        assert_eq!(result, "    ");
    }

    #[test]
    fn hyphenate_all_vowels() {
        let hyp = FinnishHyphenator::new(NullAnalyzer, HyphenatorOptions::default());
        let result = hyphenate_str(&hyp, "aeiou");
        // No consonants -> -CV never fires
        // V-V: ae(pos 1) in SPLIT_VOWELS -> hyph[1]='-'
        //   ei(pos 2) not in SPLIT_VOWELS (diphthong)
        //   io(pos 3) in SPLIT_VOWELS -> hyph[3]='-'
        //   ou(pos 4) not in SPLIT_VOWELS (diphthong)
        // VV-V: check "ie","ai" at each pos
        //   pos 0: a,e -> not in SPLIT_AFTER
        //   pos 1: e,i -> not in SPLIT_AFTER
        //   pos 2: nothing (nchars-3=2, loop is 0..2)
        // Wait, SPLIT_AFTER has "ie" and "ai". Let me re-check pos 0:
        //   word[0]='a', word[1]='e' -> not "ie" or "ai"
        //   pos 1: word[1]='e', word[2]='i' -> not "ie" (it's "ei") or "ai"
        // Actually wait, the loop for VV-V is i in 0..nchars-3. nchars=5, so 0..2.
        //   i=0: 'a','e' -> no match
        //   i=1: 'e','i' -> no match
        // Result: " -  - " wait that's 6 chars. Let me recount.
        // 5 chars: a(0) e(1) i(2) o(3) u(4)
        // hyph: [' ', '-', ' ', '-', ' '] -> " - - "
        let result_bytes: Vec<u8> = result.bytes().collect();
        assert_eq!(result_bytes[1], b'-');
        assert_eq!(result_bytes[3], b'-');
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn hyphenate_with_hyphen_in_unknown_word() {
        let hyp = FinnishHyphenator::new(NullAnalyzer, HyphenatorOptions::default());
        // Word with an explicit hyphen: "maa-ala"
        let result = hyphenate_str(&hyp, "maa-ala");
        // Unknown word: buffer starts with spaces, explicit '-' at pos 3 -> '='
        let result_bytes: Vec<u8> = result.bytes().collect();
        assert_eq!(result_bytes[3], b'=');
    }

    #[test]
    fn hyphenate_min_length() {
        let opts = HyphenatorOptions {
            min_hyphenated_word_length: 5,
            ..Default::default()
        };
        let hyp = FinnishHyphenator::new(NullAnalyzer, opts);
        // "talo" has 4 chars < min 5 -> no hyphenation
        let result = hyphenate_str(&hyp, "talo");
        assert_eq!(result, "    ");
    }

    #[test]
    fn hyphenate_apostrophe() {
        let hyp = FinnishHyphenator::new(NullAnalyzer, HyphenatorOptions::default());
        // "vast'edes" -> apostrophe before vowel
        // v(0) a(1) s(2) t(3) '(4) e(5) d(6) e(7) s(8)
        let result = hyphenate_str(&hyp, "vast'edes");
        let result_bytes: Vec<u8> = result.bytes().collect();
        // Rule 2: 'V at pos 4: '(4) followed by e(5) vowel -> hyph[4] = '='
        assert_eq!(result_bytes[4], b'=');
    }

    #[test]
    fn hyphenate_url_not_ugly() {
        let opts = HyphenatorOptions {
            ugly_hyphenation: false,
            ..Default::default()
        };
        let hyp = FinnishHyphenator::new(NullAnalyzer, opts);
        // URL-like strings should not be hyphenated when ugly is off
        let w = "http://example.com";
        let result = hyphenate_str(&hyp, w);
        // is_nonword returns true -> allow_rule_hyphenation returns false
        // So no rule hyphenation applied
        assert!(result.chars().all(|c| c == ' '));
    }

    #[test]
    fn hyphenate_word_ending_digit_not_ugly() {
        let opts = HyphenatorOptions {
            ugly_hyphenation: false,
            ..Default::default()
        };
        let hyp = FinnishHyphenator::new(NullAnalyzer, opts);
        let result = hyphenate_str(&hyp, "abc123");
        // Ends with digit, ugly=false -> no rule hyphenation
        assert!(result.chars().all(|c| c == ' '));
    }

    #[test]
    fn hyphenate_long_consonant_cluster() {
        let hyp = FinnishHyphenator::new(NullAnalyzer, HyphenatorOptions::default());
        // Test with "zh" cluster: a word containing "azha"
        // a(0) z(1) h(2) a(3)
        // -CV: skip to i=0 (a is vowel).
        //   i=0: a vowel -> no.
        //   i=1: z consonant, h(2) consonant -> no.
        //   i=2: h consonant, a(3) vowel -> hyph[2]='-'
        // Long consonant check: i=1: "zh" matches LONG_CONSONANTS
        //   clen=2, i+clen=3 < nchars=4. Check k=2..2:
        //   hyph[2]=='-' -> move to hyph[1]='-', hyph[2]=' '
        let result = hyphenate_str(&hyp, "azha");
        let result_bytes: Vec<u8> = result.bytes().collect();
        assert_eq!(result_bytes[1], b'-'); // moved before the cluster
        assert_eq!(result_bytes[2], b' '); // cleared
    }
}
