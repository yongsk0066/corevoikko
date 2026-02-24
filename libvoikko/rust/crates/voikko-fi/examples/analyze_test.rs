// Full pipeline test: load mor.vfst → FinnishVfstAnalyzer → tag parsing → Analysis
use std::fs;
use voikko_core::analysis::{
    ATTR_BASEFORM, ATTR_CLASS, ATTR_NUMBER, ATTR_SIJAMUOTO, ATTR_STRUCTURE,
};

fn main() {
    let dict_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/Users/yongseok/oss/corevoikko/voikko-fi/vvfst/mor.vfst".to_string());

    let data = fs::read(&dict_path).expect("Failed to read mor.vfst");
    println!("Loaded mor.vfst: {} bytes\n", data.len());

    let mut analyzer = voikko_fi::morphology::FinnishVfstAnalyzer::from_bytes(&data)
        .expect("Failed to create analyzer");

    let test_words = [
        "koira",
        "Koira",
        "KOIRA",
        "kissa",
        "Helsinki",
        "juoksen",
        "talo",
        "koiran",
        "kissalle",
        "juoksee",
        "talon",
        "koiranruoka",
        "hyvä",
        "suurempi",
        "asdfxyz",
    ];

    for word in &test_words {
        let chars: Vec<char> = word.chars().collect();
        let analyses = analyzer.analyze_full(&chars, chars.len(), true);

        if analyses.is_empty() {
            println!("{:15} → (no analysis)", word);
        } else {
            println!("{:15} → {} analyses", word, analyses.len());
            for (i, a) in analyses.iter().enumerate().take(3) {
                let class = a.get(ATTR_CLASS).unwrap_or("-");
                let baseform = a.get(ATTR_BASEFORM).unwrap_or("-");
                let sija = a.get(ATTR_SIJAMUOTO).unwrap_or("-");
                let number = a.get(ATTR_NUMBER).unwrap_or("-");
                let structure = a.get(ATTR_STRUCTURE).unwrap_or("-");
                println!(
                    "  [{}] class={}, baseform={}, case={}, num={}, struct={}",
                    i, class, baseform, sija, number, structure
                );
            }
            if analyses.len() > 3 {
                println!("  ... ({} more)", analyses.len() - 3);
            }
        }
    }
}
