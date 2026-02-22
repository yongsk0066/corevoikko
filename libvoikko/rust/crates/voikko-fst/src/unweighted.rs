// Unweighted transducer loading and traversal.
// Origin: UnweightedTransducer.cpp

use crate::config::UnweightedConfig;
use crate::flags::{self, FlagCheckResult};
use crate::format::{self, HEADER_SIZE};
use crate::symbols::{self, SymbolTable};
use crate::transition::{Transition, UNWEIGHTED_FINAL_SYM, unweighted_max_tc};
use crate::{MAX_LOOP_COUNT, Transducer, VfstError};

/// Unweighted VFST transducer.
///
/// Loaded from a `&[u8]` slice (the raw binary VFST data), this struct
/// provides the `prepare`/`next` traversal interface.
///
/// Origin: UnweightedTransducer.hpp, UnweightedTransducer.cpp
pub struct UnweightedTransducer {
    /// The transition table as a zero-copy slice of the backing data.
    transitions: Vec<Transition>,
    /// Symbol table.
    symbols: SymbolTable,
    /// Sentinel symbol index for unknown input characters.
    unknown_symbol_ordinal: u16,
}

impl std::fmt::Debug for UnweightedTransducer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UnweightedTransducer")
            .field("transition_count", &self.transitions.len())
            .field("symbol_count", &self.symbols.symbol_strings.len())
            .field("first_normal_char", &self.symbols.first_normal_char)
            .field("first_multi_char", &self.symbols.first_multi_char)
            .finish()
    }
}

impl UnweightedTransducer {
    /// Load an unweighted transducer from raw VFST binary data.
    ///
    /// The data is typically loaded from a `mor.vfst` or `autocorr.vfst` file.
    /// The transition table is copied into an owned `Vec<Transition>` for
    /// alignment safety (the source `&[u8]` may not be 8-byte aligned).
    ///
    /// Origin: UnweightedTransducer::UnweightedTransducer() -- UnweightedTransducer.cpp:125-189
    pub fn from_bytes(data: &[u8]) -> Result<Self, VfstError> {
        let header = format::parse_header(data)?;
        if header.weighted {
            return Err(VfstError::TypeMismatch {
                expected: false,
                actual: true,
            });
        }
        Self::from_bytes_inner(data)
    }

