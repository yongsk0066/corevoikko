// VfstAutocorrectCheck: autocorrect via FST transducer.
//
// For each word in a sentence, checks if the autocorrect transducer
// produces a correction. If it does, returns a GCERR_INVALID_SPELLING
// error with the correction as suggestion.
//
// Origin: grammar/FinnishRuleEngine/VfstAutocorrectCheck.cpp

use voikko_core::character::{is_upper, simple_lower, simple_upper};
use voikko_core::enums::TokenType;
use voikko_core::grammar_error::{GCERR_INVALID_SPELLING, GrammarError};
use voikko_fst::Transducer;
use voikko_fst::unweighted::UnweightedTransducer;

use super::paragraph::GrammarSentence;

/// Maximum buffer size for autocorrect transducer traversal.
///
/// Origin: VfstAutocorrectCheck.cpp:40
const BUFFER_SIZE: usize = 20000;

/// SOFT HYPHEN character, stripped from input before autocorrect lookup.
///
/// Origin: VfstAutocorrectCheck.cpp:103
const SOFT_HYPHEN: char = '\u{00AD}';

/// Run autocorrect check on a sentence using the given transducer.
///
/// First tries the sentence as-is. If the first word starts with uppercase
/// and no match is found at position 0, re-tries with the first letter
/// lowered. If the lowered version produces a match, the suggestion's first
/// letter is uppercased.
///
/// Origin: VfstAutocorrectCheck.cpp:59-63
pub(crate) fn gc_autocorrect(
    sentence: &GrammarSentence,
    transducer: &UnweightedTransducer,
) -> Vec<GrammarError> {
    let need_lowering = gc_autocorrect_inner(sentence, transducer, false);
    let mut errors = need_lowering.errors;
    if need_lowering.need_lowering {
        let lowered = gc_autocorrect_inner(sentence, transducer, true);
        errors.extend(lowered.errors);
    }
    errors
}

/// Result of a single autocorrect pass.
struct AutocorrectResult {
    errors: Vec<GrammarError>,
    need_lowering: bool,
}

