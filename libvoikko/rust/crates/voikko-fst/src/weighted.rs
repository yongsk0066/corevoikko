// Weighted transducer loading and traversal.
// Origin: WeightedTransducer.cpp

use crate::config::WeightedConfig;
use crate::flags::{self, FlagCheckResult};
use crate::format::{self, HEADER_SIZE};
use crate::symbols::{self, SymbolTable};
use crate::transition::{WeightedTransition, WEIGHTED_FINAL_SYM, weighted_max_tc};
use crate::{MAX_LOOP_COUNT, Transducer, VfstError};

/// Weighted VFST transducer.
///
/// Loaded from a `&[u8]` slice, this struct provides the `prepare`/`next`
/// traversal interface with weight tracking and binary search optimization.
///
/// Origin: WeightedTransducer.hpp, WeightedTransducer.cpp
pub struct WeightedTransducer {
    /// The transition table (owned, copied from source data for alignment safety).
    transitions: Vec<WeightedTransition>,
    /// Symbol table.
    symbols: SymbolTable,
}

impl std::fmt::Debug for WeightedTransducer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WeightedTransducer")
            .field("transition_count", &self.transitions.len())
            .field("symbol_count", &self.symbols.symbol_strings.len())
            .field("first_normal_char", &self.symbols.first_normal_char)
            .field("first_multi_char", &self.symbols.first_multi_char)
            .finish()
    }
}

/// Result from the weighted `next` call, including the accumulated weight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeightedResult {
    pub weight: i16,
    pub first_not_reached_position: usize,
}

impl WeightedTransducer {
    /// Load a weighted transducer from raw VFST binary data.
    ///
    /// The data is typically loaded from `spl.vfst` or `err.vfst`.
    ///
    /// Origin: WeightedTransducer::WeightedTransducer() -- WeightedTransducer.cpp:130-194
    pub fn from_bytes(data: &[u8]) -> Result<Self, VfstError> {
        let header = format::parse_header(data)?;
        if !header.weighted {
            return Err(VfstError::TypeMismatch {
                expected: true,
                actual: false,
            });
        }
        Self::from_bytes_inner(data)
    }

    fn from_bytes_inner(data: &[u8]) -> Result<Self, VfstError> {
        let (symbols, sym_end) = symbols::parse_symbol_table(data, HEADER_SIZE)?;

        // Align to 16-byte boundary (sizeof(WeightedTransition))
        let partial = sym_end % 16;
        let transition_offset = if partial > 0 {
            sym_end + (16 - partial)
        } else {
            sym_end
        };

        if transition_offset > data.len() {
            return Err(VfstError::TooShort {
                expected: transition_offset,
                actual: data.len(),
            });
        }

        let remaining = &data[transition_offset..];
        let transition_count = remaining.len() / size_of::<WeightedTransition>();

        if transition_count == 0 {
            return Err(VfstError::TooShort {
                expected: transition_offset + size_of::<WeightedTransition>(),
                actual: data.len(),
            });
        }

        // Copy into aligned Vec
        let mut transitions = vec![
            WeightedTransition {
                sym_in: 0,
                sym_out: 0,
                target_state: 0,
                weight: 0,
                more_transitions: 0,
                _reserved: 0,
            };
            transition_count
        ];
        let dst_bytes =
            bytemuck::cast_slice_mut::<WeightedTransition, u8>(&mut transitions);
        dst_bytes
            .copy_from_slice(&remaining[..transition_count * size_of::<WeightedTransition>()]);

        Ok(Self {
            transitions,
            symbols,
        })
    }

    /// Access the symbol table.
    pub fn symbols(&self) -> &SymbolTable {
        &self.symbols
    }

    /// Return the number of flag diacritic features.
    pub fn flag_feature_count(&self) -> u16 {
        self.symbols.flag_feature_count
    }

    /// Create a new configuration suitable for this transducer.
    pub fn new_config(&self, buffer_size: usize) -> WeightedConfig {
        WeightedConfig::new(self.symbols.flag_feature_count, buffer_size)
    }

