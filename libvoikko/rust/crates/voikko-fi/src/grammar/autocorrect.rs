#![allow(dead_code)]
// VfstAutocorrectCheck: autocorrect via FST transducer.
//
// For each word in a sentence, checks if the autocorrect transducer
// produces a correction. If it does, returns a GCERR_INVALID_SPELLING
// error with the correction as suggestion.
//
// Origin: grammar/FinnishRuleEngine/VfstAutocorrectCheck.cpp

use voikko_core::character::{is_upper, simple_lower, simple_upper};
use voikko_core::enums::TokenType;
use voikko_core::grammar_error::{GrammarError, GCERR_INVALID_SPELLING};
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
            ucs_normalized_positions
                .push(ucs_normalized_positions[token_idx] + 1);
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
    let mut ucs_iter = lookup_positions_ucs.iter();

    for &position in &lookup_positions_utf {
        let ucs_position = *ucs_iter.next().unwrap();

        if lower_first && position > 0 {
            break;
        }

        let remaining_input = &input_buffer[position..];
        transducer.prepare(&mut config, remaining_input);

        let mut output = String::new();
        let mut prefix_length: usize = 0;

        if transducer.next_prefix(&mut config, &mut output, &mut prefix_length)
            && prefix_length > 0
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
    use super::*;
    use super::super::paragraph::{GrammarSentence, GrammarToken};
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
        let s = sentence(
            vec![word("koira", 0), ws(" ", 5), word("kissa", 6)],
            0,
        );
        let data = build_minimal_vfst();
        let t = UnweightedTransducer::from_bytes(&data).unwrap();
        let errs = gc_autocorrect(&s, &t);
        assert!(errs.is_empty());
    }

    // Helper: build a minimal VFST that accepts nothing (no normal transitions).
    fn build_minimal_vfst() -> Vec<u8> {
        use voikko_fst::transition::Transition;

        // Symbols: [epsilon]
        let symbols: &[&str] = &[""];
        let mut data = Vec::new();

        // Header (16 bytes)
        let mut header = vec![0u8; 16];
        header[..4].copy_from_slice(&0x0001_3A6Eu32.to_le_bytes());
        header[4..8].copy_from_slice(&0x0003_51FAu32.to_le_bytes());
        header[8] = 0; // unweighted
        data.extend_from_slice(&header);

        // Symbol table
        data.extend_from_slice(&(symbols.len() as u16).to_le_bytes());
        for s in symbols {
            data.extend_from_slice(s.as_bytes());
            data.push(0);
        }

        // Align to 8 bytes
        let partial = data.len() % 8;
        if partial > 0 {
            data.extend(std::iter::repeat_n(0u8, 8 - partial));
        }

        // State 0: final transition only
        let t = Transition {
            sym_in: 0xFFFF,
            sym_out: 0,
            trans_info: 0,
        };
        data.extend_from_slice(bytemuck::bytes_of(&t));

        data
    }
}
