// Traversal configuration / state stack
// Origin: Configuration.hpp/cpp, WeightedConfiguration.hpp/cpp
//
// This module unifies the C++ Configuration and WeightedConfiguration into
// separate Rust types, each tailored to its traversal variant's needs.

/// Traversal configuration for the unweighted transducer.
///
/// Holds the explicit DFS stack and flag diacritic undo state. The unweighted
/// variant uses a destructive-update + undo strategy for flag diacritics:
/// `current_flag_values` is mutated in place, and the previous value is saved
/// in `flag_undo_value`/`flag_undo_feature` for backtracking.
///
/// Origin: Configuration.hpp:38-54, Configuration.cpp
pub struct UnweightedConfig {
    pub buffer_size: usize,
    pub stack_depth: usize,
    pub flag_depth: usize,
    pub input_depth: usize,
    pub input_length: usize,

    /// State index at each stack depth.
    pub state_index_stack: Vec<u32>,
    /// Current transition index at each stack depth (pointer into transition table).
    pub current_transition_stack: Vec<u32>,
    /// Pre-mapped input symbol indices.
    pub input_symbol_stack: Vec<u16>,
    /// Output symbol index at each stack depth (0 for epsilon/flags).
    pub output_symbol_stack: Vec<u16>,

    /// Current flag values indexed by feature (mutated in-place).
    pub current_flag_values: Vec<u16>,
    /// Previous flag value before update at each flag_depth (for undo).
    pub flag_undo_value: Vec<u16>,
    /// Which feature was updated at each flag_depth (for undo).
    pub flag_undo_feature: Vec<u16>,
}

impl UnweightedConfig {
    /// Create a new unweighted traversal configuration.
    ///
    /// `flag_feature_count`: number of distinct flag diacritic features.
    /// `buffer_size`: maximum stack depth (typically 2000).
    pub fn new(flag_feature_count: u16, buffer_size: usize) -> Self {
        let fc = flag_feature_count as usize;
        Self {
            buffer_size,
            stack_depth: 0,
            flag_depth: 0,
            input_depth: 0,
            input_length: 0,
            state_index_stack: vec![0; buffer_size],
            current_transition_stack: vec![0; buffer_size],
            input_symbol_stack: vec![0; buffer_size],
            output_symbol_stack: vec![0; buffer_size],
            current_flag_values: vec![0; fc],
            flag_undo_value: if fc > 0 {
                vec![0; buffer_size]
            } else {
                Vec::new()
            },
            flag_undo_feature: if fc > 0 {
                vec![0; buffer_size]
            } else {
                Vec::new()
            },
        }
    }

    /// Reset depths to initial state (called at the start of `prepare`).
    #[inline]
    pub fn reset(&mut self) {
        self.stack_depth = 0;
        self.flag_depth = 0;
        self.input_depth = 0;
        self.input_length = 0;
        self.state_index_stack[0] = 0;
        self.current_transition_stack[0] = 0;
        // Reset flag values to neutral
        for v in &mut self.current_flag_values {
            *v = 0;
        }
    }
}

/// Traversal configuration for the weighted transducer.
///
/// Uses a copy-on-push strategy for flag diacritics: the entire flag array
/// is copied forward one slot at each flag diacritic step, allowing instant
/// backtrack by decrementing `flag_depth`.
///
/// Origin: WeightedConfiguration.hpp:38-52, WeightedConfiguration.cpp
pub struct WeightedConfig {
    pub buffer_size: usize,
    pub stack_depth: usize,
    pub flag_depth: usize,
    pub input_depth: usize,
    pub input_length: usize,

    /// State index at each stack depth.
    pub state_index_stack: Vec<u32>,
    /// Current transition index at each stack depth.
    pub current_transition_stack: Vec<u32>,
    /// Pre-mapped input symbol indices (u32 for weighted).
    pub input_symbol_stack: Vec<u32>,
    /// Output symbol index at each stack depth (u32 for weighted).
    pub output_symbol_stack: Vec<u32>,

    /// Flattened 2D flag value stack: `[flag_depth * feature_count + feature]`.
    /// Each "row" of `feature_count` values is a snapshot of the flag state
    /// at that depth. Copy-on-push: row N+1 is copied from row N before
    /// modification.
    pub flag_value_stack: Vec<u32>,
    /// Number of flag diacritic features (used to index into flag_value_stack).
    pub flag_feature_count: u32,
}

impl WeightedConfig {
    /// Create a new weighted traversal configuration.
    ///
    /// `flag_feature_count`: number of distinct flag diacritic features.
    /// `buffer_size`: maximum stack depth (typically 2000).
    pub fn new(flag_feature_count: u16, buffer_size: usize) -> Self {
        let fc = flag_feature_count as u32;
        Self {
            buffer_size,
            stack_depth: 0,
            flag_depth: 0,
            input_depth: 0,
            input_length: 0,
            state_index_stack: vec![0; buffer_size],
            current_transition_stack: vec![0; buffer_size],
            input_symbol_stack: vec![0; buffer_size],
            output_symbol_stack: vec![0; buffer_size],
            flag_value_stack: if fc > 0 {
                vec![0; fc as usize * buffer_size]
            } else {
                Vec::new()
            },
            flag_feature_count: fc,
        }
    }

