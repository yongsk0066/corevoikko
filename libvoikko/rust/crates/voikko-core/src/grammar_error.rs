// Grammar error public API type
// Origin: grammar/VoikkoGrammarError.hpp, voikko_structs.h, grammar/error.hpp

// ---------------------------------------------------------------------------
// Grammar error codes
// Origin: grammar/error.hpp:35-53
// ---------------------------------------------------------------------------

pub const GCERR_INVALID_SPELLING: i32 = 1;
pub const GCERR_EXTRA_WHITESPACE: i32 = 2;
pub const GCERR_SPACE_BEFORE_PUNCTUATION: i32 = 3;
pub const GCERR_EXTRA_COMMA: i32 = 4;
pub const GCERR_INVALID_SENTENCE_STARTER: i32 = 5;
pub const GCERR_WRITE_FIRST_LOWERCASE: i32 = 6;
pub const GCERR_WRITE_FIRST_UPPERCASE: i32 = 7;
pub const GCERR_REPEATING_WORD: i32 = 8;
pub const GCERR_TERMINATING_PUNCTUATION_MISSING: i32 = 9;
pub const GCERR_INVALID_PUNCTUATION_AT_END_OF_QUOTATION: i32 = 10;
pub const GCERR_FOREIGN_QUOTATION_MARK: i32 = 11;
pub const GCERR_MISPLACED_CLOSING_PARENTHESIS: i32 = 12;
pub const GCERR_NEGATIVE_VERB_MISMATCH: i32 = 13;
pub const GCERR_A_INFINITIVE_REQUIRED: i32 = 14;
pub const GCERR_MA_INFINITIVE_REQUIRED: i32 = 15;
pub const GCERR_MISPLACED_SIDESANA: i32 = 16;
pub const GCERR_MISSING_MAIN_VERB: i32 = 17;
pub const GCERR_EXTRA_MAIN_VERB: i32 = 18;

/// A grammar error detected during grammar checking.
///
/// This corresponds to the C++ `VoikkoGrammarError` / `voikko_grammar_error`
/// combined type. In the C++ code, `error_level` and `error_description` are
/// marked as unused; we omit them.
///
/// Origin: voikko_structs.h:43-57, grammar/VoikkoGrammarError.hpp:42-66
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrammarError {
    /// Error code. 0 means no error was found.
    /// Origin: voikko_structs.h:45
    pub error_code: i32,

    /// Start position of the error in the text (character offset).
    /// Origin: voikko_structs.h:51
    pub start_pos: usize,

    /// Length of the erroneous span in characters.
    /// Origin: voikko_structs.h:53
    pub error_len: usize,

    /// Suggested corrections for the error.
    /// Origin: voikko_structs.h:56
    pub suggestions: Vec<String>,
}

impl GrammarError {
    /// Create a new grammar error with no suggestions.
    pub fn new(error_code: i32, start_pos: usize, error_len: usize) -> Self {
        Self {
            error_code,
            start_pos,
            error_len,
            suggestions: Vec::new(),
        }
    }

    /// Create a new grammar error with suggestions.
    pub fn with_suggestions(
        error_code: i32,
        start_pos: usize,
        error_len: usize,
        suggestions: Vec<String>,
    ) -> Self {
        Self {
            error_code,
            start_pos,
            error_len,
            suggestions,
        }
    }
}

impl Default for GrammarError {
    /// Default grammar error with error_code 0 (no error).
    fn default() -> Self {
        Self {
            error_code: 0,
            start_pos: 0,
            error_len: 0,
            suggestions: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_error() {
        let err = GrammarError::new(GCERR_INVALID_SPELLING, 5, 3);
        assert_eq!(err.error_code, 1);
        assert_eq!(err.start_pos, 5);
        assert_eq!(err.error_len, 3);
        assert!(err.suggestions.is_empty());
    }

    #[test]
    fn error_with_suggestions() {
        let err = GrammarError::with_suggestions(
            GCERR_REPEATING_WORD,
            10,
            5,
            vec!["word".to_string()],
        );
        assert_eq!(err.error_code, GCERR_REPEATING_WORD);
        assert_eq!(err.suggestions.len(), 1);
        assert_eq!(err.suggestions[0], "word");
    }

    #[test]
    fn default_error() {
        let err = GrammarError::default();
        assert_eq!(err.error_code, 0);
        assert_eq!(err.start_pos, 0);
        assert_eq!(err.error_len, 0);
        assert!(err.suggestions.is_empty());
    }

    #[test]
    fn clone_is_independent() {
        let err = GrammarError::with_suggestions(
            GCERR_EXTRA_WHITESPACE,
            0,
            2,
            vec!["fix".to_string()],
        );
        let mut cloned = err.clone();
        cloned.suggestions.push("another".to_string());
        assert_eq!(err.suggestions.len(), 1);
        assert_eq!(cloned.suggestions.len(), 2);
    }

    #[test]
    fn error_codes_match_cpp() {
        assert_eq!(GCERR_INVALID_SPELLING, 1);
        assert_eq!(GCERR_EXTRA_WHITESPACE, 2);
        assert_eq!(GCERR_REPEATING_WORD, 8);
        assert_eq!(GCERR_EXTRA_MAIN_VERB, 18);
    }
}
