use bstr::{BStr, BString, ByteSlice};
use std::collections::{BTreeMap, BTreeSet};
use std::iter;
use std::ops::Bound;

/// A map that maintains the prefix-inheritance invariant: for every string A in the map,
/// every other string that has A as a prefix contains all the K values associated with A.
#[derive(Debug, Clone)]
pub(crate) struct PrefixInheritanceMap<K> {
    prefixes: BTreeMap<BString, BTreeSet<K>>,
}

impl<K> PrefixInheritanceMap<K> {
    fn new() -> Self {
        Self {
            prefixes: BTreeMap::new(),
        }
    }

    fn clear(&mut self) {
        self.prefixes.clear();
    }
}

impl<K: Ord> PrefixInheritanceMap<K> {
    /// Finds the longest prefix in the map that matches the given value.
    ///
    /// Returns a tuple of (matched_prefix, associated_keys) if a match is found.
    pub(crate) fn longest_match(&self, value: &BStr) -> Option<(&BStr, &BTreeSet<K>)> {
        let mut upper_bound = value;

        loop {
            let (found_key, indexes) = self
                .prefixes
                .range::<BStr, _>((Bound::Unbounded, Bound::Included(upper_bound)))
                .next_back()?;

            let common_len = value
                .iter()
                .zip(found_key.iter())
                .take_while(|(a, b)| a == b)
                .count();

            if common_len == 0 {
                return None;
            }
            if common_len == found_key.len() {
                return Some((found_key.as_bstr(), indexes));
            }

            upper_bound = &value[..common_len];
        }
    }

    /// Inserts a key associated with the given prefix.
    pub(crate) fn insert(&mut self, prefix: BString, key: K)
    where
        K: Clone,
    {
        // Find all prefixes which have this prefix as a prefix, and add the key to their sets
        let range = self
            .prefixes
            .range_mut::<BStr, _>((Bound::Excluded(prefix.as_bstr()), Bound::Unbounded));
        for (longer_prefix, keys) in range {
            if !longer_prefix.starts_with(&prefix) {
                break;
            }
            keys.insert(key.clone());
        }

        let mut keys_from_shorter_prefixes = BTreeSet::new();
        keys_from_shorter_prefixes.insert(key);

        // Find the largest prefix which is itself a prefix of this prefix
        // to add to the keys for this prefix.
        // The keys for all prefixes shorter will already be present in the keys for that prefix
        if let Some((_, smaller_prefix)) = prefix.as_bstr().split_last()
            && let Some((_, keys)) = self.longest_match(smaller_prefix.as_bstr())
        {
            keys_from_shorter_prefixes.extend(keys.iter().cloned());
        }
        self.prefixes
            .entry(prefix)
            .or_default()
            .extend(keys_from_shorter_prefixes);
    }

    /// Removes a key from the given prefix and all longer prefixes.
    ///
    /// This method maintains the prefix-inheritance invariant by removing the key
    /// from all prefixes that start with the given prefix. Empty prefix entries
    /// are automatically cleaned up.
    pub(crate) fn remove(&mut self, prefix: &BStr, key: &K) {
        let range = self
            .prefixes
            .range_mut::<BStr, _>((Bound::Included(prefix.as_bstr()), Bound::Unbounded));
        let mut newly_empty_prefixes = Vec::new();
        for (longer_prefix, keys) in range {
            if !longer_prefix.starts_with(prefix) {
                break;
            }
            let did_remove = keys.remove(key);
            debug_assert!(did_remove);
            if keys.is_empty() {
                newly_empty_prefixes.push(longer_prefix.clone());
            }
        }
        for longer_prefix in newly_empty_prefixes {
            self.prefixes.remove(&longer_prefix);
        }
    }
}

/// Internal prefix lookup structure using a `BTreeMap` for efficient range queries.
///
/// Stores prefixes mapped to bitmaps of matcher indexes, with automatic
/// prefix extension to handle nested prefix relationships.
#[derive(Debug, Clone)]
pub(crate) struct InnerPrefilter<K> {
    prefix_map: PrefixInheritanceMap<K>,
    key_to_prefixes: BTreeMap<K, Vec<BString>>,
}

impl<K> InnerPrefilter<K> {
    pub(crate) fn new() -> Self {
        Self {
            prefix_map: PrefixInheritanceMap::new(),
            key_to_prefixes: BTreeMap::new(),
        }
    }

    /// Returns true if the prefilter contains no keys.
    pub(crate) fn is_empty(&self) -> bool {
        self.key_to_prefixes.is_empty()
    }

    /// Returns the number of routes in the prefilter.
    pub(crate) fn num_routes(&self) -> usize {
        self.key_to_prefixes.len()
    }
}