/// Inner autocorrect check implementation.
///
/// Builds a flat input buffer from the sentence tokens (normalizing
/// whitespace to single space, stripping soft hyphens), records word
/// start positions, then runs the transducer's `next_prefix` at each
/// word start position.
///
/// Origin: VfstAutocorrectCheck.cpp:65-171
fn gc_autocorrect_inner(
    sentence: &GrammarSentence,
    transducer: &UnweightedTransducer,
    lower_first: bool,
) -> AutocorrectResult {
    let mut errors = Vec::new();
    let mut need_lowering = false;
    let tokens = &sentence.tokens;

    // Build the input buffer and tracking arrays.
    // lookup_positions_ucs: UCS (character) positions of word starts in the original token stream
    // lookup_positions_utf: positions of word starts in the normalized input buffer
    let mut input_buffer = Vec::with_capacity(BUFFER_SIZE);
    let mut lookup_positions_utf: Vec<usize> = Vec::new();
    let mut lookup_positions_ucs: Vec<usize> = Vec::new();
    let mut ucs_original_positions: Vec<usize> = Vec::new();
    let mut ucs_normalized_positions: Vec<usize> = Vec::new();

    ucs_original_positions.push(0);
    ucs_normalized_positions.push(0);

    let mut sentence_length_utf: usize = 0;
    let mut sentence_length_ucs: usize = 0;

    for (token_idx, token) in tokens.iter().enumerate() {
        if token.token_type == TokenType::Word {
            lookup_positions_utf.push(sentence_length_utf);
            lookup_positions_ucs.push(sentence_length_ucs);
        }

        if token.token_type == TokenType::Whitespace {
            if sentence_length_utf >= BUFFER_SIZE {
                return AutocorrectResult {
                    errors,
                    need_lowering: false,
                };
            }
            input_buffer.push(' ');
            sentence_length_utf += 1;
            ucs_normalized_positions.push(ucs_normalized_positions[token_idx] + 1);
        } else {
            let mut skipped_chars: usize = 0;

            // Optionally lowercase the first token's first character
            let use_lowered = lower_first && token_idx == 0;
            let lowered_first: Option<char> = if use_lowered {
                token.text.first().map(|&c| simple_lower(c))
            } else {
                None
            };

            if sentence_length_utf + token.token_len() >= BUFFER_SIZE {
                return AutocorrectResult {
                    errors,
                    need_lowering: false,
                };
            }

            for (char_idx, &ch) in token.text.iter().enumerate() {
                let actual_ch = if use_lowered && char_idx == 0 {
                    lowered_first.unwrap_or(ch)
                } else {
                    ch
                };

                if actual_ch == SOFT_HYPHEN {
                    skipped_chars += 1;
                } else {
                    input_buffer.push(actual_ch);
                }
            }

            let token_utf_len = token.token_len() - skipped_chars;
            sentence_length_utf += token_utf_len;
            ucs_normalized_positions
                .push(ucs_normalized_positions[token_idx] + token.token_len() - skipped_chars);
        }

        sentence_length_ucs += token.token_len();
        ucs_original_positions.push(sentence_length_ucs);
    }

    // Run the transducer at each word start position.
    let mut config = transducer.new_config(BUFFER_SIZE);

    for (&position, &ucs_position) in lookup_positions_utf.iter().zip(lookup_positions_ucs.iter()) {
        if lower_first && position > 0 {
            break;
        }

        let remaining_input = &input_buffer[position..];
        transducer.prepare(&mut config, remaining_input);

        let mut output = String::new();
        let mut prefix_length: usize = 0;

        if transducer.next_prefix(&mut config, &mut output, &mut prefix_length) && prefix_length > 0
        {
            // Check that the match ends at a word boundary
            let end_at_boundary = ucs_normalized_positions
                .iter()
                .any(|&p| ucs_position + prefix_length == p);

            if !end_at_boundary {
                continue;
            }

            let start_pos = sentence.pos + ucs_position;

            // Calculate length correction for soft hyphens etc.
            let mut length_correction: usize = 0;
            for n in 0..ucs_original_positions.len() {
                let o_pos = ucs_original_positions[n];
                let n_pos = ucs_normalized_positions[n];
                if o_pos <= start_pos {
                    length_correction = o_pos.saturating_sub(n_pos);
                }
                if n_pos > start_pos + prefix_length {
                    if n > 0 {
                        let prev_correction = ucs_original_positions[n - 1]
                            .saturating_sub(ucs_normalized_positions[n - 1]);
                        length_correction = prev_correction.saturating_sub(length_correction);
                    }
                    break;
                }
            }

            let error_len = prefix_length + length_correction;

            // If we lowered the first letter, uppercase the suggestion
            let mut suggestion = output;
            if lower_first {
                let mut chars: Vec<char> = suggestion.chars().collect();
                if !chars.is_empty() {
                    chars[0] = simple_upper(chars[0]);
                    suggestion = chars.into_iter().collect();
                }
            }

            errors.push(GrammarError::with_suggestions(
                GCERR_INVALID_SPELLING,
                start_pos,
                error_len,
                vec![suggestion],
            ));
        } else if !lower_first
            && position == 0
            && !tokens.is_empty()
            && tokens[0].text.first().copied().is_some_and(is_upper)
        {
            need_lowering = true;
        }
    }

    AutocorrectResult {
        errors,
        need_lowering,
    }
}

#[cfg(test)]
mod tests {
    use super::super::paragraph::{GrammarSentence, GrammarToken};
    use super::*;
    use voikko_core::enums::TokenType;

    fn word(text: &str, pos: usize) -> GrammarToken {
        GrammarToken::new(TokenType::Word, text.chars().collect(), pos)
    }

    fn ws(text: &str, pos: usize) -> GrammarToken {
        GrammarToken::new(TokenType::Whitespace, text.chars().collect(), pos)
    }

    fn sentence(tokens: Vec<GrammarToken>, pos: usize) -> GrammarSentence {
        let mut s = GrammarSentence::new(pos);
        s.tokens = tokens;
        s
    }

    #[test]
    fn empty_sentence_no_errors() {
        // We cannot easily create a transducer in tests without building
        // a proper VFST binary. This test verifies that the function
        // handles an empty sentence gracefully.
        // The actual integration test would need a real autocorr.vfst file.
        let s = sentence(vec![], 0);

        // Build a minimal transducer that accepts nothing.
        let data = build_minimal_vfst();
        let t = UnweightedTransducer::from_bytes(&data).unwrap();
        let errs = gc_autocorrect(&s, &t);
        assert!(errs.is_empty());
    }

