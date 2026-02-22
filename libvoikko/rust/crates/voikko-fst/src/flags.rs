// Flag diacritic operations: P, C, U, R, D
// Origin: Transducer.cpp:62-123 (parsing)
// Origin: UnweightedTransducer.cpp:228-283 (check algorithm)
// Origin: WeightedTransducer.cpp:230-286 (check algorithm, copy-on-push variant)

use crate::VfstError;
use hashbrown::HashMap;

/// The five flag diacritic operations supported by VFST.
///
/// These control morphological feature constraints during FST traversal.
/// No `N` (Negative) operation exists in this implementation.
///
/// Origin: Transducer.hpp:41-47
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagOp {
    /// Positive Set: unconditionally set feature to value.
    P,
    /// Clear: reset feature to neutral (0).
    C,
    /// Unification: set if neutral, pass if same, fail if different.
    U,
    /// Require: fail if feature does not match required value.
    R,
    /// Disallow: fail if feature matches the disallowed value.
    D,
}

/// Neutral value: feature has not been set.
pub const FLAG_VALUE_NEUTRAL: u16 = 0;

/// Any non-neutral value (wildcard for R/D operations).
pub const FLAG_VALUE_ANY: u16 = 1;

/// A parsed flag diacritic operation with its feature and value indices.
///
/// Origin: Transducer.hpp:49-53
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpFeatureValue {
    pub op: FlagOp,
    pub feature: u16,
    pub value: u16,
}

impl Default for OpFeatureValue {
    fn default() -> Self {
        Self {
            op: FlagOp::P,
            feature: 0,
            value: FLAG_VALUE_NEUTRAL,
        }
    }
}

/// Result of a flag diacritic check: whether the transition is allowed,
/// and if so, whether the flag state should be updated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagCheckResult {
    /// Transition is not allowed (constraint violation).
    Reject,
    /// Transition is allowed; the feature should be set to the given value.
    AcceptAndUpdate { feature: u16, value: u16 },
    /// Transition is allowed; no flag state change needed.
    AcceptNoUpdate { feature: u16 },
}

/// Check whether a flag diacritic transition is allowed given the current flag state.
///
/// This is the core flag diacritic algorithm used by both weighted and unweighted
/// traversal. The caller is responsible for actually updating the flag state
/// based on the returned result.
///
/// Origin: UnweightedTransducer.cpp:228-283, WeightedTransducer.cpp:230-286
pub fn check_flag(ofv: &OpFeatureValue, current_value: u16) -> FlagCheckResult {
    match ofv.op {
        FlagOp::P => FlagCheckResult::AcceptAndUpdate {
            feature: ofv.feature,
            value: ofv.value,
        },
        FlagOp::C => FlagCheckResult::AcceptAndUpdate {
            feature: ofv.feature,
            value: FLAG_VALUE_NEUTRAL,
        },
        FlagOp::U => {
            if current_value != FLAG_VALUE_NEUTRAL {
                if current_value != ofv.value {
                    FlagCheckResult::Reject
                } else {
                    // Already unified to same value: no update needed
                    FlagCheckResult::AcceptNoUpdate {
                        feature: ofv.feature,
                    }
                }
            } else {
                // Neutral: set to the requested value
                FlagCheckResult::AcceptAndUpdate {
                    feature: ofv.feature,
                    value: ofv.value,
                }
            }
        }
        FlagOp::R => {
            if ofv.value == FLAG_VALUE_ANY {
                if current_value == FLAG_VALUE_NEUTRAL {
                    return FlagCheckResult::Reject;
                }
            } else if current_value != ofv.value {
                return FlagCheckResult::Reject;
            }
            FlagCheckResult::AcceptNoUpdate {
                feature: ofv.feature,
            }
        }
        FlagOp::D => {
            if (ofv.value == FLAG_VALUE_ANY && current_value != FLAG_VALUE_NEUTRAL)
                || current_value == ofv.value
            {
                return FlagCheckResult::Reject;
            }
            FlagCheckResult::AcceptNoUpdate {
                feature: ofv.feature,
            }
        }
    }
}

/// Parser state for accumulating flag diacritic features and values across
/// all symbols in a symbol table.
pub struct FlagDiacriticParser {
    features: HashMap<String, u16>,
    values: HashMap<String, u16>,
}

impl Default for FlagDiacriticParser {
    fn default() -> Self {
        Self::new()
    }
}

impl FlagDiacriticParser {
    pub fn new() -> Self {
        let mut values = HashMap::new();
        values.insert(String::new(), FLAG_VALUE_NEUTRAL);
        values.insert("@".to_string(), FLAG_VALUE_ANY);
        Self {
            features: HashMap::new(),
            values,
        }
    }

    /// Return the number of distinct features seen so far.
    pub fn feature_count(&self) -> u16 {
        self.features.len() as u16
    }

