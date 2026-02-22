// voikko-suggest: Generate spelling suggestions for words from stdin.
//
// Reads words from stdin (one per line) and prints suggestions for
// misspelled words. Correctly spelled words are printed as-is.
//
// Usage:
//   voikko-suggest [-d DICT_PATH] [OPTIONS] [WORD...]
//
// Options:
//   -d, --dict-path PATH   Dictionary directory containing mor.vfst
//   -n, --max-suggestions N Maximum number of suggestions (default: 5)
//   -h, --help              Print help

use std::io::{self, BufRead, Write};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (dict_path, args) = voikko_cli::parse_dict_path(&args);

    if voikko_cli::wants_help(&args) {
        println!("voikko-suggest: Generate spelling suggestions.");
        println!();
        println!("Usage: voikko-suggest [-d DICT_PATH] [OPTIONS] [WORD...]");
        println!();
        println!("If WORD arguments are given, suggests for each word.");
        println!("Otherwise reads words from stdin (one per line).");
        println!();
        println!("Options:");
        println!("  -d, --dict-path PATH     Dictionary directory containing mor.vfst");
        println!("  -n, --max-suggestions N  Maximum number of suggestions (default: 5)");
        println!("  -h, --help               Print this help");
        return;
    }

    let mut max_suggestions: usize = 5;
    let mut words: Vec<String> = Vec::new();
    let mut skip_next = false;

    for (i, arg) in args.iter().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg == "-n" || arg == "--max-suggestions" {
            if i + 1 < args.len() {
                max_suggestions = args[i + 1]
                    .parse()
                    .unwrap_or_else(|_| voikko_cli::fatal("invalid number for --max-suggestions"));
                skip_next = true;
            } else {
                voikko_cli::fatal("--max-suggestions requires a value");
            }
        } else if !arg.starts_with('-') {
            words.push(arg.clone());
        }
    }

    let mut handle = voikko_cli::load_handle(dict_path.as_deref())
        .unwrap_or_else(|e| voikko_cli::fatal(&e));
    handle.set_max_suggestions(max_suggestions);

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    let suggest_word = |word: &str, handle: &voikko_fi::handle::VoikkoHandle, out: &mut io::BufWriter<io::StdoutLock<'_>>| {
        if handle.spell(word) {
            let _ = writeln!(out, "{word} (correct)");
        } else {
            let suggestions = handle.suggest(word);
            if suggestions.is_empty() {
                let _ = writeln!(out, "{word}: (no suggestions)");
            } else {
                let _ = writeln!(out, "{word}:");
                for s in &suggestions {
                    let _ = writeln!(out, "  {s}");
                }
            }
        }
    };

    if words.is_empty() {
        // Read from stdin
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
            suggest_word(word, &handle, &mut out);
        }
    } else {
        for word in &words {
            suggest_word(word, &handle, &mut out);
        }
    }
}
