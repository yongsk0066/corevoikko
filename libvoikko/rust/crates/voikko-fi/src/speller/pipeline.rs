// Top-level spell check pipeline
// Origin: spellchecker/spell.cpp

use voikko_core::case::{detect_case, CaseType};
use voikko_core::character::{is_upper, simple_lower};
use voikko_core::enums::{SpellResult, MAX_WORD_CHARS};

use crate::speller::cache::SpellerCache;
use crate::speller::Speller;

/// Public API spell result values.
///
/// Origin: voikko_defines.h:49-53
const VOIKKO_SPELL_OK: i32 = 1;
const VOIKKO_SPELL_FAILED: i32 = 0;

/// Spell check options controlling how words are evaluated.
///
/// Origin: VoikkoHandle fields
#[derive(Debug, Clone)]
pub struct SpellOptions {
    /// Ignore dot at the end of the word.
    pub ignore_dot: bool,
    /// Ignore words containing digits.
    pub ignore_numbers: bool,
    /// Accept words written entirely in uppercase without checking.
    pub ignore_uppercase: bool,
    /// Accept non-words (URLs, email addresses, etc.).
    pub ignore_nonwords: bool,
    /// Accept words where the first letter is uppercase.
    pub accept_first_uppercase: bool,
    /// Accept words where all letters are uppercase (still checks spelling).
    pub accept_all_uppercase: bool,
    /// Accept missing hyphens at start/end of word.
    pub accept_missing_hyphens: bool,
}

impl Default for SpellOptions {
    /// Default option values matching C++ VoikkoHandle defaults.
    fn default() -> Self {
        Self {
            ignore_dot: false,
            ignore_numbers: false,
            ignore_uppercase: false,
            ignore_nonwords: true,
            accept_first_uppercase: true,
            accept_all_uppercase: true,
            accept_missing_hyphens: false,
        }
    }
}

/// Check whether a word is a non-word (URL or email pattern).
///
/// Non-word patterns:
/// - `X*//X*.X+` (URL-like)
/// - `X*@X+.X+` (email-like)
/// - `www.X+.X+` (www prefix)
///
/// Origin: utils/utils.cpp:94-117
fn is_nonword(word: &[char]) -> bool {
    let nchars = word.len();
    if nchars < 4 {
        return false;
    }

    // Check for "//" followed by "." -> URL
    if let Some(slash_pos) = word[..nchars.saturating_sub(3)]
        .iter()
        .position(|&c| c == '/')
    {
        if word[slash_pos + 1] == '/' && word[slash_pos + 2..].contains(&'.') {
            return true;
        }
    }

    // Check for "@" with "." after -> email
    if let Some(at_pos) = word.iter().position(|&c| c == '@') {
        if at_pos > 0 && word[at_pos + 1..].contains(&'.') {
            return true;
        }
    }

    // Check for "www." prefix
    if nchars >= 5
        && simple_lower(word[0]) == 'w'
        && simple_lower(word[1]) == 'w'
        && simple_lower(word[2]) == 'w'
        && word[3] == '.'
        && word[4..].contains(&'.')
    {
        return true;
    }

    false
}

/// Spell check with missing hyphen handling.
///
/// If the word fails and `accept_missing_hyphens` is set, tries adding
/// hyphens at the start and/or end.
///
/// Origin: spell.cpp:53-80 (hyphenAwareSpell)
fn hyphen_aware_spell(
    speller: &dyn Speller,
    word: &[char],
    len: usize,
    accept_missing_hyphens: bool,
) -> SpellResult {
    let spres = speller.spell(word, len);
    if spres != SpellResult::Failed || !accept_missing_hyphens {
        return spres;
    }

    // Hyphens already present at both ends
    if len < 2 || (word[0] == '-' && word[len - 1] == '-') {
        return SpellResult::Failed;
    }

    let mut buffer = Vec::with_capacity(len + 2);

    if word[0] == '-' {
        // Add trailing hyphen
        buffer.extend_from_slice(&word[..len]);
        buffer.push('-');
    } else {
        // Add leading hyphen
        buffer.push('-');
        buffer.extend_from_slice(&word[..len]);
        if word[len - 1] != '-' {
            // Add trailing hyphen too
            buffer.push('-');
        }
    }

    speller.spell(&buffer, buffer.len())
}

