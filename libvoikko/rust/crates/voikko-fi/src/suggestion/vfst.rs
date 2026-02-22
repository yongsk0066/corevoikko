// VFST-based suggestion generator using two weighted transducers in tandem:
// an error model and an acceptor.
//
// The error model maps misspelled input to possible correction candidates with
// error weights. The acceptor validates that each candidate is a real word.
// Combining weights from both transducers yields a relevance-ranked suggestion
// list.
//
// Origin: spellchecker/VfstSuggestion.cpp, VfstSuggestion.hpp

use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::cmp::Reverse;

use voikko_fst::Transducer;
use voikko_fst::weighted::{WeightedResult, WeightedTransducer};

use super::status::SuggestionStatus;

/// Buffer size for weighted transducer traversal configurations.
///
/// Origin: VfstSuggestion.cpp:40 -- `static const int BUFFER_SIZE = 2000;`
const BUFFER_SIZE: usize = 2000;

/// Suggestion generator that uses two weighted VFST transducers (acceptor and
/// error model) to produce correction candidates.
///
/// The error model transducer (`err.vfst`) enumerates plausible edits of the
/// misspelled word. Each candidate is then validated by the acceptor transducer
/// (typically `spl.vfst` or `mor.vfst`). Accepted candidates are scored by the
/// sum of their error-model weight and acceptor weight.
///
/// In the C++ code the acceptor is borrowed from the `VfstSpeller` and the
/// error model is loaded from `err.vfst` in the dictionary directory.
///
/// Origin: VfstSuggestion.hpp:44-57
pub struct VfstSuggestion {
    /// Error model transducer loaded from `err.vfst`.
    error_model: WeightedTransducer,
    /// Acceptor transducer (typically the same `spl.vfst` used by the speller).
    acceptor: WeightedTransducer,
}

impl VfstSuggestion {
    /// Create a new VFST suggestion generator.
    ///
    /// - `error_model`: the error model transducer (`err.vfst`), which maps
    ///   misspelled input to possible corrections with error weights.
    /// - `acceptor`: the acceptor transducer (`spl.vfst`), which validates
    ///   that a candidate word is in the language.
    ///
    /// Origin: VfstSuggestion.cpp:52-59
    pub fn new(error_model: WeightedTransducer, acceptor: WeightedTransducer) -> Self {
        Self {
            error_model,
            acceptor,
        }
    }

