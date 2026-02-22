// Paragraph structure with tokenized sentences for grammar checking
// Origin: grammar/Paragraph.hpp, Paragraph.cpp, Sentence.hpp, Sentence.cpp,
//         grammar/Token.hpp, FinnishAnalysis.cpp:194-269 (analyseParagraph/analyseSentence)

use voikko_core::enums::{SentenceType, TokenType};

use crate::tokenizer;

// ---------------------------------------------------------------------------
// Constants
// Origin: Paragraph.hpp:48, Sentence.hpp:43
// ---------------------------------------------------------------------------

/// Maximum number of sentences allowed in a single paragraph.
/// Origin: Paragraph.hpp:48
pub(crate) const MAX_SENTENCES_IN_PARAGRAPH: usize = 200;

/// Maximum number of tokens allowed in a single sentence.
/// Origin: Sentence.hpp:43
pub(crate) const MAX_TOKENS_IN_SENTENCE: usize = 500;

// ---------------------------------------------------------------------------
// FollowingVerbType
// Origin: grammar/Token.hpp:42-46
// ---------------------------------------------------------------------------

/// Types for trailing parts of compound verb constructs.
///
/// Origin: grammar/Token.hpp:42-46
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FollowingVerbType {
    /// Not a verb, or no requirement.
    None,
    /// Requires or is an A-infinitive form.
    AInfinitive,
    /// Requires or is a MA-infinitive form.
    MaInfinitive,
}

// ---------------------------------------------------------------------------
// GrammarToken
// Origin: grammar/Token.hpp:51-113
// ---------------------------------------------------------------------------

/// Grammar-checker-internal token with morphological annotation flags.
///
/// Extends the public `TokenType` with boolean flags used by the grammar
/// rule engine to detect errors. Each flag is set by `FinnishAnalysis::analyseToken`.
///
/// Origin: grammar/Token.hpp:51-113
#[derive(Debug, Clone)]
pub(crate) struct GrammarToken {
    /// The type of this token (word, punctuation, whitespace, etc.).
    /// Origin: Token.hpp:53
    pub token_type: TokenType,

    /// The text content of this token as a char vector.
    /// Origin: Token.hpp:103 (wchar_t* str)
    pub text: Vec<char>,

    /// Position of this token within the paragraph (character offset).
    /// Origin: Token.hpp:109
    pub pos: usize,

    /// True if this word token was recognized as a valid word.
    /// Origin: Token.hpp:56
    pub is_valid_word: bool,

    /// True if this is a word token that should start with a lower-case letter
    /// (determined from the STRUCTURE attribute).
    /// Origin: Token.hpp:60
    pub first_letter_lcase: bool,

    /// True if this word may be (but is not necessarily) the first word in
    /// a sentence. Set based on preceding punctuation.
    /// Origin: Token.hpp:64
    pub possible_sentence_start: bool,

    /// True if this word may be a geographical name in genitive case.
    /// Origin: Token.hpp:67
    pub is_geographical_name_in_genitive: bool,

    /// True if this is a proper noun that might be a geographical name.
    /// Origin: Token.hpp:70
    pub possible_geographical_name: bool,

    /// True if this word may be the main verb.
    /// Origin: Token.hpp:73
    pub possible_main_verb: bool,

    /// True if this word is definitely the main verb (indicative mood, all
    /// analyses agree).
    /// Origin: Token.hpp:76
    pub is_main_verb: bool,

    /// True if this word is a verb negative ("en", "et", "ei", etc.).
    /// Origin: Token.hpp:79
    pub is_verb_negative: bool,

    /// True if this word cannot be anything else than a positive verb.
    /// Origin: Token.hpp:82
    pub is_positive_verb: bool,

    /// True if this word is a conjunction.
    /// Origin: Token.hpp:85
    pub is_conjunction: bool,

    /// True if this word may be a conjunction.
    /// Origin: Token.hpp:88
    pub possible_conjunction: bool,

    /// What kind of verb must follow this verb in compound verb check.
    /// `None` if this word is not (or may not be) a verb.
    /// Origin: Token.hpp:94
    pub require_following_verb: FollowingVerbType,

    /// What kind of verb this word is if it is used as a trailing part in
    /// compound verb constructs. `None` if this word is not a verb.
    /// Origin: Token.hpp:100
    pub verb_follower_type: FollowingVerbType,
}

