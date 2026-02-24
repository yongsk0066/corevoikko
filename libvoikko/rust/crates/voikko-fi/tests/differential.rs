//! Differential tests: compare Rust output against C++ golden files.
//!
//! These tests require the mor.vfst dictionary file.
//! Set VOIKKO_DICT_PATH to the directory containing mor.vfst,
//! or place it at ../../test-data/mor.vfst.
//!
//! Run: VOIKKO_DICT_PATH=/path/to/vvfst cargo test -p voikko-fi --test differential

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use serde_json::Value;
use voikko_fi::handle::VoikkoHandle;

// ---------------------------------------------------------------------------
// Helper: locate dictionary files
// ---------------------------------------------------------------------------

/// Find the mor.vfst file. Checks VOIKKO_DICT_PATH env var first,
/// then falls back to the default test-data location.
fn find_mor_vfst() -> Option<PathBuf> {
    // Try VOIKKO_DICT_PATH (directory containing mor.vfst)
    if let Ok(dir) = std::env::var("VOIKKO_DICT_PATH") {
        let path = PathBuf::from(&dir).join("mor.vfst");
        if path.exists() {
            return Some(path);
        }
        // Maybe the env var points directly to the file
        let path = PathBuf::from(&dir);
        if path.is_file() {
            return Some(path);
        }
    }

    // Try VOIKKO_MOR_VFST (direct path to file)
    if let Ok(path) = std::env::var("VOIKKO_MOR_VFST") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }

    // Fall back to ../../test-data/mor.vfst (relative to crate root)
    let fallback = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-data/mor.vfst");
    if fallback.exists() {
        return Some(fallback);
    }

    None
}

/// Load the golden JSON file from the differential test data directory.
fn load_golden(filename: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/differential/golden")
        .join(filename);
    let contents = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read golden file {}: {}", path.display(), e));
    serde_json::from_str(&contents)
        .unwrap_or_else(|e| panic!("failed to parse golden file {}: {}", path.display(), e))
}

/// Create a VoikkoHandle or skip the test if dictionary is not found.
fn create_handle() -> Option<VoikkoHandle> {
    let mor_path = match find_mor_vfst() {
        Some(p) => p,
        None => {
            eprintln!(
                "SKIP: mor.vfst not found. Set VOIKKO_DICT_PATH or place it at test-data/mor.vfst"
            );
            return None;
        }
    };

    let mor_data = std::fs::read(&mor_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", mor_path.display(), e));

    let handle = VoikkoHandle::from_bytes(&mor_data, None, "fi")
        .unwrap_or_else(|e| panic!("failed to create VoikkoHandle: {}", e));

    Some(handle)
}

// ---------------------------------------------------------------------------
// Hyphenation pattern conversion
// ---------------------------------------------------------------------------

/// Convert a Rust hyphenation pattern (e.g., "   - " for "koira") into the
/// golden file format (e.g., "koi-ra").
///
/// The pattern string has the same character length as the input word.
/// Characters in the pattern:
/// - ' ': no hyphenation point
/// - '-': hyphenation point before this character
/// - '=': compound boundary hyphenation before this character
fn pattern_to_hyphenated(word: &str, pattern: &str) -> String {
    let word_chars: Vec<char> = word.chars().collect();
    let pattern_chars: Vec<char> = pattern.chars().collect();
    let mut result = String::with_capacity(word.len() + 10);

    for (i, ch) in word_chars.iter().enumerate() {
        if i < pattern_chars.len() && (pattern_chars[i] == '-' || pattern_chars[i] == '=') {
            result.push('-');
        }
        result.push(*ch);
    }

    result
}

// ---------------------------------------------------------------------------
// Analysis comparison helpers
// ---------------------------------------------------------------------------

/// Convert a serde_json::Value (object) into a HashMap<String, String>
/// for comparison with Analysis attributes.
fn json_object_to_map(obj: &Value) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Value::Object(o) = obj {
        for (k, v) in o {
            match v {
                Value::String(s) => {
                    map.insert(k.clone(), s.clone());
                }
                _ => {
                    map.insert(k.clone(), v.to_string());
                }
            }
        }
    }
    map
}

