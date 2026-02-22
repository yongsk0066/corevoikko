// Individual grammar check rules for Finnish.
//
// Each check is an independent function that takes annotated tokens and
// returns a list of grammar errors. The checks are composed by the engine
// module (FinnishRuleEngine).
#![allow(dead_code)]
//
// Origin: grammar/FinnishRuleEngine/checks.cpp
//         grammar/FinnishRuleEngine/CapitalizationCheck.cpp
//         grammar/FinnishRuleEngine/MissingVerbCheck.cpp
//         grammar/FinnishRuleEngine/NegativeVerbCheck.cpp
//         grammar/FinnishRuleEngine/CompoundVerbCheck.cpp
//         grammar/FinnishRuleEngine/SidesanaCheck.cpp

use voikko_core::character::{
    equals_ignore_case, is_finnish_quotation_mark, is_lower, is_upper, simple_lower, simple_upper,
};
use voikko_core::enums::TokenType;
use voikko_core::grammar_error::{
    GrammarError, GCERR_A_INFINITIVE_REQUIRED, GCERR_EXTRA_COMMA, GCERR_EXTRA_MAIN_VERB,
    GCERR_EXTRA_WHITESPACE, GCERR_FOREIGN_QUOTATION_MARK,
    GCERR_INVALID_PUNCTUATION_AT_END_OF_QUOTATION, GCERR_INVALID_SENTENCE_STARTER,
    GCERR_MA_INFINITIVE_REQUIRED, GCERR_MISPLACED_CLOSING_PARENTHESIS, GCERR_MISPLACED_SIDESANA,
    GCERR_MISSING_MAIN_VERB, GCERR_NEGATIVE_VERB_MISMATCH, GCERR_REPEATING_WORD,
    GCERR_SPACE_BEFORE_PUNCTUATION, GCERR_TERMINATING_PUNCTUATION_MISSING,
    GCERR_WRITE_FIRST_LOWERCASE, GCERR_WRITE_FIRST_UPPERCASE,
};

use voikko_core::case::{detect_case, CaseType};

// Re-export types from paragraph module for use by other grammar submodules.
pub(crate) use super::paragraph::{FollowingVerbType, GrammarSentence, GrammarToken, Paragraph};

// ============================================================================
// Convenience type aliases and additional types
// ============================================================================

/// Alias for `Paragraph` to match the name used in this module's API.
///
/// Origin: grammar/Paragraph.hpp:41-59
pub(crate) type GrammarParagraph = Paragraph;

/// Grammar checker options relevant to individual checks.
///
/// Origin: setup/setup.hpp (VoikkoHandle boolean options)
#[derive(Debug, Clone)]
#[derive(Default)]
pub(crate) struct GrammarOptions {
    /// Accept incomplete sentences in titles. Default: false.
    /// Origin: voikko_defines.h:117
    pub accept_titles_in_gc: bool,

    /// Accept incomplete sentences at end of paragraph. Default: false.
    /// Origin: voikko_defines.h:122
    pub accept_unfinished_paragraphs_in_gc: bool,

    /// Accept paragraphs valid within bulleted lists. Default: false.
    /// Origin: voikko_defines.h:131
    pub accept_bulleted_lists_in_gc: bool,
}



// ============================================================================
// Punctuation checks
// Origin: checks.cpp:45-238
// ============================================================================

/// GC errors due to wrong context-independent use of punctuation or whitespace
/// within a sentence.
///
/// Detects:
/// - Extra whitespace (GCERR_EXTRA_WHITESPACE = 2)
/// - Space before comma (GCERR_SPACE_BEFORE_PUNCTUATION = 3)
/// - Invalid sentence starter (GCERR_INVALID_SENTENCE_STARTER = 5)
/// - Extra comma (GCERR_EXTRA_COMMA = 4)
///
/// Origin: checks.cpp:45-117 (gc_local_punctuation)
pub(crate) fn gc_local_punctuation(sentence: &GrammarSentence) -> Vec<GrammarError> {
    let mut errors = Vec::new();
    let tokens = &sentence.tokens;
    let count = tokens.len();
    let mut i = 0;

    while i < count {
        let t = &tokens[i];
        match t.token_type {
            TokenType::Whitespace => {
                if t.token_len() > 1 {
                    // Extra whitespace
                    errors.push(GrammarError::with_suggestions(
                        GCERR_EXTRA_WHITESPACE,
                        t.pos,
                        t.token_len(),
                        vec![" ".to_string()],
                    ));
                } else if i + 1 < count {
                    let t2 = &tokens[i + 1];
                    if t2.token_type == TokenType::Punctuation
                        && t2.text.first().copied() == Some(',')
                    {
                        // Space before comma
                        errors.push(GrammarError::with_suggestions(
                            GCERR_SPACE_BEFORE_PUNCTUATION,
                            t.pos,
                            2,
                            vec![",".to_string()],
                        ));
                    }
                }
            }
            TokenType::Punctuation => {
                // Skip [N] constructs (footnote references)
                if t.text.first().copied() == Some('[')
                    && i + 2 < count
                    && tokens[i + 2].text.first().copied() == Some(']')
                {
                    i += 3;
                    continue;
                }
                if i == 0 {
                    // Invalid sentence starter
                    let ch = t.text.first().copied().unwrap_or('\0');
                    if matches!(ch, '(' | ')' | '\'' | '-' | '\u{201C}' | '\u{2013}' | '\u{2014}')
                        || is_finnish_quotation_mark(ch)
                    {
                        i += 1;
                        continue;
                    }
                    let (start_pos, error_len) = if t.pos == 0 {
                        (0, 1)
                    } else {
                        (t.pos - 1, 2)
                    };
                    errors.push(GrammarError::new(
                        GCERR_INVALID_SENTENCE_STARTER,
                        start_pos,
                        error_len,
                    ));
                    i += 1;
                    continue;
                }
                // Consecutive commas
                if t.text.first().copied() == Some(',') && i + 1 < count {
                    let t2 = &tokens[i + 1];
                    if t2.token_type == TokenType::Punctuation && t2.text.first().copied() == Some(',') {
                        errors.push(GrammarError::with_suggestions(
                            GCERR_EXTRA_COMMA,
                            t.pos,
                            2,
                            vec![",".to_string()],
                        ));
                    }
                }
            }
            TokenType::None | TokenType::Word | TokenType::Unknown => {}
        }
        i += 1;
    }

    errors
}

