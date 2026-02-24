// voikko-baseform: Convert text to base form frequency list.
//
// Ported from Python tools/bin/voikko-convert-to-baseform.
//
// Reads running text from stdin, tokenizes it, performs morphological
// analysis on each word token, and produces a frequency list of base
// forms. Ambiguous words have their score split evenly among possible
// readings.
//
// Usage:
//   voikko-baseform [-d DICT_PATH]
//
// Options:
//   -d, --dict-path PATH   Dictionary directory containing mor.vfst
//   -h, --help              Print help

use std::collections::HashMap;
use std::io::{self, BufRead, Write};

use voikko_core::enums::TokenType;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (dict_path, args) = voikko_cli::parse_dict_path(&args);

    if voikko_cli::wants_help(&args) {
        println!("voikko-baseform: Convert text to base form frequency list.");
        println!();
        println!("Usage: voikko-baseform [-d DICT_PATH]");
        println!();
        println!("Reads text from stdin, tokenizes words, and produces a");
        println!("frequency list of base forms. Ambiguous words have their");
        println!("score split evenly among possible readings.");
        println!();
        println!("Options:");
        println!("  -d, --dict-path PATH   Dictionary directory containing mor.vfst");
        println!("  -h, --help              Print this help");
        return;
    }

    let handle =
        voikko_cli::load_handle(dict_path.as_deref()).unwrap_or_else(|e| voikko_cli::fatal(&e));

    let stdin = io::stdin();
    let mut known_freqs: HashMap<String, f64> = HashMap::new();
    let mut unknown_freqs: HashMap<String, u64> = HashMap::new();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("error reading stdin: {e}");
                break;
            }
        };
        let paragraph = line.trim().to_string();
        if paragraph.is_empty() {
            continue;
        }

        for token in handle.tokens(&paragraph) {
            if token.token_type != TokenType::Word {
                continue;
            }
            let word = &token.text;
            let analyses = filter_analysis_list(&handle.analyze(word), word);

            if analyses.is_empty() {
                *unknown_freqs.entry(word.clone()).or_insert(0) += 1;
            } else {
                let weight = 1.0 / analyses.len() as f64;
                for analysis in &analyses {
                    let baseform = analysis.get("BASEFORM").unwrap_or(word.as_str());
                    *known_freqs.entry(baseform.to_string()).or_insert(0.0) += weight;
                }
            }
        }
    }

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    // Sort known words by frequency (descending), then alphabetically
    let mut known_list: Vec<(String, f64)> = known_freqs.into_iter().collect();
    known_list.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });

    let _ = writeln!(out, "=== Known words ===");
    for (word, freq) in &known_list {
        let _ = writeln!(out, "{word}\t{freq}");
    }

    // Sort unknown words by frequency (descending), then alphabetically
    let mut unknown_list: Vec<(String, u64)> = unknown_freqs.into_iter().collect();
    unknown_list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let _ = writeln!(out, "=== Unknown words ===");
    for (word, freq) in &unknown_list {
        let _ = writeln!(out, "{word}\t{freq}");
    }
}

/// Filter analysis list to prefer non-proper-noun interpretations for
/// words that start with a lowercase letter.
///
/// Ported from the Python `filterAnalysisList` function.
fn filter_analysis_list(
    analyses: &[voikko_core::analysis::Analysis],
    word: &str,
) -> Vec<voikko_core::analysis::Analysis> {
    let chars: Vec<char> = word.chars().collect();
    if chars.len() < 2 || analyses.is_empty() {
        return analyses.to_vec();
    }

    if chars[0].is_uppercase() && chars[1].is_lowercase() {
        // "Kari" might be a proper noun -- keep all analyses
        return analyses.to_vec();
    }

    if chars[0].is_lowercase() {
        // "kari" cannot really be a proper noun, so prefer common nouns
        let filtered: Vec<voikko_core::analysis::Analysis> = analyses
            .iter()
            .filter(|a| {
                if let Some(structure) = a.get("STRUCTURE") {
                    !structure.starts_with("=i")
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        if !filtered.is_empty() {
            return filtered;
        }
    }

    analyses.to_vec()
}
