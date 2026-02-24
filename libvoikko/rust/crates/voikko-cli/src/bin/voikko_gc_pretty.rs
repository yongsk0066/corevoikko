// voikko-gc-pretty: Pretty-print grammar checker results.
//
// Ported from Python tools/bin/voikko-gc-pretty.
//
// Reads running text from stdin, checks grammar, and prints errors in a
// human-readable, diff-able format. Each paragraph is checked independently.
//
// Usage:
//   voikko-gc-pretty [-d DICT_PATH] [OPTIONS]
//
// Options:
//   -d, --dict-path PATH   Dictionary directory containing mor.vfst
//   --empty-line            Paragraphs are separated by empty lines
//                           (default: each line is a paragraph)
//   -h, --help              Print help

use std::io::{self, BufRead, Write};

fn handle_paragraph(
    paragraph: &str,
    handle: &voikko_fi::handle::VoikkoHandle,
    out: &mut io::BufWriter<io::StdoutLock<'_>>,
) {
    let errors = handle.grammar_errors(paragraph);
    let para_chars: Vec<char> = paragraph.chars().collect();

    for error in &errors {
        let _ = writeln!(out, "{paragraph}");

        let error_range: String = para_chars
            .iter()
            .skip(error.start_pos)
            .take(error.error_len)
            .collect();

        let _ = writeln!(
            out,
            "E: {} (start={})",
            error.short_description, error.start_pos
        );
        let _ = writeln!(out, "E: \"{error_range}\"");

        for suggestion in &error.suggestions {
            let _ = writeln!(out, "S:  \"{suggestion}\"");
        }
        let _ = writeln!(out, "=================================================");
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (dict_path, args) = voikko_cli::parse_dict_path(&args);

    if voikko_cli::wants_help(&args) {
        println!("voikko-gc-pretty: Pretty-print grammar check results.");
        println!();
        println!("Usage: voikko-gc-pretty [-d DICT_PATH] [OPTIONS]");
        println!();
        println!("Checks grammar of text read from stdin and prints errors.");
        println!("Normally paragraphs are separated by line feeds. Use option");
        println!("--empty-line if paragraphs are separated by empty lines.");
        println!();
        println!("Options:");
        println!("  -d, --dict-path PATH   Dictionary directory containing mor.vfst");
        println!("  --empty-line            Paragraphs separated by empty lines");
        println!("  -h, --help              Print this help");
        return;
    }

    let empty_line_separates = args.iter().any(|a| a == "--empty-line");

    let handle =
        voikko_cli::load_handle(dict_path.as_deref()).unwrap_or_else(|e| voikko_cli::fatal(&e));

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    if !empty_line_separates {
        // Each line is a paragraph
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("error reading stdin: {e}");
                    break;
                }
            };
            let paragraph = line.trim();
            if paragraph.is_empty() {
                continue;
            }
            handle_paragraph(paragraph, &handle, &mut out);
        }
    } else {
        // Paragraphs separated by empty lines
        let mut paragraph = String::new();
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("error reading stdin: {e}");
                    break;
                }
            };
            let trimmed = line.trim();
            if trimmed.is_empty() {
                if !paragraph.is_empty() {
                    handle_paragraph(&paragraph, &handle, &mut out);
                    paragraph.clear();
                }
                continue;
            }
            if !paragraph.is_empty() {
                paragraph.push(' ');
            }
            paragraph.push_str(trimmed);
        }
        // Handle trailing paragraph
        if !paragraph.is_empty() {
            handle_paragraph(&paragraph, &handle, &mut out);
        }
    }
}
