// Shared Finnish language constants used across multiple modules.
//
// These constants capture phonological rules of Finnish that are needed
// in more than one subsystem (suggestion generators, hyphenator, etc.).
// Individual constants may be unused when their consumer feature is disabled.

/// Back vowels used in Finnish vowel harmony (lowercase + uppercase).
///
/// Origin: SuggestionGeneratorVowelChange.cpp:35, SuggestionGeneratorSwap.cpp:38
#[allow(dead_code)]
pub(crate) const BACK_VOWELS: &[char] = &['a', 'o', 'u', 'A', 'O', 'U'];

/// Front vowels corresponding to back vowels (same index order).
///
/// Origin: SuggestionGeneratorVowelChange.cpp:36, SuggestionGeneratorSwap.cpp:39
#[allow(dead_code)]
pub(crate) const FRONT_VOWELS: &[char] =
    &['\u{00E4}', '\u{00F6}', 'y', '\u{00C4}', '\u{00D6}', 'Y'];

/// Vowel pairs that may be split by a hyphen in Finnish.
/// These are vowel combinations that do NOT form diphthongs and can be separated.
///
/// Origin: AnalyzerToFinnishHyphenatorAdapter.cpp:42-44 (SPLIT_VOWELS)
#[allow(dead_code)]
pub(crate) const SPLIT_VOWELS: &[[char; 2]] = &[
    ['a', 'e'],
    ['a', 'o'],
    ['e', 'a'],
    ['e', 'o'],
    ['i', 'a'],
    ['i', 'o'],
    ['o', 'a'],
    ['o', 'e'],
    ['u', 'a'],
    ['u', 'e'],
    ['y', 'e'],
    ['e', '\u{00E4}'], // eä
    ['e', '\u{00F6}'], // eö
    ['i', '\u{00E4}'], // iä
    ['i', '\u{00F6}'], // iö
    ['y', '\u{00E4}'], // yä
    ['\u{00E4}', 'e'], // äe
    ['\u{00F6}', 'e'], // öe
];
