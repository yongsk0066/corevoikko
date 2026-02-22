// Token and Sentence public API types
// Origin: grammar/Token.hpp, sentence/Sentence.hpp

use crate::enums::{SentenceType, TokenType};

// ---------------------------------------------------------------------------
// Token
// Origin: grammar/Token.hpp:51-113
// ---------------------------------------------------------------------------

/// A text token produced by the tokenizer or used by the grammar checker.
///
/// The C++ `Token` struct has many grammar-checker-internal boolean flags
/// (isValidWord, firstLetterLcase, possibleSentenceStart, etc.). For the
/// public API, we only expose the fields needed for tokenization output.
/// Grammar-checker-specific fields will live in the grammar checker crate.
///
/// Origin: grammar/Token.hpp:51-113
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// The type of this token.
    /// Origin: Token.hpp:53
    pub token_type: TokenType,

    /// The text content of this token.
    /// Origin: Token.hpp:103 (wchar_t* str)
    pub text: String,

    /// Length of the token in characters.
    /// Origin: Token.hpp:106 (tokenlen)
    pub token_len: usize,

    /// Position of this token within the paragraph (character offset).
    /// Origin: Token.hpp:109 (pos)
    pub pos: usize,
}

impl Token {
    /// Create a new token.
    pub fn new(token_type: TokenType, text: impl Into<String>, pos: usize) -> Self {
        let text = text.into();
        let token_len = text.chars().count();
        Self {
            token_type,
            text,
            token_len,
            pos,
        }
    }

    /// Create an empty `None` token at position 0, signaling end-of-text.
    pub fn none() -> Self {
        Self {
            token_type: TokenType::None,
            text: String::new(),
            token_len: 0,
            pos: 0,
        }
    }
}

impl Default for Token {
    fn default() -> Self {
        Self::none()
    }
}

// ---------------------------------------------------------------------------
// Sentence
// Origin: sentence/Sentence.hpp:38-43
//
// The C++ Sentence class is a static helper with a `next` method. In Rust,
// we model the result of sentence detection as a simple data struct, and the
// actual detection logic will live in the tokenizer/grammar crate.
// ---------------------------------------------------------------------------

/// Result of sentence boundary detection.
///
/// Origin: sentence/Sentence.hpp:38-43
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sentence {
    /// The type of sentence boundary detected.
    pub sentence_type: SentenceType,

    /// Length of the sentence in characters.
    pub sentence_len: usize,
}

impl Sentence {
    /// Create a new sentence result.
    pub fn new(sentence_type: SentenceType, sentence_len: usize) -> Self {
        Self {
            sentence_type,
            sentence_len,
        }
    }

    /// Create a `None` sentence (end of text / no sentence found).
    pub fn none() -> Self {
        Self {
            sentence_type: SentenceType::None,
            sentence_len: 0,
        }
    }
}

impl Default for Sentence {
    fn default() -> Self {
        Self::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Token tests --

    #[test]
    fn token_new() {
        let tok = Token::new(TokenType::Word, "koira", 0);
        assert_eq!(tok.token_type, TokenType::Word);
        assert_eq!(tok.text, "koira");
        assert_eq!(tok.token_len, 5);
        assert_eq!(tok.pos, 0);
    }

    #[test]
    fn token_new_with_position() {
        let tok = Token::new(TokenType::Punctuation, ".", 10);
        assert_eq!(tok.token_type, TokenType::Punctuation);
        assert_eq!(tok.text, ".");
        assert_eq!(tok.token_len, 1);
        assert_eq!(tok.pos, 10);
    }

    #[test]
    fn token_unicode_length() {
        // "äiti" is 4 characters, 5 bytes in UTF-8
        let tok = Token::new(TokenType::Word, "\u{00E4}iti", 0);
        assert_eq!(tok.token_len, 4); // character count, not byte count
    }

    #[test]
    fn token_none() {
        let tok = Token::none();
        assert_eq!(tok.token_type, TokenType::None);
        assert!(tok.text.is_empty());
        assert_eq!(tok.token_len, 0);
        assert_eq!(tok.pos, 0);
    }

    #[test]
    fn token_default_is_none() {
        let tok = Token::default();
        assert_eq!(tok.token_type, TokenType::None);
    }

    #[test]
    fn token_clone() {
        let tok = Token::new(TokenType::Word, "koira", 0);
        let cloned = tok.clone();
        assert_eq!(tok, cloned);
    }

    // -- Sentence tests --

    #[test]
    fn sentence_new() {
        let s = Sentence::new(SentenceType::Probable, 42);
        assert_eq!(s.sentence_type, SentenceType::Probable);
        assert_eq!(s.sentence_len, 42);
    }

    #[test]
    fn sentence_none() {
        let s = Sentence::none();
        assert_eq!(s.sentence_type, SentenceType::None);
        assert_eq!(s.sentence_len, 0);
    }

    #[test]
    fn sentence_default_is_none() {
        let s = Sentence::default();
        assert_eq!(s.sentence_type, SentenceType::None);
    }

    #[test]
    fn sentence_clone() {
        let s = Sentence::new(SentenceType::Possible, 15);
        let cloned = s.clone();
        assert_eq!(s, cloned);
    }
}