    /// Generate suggestions for the misspelled word tracked by `status`.
    ///
    /// Algorithm:
    /// 1. Prepare the error model with the misspelled word.
    /// 2. Iterate over error model outputs (candidate corrections).
    /// 3. For each candidate, prepare the acceptor and check if it accepts.
    /// 4. If accepted, record the candidate with combined weight (error model +
    ///    acceptor), keeping the minimum weight per unique string.
    /// 5. If rejected, backtrack the error model to the output depth where the
    ///    acceptor failed, pruning the search tree.
    /// 6. After exhausting the error model, sort candidates by weight and add
    ///    them to `status` in order.
    ///
    /// Unlike the other generators, VfstSuggestion does NOT use the `Speller`
    /// trait -- it validates candidates directly via the acceptor transducer.
    /// This is why it has its own `generate` method rather than implementing
    /// `SuggestionGenerator`.
    ///
    /// Origin: VfstSuggestion.cpp:62-101
    pub fn generate(&self, status: &mut SuggestionStatus<'_>) {
        // Not actually used for cost tracking in this generator, but matches
        // the C++ behavior where setMaxCost(100) is called.
        // Origin: VfstSuggestion.cpp:63
        status.set_max_cost(100);

        let word: Vec<char> = status.word().to_vec();
        let wlen = status.word_len();

        let mut error_model_conf = self.error_model.new_config(BUFFER_SIZE);
        let mut acceptor_conf = self.acceptor.new_config(BUFFER_SIZE);

        // Map from suggestion string to its minimum combined weight.
        // Origin: VfstSuggestion.cpp:67
        let mut suggestion_weights: HashMap<String, i32> = HashMap::new();

        let mut error_model_output = String::new();
        let mut error_model_result = WeightedResult {
            weight: 0,
            first_not_reached_position: 0,
        };

        let mut acceptor_output = String::new();
        let mut acceptor_result = WeightedResult {
            weight: 0,
            first_not_reached_position: 0,
        };

        // Origin: VfstSuggestion.cpp:68
        if self.error_model.prepare(&mut error_model_conf, &word[..wlen]) {
            // Origin: VfstSuggestion.cpp:69
            while !status.should_abort()
                && self.error_model.next_weighted(
                    &mut error_model_conf,
                    &mut error_model_output,
                    &mut error_model_result,
                )
            {
                // Convert error model output to chars for the acceptor.
                let candidate_chars: Vec<char> = error_model_output.chars().collect();

                // Origin: VfstSuggestion.cpp:70
                if self.acceptor.prepare(&mut acceptor_conf, &candidate_chars) {
                    // Origin: VfstSuggestion.cpp:72
                    if self.acceptor.next_weighted(
                        &mut acceptor_conf,
                        &mut acceptor_output,
                        &mut acceptor_result,
                    ) {
                        // Accepted: combine weights.
                        // Origin: VfstSuggestion.cpp:73-80
                        // Use i32 for combined weight to avoid i16 overflow
                        let weight = acceptor_result.weight as i32 + error_model_result.weight as i32;
                        suggestion_weights
                            .entry(error_model_output.clone())
                            .and_modify(|existing| *existing = (*existing).min(weight))
                            .or_insert(weight);
                    } else {
                        // Rejected: prune the error model search tree.
                        // Origin: VfstSuggestion.cpp:83
                        self.error_model.backtrack_to_output_depth(
                            &mut error_model_conf,
                            acceptor_result.first_not_reached_position,
                        );
                    }
                }
            }
        }

        // Sort suggestions by weight (ascending -- lower is better) and add
        // them to `status`.
        //
        // The C++ uses a priority_queue (max-heap with inverted comparison),
        // which pops elements in ascending weight order. We use a min-heap
        // via `Reverse`.
        //
        // Origin: VfstSuggestion.cpp:89-101
        let mut heap: BinaryHeap<Reverse<(i32, String)>> = BinaryHeap::new();
        for (suggestion, weight) in suggestion_weights {
            heap.push(Reverse((weight, suggestion)));
        }

        while let Some(Reverse((weight, suggestion))) = heap.pop() {
            // The C++ code passes the weight directly as the priority.
            // Our SuggestionStatus::add_suggestion takes an i32 priority.
            // Origin: VfstSuggestion.cpp:100
            status.add_suggestion(suggestion, weight);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voikko_fst::transition::WeightedTransition;
    use voikko_fst::weighted::WeightedTransducer;

    // -----------------------------------------------------------------------
    // Test VFST builder helpers
    // -----------------------------------------------------------------------

    fn build_header(weighted: bool) -> Vec<u8> {
        let mut buf = vec![0u8; 16];
        buf[..4].copy_from_slice(&0x0001_3A6Eu32.to_le_bytes());
        buf[4..8].copy_from_slice(&0x0003_51FAu32.to_le_bytes());
        buf[8] = if weighted { 1 } else { 0 };
        buf
    }

    fn build_symbol_table(symbols: &[&str]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&(symbols.len() as u16).to_le_bytes());
        for s in symbols {
            buf.extend_from_slice(s.as_bytes());
            buf.push(0);
        }
        buf
    }

    fn make_transition(
        sym_in: u32,
        sym_out: u32,
        target: u32,
        weight: i16,
        more: u8,
    ) -> WeightedTransition {
        WeightedTransition {
            sym_in,
            sym_out,
            target_state: target,
            weight,
            more_transitions: more,
            _reserved: 0,
        }
    }

    fn build_vfst(symbols: &[&str], transitions: &[WeightedTransition]) -> Vec<u8> {
        let header = build_header(true);
        let sym_table = build_symbol_table(symbols);

        let mut data = Vec::new();
        data.extend_from_slice(&header);
        data.extend_from_slice(&sym_table);

        // Align to 16-byte boundary (sizeof(WeightedTransition))
        let partial = data.len() % 16;
        if partial > 0 {
            data.extend(std::iter::repeat_n(0u8, 16 - partial));
        }

        for t in transitions {
            data.extend_from_slice(bytemuck::bytes_of(t));
        }
        data
    }

    // -----------------------------------------------------------------------
    // Constructor tests
    // -----------------------------------------------------------------------

    #[test]
    fn new_creates_vfst_suggestion() {
        // Minimal transducers (identity: "a" -> "a")
        let symbols: &[&str] = &["", "a"];
        let transitions = vec![
            make_transition(1, 1, 1, 0, 0),
            make_transition(0xFFFFFFFF, 0, 0, 0, 0),
        ];
        let data = build_vfst(symbols, &transitions);

        let error_model = WeightedTransducer::from_bytes(&data).unwrap();
        let acceptor = WeightedTransducer::from_bytes(&data).unwrap();

        let _gen = VfstSuggestion::new(error_model, acceptor);
    }

    // -----------------------------------------------------------------------
    // Generate tests with hand-crafted VFST data
    // -----------------------------------------------------------------------

    /// Build an error model that maps "x" -> "a" with weight 5.
    /// Build an acceptor that accepts "a" with weight 3.
    /// Expected: suggestion "a" with combined weight 8.
    #[test]
    fn generate_single_suggestion() {
        // Error model: input "x", output "a", weight 5
        // Symbols: ["", "x", "a"]  (x=1, a=2)
        let err_symbols: &[&str] = &["", "x", "a"];
        let err_transitions = vec![
            // State 0: 'x'(1) -> 'a'(2), target=1, weight=5
            make_transition(1, 2, 1, 5, 0),
            // State 1: final, weight=0
            make_transition(0xFFFFFFFF, 0, 0, 0, 0),
        ];
        let err_data = build_vfst(err_symbols, &err_transitions);
        let error_model = WeightedTransducer::from_bytes(&err_data).unwrap();

        // Acceptor: accepts "a", weight 3
        // Symbols: ["", "a"]  (a=1)
        let acc_symbols: &[&str] = &["", "a"];
        let acc_transitions = vec![
            // State 0: 'a'(1) -> 'a'(1), target=1, weight=3
            make_transition(1, 1, 1, 3, 0),
            // State 1: final, weight=0
            make_transition(0xFFFFFFFF, 0, 0, 0, 0),
        ];
        let acc_data = build_vfst(acc_symbols, &acc_transitions);
        let acceptor = WeightedTransducer::from_bytes(&acc_data).unwrap();

        let sg =VfstSuggestion::new(error_model, acceptor);

        let word: Vec<char> = "x".chars().collect();
        let mut status = SuggestionStatus::new(&word, 10);

        sg.generate(&mut status);

        assert_eq!(status.suggestion_count(), 1);
        assert_eq!(status.suggestions()[0].word, "a");
        // Weight 5 (error model) + 3 (acceptor) = 8
        // SuggestionStatus multiplies by (count + 5), so first suggestion:
        // priority = 8 * (0 + 5) = 40
        assert_eq!(status.suggestions()[0].priority, 40);
    }

    /// Build an error model that maps "x" -> "a" (weight 5) and "x" -> "b" (weight 10).
    /// Build an acceptor that accepts both "a" (weight 3) and "b" (weight 1).
    /// Expected: "a" (weight 8), "b" (weight 11), sorted by weight.
    #[test]
    fn generate_multiple_suggestions_sorted_by_weight() {
        // Error model: input "x", output "a" or "b"
        // Symbols: ["", "x", "a", "b"]  (x=1, a=2, b=3)
        let err_symbols: &[&str] = &["", "x", "a", "b"];
        let err_transitions = vec![
            // State 0: two transitions (more=1)
            // 'x'(1) -> 'a'(2), target=2, weight=5
            make_transition(1, 2, 2, 5, 1),
            // 'x'(1) -> 'b'(3), target=3, weight=10
            make_transition(1, 3, 3, 10, 0),
            // State 2: final, weight=0
            make_transition(0xFFFFFFFF, 0, 0, 0, 0),
            // State 3: final, weight=0
            make_transition(0xFFFFFFFF, 0, 0, 0, 0),
        ];
        let err_data = build_vfst(err_symbols, &err_transitions);
        let error_model = WeightedTransducer::from_bytes(&err_data).unwrap();

        // Acceptor: accepts "a" (weight 3) and "b" (weight 1)
        // Symbols: ["", "a", "b"]  (a=1, b=2)
        let acc_symbols: &[&str] = &["", "a", "b"];
        let acc_transitions = vec![
            // State 0: two transitions (more=1)
            // 'a'(1) -> 'a'(1), target=2, weight=3
            make_transition(1, 1, 2, 3, 1),
            // 'b'(2) -> 'b'(2), target=3, weight=1
            make_transition(2, 2, 3, 1, 0),
            // State 2: final, weight=0
            make_transition(0xFFFFFFFF, 0, 0, 0, 0),
            // State 3: final, weight=0
            make_transition(0xFFFFFFFF, 0, 0, 0, 0),
        ];
        let acc_data = build_vfst(acc_symbols, &acc_transitions);
        let acceptor = WeightedTransducer::from_bytes(&acc_data).unwrap();

        let sg =VfstSuggestion::new(error_model, acceptor);

        let word: Vec<char> = "x".chars().collect();
        let mut status = SuggestionStatus::new(&word, 10);

        sg.generate(&mut status);

        assert_eq!(status.suggestion_count(), 2);

        // Suggestions are added in weight order: "a" (8) first, "b" (11) second
        // After SuggestionStatus priority scaling:
        //   "a": 8 * (0 + 5) = 40
        //   "b": 11 * (1 + 5) = 66
        status.sort_suggestions();
        assert_eq!(status.suggestions()[0].word, "a");
        assert_eq!(status.suggestions()[1].word, "b");
    }

    /// Error model produces a candidate that the acceptor rejects.
    /// Expected: no suggestions.
    #[test]
    fn generate_no_suggestions_when_acceptor_rejects() {
        // Error model: "x" -> "z" with weight 5
        let err_symbols: &[&str] = &["", "x", "z"];
        let err_transitions = vec![
            make_transition(1, 2, 1, 5, 0),
            make_transition(0xFFFFFFFF, 0, 0, 0, 0),
        ];
        let err_data = build_vfst(err_symbols, &err_transitions);
        let error_model = WeightedTransducer::from_bytes(&err_data).unwrap();

        // Acceptor: only accepts "a", does NOT know "z"
        let acc_symbols: &[&str] = &["", "a"];
        let acc_transitions = vec![
            make_transition(1, 1, 1, 0, 0),
            make_transition(0xFFFFFFFF, 0, 0, 0, 0),
        ];
        let acc_data = build_vfst(acc_symbols, &acc_transitions);
        let acceptor = WeightedTransducer::from_bytes(&acc_data).unwrap();

        let sg =VfstSuggestion::new(error_model, acceptor);

        let word: Vec<char> = "x".chars().collect();
        let mut status = SuggestionStatus::new(&word, 10);

        sg.generate(&mut status);

        assert_eq!(status.suggestion_count(), 0);
    }

    /// Error model cannot prepare for a word with unknown symbols.
    /// Expected: no suggestions, no panic.
    #[test]
    fn generate_unknown_input_symbol_no_panic() {
        // Error model only knows "a"
        let err_symbols: &[&str] = &["", "a"];
        let err_transitions = vec![
            make_transition(1, 1, 1, 0, 0),
            make_transition(0xFFFFFFFF, 0, 0, 0, 0),
        ];
        let err_data = build_vfst(err_symbols, &err_transitions);
        let error_model = WeightedTransducer::from_bytes(&err_data).unwrap();

        let acc_data = err_data.clone();
        let acceptor = WeightedTransducer::from_bytes(&acc_data).unwrap();

        let sg =VfstSuggestion::new(error_model, acceptor);

        // Input "z" is unknown to the error model
        let word: Vec<char> = "z".chars().collect();
        let mut status = SuggestionStatus::new(&word, 10);

        sg.generate(&mut status);

        assert_eq!(status.suggestion_count(), 0);
    }

    /// Verify that the generator respects the abort condition.
    #[test]
    fn generate_respects_abort() {
        // Error model: "x" -> "a" with weight 5
        let err_symbols: &[&str] = &["", "x", "a"];
        let err_transitions = vec![
            make_transition(1, 2, 1, 5, 0),
            make_transition(0xFFFFFFFF, 0, 0, 0, 0),
        ];
        let err_data = build_vfst(err_symbols, &err_transitions);
        let error_model = WeightedTransducer::from_bytes(&err_data).unwrap();

        // Acceptor: accepts "a"
        let acc_symbols: &[&str] = &["", "a"];
        let acc_transitions = vec![
            make_transition(1, 1, 1, 0, 0),
            make_transition(0xFFFFFFFF, 0, 0, 0, 0),
        ];
        let acc_data = build_vfst(acc_symbols, &acc_transitions);
        let acceptor = WeightedTransducer::from_bytes(&acc_data).unwrap();

        let sg =VfstSuggestion::new(error_model, acceptor);

        // max_suggestions=0 means already full -> should abort immediately
        let word: Vec<char> = "x".chars().collect();
        let mut status = SuggestionStatus::new(&word, 0);

        sg.generate(&mut status);

        // Should not panic, and no suggestions added (capacity 0)
        assert_eq!(status.suggestion_count(), 0);
    }

    /// When the error model produces the same suggestion twice via different
    /// paths, only the minimum weight should be kept.
    #[test]
    fn generate_deduplicates_by_minimum_weight() {
        // Error model: two paths for "x" -> "a", weights 5 and 15
        // Symbols: ["", "x", "a"]  (x=1, a=2)
        let err_symbols: &[&str] = &["", "x", "a"];
        let err_transitions = vec![
            // State 0: two transitions for 'x' (more=1)
            // Path 1: 'x'(1) -> 'a'(2), target=2, weight=5
            make_transition(1, 2, 2, 5, 1),
            // Path 2: 'x'(1) -> 'a'(2), target=3, weight=15
            make_transition(1, 2, 3, 15, 0),
            // State 2: final, weight=0
            make_transition(0xFFFFFFFF, 0, 0, 0, 0),
            // State 3: final, weight=0
            make_transition(0xFFFFFFFF, 0, 0, 0, 0),
        ];
        let err_data = build_vfst(err_symbols, &err_transitions);
        let error_model = WeightedTransducer::from_bytes(&err_data).unwrap();

        // Acceptor: accepts "a", weight 3
        let acc_symbols: &[&str] = &["", "a"];
        let acc_transitions = vec![
            make_transition(1, 1, 1, 3, 0),
            make_transition(0xFFFFFFFF, 0, 0, 0, 0),
        ];
        let acc_data = build_vfst(acc_symbols, &acc_transitions);
        let acceptor = WeightedTransducer::from_bytes(&acc_data).unwrap();

        let sg =VfstSuggestion::new(error_model, acceptor);

        let word: Vec<char> = "x".chars().collect();
        let mut status = SuggestionStatus::new(&word, 10);

        sg.generate(&mut status);

        // Only one unique suggestion "a", with minimum weight = 5 + 3 = 8
        assert_eq!(status.suggestion_count(), 1);
        assert_eq!(status.suggestions()[0].word, "a");
        // priority = 8 * (0 + 5) = 40
        assert_eq!(status.suggestions()[0].priority, 40);
    }
}
