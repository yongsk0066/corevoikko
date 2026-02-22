// Grammar checker cache for paragraph-level results
// Origin: grammar/GcCache.hpp, GcCache.cpp, CacheEntry.hpp, CacheEntry.cpp

use voikko_core::grammar_error::GrammarError;

// ---------------------------------------------------------------------------
// GcCache
// Origin: grammar/GcCache.hpp:39-57, GcCache.cpp:34-75
// ---------------------------------------------------------------------------

/// Grammar checker cache.
///
/// Caches grammar check results for a single paragraph. If the same paragraph
/// text is checked again, the cached errors are returned without re-running
/// the grammar rules.
///
/// The C++ implementation stores a single paragraph as a linked list of
/// `CacheEntry` nodes sorted by start position. In Rust we use a `Vec` of
/// `GrammarError` sorted by `start_pos`, and store the paragraph text as a
/// `Vec<char>` for direct comparison.
///
/// Origin: grammar/GcCache.hpp:39-57
pub(crate) struct GcCache {
    /// The cached paragraph text, or `None` if the cache is empty.
    paragraph: Option<Vec<char>>,

    /// Cached grammar errors for the paragraph, sorted by `start_pos`.
    errors: Vec<GrammarError>,
}

impl GcCache {
    /// Create an empty cache.
    ///
    /// Origin: GcCache.cpp:34-37
    pub fn new() -> Self {
        Self {
            paragraph: None,
            errors: Vec::new(),
        }
    }

    /// Clear the cache, discarding the stored paragraph and errors.
    ///
    /// Origin: GcCache.cpp:39-49
    pub fn clear(&mut self) {
        self.paragraph = None;
        self.errors.clear();
    }

    /// Check whether the cache contains results for the given paragraph text.
    ///
    /// Returns `Some(&[GrammarError])` if the cached text matches, `None`
    /// otherwise.
    pub fn check_cache(&self, text: &[char]) -> Option<&[GrammarError]> {
        match &self.paragraph {
            Some(cached) if cached.as_slice() == text => Some(&self.errors),
            _ => None,
        }
    }

    /// Store grammar check results for a paragraph.
    ///
    /// Replaces any previously cached paragraph. The errors are stored sorted
    /// by `start_pos`.
    pub fn store_cache(&mut self, text: &[char], mut errors: Vec<GrammarError>) {
        errors.sort_by_key(|e| e.start_pos);
        self.paragraph = Some(text.to_vec());
        self.errors = errors;
    }

    /// Append a single error to the cached results, maintaining sorted order
    /// by `start_pos`.
    ///
    /// This mirrors the C++ `GcCache::appendError` which inserts into a
    /// sorted linked list.
    ///
    /// Origin: GcCache.cpp:51-75
    pub fn append_error(&mut self, error: GrammarError) {
        let insert_pos = self
            .errors
            .partition_point(|e| e.start_pos <= error.start_pos);
        self.errors.insert(insert_pos, error);
    }

    /// Return the number of cached errors.
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// Return whether the cache has a stored paragraph.
    pub fn is_empty(&self) -> bool {
        self.paragraph.is_none()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn new_cache_is_empty() {
        let cache = GcCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.error_count(), 0);
    }

    #[test]
    fn check_empty_cache_returns_none() {
        let cache = GcCache::new();
        assert!(cache.check_cache(&chars("hello")).is_none());
    }

    #[test]
    fn store_and_check_same_text() {
        let mut cache = GcCache::new();
        let text = chars("Koira juoksi.");
        let errors = vec![GrammarError::new(1, 0, 5)];
        cache.store_cache(&text, errors.clone());

        let result = cache.check_cache(&text);
        assert!(result.is_some());
        let cached_errors = result.unwrap();
        assert_eq!(cached_errors.len(), 1);
        assert_eq!(cached_errors[0].error_code, 1);
        assert_eq!(cached_errors[0].start_pos, 0);
    }