impl<K: Ord> InnerPrefilter<K> {
    /// Inserts a key with the given prefixes into the prefilter.
    ///
    /// Each prefix is added to the prefix map, maintaining the prefix-inheritance invariant.
    ///
    /// No prefix in `prefixes` may be a prefix of another entry in `prefixes`.
    /// This precondition is upheld by the caller (`MatcherVisitor::finish`
    /// applies `optimize_for_prefix_by_preference`, which collapses such
    /// overlapping literals). Violating this causes `remove` to trip a
    /// debug assertion.
    pub(crate) fn insert(&mut self, key: K, prefixes: Vec<Vec<u8>>)
    where
        K: Clone,
    {
        let prefixes: Vec<BString> = prefixes.into_iter().map(BString::new).collect();
        if let Some(old_prefixes) = self.key_to_prefixes.insert(key.clone(), prefixes.clone()) {
            for prefix in old_prefixes {
                self.prefix_map.remove(prefix.as_bstr(), &key);
            }
        }
        let prefixes_len = prefixes.len();
        // Use repeat_n to avoid cloning the last iteration
        for (prefix, key) in prefixes.into_iter().zip(iter::repeat_n(key, prefixes_len)) {
            self.prefix_map.insert(prefix, key);
        }
    }

    /// Removes a key and all its associated prefixes from the prefilter.
    pub(crate) fn remove(&mut self, key: &K) {
        let Some(prefixes) = self.key_to_prefixes.remove(key) else {
            return;
        };
        for prefix in prefixes {
            self.prefix_map.remove(prefix.as_bstr(), key);
        }
    }

    pub(crate) fn clear(&mut self) {
        self.key_to_prefixes.clear();
        self.prefix_map.clear()
    }

