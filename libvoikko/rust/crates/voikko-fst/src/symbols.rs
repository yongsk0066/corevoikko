// Symbol table: char-to-index and index-to-string mapping.
// Origin: UnweightedTransducer.cpp:125-189, WeightedTransducer.cpp:130-194

use crate::VfstError;
use crate::flags::{FlagDiacriticParser, OpFeatureValue};
use hashbrown::HashMap;

/// Parsed symbol table from a VFST binary file.
///
/// Symbols are ordered in the binary as:
/// 1. Epsilon (index 0) -- empty string
/// 2. Flag diacritics -- strings starting with `@` (e.g., `@P.FEAT.VAL@`)
/// 3. Normal single-character symbols -- regular characters (e.g., `a`, `b`)
/// 4. Multi-character symbols -- strings starting with `[` (e.g., `[Ln]`, `[Bc]`)
///
/// Origin: UnweightedTransducer.cpp:125-189
pub struct SymbolTable {
    /// Maps symbol index to its string representation.
    pub symbol_strings: Vec<String>,
    /// Maps symbol index to string length in characters.
    pub symbol_lengths: Vec<usize>,
    /// Maps a single character to its symbol index (only for normal chars).
    pub char_to_symbol: HashMap<char, u16>,
    /// Maps symbol index to its parsed flag diacritic operation.
    /// Only populated for indices 1..first_normal_char.
    pub symbol_to_diacritic: Vec<OpFeatureValue>,
    /// Index of the first normal (non-flag, non-epsilon) character symbol.
    pub first_normal_char: u16,
    /// Index of the first multi-character symbol (starts with `[`).
    pub first_multi_char: u16,
    /// Number of distinct flag diacritic features.
    pub flag_feature_count: u16,
}