/// GC errors due to wrong punctuation in quotations.
///
/// Detects:
/// - Foreign quotation mark U+201C (GCERR_FOREIGN_QUOTATION_MARK = 11)
/// - Wrong punctuation order at end of quotation (GCERR_INVALID_PUNCTUATION_AT_END_OF_QUOTATION = 10)
///
/// Origin: checks.cpp:120-185 (gc_punctuation_of_quotations)
pub(crate) fn gc_punctuation_of_quotations(sentence: &GrammarSentence) -> Vec<GrammarError> {
    let mut errors = Vec::new();
    let tokens = &sentence.tokens;
    let count = tokens.len();

    let mut i = 0;
    while i + 2 < count {
        if tokens[i].token_type != TokenType::Punctuation {
            i += 1;
            continue;
        }

        let ch = tokens[i].text.first().copied().unwrap_or('\0');

        // Foreign quotation mark U+201C
        if ch == '\u{201C}' {
            errors.push(GrammarError::with_suggestions(
                GCERR_FOREIGN_QUOTATION_MARK,
                tokens[i].pos,
                1,
                vec!["\u{201D}".to_string()],
            ));
            return errors; // Stop processing after first foreign quotation mark
        }

        if tokens[i + 1].token_type != TokenType::Punctuation {
            i += 1;
            continue;
        }
        let q_char = tokens[i + 1].text.first().copied().unwrap_or('\0');
        if !is_finnish_quotation_mark(q_char) {
            i += 1;
            continue;
        }

        if tokens[i + 2].token_type != TokenType::Punctuation {
            i += 1;
            continue;
        }
        if tokens[i + 2].text.first().copied() != Some(',') {
            i += 1;
            continue;
        }

        match ch {
            '.' => {
                // ."  ,  ->  should be  ",
                let suggestion = format!("{},", q_char);
                errors.push(GrammarError::with_suggestions(
                    GCERR_INVALID_PUNCTUATION_AT_END_OF_QUOTATION,
                    tokens[i].pos,
                    3,
                    vec![suggestion],
                ));
            }
            '!' | '?' => {
                // !"  ,  ->  !"    or    ?"  ,  ->  ?"
                let suggestion = format!("{}{}", ch, q_char);
                errors.push(GrammarError::with_suggestions(
                    GCERR_INVALID_PUNCTUATION_AT_END_OF_QUOTATION,
                    tokens[i].pos,
                    3,
                    vec![suggestion],
                ));
            }
            _ => {}
        }

        i += 1;
    }

    errors
}

/// GC errors due to word repetition.
///
/// Detects consecutive identical words separated by whitespace, ignoring
/// certain Finnish words that can be legitimately repeated ("ollut",
/// "olleet", "sillä") and words starting with digits.
///
/// Origin: checks.cpp:187-223 (gc_repeating_words)
pub(crate) fn gc_repeating_words(sentence: &GrammarSentence) -> Vec<GrammarError> {
    let mut errors = Vec::new();
    let tokens = &sentence.tokens;
    let count = tokens.len();
    let mut i = 0;

    while i + 2 < count {
        if tokens[i].token_type != TokenType::Word {
            i += 1;
            continue;
        }
        if tokens[i + 1].token_type != TokenType::Whitespace {
            i += 2;
            continue;
        }
        if tokens[i + 2].token_type != TokenType::Word {
            i += 3;
            continue;
        }

        // Case-insensitive comparison
        if !equals_ignore_case(&tokens[i].text, &tokens[i + 2].text) {
            i += 1;
            continue;
        }

        // Skip words starting with digits
        if let Some(c) = tokens[i].text.first().copied() {
            if c.is_ascii_digit() {
                i += 1;
                continue;
            }
        }

        // Skip words that can be legitimately repeated
        let word_str = tokens[i].text.iter().collect::<String>();
        if word_str == "ollut" || word_str == "olleet" || word_str == "sill\u{00e4}" {
            i += 1;
            continue;
        }

        let error_len =
            tokens[i].token_len() + tokens[i + 1].token_len() + tokens[i + 2].token_len();
        errors.push(GrammarError::with_suggestions(
            GCERR_REPEATING_WORD,
            tokens[i].pos,
            error_len,
            vec![tokens[i].text.iter().collect::<String>()],
        ));

        i += 1;
    }

    errors
}

/// GC error for missing punctuation at the end of a paragraph.
///
/// Origin: checks.cpp:225-238 (gc_end_punctuation)
pub(crate) fn gc_end_punctuation(
    paragraph: &GrammarParagraph,
    options: &GrammarOptions,
) -> Vec<GrammarError> {
    if options.accept_titles_in_gc && paragraph.sentences.len() == 1 {
        return Vec::new();
    }
    if options.accept_unfinished_paragraphs_in_gc {
        return Vec::new();
    }
    if options.accept_bulleted_lists_in_gc {
        return Vec::new();
    }

    let Some(last_sentence) = paragraph.sentences.last() else {
        return Vec::new();
    };
    let Some(last_token) = last_sentence.tokens.last() else {
        return Vec::new();
    };

    if last_token.token_type == TokenType::Punctuation {
        return Vec::new();
    }

    vec![GrammarError::new(
        GCERR_TERMINATING_PUNCTUATION_MISSING,
        last_token.pos,
        last_token.token_len(),
    )]
}

// ============================================================================
// Finnish verb rule checks
// Origin: MissingVerbCheck.cpp, NegativeVerbCheck.cpp,
//         CompoundVerbCheck.cpp, SidesanaCheck.cpp
// ============================================================================

