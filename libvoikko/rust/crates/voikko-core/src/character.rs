// Character classification and Unicode utilities
// Origin: character/SimpleChar.hpp, SimpleChar.cpp, charset.hpp, charset.cpp

// ---------------------------------------------------------------------------
// Finnish phonological constants
// Origin: utils/utils.hpp:40-41 (VOIKKO_CONSONANTS, VOIKKO_VOWELS)
// ---------------------------------------------------------------------------

/// Finnish vowels (lowercase): a e i o u y ä ö
const FINNISH_VOWELS: &[char] = &['a', 'e', 'i', 'o', 'u', 'y', '\u{00E4}', '\u{00F6}'];

/// Finnish consonants (lowercase): b c d f g h j k l m n p q r s t v w x z š ž
const FINNISH_CONSONANTS: &[char] = &[
    'b', 'c', 'd', 'f', 'g', 'h', 'j', 'k', 'l', 'm', 'n', 'p', 'q', 'r', 's', 't', 'v', 'w', 'x',
    'z', '\u{0161}', '\u{017E}',
];

// ---------------------------------------------------------------------------
// Character type classification
// Origin: charset.hpp:36, charset.cpp:42-74
// ---------------------------------------------------------------------------

/// Character type classification.
/// Origin: charset.hpp:36
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CharType {
    Unknown,
    Letter,
    Digit,
    Whitespace,
    Punctuation,
}

/// Returns the character type for a given character.
///
/// This classifies characters into letters, digits, whitespace, punctuation,
/// or unknown following the same logic as the C++ `get_char_type`.
///
/// Origin: charset.cpp:42-74
pub fn get_char_type(c: char) -> CharType {
    let cp = c as u32;
    if (0x41..=0x5A).contains(&cp)           // A-Z
        || (0x61..=0x7A).contains(&cp)       // a-z
        || (0xC1..=0xD6).contains(&cp)       // À-Ö (note: starts at C1, not C0)
        || (0xD8..=0xF6).contains(&cp)       // Ø-ö
        || (0x00F8..=0x02AF).contains(&cp)   // ø-ɏ
        || (0x0400..=0x0481).contains(&cp)   // Cyrillic Ѐ-ҁ
        || (0x048A..=0x0527).contains(&cp)   // Cyrillic extended Ҋ-ԧ
        || (0x1400..=0x15C3).contains(&cp)   // Canadian syllabics ᐀-ᗃ
        || (0xFB00..=0xFB04).contains(&cp)
    // Alphabetic presentation forms
    {
        return CharType::Letter;
    }
    if is_whitespace(c) {
        return CharType::Whitespace;
    }
    if is_punctuation_char(c) {
        return CharType::Punctuation;
    }
    if is_finnish_quotation_mark(c) {
        return CharType::Punctuation;
    }
    if c.is_ascii_digit() {
        return CharType::Digit;
    }
    CharType::Unknown
}

/// Check whether a character is a punctuation character recognized by Voikko.
/// Origin: charset.cpp:57-66
fn is_punctuation_char(c: char) -> bool {
    matches!(
        c,
        '.' | ','
            | ';'
            | '-'
            | '!'
            | '?'
            | ':'
            | '\''
            | '('
            | ')'
            | '['
            | ']'
            | '{'
            | '}'
            | '/'
            | '&'
            | '\u{00AD}' // SOFT HYPHEN
            | '\u{2019}' // RIGHT SINGLE QUOTATION MARK
            | '\u{2010}' // HYPHEN
            | '\u{2011}' // NON-BREAKING HYPHEN
            | '\u{2013}' // EN DASH
            | '\u{2014}' // EM DASH
            | '\u{201C}' // LEFT DOUBLE QUOTATION MARK
            | '\u{2026}' // HORIZONTAL ELLIPSIS
    )
}

// ---------------------------------------------------------------------------
// Finnish quotation marks
// Origin: charset.cpp:76-80
// ---------------------------------------------------------------------------

/// Check whether a character is a Finnish quotation mark.
///
/// Finnish uses `"`, `»`, and U+201D (right double quotation mark).
///
/// Origin: charset.cpp:76-80
pub fn is_finnish_quotation_mark(c: char) -> bool {
    matches!(
        c,
        '"' | '\u{00BB}' // » RIGHT-POINTING DOUBLE ANGLE QUOTATION MARK
            | '\u{201D}' // RIGHT DOUBLE QUOTATION MARK
    )
}

// ---------------------------------------------------------------------------
// Finnish phonological classification
// Origin: utils/utils.hpp:40-41
// ---------------------------------------------------------------------------

/// Check whether a character is a Finnish vowel (case-insensitive).
/// Finnish vowels: a, e, i, o, u, y, ä, ö.
///
/// Origin: utils/utils.hpp:41 (VOIKKO_VOWELS)
pub fn is_vowel(c: char) -> bool {
    let lower = simple_lower(c);
    FINNISH_VOWELS.contains(&lower)
}

