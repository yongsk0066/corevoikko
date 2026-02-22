// Criterion benchmarks for voikko-fi.
//
// Requires a mor.vfst dictionary file. Set VOIKKO_DICT_PATH to the directory
// containing mor.vfst, or place it at ../../test-data/mor.vfst relative to
// the crate root. If the dictionary is not found the benchmarks print a
// message and run no-op iterations.
//
// Run:
//   cargo bench -p voikko-fi --features handle
//   VOIKKO_DICT_PATH=/path/to/dict cargo bench -p voikko-fi --features handle

use criterion::{Criterion, criterion_group, criterion_main};

// ---------------------------------------------------------------------------
// Dictionary discovery
// ---------------------------------------------------------------------------

fn find_mor_vfst() -> Option<std::path::PathBuf> {
    if let Ok(dir) = std::env::var("VOIKKO_DICT_PATH") {
        let path = std::path::PathBuf::from(&dir).join("mor.vfst");
        if path.exists() {
            return Some(path);
        }
    }
    let fallback =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-data/mor.vfst");
    if fallback.exists() {
        return Some(fallback);
    }
    None
}

fn load_wordlist() -> Vec<String> {
    let path =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/differential/wordlist.txt");
    std::fs::read_to_string(&path)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect()
}

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

/// Spell-check all 246 words from the differential test wordlist.
fn bench_spell_words(c: &mut Criterion) {
    let Some(dict_path) = find_mor_vfst() else {
        eprintln!("[bench_spell_words] mor.vfst not found — skipping (set VOIKKO_DICT_PATH)");
        c.bench_function("spell_words (skipped)", |b| b.iter(|| {}));
        return;
    };

    let mor_data = std::fs::read(&dict_path).expect("failed to read mor.vfst");
    let handle =
        voikko_fi::handle::VoikkoHandle::from_bytes(&mor_data, None, "fi").expect("VoikkoHandle");
    let words = load_wordlist();

    c.bench_function("spell_246_words", |b| {
        b.iter(|| {
            for word in &words {
                std::hint::black_box(handle.spell(word));
            }
        });
    });
}

/// Run analyze() on the first 100 words from the wordlist.
fn bench_analyze_words(c: &mut Criterion) {
    let Some(dict_path) = find_mor_vfst() else {
        eprintln!("[bench_analyze_words] mor.vfst not found — skipping (set VOIKKO_DICT_PATH)");
        c.bench_function("analyze_words (skipped)", |b| b.iter(|| {}));
        return;
    };

    let mor_data = std::fs::read(&dict_path).expect("failed to read mor.vfst");
    let handle =
        voikko_fi::handle::VoikkoHandle::from_bytes(&mor_data, None, "fi").expect("VoikkoHandle");
    let words: Vec<String> = load_wordlist().into_iter().take(100).collect();

    c.bench_function("analyze_100_words", |b| {
        b.iter(|| {
            for word in &words {
                std::hint::black_box(handle.analyze(word));
            }
        });
    });
}

/// Measure raw FST traversal via FinnishVfstAnalyzer on 50 common words.
fn bench_fst_traverse(c: &mut Criterion) {
    let Some(dict_path) = find_mor_vfst() else {
        eprintln!("[bench_fst_traverse] mor.vfst not found — skipping (set VOIKKO_DICT_PATH)");
        c.bench_function("fst_traverse (skipped)", |b| b.iter(|| {}));
        return;
    };

    let mor_data = std::fs::read(&dict_path).expect("failed to read mor.vfst");
    let analyzer = voikko_fi::morphology::FinnishVfstAnalyzer::from_bytes(&mor_data)
        .expect("FinnishVfstAnalyzer");

    // 50 common Finnish words for raw FST traversal benchmarking.
    let common_words: Vec<Vec<char>> = [
        "koira", "kissa", "talo", "auto", "vesi", "metsä", "järvi", "puu",
        "kukka", "meri", "joki", "saari", "vuori", "ruoho", "kasvi", "eläin",
        "lapsi", "nainen", "mies", "poika", "tyttö", "äiti", "isä", "perhe",
        "koulu", "kauppa", "pankki", "kirjasto", "sairaala", "hotelli",
        "ravintola", "museo", "teatteri", "musiikki", "taide", "historia",
        "fysiikka", "kemia", "biologia", "talous", "politiikka", "luonto",
        "ympäristö", "matka", "lento", "juna", "bussi", "laiva", "tennis",
        "golf",
    ]
    .iter()
    .map(|w| w.chars().collect::<Vec<char>>())
    .collect();

    use voikko_fi::morphology::Analyzer;

    c.bench_function("fst_traverse_50_words", |b| {
        b.iter(|| {
            for word in &common_words {
                let wlen = word.len();
                std::hint::black_box(analyzer.analyze(word, wlen));
            }
        });
    });
}

