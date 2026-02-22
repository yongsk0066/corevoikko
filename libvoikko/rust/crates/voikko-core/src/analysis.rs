// Morphological analysis result type
// Origin: morphology/Analysis.hpp, Analysis.cpp

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Attribute key constants
// Origin: Analysis.hpp:44-66 (voikko_mor_analysis::Key enum)
// ---------------------------------------------------------------------------

pub const ATTR_BASEFORM: &str = "BASEFORM";
pub const ATTR_CLASS: &str = "CLASS";
pub const ATTR_COMPARISON: &str = "COMPARISON";
pub const ATTR_FOCUS: &str = "FOCUS";
pub const ATTR_FSTOUTPUT: &str = "FSTOUTPUT";
pub const ATTR_KYSYMYSLIITE: &str = "KYSYMYSLIITE";
pub const ATTR_MALAGA_VAPAA_JALKIOSA: &str = "MALAGA_VAPAA_JALKIOSA";
pub const ATTR_MOOD: &str = "MOOD";
pub const ATTR_NEGATIVE: &str = "NEGATIVE";
pub const ATTR_NUMBER: &str = "NUMBER";
pub const ATTR_PARTICIPLE: &str = "PARTICIPLE";
pub const ATTR_PERSON: &str = "PERSON";
pub const ATTR_POSSESSIVE: &str = "POSSESSIVE";
pub const ATTR_POSSIBLE_GEOGRAPHICAL_NAME: &str = "POSSIBLE_GEOGRAPHICAL_NAME";
pub const ATTR_REQUIRE_FOLLOWING_VERB: &str = "REQUIRE_FOLLOWING_VERB";
pub const ATTR_SIJAMUOTO: &str = "SIJAMUOTO";
pub const ATTR_STRUCTURE: &str = "STRUCTURE";
pub const ATTR_TENSE: &str = "TENSE";
pub const ATTR_WEIGHT: &str = "WEIGHT";
pub const ATTR_WORDBASES: &str = "WORDBASES";
pub const ATTR_WORDIDS: &str = "WORDIDS";

/// Result of morphological analysis: a set of key-value attribute pairs.
///
/// In the C++ code this is `voikko_mor_analysis` which stores `map<Key, wchar_t*>`.
/// In Rust we use a simple `HashMap<String, String>` and expose typed accessors
/// for the well-known attribute keys.
///
/// Origin: morphology/Analysis.hpp:42-131
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Analysis {
    attributes: HashMap<String, String>,
}

impl Analysis {
    /// Create a new empty analysis.
    /// Origin: Analysis.cpp:83
    pub fn new() -> Self {
        Self {
            attributes: HashMap::new(),
        }
    }

    /// Set an attribute value. Replaces any previous value for the same key.
    /// Origin: Analysis.cpp:97-98
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.attributes.insert(key.into(), value.into());
    }

    /// Get an attribute value by key. Returns `None` if not present.
    /// Origin: Analysis.cpp:131-140
    pub fn get(&self, key: &str) -> Option<&str> {
        self.attributes.get(key).map(String::as_str)
    }

    /// Remove an attribute by key.
    /// Origin: Analysis.cpp:106-117
    pub fn remove(&mut self, key: &str) {
        self.attributes.remove(key);
    }

    /// Return the list of attribute keys present in this analysis.
    /// Origin: Analysis.cpp:123-129
    pub fn keys(&self) -> Vec<&str> {
        self.attributes.keys().map(String::as_str).collect()
    }

    /// Check whether a given attribute key is present.
    pub fn contains_key(&self, key: &str) -> bool {
        self.attributes.contains_key(key)
    }

    /// Return a reference to the underlying attribute map.
    pub fn attributes(&self) -> &HashMap<String, String> {
        &self.attributes
    }

    /// Return the number of attributes.
    pub fn len(&self) -> usize {
        self.attributes.len()
    }

    /// Check whether the analysis has no attributes.
    pub fn is_empty(&self) -> bool {
        self.attributes.is_empty()
    }
}

impl Default for Analysis {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_analysis_is_empty() {
        let a = Analysis::new();
        assert!(a.is_empty());
        assert_eq!(a.len(), 0);
    }

    #[test]
    fn set_and_get() {
        let mut a = Analysis::new();
        a.set(ATTR_BASEFORM, "koira");
        assert_eq!(a.get(ATTR_BASEFORM), Some("koira"));
        assert_eq!(a.get(ATTR_CLASS), None);
    }

    #[test]
    fn set_replaces_existing() {
        let mut a = Analysis::new();
        a.set(ATTR_BASEFORM, "koira");
        a.set(ATTR_BASEFORM, "kissa");
        assert_eq!(a.get(ATTR_BASEFORM), Some("kissa"));
        assert_eq!(a.len(), 1);
    }

    #[test]
    fn remove_attribute() {
        let mut a = Analysis::new();
        a.set(ATTR_CLASS, "nimisana");
        assert!(a.contains_key(ATTR_CLASS));
        a.remove(ATTR_CLASS);
        assert!(!a.contains_key(ATTR_CLASS));
        assert!(a.is_empty());
    }

    #[test]
    fn remove_nonexistent_is_noop() {
        let mut a = Analysis::new();
        a.remove(ATTR_BASEFORM); // should not panic
        assert!(a.is_empty());
    }

    #[test]
    fn keys_returns_all_keys() {
        let mut a = Analysis::new();
        a.set(ATTR_BASEFORM, "koira");
        a.set(ATTR_CLASS, "nimisana");
        a.set(ATTR_STRUCTURE, "=pp");
        let mut keys = a.keys();
        keys.sort();
        assert_eq!(keys, vec!["BASEFORM", "CLASS", "STRUCTURE"]);
    }

    #[test]
    fn default_is_empty() {
        let a = Analysis::default();
        assert!(a.is_empty());
    }

    #[test]
    fn clone_is_independent() {
        let mut a = Analysis::new();
        a.set(ATTR_BASEFORM, "koira");
        let mut b = a.clone();
        b.set(ATTR_BASEFORM, "kissa");
        assert_eq!(a.get(ATTR_BASEFORM), Some("koira"));
        assert_eq!(b.get(ATTR_BASEFORM), Some("kissa"));
    }
}