    /// Yield the next output with its accumulated weight.
    ///
    /// Returns `true` if an output was found. The weight and first-not-reached
    /// position are written to `result`.
    ///
    /// Origin: WeightedTransducer::next() -- WeightedTransducer.cpp:298-406
    pub fn next_weighted(
        &self,
        config: &mut WeightedConfig,
        output: &mut String,
        result: &mut WeightedResult,
    ) -> bool {
        let transitions = &self.transitions;
        let first_normal = self.symbols.first_normal_char as u32;
        let flag_feature_count = self.symbols.flag_feature_count;

        let mut loop_counter: u32 = 0;
        result.first_not_reached_position = config.input_depth;

        'outer: while loop_counter < MAX_LOOP_COUNT {
            let state_idx = config.state_index_stack[config.stack_depth];
            let current_idx = config.current_transition_stack[config.stack_depth];
            let start_transition_index = current_idx - state_idx;
            let max_tc = weighted_max_tc(transitions, state_idx);

            let input_sym: u32 = if config.input_depth == config.input_length {
                0
            } else {
                config.input_symbol_stack[config.input_depth]
            };

            let mut tc = start_transition_index;
            let mut trans_idx = current_idx;

            while tc <= max_tc {
                if tc == 1 && max_tc >= 255 {
                    tc += 1;
                    trans_idx += 1;
                }

                let ct = &transitions[trans_idx as usize];

                if ct.sym_in == WEIGHTED_FINAL_SYM {
                    // Final state
                    if config.input_depth == config.input_length {
                        // Build output
                        output.clear();
                        for i in 0..config.stack_depth {
                            let out_sym = config.output_symbol_stack[i] as usize;
                            let sym_str = &self.symbols.symbol_strings[out_sym];
                            output.push_str(sym_str);
                        }
                        config.current_transition_stack[config.stack_depth] = trans_idx + 1;

                        // Compute total weight
                        let mut total_weight = ct.weight;
                        for i in 0..config.stack_depth {
                            total_weight += transitions
                                [config.current_transition_stack[i] as usize]
                                .weight;
                        }
                        result.weight = total_weight;
                        return true;
                    }
                } else if input_sym == 0 && ct.sym_in >= first_normal {
                    // Only normal transitions left but input is exhausted
                    break;
                } else if (config.input_depth < config.input_length
                    && input_sym == ct.sym_in)
                    || (ct.sym_in < first_normal
                        && self.flag_diacritic_check(config, ct.sym_in as u16))
                {
                    // Push down
                    if config.stack_depth + 2 == config.buffer_size {
                        return false;
                    }

                    config.output_symbol_stack[config.stack_depth] =
                        if ct.sym_out >= first_normal {
                            ct.sym_out
                        } else {
                            0
                        };
                    config.current_transition_stack[config.stack_depth] = trans_idx;
                    config.stack_depth += 1;
                    config.state_index_stack[config.stack_depth] = ct.target_state;
                    config.current_transition_stack[config.stack_depth] =
                        ct.target_state;
                    if ct.sym_in >= first_normal {
                        config.input_depth += 1;
                        if result.first_not_reached_position < config.input_depth {
                            result.first_not_reached_position = config.input_depth;
                        }
                    }
                    loop_counter += 1;
                    continue 'outer;
                } else if ct.sym_in > input_sym {
                    // Transitions are sorted; no more matches possible
                    break;
                } else if tc >= 1 && ct.sym_in >= first_normal && ct.sym_in < input_sym {
                    // Binary search for the matching input symbol
                    let mut min: u32 = 0;
                    let mut max: u32 = max_tc - tc;
                    while min + 1 < max {
                        let middle = (min + max) / 2;
                        if transitions[(trans_idx + middle) as usize].sym_in < input_sym {
                            min = middle;
                        } else {
                            max = middle;
                        }
                    }
                    tc += min;
                    trans_idx += min;
                }

                tc += 1;
                trans_idx += 1;
            }

            // All transitions exhausted
            if config.stack_depth == 0 {
                return false;
            }

            // Pop (backtrack up)
            config.stack_depth -= 1;
            let prev_trans_idx = config.current_transition_stack[config.stack_depth];
            let previous_sym_in = transitions[prev_trans_idx as usize].sym_in;
            if previous_sym_in >= first_normal {
                config.input_depth -= 1;
            } else if flag_feature_count > 0 && previous_sym_in != 0 {
                config.flag_depth -= 1;
            }
            config.current_transition_stack[config.stack_depth] += 1;

            loop_counter += 1;
        }