/// Suggest corrections for a small set of misspelled Finnish words.
fn bench_suggest_misspelled(c: &mut Criterion) {
    let Some(dict_path) = find_mor_vfst() else {
        eprintln!(
            "[bench_suggest_misspelled] mor.vfst not found — skipping (set VOIKKO_DICT_PATH)"
        );
        c.bench_function("suggest_misspelled (skipped)", |b| b.iter(|| {}));
        return;
    };

    let mor_data = std::fs::read(&dict_path).expect("failed to read mor.vfst");
    let handle =
        voikko_fi::handle::VoikkoHandle::from_bytes(&mor_data, None, "fi").expect("VoikkoHandle");

    let misspelled = ["koirra", "kissaa", "Helinki", "tsjälkeen", "autoo"];

    c.bench_function("suggest_5_misspelled", |b| {
        b.iter(|| {
            for word in &misspelled {
                std::hint::black_box(handle.suggest(word));
            }
        });
    });
}

/// Hyphenate all 246 words from the differential test wordlist.
fn bench_hyphenate_words(c: &mut Criterion) {
    let Some(dict_path) = find_mor_vfst() else {
        eprintln!("[bench_hyphenate_words] mor.vfst not found — skipping (set VOIKKO_DICT_PATH)");
        c.bench_function("hyphenate_words (skipped)", |b| b.iter(|| {}));
        return;
    };

    let mor_data = std::fs::read(&dict_path).expect("failed to read mor.vfst");
    let handle =
        voikko_fi::handle::VoikkoHandle::from_bytes(&mor_data, None, "fi").expect("VoikkoHandle");
    let words = load_wordlist();

    c.bench_function("hyphenate_246_words", |b| {
        b.iter(|| {
            for word in &words {
                std::hint::black_box(handle.hyphenate(word));
            }
        });
    });
}

/// Check grammar on a set of Finnish paragraphs.
fn bench_grammar_check(c: &mut Criterion) {
    let Some(dict_path) = find_mor_vfst() else {
        eprintln!("[bench_grammar_check] mor.vfst not found — skipping (set VOIKKO_DICT_PATH)");
        c.bench_function("grammar_check (skipped)", |b| b.iter(|| {}));
        return;
    };

    let mor_data = std::fs::read(&dict_path).expect("failed to read mor.vfst");
    let handle =
        voikko_fi::handle::VoikkoHandle::from_bytes(&mor_data, None, "fi").expect("VoikkoHandle");

    let paragraphs = [
        "Hyvää huomenta, miten menee tänään?",
        "Koira juoksi nopeasti metsässä ja näki jäniksen.",
        "Suomen kielen opiskelu on mielenkiintoista mutta haastavaa.",
        "Helsinki on Suomen pääkaupunki ja suurin kaupunki.",
        "Tämä  on virheellinen lause jossa on  ylimääräisiä välilyöntejä.",
    ];

    c.bench_function("grammar_5_paragraphs", |b| {
        b.iter(|| {
            for text in &paragraphs {
                std::hint::black_box(handle.grammar_errors(text));
            }
        });
    });
}

/// Tokenize a medium-length Finnish text.
fn bench_tokenize(c: &mut Criterion) {
    let Some(dict_path) = find_mor_vfst() else {
        eprintln!("[bench_tokenize] mor.vfst not found — skipping (set VOIKKO_DICT_PATH)");
        c.bench_function("tokenize (skipped)", |b| b.iter(|| {}));
        return;
    };

    let mor_data = std::fs::read(&dict_path).expect("failed to read mor.vfst");
    let handle =
        voikko_fi::handle::VoikkoHandle::from_bytes(&mor_data, None, "fi").expect("VoikkoHandle");

    let text = "Koira juoksi nopeasti metsässä ja näki jäniksen. \
                Jänis pakeni koloonsa, mutta koira jäi odottamaan. \
                Lopulta molemmat väsyivät ja menivät nukkumaan.";

    c.bench_function("tokenize_3_sentences", |b| {
        b.iter(|| {
            std::hint::black_box(handle.tokens(text));
        });
    });
}

criterion_group!(
    benches,
    bench_spell_words,
    bench_analyze_words,
    bench_fst_traverse,
    bench_suggest_misspelled,
    bench_hyphenate_words,
    bench_grammar_check,
    bench_tokenize,
);
criterion_main!(benches);
