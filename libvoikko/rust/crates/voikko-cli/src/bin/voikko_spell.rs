// voikko-spell: Check spelling of words from stdin.
//
// Reads words from stdin (one per line) and reports whether each word
// is correctly spelled. Output format matches the C++ voikkospell tool:
//   C: word    (correct)
//   W: word    (wrong / misspelled)
//
// Usage:
//   voikko-spell [-d DICT_PATH] [OPTIONS]
//
// Options:
//   -d, --dict-path PATH   Dictionary directory containing mor.vfst
//   -s, --suggest           Also print suggestions for misspelled words
//   --ignore-dot            Ignore trailing dot
//   --ignore-numbers        Ignore words containing numbers
//   -h, --help              Print help

use std::io::{self, BufRead, Write};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (dict_path, args) = voikko_cli::parse_dict_path(&args);

    if voikko_cli::wants_help(&args) {
        println!("voikko-spell: Check spelling of words from stdin.");
        println!();
        println!("Usage: voikko-spell [-d DICT_PATH] [OPTIONS]");
        println!();
        println!("Reads words from stdin (one per line). Prints:");
        println!("  C: word    (correct)");
        println!("  W: word    (misspelled)");
        println!();
        println!("Options:");
        println!("  -d, --dict-path PATH   Dictionary directory containing mor.vfst");
        println!("  -s, --suggest           Also print suggestions for misspelled words");
        println!("  --ignore-dot            Ignore trailing dot in words");
        println!("  --ignore-numbers        Ignore words containing numbers");
        println!("  -h, --help              Print this help");
        return;
    }

    let show_suggestions = args.iter().any(|a| a == "-s" || a == "--suggest");
    let ignore_dot = args.iter().any(|a| a == "--ignore-dot");
    let ignore_numbers = args.iter().any(|a| a == "--ignore-numbers");

    let mut handle = voikko_cli::load_handle(dict_path.as_deref())
        .unwrap_or_else(|e| voikko_cli::fatal(&e));

    if ignore_dot {
        handle.set_ignore_dot(true);
    }
    if ignore_numbers {
        handle.set_ignore_numbers(true);
    }

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

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

        if handle.spell(word) {
            let _ = writeln!(out, "C: {word}");
        } else {
            let _ = writeln!(out, "W: {word}");
            if show_suggestions {
                for suggestion in handle.suggest(word) {
                    let _ = writeln!(out, "S: {suggestion}");
                }
            }
        }
    }
}