impl GrammarToken {
    /// Create a new grammar token with default flags.
    ///
    /// All boolean flags are set to their default (false) values. The caller
    /// is responsible for running morphological analysis to set them.
    pub fn new(token_type: TokenType, text: Vec<char>, pos: usize) -> Self {
        Self {
            token_type,
            text,
            pos,
            is_valid_word: false,
            first_letter_lcase: false,
            possible_sentence_start: false,
            is_geographical_name_in_genitive: false,
            possible_geographical_name: false,
            possible_main_verb: false,
            is_main_verb: false,
            is_verb_negative: false,
            is_positive_verb: false,
            is_conjunction: false,
            possible_conjunction: false,
            require_following_verb: FollowingVerbType::None,
            verb_follower_type: FollowingVerbType::None,
        }
    }

    /// Return the token text length in characters.
    pub fn token_len(&self) -> usize {
        self.text.len()
    }
}

// ---------------------------------------------------------------------------
// GrammarSentence
// Origin: grammar/Sentence.hpp:39-63
// ---------------------------------------------------------------------------

/// A sentence within a paragraph, containing grammar-annotated tokens.
///
/// Origin: grammar/Sentence.hpp:39-63
#[derive(Debug, Clone)]
pub(crate) struct GrammarSentence {
    /// The sentence boundary type (how the next sentence starts).
    /// Origin: Sentence.hpp:49
    pub sentence_type: SentenceType,

    /// The tokens in this sentence.
    /// Origin: Sentence.hpp:52
    pub tokens: Vec<GrammarToken>,

    /// Position of this sentence within the paragraph (character offset).
    /// Origin: Sentence.hpp:58
    pub pos: usize,
}

impl GrammarSentence {
    /// Create a new empty sentence at the given paragraph offset.
    pub fn new(pos: usize) -> Self {
        Self {
            sentence_type: SentenceType::None,
            tokens: Vec::new(),
            pos,
        }
    }
}

// ---------------------------------------------------------------------------
// Paragraph
// Origin: grammar/Paragraph.hpp:41-59
// ---------------------------------------------------------------------------

/// A tokenized and analyzed paragraph for grammar checking.
///
/// Created by `analyse_paragraph`, which splits the text into sentences and
/// tokens, then runs spell checking and morphological analysis on each word
/// token.
///
/// Origin: grammar/Paragraph.hpp:41-59
#[derive(Debug, Clone)]
pub(crate) struct Paragraph {
    /// The sentences in this paragraph.
    /// Origin: Paragraph.hpp:51-52
    pub sentences: Vec<GrammarSentence>,
}