/// Check for sentences missing a main verb.
///
/// A sentence is flagged if it has 2+ words, ends with '.' or '?'
/// (but not '...'), and contains no verb, verb negative, or unrecognized word.
/// Also detects extra main verbs within clauses.
///
/// Origin: MissingVerbCheck.cpp:35-110
pub(crate) fn gc_missing_verb(
    sentence: &GrammarSentence,
    options: &GrammarOptions,
) -> Vec<GrammarError> {
    let mut errors = Vec::new();
    let tokens = &sentence.tokens;

    if tokens.is_empty() {
        return errors;
    }

    let first_token = &tokens[0];

    // If sentence starts with punctuation, skip
    if first_token.token_type == TokenType::Punctuation {
        return errors;
    }

    // Bulleted lists: skip if sentence is at paragraph start and starts with
    // lowercase or its first letter is not a lowercase form
    if options.accept_bulleted_lists_in_gc
        && sentence.pos == 0
        && (first_token.text.first().copied().is_some_and(is_lower)
            || !first_token.first_letter_lcase)
    {
        return errors;
    }

    let mut word_count = 0;
    let mut last_non_whitespace: Option<&GrammarToken> = None;
    let mut found_verb_in_sentence = false;
    let mut found_verb_in_current_clause = false;
    let mut last_verb_start_token: usize = 0;

    for (i, token) in tokens.iter().enumerate() {
        // Tab character means we should skip
        if token.text.first().copied() == Some('\t') {
            return errors;
        }

        if token.token_type == TokenType::Word {
            word_count += 1;

            // An unrecognized word, a possible main verb, or a negative verb
            // all satisfy the "verb found" requirement
            if !token.is_valid_word || token.possible_main_verb || token.is_verb_negative {
                found_verb_in_sentence = true;
            }

            if token.possible_conjunction {
                found_verb_in_current_clause = false;
            } else if i + 2 < tokens.len()
                && starts_with_chars(&tokens[i].text, &chars("siin\u{00e4}"))
                && starts_with_chars(&tokens[i + 2].text, &chars("miss\u{00e4}"))
            {
                // "siinä missä" can separate clauses without a comma
                found_verb_in_current_clause = false;
            } else if i + 2 < tokens.len()
                && starts_with_chars(&tokens[i].text, &chars("k\u{00e4}vi"))
                && starts_with_chars(&tokens[i + 2].text, &chars("miten"))
            {
                // "kävi miten kävi" does not require a comma
                found_verb_in_current_clause = false;
            } else if token.is_main_verb {
                if found_verb_in_current_clause {
                    // Extra main verb detected — but suppress if it would also
                    // be caught by the repeating word check
                    let suppress = i == last_verb_start_token + 2
                        && token.token_len() == tokens[last_verb_start_token].token_len()
                        && tokens[last_verb_start_token].text == token.text;
                    if !suppress {
                        let start_pos = tokens[last_verb_start_token].pos;
                        let error_len = token.pos + token.token_len() - start_pos;
                        errors.push(GrammarError::new(
                            GCERR_EXTRA_MAIN_VERB,
                            start_pos,
                            error_len,
                        ));
                    }
                    found_verb_in_current_clause = false;
                } else {
                    found_verb_in_current_clause = true;
                    last_verb_start_token = i;
                }
            }
        } else if token.token_type == TokenType::Punctuation {
            found_verb_in_current_clause = false;
        }

        if token.token_type != TokenType::Whitespace {
            last_non_whitespace = Some(token);
        }
    }

    // Determine if we should report a missing verb
    let Some(last_nw) = last_non_whitespace else {
        return errors;
    };

    if found_verb_in_sentence || word_count < 2 {
        return errors;
    }

    let last_char = last_nw.text.first().copied().unwrap_or('\0');
    if last_char != '.' && last_char != '?' {
        return errors;
    }
    // "..." (ellipsis) is not a sentence-ending period
    if last_nw.text.iter().collect::<String>() == "..." {
        return errors;
    }

    let error_len = last_nw.pos + last_nw.token_len() - sentence.pos;
    errors.push(GrammarError::new(
        GCERR_MISSING_MAIN_VERB,
        sentence.pos,
        error_len,
    ));

    errors
}

/// Check for negative verb followed by positive verb form mismatch.
///
/// Detects patterns like "en syön" (negative verb + positive verb form)
/// where the correct form would be "en syö" (negative verb + connegative).
///
/// Origin: NegativeVerbCheck.cpp:36-52
pub(crate) fn gc_negative_verb_mismatch(sentence: &GrammarSentence) -> Vec<GrammarError> {
    let mut errors = Vec::new();
    let tokens = &sentence.tokens;
    let count = tokens.len();

    let mut i = 0;
    while i + 2 < count {
        let token = &tokens[i];
        if token.token_type == TokenType::Word
            && tokens[i + 1].token_type == TokenType::Whitespace
            && tokens[i + 2].token_type == TokenType::Word
        {
            let word2 = &tokens[i + 2];
            if token.is_verb_negative && word2.is_positive_verb {
                let error_len = word2.pos + word2.token_len() - token.pos;
                errors.push(GrammarError::new(
                    GCERR_NEGATIVE_VERB_MISMATCH,
                    token.pos,
                    error_len,
                ));
            }
        }
        i += 1;
    }

    errors
}

/// Check for compound verb infinitive type mismatch.
///
/// Detects:
/// - A-infinitive required (GCERR_A_INFINITIVE_REQUIRED = 14)
///   e.g. "haluan syömään" instead of "haluan syödä"
/// - MA-infinitive required (GCERR_MA_INFINITIVE_REQUIRED = 15)
///   e.g. "menen syödä" instead of "menen syömään"
///
/// Origin: CompoundVerbCheck.cpp:36-60
pub(crate) fn gc_compound_verb(sentence: &GrammarSentence) -> Vec<GrammarError> {
    let mut errors = Vec::new();
    let tokens = &sentence.tokens;
    let count = tokens.len();

    let mut i = 0;
    while i + 2 < count {
        let token = &tokens[i];
        if token.token_type == TokenType::Word
            && tokens[i + 1].token_type == TokenType::Whitespace
            && tokens[i + 2].token_type == TokenType::Word
        {
            let word2 = &tokens[i + 2];
            if token.require_following_verb == FollowingVerbType::AInfinitive
                && word2.verb_follower_type == FollowingVerbType::MaInfinitive
            {
                let error_len = word2.pos + word2.token_len() - token.pos;
                errors.push(GrammarError::new(
                    GCERR_A_INFINITIVE_REQUIRED,
                    token.pos,
                    error_len,
                ));
            } else if token.require_following_verb == FollowingVerbType::MaInfinitive
                && word2.verb_follower_type == FollowingVerbType::AInfinitive
            {
                let error_len = word2.pos + word2.token_len() - token.pos;
                errors.push(GrammarError::new(
                    GCERR_MA_INFINITIVE_REQUIRED,
                    token.pos,
                    error_len,
                ));
            }
        }
        i += 1;
    }

    errors
}

/// Check for misplaced conjunction at the end of a sentence.
///
/// A conjunction (other than "vaan") followed by a period at the end of
/// a sentence is flagged as a misplaced conjunction.
///
/// Origin: SidesanaCheck.cpp:36-52
pub(crate) fn gc_sidesana(sentence: &GrammarSentence) -> Vec<GrammarError> {
    let tokens = &sentence.tokens;
    let mut token_count = tokens.len();

    if token_count == 0 {
        return Vec::new();
    }

    // Strip trailing whitespace
    if tokens[token_count - 1].token_type == TokenType::Whitespace {
        token_count -= 1;
    }

    if token_count < 2 {
        return Vec::new();
    }

    let second_last = &tokens[token_count - 2];
    let last = &tokens[token_count - 1];

    if second_last.is_conjunction
        && second_last.text.iter().collect::<String>() != "vaan"
        && last.token_type == TokenType::Punctuation
        && last.text.iter().collect::<String>() == "."
    {
        return vec![GrammarError::new(
            GCERR_MISPLACED_SIDESANA,
            second_last.pos,
            second_last.token_len(),
        )];
    }

    Vec::new()
}

// ============================================================================
// Capitalization check (5-state FSA)
// Origin: CapitalizationCheck.cpp:43-377
// ============================================================================