    #[test]
    fn no_match_no_error() {
        let s = sentence(vec![word("koira", 0), ws(" ", 5), word("kissa", 6)], 0);
        let data = build_minimal_vfst();
        let t = UnweightedTransducer::from_bytes(&data).unwrap();
        let errs = gc_autocorrect(&s, &t);
        assert!(errs.is_empty());
    }

    // ====================================================================
    // VFST builder helpers
    // ====================================================================

    use voikko_fst::transition::Transition;

    fn build_header() -> Vec<u8> {
        let mut buf = vec![0u8; 16];
        buf[..4].copy_from_slice(&0x0001_3A6Eu32.to_le_bytes());
        buf[4..8].copy_from_slice(&0x0003_51FAu32.to_le_bytes());
        buf[8] = 0; // unweighted
        buf
    }

    fn build_symbol_table(symbols: &[&str]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&(symbols.len() as u16).to_le_bytes());
        for s in symbols {
            buf.extend_from_slice(s.as_bytes());
            buf.push(0);
        }
        buf
    }

    fn make_transition(sym_in: u16, sym_out: u16, target: u32, more: u8) -> Transition {
        Transition {
            sym_in,
            sym_out,
            trans_info: (target & 0x00FF_FFFF) | ((more as u32) << 24),
        }
    }

    fn align_to_8(data: &mut Vec<u8>) {
        let partial = data.len() % 8;
        if partial > 0 {
            data.extend(std::iter::repeat_n(0u8, 8 - partial));
        }
    }

    /// Build a minimal VFST that accepts nothing (no normal transitions).
    fn build_minimal_vfst() -> Vec<u8> {
        let symbols: &[&str] = &[""];
        let mut data = Vec::new();
        data.extend_from_slice(&build_header());
        data.extend_from_slice(&build_symbol_table(symbols));
        align_to_8(&mut data);

        // State 0: final transition only
        data.extend_from_slice(bytemuck::bytes_of(&make_transition(0xFFFF, 0, 0, 0)));
        data
    }

    /// Build a VFST that maps "ab" -> "xy".
    ///
    /// Symbol table: ["", "a", "b", "x", "y"]
    ///   index:        0    1    2    3    4
    ///
    /// States:
    ///   State 0 (idx 0): 'a'(1) -> State 1, output 'x'(3)
    ///   State 1 (idx 1): 'b'(2) -> State 2, output 'y'(4)
    ///   State 2 (idx 2): final (0xFFFF)
    fn build_ab_to_xy_vfst() -> Vec<u8> {
        let symbols: &[&str] = &["", "a", "b", "x", "y"];
        let mut data = Vec::new();
        data.extend_from_slice(&build_header());
        data.extend_from_slice(&build_symbol_table(symbols));
        align_to_8(&mut data);

        // State 0: 'a' -> state 1, output 'x'
        data.extend_from_slice(bytemuck::bytes_of(&make_transition(1, 3, 1, 0)));
        // State 1: 'b' -> state 2, output 'y'
        data.extend_from_slice(bytemuck::bytes_of(&make_transition(2, 4, 2, 0)));
        // State 2: final
        data.extend_from_slice(bytemuck::bytes_of(&make_transition(0xFFFF, 0, 0, 0)));

        data
    }

    /// Build a VFST that maps "abc" -> "xyz".
    ///
    /// Symbol table: ["", "a", "b", "c", "x", "y", "z"]
    ///   index:        0    1    2    3    4    5    6
    fn build_abc_to_xyz_vfst() -> Vec<u8> {
        let symbols: &[&str] = &["", "a", "b", "c", "x", "y", "z"];
        let mut data = Vec::new();
        data.extend_from_slice(&build_header());
        data.extend_from_slice(&build_symbol_table(symbols));
        align_to_8(&mut data);

        // State 0: 'a'(1) -> state 1, output 'x'(4)
        data.extend_from_slice(bytemuck::bytes_of(&make_transition(1, 4, 1, 0)));
        // State 1: 'b'(2) -> state 2, output 'y'(5)
        data.extend_from_slice(bytemuck::bytes_of(&make_transition(2, 5, 2, 0)));
        // State 2: 'c'(3) -> state 3, output 'z'(6)
        data.extend_from_slice(bytemuck::bytes_of(&make_transition(3, 6, 3, 0)));
        // State 3: final
        data.extend_from_slice(bytemuck::bytes_of(&make_transition(0xFFFF, 0, 0, 0)));

        data
    }

    /// Build a VFST that maps "ab cd" -> "ef gh" (multi-word prefix match).
    ///
    /// The transducer matches "ab cd" as a 5-character prefix (a, b, space, c, d).
    ///
    /// Symbol table: ["", "a", "b", " ", "c", "d", "e", "f", "g", "h"]
    ///   index:        0    1    2    3    4    5    6    7    8    9
    fn build_ab_cd_to_ef_gh_vfst() -> Vec<u8> {
        let symbols: &[&str] = &["", "a", "b", " ", "c", "d", "e", "f", "g", "h"];
        let mut data = Vec::new();
        data.extend_from_slice(&build_header());
        data.extend_from_slice(&build_symbol_table(symbols));
        align_to_8(&mut data);

        // State 0: 'a'(1) -> state 1, output 'e'(6)
        data.extend_from_slice(bytemuck::bytes_of(&make_transition(1, 6, 1, 0)));
        // State 1: 'b'(2) -> state 2, output 'f'(7)
        data.extend_from_slice(bytemuck::bytes_of(&make_transition(2, 7, 2, 0)));
        // State 2: ' '(3) -> state 3, output ' '(3)
        data.extend_from_slice(bytemuck::bytes_of(&make_transition(3, 3, 3, 0)));
        // State 3: 'c'(4) -> state 4, output 'g'(8)
        data.extend_from_slice(bytemuck::bytes_of(&make_transition(4, 8, 4, 0)));
        // State 4: 'd'(5) -> state 5, output 'h'(9)
        data.extend_from_slice(bytemuck::bytes_of(&make_transition(5, 9, 5, 0)));
        // State 5: final
        data.extend_from_slice(bytemuck::bytes_of(&make_transition(0xFFFF, 0, 0, 0)));

        data
    }

    // ====================================================================
    // Successful match tests
    // ====================================================================

    #[test]
    fn single_word_match_produces_correction() {
        // Sentence: "ab" (single word token).
        // Transducer maps "ab" -> "xy".
        // Expected: one GCERR_INVALID_SPELLING error with suggestion "xy".
        let s = sentence(vec![word("ab", 0)], 0);
        let data = build_ab_to_xy_vfst();
        let t = UnweightedTransducer::from_bytes(&data).unwrap();
        let errs = gc_autocorrect(&s, &t);

        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].error_code, GCERR_INVALID_SPELLING);
        assert_eq!(errs[0].start_pos, 0);
        assert_eq!(errs[0].error_len, 2);
        assert_eq!(errs[0].suggestions, vec!["xy"]);
    }

    #[test]
    fn multi_char_input_match() {
        // Sentence: "abc" (single word token).
        // Transducer maps "abc" -> "xyz".
        let s = sentence(vec![word("abc", 0)], 0);
        let data = build_abc_to_xyz_vfst();
        let t = UnweightedTransducer::from_bytes(&data).unwrap();
        let errs = gc_autocorrect(&s, &t);

        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].error_code, GCERR_INVALID_SPELLING);
        assert_eq!(errs[0].start_pos, 0);
        assert_eq!(errs[0].error_len, 3);
        assert_eq!(errs[0].suggestions, vec!["xyz"]);
    }

    #[test]
    fn match_at_second_word() {
        // Sentence: "zz ab" — first word "zz" has no match, second word "ab" matches.
        // Transducer maps "ab" -> "xy".
        // Expected: one error at position 3 (after "zz ").
        let s = sentence(vec![word("zz", 0), ws(" ", 2), word("ab", 3)], 0);
        let data = build_ab_to_xy_vfst();
        let t = UnweightedTransducer::from_bytes(&data).unwrap();
        let errs = gc_autocorrect(&s, &t);

        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].start_pos, 3);
        assert_eq!(errs[0].error_len, 2);
        assert_eq!(errs[0].suggestions, vec!["xy"]);
    }

    #[test]
    fn prefix_must_end_at_word_boundary() {
        // Sentence: "abc" but transducer matches "ab" (prefix).
        // The prefix "ab" ends at position 2, but the word "abc" ends at 3.
        // Since the prefix does NOT end at a word boundary (no token boundary
        // at position 2), no error should be produced.
        let s = sentence(vec![word("abc", 0)], 0);
        let data = build_ab_to_xy_vfst();
        let t = UnweightedTransducer::from_bytes(&data).unwrap();
        let errs = gc_autocorrect(&s, &t);

        assert!(errs.is_empty());
    }

    #[test]
    fn multi_word_prefix_match() {
        // Sentence: "ab cd" — two word tokens.
        // Transducer maps "ab cd" -> "ef gh" (spans word boundary including space).
        // The prefix "ab cd" is 5 chars and ends at the boundary after "cd".
        let s = sentence(vec![word("ab", 0), ws(" ", 2), word("cd", 3)], 0);
        let data = build_ab_cd_to_ef_gh_vfst();
        let t = UnweightedTransducer::from_bytes(&data).unwrap();
        let errs = gc_autocorrect(&s, &t);

        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].error_code, GCERR_INVALID_SPELLING);
        assert_eq!(errs[0].start_pos, 0);
        assert_eq!(errs[0].error_len, 5);
        assert_eq!(errs[0].suggestions, vec!["ef gh"]);
    }

    // ====================================================================
    // Uppercase lowering / re-uppercasing tests
    // ====================================================================

    #[test]
    fn uppercase_first_letter_retried_with_lowercase() {
        // Sentence: "Ab" — first letter uppercase.
        // Transducer maps "ab" -> "xy" (lowercase only).
        // gc_autocorrect first tries "Ab" which doesn't match, then sets
        // need_lowering=true, and retries with "ab" which matches.
        // The suggestion should be uppercased: "Xy".
        let s = sentence(vec![word("Ab", 0)], 0);
        let data = build_ab_to_xy_vfst();
        let t = UnweightedTransducer::from_bytes(&data).unwrap();
        let errs = gc_autocorrect(&s, &t);

        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].error_code, GCERR_INVALID_SPELLING);
        assert_eq!(errs[0].start_pos, 0);
        assert_eq!(errs[0].error_len, 2);
        assert_eq!(errs[0].suggestions, vec!["Xy"]);
    }

    #[test]
    fn lowercase_first_no_retry() {
        // Sentence: "ab" — already lowercase.
        // Transducer maps "ab" -> "xy".
        // No need_lowering since first letter is not uppercase.
        // Suggestion stays "xy".
        let s = sentence(vec![word("ab", 0)], 0);
        let data = build_ab_to_xy_vfst();
        let t = UnweightedTransducer::from_bytes(&data).unwrap();
        let errs = gc_autocorrect(&s, &t);

        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].suggestions, vec!["xy"]);
    }

    // ====================================================================
    // Sentence position offset tests
    // ====================================================================

    #[test]
    fn sentence_pos_offset_applied() {
        // Sentence at paragraph offset 10.
        // Word "ab" at position 10 within the paragraph.
        let s = sentence(vec![word("ab", 10)], 10);
        let data = build_ab_to_xy_vfst();
        let t = UnweightedTransducer::from_bytes(&data).unwrap();
        let errs = gc_autocorrect(&s, &t);

        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].start_pos, 10);
        assert_eq!(errs[0].error_len, 2);
    }

    // ====================================================================
    // No-match scenarios
    // ====================================================================

    #[test]
    fn partial_prefix_no_word_boundary_no_error() {
        // Sentence: "abcd" — single word, 4 characters.
        // Transducer matches prefix "abc" (3 chars) but no word boundary at pos 3.
        let s = sentence(vec![word("abcd", 0)], 0);
        let data = build_abc_to_xyz_vfst();
        let t = UnweightedTransducer::from_bytes(&data).unwrap();
        let errs = gc_autocorrect(&s, &t);

        assert!(errs.is_empty());
    }

    #[test]
    fn unknown_characters_no_match() {
        // Sentence has characters not in the transducer's symbol table.
        let s = sentence(vec![word("zzz", 0)], 0);
        let data = build_ab_to_xy_vfst();
        let t = UnweightedTransducer::from_bytes(&data).unwrap();
        let errs = gc_autocorrect(&s, &t);

        assert!(errs.is_empty());
    }
}
