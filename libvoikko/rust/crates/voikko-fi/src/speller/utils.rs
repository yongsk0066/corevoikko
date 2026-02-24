// STRUCTURE attribute matching utilities
// Origin: spellchecker/SpellUtils.cpp

use voikko_core::character::{is_lower, is_upper};
use voikko_core::enums::SpellResult;

/// Match a word's actual character cases against the expected STRUCTURE pattern.
///
/// The STRUCTURE attribute is a per-character encoding of expected case:
/// - `=` compound boundary (skipped)
/// - `p` lowercase letter required
/// - `q` lowercase letter required (abbreviation context)
/// - `i` uppercase letter required
/// - `j` uppercase letter required (abbreviation context)
/// - `-` literal hyphen
/// - `:` literal colon
///
/// Returns:
/// - `SpellResult::Ok` if all letters match the expected case.
/// - `SpellResult::CapitalizeFirst` if only the first letter needs to be uppercased.
/// - `SpellResult::CapitalizationError` if a non-first letter has wrong case.
///
/// Origin: SpellUtils.cpp:36-76
pub fn match_word_and_analysis(word: &[char], structure: &str) -> SpellResult {
    let mut result = SpellResult::Ok;
    let structure_chars: Vec<char> = structure.chars().collect();
    let mut j = 0;

    for (i, &ch) in word.iter().enumerate() {
        // Skip compound boundary markers
        while j < structure_chars.len() && structure_chars[j] == '=' {
            j += 1;
        }
        if j >= structure_chars.len() {
            break;
        }

        // Classify the actual character's case
        // 'i' = uppercase letter, 'p' = lowercase letter, 'v' = punctuation/other
        let captype = if is_upper(ch) {
            'i'
        } else if is_lower(ch) {
            'p'
        } else {
            'v'
        };

        // Lowercase letter where uppercase is expected
        if captype == 'p' && (structure_chars[j] == 'i' || structure_chars[j] == 'j') {
            if i == 0 {
                result = SpellResult::CapitalizeFirst;
            } else {
                result = SpellResult::CapitalizationError;
            }
        }

        // Uppercase letter where lowercase is expected
        if captype == 'i' && (structure_chars[j] == 'p' || structure_chars[j] == 'q') {
            result = SpellResult::CapitalizationError;
        }

        if result == SpellResult::CapitalizationError {
            break;
        }

        j += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_lowercase_matches_lowercase_structure() {
        let word: Vec<char> = "koira".chars().collect();
        assert_eq!(match_word_and_analysis(&word, "=ppppp"), SpellResult::Ok);
    }

    #[test]
    fn first_upper_matches_uppercase_first_structure() {
        let word: Vec<char> = "Helsinki".chars().collect();
        assert_eq!(match_word_and_analysis(&word, "=ippppppp"), SpellResult::Ok);
    }

    #[test]
    fn lowercase_first_when_uppercase_expected_returns_cap_first() {
        let word: Vec<char> = "helsinki".chars().collect();
        assert_eq!(
            match_word_and_analysis(&word, "=ippppppp"),
            SpellResult::CapitalizeFirst
        );
    }

    #[test]
    fn uppercase_where_lowercase_expected_returns_cap_error() {
        let word: Vec<char> = "koIra".chars().collect();
        assert_eq!(
            match_word_and_analysis(&word, "=ppppp"),
            SpellResult::CapitalizationError
        );
    }

    #[test]
    fn compound_boundary_markers_are_skipped() {
        // STRUCTURE "=ppp=ppppp" for a compound word like "koiratalo"
        let word: Vec<char> = "koiratalo".chars().collect();
        assert_eq!(
            match_word_and_analysis(&word, "=ppp=pppppp"),
            SpellResult::Ok
        );
    }

    #[test]
    fn abbreviation_context_q_accepts_lowercase() {
        let word: Vec<char> = "abc".chars().collect();
        assert_eq!(match_word_and_analysis(&word, "=qqq"), SpellResult::Ok);
    }

    #[test]
    fn abbreviation_context_j_accepts_uppercase() {
        let word: Vec<char> = "ABC".chars().collect();
        assert_eq!(match_word_and_analysis(&word, "=jjj"), SpellResult::Ok);
    }

    #[test]
    fn uppercase_where_q_expected_returns_cap_error() {
        let word: Vec<char> = "ABC".chars().collect();
        assert_eq!(
            match_word_and_analysis(&word, "=qqq"),
            SpellResult::CapitalizationError
        );
    }

    #[test]
    fn punctuation_in_word_does_not_affect_result() {
        // Digit/punctuation has captype 'v', which doesn't match any error condition
        let word: Vec<char> = "a1b".chars().collect();
        assert_eq!(match_word_and_analysis(&word, "=pip"), SpellResult::Ok);
    }

    #[test]
    fn empty_word_is_ok() {
        assert_eq!(match_word_and_analysis(&[], "=ppp"), SpellResult::Ok);
    }

    #[test]
    fn empty_structure_is_ok() {
        let word: Vec<char> = "abc".chars().collect();
        assert_eq!(match_word_and_analysis(&word, ""), SpellResult::Ok);
    }

    #[test]
    fn multiple_compound_boundaries() {
        // "syyssilta" with structure "=ppp=pppppp" (compound at position 3)
        let word: Vec<char> = "syyssilta".chars().collect();
        assert_eq!(
            match_word_and_analysis(&word, "=ppp=pppppp"),
            SpellResult::Ok
        );
    }

    #[test]
    fn second_letter_uppercase_where_lowercase_expected() {
        let word: Vec<char> = "kOira".chars().collect();
        assert_eq!(
            match_word_and_analysis(&word, "=ppppp"),
            SpellResult::CapitalizationError
        );
    }

    #[test]
    fn first_letter_lowercase_with_uppercase_j_expected() {
        let word: Vec<char> = "abc".chars().collect();
        assert_eq!(
            match_word_and_analysis(&word, "=jpp"),
            SpellResult::CapitalizeFirst
        );
    }

    #[test]
    fn finnish_chars_case_detection() {
        // "Aaiti" with first upper expected
        let word: Vec<char> = "\u{00C4}iti".chars().collect(); // Aiti
        assert_eq!(match_word_and_analysis(&word, "=ippp"), SpellResult::Ok);
    }
}