/// Internal context for the capitalization FSA.
struct CapitalizationContext<'a> {
    paragraph: &'a GrammarParagraph,
    current_sentence: usize,
    current_token: usize,
    token_before_next_word: Option<&'a GrammarToken>,
    next_word: Option<&'a GrammarToken>,
    options: &'a GrammarOptions,
    quotes: Vec<char>,
    sentence_ended: bool,
    errors: Vec<GrammarError>,
}

/// Capitalization FSA states.
///
/// Origin: CapitalizationCheck.cpp:56-62
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CapState {
    Initial,
    Upper,
    Lower,
    DontCare,
    Quoted,
}

/// Check whether a sentence is written fully in upper case letters (or
/// contains a foreign quotation mark). If so, capitalization checks should
/// be skipped for that sentence.
///
/// Origin: CapitalizationCheck.cpp:64-89
fn should_skip_sentence(sentence: &GrammarSentence) -> bool {
    let mut only_upper = true;
    for t in &sentence.tokens {
        if t.text.first().copied() == Some('\u{201C}') {
            return true;
        }
        if t.token_type != TokenType::Word {
            continue;
        }
        for &c in &t.text {
            if is_lower(c) {
                only_upper = false;
                break;
            }
        }
        if !only_upper {
            break;
        }
    }
    only_upper
}

/// Get the next token from the paragraph and advance the context's position.
/// Skips sentences that should be skipped (all-uppercase, foreign quotation).
///
/// Origin: CapitalizationCheck.cpp:91-112
fn get_token_and_advance<'a>(ctx: &mut CapitalizationContext<'a>) -> Option<&'a GrammarToken> {
    loop {
        if ctx.current_sentence >= ctx.paragraph.sentences.len() {
            return None;
        }
        let sentence = &ctx.paragraph.sentences[ctx.current_sentence];
        if should_skip_sentence(sentence) {
            ctx.current_sentence += 1;
            continue;
        }

        let token = &sentence.tokens[ctx.current_token];
        ctx.current_token += 1;
        if ctx.current_token >= sentence.tokens.len() {
            ctx.current_token = 0;
            ctx.current_sentence += 1;
        }
        return Some(token);
    }
}

/// Collect all tokens between the current position and the next word token.
/// Sets `ctx.next_word` to the next word, and `ctx.token_before_next_word`
/// to the last non-word token before it.
///
/// Origin: CapitalizationCheck.cpp:114-131
fn get_tokens_until_next_word<'a>(
    ctx: &mut CapitalizationContext<'a>,
) -> Vec<&'a GrammarToken> {
    let mut tokens = Vec::new();
    ctx.token_before_next_word = ctx.next_word;
    loop {
        let Some(token) = get_token_and_advance(ctx) else {
            ctx.next_word = None;
            break;
        };
        if token.token_type == TokenType::Word {
            ctx.next_word = Some(token);
            break;
        }
        tokens.push(token);
        ctx.token_before_next_word = Some(token);
    }
    tokens
}

/// Check whether any token in the list has text matching `expected_text`.
///
/// Origin: CapitalizationCheck.cpp:133-142
fn contains_token_text(tokens: &[&GrammarToken], expected_text: &str) -> bool {
    let expected: Vec<char> = expected_text.chars().collect();
    tokens.iter().any(|t| t.text == expected)
}

/// Check whether the last punctuation token in the list ends a sentence
/// (i.e. is '.', '?', or '!', but not ',').
///
/// Origin: CapitalizationCheck.cpp:144-153
fn last_punctuation_ends_sentence(tokens: &[&GrammarToken]) -> bool {
    for t in tokens.iter().rev() {
        if t.token_type == TokenType::Punctuation && t.text.first().copied() != Some(',') {
            return matches!(t.text.first().copied(), Some('.' | '?' | '!'));
        }
    }
    false
}

/// Check whether the word is a geographical name in genitive and the
/// separator is a single space (place name in institution name).
///
/// Origin: CapitalizationCheck.cpp:155-158
fn place_name_in_institution_name(word: &GrammarToken, separators: &[&GrammarToken]) -> bool {
    word.is_geographical_name_in_genitive
        && separators.len() == 1
        && separators[0].text.first().copied() == Some(' ')
}

/// Check whether a word is a possible list item (single char, chapter number,
/// or roman numeral) followed by a closing parenthesis.
///
/// Origin: CapitalizationCheck.cpp:220-228
fn is_list_item_and_closing_parenthesis(
    word: &GrammarToken,
    separators: &[&GrammarToken],
) -> bool {
    if separators.is_empty() || separators[0].text.first().copied() != Some(')') {
        return false;
    }
    is_possible_list_item(&word.text)
}

/// Check if a word is a possible list item (single char, chapter number, or
/// roman numeral).
///
/// Origin: StringUtils.cpp:262-273
fn is_possible_list_item(word: &[char]) -> bool {
    if word.len() == 1 {
        return true;
    }
    if is_chapter_number(word) {
        return true;
    }
    if is_roman_numeral(word) {
        return true;
    }
    false
}

/// Check if a string is a positive integer (digits only).
///
/// Origin: StringUtils.cpp:219-226
fn is_integer(word: &[char]) -> bool {
    if word.is_empty() {
        return false;
    }
    word.iter().all(|&c| c.is_ascii_digit())
}

/// Check if a string is a chapter number (e.g. "3", "3.4", "3.65.3").
///
/// Origin: StringUtils.cpp:232-249
fn is_chapter_number(word: &[char]) -> bool {
    if word.is_empty() {
        return false;
    }
    let mut dot_last = false;
    for (i, &c) in word.iter().enumerate() {
        if c == '.' {
            if i == 0 || dot_last {
                return false;
            }
            dot_last = true;
        } else if !c.is_ascii_digit() {
            return false;
        } else {
            dot_last = false;
        }
    }
    !dot_last
}

/// Check if a string is a roman numeral (very simple check).
///
/// Origin: StringUtils.cpp:251-259
fn is_roman_numeral(word: &[char]) -> bool {
    if word.is_empty() {
        return false;
    }
    word.iter()
        .all(|&c| matches!(c, 'i' | 'I' | 'v' | 'V' | 'x' | 'X'))
}