impl Paragraph {
    /// Create a new empty paragraph.
    /// Origin: Paragraph.cpp:33
    pub fn new() -> Self {
        Self {
            sentences: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Sentence-level punctuation characters that may indicate sentence end
// Origin: FinnishAnalysis.cpp:224-228
// ---------------------------------------------------------------------------

/// Characters that, when appearing as single-character punctuation tokens,
/// indicate a possible sentence start for the next word.
///
/// Origin: FinnishAnalysis.cpp:225 — `wcschr(L".:\u2026\u2013\u2014", tstr[0])`
const SENTENCE_SEPARATING_PUNCTUATION: &[char] = &[
    '.',
    ':',
    '\u{2026}', // horizontal ellipsis
    '\u{2013}', // en dash
    '\u{2014}', // em dash
];

// ---------------------------------------------------------------------------
// analyse_sentence
// Origin: FinnishAnalysis.cpp:195-238 (FinnishAnalysis::analyseSentence)
// ---------------------------------------------------------------------------

/// Tokenize a sentence region and populate a `GrammarSentence` with tokens.
///
/// The `text` slice is the full paragraph text, `sentence_start` and
/// `sentence_len` define the region for this sentence. Each word token is
/// checked for grammar-relevant properties by `analyse_token_fn`.
///
/// Origin: FinnishAnalysis.cpp:195-238
fn analyse_sentence<F>(
    text: &[char],
    sentence_start: usize,
    sentence_len: usize,
    analyse_token_fn: &mut F,
) -> Option<GrammarSentence>
where
    F: FnMut(&mut GrammarToken),
{
    let mut sentence = GrammarSentence::new(sentence_start);
    let slice = &text[sentence_start..sentence_start + sentence_len];
    let remaining = sentence_len;

    let mut pos: usize = 0;
    let mut next_word_is_possible_sentence_start = false;

    for _ in 0..MAX_TOKENS_IN_SENTENCE {
        // Origin: FinnishAnalysis.cpp:204-206
        // The C++ code forces ignore_dot=0 during sentence tokenization.
        let (tt, tokenlen) = tokenizer::next_token(slice, remaining, pos);
        if tt == TokenType::None {
            return Some(sentence);
        }

        let token_text: Vec<char> = slice[pos..pos + tokenlen].to_vec();
        let token_pos = sentence_start + pos;

        let mut token = GrammarToken::new(tt, token_text, token_pos);

        // Run morphological/spelling analysis on the token.
        // Origin: FinnishAnalysis.cpp:218
        analyse_token_fn(&mut token);

        // Origin: FinnishAnalysis.cpp:220-228
        // Set possible_sentence_start for word tokens after certain punctuation.
        if next_word_is_possible_sentence_start && tt == TokenType::Word {
            token.possible_sentence_start = true;
            next_word_is_possible_sentence_start = false;
        } else if tt == TokenType::Punctuation {
            // . : ... (3-char punctuation) and Unicode ellipsis, en/em dash
            let is_three_char_ellipsis = tokenlen == 3;
            let is_single_separator =
                tokenlen == 1 && SENTENCE_SEPARATING_PUNCTUATION.contains(&token.text[0]);
            if is_three_char_ellipsis || is_single_separator {
                next_word_is_possible_sentence_start = true;
            }
        }

        sentence.tokens.push(token);
        pos += tokenlen;
        if pos >= remaining {
            return Some(sentence);
        }
    }

    // Too long sentence or error.
    // Origin: FinnishAnalysis.cpp:236-237
    None
}

// ---------------------------------------------------------------------------
// analyse_paragraph
// Origin: FinnishAnalysis.cpp:241-269 (FinnishAnalysis::analyseParagraph)
// ---------------------------------------------------------------------------

/// Tokenize and analyze a paragraph into sentences and tokens.
///
/// This is the Rust equivalent of `FinnishAnalysis::analyseParagraph`. It
/// splits the text into sentences using the sentence detector, then tokenizes
/// each sentence and runs the provided analysis function on each word token.
///
/// The `analyse_token_fn` callback is responsible for running morphological
/// analysis and spell checking on each token, setting the grammar flags.
///
/// Returns `None` if a sentence is too long (> MAX_TOKENS_IN_SENTENCE tokens).
///
/// Origin: FinnishAnalysis.cpp:241-269
pub(crate) fn analyse_paragraph<F>(
    text: &[char],
    text_len: usize,
    analyse_token_fn: &mut F,
) -> Option<Paragraph>
where
    F: FnMut(&mut GrammarToken),
{
    let mut paragraph = Paragraph::new();
    let mut pos: usize = 0;
    let remaining_total = text_len;

    loop {
        if pos >= remaining_total {
            break;
        }

        // Accumulate sentence fragments until we get a definitive boundary.
        // Origin: FinnishAnalysis.cpp:247-256
        let sentence_start = pos;
        let mut sentence_len: usize = 0;
        let mut st;
        loop {
            let (stype, slen) = tokenizer::next_sentence(
                text,
                remaining_total,
                sentence_start + sentence_len,
            );
            sentence_len += slen;
            st = stype;
            if st != SentenceType::Possible {
                break;
            }
        }

        // Analyse the sentence.
        // Origin: FinnishAnalysis.cpp:258-263
        let sentence = analyse_sentence(text, sentence_start, sentence_len, analyse_token_fn);
        match sentence {
            Some(mut s) => {
                s.sentence_type = st;
                paragraph.sentences.push(s);
            }
            None => {
                // Sentence too long.
                return None;
            }
        }

        pos = sentence_start + sentence_len;

        // Origin: FinnishAnalysis.cpp:266-267
        if st == SentenceType::None || st == SentenceType::NoStart {
            break;
        }
        if paragraph.sentences.len() >= MAX_SENTENCES_IN_PARAGRAPH {
            break;
        }
    }

    Some(paragraph)
}

// ---------------------------------------------------------------------------
// Convenience: strip soft hyphens
// Origin: utils/StringUtils.cpp:206-217 (stripSpecialCharsForMalaga)
// ---------------------------------------------------------------------------

/// Strip soft hyphens (U+00AD) from a word token's text.
///
/// Used before morphological analysis, since the analyzer does not recognize
/// words containing soft hyphens.
///
/// Origin: utils/StringUtils.cpp:206-217
pub(crate) fn strip_soft_hyphens(text: &[char]) -> Vec<char> {
    text.iter().copied().filter(|&c| c != '\u{00AD}').collect()
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- GrammarToken tests --

    #[test]
    fn grammar_token_new_defaults() {
        let token = GrammarToken::new(TokenType::Word, vec!['k', 'o', 'i', 'r', 'a'], 0);
        assert_eq!(token.token_type, TokenType::Word);
        assert_eq!(token.text, vec!['k', 'o', 'i', 'r', 'a']);
        assert_eq!(token.pos, 0);
        assert_eq!(token.token_len(), 5);
        assert!(!token.is_valid_word);
        assert!(!token.first_letter_lcase);
        assert!(!token.possible_sentence_start);
        assert!(!token.is_geographical_name_in_genitive);
        assert!(!token.possible_geographical_name);
        assert!(!token.possible_main_verb);
        assert!(!token.is_main_verb);
        assert!(!token.is_verb_negative);
        assert!(!token.is_positive_verb);
        assert!(!token.is_conjunction);
        assert!(!token.possible_conjunction);
        assert_eq!(token.require_following_verb, FollowingVerbType::None);
        assert_eq!(token.verb_follower_type, FollowingVerbType::None);
    }

    #[test]
    fn grammar_token_with_position() {
        let token = GrammarToken::new(TokenType::Punctuation, vec!['.'], 10);
        assert_eq!(token.pos, 10);
        assert_eq!(token.token_len(), 1);
    }

    // -- GrammarSentence tests --

    #[test]
    fn grammar_sentence_new() {
        let sentence = GrammarSentence::new(5);
        assert_eq!(sentence.pos, 5);
        assert!(sentence.tokens.is_empty());
        assert_eq!(sentence.sentence_type, SentenceType::None);
    }

    // -- Paragraph tests --

    #[test]
    fn paragraph_new() {
        let paragraph = Paragraph::new();
        assert!(paragraph.sentences.is_empty());
    }

    // -- FollowingVerbType tests --

    #[test]
    fn following_verb_type_equality() {
        assert_eq!(FollowingVerbType::None, FollowingVerbType::None);
        assert_ne!(FollowingVerbType::AInfinitive, FollowingVerbType::MaInfinitive);
    }

    // -- strip_soft_hyphens tests --

    #[test]
    fn strip_soft_hyphens_no_hyphens() {
        let text: Vec<char> = "koira".chars().collect();
        assert_eq!(strip_soft_hyphens(&text), text);
    }

    #[test]
    fn strip_soft_hyphens_with_hyphens() {
        let text: Vec<char> = "koi\u{00AD}ra".chars().collect();
        let expected: Vec<char> = "koira".chars().collect();
        assert_eq!(strip_soft_hyphens(&text), expected);
    }

    #[test]
    fn strip_soft_hyphens_only_hyphens() {
        let text: Vec<char> = "\u{00AD}\u{00AD}".chars().collect();
        assert!(strip_soft_hyphens(&text).is_empty());
    }

    #[test]
    fn strip_soft_hyphens_empty() {
        assert!(strip_soft_hyphens(&[]).is_empty());
    }

    // -- analyse_paragraph integration tests --

    #[test]
    fn analyse_paragraph_empty_text() {
        let text: Vec<char> = Vec::new();
        let mut noop = |_: &mut GrammarToken| {};
        let result = analyse_paragraph(&text, 0, &mut noop);
        assert!(result.is_some());
        let p = result.unwrap();
        // An empty text produces no sentences (the loop exits immediately
        // because pos >= remaining_total).
        assert!(p.sentences.is_empty());
    }

    #[test]
    fn analyse_paragraph_single_word() {
        let text: Vec<char> = "koira".chars().collect();
        let text_len = text.len();
        let mut noop = |_: &mut GrammarToken| {};
        let result = analyse_paragraph(&text, text_len, &mut noop);
        assert!(result.is_some());
        let p = result.unwrap();
        assert!(!p.sentences.is_empty());
        // The word "koira" should appear as a token in the first sentence.
        let words: Vec<String> = p.sentences[0]
            .tokens
            .iter()
            .filter(|t| t.token_type == TokenType::Word)
            .map(|t| t.text.iter().collect())
            .collect();
        assert_eq!(words, vec!["koira"]);
    }

    #[test]
    fn analyse_paragraph_two_sentences() {
        let text: Vec<char> = "Koira juoksi. Kissa nukkui.".chars().collect();
        let text_len = text.len();
        let mut noop = |_: &mut GrammarToken| {};
        let result = analyse_paragraph(&text, text_len, &mut noop);
        assert!(result.is_some());
        let p = result.unwrap();
        // Should have at least 2 sentences.
        assert!(p.sentences.len() >= 2);
    }

    #[test]
    fn analyse_paragraph_token_positions_are_paragraph_relative() {
        let text: Vec<char> = "Hei! Moi.".chars().collect();
        let text_len = text.len();
        let mut noop = |_: &mut GrammarToken| {};
        let result = analyse_paragraph(&text, text_len, &mut noop).unwrap();

        // Collect all word token positions.
        let positions: Vec<usize> = result
            .sentences
            .iter()
            .flat_map(|s| s.tokens.iter())
            .filter(|t| t.token_type == TokenType::Word)
            .map(|t| t.pos)
            .collect();

        // "Hei" starts at 0, "Moi" starts at 5.
        assert!(positions.contains(&0));
        assert!(positions.contains(&5));
    }

    #[test]
    fn analyse_paragraph_calls_analyse_fn() {
        let text: Vec<char> = "koira kissa".chars().collect();
        let text_len = text.len();
        let result = analyse_paragraph(&text, text_len, &mut |token: &mut GrammarToken| {
            if token.token_type == TokenType::Word {
                // Mark all words as valid.
                token.is_valid_word = true;
            }
        });
        assert!(result.is_some());
        let p = result.unwrap();
        // The analysis function should have been called for each word token.
        let word_tokens: Vec<_> = p.sentences[0]
            .tokens
            .iter()
            .filter(|t| t.token_type == TokenType::Word)
            .collect();
        assert_eq!(word_tokens.len(), 2);
        assert!(word_tokens.iter().all(|t| t.is_valid_word));
    }

    #[test]
    fn analyse_paragraph_two_sentences_split_at_period() {
        // "Koira juoksi. Kissa nukkui." should split into two sentences.
        // The period is at the sentence boundary, so each sentence is separate.
        let text: Vec<char> = "Koira juoksi. Kissa nukkui.".chars().collect();
        let text_len = text.len();
        let mut noop = |_: &mut GrammarToken| {};
        let result = analyse_paragraph(&text, text_len, &mut noop).unwrap();

        // Should have at least 2 sentences.
        assert!(result.sentences.len() >= 2);

        // First sentence should contain "Koira" and "juoksi".
        let first_words: Vec<String> = result.sentences[0]
            .tokens
            .iter()
            .filter(|t| t.token_type == TokenType::Word)
            .map(|t| t.text.iter().collect())
            .collect();
        assert!(first_words.contains(&"Koira".to_string()));
        assert!(first_words.contains(&"juoksi".to_string()));
    }

    #[test]
    fn analyse_paragraph_intra_sentence_possible_start() {
        // Colons produce SentenceType::Possible, which gets merged into a
        // single sentence. The word after the colon should be marked.
        let text: Vec<char> = "Huom: kissa juoksi. Loppu.".chars().collect();
        let text_len = text.len();
        let mut noop = |_: &mut GrammarToken| {};
        let result = analyse_paragraph(&text, text_len, &mut noop).unwrap();

        // Within the first sentence (which includes "Huom: kissa juoksi."),
        // "kissa" should have possible_sentence_start = true because ":" is
        // a sentence-separating punctuation.
        let first_sentence = &result.sentences[0];
        let kissa_token = first_sentence
            .tokens
            .iter()
            .find(|t| {
                t.token_type == TokenType::Word && t.text.iter().collect::<String>() == "kissa"
            });

        // The colon is a sentence-separating punctuation, so possible_sentence_start
        // should be set on the word following it.
        let token = kissa_token.expect("Expected 'kissa' token in first sentence");
        assert!(
            token.possible_sentence_start,
            "Expected 'kissa' to have possible_sentence_start after colon"
        );
    }

    #[test]
    fn analyse_paragraph_sentence_type_propagated() {
        let text: Vec<char> = "Koira juoksi. Kissa.".chars().collect();
        let text_len = text.len();
        let mut noop = |_: &mut GrammarToken| {};
        let result = analyse_paragraph(&text, text_len, &mut noop).unwrap();

        // The first sentence should have Probable type (period followed by space + word).
        // The second sentence should have None type (end of text).
        assert!(result.sentences.len() >= 2);
        assert_eq!(result.sentences[0].sentence_type, SentenceType::Probable);
        assert_eq!(
            result.sentences[result.sentences.len() - 1].sentence_type,
            SentenceType::None
        );
    }
}