    /// Parse a flag diacritic symbol string like `@P.FEATURE.VALUE@` or `@C.FEATURE@`.
    ///
    /// Returns the parsed operation with feature and value indices. Features and values
    /// are assigned sequential indices as they are first encountered.
    ///
    /// Origin: Transducer::getDiacriticOperation() -- Transducer.cpp:62-123
    pub fn parse(&mut self, symbol: &str) -> Result<OpFeatureValue, VfstError> {
        let bytes = symbol.as_bytes();
        if bytes.len() <= 4 {
            return Err(VfstError::InvalidFlagDiacritic(format!(
                "too short: {symbol:?}"
            )));
        }

        let op = match bytes[1] {
            b'P' => FlagOp::P,
            b'C' => FlagOp::C,
            b'U' => FlagOp::U,
            b'R' => FlagOp::R,
            b'D' => FlagOp::D,
            _ => {
                return Err(VfstError::InvalidFlagDiacritic(format!(
                    "unknown operation '{}' in {symbol:?}",
                    bytes[1] as char,
                )));
            }
        };

        // Extract feature and value from `@OP.FEATURE.VALUE@` or `@OP.FEATURE@`
        // symbol[3..symbol.len()-1] gives "FEATURE.VALUE" or "FEATURE"
        let inner = &symbol[3..symbol.len() - 1];
        let (feature_str, value_str) = match inner.find('.') {
            Some(dot_pos) => (&inner[..dot_pos], &inner[dot_pos + 1..]),
            None => (inner, "@"), // no value -> use "@" (FlagValueAny mapping)
        };

        let feature = {
            let next_idx = self.features.len() as u16;
            *self.features.entry(feature_str.to_string()).or_insert(next_idx)
        };

        let value = {
            let next_idx = self.values.len() as u16;
            *self.values.entry(value_str.to_string()).or_insert(next_idx)
        };

        Ok(OpFeatureValue { op, feature, value })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- check_flag tests ---

    #[test]
    fn positive_set_always_updates() {
        let ofv = OpFeatureValue {
            op: FlagOp::P,
            feature: 0,
            value: 5,
        };
        let result = check_flag(&ofv, FLAG_VALUE_NEUTRAL);
        assert_eq!(
            result,
            FlagCheckResult::AcceptAndUpdate {
                feature: 0,
                value: 5
            }
        );

        // Even if already set to something else
        let result2 = check_flag(&ofv, 3);
        assert_eq!(
            result2,
            FlagCheckResult::AcceptAndUpdate {
                feature: 0,
                value: 5
            }
        );
    }

    #[test]
    fn clear_resets_to_neutral() {
        let ofv = OpFeatureValue {
            op: FlagOp::C,
            feature: 2,
            value: 99, // value is ignored by C; it always clears to neutral
        };
        let result = check_flag(&ofv, 5);
        assert_eq!(
            result,
            FlagCheckResult::AcceptAndUpdate {
                feature: 2,
                value: FLAG_VALUE_NEUTRAL
            }
        );
    }

    #[test]
    fn unification_from_neutral() {
        let ofv = OpFeatureValue {
            op: FlagOp::U,
            feature: 0,
            value: 3,
        };
        // Feature is neutral -> set it
        let result = check_flag(&ofv, FLAG_VALUE_NEUTRAL);
        assert_eq!(
            result,
            FlagCheckResult::AcceptAndUpdate {
                feature: 0,
                value: 3
            }
        );
    }

    #[test]
    fn unification_same_value_passes() {
        let ofv = OpFeatureValue {
            op: FlagOp::U,
            feature: 0,
            value: 3,
        };
        // Already set to same value -> no update
        let result = check_flag(&ofv, 3);
        assert_eq!(result, FlagCheckResult::AcceptNoUpdate { feature: 0 });
    }

    #[test]
    fn unification_different_value_rejects() {
        let ofv = OpFeatureValue {
            op: FlagOp::U,
            feature: 0,
            value: 3,
        };
        let result = check_flag(&ofv, 5);
        assert_eq!(result, FlagCheckResult::Reject);
    }

    #[test]
    fn require_any_when_set_passes() {
        let ofv = OpFeatureValue {
            op: FlagOp::R,
            feature: 0,
            value: FLAG_VALUE_ANY,
        };
        let result = check_flag(&ofv, 3);
        assert_eq!(result, FlagCheckResult::AcceptNoUpdate { feature: 0 });
    }

    #[test]
    fn require_any_when_neutral_rejects() {
        let ofv = OpFeatureValue {
            op: FlagOp::R,
            feature: 0,
            value: FLAG_VALUE_ANY,
        };
        let result = check_flag(&ofv, FLAG_VALUE_NEUTRAL);
        assert_eq!(result, FlagCheckResult::Reject);
    }

    #[test]
    fn require_specific_value_matches() {
        let ofv = OpFeatureValue {
            op: FlagOp::R,
            feature: 0,
            value: 5,
        };
        let result = check_flag(&ofv, 5);
        assert_eq!(result, FlagCheckResult::AcceptNoUpdate { feature: 0 });
    }

    #[test]
    fn require_specific_value_mismatches() {
        let ofv = OpFeatureValue {
            op: FlagOp::R,
            feature: 0,
            value: 5,
        };
        let result = check_flag(&ofv, 3);
        assert_eq!(result, FlagCheckResult::Reject);
    }

    #[test]
    fn disallow_any_when_neutral_passes() {
        let ofv = OpFeatureValue {
            op: FlagOp::D,
            feature: 0,
            value: FLAG_VALUE_ANY,
        };
        let result = check_flag(&ofv, FLAG_VALUE_NEUTRAL);
        assert_eq!(result, FlagCheckResult::AcceptNoUpdate { feature: 0 });
    }

    #[test]
    fn disallow_any_when_set_rejects() {
        let ofv = OpFeatureValue {
            op: FlagOp::D,
            feature: 0,
            value: FLAG_VALUE_ANY,
        };
        let result = check_flag(&ofv, 3);
        assert_eq!(result, FlagCheckResult::Reject);
    }

    #[test]
    fn disallow_specific_value_matches_rejects() {
        let ofv = OpFeatureValue {
            op: FlagOp::D,
            feature: 0,
            value: 5,
        };
        let result = check_flag(&ofv, 5);
        assert_eq!(result, FlagCheckResult::Reject);
    }

    #[test]
    fn disallow_specific_value_differs_passes() {
        let ofv = OpFeatureValue {
            op: FlagOp::D,
            feature: 0,
            value: 5,
        };
        let result = check_flag(&ofv, 3);
        assert_eq!(result, FlagCheckResult::AcceptNoUpdate { feature: 0 });
    }

    #[test]
    fn disallow_specific_value_neutral_passes() {
        let ofv = OpFeatureValue {
            op: FlagOp::D,
            feature: 0,
            value: 5,
        };
        let result = check_flag(&ofv, FLAG_VALUE_NEUTRAL);
        assert_eq!(result, FlagCheckResult::AcceptNoUpdate { feature: 0 });
    }

    // --- FlagDiacriticParser tests ---

    #[test]
    fn parse_positive_set_with_value() {
        let mut parser = FlagDiacriticParser::new();
        let ofv = parser.parse("@P.CASE.NOM@").unwrap();
        assert_eq!(ofv.op, FlagOp::P);
        // Feature and value indices are assigned sequentially
        assert_eq!(ofv.feature, 0); // first feature
        assert!(ofv.value >= 2); // 0=neutral(""), 1=any("@"), 2+=user values
    }

    #[test]
    fn parse_clear_without_value() {
        let mut parser = FlagDiacriticParser::new();
        let ofv = parser.parse("@C.CASE@").unwrap();
        assert_eq!(ofv.op, FlagOp::C);
        assert_eq!(ofv.feature, 0);
        // No value dot -> value_str is "@" -> maps to FlagValueAny(1)
        assert_eq!(ofv.value, FLAG_VALUE_ANY);
    }

    #[test]
    fn parse_unification() {
        let mut parser = FlagDiacriticParser::new();
        let ofv = parser.parse("@U.VOWEL.BACK@").unwrap();
        assert_eq!(ofv.op, FlagOp::U);
    }

    #[test]
    fn parse_require() {
        let mut parser = FlagDiacriticParser::new();
        let ofv = parser.parse("@R.NUM.SG@").unwrap();
        assert_eq!(ofv.op, FlagOp::R);
    }

    #[test]
    fn parse_disallow() {
        let mut parser = FlagDiacriticParser::new();
        let ofv = parser.parse("@D.POSS@").unwrap();
        assert_eq!(ofv.op, FlagOp::D);
    }

    #[test]
    fn feature_indices_are_stable() {
        let mut parser = FlagDiacriticParser::new();
        let ofv1 = parser.parse("@P.CASE.NOM@").unwrap();
        let ofv2 = parser.parse("@P.NUM.SG@").unwrap();
        let ofv3 = parser.parse("@R.CASE.GEN@").unwrap();

        // CASE appears first, gets index 0
        assert_eq!(ofv1.feature, 0);
        // NUM is second feature
        assert_eq!(ofv2.feature, 1);
        // CASE again -> same index
        assert_eq!(ofv3.feature, 0);

        assert_eq!(parser.feature_count(), 2);
    }

    #[test]
    fn value_indices_are_stable() {
        let mut parser = FlagDiacriticParser::new();
        let ofv1 = parser.parse("@P.X.NOM@").unwrap();
        let ofv2 = parser.parse("@P.X.GEN@").unwrap();
        let ofv3 = parser.parse("@P.Y.NOM@").unwrap();

        // NOM first user value: index 2
        assert_eq!(ofv1.value, 2);
        // GEN second user value: index 3
        assert_eq!(ofv2.value, 3);
        // NOM again -> same index
        assert_eq!(ofv3.value, 2);
    }

    #[test]
    fn reject_too_short_symbol() {
        let mut parser = FlagDiacriticParser::new();
        let err = parser.parse("@P@").unwrap_err();
        assert!(matches!(err, VfstError::InvalidFlagDiacritic(_)));
    }

    #[test]
    fn reject_unknown_operation() {
        let mut parser = FlagDiacriticParser::new();
        let err = parser.parse("@X.FOO@").unwrap_err();
        assert!(matches!(err, VfstError::InvalidFlagDiacritic(_)));
    }
}