    /// Checks bytes against the prefilter, returning a bitmap of possible matcher indexes.
    pub(crate) fn check(&self, bytes: &[u8]) -> Option<&BTreeSet<K>> {
        self.prefix_map
            .longest_match(BStr::new(bytes))
            .map(|(_prefix, keys)| keys)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefix_inheritance_map_basic() {
        let mut map = PrefixInheritanceMap::new();
        map.insert(BString::from("/api"), 1);
        map.insert(BString::from("/api/v1"), 2);

        // Looking up "/api/v1/users" returns both keys 1 and 2
        let result = map.longest_match(BStr::new("/api/v1/users".as_bytes()));
        assert!(result.is_some());
        let (prefix, keys) = result.unwrap();
        assert_eq!(prefix, "/api/v1".as_bytes());
        assert!(keys.contains(&1));
        assert!(keys.contains(&2));
    }

    #[test]
    fn test_empty_patterns() {
        let prefilter = InnerPrefilter::<u8>::new();
        assert_eq!(prefilter.check(b""), None);
    }

    #[test]
    fn test_simple_match() {
        let patterns = vec![b"/api/users".to_vec(), b"/api/posts".to_vec()];
        let mut prefilter = InnerPrefilter::new();
        for (i, pattern) in patterns.into_iter().enumerate() {
            prefilter.insert(i, vec![pattern]);
        }

        let result = prefilter.check(b"/api/users/123").unwrap();
        assert!(result.contains(&0));
        assert!(!result.contains(&1));
    }

    #[test]
    fn test_overlapping_matches() {
        let patterns = vec![b"/api".to_vec(), b"/api/v1".to_vec()];
        let indexes = vec![0, 1];
        let mut prefilter = InnerPrefilter::new();
        for (index, pattern) in indexes.into_iter().zip(patterns.into_iter()) {
            prefilter.insert(index, vec![pattern]);
        }

        let result = prefilter.check(b"/api/v1/users").unwrap();
        assert!(result.contains(&0));
        assert!(result.contains(&1));
    }

    #[test]
    fn test_multiple_same_prefix() {
        let patterns = vec![b"/api".to_vec(), b"/api".to_vec(), b"/users".to_vec()];
        let indexes = vec![0, 1, 2];
        let mut prefilter = InnerPrefilter::new();
        for (index, pattern) in indexes.into_iter().zip(patterns.into_iter()) {
            prefilter.insert(index, vec![pattern]);
        }

        let result = prefilter.check(b"/api/v1").unwrap();
        assert!(result.contains(&0));
        assert!(result.contains(&1));
        assert!(!result.contains(&2));
    }

    #[test]
    fn test_nested_prefixes() {
        let patterns = vec![
            b"/".to_vec(),
            b"/a".to_vec(),
            b"/ab".to_vec(),
            b"/abc".to_vec(),
        ];
        let indexes = vec![0, 1, 2, 3];
        let mut prefilter = InnerPrefilter::new();
        for (index, pattern) in indexes.into_iter().zip(patterns.into_iter()) {
            prefilter.insert(index, vec![pattern]);
        }

        let result = prefilter.check(b"/abc/def").unwrap();
        assert!(result.contains(&0));
        assert!(result.contains(&1));
        assert!(result.contains(&2));
        assert!(result.contains(&3));

        let result = prefilter.check(b"/ab").unwrap();
        assert!(result.contains(&0));
        assert!(result.contains(&1));
        assert!(result.contains(&2));
        assert!(!result.contains(&3));
    }

    #[test]
    fn test_sparse_prefixes_efficiency() {
        // Create a sparse set with many non-matching prefixes
        let mut patterns = vec![];
        let mut indexes = vec![];

        // Add many decoy patterns
        for i in 0..100 {
            patterns.push(format!("/decoy{:03}", i).into_bytes());
            indexes.push(i);
        }

        // Add actual matching patterns
        patterns.push(b"/".to_vec());
        patterns.push(b"/target".to_vec());
        indexes.push(1000);
        indexes.push(1001);

        let mut prefilter = InnerPrefilter::new();
        for (index, pattern) in indexes.into_iter().zip(patterns.into_iter()) {
            prefilter.insert(index, vec![pattern]);
        }
        let result = prefilter.check(b"/target/resource").unwrap();

        assert!(result.contains(&1000)); // "/" matches
        assert!(result.contains(&1001)); // "/target" matches
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_common_prefix_skipping() {
        // Test that common prefix analysis works correctly
        let patterns = vec![
            b"/".to_vec(),
            b"/api".to_vec(),
            b"/api/v999".to_vec(), // Won't match but helps test skipping
            b"/other".to_vec(),
        ];
        let indexes = vec![0, 1, 2, 3];
        let mut prefilter = InnerPrefilter::new();
        for (index, pattern) in indexes.into_iter().zip(patterns.into_iter()) {
            prefilter.insert(index, vec![pattern]);
        }

        let result = prefilter.check(b"/api/users/123").unwrap();
        assert!(result.contains(&0)); // "/" matches
        assert!(result.contains(&1)); // "/api" matches
        assert!(!result.contains(&2)); // "/api/v999" doesn't match
        assert!(!result.contains(&3)); // "/other" doesn't match
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_remove() {
        let mut prefilter = InnerPrefilter::new();
        prefilter.insert(0, vec![b"/api".to_vec()]);
        prefilter.insert(1, vec![b"/api/v1".to_vec()]);
        prefilter.insert(2, vec![b"/users".to_vec()]);

        assert!(!prefilter.is_empty());
        assert_eq!(prefilter.num_routes(), 3);

        // Remove a route
        prefilter.remove(&1);
        assert_eq!(prefilter.num_routes(), 2);

        // Verify it's gone
        let result = prefilter.check(b"/api/v1/users").unwrap();
        assert!(result.contains(&0)); // "/api" still matches
        assert!(!result.contains(&1)); // "/api/v1" was removed

        // Remove all routes
        prefilter.remove(&0);
        prefilter.remove(&2);
        assert!(prefilter.is_empty());
    }

    #[test]
    fn test_prefix_map_remove() {
        let mut map = PrefixInheritanceMap::new();
        map.insert(BString::from("/api"), 1);
        map.insert(BString::from("/api/v1"), 2);
        map.insert(BString::from("/api/v1/users"), 3);

        // All three should match initially
        let result = map.longest_match(BStr::new(b"/api/v1/users/123"));
        assert!(result.is_some());
        let (_, keys) = result.unwrap();
        assert!(keys.contains(&1));
        assert!(keys.contains(&2));
        assert!(keys.contains(&3));

        // Remove from middle prefix
        map.remove(BStr::new(b"/api/v1"), &2);

        // Key 2 should be removed from /api/v1 and /api/v1/users
        let result = map.longest_match(BStr::new(b"/api/v1/users/123"));
        assert!(result.is_some());
        let (_, keys) = result.unwrap();
        assert!(keys.contains(&1)); // Still has key from /api
        assert!(!keys.contains(&2)); // Removed
        assert!(keys.contains(&3)); // Still has its own key

        // Remove last key from a prefix
        map.remove(BStr::new(b"/api/v1/users"), &1);
        map.remove(BStr::new(b"/api/v1/users"), &3);

        // /api/v1/users should be removed entirely since it's empty
        let result = map.longest_match(BStr::new(b"/api/v1/users/123"));
        assert!(result.is_some());
        let (prefix, _) = result.unwrap();
        // Should only match /api/v1 now (which still has key 1 from /api)
        assert_eq!(prefix, b"/api/v1" as &[u8]);
    }

    #[test]
    fn test_no_common_prefix() {
        let mut map = PrefixInheritanceMap::new();
        map.insert(BString::from("zzz"), 1);

        // Search for something that shares no common prefix
        let result = map.longest_match(BStr::new(b"aaa"));
        assert_eq!(result, None);
    }

    #[test]
    fn test_is_empty_and_num_routes() {
        let mut prefilter = InnerPrefilter::new();
        assert!(prefilter.is_empty());
        assert_eq!(prefilter.num_routes(), 0);

        prefilter.insert(0, vec![b"/api".to_vec()]);
        assert!(!prefilter.is_empty());
        assert_eq!(prefilter.num_routes(), 1);

        prefilter.insert(1, vec![b"/users".to_vec()]);
        assert_eq!(prefilter.num_routes(), 2);

        prefilter.remove(&0);
        assert_eq!(prefilter.num_routes(), 1);

        prefilter.remove(&1);
        assert!(prefilter.is_empty());
        assert_eq!(prefilter.num_routes(), 0);
    }
}