    fn from_bytes_inner(data: &[u8]) -> Result<Self, VfstError> {
        let (symbols, sym_end) = symbols::parse_symbol_table(data, HEADER_SIZE)?;

        // Align to 8-byte boundary (sizeof(Transition))
        let partial = sym_end % 8;
        let transition_offset = if partial > 0 { sym_end + (8 - partial) } else { sym_end };

        if transition_offset > data.len() {
            return Err(VfstError::TooShort {
                expected: transition_offset,
                actual: data.len(),
            });
        }

        let remaining = &data[transition_offset..];
        let transition_count = remaining.len() / size_of::<Transition>();

        if transition_count == 0 {
            return Err(VfstError::TooShort {
                expected: transition_offset + size_of::<Transition>(),
                actual: data.len(),
            });
        }

        // Copy transition data into an aligned Vec<Transition> for safety.
        // The source slice may not be properly aligned for zero-copy cast.
        let mut transitions = vec![Transition { sym_in: 0, sym_out: 0, trans_info: 0 }; transition_count];
        let dst_bytes = bytemuck::cast_slice_mut::<Transition, u8>(&mut transitions);
        dst_bytes.copy_from_slice(&remaining[..transition_count * size_of::<Transition>()]);

        let unknown_symbol_ordinal = symbols.symbol_strings.len() as u16;

        Ok(Self {
            transitions,
            symbols,
            unknown_symbol_ordinal,
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
    pub fn new_config(&self, buffer_size: usize) -> UnweightedConfig {
        UnweightedConfig::new(self.symbols.flag_feature_count, buffer_size)
    }

    /// Yield the next prefix match from the transducer.
    ///
    /// Like `next()`, but also returns the length of the input prefix that was
    /// consumed. Used by autocorrect (VfstAutocorrectCheck).
    ///
    /// Origin: UnweightedTransducer::nextPrefix() -- UnweightedTransducer.cpp:289-370
    pub fn next_prefix(
        &self,
        config: &mut UnweightedConfig,
        output: &mut String,
        prefix_length: &mut usize,
    ) -> bool {
        self.next_inner(config, output, Some(prefix_length))
    }

    /// Core traversal: iterative DFS with backtracking.
    ///
    /// If `prefix_length` is `Some`, matches any prefix of the input (not just
    /// complete input). Otherwise, only matches when all input is consumed.
    ///
    /// The C++ code uses `goto nextInMainLoop` to skip the backtracking check
    /// after a push. In Rust, this is implemented with `continue 'outer`.
    ///
    /// Origin: UnweightedTransducer::nextPrefix() -- UnweightedTransducer.cpp:289-370
    fn next_inner(
        &self,
        config: &mut UnweightedConfig,
        output: &mut String,
        mut prefix_length: Option<&mut usize>,
    ) -> bool {
        let transitions = &self.transitions;
        let first_normal = self.symbols.first_normal_char;
        let flag_feature_count = self.symbols.flag_feature_count;

        let mut loop_counter: u32 = 0;

        'outer: while loop_counter < MAX_LOOP_COUNT {
            let state_idx = config.state_index_stack[config.stack_depth];
            let current_idx = config.current_transition_stack[config.stack_depth];
            let start_transition_index = current_idx - state_idx;
            let max_tc = unweighted_max_tc(transitions, state_idx);

            let mut tc = start_transition_index;
            let mut trans_idx = current_idx;

            while tc <= max_tc {
                if tc == 1 && max_tc >= 255 {
                    // Skip overflow cell
                    tc += 1;
                    trans_idx += 1;
                }

                let current_transition = &transitions[trans_idx as usize];

                if current_transition.sym_in == UNWEIGHTED_FINAL_SYM {
                    // Final state
                    if config.input_depth == config.input_length || prefix_length.is_some() {
                        // Build output string
                        output.clear();
                        for i in 0..config.stack_depth {
                            let out_sym = config.output_symbol_stack[i] as usize;
                            let sym_str = &self.symbols.symbol_strings[out_sym];
                            output.push_str(sym_str);
                        }
                        config.current_transition_stack[config.stack_depth] = trans_idx + 1;
                        if let Some(ref mut pl) = prefix_length {
                            **pl = config.input_depth;
                        }
                        return true;
                    }
                } else if (config.input_depth < config.input_length
                    && config.input_symbol_stack[config.input_depth] == current_transition.sym_in)
                    || (current_transition.sym_in < first_normal
                        && self.flag_diacritic_check(config, current_transition.sym_in))
                {
                    // Push down
                    if config.stack_depth + 2 == config.buffer_size {
                        // Max stack depth reached
                        return false;
                    }

                    config.output_symbol_stack[config.stack_depth] = if current_transition.sym_out
                        >= first_normal
                    {
                        current_transition.sym_out
                    } else {
                        0
                    };
                    config.current_transition_stack[config.stack_depth] = trans_idx;
                    config.stack_depth += 1;
                    config.state_index_stack[config.stack_depth] =
                        current_transition.target_state();
                    config.current_transition_stack[config.stack_depth] =
                        current_transition.target_state();
                    if current_transition.sym_in >= first_normal {
                        config.input_depth += 1;
                    }
                    loop_counter += 1;
                    continue 'outer;
                }

                tc += 1;
                trans_idx += 1;
            }

            // All transitions exhausted at this depth
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
                let undo_feature =
                    config.flag_undo_feature[config.flag_depth] as usize;
                let undo_value = config.flag_undo_value[config.flag_depth];
                config.current_flag_values[undo_feature] = undo_value;
            }
            config.current_transition_stack[config.stack_depth] += 1;

            loop_counter += 1;
        }

        false
    }

    /// Check flag diacritic and update state if allowed.
    ///
    /// Returns `true` if the transition is allowed.
    ///
    /// Origin: flagDiacriticCheck() -- UnweightedTransducer.cpp:228-283
    fn flag_diacritic_check(&self, config: &mut UnweightedConfig, symbol: u16) -> bool {
        let flag_feature_count = self.symbols.flag_feature_count;
        if flag_feature_count == 0 || symbol == 0 {
            return true;
        }

        let ofv = &self.symbols.symbol_to_diacritic[symbol as usize];
        let current_value = config.current_flag_values[ofv.feature as usize];

        let result = flags::check_flag(ofv, current_value);

        match result {
            FlagCheckResult::Reject => false,
            FlagCheckResult::AcceptAndUpdate { feature, value } => {
                // Save old value for undo
                config.flag_undo_feature[config.flag_depth] = feature;
                config.flag_undo_value[config.flag_depth] =
                    config.current_flag_values[feature as usize];
                config.current_flag_values[feature as usize] = value;
                config.flag_depth += 1;
                true
            }
            FlagCheckResult::AcceptNoUpdate { feature } => {
                // Save for undo (no actual change, but the undo stack must still record)
                config.flag_undo_feature[config.flag_depth] = feature;
                config.flag_undo_value[config.flag_depth] =
                    config.current_flag_values[feature as usize];
                config.flag_depth += 1;
                true
            }
        }
    }
}

impl Transducer for UnweightedTransducer {
    type Config = UnweightedConfig;