/// Parse the symbol table from the VFST binary data starting at offset 16 (after header).
///
/// Returns the parsed symbol table and the byte offset immediately after the symbol table
/// data (before padding). The caller is responsible for aligning this offset to the
/// transition table boundary.
///
/// Origin: UnweightedTransducer.cpp:125-189, WeightedTransducer.cpp:130-194
pub fn parse_symbol_table(data: &[u8], offset: usize) -> Result<(SymbolTable, usize), VfstError> {
    if offset + 2 > data.len() {
        return Err(VfstError::TooShort {
            expected: offset + 2,
            actual: data.len(),
        });
    }

    let symbol_count = u16::from_le_bytes([data[offset], data[offset + 1]]);
    let mut pos = offset + 2;

    let mut symbol_strings = Vec::with_capacity(symbol_count as usize);
    let mut symbol_lengths = Vec::with_capacity(symbol_count as usize);
    let mut char_to_symbol = HashMap::new();
    let mut symbol_to_diacritic = Vec::new();
    let mut first_normal_char: u16 = 0;
    let mut first_multi_char: u16 = 0;

    let mut flag_parser = FlagDiacriticParser::new();

    for i in 0..symbol_count {
        // Find the null terminator for this symbol
        let str_start = pos;
        while pos < data.len() && data[pos] != 0 {
            pos += 1;
        }
        if pos >= data.len() {
            return Err(VfstError::InvalidSymbolTable(
                "unterminated symbol string".to_string(),
            ));
        }

        let symbol_bytes = &data[str_start..pos];
        pos += 1; // skip null terminator

        if i == 0 {
            // Epsilon (index 0): empty string, zero length
            symbol_strings.push(String::new());
            symbol_lengths.push(0);
            symbol_to_diacritic.push(OpFeatureValue::default());
        } else {
            let symbol_str = std::str::from_utf8(symbol_bytes).map_err(|_| {
                VfstError::InvalidSymbolTable(format!("invalid UTF-8 in symbol {i}"))
            })?;
            let char_len = symbol_str.chars().count();

            symbol_strings.push(symbol_str.to_string());
            symbol_lengths.push(char_len);

            if first_normal_char == 0 {
                if symbol_str.starts_with('@') {
                    // Flag diacritic
                    let ofv = flag_parser.parse(symbol_str)?;
                    symbol_to_diacritic.push(ofv);
                } else {
                    first_normal_char = i;
                }
            } else if first_multi_char == 0 && symbol_str.starts_with('[') {
                first_multi_char = i;
            }

            // Build char-to-symbol mapping for normal single-char symbols
            if first_normal_char > 0 && first_multi_char == 0 {
                if let Some(ch) = symbol_str.chars().next() {
                    char_to_symbol.insert(ch, i);
                }
            }
        }
    }

    // If no normal chars were found, set first_normal_char to symbol_count
    // (the C++ code leaves it at 0 which works but is conceptually different)
    // We keep consistent with C++ behavior: 0 means "not yet found" which
    // means no normal chars exist.

    // If no multi-char symbols found, set to symbol_count
    if first_multi_char == 0 && first_normal_char > 0 {
        first_multi_char = symbol_count;
    }

    let flag_feature_count = flag_parser.feature_count();

    Ok((
        SymbolTable {
            symbol_strings,
            symbol_lengths,
            char_to_symbol,
            symbol_to_diacritic,
            first_normal_char,
            first_multi_char,
            flag_feature_count,
        },
        pos,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal symbol table binary:
    /// count(u16) + null-terminated strings
    fn make_symbol_table(symbols: &[&str]) -> Vec<u8> {
        let mut buf = Vec::new();
        let count = symbols.len() as u16;
        buf.extend_from_slice(&count.to_le_bytes());
        for sym in symbols {
            buf.extend_from_slice(sym.as_bytes());
            buf.push(0); // null terminator
        }
        buf
    }

    #[test]
    fn parse_simple_symbol_table() {
        // epsilon, then two normal chars
        let data = make_symbol_table(&["", "a", "b"]);
        let (table, end_pos) = parse_symbol_table(&data, 0).unwrap();

        assert_eq!(table.symbol_strings.len(), 3);
        assert_eq!(table.symbol_strings[0], "");
        assert_eq!(table.symbol_strings[1], "a");
        assert_eq!(table.symbol_strings[2], "b");

        assert_eq!(table.symbol_lengths[0], 0);
        assert_eq!(table.symbol_lengths[1], 1);
        assert_eq!(table.symbol_lengths[2], 1);

        assert_eq!(table.first_normal_char, 1);
        assert_eq!(table.first_multi_char, 3); // no multi-chars -> set to count

        assert_eq!(*table.char_to_symbol.get(&'a').unwrap(), 1);
        assert_eq!(*table.char_to_symbol.get(&'b').unwrap(), 2);
        assert_eq!(table.flag_feature_count, 0);
        assert_eq!(end_pos, data.len());
    }

    #[test]
    fn parse_with_flags_and_multi_chars() {
        let data = make_symbol_table(&["", "@P.CASE.NOM@", "@C.NUM@", "a", "b", "[Ln]", "[Bc]"]);
        let (table, _) = parse_symbol_table(&data, 0).unwrap();

        assert_eq!(table.symbol_strings.len(), 7);
        assert_eq!(table.first_normal_char, 3);
        assert_eq!(table.first_multi_char, 5);
        assert_eq!(table.flag_feature_count, 2); // CASE and NUM
        assert_eq!(table.symbol_to_diacritic.len(), 3); // epsilon + 2 flags

        // Normal chars should be in char_to_symbol
        assert_eq!(*table.char_to_symbol.get(&'a').unwrap(), 3);
        assert_eq!(*table.char_to_symbol.get(&'b').unwrap(), 4);

        // Multi-char symbols should NOT be in char_to_symbol
        assert!(!table.char_to_symbol.contains_key(&'['));
    }

    #[test]
    fn parse_epsilon_only() {
        let data = make_symbol_table(&[""]);
        let (table, _) = parse_symbol_table(&data, 0).unwrap();
        assert_eq!(table.symbol_strings.len(), 1);
        assert_eq!(table.first_normal_char, 0);
        assert_eq!(table.char_to_symbol.len(), 0);
    }

    #[test]
    fn parse_with_offset() {
        // Simulate data starting after a 16-byte header
        let mut data = vec![0u8; 16]; // header placeholder
        let sym_data = make_symbol_table(&["", "x", "y"]);
        data.extend_from_slice(&sym_data);

        let (table, end_pos) = parse_symbol_table(&data, 16).unwrap();
        assert_eq!(table.symbol_strings.len(), 3);
        assert_eq!(table.first_normal_char, 1);
        assert_eq!(end_pos, data.len());
    }

    #[test]
    fn parse_multibyte_utf8_symbols() {
        let data = make_symbol_table(&["", "\u{00e4}", "\u{00f6}"]); // ä, ö
        let (table, _) = parse_symbol_table(&data, 0).unwrap();
        assert_eq!(table.symbol_strings[1], "\u{00e4}");
        assert_eq!(table.symbol_strings[2], "\u{00f6}");
        assert_eq!(table.symbol_lengths[1], 1); // 1 character
        assert_eq!(table.symbol_lengths[2], 1);
        assert_eq!(*table.char_to_symbol.get(&'\u{00e4}').unwrap(), 1);
        assert_eq!(*table.char_to_symbol.get(&'\u{00f6}').unwrap(), 2);
    }

    #[test]
    fn reject_truncated_data() {
        let data = [0u8; 1]; // too short for count
        let result = parse_symbol_table(&data, 0);
        assert!(result.is_err());
    }

    #[test]
    fn reject_unterminated_string() {
        let mut data = Vec::new();
        data.extend_from_slice(&2u16.to_le_bytes());
        data.push(0); // epsilon
        data.extend_from_slice(b"abc"); // no null terminator
        let result = parse_symbol_table(&data, 0);
        assert!(result.is_err());
    }
}