        false
    }

    /// Backtrack the traversal state to a specific output depth.
    ///
    /// Used by the suggestion generator (`VfstSuggestion`) to rewind the error
    /// model traversal when the acceptor fails, pruning the search space.
    ///
    /// Origin: WeightedTransducer::backtrackToOutputDepth() -- WeightedTransducer.cpp:408-428
    pub fn backtrack_to_output_depth(&self, config: &mut WeightedConfig, depth: usize) {
        let first_normal = self.symbols.first_normal_char as u32;
        let transitions = &self.transitions;

        let mut output_depth: usize = 0;
        let mut stack_index: usize = 0;

        while output_depth < depth + 1 && stack_index < config.stack_depth {
            let trans_idx = config.current_transition_stack[stack_index] as usize;
            let output_symbol = transitions[trans_idx].sym_out;
            if output_symbol >= first_normal {
                output_depth += 1;
            }
            stack_index += 1;
        }

        while stack_index < config.stack_depth {
            config.stack_depth -= 1;
            let prev_trans_idx = config.current_transition_stack[config.stack_depth] as usize;
            let previous_sym_in = transitions[prev_trans_idx].sym_in;
            if previous_sym_in >= first_normal {
                config.input_depth -= 1;
            }
            config.current_transition_stack[config.stack_depth] += 1;
        }
    }

    /// Check flag diacritic and update state if allowed (copy-on-push variant).
    ///
    /// Origin: flagDiacriticCheck() -- WeightedTransducer.cpp:230-286
    fn flag_diacritic_check(&self, config: &mut WeightedConfig, symbol: u16) -> bool {
        let flag_feature_count = self.symbols.flag_feature_count;
        if flag_feature_count == 0 || symbol == 0 {
            return true;
        }

        let ofv = &self.symbols.symbol_to_diacritic[symbol as usize];
        let current_value = config.current_flags()[ofv.feature as usize] as u16;

        let result = flags::check_flag(ofv, current_value);

        match result {
            FlagCheckResult::Reject => false,
            FlagCheckResult::AcceptAndUpdate { feature, value } => {
                // Copy-on-push: copy current flag row forward, then update
                config.push_flags();
                config.current_flags_mut()[feature as usize] = value as u32;
                true
            }
            FlagCheckResult::AcceptNoUpdate { .. } => {
                // Still need to push a copy for consistent depth tracking
                config.push_flags();
                true
            }
        }
    }
}

impl Transducer for WeightedTransducer {
    type Config = WeightedConfig;

    /// Prepare for traversal with the given input characters.
    ///
    /// Returns `false` immediately if any input character is unknown (no
    /// traversal is possible with unknown symbols in weighted transducers).
    ///
    /// Origin: WeightedTransducer::prepare() -- WeightedTransducer.cpp:202-219
    fn prepare(&self, config: &mut Self::Config, input: &[char]) -> bool {
        config.reset();
        for &ch in input {
            match self.symbols.char_to_symbol.get(&ch) {
                Some(&sym_idx) => {
                    config.input_symbol_stack[config.input_length] = sym_idx as u32;
                }
                None => {
                    return false;
                }
            }
            config.input_length += 1;
        }
        true
    }