/// Cached spell check: looks up the cache first, then calls the speller.
///
/// Origin: spell.cpp:89-103
fn cached_spell(
    cache: Option<&mut SpellerCache>,
    speller: &dyn Speller,
    buffer: &[char],
    len: usize,
    accept_missing_hyphens: bool,
) -> SpellResult {
    match cache {
        Some(cache) => {
            if cache.is_in_cache(buffer, len) {
                return cache.get_spell_result(buffer, len);
            }
            let result = hyphen_aware_spell(speller, buffer, len, accept_missing_hyphens);
            cache.set_spell_result(buffer, len, result);
            result
        }
        None => hyphen_aware_spell(speller, buffer, len, accept_missing_hyphens),
    }
}

/// Simple Unicode normalization stub.
///
/// A full implementation would handle ligature decomposition and
/// character substitutions (see charset.cpp:voikko_normalise).
/// For now, this is a passthrough that copies the input as-is.
///
/// TODO: Implement full normalization matching C++ voikko_normalise.
fn normalize(word: &[char]) -> Vec<char> {
    // Basic normalization: replace some common equivalents
    let mut result = Vec::with_capacity(word.len() + word.len() / 2);
    let len = word.len();
    let mut i = 0;
    while i < len {
        match word[i] {
            // HYPHEN (U+2010) -> HYPHEN-MINUS
            '\u{2010}' => {
                result.push('-');
                i += 1;
            }
            // NON-BREAKING HYPHEN (U+2011) -> HYPHEN-MINUS
            '\u{2011}' => {
                result.push('-');
                i += 1;
            }
            // Ligature decompositions
            '\u{FB00}' => {
                result.push('f');
                result.push('f');
                i += 1;
            }
            '\u{FB01}' => {
                result.push('f');
                result.push('i');
                i += 1;
            }
            '\u{FB02}' => {
                result.push('f');
                result.push('l');
                i += 1;
            }
            '\u{FB03}' => {
                result.push('f');
                result.push('f');
                result.push('i');
                i += 1;
            }
            '\u{FB04}' => {
                result.push('f');
                result.push('f');
                result.push('l');
                i += 1;
            }
            // DEGREE CELSIUS (U+2103) -> degree sign + C
            '\u{2103}' => {
                result.push('\u{00B0}');
                result.push('C');
                i += 1;
            }
            // DEGREE FAHRENHEIT (U+2109) -> degree sign + F
            '\u{2109}' => {
                result.push('\u{00B0}');
                result.push('F');
                i += 1;
            }
            c => {
                result.push(c);
                i += 1;
            }
        }
    }
    result
}

/// Check if a character is a digit (matching C++ SimpleChar::isDigit).
fn is_digit(c: char) -> bool {
    c.is_ascii_digit()
}

