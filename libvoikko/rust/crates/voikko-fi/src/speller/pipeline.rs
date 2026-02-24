// Top-level spell check pipeline
// Origin: spellchecker/spell.cpp

use voikko_core::case::{CaseType, detect_case};
use voikko_core::character::{is_upper, simple_lower};
use voikko_core::enums::{MAX_WORD_CHARS, SpellResult};

use crate::speller::Speller;
use crate::speller::cache::SpellerCache;

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

/// 2-to-1 combining diacritical mark composition table.
///
/// Each entry is `(base, combining_mark, precomposed)`.
/// Origin: charset.cpp CONV_2TO1 (67 entries)
const CONV_2TO1: [(char, char, char); 67] = [
    // Basic Latin + Combining Diacritical Marks --> Latin-1 Supplement
    ('A', '\u{0300}', '\u{00C0}'), // LATIN CAPITAL LETTER A WITH GRAVE
    ('A', '\u{0301}', '\u{00C1}'), // LATIN CAPITAL LETTER A WITH ACUTE
    ('A', '\u{0302}', '\u{00C2}'), // LATIN CAPITAL LETTER A WITH CIRCUMFLEX
    ('A', '\u{0303}', '\u{00C3}'), // LATIN CAPITAL LETTER A WITH TILDE
    ('A', '\u{0308}', '\u{00C4}'), // LATIN CAPITAL LETTER A WITH DIAERESIS
    ('A', '\u{030A}', '\u{00C5}'), // LATIN CAPITAL LETTER A WITH RING ABOVE
    ('C', '\u{0327}', '\u{00C7}'), // LATIN CAPITAL LETTER C WITH CEDILLA
    ('E', '\u{0300}', '\u{00C8}'), // LATIN CAPITAL LETTER E WITH GRAVE
    ('E', '\u{0301}', '\u{00C9}'), // LATIN CAPITAL LETTER E WITH ACUTE
    ('E', '\u{0302}', '\u{00CA}'), // LATIN CAPITAL LETTER E WITH CIRCUMFLEX
    ('E', '\u{0308}', '\u{00CB}'), // LATIN CAPITAL LETTER E WITH DIAERESIS
    ('I', '\u{0300}', '\u{00CC}'), // LATIN CAPITAL LETTER I WITH GRAVE
    ('I', '\u{0301}', '\u{00CD}'), // LATIN CAPITAL LETTER I WITH ACUTE
    ('I', '\u{0302}', '\u{00CE}'), // LATIN CAPITAL LETTER I WITH CIRCUMFLEX
    ('I', '\u{0308}', '\u{00CF}'), // LATIN CAPITAL LETTER I WITH DIAERESIS
    ('N', '\u{0303}', '\u{00D1}'), // LATIN CAPITAL LETTER N WITH TILDE
    ('O', '\u{0300}', '\u{00D2}'), // LATIN CAPITAL LETTER O WITH GRAVE
    ('O', '\u{0301}', '\u{00D3}'), // LATIN CAPITAL LETTER O WITH ACUTE
    ('O', '\u{0302}', '\u{00D4}'), // LATIN CAPITAL LETTER O WITH CIRCUMFLEX
    ('O', '\u{0303}', '\u{00D5}'), // LATIN CAPITAL LETTER O WITH TILDE
    ('O', '\u{0308}', '\u{00D6}'), // LATIN CAPITAL LETTER O WITH DIAERESIS
    ('U', '\u{0300}', '\u{00D9}'), // LATIN CAPITAL LETTER U WITH GRAVE
    ('U', '\u{0301}', '\u{00DA}'), // LATIN CAPITAL LETTER U WITH ACUTE
    ('U', '\u{0302}', '\u{00DB}'), // LATIN CAPITAL LETTER U WITH CIRCUMFLEX
    ('U', '\u{0308}', '\u{00DC}'), // LATIN CAPITAL LETTER U WITH DIAERESIS
    ('Y', '\u{0301}', '\u{00DD}'), // LATIN CAPITAL LETTER Y WITH ACUTE
    ('a', '\u{0300}', '\u{00E0}'), // LATIN SMALL LETTER A WITH GRAVE
    ('a', '\u{0301}', '\u{00E1}'), // LATIN SMALL LETTER A WITH ACUTE
    ('a', '\u{0302}', '\u{00E2}'), // LATIN SMALL LETTER A WITH CIRCUMFLEX
    ('a', '\u{0303}', '\u{00E3}'), // LATIN SMALL LETTER A WITH TILDE
    ('a', '\u{0308}', '\u{00E4}'), // LATIN SMALL LETTER A WITH DIAERESIS
    ('a', '\u{030A}', '\u{00E5}'), // LATIN SMALL LETTER A WITH RING ABOVE
    ('c', '\u{0327}', '\u{00E7}'), // LATIN SMALL LETTER C WITH CEDILLA
    ('e', '\u{0300}', '\u{00E8}'), // LATIN SMALL LETTER E WITH GRAVE
    ('e', '\u{0301}', '\u{00E9}'), // LATIN SMALL LETTER E WITH ACUTE
    ('e', '\u{0302}', '\u{00EA}'), // LATIN SMALL LETTER E WITH CIRCUMFLEX
    ('e', '\u{0308}', '\u{00EB}'), // LATIN SMALL LETTER E WITH DIAERESIS
    ('i', '\u{0300}', '\u{00EC}'), // LATIN SMALL LETTER I WITH GRAVE
    ('i', '\u{0301}', '\u{00ED}'), // LATIN SMALL LETTER I WITH ACUTE
    ('i', '\u{0302}', '\u{00EE}'), // LATIN SMALL LETTER I WITH CIRCUMFLEX
    ('i', '\u{0308}', '\u{00EF}'), // LATIN SMALL LETTER I WITH DIAERESIS
    ('n', '\u{0303}', '\u{00F1}'), // LATIN SMALL LETTER N WITH TILDE
    ('o', '\u{0300}', '\u{00F2}'), // LATIN SMALL LETTER O WITH GRAVE
    ('o', '\u{0301}', '\u{00F3}'), // LATIN SMALL LETTER O WITH ACUTE
    ('o', '\u{0302}', '\u{00F4}'), // LATIN SMALL LETTER O WITH CIRCUMFLEX
    ('o', '\u{0303}', '\u{00F5}'), // LATIN SMALL LETTER O WITH TILDE
    ('o', '\u{0308}', '\u{00F6}'), // LATIN SMALL LETTER O WITH DIAERESIS
    ('u', '\u{0300}', '\u{00F9}'), // LATIN SMALL LETTER U WITH GRAVE
    ('u', '\u{0301}', '\u{00FA}'), // LATIN SMALL LETTER U WITH ACUTE
    ('u', '\u{0302}', '\u{00FB}'), // LATIN SMALL LETTER U WITH CIRCUMFLEX
    ('u', '\u{0308}', '\u{00FC}'), // LATIN SMALL LETTER U WITH DIAERESIS
    ('y', '\u{0301}', '\u{00FD}'), // LATIN SMALL LETTER Y WITH ACUTE
    ('y', '\u{0308}', '\u{00FF}'), // LATIN SMALL LETTER Y WITH DIAERESIS
    // Basic Latin + Combining Diacritical Marks --> Latin Extended-A
    ('S', '\u{030C}', '\u{0160}'), // LATIN CAPITAL LETTER S WITH CARON
    ('s', '\u{030C}', '\u{0161}'), // LATIN SMALL LETTER S WITH CARON
    ('Z', '\u{030C}', '\u{017D}'), // LATIN CAPITAL LETTER Z WITH CARON
    ('z', '\u{030C}', '\u{017E}'), // LATIN SMALL LETTER Z WITH CARON
    // Basic Russian alphabet + Combining Diacritical Marks --> Basic Russian alphabet
    ('\u{0418}', '\u{0306}', '\u{0419}'), // CYRILLIC CAPITAL LETTER SHORT I
    ('\u{0438}', '\u{0306}', '\u{0439}'), // CYRILLIC SMALL LETTER SHORT I
    // Basic Russian alphabet + Combining Diacritical Marks --> Cyrillic extensions
    ('\u{0415}', '\u{0300}', '\u{0400}'), // CYRILLIC CAPITAL LETTER IE WITH GRAVE
    ('\u{0435}', '\u{0300}', '\u{0450}'), // CYRILLIC SMALL LETTER IE WITH GRAVE
    ('\u{0415}', '\u{0308}', '\u{0401}'), // CYRILLIC CAPITAL LETTER IO
    ('\u{0435}', '\u{0308}', '\u{0451}'), // CYRILLIC SMALL LETTER IO
    ('\u{0413}', '\u{0301}', '\u{0403}'), // CYRILLIC CAPITAL LETTER GJE
    ('\u{0433}', '\u{0301}', '\u{0453}'), // CYRILLIC SMALL LETTER GJE
    // Basic Russian alphabet + Combining Diacritical Marks --> Extended Cyrillic
    ('\u{041E}', '\u{0308}', '\u{04E6}'), // CYRILLIC CAPITAL LETTER O WITH DIAERESIS
    ('\u{043E}', '\u{0308}', '\u{04E7}'), // CYRILLIC SMALL LETTER O WITH DIAERESIS
];