/// Compare two sets of analyses (Rust vs golden), treating each analysis
/// as a set of key-value pairs. The order of analyses may differ between
/// C++ and Rust, so we compare as sets.
///
/// Returns a list of mismatch descriptions, empty if all match.
fn compare_analyses(
    word: &str,
    rust_analyses: &[HashMap<String, String>],
    golden_analyses: &[Value],
) -> Vec<String> {
    let mut mismatches = Vec::new();

    let golden_maps: Vec<HashMap<String, String>> =
        golden_analyses.iter().map(json_object_to_map).collect();

    // Check count
    if rust_analyses.len() != golden_maps.len() {
        mismatches.push(format!(
            "  [{}] analysis count: rust={}, golden={}",
            word,
            rust_analyses.len(),
            golden_maps.len()
        ));
    }

    // Find analyses in golden but not in rust
    for (i, golden) in golden_maps.iter().enumerate() {
        if !rust_analyses.iter().any(|r| r == golden) {
            mismatches.push(format!(
                "  [{}] golden analysis #{} not found in rust output: {:?}",
                word, i, golden
            ));
        }
    }

    // Find analyses in rust but not in golden
    for (i, rust_a) in rust_analyses.iter().enumerate() {
        if !golden_maps.iter().any(|g| g == rust_a) {
            mismatches.push(format!(
                "  [{}] rust analysis #{} not found in golden output: {:?}",
                word, i, rust_a
            ));
        }
    }

    mismatches
}

// ===========================================================================
// Tests
// ===========================================================================

#[test]
fn differential_spell() {
    let handle = match create_handle() {
        Some(h) => h,
        None => return,
    };

    let golden = load_golden("spell.json");
    let golden_map = golden.as_object().expect("spell.json should be an object");

    let mut mismatches = Vec::new();
    let mut total = 0;

    // Sort keys for deterministic output
    let mut words: Vec<&String> = golden_map.keys().collect();
    words.sort();

    for word in &words {
        total += 1;
        let expected = golden_map[*word]
            .as_bool()
            .unwrap_or_else(|| panic!("spell.json value for '{}' should be boolean", word));
        let actual = handle.spell(word);

        if actual != expected {
            mismatches.push(format!(
                "  [{}] expected={}, got={}",
                word, expected, actual
            ));
        }
    }

    if !mismatches.is_empty() {
        eprintln!("\n=== SPELL MISMATCHES: {}/{} ===", mismatches.len(), total);
        for m in &mismatches {
            eprintln!("{}", m);
        }
        eprintln!("=== END SPELL MISMATCHES ===\n");
    }

    assert!(
        mismatches.is_empty(),
        "spell: {}/{} mismatches (see stderr for details)",
        mismatches.len(),
        total,
    );
}

#[test]
fn differential_analyze() {
    let handle = match create_handle() {
        Some(h) => h,
        None => return,
    };

    let golden = load_golden("analyze.json");
    let golden_map = golden
        .as_object()
        .expect("analyze.json should be an object");

    let mut mismatches = Vec::new();
    let mut total = 0;

    let mut words: Vec<&String> = golden_map.keys().collect();
    words.sort();

    for word in &words {
        total += 1;
        let golden_analyses = golden_map[*word]
            .as_array()
            .unwrap_or_else(|| panic!("analyze.json value for '{}' should be an array", word));

        let rust_analyses_raw = handle.analyze(word);
        let rust_analyses: Vec<HashMap<String, String>> = rust_analyses_raw
            .iter()
            .map(|a| a.attributes().clone())
            .collect();

        let word_mismatches = compare_analyses(word, &rust_analyses, golden_analyses);
        mismatches.extend(word_mismatches);
    }

    if !mismatches.is_empty() {
        eprintln!(
            "\n=== ANALYZE MISMATCHES: {} issues across {} words ===",
            mismatches.len(),
            total
        );
        for m in &mismatches {
            eprintln!("{}", m);
        }
        eprintln!("=== END ANALYZE MISMATCHES ===\n");
    }

    assert!(
        mismatches.is_empty(),
        "analyze: {} mismatch issues across {} words (see stderr for details)",
        mismatches.len(),
        total,
    );
}

