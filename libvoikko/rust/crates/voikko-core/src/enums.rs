// Shared enums: TokenType, SentenceType, SpellResult, option constants
// Origin: voikko_enums.h, voikko_defines.h

/// Token types for string tokenization.
/// Origin: voikko_enums.h:40
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenType {
    /// End of text or error.
    None,
    /// Word token.
    Word,
    /// Punctuation token.
    Punctuation,
    /// Whitespace token.
    Whitespace,
    /// Character not used in any supported natural language.
    Unknown,
}

/// Sentence start types for sentence detection.
/// Origin: voikko_enums.h:49
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SentenceType {
    /// End of text reached or error.
    None,
    /// This is not a start of a new sentence.
    NoStart,
    /// This is a probable start of a new sentence.
    Probable,
    /// This may be a start of a new sentence.
    Possible,
}

/// Internal spell-checker result type.
/// Origin: voikko_defines.h:49-53 (VOIKKO_SPELL_*)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SpellResult {
    /// Word is correctly spelled (VOIKKO_SPELL_OK = 1).
    Ok,
    /// Word is correct if the first letter is capitalized.
    CapitalizeFirst,
    /// Word has a capitalization error.
    CapitalizationError,
    /// Word is misspelled (VOIKKO_SPELL_FAILED = 0).
    Failed,
}

// ---------------------------------------------------------------------------
// Option constants
// Origin: voikko_defines.h:47-145
// ---------------------------------------------------------------------------

/// Maximum number of characters in a word (legacy; backends may vary).
/// Origin: voikko_defines.h:47
pub const MAX_WORD_CHARS: usize = 255;

// -- Boolean options --------------------------------------------------------

/// Ignore dot at the end of the word. Default: false.
/// Origin: voikko_defines.h:63
pub const OPT_IGNORE_DOT: i32 = 0;

/// Ignore words containing numbers (spell checking only). Default: false.
/// Origin: voikko_defines.h:67
pub const OPT_IGNORE_NUMBERS: i32 = 1;

/// Accept words written completely in uppercase without checking. Default: false.
/// Origin: voikko_defines.h:72
pub const OPT_IGNORE_UPPERCASE: i32 = 3;

/// Do not insert ugly but correct hyphenation positions. Default: false.
/// Origin: voikko_defines.h:86
pub const OPT_NO_UGLY_HYPHENATION: i32 = 4;

/// Accept words even when the first letter is uppercase. Default: true.
/// Origin: voikko_defines.h:76
pub const OPT_ACCEPT_FIRST_UPPERCASE: i32 = 6;

/// Accept words even when all letters are uppercase (still checked). Default: true.
/// Origin: voikko_defines.h:82
pub const OPT_ACCEPT_ALL_UPPERCASE: i32 = 7;

/// Use suggestions optimized for OCR software. Default: false.
/// Origin: voikko_defines.h:91
pub const OPT_OCR_SUGGESTIONS: i32 = 8;

/// Ignore non-words such as URLs and email addresses (spell checking only). Default: true.
/// Origin: voikko_defines.h:95
pub const OPT_IGNORE_NONWORDS: i32 = 10;

/// Allow some extra hyphens in words (spell checking only). Default: false.
/// Origin: voikko_defines.h:102
pub const OPT_ACCEPT_EXTRA_HYPHENS: i32 = 11;

/// Accept missing hyphens at start/end of word (spell checking only). Default: false.
/// Origin: voikko_defines.h:110
pub const OPT_ACCEPT_MISSING_HYPHENS: i32 = 12;

/// Accept incomplete sentences in titles (grammar checking only). Default: false.
/// Origin: voikko_defines.h:117
pub const OPT_ACCEPT_TITLES_IN_GC: i32 = 13;

/// Accept incomplete sentences at end of paragraph (grammar checking only). Default: false.
/// Origin: voikko_defines.h:122
pub const OPT_ACCEPT_UNFINISHED_PARAGRAPHS_IN_GC: i32 = 14;

/// Hyphenate unknown words (hyphenation only). Default: true.
/// Origin: voikko_defines.h:126
pub const OPT_HYPHENATE_UNKNOWN_WORDS: i32 = 15;

/// Accept paragraphs valid within bulleted lists (grammar checking only). Default: false.
/// Origin: voikko_defines.h:131
pub const OPT_ACCEPT_BULLETED_LISTS_IN_GC: i32 = 16;

// -- Integer options --------------------------------------------------------

/// Minimum length for words that may be hyphenated. Default: 2.
/// Origin: voikko_defines.h:138
pub const MIN_HYPHENATED_WORD_LENGTH: i32 = 9;

/// Size of the spell checker cache. -1 = no cache, >= 0 = 2^n * block. Default: 0.
/// Origin: voikko_defines.h:143
pub const SPELLER_CACHE_SIZE: i32 = 17;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_type_equality() {
        assert_eq!(TokenType::Word, TokenType::Word);
        assert_ne!(TokenType::Word, TokenType::Punctuation);
    }

    #[test]
    fn sentence_type_equality() {
        assert_eq!(SentenceType::None, SentenceType::None);
        assert_ne!(SentenceType::Probable, SentenceType::Possible);
    }

    #[test]
    fn spell_result_equality() {
        assert_eq!(SpellResult::Ok, SpellResult::Ok);
        assert_ne!(SpellResult::Ok, SpellResult::Failed);
    }

    #[test]
    fn token_type_is_copy() {
        let a = TokenType::Word;
        let b = a; // Copy
        assert_eq!(a, b);
    }

    #[test]
    fn option_constants_match_cpp() {
        // Verify a few key constants match the C++ defines
        assert_eq!(OPT_IGNORE_DOT, 0);
        assert_eq!(OPT_IGNORE_NUMBERS, 1);
        assert_eq!(OPT_IGNORE_UPPERCASE, 3);
        assert_eq!(OPT_ACCEPT_FIRST_UPPERCASE, 6);
        assert_eq!(OPT_ACCEPT_ALL_UPPERCASE, 7);
        assert_eq!(OPT_IGNORE_NONWORDS, 10);
        assert_eq!(SPELLER_CACHE_SIZE, 17);
        assert_eq!(MAX_WORD_CHARS, 255);
    }
}
