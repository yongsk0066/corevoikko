// Quick test: load mor.vfst and check real Finnish words through the FST engine
use std::fs;

fn main() {
    let dict_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/Users/yongseok/oss/corevoikko/voikko-fi/vvfst/mor.vfst".to_string());

    let data = fs::read(&dict_path).expect("Failed to read VFST file");
    println!("Loaded {}: {} bytes", dict_path, data.len());

    let t = voikko_fst::unweighted::UnweightedTransducer::from_bytes(&data)
        .expect("Failed to parse as unweighted transducer");

    println!(
        "Symbols: {}, first_normal: {}, first_multi: {}, flags: {}",
        t.symbols().symbol_strings.len(),
        t.symbols().first_normal_char,
        t.symbols().first_multi_char,
        t.symbols().flag_feature_count,
    );

    let test_words = [
        "koira", "kissa", "helsinki", "juoksen", "talo",
        "koiran", "kissalle", "juoksee", "talon",
        "koiranruoka",
        "asdfxyz",
    ];

    let mut config = t.new_config(2000);
    for word in &test_words {
        let chars: Vec<char> = word.chars().collect();
        voikko_fst::Transducer::prepare(&t, &mut config, &chars);

        let mut output = String::new();
        let mut outputs = Vec::new();
        while outputs.len() < 20 && voikko_fst::Transducer::next(&t, &mut config, &mut output) {
            outputs.push(output.clone());
        }

        if outputs.is_empty() {
            println!("\n{:15} → (no match)", word);
        } else {
            println!("\n{:15} → {} analyses", word, outputs.len());
            for (i, o) in outputs.iter().enumerate().take(5) {
                println!("  [{}] {}", i, o);
            }
            if outputs.len() > 5 {
                println!("  ... ({} more)", outputs.len() - 5);
            }
        }
    }
}
