use fst::{Automaton, IntoStreamer, Set, Streamer};
use roaring::RoaringBitmap;
use std::collections::HashMap;

type Idx = u32;

#[derive(Debug, Clone)]
pub struct AhoCorasickPrefilter {
    // FST containing all prefixes (sorted)
    fst: Set<Vec<u8>>,
    // Map from prefix to the set of matcher indexes
    prefix_to_indexes: HashMap<Vec<u8>, RoaringBitmap>,
    first_idx: Idx,
}

impl AhoCorasickPrefilter {
    /// Builds a new prefilter from patterns and their corresponding matcher indexes.
    ///
    /// Returns [`None`] if patterns is empty or if the FST fails to build.
    pub fn new(patterns: &[Vec<u8>], pattern_indexes: Vec<Idx>) -> Option<Self> {
        assert_eq!(patterns.len(), pattern_indexes.len());
        if patterns.is_empty() {
            return None;
        }

        let first_idx = pattern_indexes[0];

        // Build the prefix -> indexes map
        let mut prefix_to_indexes: HashMap<Vec<u8>, RoaringBitmap> = HashMap::new();
        for (pattern, idx) in patterns.iter().zip(pattern_indexes) {
            prefix_to_indexes
                .entry(pattern.clone())
                .or_insert_with(RoaringBitmap::new)
                .insert(idx);
        }

        let fst = Set::from_iter(patterns).ok()?;

        Some(Self {
            fst,
            prefix_to_indexes,
            first_idx,
        })
    }

    /// Checks bytes against the prefilter, returning a bitmap of possible matcher indexes.
    pub fn check(&self, bytes: &[u8]) -> RoaringBitmap {
        let mut possible_indexes = RoaringBitmap::new();

        // Use custom automaton to find all FST keys that are prefixes of bytes
        let automaton = PrefixFinder::new(bytes);
        let mut stream = self.fst.search(automaton).into_stream();

        while let Some(prefix) = stream.next() {
            if let Some(indexes) = self.prefix_to_indexes.get(prefix) {
                possible_indexes |= indexes;
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

/// Custom automaton that finds all FST keys that are prefixes of the input bytes.
struct PrefixFinder<'a> {
    input: &'a [u8],
}

impl<'a> PrefixFinder<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self { input }
    }
}

impl<'a> Automaton for PrefixFinder<'a> {
    type State = usize;

    fn start(&self) -> Self::State {
        0
    }

    fn is_match(&self, state: &Self::State) -> bool {
        // Every state is a potential match since we want to collect
        // all prefixes. The FST itself determines which states are
        // final (represent complete keys).
        // We always return true here and let the FST filter.
        *state <= self.input.len()
    }

    fn accept(&self, state: &Self::State, byte: u8) -> Self::State {
        // If we've already consumed all input, reject
        let Some(&actual_byte) = self.input.get(*state) else {
            return usize::MAX; // Reject state
        };

        // Only accept if the next byte in input matches
        if byte == actual_byte {
            state + 1
        } else {
            usize::MAX // Reject state
        }
    }

    fn can_match(&self, state: &Self::State) -> bool {
        *state <= self.input.len()
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
        let patterns = vec![
            b"/api/users".to_vec(),
            b"/api/posts".to_vec(),
        ];
        let indexes = vec![0, 1];
        let prefilter = AhoCorasickPrefilter::new(&patterns, indexes).unwrap();

        let result = prefilter.check(b"/api/users/123");
        assert!(result.contains(0));
        assert!(!result.contains(1));
    }

    #[test]
    fn test_overlapping_matches() {
        let patterns = vec![
            b"/api".to_vec(),
            b"/api/v1".to_vec(),
        ];
        let indexes = vec![0, 1];
        let prefilter = AhoCorasickPrefilter::new(&patterns, indexes).unwrap();

        let result = prefilter.check(b"/api/v1/users");
        assert!(result.contains(0));
        assert!(result.contains(1));
    }

    #[test]
    fn test_multiple_same_prefix() {
        let patterns = vec![
            b"/api".to_vec(),
            b"/api".to_vec(),
            b"/users".to_vec(),
        ];
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
        // Test that the automaton works correctly with various prefixes
        let patterns = vec![
            b"/".to_vec(),
            b"/api".to_vec(),
            b"/api/v999".to_vec(),  // Won't match but helps test the automaton
            b"/other".to_vec(),
        ];
        let indexes = vec![0, 1, 2, 3];
        let prefilter = AhoCorasickPrefilter::new(&patterns, indexes).unwrap();

        let result = prefilter.check(b"/api/users/123");
        assert!(result.contains(0));  // "/" matches
        assert!(result.contains(1));  // "/api" matches
        assert!(!result.contains(2)); // "/api/v999" doesn't match
        assert!(!result.contains(3)); // "/other" doesn't match
        assert_eq!(result.len(), 2);
    }
}