/// Check whether a character is a Finnish consonant (case-insensitive).
/// Finnish consonants: b, c, d, f, g, h, j, k, l, m, n, p, q, r, s, t, v, w, x, z, š, ž.
///
/// Origin: utils/utils.hpp:40 (VOIKKO_CONSONANTS)
pub fn is_consonant(c: char) -> bool {
    let lower = simple_lower(c);
    FINNISH_CONSONANTS.contains(&lower)
}

// ---------------------------------------------------------------------------
// Simple case conversion
// Origin: SimpleChar.cpp:36-160
//
// The C++ code manually maps Unicode ranges because it targets limited
// wchar_t environments. In Rust we delegate to the standard library which
// handles full Unicode case mapping. The standard library's to_lowercase /
// to_uppercase produce iterators because some characters map to multiple
// characters, but for the "simple" one-to-one mapping we only take the
// first character (matching the C++ behavior).
// ---------------------------------------------------------------------------

/// Convert a character to its simple lowercase equivalent.
///
/// Uses Rust's built-in Unicode case mapping. For characters with
/// multi-character lowercase expansions, returns only the first character
/// (matching the C++ `SimpleChar::lower` behavior of one-to-one mapping).
///
/// Origin: SimpleChar.cpp:36-97
pub fn simple_lower(c: char) -> char {
    let mut iter = c.to_lowercase();
    iter.next().unwrap_or(c)
}

/// Convert a character to its simple uppercase equivalent.
///
/// Uses Rust's built-in Unicode case mapping. For characters with
/// multi-character uppercase expansions, returns only the first character.
///
/// Origin: SimpleChar.cpp:99-159
pub fn simple_upper(c: char) -> char {
    let mut iter = c.to_uppercase();
    iter.next().unwrap_or(c)
}

/// Check whether a character is an uppercase letter.
///
/// Origin: SimpleChar.cpp:162-165
pub fn is_upper(c: char) -> bool {
    c != simple_lower(c) || c == '\u{018F}' // LATIN CAPITAL LETTER SCHWA
}

/// Check whether a character is a lowercase letter.
///
/// Origin: SimpleChar.cpp:167-169
pub fn is_lower(c: char) -> bool {
    c != simple_upper(c)
}

/// Check whether a character is a whitespace character (matching C++ behavior).
///
/// This recognizes the same set of whitespace characters as the C++ `SimpleChar::isWhitespace`.
///
/// Origin: SimpleChar.cpp:175-188
pub fn is_whitespace(c: char) -> bool {
    let cp = c as u32;
    (0x09..=0x0D).contains(&cp)
        || cp == 0x20
        || cp == 0x85
        || cp == 0xA0
        || cp == 0x1680
        || cp == 0x180E
        || (0x2000..=0x200A).contains(&cp)
        || cp == 0x2028
        || cp == 0x2029
        || cp == 0x202F
        || cp == 0x205F
        || cp == 0x3000
}

