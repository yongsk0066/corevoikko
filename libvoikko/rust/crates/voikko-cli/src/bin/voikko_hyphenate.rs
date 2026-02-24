// voikko-hyphenate: Hyphenate words from stdin.
//
// Reads words from stdin (one per line) and prints the hyphenated form.
// By default uses '-' as separator. The raw pattern can also be shown.
//
// Usage:
//   voikko-hyphenate [-d DICT_PATH] [OPTIONS] [WORD...]
//
// Options:
//   -d, --dict-path PATH   Dictionary directory containing mor.vfst
//   --separator SEP         Hyphen separator character (default: -)
//   --pattern               Show raw hyphenation pattern instead of inserting hyphens
//   --no-ugly               Suppress ugly hyphenation points
//   --min-length N          Minimum word length for hyphenation (default: 2)
//   -h, --help              Print help

use std::io::{self, BufRead, Write};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (dict_path, args) = voikko_cli::parse_dict_path(&args);

    if voikko_cli::wants_help(&args) {
        println!("voikko-hyphenate: Hyphenate Finnish words.");
        println!();
        println!("Usage: voikko-hyphenate [-d DICT_PATH] [OPTIONS] [WORD...]");
        println!();
        println!("If WORD arguments are given, hyphenates each word.");
        println!("Otherwise reads words from stdin (one per line).");
        println!();
        println!("Options:");
        println!("  -d, --dict-path PATH   Dictionary directory containing mor.vfst");
        println!("  --separator SEP         Hyphen separator character (default: -)");
        println!("  --pattern               Show raw pattern instead of inserting hyphens");
        println!("  --no-ugly               Suppress ugly hyphenation points");
        println!("  --min-length N          Minimum word length for hyphenation (default: 2)");
        println!("  -h, --help              Print this help");
        return;
    }

    let mut separator = "-".to_string();
    let mut show_pattern = false;
    let mut no_ugly = false;
    let mut min_length: usize = 2;
    let mut words: Vec<String> = Vec::new();
    let mut skip_next = false;

    for (i, arg) in args.iter().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }
        match arg.as_str() {
            "--separator" => {
                if i + 1 < args.len() {
                    separator = args[i + 1].clone();
                    skip_next = true;
                }
            }
            "--pattern" => show_pattern = true,
            "--no-ugly" => no_ugly = true,
            "--min-length" => {
                if i + 1 < args.len() {
                    min_length = args[i + 1]
                        .parse()
                        .unwrap_or_else(|_| voikko_cli::fatal("invalid number for --min-length"));
                    skip_next = true;
                }
            }
            s if !s.starts_with('-') => words.push(arg.clone()),
            _ => {}
        }
    }

    let mut handle =
        voikko_cli::load_handle(dict_path.as_deref()).unwrap_or_else(|e| voikko_cli::fatal(&e));

    if no_ugly {
        handle.set_no_ugly_hyphenation(true);
    }
    handle.set_min_hyphenated_word_length(min_length);

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    let hyphenate_word = |word: &str,
                          handle: &voikko_fi::handle::VoikkoHandle,
                          out: &mut io::BufWriter<io::StdoutLock<'_>>| {
        if show_pattern {
            let pattern = handle.hyphenate(word);
            let _ = writeln!(out, "{word} {pattern}");
        } else {
            let result = handle.insert_hyphens(word, &separator, true);
            let _ = writeln!(out, "{result}");
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
            hyphenate_word(word, &handle, &mut out);
        }
    } else {
        for word in &words {
            hyphenate_word(word, &handle, &mut out);
        }
    }
}