/// Push and pop quotation marks from the stack; report misplaced closing
/// parentheses and detect sentence-ending punctuation.
///
/// Returns `true` if quote characters were found.
///
/// Origin: CapitalizationCheck.cpp:161-202
fn push_and_pop_quotes(
    ctx: &mut CapitalizationContext<'_>,
    tokens: &[&GrammarToken],
) -> bool {
    let mut has_quotes = false;
    for t in tokens {
        if t.token_type == TokenType::Punctuation {
            let ch = t.text.first().copied().unwrap_or('\0');
            if is_finnish_quotation_mark(ch) {
                has_quotes = true;
                if ctx.quotes.is_empty() {
                    ctx.quotes.push(ch);
                } else {
                    let &previous = ctx.quotes.last().unwrap();
                    if previous == ch {
                        ctx.quotes.pop();
                    } else {
                        ctx.quotes.push(ch);
                    }
                }
            } else if ch == '(' || ch == '[' {
                ctx.quotes.push(ch);
            } else if ch == ')' || ch == ']' {
                if ctx.quotes.is_empty() {
                    ctx.errors.push(GrammarError::new(
                        GCERR_MISPLACED_CLOSING_PARENTHESIS,
                        t.pos,
                        1,
                    ));
                } else if ctx.quotes.last() == Some(&'(')
                    || ctx.quotes.last() == Some(&'[')
                {
                    ctx.quotes.pop();
                }
            } else if matches!(ch, '.' | '!' | '?') {
                ctx.sentence_ended = true;
            }
        }
    }
    has_quotes
}

/// INITIAL state: collect separators until the first word.
///
/// Origin: CapitalizationCheck.cpp:204-218
fn in_initial(ctx: &mut CapitalizationContext<'_>) -> CapState {
    let separators = get_tokens_until_next_word(ctx);
    push_and_pop_quotes(ctx, &separators);
    if !ctx.quotes.is_empty() {
        return CapState::Quoted;
    }
    if ctx.options.accept_bulleted_lists_in_gc {
        return CapState::DontCare;
    }
    if contains_token_text(&separators, "-") {
        return CapState::DontCare;
    }
    CapState::Upper
}

/// UPPER state: the next word is expected to start with an uppercase letter.
///
/// Origin: CapitalizationCheck.cpp:230-272
fn in_upper(ctx: &mut CapitalizationContext<'_>) -> CapState {
    let token_before_word = ctx.token_before_next_word;
    let Some(word) = ctx.next_word else {
        return CapState::DontCare;
    };
    let separators = get_tokens_until_next_word(ctx);

    if is_list_item_and_closing_parenthesis(word, &separators) {
        let separators_rest = if separators.len() > 1 {
            &separators[1..]
        } else {
            &[]
        };
        push_and_pop_quotes(ctx, separators_rest);
        return CapState::DontCare;
    }

    if let Some(first_ch) = word.text.first().copied() {
        if !is_upper(first_ch) && !first_ch.is_ascii_digit() && !word.possible_sentence_start {
            // Error: should start with uppercase
            let mut suggestion_chars = word.text.clone();
            suggestion_chars[0] = simple_upper(suggestion_chars[0]);
            let suggestion: String = suggestion_chars.iter().collect();
            ctx.errors.push(GrammarError::with_suggestions(
                GCERR_WRITE_FIRST_UPPERCASE,
                word.pos,
                word.token_len(),
                vec![suggestion],
            ));
        }
    }

    push_and_pop_quotes(ctx, &separators);
    if !ctx.quotes.is_empty() {
        return CapState::Quoted;
    }
    if contains_token_text(&separators, "\t")
        || place_name_in_institution_name(word, &separators)
    {
        return CapState::DontCare;
    }
    if ctx.options.accept_titles_in_gc && is_chapter_number(&word.text) {
        return CapState::DontCare;
    }
    if is_integer(&word.text) {
        if let Some(t) = token_before_word {
            if t.token_type != TokenType::Whitespace {
                return CapState::DontCare;
            }
        }
    }
    if last_punctuation_ends_sentence(&separators) {
        ctx.sentence_ended = true;
        return CapState::Upper;
    }
    CapState::Lower
}

/// LOWER state: the next word is expected to start with a lowercase letter.
///
/// Origin: CapitalizationCheck.cpp:274-315
fn in_lower(ctx: &mut CapitalizationContext<'_>) -> CapState {
    let Some(word) = ctx.next_word else {
        return CapState::DontCare;
    };

    if word.is_valid_word
        && word.first_letter_lcase
        && word.text.first().copied().is_some_and(is_upper)
        && !word.possible_sentence_start
        && word.token_len() > 1
        && word.text.get(1) != Some(&'-')
        && word.text.get(1) != Some(&':')
        && detect_case(&word.text) != CaseType::AllUpper
        && !word.possible_geographical_name
    {
        // Error: should start with lowercase
        let mut suggestion_chars = word.text.clone();
        suggestion_chars[0] = simple_lower(suggestion_chars[0]);
        let suggestion: String = suggestion_chars.iter().collect();
        ctx.errors.push(GrammarError::with_suggestions(
            GCERR_WRITE_FIRST_LOWERCASE,
            word.pos,
            word.token_len(),
            vec![suggestion],
        ));
    }

    let separators = get_tokens_until_next_word(ctx);
    if is_list_item_and_closing_parenthesis(word, &separators) {
        let separators_rest = if separators.len() > 1 {
            &separators[1..]
        } else {
            &[]
        };
        push_and_pop_quotes(ctx, separators_rest);
        return CapState::DontCare;
    }
    push_and_pop_quotes(ctx, &separators);
    if !ctx.quotes.is_empty() {
        return CapState::Quoted;
    }
    if contains_token_text(&separators, "\t")
        || place_name_in_institution_name(word, &separators)
    {
        return CapState::DontCare;
    }
    if last_punctuation_ends_sentence(&separators) {
        ctx.sentence_ended = true;
        return CapState::Upper;
    }
    CapState::Lower
}

/// DONT_CARE state: no capitalization expectation.
///
/// Origin: CapitalizationCheck.cpp:317-341
fn in_dont_care(ctx: &mut CapitalizationContext<'_>) -> CapState {
    let Some(word) = ctx.next_word else {
        return CapState::DontCare;
    };
    let separators = get_tokens_until_next_word(ctx);
    if is_list_item_and_closing_parenthesis(word, &separators) {
        let separators_rest = if separators.len() > 1 {
            &separators[1..]
        } else {
            &[]
        };
        push_and_pop_quotes(ctx, separators_rest);
        return CapState::DontCare;
    }
    push_and_pop_quotes(ctx, &separators);
    if !ctx.quotes.is_empty() {
        return CapState::Quoted;
    }
    if contains_token_text(&separators, "\t") {
        return CapState::DontCare;
    }
    if ctx.options.accept_titles_in_gc && is_chapter_number(&word.text) {
        return CapState::DontCare;
    }
    if last_punctuation_ends_sentence(&separators) {
        ctx.sentence_ended = true;
        return CapState::Upper;
    }
    CapState::Lower
}

/// QUOTED state: inside quotation marks.
///
/// Origin: CapitalizationCheck.cpp:343-358
fn in_quoted(ctx: &mut CapitalizationContext<'_>) -> CapState {
    let separators = get_tokens_until_next_word(ctx);
    let had_quotes = push_and_pop_quotes(ctx, &separators);
    if !ctx.quotes.is_empty() {
        return CapState::Quoted;
    }
    if last_punctuation_ends_sentence(&separators) {
        ctx.sentence_ended = false;
        return CapState::Upper;
    }
    if had_quotes || ctx.sentence_ended {
        ctx.sentence_ended = false;
        return CapState::DontCare;
    }
    CapState::Lower
}

