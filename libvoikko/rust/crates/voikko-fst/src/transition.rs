// Transition and WeightedTransition structs for zero-copy access to VFST binary data.
// Origin: Transition.hpp, WeightedTransition.hpp

use bytemuck::{Pod, Zeroable};

/// Unweighted transition (8 bytes).
///
/// Layout matches the C++ `Transition` struct exactly:
/// - `sym_in` (u16): input symbol index
/// - `sym_out` (u16): output symbol index
/// - `trans_info` (u32): packed bitfield containing target_state (bits 0-23)
///   and more_transitions (bits 24-31)
///
/// The `transinfo_t` C++ bitfield is stored as a raw u32 with explicit
/// bit-mask extraction for portability.
///
/// Origin: Transition.hpp:41-45
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Transition {
    pub sym_in: u16,
    pub sym_out: u16,
    pub trans_info: u32,
}

/// Sentinel value for final-state input symbol in unweighted transducers.
pub const UNWEIGHTED_FINAL_SYM: u16 = 0xFFFF;

impl Transition {
    /// Extract the target state index from the packed transinfo field (bits 0-23).
    #[inline]
    pub fn target_state(&self) -> u32 {
        self.trans_info & 0x00FF_FFFF
    }

    /// Extract the more_transitions count from the packed transinfo field (bits 24-31).
    #[inline]
    pub fn more_transitions(&self) -> u8 {
        (self.trans_info >> 24) as u8
    }
}

/// Unweighted overflow cell (8 bytes).
///
/// When `more_transitions == 255`, the next slot in the transition table is an
/// overflow cell that holds the actual count.
///
/// Origin: Transition.hpp:47-50
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct OverflowCell {
    pub more_transitions: u32,
    pub _padding: u32,
}

/// Weighted transition (16 bytes).
///
/// Layout matches the C++ `WeightedTransition` struct exactly:
/// - `sym_in` (u32): input symbol index
/// - `sym_out` (u32): output symbol index
/// - `target_state` (u32): target state index
/// - `weight` (i16): transition weight (signed)
/// - `more_transitions` (u8): extra transition count (255 = overflow)
/// - `_reserved` (u8): padding
///
/// Origin: WeightedTransition.hpp:36-43
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct WeightedTransition {
    pub sym_in: u32,
    pub sym_out: u32,
    pub target_state: u32,
    pub weight: i16,
    pub more_transitions: u8,
    pub _reserved: u8,
}

/// Sentinel value for final-state input symbol in weighted transducers.
pub const WEIGHTED_FINAL_SYM: u32 = 0xFFFF_FFFF;

/// Weighted overflow cell (16 bytes).
///
/// Origin: WeightedTransition.hpp:45-49
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct WeightedOverflowCell {
    pub more_transitions: u32,
    pub _short_padding: u32,
    pub _padding: u64,
}

/// Compute the maximum transition index (0-based) for a state, given its
/// first transition in the unweighted transition table.
///
/// If `more_transitions < 255`, the state has `more_transitions + 1` transitions
/// (indices 0..=more_transitions). If `more_transitions == 255`, the next slot
/// is an `OverflowCell` whose `more_transitions` field gives the adjusted count.
///
/// Origin: UnweightedTransducer.cpp:219-226
#[inline]
pub fn unweighted_max_tc(transitions: &[Transition], state_index: u32) -> u32 {
    let state_head = &transitions[state_index as usize];
    let max_tc = state_head.more_transitions() as u32;
    if max_tc == 255 {
        // The next slot is an overflow cell
        let overflow_bytes =
            bytemuck::bytes_of(&transitions[state_index as usize + 1]);
        let oc: &OverflowCell = bytemuck::from_bytes(overflow_bytes);
        oc.more_transitions + 1
    } else {
        max_tc
    }
}

/// Compute the maximum transition index for a state in the weighted transition table.
///
/// Origin: WeightedTransducer.cpp:221-228
#[inline]
pub fn weighted_max_tc(transitions: &[WeightedTransition], state_index: u32) -> u32 {
    let state_head = &transitions[state_index as usize];
    let max_tc = state_head.more_transitions as u32;
    if max_tc == 255 {
        let overflow_bytes =
            bytemuck::bytes_of(&transitions[state_index as usize + 1]);
        let oc: &WeightedOverflowCell = bytemuck::from_bytes(overflow_bytes);
        oc.more_transitions + 1
    } else {
        max_tc
    }
}