#[test]
fn differential_hyphenate() {
    let handle = match create_handle() {
        Some(h) => h,
        None => return,
    };

    let golden = load_golden("hyphenate.json");
    let golden_map = golden
        .as_object()
        .expect("hyphenate.json should be an object");

    let mut mismatches = Vec::new();
    let mut total = 0;

    let mut words: Vec<&String> = golden_map.keys().collect();
    words.sort();

    for word in &words {
        total += 1;
        let expected = golden_map[*word]
            .as_str()
            .unwrap_or_else(|| panic!("hyphenate.json value for '{}' should be a string", word));

        let pattern = handle.hyphenate(word);
        let actual = pattern_to_hyphenated(word, &pattern);

        if actual != expected {
            mismatches.push(format!(
                "  [{}] expected=\"{}\", got=\"{}\" (pattern=\"{}\")",
                word, expected, actual, pattern
            ));
        }
    }

    if !mismatches.is_empty() {
        eprintln!(
            "\n=== HYPHENATE MISMATCHES: {}/{} ===",
            mismatches.len(),
            total
        );
        for m in &mismatches {
            eprintln!("{}", m);
        }
        eprintln!("=== END HYPHENATE MISMATCHES ===\n");
    }

    assert!(
        mismatches.is_empty(),
        "hyphenate: {}/{} mismatches (see stderr for details)",
        mismatches.len(),
        total,
    );
}

#[test]
fn differential_suggest() {
    let handle = match create_handle() {
        Some(h) => h,
        None => return,
    };

    let golden = load_golden("suggest.json");
    let golden_map = golden
        .as_object()
        .expect("suggest.json should be an object");

    let mut mismatches = Vec::new();
    let mut total = 0;

    let mut words: Vec<&String> = golden_map.keys().collect();
    words.sort();

    for word in &words {
        total += 1;
        let golden_suggestions: Vec<String> = golden_map[*word]
            .as_array()
            .unwrap_or_else(|| panic!("suggest.json value for '{}' should be an array", word))
            .iter()
            .map(|v| {
                v.as_str()
                    .unwrap_or_else(|| {
                        panic!("suggest.json suggestion for '{}' should be a string", word)
                    })
                    .to_string()
            })
            .collect();

        let rust_suggestions = handle.suggest(word);

        // Compare as sets: the golden file's suggestions should all appear
        // in the Rust output (order may differ between C++ and Rust).
        let golden_set: HashSet<&str> = golden_suggestions.iter().map(|s| s.as_str()).collect();
        let rust_set: HashSet<&str> = rust_suggestions.iter().map(|s| s.as_str()).collect();

        // Check which golden suggestions are missing from Rust
        let missing: Vec<&str> = golden_set.difference(&rust_set).copied().collect();
        // Check which Rust suggestions are extra (not in golden)
        let extra: Vec<&str> = rust_set.difference(&golden_set).copied().collect();

        if !missing.is_empty() || !extra.is_empty() {
            let mut parts = Vec::new();
            if !missing.is_empty() {
                parts.push(format!("missing={:?}", missing));
            }
            if !extra.is_empty() {
                parts.push(format!("extra={:?}", extra));
            }
            mismatches.push(format!(
                "  [{}] golden={:?}, rust={:?} ({})",
                word,
                golden_suggestions,
                rust_suggestions,
                parts.join(", ")
            ));
        }
    }

    if !mismatches.is_empty() {
        eprintln!(
            "\n=== SUGGEST MISMATCHES: {}/{} ===",
            mismatches.len(),
            total
        );
        for m in &mismatches {
            eprintln!("{}", m);
        }
        eprintln!("=== END SUGGEST MISMATCHES ===\n");
    }

    assert!(
        mismatches.is_empty(),
        "suggest: {}/{} mismatches (see stderr for details)",
        mismatches.len(),
        total,
    );
}