/// Compare two character slices for equality, ignoring character case.
///
/// Origin: SimpleChar.cpp:190-200
pub fn equals_ignore_case(a: &[char], b: &[char]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .all(|(&ca, &cb)| simple_lower(ca) == simple_lower(cb))
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- CharType tests --

    #[test]
    fn char_type_letters() {
        assert_eq!(get_char_type('A'), CharType::Letter);
        assert_eq!(get_char_type('z'), CharType::Letter);
        assert_eq!(get_char_type('\u{00C4}'), CharType::Letter); // Ä
        assert_eq!(get_char_type('\u{00F6}'), CharType::Letter); // ö
    }

    #[test]
    fn char_type_c0_is_not_letter() {
        // U+00C0 (À) is NOT classified as a letter in the C++ code (range starts at 0xC1)
        assert_ne!(get_char_type('\u{00C0}'), CharType::Letter);
    }

    #[test]
    fn char_type_digits() {
        assert_eq!(get_char_type('0'), CharType::Digit);
        assert_eq!(get_char_type('9'), CharType::Digit);
    }

    #[test]
    fn char_type_whitespace() {
        assert_eq!(get_char_type(' '), CharType::Whitespace);
        assert_eq!(get_char_type('\t'), CharType::Whitespace);
        assert_eq!(get_char_type('\n'), CharType::Whitespace);
    }

    #[test]
    fn char_type_punctuation() {
        assert_eq!(get_char_type('.'), CharType::Punctuation);
        assert_eq!(get_char_type(','), CharType::Punctuation);
        assert_eq!(get_char_type('!'), CharType::Punctuation);
        assert_eq!(get_char_type('\u{2013}'), CharType::Punctuation); // EN DASH
    }

    #[test]
    fn char_type_finnish_quotation() {
        assert_eq!(get_char_type('"'), CharType::Punctuation);
        assert_eq!(get_char_type('\u{00BB}'), CharType::Punctuation); // »
        assert_eq!(get_char_type('\u{201D}'), CharType::Punctuation); // "
    }

    #[test]
    fn char_type_unknown() {
        assert_eq!(get_char_type('@'), CharType::Unknown);
        assert_eq!(get_char_type('#'), CharType::Unknown);
    }

    // -- Finnish quotation marks --

    #[test]
    fn finnish_quotation_marks() {
        assert!(is_finnish_quotation_mark('"'));
        assert!(is_finnish_quotation_mark('\u{00BB}')); // »
        assert!(is_finnish_quotation_mark('\u{201D}')); // "
        assert!(!is_finnish_quotation_mark('\''));
        assert!(!is_finnish_quotation_mark('\u{201C}')); // " left double
    }

    // -- Vowel / Consonant tests --

    #[test]
    fn finnish_vowels() {
        assert!(is_vowel('a'));
        assert!(is_vowel('A'));
        assert!(is_vowel('e'));
        assert!(is_vowel('\u{00E4}')); // ä
        assert!(is_vowel('\u{00C4}')); // Ä
        assert!(is_vowel('\u{00F6}')); // ö
        assert!(is_vowel('\u{00D6}')); // Ö
        assert!(!is_vowel('b'));
        assert!(!is_vowel('k'));
    }

    #[test]
    fn finnish_consonants() {
        assert!(is_consonant('b'));
        assert!(is_consonant('k'));
        assert!(is_consonant('K'));
        assert!(is_consonant('\u{0161}')); // š
        assert!(is_consonant('\u{0160}')); // Š
        assert!(!is_consonant('a'));
        assert!(!is_consonant('1'));
    }

    // -- Case functions --

    #[test]
    fn simple_lower_basic_latin() {
        assert_eq!(simple_lower('A'), 'a');
        assert_eq!(simple_lower('Z'), 'z');
        assert_eq!(simple_lower('a'), 'a');
    }

    #[test]
    fn simple_lower_extended() {
        assert_eq!(simple_lower('\u{00C4}'), '\u{00E4}'); // Ä -> ä
        assert_eq!(simple_lower('\u{00D6}'), '\u{00F6}'); // Ö -> ö
    }

    #[test]
    fn simple_upper_basic_latin() {
        assert_eq!(simple_upper('a'), 'A');
        assert_eq!(simple_upper('z'), 'Z');
        assert_eq!(simple_upper('A'), 'A');
    }

    #[test]
    fn simple_upper_extended() {
        assert_eq!(simple_upper('\u{00E4}'), '\u{00C4}'); // ä -> Ä
        assert_eq!(simple_upper('\u{00F6}'), '\u{00D6}'); // ö -> Ö
    }

    #[test]
    fn is_upper_basic() {
        assert!(is_upper('A'));
        assert!(is_upper('Z'));
        assert!(is_upper('\u{00C4}')); // Ä
        assert!(!is_upper('a'));
        assert!(!is_upper('1'));
    }

    #[test]
    fn is_upper_schwa() {
        // LATIN CAPITAL LETTER SCHWA is special-cased
        assert!(is_upper('\u{018F}'));
    }

    #[test]
    fn is_lower_basic() {
        assert!(is_lower('a'));
        assert!(is_lower('z'));
        assert!(is_lower('\u{00E4}')); // ä
        assert!(!is_lower('A'));
        assert!(!is_lower('1'));
    }

    #[test]
    fn whitespace_chars() {
        assert!(is_whitespace(' '));
        assert!(is_whitespace('\t'));
        assert!(is_whitespace('\n'));
        assert!(is_whitespace('\r'));
        assert!(is_whitespace('\u{00A0}')); // NO-BREAK SPACE
        assert!(is_whitespace('\u{3000}')); // IDEOGRAPHIC SPACE
        assert!(!is_whitespace('a'));
        assert!(!is_whitespace('0'));
    }

    #[test]
    fn equals_ignore_case_basic() {
        let a: Vec<char> = "Hello".chars().collect();
        let b: Vec<char> = "hello".chars().collect();
        let c: Vec<char> = "HELLO".chars().collect();
        let d: Vec<char> = "world".chars().collect();
        assert!(equals_ignore_case(&a, &b));
        assert!(equals_ignore_case(&a, &c));
        assert!(!equals_ignore_case(&a, &d));
    }

    #[test]
    fn equals_ignore_case_different_lengths() {
        let a: Vec<char> = "ab".chars().collect();
        let b: Vec<char> = "abc".chars().collect();
        assert!(!equals_ignore_case(&a, &b));
    }

    #[test]
    fn equals_ignore_case_empty() {
        assert!(equals_ignore_case(&[], &[]));
    }
}