/// Top-level spell check entry point.
///
/// This is the public-facing spell check function that:
/// 1. Normalizes the word
/// 2. Handles option-based bypasses (ignore_numbers, ignore_uppercase, etc.)
/// 3. Detects the case pattern
/// 4. Lowercases the word for FST lookup
/// 5. Handles trailing dot
/// 6. Dispatches to cached or direct spell check
/// 7. Maps the internal SpellResult to a public OK/FAILED result
///
/// Origin: spell.cpp:106-234 (voikkoSpellUcs4)
pub fn spell_check(
    word: &[char],
    speller: &dyn Speller,
    cache: Option<&mut SpellerCache>,
    options: &SpellOptions,
) -> i32 {
    let nchars = word.len();

    if nchars == 0 {
        return VOIKKO_SPELL_OK;
    }
    if nchars > MAX_WORD_CHARS {
        return VOIKKO_SPELL_FAILED;
    }

    // Normalize
    let nword = normalize(word);
    let nchars = nword.len();

    // Ignore words containing digits
    if options.ignore_numbers && nword.iter().any(|c| is_digit(*c)) {
        return VOIKKO_SPELL_OK;
    }

    // Detect case pattern
    let mut caps = detect_case(&nword);

    // Ignore all-uppercase words if requested
    if options.ignore_uppercase && caps == CaseType::AllUpper {
        return VOIKKO_SPELL_OK;
    }

    // Ignore non-words if requested
    if options.ignore_nonwords && is_nonword(&nword) {
        return VOIKKO_SPELL_OK;
    }

    // If all-uppercase but not accepting all-uppercase as special, treat as complex
    if caps == CaseType::AllUpper && !options.accept_all_uppercase {
        caps = CaseType::Complex;
    }

    // Lowercase the word
    let mut buffer: Vec<char> = nword.iter().map(|&c| simple_lower(c)).collect();

    // Handle trailing dot
    let dot_index: Option<usize> = if options.ignore_dot && buffer.last() == Some(&'.') {
        let idx = nchars - 1;
        buffer[idx] = '\0'; // mark as removed (won't be passed to spell)
        Some(idx)
    } else {
        None
    };

    let real_chars = match dot_index {
        Some(idx) => idx,
        None => nchars,
    };

    // --- COMPLEX / NO_LETTERS case: exact capitalization check ---
    // Origin: spell.cpp:162-184
    if caps == CaseType::Complex || caps == CaseType::NoLetters {
        // Restore original case except lowercase first character
        buffer.clear();
        buffer.extend(nword.iter().copied());
        buffer[0] = simple_lower(buffer[0]);

        let sres = hyphen_aware_spell(speller, &buffer, nchars, options.accept_missing_hyphens);
        let mut result = if sres == SpellResult::Ok
            || (sres == SpellResult::CapitalizeFirst
                && options.accept_first_uppercase
                && is_upper(nword[0]))
        {
            VOIKKO_SPELL_OK
        } else {
            VOIKKO_SPELL_FAILED
        };

        // Try without trailing dot
        if let Some(dot_idx) = dot_index.filter(|_| result == VOIKKO_SPELL_FAILED) {
            let sres = hyphen_aware_spell(
                speller,
                &buffer[..dot_idx],
                dot_idx,
                options.accept_missing_hyphens,
            );
            if sres == SpellResult::Ok
                || (sres == SpellResult::CapitalizeFirst
                    && options.accept_first_uppercase
                    && is_upper(nword[0]))
            {
                result = VOIKKO_SPELL_OK;
            }
        }

        return result;
    }

    // --- Simple cases: ALL_LOWER, FIRST_UPPER, ALL_UPPER ---
    // Origin: spell.cpp:188-208

    let sres = cached_spell(
        cache,
        speller,
        &buffer[..real_chars],
        real_chars,
        options.accept_missing_hyphens,
    );

    let mut result = map_spell_result(sres, caps, options);

    if result == VOIKKO_SPELL_OK {
        return VOIKKO_SPELL_OK;
    }

    // Retry with trailing dot if present
    // Origin: spell.cpp:211-231
    if let Some(dot_idx) = dot_index {
        buffer[dot_idx] = '.';
        let sres = cached_spell(
            // Cache was consumed by the first call; since Rust borrows are exclusive,
            // we pass None here. The C++ code reuses the same cache pointer, which
            // works because cached_spell is the same function. We handle this by
            // not using cache for the retry (acceptable trade-off: the retry is rare).
            None,
            speller,
            &buffer[..nchars],
            nchars,
            options.accept_missing_hyphens,
        );
        result = map_spell_result(sres, caps, options);
    }

    result
}