/// Unicode normalization matching C++ voikko_normalise.
///
/// Applies character conversions in priority order:
/// 1. 2-to-1: base + combining mark -> precomposed character
/// 2. 1-to-1: simple substitutions (hyphens, quotation marks)
/// 3. 1-to-2: single char -> two chars (degree symbols, ligatures)
/// 4. 1-to-3: single char -> three chars (triple ligatures)
/// 5. passthrough
///
/// Origin: charset.cpp:voikko_normalise
fn normalize(word: &[char]) -> Vec<char> {
    // Worst case: every char is a 1-to-3 ligature
    let mut result = Vec::with_capacity(word.len() * 3);
    let len = word.len();
    let mut i = 0;
    while i < len {
        // --- Priority 1: 2-to-1 combining diacritical mark composition ---
        if i < len - 1 {
            let mut found_2to1 = false;
            for &(base, combining, precomposed) in &CONV_2TO1 {
                if word[i] == base && word[i + 1] == combining {
                    result.push(precomposed);
                    i += 2;
                    found_2to1 = true;
                    break;
                }
            }
            if found_2to1 {
                continue;
            }
        }

        // --- Priority 2: 1-to-1 simple substitutions ---
        // --- Priority 3: 1-to-2 expansions ---
        // --- Priority 4: 1-to-3 expansions ---
        match word[i] {
            // 1-to-1: General Punctuation --> Basic Latin
            '\u{2019}' => result.push('\''), // RIGHT SINGLE QUOTATION MARK -> APOSTROPHE
            '\u{2010}' => result.push('-'),  // HYPHEN -> HYPHEN-MINUS
            '\u{2011}' => result.push('-'),  // NON-BREAKING HYPHEN -> HYPHEN-MINUS

            // 1-to-2: Letterlike Symbols
            '\u{2103}' => {
                // DEGREE CELSIUS -> degree sign + C
                result.push('\u{00B0}');
                result.push('C');
            }
            '\u{2109}' => {
                // DEGREE FAHRENHEIT -> degree sign + F
                result.push('\u{00B0}');
                result.push('F');
            }
            // 1-to-2: Alphabetic Presentation Forms (2-char ligatures)
            '\u{FB00}' => {
                result.push('f');
                result.push('f');
            }
            '\u{FB01}' => {
                result.push('f');
                result.push('i');
            }
            '\u{FB02}' => {
                result.push('f');
                result.push('l');
            }

            // 1-to-3: Alphabetic Presentation Forms (3-char ligatures)
            '\u{FB03}' => {
                result.push('f');
                result.push('f');
                result.push('i');
            }
            '\u{FB04}' => {
                result.push('f');
                result.push('f');
                result.push('l');
            }

            // Passthrough
            c => result.push(c),
        }
        i += 1;
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
    use voikko_core::analysis::{ATTR_STRUCTURE, Analysis};

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

    #[test]
    fn normalize_ligature_fi() {
        let word = chars("\u{FB01}"); // LATIN SMALL LIGATURE FI
        let result = normalize(&word);
        assert_eq!(result, chars("fi"));
    }

    #[test]
    fn normalize_ligature_fl() {
        let word = chars("\u{FB02}"); // LATIN SMALL LIGATURE FL
        let result = normalize(&word);
        assert_eq!(result, chars("fl"));
    }

    #[test]
    fn normalize_ligature_ffi() {
        let word = chars("\u{FB03}"); // LATIN SMALL LIGATURE FFI
        let result = normalize(&word);
        assert_eq!(result, chars("ffi"));
    }

    #[test]
    fn normalize_ligature_ffl() {
        let word = chars("\u{FB04}"); // LATIN SMALL LIGATURE FFL
        let result = normalize(&word);
        assert_eq!(result, chars("ffl"));
    }

    #[test]
    fn normalize_non_breaking_hyphen() {
        let word = chars("syy\u{2011}silta"); // NON-BREAKING HYPHEN (U+2011)
        let result = normalize(&word);
        assert_eq!(result, chars("syy-silta"));
    }

    #[test]
    fn normalize_right_single_quotation_mark() {
        let word = chars("it\u{2019}s"); // RIGHT SINGLE QUOTATION MARK (U+2019)
        let result = normalize(&word);
        assert_eq!(result, chars("it's"));
    }

    #[test]
    fn normalize_degree_celsius() {
        let word = chars("20\u{2103}"); // DEGREE CELSIUS (U+2103)
        let result = normalize(&word);
        assert_eq!(result, chars("20\u{00B0}C"));
    }

    #[test]
    fn normalize_degree_fahrenheit() {
        let word = chars("68\u{2109}"); // DEGREE FAHRENHEIT (U+2109)
        let result = normalize(&word);
        assert_eq!(result, chars("68\u{00B0}F"));
    }

    // --- 2-to-1 combining diacritical mark tests ---

    #[test]
    fn normalize_a_combining_diaeresis_lower() {
        // a + U+0308 COMBINING DIAERESIS -> U+00E4 (a with diaeresis)
        let word = chars("a\u{0308}");
        let result = normalize(&word);
        assert_eq!(result, chars("\u{00E4}"));
    }

    #[test]
    fn normalize_a_combining_diaeresis_upper() {
        // A + U+0308 COMBINING DIAERESIS -> U+00C4 (A with diaeresis)
        let word = chars("A\u{0308}");
        let result = normalize(&word);
        assert_eq!(result, chars("\u{00C4}"));
    }

    #[test]
    fn normalize_o_combining_diaeresis_lower() {
        // o + U+0308 COMBINING DIAERESIS -> U+00F6 (o with diaeresis)
        let word = chars("o\u{0308}");
        let result = normalize(&word);
        assert_eq!(result, chars("\u{00F6}"));
    }

    #[test]
    fn normalize_o_combining_diaeresis_upper() {
        // O + U+0308 COMBINING DIAERESIS -> U+00D6 (O with diaeresis)
        let word = chars("O\u{0308}");
        let result = normalize(&word);
        assert_eq!(result, chars("\u{00D6}"));
    }

    #[test]
    fn normalize_e_combining_acute() {
        // e + U+0301 COMBINING ACUTE -> U+00E9 (e with acute)
        let word = chars("e\u{0301}");
        let result = normalize(&word);
        assert_eq!(result, chars("\u{00E9}"));
    }

    #[test]
    fn normalize_n_combining_tilde() {
        // n + U+0303 COMBINING TILDE -> U+00F1 (n with tilde)
        let word = chars("n\u{0303}");
        let result = normalize(&word);
        assert_eq!(result, chars("\u{00F1}"));
    }

    #[test]
    fn normalize_a_combining_ring_above() {
        // a + U+030A COMBINING RING ABOVE -> U+00E5 (a with ring above)
        let word = chars("a\u{030A}");
        let result = normalize(&word);
        assert_eq!(result, chars("\u{00E5}"));
    }

    #[test]
    fn normalize_c_combining_cedilla() {
        // c + U+0327 COMBINING CEDILLA -> U+00E7 (c with cedilla)
        let word = chars("c\u{0327}");
        let result = normalize(&word);
        assert_eq!(result, chars("\u{00E7}"));
    }

    #[test]
    fn normalize_s_combining_caron() {
        // s + U+030C COMBINING CARON -> U+0161 (s with caron)
        let word = chars("s\u{030C}");
        let result = normalize(&word);
        assert_eq!(result, chars("\u{0161}"));
    }

    #[test]
    fn normalize_z_combining_caron_upper() {
        // Z + U+030C COMBINING CARON -> U+017D (Z with caron)
        let word = chars("Z\u{030C}");
        let result = normalize(&word);
        assert_eq!(result, chars("\u{017D}"));
    }

    #[test]
    fn normalize_cyrillic_short_i() {
        // CYRILLIC CAPITAL LETTER I (U+0418) + COMBINING BREVE (U+0306)
        // -> CYRILLIC CAPITAL LETTER SHORT I (U+0419)
        let word = chars("\u{0418}\u{0306}");
        let result = normalize(&word);
        assert_eq!(result, chars("\u{0419}"));
    }

    #[test]
    fn normalize_cyrillic_io_lower() {
        // CYRILLIC SMALL LETTER IE (U+0435) + COMBINING DIAERESIS (U+0308)
        // -> CYRILLIC SMALL LETTER IO (U+0451)
        let word = chars("\u{0435}\u{0308}");
        let result = normalize(&word);
        assert_eq!(result, chars("\u{0451}"));
    }

    #[test]
    fn normalize_word_with_combining_marks() {
        // "pa\u{0308}a\u{0308}" (p + a+diaeresis + a+diaeresis) -> "p\u{00E4}\u{00E4}"
        let word = chars("pa\u{0308}a\u{0308}");
        let result = normalize(&word);
        assert_eq!(result, chars("p\u{00E4}\u{00E4}"));
    }

    #[test]
    fn normalize_mixed_combining_and_ligature() {
        // Mix: combining mark composition + ligature in same word
        // "a\u{0308}\u{FB01}n" -> "\u{00E4}fin"
        let word = chars("a\u{0308}\u{FB01}n");
        let result = normalize(&word);
        assert_eq!(result, chars("\u{00E4}fin"));
    }

    #[test]
    fn normalize_2to1_has_priority_over_1to1() {
        // If a char matches 2-to-1 (with next char), that takes priority.
        // U+0415 (Cyrillic IE) + U+0300 -> U+0400 (Cyrillic IE WITH GRAVE)
        // not passed through individually
        let word = chars("\u{0415}\u{0300}");
        let result = normalize(&word);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], '\u{0400}');
    }

    #[test]
    fn normalize_combining_mark_at_end_passthrough() {
        // A combining mark at the very end with no recognized pair: pass through
        let word = chars("x\u{0306}"); // x + COMBINING BREVE (not in table)
        let result = normalize(&word);
        assert_eq!(result, chars("x\u{0306}"));
    }

    #[test]
    fn normalize_empty_word() {
        let word: Vec<char> = vec![];
        let result = normalize(&word);
        assert!(result.is_empty());
    }

    #[test]
    fn normalize_single_char() {
        let word = chars("k");
        let result = normalize(&word);
        assert_eq!(result, chars("k"));
    }

    #[test]
    fn normalize_conv_2to1_table_count() {
        // Verify table has exactly 67 entries as in C++
        assert_eq!(CONV_2TO1.len(), 67);
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
        assert_eq!(
            spell_word(&long_word, &default_options()),
            VOIKKO_SPELL_FAILED
        );
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
