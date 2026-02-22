// voikko-analyze: Morphological analysis of words from stdin.
//
// Reads words from stdin (one per line) and prints all morphological
// analyses for each word. Each analysis is printed as key=value pairs.
//
// Usage:
//   voikko-analyze [-d DICT_PATH] [WORD...]
//
// Options:
//   -d, --dict-path PATH   Dictionary directory containing mor.vfst
//   -h, --help              Print help

use std::io::{self, BufRead, Write};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (dict_path, args) = voikko_cli::parse_dict_path(&args);

    if voikko_cli::wants_help(&args) {
        println!("voikko-analyze: Morphological analysis of Finnish words.");
        println!();
        println!("Usage: voikko-analyze [-d DICT_PATH] [WORD...]");
        println!();
        println!("If WORD arguments are given, analyzes each word.");
        println!("Otherwise reads words from stdin (one per line).");
        println!();
        println!("Options:");
        println!("  -d, --dict-path PATH   Dictionary directory containing mor.vfst");
        println!("  -h, --help              Print this help");
        return;
    }

    let words: Vec<String> = args.iter().filter(|a| !a.starts_with('-')).cloned().collect();

    let handle = voikko_cli::load_handle(dict_path.as_deref())
        .unwrap_or_else(|e| voikko_cli::fatal(&e));

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    let analyze_word = |word: &str, handle: &voikko_fi::handle::VoikkoHandle, out: &mut io::BufWriter<io::StdoutLock<'_>>| {
        let analyses = handle.analyze(word);
        if analyses.is_empty() {
            let _ = writeln!(out, "{word}: (no analysis)");
        } else {
            let _ = writeln!(out, "{word}:");
            for (i, analysis) in analyses.iter().enumerate() {
                let _ = writeln!(out, "  Analysis {}:", i + 1);
                let mut keys: Vec<&str> = analysis.keys();
                keys.sort();
                for key in keys {
                    if let Some(val) = analysis.get(key) {
                        let _ = writeln!(out, "    {key}={val}");
                    }
                }
            }
        }
    };

    if words.is_empty() {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("error reading stdin: {e}");
                    break;
                }
            };
            let word = line.trim();
            if word.is_empty() {
                continue;
            }
            analyze_word(word, &handle, &mut out);
        }
    } else {
        for word in &words {
            analyze_word(word, &handle, &mut out);
        }
    }
}
