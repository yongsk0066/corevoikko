// Case type detection and conversion
// Origin: utils/utils.hpp:45-59, utils/utils.cpp:38-92

use crate::character::{is_lower, is_upper, simple_lower, simple_upper};

/// Classification of character casing within a word.
/// Origin: utils/utils.hpp:45
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CaseType {
    /// No letters found in the word (only digits, punctuation, etc.).
    NoLetters,
    /// All letters are lowercase: "koira".
    AllLower,
    /// First letter is uppercase, rest are lowercase: "Koira".
    FirstUpper,
    /// Mixed case that does not fit other patterns: "koIra".
    Complex,
    /// All letters are uppercase: "KOIRA".
    AllUpper,
}

/// Detect the case pattern of a character slice.
///
/// Scans the characters and returns the appropriate `CaseType`.
/// Non-letter characters (digits, punctuation) are ignored when
/// determining the case pattern.
///
/// Origin: utils/utils.cpp:38-67
pub fn detect_case(word: &[char]) -> CaseType {
    if word.is_empty() {
        return CaseType::NoLetters;
    }

    let mut first_uc = false;
    let mut rest_lc = true;
    let mut all_uc = true;
    let mut no_letters = true;

    if is_upper(word[0]) {
        first_uc = true;
        no_letters = false;
    }
    if is_lower(word[0]) {
        all_uc = false;
        no_letters = false;
    }

    for &c in &word[1..] {
        if is_upper(c) {
            no_letters = false;
            rest_lc = false;
        }
        if is_lower(c) {
            all_uc = false;
            no_letters = false;
        }
    }

    if no_letters {
        return CaseType::NoLetters;
    }
    if all_uc {
        return CaseType::AllUpper;
    }
    if !rest_lc {
        return CaseType::Complex;
    }
    if first_uc {
        CaseType::FirstUpper
    } else {
        CaseType::AllLower
    }
}

/// Apply a case transformation to a mutable character slice.
///
/// - `NoLetters` / `Complex` -- no change (the C++ code also does nothing).
/// - `AllLower` -- every letter is lowercased.
/// - `AllUpper` -- every letter is uppercased.
/// - `FirstUpper` -- first character is uppercased, rest are lowercased.
///
/// Origin: utils/utils.cpp:69-92
pub fn set_case(word: &mut [char], case_type: CaseType) {
    if word.is_empty() {
        return;
    }
    match case_type {
        CaseType::NoLetters | CaseType::Complex => {
            // Do nothing, matching C++ behavior
        }
        CaseType::AllLower => {
            for c in word.iter_mut() {
                *c = simple_lower(*c);
            }
        }
        CaseType::AllUpper => {
            for c in word.iter_mut() {
                *c = simple_upper(*c);
            }
        }
        CaseType::FirstUpper => {
            word[0] = simple_upper(word[0]);
            for c in word[1..].iter_mut() {
                *c = simple_lower(*c);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    fn to_string(cs: &[char]) -> String {
        cs.iter().collect()
    }

    // -- detect_case tests --

    #[test]
    fn detect_empty() {
        assert_eq!(detect_case(&[]), CaseType::NoLetters);
    }

    #[test]
    fn detect_no_letters() {
        assert_eq!(detect_case(&chars("123")), CaseType::NoLetters);
        assert_eq!(detect_case(&chars("...")), CaseType::NoLetters);
    }

    #[test]
    fn detect_all_lower() {
        assert_eq!(detect_case(&chars("koira")), CaseType::AllLower);
        assert_eq!(detect_case(&chars("a")), CaseType::AllLower);
    }

    #[test]
    fn detect_first_upper() {
        assert_eq!(detect_case(&chars("Koira")), CaseType::FirstUpper);
        assert_eq!(detect_case(&chars("Helsinki")), CaseType::FirstUpper);
    }

    #[test]
    fn detect_all_upper() {
        assert_eq!(detect_case(&chars("KOIRA")), CaseType::AllUpper);
        assert_eq!(detect_case(&chars("A")), CaseType::AllUpper);
    }

    #[test]
    fn detect_complex() {
        assert_eq!(detect_case(&chars("koIra")), CaseType::Complex);
        assert_eq!(detect_case(&chars("McDonalds")), CaseType::Complex);
    }

    #[test]
    fn detect_with_digits() {
        // Digits are not letters, so they don't affect the case pattern
        assert_eq!(detect_case(&chars("abc123")), CaseType::AllLower);
        assert_eq!(detect_case(&chars("ABC123")), CaseType::AllUpper);
        assert_eq!(detect_case(&chars("Abc123")), CaseType::FirstUpper);
    }

    #[test]
    fn detect_finnish_chars() {
        assert_eq!(detect_case(&chars("k\u{00E4}vel\u{00F6}")), CaseType::AllLower); // kävelö
        assert_eq!(detect_case(&chars("\u{00C4}iti")), CaseType::FirstUpper); // Äiti
        assert_eq!(detect_case(&chars("\u{00C4}\u{00D6}")), CaseType::AllUpper); // ÄÖ
    }

    // -- set_case tests --

    #[test]
    fn set_case_all_lower() {
        let mut w = chars("KOIRA");
        set_case(&mut w, CaseType::AllLower);
        assert_eq!(to_string(&w), "koira");
    }

    #[test]
    fn set_case_all_upper() {
        let mut w = chars("koira");
        set_case(&mut w, CaseType::AllUpper);
        assert_eq!(to_string(&w), "KOIRA");
    }

    #[test]
    fn set_case_first_upper() {
        let mut w = chars("koira");
        set_case(&mut w, CaseType::FirstUpper);
        assert_eq!(to_string(&w), "Koira");
    }

    #[test]
    fn set_case_first_upper_from_all_upper() {
        let mut w = chars("KOIRA");
        set_case(&mut w, CaseType::FirstUpper);
        assert_eq!(to_string(&w), "Koira");
    }

    #[test]
    fn set_case_no_letters_noop() {
        let mut w = chars("123");
        set_case(&mut w, CaseType::NoLetters);
        assert_eq!(to_string(&w), "123");
    }

    #[test]
    fn set_case_complex_noop() {
        let mut w = chars("McDonalds");
        let original = to_string(&w);
        set_case(&mut w, CaseType::Complex);
        assert_eq!(to_string(&w), original);
    }

    #[test]
    fn set_case_empty() {
        let mut w: Vec<char> = vec![];
        set_case(&mut w, CaseType::AllUpper); // should not panic
        assert!(w.is_empty());
    }

    #[test]
    fn set_case_finnish_chars() {
        let mut w = chars("\u{00E4}iti"); // äiti
        set_case(&mut w, CaseType::AllUpper);
        assert_eq!(to_string(&w), "\u{00C4}ITI"); // ÄITI
    }

    #[test]
    fn roundtrip_detect_and_set() {
        let original = chars("Helsinki");
        let case = detect_case(&original);
        assert_eq!(case, CaseType::FirstUpper);

        let mut lowered = original.clone();
        set_case(&mut lowered, CaseType::AllLower);
        assert_eq!(to_string(&lowered), "helsinki");

        set_case(&mut lowered, case);
        assert_eq!(to_string(&lowered), "Helsinki");
    }
}