/// Run the capitalization check on an entire paragraph.
///
/// This is the main entry point for the capitalization FSA. It walks through
/// all tokens across all sentences in the paragraph, checking that words
/// start with the appropriate case.
///
/// Origin: CapitalizationCheck.cpp:362-377
pub(crate) fn gc_capitalization(
    paragraph: &GrammarParagraph,
    options: &GrammarOptions,
) -> Vec<GrammarError> {
    let mut ctx = CapitalizationContext {
        paragraph,
        current_sentence: 0,
        current_token: 0,
        token_before_next_word: None,
        next_word: None,
        options,
        quotes: Vec::new(),
        sentence_ended: false,
        errors: Vec::new(),
    };

    let mut state = CapState::Initial;

    while ctx.current_sentence < ctx.paragraph.sentences.len()
        && ctx.current_token < ctx.paragraph.sentences[ctx.current_sentence].tokens.len()
    {
        state = match state {
            CapState::Initial => in_initial(&mut ctx),
            CapState::Upper => in_upper(&mut ctx),
            CapState::Lower => in_lower(&mut ctx),
            CapState::DontCare => in_dont_care(&mut ctx),
            CapState::Quoted => in_quoted(&mut ctx),
        };
    }

    ctx.errors
}

// ============================================================================
// Utility helpers
// ============================================================================

/// Convert a &str to Vec<char>.
fn chars(s: &str) -> Vec<char> {
    s.chars().collect()
}