/// Map an internal SpellResult + case type to the public VOIKKO_SPELL result.
///
/// Origin: spell.cpp:189-203, 213-230
fn map_spell_result(sres: SpellResult, caps: CaseType, options: &SpellOptions) -> i32 {
    match caps {
        CaseType::AllLower => {
            if sres == SpellResult::Ok {
                VOIKKO_SPELL_OK
            } else {
                VOIKKO_SPELL_FAILED
            }
        }
        CaseType::FirstUpper => {
            if (sres == SpellResult::Ok && options.accept_first_uppercase)
                || sres == SpellResult::CapitalizeFirst
            {
                VOIKKO_SPELL_OK
            } else {
                VOIKKO_SPELL_FAILED
            }
        }
        CaseType::AllUpper => {
            if sres == SpellResult::Failed {
                VOIKKO_SPELL_FAILED
            } else {
                VOIKKO_SPELL_OK
            }
        }
        _ => VOIKKO_SPELL_FAILED, // should not happen for simple cases
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voikko_core::analysis::{Analysis, ATTR_STRUCTURE};

    use crate::morphology::Analyzer;
    use crate::speller::adapter::AnalyzerToSpellerAdapter;

    struct MockPipelineAnalyzer;

    impl MockPipelineAnalyzer {
        fn make_analysis(structure: &str) -> Analysis {
            let mut a = Analysis::new();
            a.set(ATTR_STRUCTURE, structure);
            a
        }
    }

    impl Analyzer for MockPipelineAnalyzer {
        fn analyze(&self, word: &[char], _word_len: usize) -> Vec<Analysis> {
            let s: String = word.iter().collect();
            match s.as_str() {
                "koira" => vec![Self::make_analysis("=ppppp")],
                "helsinki" => vec![Self::make_analysis("=ippppppp")],
                "eu" => vec![Self::make_analysis("=jj")],
                "1.5" => vec![], // number, not a word
                _ => vec![],
            }
        }
    }

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    fn spell_word(word: &str, options: &SpellOptions) -> i32 {
        let analyzer = MockPipelineAnalyzer;
        let adapter = AnalyzerToSpellerAdapter::new(&analyzer);
        let word_chars = chars(word);
        spell_check(&word_chars, &adapter, None, options)
    }

    fn default_options() -> SpellOptions {
        SpellOptions::default()
    }

    // --- is_nonword tests ---

    #[test]
    fn nonword_url() {
        assert!(is_nonword(&chars("http://example.com")));
    }

    #[test]
    fn nonword_email() {
        assert!(is_nonword(&chars("user@example.com")));
    }

    #[test]
    fn nonword_www() {
        assert!(is_nonword(&chars("www.example.com")));
    }

    #[test]
    fn not_nonword_regular() {
        assert!(!is_nonword(&chars("koira")));
    }

    #[test]
    fn not_nonword_short() {
        assert!(!is_nonword(&chars("abc")));
    }

    // --- normalize tests ---

    #[test]
    fn normalize_plain_text() {
        let word = chars("koira");
        let result = normalize(&word);
        assert_eq!(result, word);
    }

    #[test]
    fn normalize_hyphen_variants() {
        let word = chars("syy\u{2010}silta"); // HYPHEN (U+2010)
        let result = normalize(&word);
        assert_eq!(result, chars("syy-silta"));
    }

    #[test]
    fn normalize_ligature_ff() {
        let word = chars("\u{FB00}"); // LATIN SMALL LIGATURE FF
        let result = normalize(&word);
        assert_eq!(result, chars("ff"));
    }

    // --- Pipeline tests ---

    #[test]
    fn empty_word_is_ok() {
        assert_eq!(spell_word("", &default_options()), VOIKKO_SPELL_OK);
    }

    #[test]
    fn known_lowercase_word() {
        assert_eq!(spell_word("koira", &default_options()), VOIKKO_SPELL_OK);
    }

    #[test]
    fn unknown_word_fails() {
        assert_eq!(spell_word("xyzzy", &default_options()), VOIKKO_SPELL_FAILED);
    }

    #[test]
    fn first_upper_proper_noun() {
        assert_eq!(spell_word("Helsinki", &default_options()), VOIKKO_SPELL_OK);
    }

    #[test]
    fn all_upper_proper_noun() {
        // "HELSINKI" -> lowercased "helsinki" -> analysis has "=ippppppp"
        // SpellResult::CapitalizeFirst -> for ALL_UPPER, not failed -> OK
        assert_eq!(spell_word("HELSINKI", &default_options()), VOIKKO_SPELL_OK);
    }

    #[test]
    fn all_upper_common_word() {
        // "KOIRA" -> lowercased "koira" -> SpellResult::Ok -> for ALL_UPPER -> OK
        assert_eq!(spell_word("KOIRA", &default_options()), VOIKKO_SPELL_OK);
    }

    #[test]
    fn ignore_uppercase_returns_ok() {
        let mut opts = default_options();
        opts.ignore_uppercase = true;
        assert_eq!(spell_word("XYZZY", &opts), VOIKKO_SPELL_OK);
    }

    #[test]
    fn ignore_numbers_returns_ok_for_digits() {
        let mut opts = default_options();
        opts.ignore_numbers = true;
        assert_eq!(spell_word("abc123", &opts), VOIKKO_SPELL_OK);
    }

    #[test]
    fn ignore_nonwords_returns_ok_for_url() {
        let opts = default_options(); // ignore_nonwords is true by default
        assert_eq!(spell_word("http://example.com", &opts), VOIKKO_SPELL_OK);
    }

    #[test]
    fn first_upper_without_accept_first_uppercase() {
        let mut opts = default_options();
        opts.accept_first_uppercase = false;
        // "Koira" -> lowercased "koira" -> SpellResult::Ok
        // Without accept_first_uppercase, FIRST_UPPER requires SpellResult::CapitalizeFirst
        assert_eq!(spell_word("Koira", &opts), VOIKKO_SPELL_FAILED);
    }

    #[test]
    fn first_upper_proper_noun_without_accept_first_uppercase() {
        let mut opts = default_options();
        opts.accept_first_uppercase = false;
        // "Helsinki" -> lowercased "helsinki" -> SpellResult::CapitalizeFirst -> OK
        assert_eq!(spell_word("Helsinki", &opts), VOIKKO_SPELL_OK);
    }

    #[test]
    fn all_upper_without_accept_all_uppercase() {
        let mut opts = default_options();
        opts.accept_all_uppercase = false;
        // "KOIRA" -> caps becomes COMPLEX -> exact case check
        // buffer = "kOIRA" (lowercase first, rest original) -> won't match "=ppppp"
        // Actually the COMPLEX path lowercases first char: "kOIRA"
        // The word "kOIRA" checked against "=ppppp" -> 'O' uppercase at position 1 -> CapError
        assert_eq!(spell_word("KOIRA", &opts), VOIKKO_SPELL_FAILED);
    }

    #[test]
    fn word_exceeding_max_length_fails() {
        let long_word: String = "a".repeat(MAX_WORD_CHARS + 1);
        assert_eq!(spell_word(&long_word, &default_options()), VOIKKO_SPELL_FAILED);
    }

    #[test]
    fn all_upper_abbreviation() {
        // "EU" -> lowercased "eu" -> analysis structure "=jj" expects uppercase
        // SpellResult::CapitalizeFirst (lowercase 'e' where 'j' expected)
        // For ALL_UPPER, CapitalizeFirst != Failed -> OK
        assert_eq!(spell_word("EU", &default_options()), VOIKKO_SPELL_OK);
    }

    // --- map_spell_result tests ---

    #[test]
    fn map_all_lower_ok() {
        let opts = default_options();
        assert_eq!(
            map_spell_result(SpellResult::Ok, CaseType::AllLower, &opts),
            VOIKKO_SPELL_OK
        );
    }

    #[test]
    fn map_all_lower_cap_first_fails() {
        let opts = default_options();
        assert_eq!(
            map_spell_result(SpellResult::CapitalizeFirst, CaseType::AllLower, &opts),
            VOIKKO_SPELL_FAILED
        );
    }

    #[test]
    fn map_first_upper_cap_first_ok() {
        let opts = default_options();
        assert_eq!(
            map_spell_result(SpellResult::CapitalizeFirst, CaseType::FirstUpper, &opts),
            VOIKKO_SPELL_OK
        );
    }

    #[test]
    fn map_first_upper_ok_with_accept() {
        let opts = default_options();
        assert_eq!(
            map_spell_result(SpellResult::Ok, CaseType::FirstUpper, &opts),
            VOIKKO_SPELL_OK
        );
    }

    #[test]
    fn map_first_upper_ok_without_accept() {
        let mut opts = default_options();
        opts.accept_first_uppercase = false;
        assert_eq!(
            map_spell_result(SpellResult::Ok, CaseType::FirstUpper, &opts),
            VOIKKO_SPELL_FAILED
        );
    }

    #[test]
    fn map_all_upper_ok() {
        let opts = default_options();
        assert_eq!(
            map_spell_result(SpellResult::Ok, CaseType::AllUpper, &opts),
            VOIKKO_SPELL_OK
        );
    }

    #[test]
    fn map_all_upper_cap_first_ok() {
        let opts = default_options();
        assert_eq!(
            map_spell_result(SpellResult::CapitalizeFirst, CaseType::AllUpper, &opts),
            VOIKKO_SPELL_OK
        );
    }

    #[test]
    fn map_all_upper_failed() {
        let opts = default_options();
        assert_eq!(
            map_spell_result(SpellResult::Failed, CaseType::AllUpper, &opts),
            VOIKKO_SPELL_FAILED
        );
    }

    // --- hyphen_aware_spell tests ---

    #[test]
    fn hyphen_aware_no_missing_hyphens() {
        struct OkSpeller;
        impl Speller for OkSpeller {
            fn spell(&self, _word: &[char], _wlen: usize) -> SpellResult {
                SpellResult::Ok
            }
        }
        let word = chars("koira");
        assert_eq!(
            hyphen_aware_spell(&OkSpeller, &word, word.len(), false),
            SpellResult::Ok
        );
    }

    #[test]
    fn hyphen_aware_with_missing_hyphens() {
        // Speller that only accepts words starting and ending with '-'
        struct HyphenSpeller;
        impl Speller for HyphenSpeller {
            fn spell(&self, word: &[char], wlen: usize) -> SpellResult {
                if wlen >= 2 && word[0] == '-' && word[wlen - 1] == '-' {
                    SpellResult::Ok
                } else {
                    SpellResult::Failed
                }
            }
        }
        let word = chars("koira");
        assert_eq!(
            hyphen_aware_spell(&HyphenSpeller, &word, word.len(), true),
            SpellResult::Ok
        );
    }
}
