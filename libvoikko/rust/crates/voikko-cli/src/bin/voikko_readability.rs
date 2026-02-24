// voikko-readability: Calculate readability statistics for Finnish text.
//
// Ported from Python tools/bin/voikko-readability + pylib/voikkostatistics.py.
//
// Reads text from stdin and calculates:
//   - Number of sentences, words, syllables, characters
//   - Flesch Reading Ease
//   - Flesch-Kincaid Grade Level
//   - Wiio simple grade level (Finnish readability metric)
//
// Usage:
//   voikko-readability [-d DICT_PATH]
//
// Options:
//   -d, --dict-path PATH   Dictionary directory containing mor.vfst
//   -h, --help              Print help
//
// References:
//   - Flesch-Kincaid: https://en.wikipedia.org/wiki/Flesch%E2%80%93Kincaid_readability_test
//   - Wiio: http://media.tkk.fi/GTTS/Suomi/dt&raportit/DI_J_Haataja.pdf

use std::collections::HashMap;
use std::io::{self, Read, Write};

use voikko_core::enums::{SentenceType, TokenType};

/// Count syllables in a word by counting hyphenation points.
fn syllables_in_word(word: &str, handle: &voikko_fi::handle::VoikkoHandle) -> usize {
    let pattern = handle.hyphenate(word);
    let hyphens = pattern.chars().filter(|&c| c != ' ').count();
    hyphens + 1
}

/// Count syllables in the base form of a word.
/// Returns 0 if the word has no analysis.
fn syllables_in_baseform(word: &str, handle: &voikko_fi::handle::VoikkoHandle) -> usize {
    let analyses = handle.analyze(word);
    for analysis in &analyses {
        if let Some(baseform) = analysis.get("BASEFORM") {
            return syllables_in_word(baseform, handle);
        }
    }
    0
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (dict_path, args) = voikko_cli::parse_dict_path(&args);

    if voikko_cli::wants_help(&args) {
        println!("voikko-readability: Calculate readability statistics for Finnish text.");
        println!();
        println!("Usage: voikko-readability [-d DICT_PATH]");
        println!();
        println!("Reads text from stdin and calculates readability metrics:");
        println!("  - Sentence, word, syllable, character counts");
        println!("  - Flesch Reading Ease");
        println!("  - Flesch-Kincaid Grade Level");
        println!("  - Wiio simple grade level (Finnish metric)");
        println!();
        println!("Options:");
        println!("  -d, --dict-path PATH   Dictionary directory containing mor.vfst");
        println!("  -h, --help              Print this help");
        return;
    }

    let mut handle =
        voikko_cli::load_handle(dict_path.as_deref()).unwrap_or_else(|e| voikko_cli::fatal(&e));

    // Match the Python tool: no_ugly=false, hyphenate_unknown=true
    handle.set_no_ugly_hyphenation(false);
    handle.set_hyphenate_unknown_words(true);

    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .unwrap_or_else(|e| voikko_cli::fatal(&format!("failed to read stdin: {e}")));

    // Count sentences
    let mut sentence_count: usize = 0;
    for sentence in handle.sentences(&input) {
        if matches!(
            sentence.sentence_type,
            SentenceType::None | SentenceType::Probable
        ) {
            sentence_count += 1;
        }
    }

    // Count words, syllables, characters
    let mut word_count: usize = 0;
    let mut known_words: usize = 0;
    let mut syllable_count: usize = 0;
    let mut character_count: usize = 0;
    let mut punctuation_count: usize = 0;
    let mut baseform_histogram: HashMap<usize, usize> = HashMap::new();

    for token in handle.tokens(&input) {
        match token.token_type {
            TokenType::Word => {
                word_count += 1;
                syllable_count += syllables_in_word(&token.text, &handle);
                character_count += token.text.chars().count();

                let syls = syllables_in_baseform(&token.text, &handle);
                *baseform_histogram.entry(syls).or_insert(0) += 1;
                if syls > 0 {
                    known_words += 1;
                }
            }
            TokenType::Punctuation => {
                punctuation_count += 1;
            }
            _ => {}
        }
    }

    // Calculate derived statistics
    let flesch_reading_ease;
    let flesch_kincaid_grade;
    let wiio_simple;

    if known_words == 0 || sentence_count == 0 {
        flesch_reading_ease = 0.0;
        flesch_kincaid_grade = 0.0;
        wiio_simple = 0.0;
    } else {
        let words_per_sentence = word_count as f64 / sentence_count as f64;
        let syllables_per_word = syllable_count as f64 / word_count as f64;

        flesch_reading_ease = 206.823 - 1.015 * words_per_sentence - 84.6 * syllables_per_word;
        flesch_kincaid_grade = 0.39 * words_per_sentence + 11.8 * syllables_per_word - 15.59;

        // Wiio: count words with baseform >= 4 syllables
        let long_words: usize = baseform_histogram
            .iter()
            .filter(|(bin, _)| **bin >= 4)
            .map(|(_, &count)| count)
            .sum();
        wiio_simple = 2.7 + 30.0 * long_words as f64 / known_words as f64;
    }

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    let _ = writeln!(out, "Number of sentences: {sentence_count}");
    let _ = writeln!(out, "Number of words: {word_count}");
    let _ = writeln!(out, "Number of syllables: {syllable_count}");
    let _ = writeln!(
        out,
        "Number of characters (without punctuation): {character_count}"
    );
    let _ = writeln!(
        out,
        "Number of characters (with punctuation): {}",
        character_count + punctuation_count
    );
    let _ = writeln!(out, "Flesch Reading Ease: {flesch_reading_ease:.1}");
    let _ = writeln!(out, "Flesch-Kincaid Grade Level: {flesch_kincaid_grade:.1}");
    let _ = writeln!(
        out,
        "Wiion yksinkertainen luokkataso (1-12): {wiio_simple:.1}"
    );
}
