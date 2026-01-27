use super::Idx;
use bstr::{BStr, BString};
use roaring::RoaringBitmap;
use std::collections::BTreeMap;
use std::ops::Bound;

#[derive(Debug, Clone)]
pub struct InnerPrefilter {
    prefixes: BTreeMap<BString, RoaringBitmap>,
}

impl InnerPrefilter {
    /// Builds a new prefilter from patterns and their corresponding matcher indexes.
    ///
    /// Returns [`None`] if patterns is empty or if the automaton fails to build.
    pub fn new(patterns: &[Vec<u8>], pattern_indexes: Vec<Idx>) -> Option<Self> {
        debug_assert_eq!(patterns.len(), pattern_indexes.len());
        debug_assert!(pattern_indexes.is_sorted());
        if patterns.is_empty() {
            return None;
        }

        let mut prefixes = BTreeMap::new();

        for (pattern, idx) in patterns.iter().zip(pattern_indexes) {
            prefixes
                .entry(BString::new(pattern.clone()))
                .or_insert_with(RoaringBitmap::new)
                .insert(idx);
        }

        recursively_extend_further_prefixes(&mut prefixes);

        Some(Self { prefixes })
    }

    /// Checks bytes against the prefilter, returning a bitmap of possible matcher indexes.
    pub fn check(&self, bytes: &[u8]) -> Option<&RoaringBitmap> {
        longest_contained_prefix(BStr::new(bytes), &self.prefixes)
    }
}

fn longest_contained_prefix<'a>(
    value: &BStr,
    prefixes: &'a BTreeMap<BString, RoaringBitmap>,
) -> Option<&'a RoaringBitmap> {
    let mut upper_bound = value;

    loop {
        let (found_key, indexes) = prefixes
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
            return Some(indexes);
        }

        upper_bound = &value[..common_len];
    }
}

/// Recursively extends the values from shorter prefixes to longer prefixes.
///
/// e.g. with the prefixes:
/// "abc" => [1]
/// "abcd" => [2]
/// "abcde" => [3]
/// "abcz" => [4]
///
/// will be merged to:
/// "abc" => [1]
/// "abcd" => [1, 2]
/// "abcde" => [1, 2, 3]
/// "abcz" => [1, 4]
fn recursively_extend_further_prefixes(prefixes: &mut BTreeMap<BString, RoaringBitmap>) {
    let Some(mut current_key) = prefixes.keys().next().cloned() else {
        return;
    };
    loop {
        if let Some(values) = current_key
            .len()
            .checked_sub(1)
            .and_then(|len| longest_contained_prefix(BStr::new(&current_key[..len]), prefixes))
        {
            let values = values.clone();
            *prefixes.get_mut(&current_key).unwrap() |= values;
        }

        match prefixes
            .range((Bound::Excluded(current_key), Bound::Unbounded))
            .next()
            .map(|(k, _)| k.clone())
        {
            Some(key) => current_key = key,
            None => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_patterns() {
        let patterns: Vec<Vec<u8>> = vec![];
        let indexes: Vec<u32> = vec![];
        let prefilter = InnerPrefilter::new(&patterns, indexes);
        assert!(prefilter.is_none());
    }

    #[test]
    fn test_simple_match() {
        let patterns = vec![b"/api/users".to_vec(), b"/api/posts".to_vec()];
        let indexes = vec![0, 1];
        let prefilter = InnerPrefilter::new(&patterns, indexes).unwrap();

        let result = prefilter.check(b"/api/users/123").unwrap();
        assert!(result.contains(0));
        assert!(!result.contains(1));
    }

    #[test]
    fn test_overlapping_matches() {
        let patterns = vec![b"/api".to_vec(), b"/api/v1".to_vec()];
        let indexes = vec![0, 1];
        let prefilter = InnerPrefilter::new(&patterns, indexes).unwrap();

        let result = prefilter.check(b"/api/v1/users").unwrap();
        assert!(result.contains(0));
        assert!(result.contains(1));
    }

    #[test]
    fn test_multiple_same_prefix() {
        let patterns = vec![b"/api".to_vec(), b"/api".to_vec(), b"/users".to_vec()];
        let indexes = vec![0, 1, 2];
        let prefilter = InnerPrefilter::new(&patterns, indexes).unwrap();

        let result = prefilter.check(b"/api/v1").unwrap();
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
        let prefilter = InnerPrefilter::new(&patterns, indexes).unwrap();

        let result = prefilter.check(b"/abc/def").unwrap();
        assert!(result.contains(0));
        assert!(result.contains(1));
        assert!(result.contains(2));
        assert!(result.contains(3));

        let result = prefilter.check(b"/ab").unwrap();
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

        let prefilter = InnerPrefilter::new(&patterns, indexes).unwrap();
        let result = prefilter.check(b"/target/resource").unwrap();

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
        let prefilter = InnerPrefilter::new(&patterns, indexes).unwrap();

        let result = prefilter.check(b"/api/users/123").unwrap();
        assert!(result.contains(0)); // "/" matches
        assert!(result.contains(1)); // "/api" matches
        assert!(!result.contains(2)); // "/api/v999" doesn't match
        assert!(!result.contains(3)); // "/other" doesn't match
        assert_eq!(result.len(), 2);
    }
}