    /// Reset depths to initial state (called at the start of `prepare`).
    #[inline]
    pub fn reset(&mut self) {
        self.stack_depth = 0;
        self.flag_depth = 0;
        self.input_depth = 0;
        self.input_length = 0;
        self.state_index_stack[0] = 0;
        self.current_transition_stack[0] = 0;
        // Reset initial flag row to neutral
        let fc = self.flag_feature_count as usize;
        if fc > 0 {
            for i in 0..fc {
                self.flag_value_stack[i] = 0;
            }
        }
    }

    /// Get a slice of the current flag values (at the current flag_depth).
    #[inline]
    pub fn current_flags(&self) -> &[u32] {
        let fc = self.flag_feature_count as usize;
        if fc == 0 {
            return &[];
        }
        let start = self.flag_depth * fc;
        &self.flag_value_stack[start..start + fc]
    }

    /// Get a mutable slice of the current flag values.
    #[inline]
    pub fn current_flags_mut(&mut self) -> &mut [u32] {
        let fc = self.flag_feature_count as usize;
        if fc == 0 {
            return &mut [];
        }
        let start = self.flag_depth * fc;
        &mut self.flag_value_stack[start..start + fc]
    }

    /// Copy current flag row forward for the next depth (copy-on-push).
    /// After this, `flag_depth` is incremented by 1.
    #[inline]
    pub fn push_flags(&mut self) {
        let fc = self.flag_feature_count as usize;
        if fc == 0 {
            return;
        }
        let src_start = self.flag_depth * fc;
        let dst_start = src_start + fc;
        // Copy within the same vector
        for i in 0..fc {
            self.flag_value_stack[dst_start + i] = self.flag_value_stack[src_start + i];
        }
        self.flag_depth += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unweighted_config_creation() {
        let config = UnweightedConfig::new(3, 100);
        assert_eq!(config.buffer_size, 100);
        assert_eq!(config.stack_depth, 0);
        assert_eq!(config.state_index_stack.len(), 100);
        assert_eq!(config.current_flag_values.len(), 3);
        assert_eq!(config.flag_undo_value.len(), 100);
        assert_eq!(config.flag_undo_feature.len(), 100);
    }

    #[test]
    fn unweighted_config_no_flags() {
        let config = UnweightedConfig::new(0, 50);
        assert_eq!(config.current_flag_values.len(), 0);
        assert_eq!(config.flag_undo_value.len(), 0);
        assert_eq!(config.flag_undo_feature.len(), 0);
    }

    #[test]
    fn unweighted_config_reset() {
        let mut config = UnweightedConfig::new(2, 50);
        config.stack_depth = 10;
        config.flag_depth = 5;
        config.input_depth = 3;
        config.input_length = 7;
        config.current_flag_values[0] = 42;
        config.current_flag_values[1] = 99;

        config.reset();

        assert_eq!(config.stack_depth, 0);
        assert_eq!(config.flag_depth, 0);
        assert_eq!(config.input_depth, 0);
        assert_eq!(config.input_length, 0);
        assert_eq!(config.current_flag_values[0], 0);
        assert_eq!(config.current_flag_values[1], 0);
    }

    #[test]
    fn weighted_config_creation() {
        let config = WeightedConfig::new(3, 100);
        assert_eq!(config.buffer_size, 100);
        assert_eq!(config.flag_feature_count, 3);
        assert_eq!(config.flag_value_stack.len(), 300); // 3 * 100
    }

    #[test]
    fn weighted_config_no_flags() {
        let config = WeightedConfig::new(0, 50);
        assert_eq!(config.flag_value_stack.len(), 0);
    }

    #[test]
    fn weighted_config_push_flags() {
        let mut config = WeightedConfig::new(3, 10);
        // Set some flag values at depth 0
        config.current_flags_mut()[0] = 5;
        config.current_flags_mut()[1] = 10;
        config.current_flags_mut()[2] = 15;

        // Push flags (copy to depth 1)
        config.push_flags();
        assert_eq!(config.flag_depth, 1);

        // Values at depth 1 should be copied from depth 0
        assert_eq!(config.current_flags()[0], 5);
        assert_eq!(config.current_flags()[1], 10);
        assert_eq!(config.current_flags()[2], 15);

        // Modify at depth 1
        config.current_flags_mut()[0] = 99;

        // Pop back to depth 0
        config.flag_depth -= 1;

        // Original values at depth 0 should be intact
        assert_eq!(config.current_flags()[0], 5);
    }

    #[test]
    fn weighted_config_reset() {
        let mut config = WeightedConfig::new(2, 50);
        config.stack_depth = 10;
        config.flag_depth = 3;
        config.flag_value_stack[0] = 42;
        config.flag_value_stack[1] = 99;

        config.reset();

        assert_eq!(config.stack_depth, 0);
        assert_eq!(config.flag_depth, 0);
        assert_eq!(config.flag_value_stack[0], 0);
        assert_eq!(config.flag_value_stack[1], 0);
    }
}