    #[test]
    fn check_different_text_returns_none() {
        let mut cache = GcCache::new();
        let text1 = chars("Koira juoksi.");
        let text2 = chars("Kissa nukkui.");
        cache.store_cache(&text1, vec![]);

        assert!(cache.check_cache(&text2).is_none());
    }

    #[test]
    fn store_replaces_previous() {
        let mut cache = GcCache::new();
        let text1 = chars("First.");
        let text2 = chars("Second.");
        cache.store_cache(&text1, vec![GrammarError::new(1, 0, 5)]);
        cache.store_cache(&text2, vec![GrammarError::new(2, 0, 6)]);

        assert!(cache.check_cache(&text1).is_none());
        let result = cache.check_cache(&text2).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].error_code, 2);
    }

    #[test]
    fn clear_empties_cache() {
        let mut cache = GcCache::new();
        let text = chars("Koira.");
        cache.store_cache(&text, vec![GrammarError::new(1, 0, 5)]);
        assert!(!cache.is_empty());

        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.error_count(), 0);
        assert!(cache.check_cache(&text).is_none());
    }

    #[test]
    fn store_sorts_errors_by_start_pos() {
        let mut cache = GcCache::new();
        let text = chars("Koira juoksi nopeasti.");
        let errors = vec![
            GrammarError::new(1, 15, 3),
            GrammarError::new(2, 0, 5),
            GrammarError::new(3, 6, 6),
        ];
        cache.store_cache(&text, errors);

        let result = cache.check_cache(&text).unwrap();
        assert_eq!(result[0].start_pos, 0);
        assert_eq!(result[1].start_pos, 6);
        assert_eq!(result[2].start_pos, 15);
    }

    #[test]
    fn append_error_maintains_sorted_order() {
        let mut cache = GcCache::new();
        let text = chars("Koira juoksi.");
        cache.store_cache(&text, vec![
            GrammarError::new(1, 0, 5),
            GrammarError::new(3, 10, 3),
        ]);

        // Insert in the middle.
        cache.append_error(GrammarError::new(2, 6, 4));

        let result = cache.check_cache(&text).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].start_pos, 0);
        assert_eq!(result[1].start_pos, 6);
        assert_eq!(result[2].start_pos, 10);
    }

    #[test]
    fn append_error_at_beginning() {
        let mut cache = GcCache::new();
        let text = chars("Hello.");
        cache.store_cache(&text, vec![GrammarError::new(1, 5, 1)]);

        cache.append_error(GrammarError::new(2, 0, 3));

        let result = cache.check_cache(&text).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].start_pos, 0);
        assert_eq!(result[1].start_pos, 5);
    }

    #[test]
    fn append_error_at_end() {
        let mut cache = GcCache::new();
        let text = chars("Hello.");
        cache.store_cache(&text, vec![GrammarError::new(1, 0, 3)]);

        cache.append_error(GrammarError::new(2, 5, 1));

        let result = cache.check_cache(&text).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].start_pos, 0);
        assert_eq!(result[1].start_pos, 5);
    }

    #[test]
    fn append_error_with_same_start_pos() {
        let mut cache = GcCache::new();
        let text = chars("Hello.");
        cache.store_cache(&text, vec![GrammarError::new(1, 0, 3)]);

        cache.append_error(GrammarError::new(2, 0, 5));

        let result = cache.check_cache(&text).unwrap();
        assert_eq!(result.len(), 2);
        // Both at position 0; order is: existing first, then appended.
        assert_eq!(result[0].error_code, 1);
        assert_eq!(result[1].error_code, 2);
    }

    #[test]
    fn check_cache_empty_text() {
        let mut cache = GcCache::new();
        let empty: Vec<char> = Vec::new();
        cache.store_cache(&empty, vec![]);

        let result = cache.check_cache(&empty);
        assert!(result.is_some());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn store_with_no_errors() {
        let mut cache = GcCache::new();
        let text = chars("Clean paragraph.");
        cache.store_cache(&text, vec![]);

        let result = cache.check_cache(&text).unwrap();
        assert!(result.is_empty());
        assert!(!cache.is_empty());
    }
}
