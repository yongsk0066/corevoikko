// voikko-tokenize: Tokenize text from stdin.
//
// Reads text from stdin and prints tokens with their types.
// Optionally also shows sentence boundaries.
//
// Usage:
//   voikko-tokenize [-d DICT_PATH] [OPTIONS]
//
// Options:
//   -d, --dict-path PATH   Dictionary directory containing mor.vfst
//   --sentences             Also show sentence boundaries
//   -h, --help              Print help

use std::io::{self, Read, Write};
use voikko_core::enums::TokenType;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (dict_path, args) = voikko_cli::parse_dict_path(&args);

    if voikko_cli::wants_help(&args) {
        println!("voikko-tokenize: Tokenize Finnish text.");
        println!();
        println!("Usage: voikko-tokenize [-d DICT_PATH] [OPTIONS]");
        println!();
        println!("Reads text from stdin, prints tokens with types:");
        println!("  WORD:        <text>");
        println!("  PUNCTUATION: <text>");
        println!("  WHITESPACE:  <text>");
        println!("  UNKNOWN:     <text>");
        println!();
        println!("Options:");
        println!("  -d, --dict-path PATH   Dictionary directory containing mor.vfst");
        println!("  --sentences             Also show sentence boundaries");
        println!("  -h, --help              Print this help");
        return;
    }

    let show_sentences = args.iter().any(|a| a == "--sentences");

    let handle =
        voikko_cli::load_handle(dict_path.as_deref()).unwrap_or_else(|e| voikko_cli::fatal(&e));

    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .unwrap_or_else(|e| voikko_cli::fatal(&format!("failed to read stdin: {e}")));

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    // Print tokens
    let _ = writeln!(out, "=== Tokens ===");
    for token in handle.tokens(&input) {
        let type_str = match token.token_type {
            TokenType::Word => "WORD",
            TokenType::Punctuation => "PUNCTUATION",
            TokenType::Whitespace => "WHITESPACE",
            TokenType::Unknown => "UNKNOWN",
            TokenType::None => "NONE",
        };
        let display_text = token
            .text
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t");
        let _ = writeln!(
            out,
            "{type_str:13} [{:>4}..{:>4}]: {display_text}",
            token.pos,
            token.pos + token.token_len
        );
    }

    // Print sentences if requested
    if show_sentences {
        let _ = writeln!(out);
        let _ = writeln!(out, "=== Sentences ===");
        let mut offset = 0;
        for sentence in handle.sentences(&input) {
            let end = offset + sentence.sentence_len;
            let snippet: String = input
                .chars()
                .skip(offset)
                .take(sentence.sentence_len)
                .collect();
            let snippet = snippet.replace('\n', "\\n");
            let type_str = format!("{:?}", sentence.sentence_type);
            let _ = writeln!(out, "{type_str:8} [{offset:>4}..{end:>4}]: {snippet}");
            offset = end;
        }
    }
}