// Static assertions for struct sizes
const _: () = assert!(size_of::<Transition>() == 8);
const _: () = assert!(size_of::<OverflowCell>() == 8);
const _: () = assert!(size_of::<WeightedTransition>() == 16);
const _: () = assert!(size_of::<WeightedOverflowCell>() == 16);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_size() {
        assert_eq!(size_of::<Transition>(), 8);
    }

    #[test]
    fn weighted_transition_size() {
        assert_eq!(size_of::<WeightedTransition>(), 16);
    }

    #[test]
    fn overflow_cell_size() {
        assert_eq!(size_of::<OverflowCell>(), 8);
    }

    #[test]
    fn weighted_overflow_cell_size() {
        assert_eq!(size_of::<WeightedOverflowCell>(), 16);
    }

    #[test]
    fn transition_field_extraction() {
        let t = Transition {
            sym_in: 5,
            sym_out: 10,
            // target_state=0x123456 (bits 0-23), more_transitions=0xAB (bits 24-31)
            trans_info: 0xAB_123456,
        };
        assert_eq!(t.sym_in, 5);
        assert_eq!(t.sym_out, 10);
        assert_eq!(t.target_state(), 0x123456);
        assert_eq!(t.more_transitions(), 0xAB);
    }

    #[test]
    fn transition_zero_target() {
        let t = Transition {
            sym_in: 0,
            sym_out: 0,
            trans_info: 0x03_000000, // more_transitions=3, target_state=0
        };
        assert_eq!(t.target_state(), 0);
        assert_eq!(t.more_transitions(), 3);
    }

    #[test]
    fn transition_max_target_state() {
        let t = Transition {
            sym_in: 0,
            sym_out: 0,
            trans_info: 0x00_FFFFFF, // target_state = max 24-bit
        };
        assert_eq!(t.target_state(), 0x00FF_FFFF);
        assert_eq!(t.more_transitions(), 0);
    }

    #[test]
    fn zero_copy_cast_unweighted() {
        let raw: [u8; 16] = [
            // Transition 1: sym_in=1, sym_out=2, target=3, more=0
            0x01, 0x00, // sym_in = 1
            0x02, 0x00, // sym_out = 2
            0x03, 0x00, 0x00, 0x00, // trans_info: target=3, more=0
            // Transition 2: sym_in=4, sym_out=5, target=6, more=1
            0x04, 0x00, // sym_in = 4
            0x05, 0x00, // sym_out = 5
            0x06, 0x00, 0x00, 0x01, // trans_info: target=6, more=1
        ];
        let transitions: &[Transition] = bytemuck::cast_slice(&raw);
        assert_eq!(transitions.len(), 2);
        assert_eq!(transitions[0].sym_in, 1);
        assert_eq!(transitions[0].sym_out, 2);
        assert_eq!(transitions[0].target_state(), 3);
        assert_eq!(transitions[0].more_transitions(), 0);
        assert_eq!(transitions[1].sym_in, 4);
        assert_eq!(transitions[1].sym_out, 5);
        assert_eq!(transitions[1].target_state(), 6);
        assert_eq!(transitions[1].more_transitions(), 1);
    }

    #[test]
    fn zero_copy_cast_weighted() {
        let raw: [u8; 16] = [
            0x01, 0x00, 0x00, 0x00, // sym_in = 1
            0x02, 0x00, 0x00, 0x00, // sym_out = 2
            0x03, 0x00, 0x00, 0x00, // target_state = 3
            0x0A, 0x00, // weight = 10
            0x02, // more_transitions = 2
            0x00, // reserved
        ];
        let transitions: &[WeightedTransition] = bytemuck::cast_slice(&raw);
        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].sym_in, 1);
        assert_eq!(transitions[0].sym_out, 2);
        assert_eq!(transitions[0].target_state, 3);
        assert_eq!(transitions[0].weight, 10);
        assert_eq!(transitions[0].more_transitions, 2);
    }

    #[test]
    fn weighted_negative_weight() {
        let wt = WeightedTransition {
            sym_in: 0,
            sym_out: 0,
            target_state: 0,
            weight: -500,
            more_transitions: 0,
            _reserved: 0,
        };
        assert_eq!(wt.weight, -500);
    }

    #[test]
    fn unweighted_max_tc_simple() {
        // A state with 3 transitions (more_transitions = 2 means 3 total)
        let transitions = vec![
            Transition {
                sym_in: 0,
                sym_out: 0,
                trans_info: 0x02_000000,
            }, // state head, more=2
            Transition {
                sym_in: 1,
                sym_out: 1,
                trans_info: 0,
            },
            Transition {
                sym_in: 2,
                sym_out: 2,
                trans_info: 0,
            },
        ];
        assert_eq!(unweighted_max_tc(&transitions, 0), 2);
    }

    #[test]
    fn unweighted_max_tc_overflow() {
        // State head with more_transitions=255, followed by overflow cell with count=300
        let mut transitions = vec![
            Transition {
                sym_in: 0,
                sym_out: 0,
                trans_info: 0xFF_000000,
            }, // more=255
        ];
        // Overflow cell encoded as a Transition (same 8 bytes)
        let oc = OverflowCell {
            more_transitions: 300,
            _padding: 0,
        };
        let oc_as_transition: Transition = bytemuck::cast(oc);
        transitions.push(oc_as_transition);

        // more_transitions + 1 = 301
        assert_eq!(unweighted_max_tc(&transitions, 0), 301);
    }

    #[test]
    fn weighted_max_tc_simple() {
        let transitions = vec![WeightedTransition {
            sym_in: 0,
            sym_out: 0,
            target_state: 0,
            weight: 0,
            more_transitions: 5,
            _reserved: 0,
        }];
        assert_eq!(weighted_max_tc(&transitions, 0), 5);
    }
}