/// Check if a char slice starts with the given prefix.
fn starts_with_chars(text: &[char], prefix: &[char]) -> bool {
    if text.len() < prefix.len() {
        return false;
    }
    text[..prefix.len()] == *prefix
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- Helper functions for building test token sequences --

    fn word(text: &str, pos: usize) -> GrammarToken {
        GrammarToken::new(TokenType::Word, text.chars().collect(), pos)
    }

    fn ws(text: &str, pos: usize) -> GrammarToken {
        GrammarToken::new(TokenType::Whitespace, text.chars().collect(), pos)
    }

    fn punct(text: &str, pos: usize) -> GrammarToken {
        GrammarToken::new(TokenType::Punctuation, text.chars().collect(), pos)
    }

    fn sentence(tokens: Vec<GrammarToken>, pos: usize) -> GrammarSentence {
        let mut s = GrammarSentence::new(pos);
        s.tokens = tokens;
        s
    }

    fn default_opts() -> GrammarOptions {
        GrammarOptions::default()
    }

    // ---- gc_local_punctuation tests ----

    #[test]
    fn extra_whitespace() {
        let s = sentence(
            vec![
                word("koira", 0),
                ws("  ", 5),
                word("kissa", 7),
            ],
            0,
        );
        let errs = gc_local_punctuation(&s);
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].error_code, GCERR_EXTRA_WHITESPACE);
        assert_eq!(errs[0].start_pos, 5);
        assert_eq!(errs[0].error_len, 2);
        assert_eq!(errs[0].suggestions, vec![" "]);
    }

    #[test]
    fn space_before_comma() {
        let s = sentence(
            vec![
                word("koira", 0),
                ws(" ", 5),
                punct(",", 6),
            ],
            0,
        );
        let errs = gc_local_punctuation(&s);
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].error_code, GCERR_SPACE_BEFORE_PUNCTUATION);
        assert_eq!(errs[0].start_pos, 5);
        assert_eq!(errs[0].error_len, 2);
    }

    #[test]
    fn extra_comma() {
        let s = sentence(
            vec![
                word("koira", 0),
                punct(",", 5),
                punct(",", 6),
            ],
            0,
        );
        let errs = gc_local_punctuation(&s);
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].error_code, GCERR_EXTRA_COMMA);
    }

    #[test]
    fn invalid_sentence_starter_comma() {
        let s = sentence(
            vec![
                punct(",", 5),
                ws(" ", 6),
                word("koira", 7),
            ],
            5,
        );
        let errs = gc_local_punctuation(&s);
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].error_code, GCERR_INVALID_SENTENCE_STARTER);
    }

    #[test]
    fn valid_sentence_starter_open_paren() {
        let s = sentence(
            vec![
                punct("(", 0),
                word("koira", 1),
                punct(")", 6),
            ],
            0,
        );
        let errs = gc_local_punctuation(&s);
        assert!(errs.is_empty());
    }

    #[test]
    fn no_punctuation_errors_normal_sentence() {
        let s = sentence(
            vec![
                word("Koira", 0),
                ws(" ", 5),
                word("juoksee", 6),
                punct(".", 13),
            ],
            0,
        );
        let errs = gc_local_punctuation(&s);
        assert!(errs.is_empty());
    }

    #[test]
    fn skip_footnote_bracket() {
        let s = sentence(
            vec![
                punct("[", 0),
                word("2", 1),
                punct("]", 2),
                ws(" ", 3),
                word("koira", 4),
            ],
            0,
        );
        let errs = gc_local_punctuation(&s);
        assert!(errs.is_empty());
    }

    // ---- gc_punctuation_of_quotations tests ----

    #[test]
    fn foreign_quotation_mark() {
        let s = sentence(
            vec![
                punct("\u{201C}", 0),
                word("koira", 1),
                punct("\u{201D}", 6),
            ],
            0,
        );
        let errs = gc_punctuation_of_quotations(&s);
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].error_code, GCERR_FOREIGN_QUOTATION_MARK);
    }

    #[test]
    fn invalid_punctuation_at_end_of_quotation_dot() {
        // ."\u{201D},  ->  should be  \u{201D},
        let s = sentence(
            vec![
                word("koira", 0),
                punct(".", 5),
                punct("\u{201D}", 6),
                punct(",", 7),
            ],
            0,
        );
        let errs = gc_punctuation_of_quotations(&s);
        assert_eq!(errs.len(), 1);
        assert_eq!(
            errs[0].error_code,
            GCERR_INVALID_PUNCTUATION_AT_END_OF_QUOTATION
        );
        assert_eq!(errs[0].suggestions[0], "\u{201D},");
    }

    #[test]
    fn invalid_punctuation_at_end_of_quotation_exclamation() {
        // !",  -> !"
        let s = sentence(
            vec![
                word("koira", 0),
                punct("!", 5),
                punct("\"", 6),
                punct(",", 7),
            ],
            0,
        );
        let errs = gc_punctuation_of_quotations(&s);
        assert_eq!(errs.len(), 1);
        assert_eq!(
            errs[0].error_code,
            GCERR_INVALID_PUNCTUATION_AT_END_OF_QUOTATION
        );
        assert_eq!(errs[0].suggestions[0], "!\"");
    }

    // ---- gc_repeating_words tests ----

    #[test]
    fn repeating_word_detected() {
        let s = sentence(
            vec![
                word("koira", 0),
                ws(" ", 5),
                word("koira", 6),
            ],
            0,
        );
        let errs = gc_repeating_words(&s);
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].error_code, GCERR_REPEATING_WORD);
        assert_eq!(errs[0].start_pos, 0);
        assert_eq!(errs[0].error_len, 11);
        assert_eq!(errs[0].suggestions, vec!["koira"]);
    }

    #[test]
    fn repeating_word_case_insensitive() {
        let s = sentence(
            vec![
                word("Koira", 0),
                ws(" ", 5),
                word("koira", 6),
            ],
            0,
        );
        let errs = gc_repeating_words(&s);
        assert_eq!(errs.len(), 1);
    }

    #[test]
    fn repeating_word_exception_ollut() {
        let s = sentence(
            vec![
                word("ollut", 0),
                ws(" ", 5),
                word("ollut", 6),
            ],
            0,
        );
        let errs = gc_repeating_words(&s);
        assert!(errs.is_empty());
    }

    #[test]
    fn repeating_word_exception_silla() {
        let s = sentence(
            vec![
                word("sill\u{00e4}", 0),
                ws(" ", 6),
                word("sill\u{00e4}", 7),
            ],
            0,
        );
        let errs = gc_repeating_words(&s);
        assert!(errs.is_empty());
    }

    #[test]
    fn repeating_digit_word_not_flagged() {
        let s = sentence(
            vec![word("123", 0), ws(" ", 3), word("123", 4)],
            0,
        );
        let errs = gc_repeating_words(&s);
        assert!(errs.is_empty());
    }

    #[test]
    fn different_words_not_flagged() {
        let s = sentence(
            vec![word("koira", 0), ws(" ", 5), word("kissa", 6)],
            0,
        );
        let errs = gc_repeating_words(&s);
        assert!(errs.is_empty());
    }

    // ---- gc_end_punctuation tests ----

    #[test]
    fn end_punctuation_missing() {
        let s = sentence(vec![word("koira", 0)], 0);
        let p = GrammarParagraph {
            sentences: vec![s.clone(), s],
        };
        let errs = gc_end_punctuation(&p, &default_opts());
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].error_code, GCERR_TERMINATING_PUNCTUATION_MISSING);
    }

    #[test]
    fn end_punctuation_present() {
        let s = sentence(
            vec![word("koira", 0), punct(".", 5)],
            0,
        );
        let p = GrammarParagraph {
            sentences: vec![s],
        };
        let errs = gc_end_punctuation(&p, &default_opts());
        assert!(errs.is_empty());
    }

    #[test]
    fn end_punctuation_accept_titles() {
        let s = sentence(vec![word("Otsikko", 0)], 0);
        let p = GrammarParagraph {
            sentences: vec![s],
        };
        let opts = GrammarOptions {
            accept_titles_in_gc: true,
            ..Default::default()
        };
        let errs = gc_end_punctuation(&p, &opts);
        assert!(errs.is_empty());
    }

    #[test]
    fn end_punctuation_accept_unfinished() {
        let s1 = sentence(vec![word("Koira", 0), punct(".", 5)], 0);
        let s2 = sentence(vec![word("Kissa", 7)], 7);
        let p = GrammarParagraph {
            sentences: vec![s1, s2],
        };
        let opts = GrammarOptions {
            accept_unfinished_paragraphs_in_gc: true,
            ..Default::default()
        };
        let errs = gc_end_punctuation(&p, &opts);
        assert!(errs.is_empty());
    }

    // ---- gc_missing_verb tests ----

    #[test]
    fn missing_verb_detected() {
        let mut w1 = word("Koira", 0);
        w1.is_valid_word = true;
        let mut w2 = word("suuri", 6);
        w2.is_valid_word = true;
        let s = sentence(
            vec![w1, ws(" ", 5), w2, punct(".", 11)],
            0,
        );
        let errs = gc_missing_verb(&s, &default_opts());
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].error_code, GCERR_MISSING_MAIN_VERB);
    }

    #[test]
    fn no_missing_verb_with_verb() {
        let mut w1 = word("Koira", 0);
        w1.is_valid_word = true;
        let mut w2 = word("juoksee", 6);
        w2.is_valid_word = true;
        w2.possible_main_verb = true;
        let s = sentence(
            vec![w1, ws(" ", 5), w2, punct(".", 13)],
            0,
        );
        let errs = gc_missing_verb(&s, &default_opts());
        assert!(errs.is_empty());
    }

    #[test]
    fn no_missing_verb_single_word() {
        let mut w1 = word("Koira", 0);
        w1.is_valid_word = true;
        let s = sentence(vec![w1, punct(".", 5)], 0);
        let errs = gc_missing_verb(&s, &default_opts());
        assert!(errs.is_empty());
    }

    #[test]
    fn no_missing_verb_unrecognized_word() {
        let w1 = word("Koira", 0); // is_valid_word = false
        let w2 = word("xyz", 6);
        let s = sentence(
            vec![w1, ws(" ", 5), w2, punct(".", 9)],
            0,
        );
        let errs = gc_missing_verb(&s, &default_opts());
        assert!(errs.is_empty());
    }

    #[test]
    fn extra_main_verb_detected() {
        let mut w1 = word("Koira", 0);
        w1.is_valid_word = true;
        w1.is_main_verb = true;
        let mut w2 = word("juoksee", 6);
        w2.is_valid_word = true;
        w2.is_main_verb = true;
        let s = sentence(
            vec![w1, ws(" ", 5), w2, punct(".", 13)],
            0,
        );
        let errs = gc_missing_verb(&s, &default_opts());
        assert!(errs.iter().any(|e| e.error_code == GCERR_EXTRA_MAIN_VERB));
    }

    // ---- gc_negative_verb_mismatch tests ----

    #[test]
    fn negative_verb_mismatch() {
        let mut w1 = word("en", 0);
        w1.is_verb_negative = true;
        let mut w2 = word("sy\u{00f6}n", 3);
        w2.is_positive_verb = true;
        let s = sentence(vec![w1, ws(" ", 2), w2], 0);
        let errs = gc_negative_verb_mismatch(&s);
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].error_code, GCERR_NEGATIVE_VERB_MISMATCH);
    }

    #[test]
    fn no_negative_verb_mismatch() {
        let mut w1 = word("en", 0);
        w1.is_verb_negative = true;
        let w2 = word("sy\u{00f6}", 3); // not marked as positive verb
        let s = sentence(vec![w1, ws(" ", 2), w2], 0);
        let errs = gc_negative_verb_mismatch(&s);
        assert!(errs.is_empty());
    }

    // ---- gc_compound_verb tests ----

    #[test]
    fn a_infinitive_required() {
        let mut w1 = word("haluan", 0);
        w1.require_following_verb = FollowingVerbType::AInfinitive;
        let mut w2 = word("sy\u{00f6}m\u{00e4}\u{00e4}n", 7);
        w2.verb_follower_type = FollowingVerbType::MaInfinitive;
        let s = sentence(vec![w1, ws(" ", 6), w2], 0);
        let errs = gc_compound_verb(&s);
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].error_code, GCERR_A_INFINITIVE_REQUIRED);
    }

    #[test]
    fn ma_infinitive_required() {
        let mut w1 = word("menen", 0);
        w1.require_following_verb = FollowingVerbType::MaInfinitive;
        let mut w2 = word("sy\u{00f6}d\u{00e4}", 6);
        w2.verb_follower_type = FollowingVerbType::AInfinitive;
        let s = sentence(vec![w1, ws(" ", 5), w2], 0);
        let errs = gc_compound_verb(&s);
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].error_code, GCERR_MA_INFINITIVE_REQUIRED);
    }

    #[test]
    fn no_compound_verb_error_matching() {
        let mut w1 = word("haluan", 0);
        w1.require_following_verb = FollowingVerbType::AInfinitive;
        let mut w2 = word("sy\u{00f6}d\u{00e4}", 7);
        w2.verb_follower_type = FollowingVerbType::AInfinitive;
        let s = sentence(vec![w1, ws(" ", 6), w2], 0);
        let errs = gc_compound_verb(&s);
        assert!(errs.is_empty());
    }

    // ---- gc_sidesana tests ----

    #[test]
    fn misplaced_conjunction() {
        let mut w1 = word("koira", 0);
        w1.is_valid_word = true;
        let mut conj = word("ja", 6);
        conj.is_conjunction = true;
        let s = sentence(
            vec![w1, ws(" ", 5), conj, punct(".", 8)],
            0,
        );
        let errs = gc_sidesana(&s);
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].error_code, GCERR_MISPLACED_SIDESANA);
    }

    #[test]
    fn vaan_not_misplaced() {
        let mut conj = word("vaan", 0);
        conj.is_conjunction = true;
        let s = sentence(vec![conj, punct(".", 4)], 0);
        let errs = gc_sidesana(&s);
        assert!(errs.is_empty());
    }

    #[test]
    fn conjunction_not_at_end_ok() {
        let mut conj = word("ja", 0);
        conj.is_conjunction = true;
        let s = sentence(
            vec![conj, ws(" ", 2), word("koira", 3), punct(".", 8)],
            0,
        );
        let errs = gc_sidesana(&s);
        assert!(errs.is_empty());
    }

    // ---- Utility function tests ----

    #[test]
    fn test_is_integer() {
        assert!(is_integer(&chars("123")));
        assert!(is_integer(&chars("0")));
        assert!(!is_integer(&chars("12a")));
        assert!(!is_integer(&chars("")));
    }

    #[test]
    fn test_is_chapter_number() {
        assert!(is_chapter_number(&chars("3")));
        assert!(is_chapter_number(&chars("3.4")));
        assert!(is_chapter_number(&chars("3.65.3")));
        assert!(!is_chapter_number(&chars("3.")));
        assert!(!is_chapter_number(&chars(".3")));
        assert!(!is_chapter_number(&chars("3..4")));
        assert!(!is_chapter_number(&chars("abc")));
    }

    #[test]
    fn test_is_roman_numeral() {
        assert!(is_roman_numeral(&chars("i")));
        assert!(is_roman_numeral(&chars("III")));
        assert!(is_roman_numeral(&chars("xVi")));
        assert!(!is_roman_numeral(&chars("abc")));
        assert!(!is_roman_numeral(&chars("")));
    }

    #[test]
    fn test_is_possible_list_item() {
        assert!(is_possible_list_item(&chars("a")));
        assert!(is_possible_list_item(&chars("1")));
        assert!(is_possible_list_item(&chars("3.4")));
        assert!(is_possible_list_item(&chars("III")));
        assert!(!is_possible_list_item(&chars("abc")));
    }

    // ---- Capitalization check tests ----

    #[test]
    fn capitalization_uppercase_required() {
        let mut w1 = word("koira", 0);
        w1.is_valid_word = true;
        w1.first_letter_lcase = true;
        let s = sentence(vec![w1, punct(".", 5)], 0);
        let p = GrammarParagraph {
            sentences: vec![s],
        };
        let errs = gc_capitalization(&p, &default_opts());
        assert!(errs.iter().any(|e| e.error_code == GCERR_WRITE_FIRST_UPPERCASE));
    }

    #[test]
    fn capitalization_no_error_uppercase() {
        let mut w1 = word("Koira", 0);
        w1.is_valid_word = true;
        w1.first_letter_lcase = true;
        let mut w2 = word("juoksee", 6);
        w2.is_valid_word = true;
        w2.first_letter_lcase = true;
        let s = sentence(
            vec![w1, ws(" ", 5), w2, punct(".", 13)],
            0,
        );
        let p = GrammarParagraph {
            sentences: vec![s],
        };
        let errs = gc_capitalization(&p, &default_opts());
        assert!(errs
            .iter()
            .all(|e| e.error_code != GCERR_WRITE_FIRST_UPPERCASE));
    }

    #[test]
    fn capitalization_lowercase_required() {
        // Second word in sentence starts with uppercase but should be lowercase
        let mut w1 = word("Koira", 0);
        w1.is_valid_word = true;
        w1.first_letter_lcase = true;
        let mut w2 = word("Juoksee", 6);
        w2.is_valid_word = true;
        w2.first_letter_lcase = true;
        // token_len() returns text.len(), which is already 7 for "Juoksee"
        let s = sentence(
            vec![w1, ws(" ", 5), w2, punct(".", 13)],
            0,
        );
        let p = GrammarParagraph {
            sentences: vec![s],
        };
        let errs = gc_capitalization(&p, &default_opts());
        assert!(errs
            .iter()
            .any(|e| e.error_code == GCERR_WRITE_FIRST_LOWERCASE));
    }

    #[test]
    fn capitalization_misplaced_closing_parenthesis() {
        let s = sentence(
            vec![
                word("Koira", 0),
                ws(" ", 5),
                punct(")", 6),
                ws(" ", 7),
                word("kissa", 8),
            ],
            0,
        );
        let p = GrammarParagraph {
            sentences: vec![s],
        };
        let errs = gc_capitalization(&p, &default_opts());
        assert!(errs
            .iter()
            .any(|e| e.error_code == GCERR_MISPLACED_CLOSING_PARENTHESIS));
    }
}