    /// Yield the next output from the weighted transducer (discarding weight info).
    ///
    /// For weight-aware traversal, use [`next_weighted`](Self::next_weighted).
    fn next(&self, config: &mut Self::Config, output: &mut String) -> bool {
        let mut result = WeightedResult {
            weight: 0,
            first_not_reached_position: 0,
        };
        self.next_weighted(config, output, &mut result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn make_weighted_transition(
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

    /// Build a minimal weighted VFST: "ab" -> "xy" with weights.
    fn build_simple_weighted_vfst() -> Vec<u8> {
        let symbols: &[&str] = &["", "a", "b", "x", "y"];
        let header = build_header(true);
        let sym_table = build_symbol_table(symbols);

        let mut data = Vec::new();
        data.extend_from_slice(&header);
        data.extend_from_slice(&sym_table);

        // Align to 16-byte boundary
        let partial = data.len() % 16;
        if partial > 0 {
            data.extend(std::iter::repeat_n(0u8, 16 - partial));
        }

        // State 0 (index 0): 'a'(1) -> 'x'(3), target=1, weight=10
        let t0 = make_weighted_transition(1, 3, 1, 10, 0);
        data.extend_from_slice(bytemuck::bytes_of(&t0));

        // State 1 (index 1): 'b'(2) -> 'y'(4), target=2, weight=20
        let t1 = make_weighted_transition(2, 4, 2, 20, 0);
        data.extend_from_slice(bytemuck::bytes_of(&t1));

        // State 2 (index 2): final, weight=5
        let t2 = make_weighted_transition(0xFFFFFFFF, 0, 0, 5, 0);
        data.extend_from_slice(bytemuck::bytes_of(&t2));

        data
    }

    #[test]
    fn load_weighted_transducer() {
        let data = build_simple_weighted_vfst();
        let t = WeightedTransducer::from_bytes(&data).unwrap();
        assert_eq!(t.symbols.symbol_strings.len(), 5);
        assert_eq!(t.symbols.first_normal_char, 1);
    }

    #[test]
    fn reject_unweighted_data() {
        let mut data = build_simple_weighted_vfst();
        data[8] = 0x00; // mark as unweighted
        let err = WeightedTransducer::from_bytes(&data).unwrap_err();
        assert!(matches!(err, VfstError::TypeMismatch { .. }));
    }

    #[test]
    fn traverse_weighted_ab_to_xy() {
        let data = build_simple_weighted_vfst();
        let t = WeightedTransducer::from_bytes(&data).unwrap();
        let mut config = t.new_config(100);
        let input: Vec<char> = "ab".chars().collect();

        assert!(t.prepare(&mut config, &input));

        let mut output = String::new();
        let mut result = WeightedResult {
            weight: 0,
            first_not_reached_position: 0,
        };

        assert!(t.next_weighted(&mut config, &mut output, &mut result));
        assert_eq!(output, "xy");
        // Total weight: t0.weight(10) + t1.weight(20) + final.weight(5) = 35
        assert_eq!(result.weight, 35);
    }

    #[test]
    fn traverse_weighted_unknown_input() {
        let data = build_simple_weighted_vfst();
        let t = WeightedTransducer::from_bytes(&data).unwrap();
        let mut config = t.new_config(100);
        let input: Vec<char> = "zz".chars().collect();

        // Weighted transducer rejects unknown input immediately
        assert!(!t.prepare(&mut config, &input));
    }

    #[test]
    fn traverse_weighted_via_trait() {
        let data = build_simple_weighted_vfst();
        let t = WeightedTransducer::from_bytes(&data).unwrap();
        let mut config = t.new_config(100);
        let input: Vec<char> = "ab".chars().collect();

        t.prepare(&mut config, &input);

        let mut output = String::new();
        assert!(t.next(&mut config, &mut output));
        assert_eq!(output, "xy");
        assert!(!t.next(&mut config, &mut output));
    }

    #[test]
    fn weighted_multiple_paths() {
        // Two paths for "a": "x" (weight 10+5=15) and "y" (weight 20+5=25)
        let symbols: &[&str] = &["", "a", "x", "y"];
        let header = build_header(true);
        let sym_table = build_symbol_table(symbols);

        let mut data = Vec::new();
        data.extend_from_slice(&header);
        data.extend_from_slice(&sym_table);

        let partial = data.len() % 16;
        if partial > 0 {
            data.extend(std::iter::repeat_n(0u8, 16 - partial));
        }

        // State 0: 2 transitions (more=1)
        //   'a'(1) -> state 2, output 'x'(2), weight=10
        //   'a'(1) -> state 3, output 'y'(3), weight=20
        data.extend_from_slice(bytemuck::bytes_of(&make_weighted_transition(
            1, 2, 2, 10, 1,
        )));
        data.extend_from_slice(bytemuck::bytes_of(&make_weighted_transition(
            1, 3, 3, 20, 0,
        )));

        // State 2: final, weight=5
        data.extend_from_slice(bytemuck::bytes_of(&make_weighted_transition(
            0xFFFFFFFF,
            0,
            0,
            5,
            0,
        )));

        // State 3: final, weight=5
        data.extend_from_slice(bytemuck::bytes_of(&make_weighted_transition(
            0xFFFFFFFF,
            0,
            0,
            5,
            0,
        )));

        let t = WeightedTransducer::from_bytes(&data).unwrap();
        let mut config = t.new_config(100);
        let input: Vec<char> = "a".chars().collect();

        t.prepare(&mut config, &input);

        let mut output = String::new();
        let mut result = WeightedResult {
            weight: 0,
            first_not_reached_position: 0,
        };

        assert!(t.next_weighted(&mut config, &mut output, &mut result));
        assert_eq!(output, "x");
        assert_eq!(result.weight, 15); // 10 + 5

        assert!(t.next_weighted(&mut config, &mut output, &mut result));
        assert_eq!(output, "y");
        assert_eq!(result.weight, 25); // 20 + 5

        assert!(!t.next_weighted(&mut config, &mut output, &mut result));
    }

    #[test]
    fn weighted_early_break_on_exhausted_input() {
        // Input "a", but state has transitions for both 'a' and 'b'
        // After consuming 'a', the next state should not try to match 'b'
        let symbols: &[&str] = &["", "a", "b"];
        let header = build_header(true);
        let sym_table = build_symbol_table(symbols);

        let mut data = Vec::new();
        data.extend_from_slice(&header);
        data.extend_from_slice(&sym_table);

        let partial = data.len() % 16;
        if partial > 0 {
            data.extend(std::iter::repeat_n(0u8, 16 - partial));
        }

        // State 0: 'a'(1) -> state 1, weight=0
        data.extend_from_slice(bytemuck::bytes_of(&make_weighted_transition(
            1, 1, 1, 0, 0,
        )));

        // State 1: 2 transitions (more=1)
        //   final (0xFFFFFFFF), weight=0
        //   'b'(2) -> state 2, weight=0 (should not be reached)
        data.extend_from_slice(bytemuck::bytes_of(&make_weighted_transition(
            0xFFFFFFFF,
            0,
            0,
            0,
            1,
        )));
        data.extend_from_slice(bytemuck::bytes_of(&make_weighted_transition(
            2, 2, 2, 0, 0,
        )));

        // State 2 (should never reach): final
        data.extend_from_slice(bytemuck::bytes_of(&make_weighted_transition(
            0xFFFFFFFF,
            0,
            0,
            0,
            0,
        )));

        let t = WeightedTransducer::from_bytes(&data).unwrap();
        let mut config = t.new_config(100);
        let input: Vec<char> = "a".chars().collect();

        t.prepare(&mut config, &input);

        let mut output = String::new();
        let mut result = WeightedResult {
            weight: 0,
            first_not_reached_position: 0,
        };

        assert!(t.next_weighted(&mut config, &mut output, &mut result));
        assert_eq!(output, "a");
        assert!(!t.next_weighted(&mut config, &mut output, &mut result));
    }
}