    /// Prepare the configuration for traversing with the given input characters.
    ///
    /// Returns `true` if all input characters are known symbols.
    /// Unknown characters are mapped to `unknown_symbol_ordinal` (sentinel);
    /// traversal can still proceed but will not match those characters.
    ///
    /// Origin: UnweightedTransducer::prepare() -- UnweightedTransducer.cpp:197-217
    fn prepare(&self, config: &mut Self::Config, input: &[char]) -> bool {
        config.reset();
        let mut all_known = true;
        for &ch in input {
            match self.symbols.char_to_symbol.get(&ch) {
                Some(&sym_idx) => {
                    config.input_symbol_stack[config.input_length] = sym_idx;
                }
                None => {
                    config.input_symbol_stack[config.input_length] =
                        self.unknown_symbol_ordinal;
                    all_known = false;
                }
            }
            config.input_length += 1;
        }
        all_known
    }

    /// Yield the next complete output from the transducer.
    ///
    /// Only matches when the entire input has been consumed. For prefix matching,
    /// use [`next_prefix`](Self::next_prefix).
    ///
    /// Origin: UnweightedTransducer::next() -- UnweightedTransducer.cpp:285-287
    fn next(&self, config: &mut Self::Config, output: &mut String) -> bool {
        self.next_inner(config, output, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal VFST binary for testing.
    ///
    /// This creates a transducer that accepts "ab" and outputs "xy".
    ///
    /// Structure:
    /// - State 0: transition on 'a' -> State 1 (output 'x')
    /// - State 1: transition on 'b' -> State 2 (output 'y')
    /// - State 2: final transition (sym_in=0xFFFF)
    fn build_simple_vfst() -> Vec<u8> {
        // Symbols: [epsilon, a, b, x, y]
        let symbols: &[&str] = &["", "a", "b", "x", "y"];
        let header = build_header(false);
        let sym_table = build_symbol_table(symbols);

        let mut data = Vec::new();
        data.extend_from_slice(&header);
        data.extend_from_slice(&sym_table);

        // Align to 8-byte boundary
        let partial = data.len() % 8;
        if partial > 0 {
            let padding = 8 - partial;
            data.extend(std::iter::repeat_n(0u8, padding));
        }

        // State 0 (index 0): one transition: 'a'(1) -> 'x'(3), target=1, more=0
        // Since we have single transitions per state, more_transitions = 0
        let t0 = make_transition(1, 3, 1, 0); // sym_in=a(1), sym_out=x(3), target=1, more=0
        data.extend_from_slice(bytemuck::bytes_of(&t0));

        // State 1 (index 1): one transition: 'b'(2) -> 'y'(4), target=2, more=0
        let t1 = make_transition(2, 4, 2, 0);
        data.extend_from_slice(bytemuck::bytes_of(&t1));

        // State 2 (index 2): one transition: final (0xFFFF)
        let t2 = make_transition(0xFFFF, 0, 0, 0);
        data.extend_from_slice(bytemuck::bytes_of(&t2));

        data
    }

    /// Build a transducer that accepts "a" and outputs "a" with no flags.
    /// Also has an epsilon transition to test epsilon handling.
    ///
    /// Structure:
    /// - State 0: epsilon transition -> State 1 (output epsilon)
    ///            more_transitions = 1 (2 transitions total)
    /// - State 1: transition on 'a' -> State 2 (output 'a')
    /// - State 2: final transition
    fn build_epsilon_vfst() -> Vec<u8> {
        let symbols: &[&str] = &["", "a"];
        let header = build_header(false);
        let sym_table = build_symbol_table(symbols);

        let mut data = Vec::new();
        data.extend_from_slice(&header);
        data.extend_from_slice(&sym_table);

        let partial = data.len() % 8;
        if partial > 0 {
            data.extend(std::iter::repeat_n(0u8, 8 - partial));
        }

        // State 0 (index 0): 2 transitions (more=1)
        //   - epsilon(0) -> state 2, output 0
        //   - 'a'(1) -> state 3, output 'a'(1)
        let t0_0 = make_transition(0, 0, 2, 1); // epsilon -> state 2
        data.extend_from_slice(bytemuck::bytes_of(&t0_0));
        let t0_1 = make_transition(1, 1, 3, 0); // 'a' -> state 3
        data.extend_from_slice(bytemuck::bytes_of(&t0_1));

        // State 2 (index 2): transition on 'a' -> state 3
        let t2 = make_transition(1, 1, 3, 0);
        data.extend_from_slice(bytemuck::bytes_of(&t2));

        // State 3 (index 3): final
        let t3 = make_transition(0xFFFF, 0, 0, 0);
        data.extend_from_slice(bytemuck::bytes_of(&t3));

        data
    }

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

    fn make_transition(sym_in: u16, sym_out: u16, target: u32, more: u8) -> Transition {
        Transition {
            sym_in,
            sym_out,
            trans_info: (target & 0x00FF_FFFF) | ((more as u32) << 24),
        }
    }

    #[test]
    fn load_simple_transducer() {
        let data = build_simple_vfst();
        let t = UnweightedTransducer::from_bytes(&data).unwrap();
        assert_eq!(t.symbols.symbol_strings.len(), 5);
        assert_eq!(t.symbols.first_normal_char, 1);
    }

    #[test]
    fn reject_weighted_data() {
        let mut data = build_simple_vfst();
        data[8] = 0x01; // mark as weighted
        let err = UnweightedTransducer::from_bytes(&data).unwrap_err();
        assert!(matches!(err, VfstError::TypeMismatch { .. }));
    }

    #[test]
    fn traverse_simple_ab_to_xy() {
        let data = build_simple_vfst();
        let t = UnweightedTransducer::from_bytes(&data).unwrap();
        let mut config = t.new_config(100);
        let input: Vec<char> = "ab".chars().collect();

        assert!(t.prepare(&mut config, &input));

        let mut output = String::new();
        assert!(t.next(&mut config, &mut output));
        assert_eq!(output, "xy");

        // No more results
        assert!(!t.next(&mut config, &mut output));
    }

    #[test]
    fn traverse_unknown_input() {
        let data = build_simple_vfst();
        let t = UnweightedTransducer::from_bytes(&data).unwrap();
        let mut config = t.new_config(100);
        let input: Vec<char> = "zz".chars().collect();

        // prepare returns false because 'z' is unknown
        assert!(!t.prepare(&mut config, &input));

        // next returns false because 'z' doesn't match any transition
        let mut output = String::new();
        assert!(!t.next(&mut config, &mut output));
    }

    #[test]
    fn traverse_partial_input_no_match() {
        let data = build_simple_vfst();
        let t = UnweightedTransducer::from_bytes(&data).unwrap();
        let mut config = t.new_config(100);
        let input: Vec<char> = "a".chars().collect();

        t.prepare(&mut config, &input);

        let mut output = String::new();
        // Input 'a' alone should not produce output (final state requires both 'a' and 'b')
        assert!(!t.next(&mut config, &mut output));
    }

    #[test]
    fn traverse_epsilon() {
        let data = build_epsilon_vfst();
        let t = UnweightedTransducer::from_bytes(&data).unwrap();
        let mut config = t.new_config(100);
        let input: Vec<char> = "a".chars().collect();

        t.prepare(&mut config, &input);

        let mut output = String::new();
        // Should find "a" via either the epsilon path or direct path
        assert!(t.next(&mut config, &mut output));
        assert_eq!(output, "a");
    }

    #[test]
    fn next_prefix_matches_prefix() {
        // Build a transducer that accepts just "a" (single char)
        let symbols: &[&str] = &["", "a", "b"];
        let header = build_header(false);
        let sym_table = build_symbol_table(symbols);
        let mut data = Vec::new();
        data.extend_from_slice(&header);
        data.extend_from_slice(&sym_table);
        let partial = data.len() % 8;
        if partial > 0 {
            data.extend(std::iter::repeat_n(0u8, 8 - partial));
        }

        // State 0: 'a' -> state 1
        data.extend_from_slice(bytemuck::bytes_of(&make_transition(1, 1, 1, 0)));
        // State 1: final
        data.extend_from_slice(bytemuck::bytes_of(&make_transition(0xFFFF, 0, 0, 0)));

        let t = UnweightedTransducer::from_bytes(&data).unwrap();
        let mut config = t.new_config(100);
        let input: Vec<char> = "ab".chars().collect(); // 'b' is extra

        t.prepare(&mut config, &input);

        let mut output = String::new();
        let mut prefix_len = 0;

        // next would fail (input not fully consumed), but next_prefix should match 'a'
        assert!(t.next_prefix(&mut config, &mut output, &mut prefix_len));
        assert_eq!(output, "a");
        assert_eq!(prefix_len, 1); // consumed 1 character
    }

    #[test]
    fn multiple_outputs() {
        // Build a transducer with two paths for "a": outputs "x" and "y"
        let symbols: &[&str] = &["", "a", "x", "y"];
        let header = build_header(false);
        let sym_table = build_symbol_table(symbols);
        let mut data = Vec::new();
        data.extend_from_slice(&header);
        data.extend_from_slice(&sym_table);
        let partial = data.len() % 8;
        if partial > 0 {
            data.extend(std::iter::repeat_n(0u8, 8 - partial));
        }

        // State 0: 2 transitions (more=1)
        //   - 'a'(1) -> state 2, output 'x'(2)
        //   - 'a'(1) -> state 3, output 'y'(3)
        data.extend_from_slice(bytemuck::bytes_of(&make_transition(1, 2, 2, 1)));
        data.extend_from_slice(bytemuck::bytes_of(&make_transition(1, 3, 3, 0)));

        // State 2: final
        data.extend_from_slice(bytemuck::bytes_of(&make_transition(0xFFFF, 0, 0, 0)));

        // State 3: final
        data.extend_from_slice(bytemuck::bytes_of(&make_transition(0xFFFF, 0, 0, 0)));

        let t = UnweightedTransducer::from_bytes(&data).unwrap();
        let mut config = t.new_config(100);
        let input: Vec<char> = "a".chars().collect();

        t.prepare(&mut config, &input);

        let mut output = String::new();
        assert!(t.next(&mut config, &mut output));
        assert_eq!(output, "x");

        assert!(t.next(&mut config, &mut output));
        assert_eq!(output, "y");

        assert!(!t.next(&mut config, &mut output));
    }
}
