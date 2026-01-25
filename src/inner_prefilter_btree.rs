use roaring::RoaringBitmap;
use std::collections::BTreeMap;
use std::ops::Bound;

type Idx = u32;

#[derive(Debug, Clone)]
pub struct AhoCorasickPrefilter {
    prefixes: BTreeMap<Vec<u8>, RoaringBitmap>,
    first_idx: Idx,
}

impl AhoCorasickPrefilter {
    /// Builds a new prefilter from patterns and their corresponding matcher indexes.
    ///
    /// Returns [`None`] if patterns is empty or if the automaton fails to build.
    pub fn new(patterns: &[Vec<u8>], pattern_indexes: Vec<Idx>) -> Option<Self> {
        assert_eq!(patterns.len(), pattern_indexes.len());
        if patterns.is_empty() {
            return None;
        }

        let first_idx = pattern_indexes[0];
        let mut prefixes = BTreeMap::new();

        for (pattern, idx) in patterns.iter().zip(pattern_indexes) {
            prefixes
                .entry(pattern.clone())
                .or_insert_with(RoaringBitmap::new)
                .insert(idx);
        }

        Some(Self {
            prefixes,
            first_idx,
        })
    }

    /// Checks bytes against the prefilter, returning a bitmap of possible matcher indexes.
    pub fn check(&self, bytes: &[u8]) -> RoaringBitmap {
        let mut possible_indexes = RoaringBitmap::new();
        let mut search_len = bytes.len();

        while search_len > 0 {
            let min_slice = &bytes[..1];
            let search_slice = &bytes[..search_len];
            let mut range = self
                .prefixes
                .range::<[u8], _>((Bound::Included(min_slice), Bound::Included(search_slice)));

            match range.next_back() {
                Some((found_key, indexes)) => {
                    if search_slice.starts_with(found_key) {
                        possible_indexes |= indexes;
                        search_len = found_key.len().saturating_sub(1);
                    } else {
                        let common_len = search_slice
                            .iter()
                            .zip(found_key.iter())
                            .take_while(|(a, b)| a == b)
                            .count();
                        search_len = common_len;
                    }
                }
                None => break,
            }
        }

        possible_indexes
    }

    /// Returns the first pattern index.
    ///
    /// This is guaranteed to exist because the prefilter requires at least one pattern.
    pub fn first_index(&self) -> Idx {
        self.first_idx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_patterns() {
        let patterns: Vec<Vec<u8>> = vec![];
        let indexes: Vec<u32> = vec![];
        let prefilter = AhoCorasickPrefilter::new(&patterns, indexes);
        assert!(prefilter.is_none());
    }

    #[test]
    fn test_simple_match() {
        let patterns = vec![b"/api/users".to_vec(), b"/api/posts".to_vec()];
        let indexes = vec![0, 1];
        let prefilter = AhoCorasickPrefilter::new(&patterns, indexes).unwrap();

        let result = prefilter.check(b"/api/users/123");
        assert!(result.contains(0));
        assert!(!result.contains(1));
    }

    #[test]
    fn test_overlapping_matches() {
        let patterns = vec![b"/api".to_vec(), b"/api/v1".to_vec()];
        let indexes = vec![0, 1];
        let prefilter = AhoCorasickPrefilter::new(&patterns, indexes).unwrap();

        let result = prefilter.check(b"/api/v1/users");
        assert!(result.contains(0));
        assert!(result.contains(1));
    }

    #[test]
    fn test_multiple_same_prefix() {
        let patterns = vec![b"/api".to_vec(), b"/api".to_vec(), b"/users".to_vec()];
        let indexes = vec![0, 1, 2];
        let prefilter = AhoCorasickPrefilter::new(&patterns, indexes).unwrap();

        let result = prefilter.check(b"/api/v1");
        assert!(result.contains(0));
        assert!(result.contains(1));
        assert!(!result.contains(2));
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
        let prefilter = AhoCorasickPrefilter::new(&patterns, indexes).unwrap();

        let result = prefilter.check(b"/abc/def");
        assert!(result.contains(0));
        assert!(result.contains(1));
        assert!(result.contains(2));
        assert!(result.contains(3));

        let result = prefilter.check(b"/ab");
        assert!(result.contains(0));
        assert!(result.contains(1));
        assert!(result.contains(2));
        assert!(!result.contains(3));
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

        let prefilter = AhoCorasickPrefilter::new(&patterns, indexes).unwrap();
        let result = prefilter.check(b"/target/resource");

        assert!(result.contains(1000)); // "/" matches
        assert!(result.contains(1001)); // "/target" matches
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
        let prefilter = AhoCorasickPrefilter::new(&patterns, indexes).unwrap();

        let result = prefilter.check(b"/api/users/123");
        assert!(result.contains(0)); // "/" matches
        assert!(result.contains(1)); // "/api" matches
        assert!(!result.contains(2)); // "/api/v999" doesn't match
        assert!(!result.contains(3)); // "/other" doesn't match
        assert_eq!(result.len(), 2);
    }
}
