use bstr::{BStr, BString, ByteSlice};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Bound;

/// Internal prefix lookup structure using a BTreeMap for efficient range queries.
///
/// Stores prefixes mapped to bitmaps of matcher indexes, with automatic
/// prefix extension to handle nested prefix relationships.
#[derive(Debug, Clone)]
pub struct InnerPrefilter<K> {
    prefixes: BTreeMap<BString, BTreeSet<K>>,
    key_to_prefixes: BTreeMap<K, Vec<BString>>,
}

impl<K> Default for InnerPrefilter<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K> InnerPrefilter<K> {
    pub fn new() -> Self {
        Self {
            prefixes: BTreeMap::new(),
            key_to_prefixes: BTreeMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.key_to_prefixes.is_empty()
    }
}

impl<K: Ord> InnerPrefilter<K> {
    pub fn insert(&mut self, key: K, prefixes: Vec<Vec<u8>>)
    where
        K: Clone,
    {
        let prefixes: Vec<BString> = prefixes.into_iter().map(BString::new).collect();
        self.key_to_prefixes.insert(key.clone(), prefixes.clone());
        for prefix in prefixes {
            recursively_add_to_longer_prefixes(prefix, key.clone(), &mut self.prefixes);
        }
    }

    pub fn remove(&mut self, key: &K) {
        let Some(prefixes) = self.key_to_prefixes.remove(key) else {
            return;
        };
        for prefix in prefixes {
            recursively_remove_from_longer_prefixes(prefix.as_bstr(), key, &mut self.prefixes);
        }
    }

    /// Checks bytes against the prefilter, returning a bitmap of possible matcher indexes.
    pub fn check(&self, bytes: &[u8]) -> Option<&BTreeSet<K>> {
        longest_contained_prefix(BStr::new(bytes), &self.prefixes).map(|(_prefix, keys)| keys)
    }
}

fn longest_contained_prefix<'a, K: Ord>(
    value: &BStr,
    prefixes: &'a BTreeMap<BString, BTreeSet<K>>,
) -> Option<(&'a BStr, &'a BTreeSet<K>)> {
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
            return Some((found_key.as_bstr(), indexes));
        }

        upper_bound = &value[..common_len];
    }
}

fn recursively_add_to_longer_prefixes<K: Ord + Clone>(
    prefix: BString,
    key: K,
    prefixes: &mut BTreeMap<BString, BTreeSet<K>>,
) {
    let mut keys_from_shorter_prefixes = BTreeSet::new();
    keys_from_shorter_prefixes.insert(key.clone());

    let mut upper_bound = prefix.as_bstr();
    while let Some((_, smaller_prefix)) = upper_bound.split_last() {
        upper_bound = match longest_contained_prefix(smaller_prefix.as_bstr(), prefixes) {
            Some((smaller_prefix, keys)) => {
                keys_from_shorter_prefixes.extend(keys.iter().cloned());
                smaller_prefix
            }
            None => break,
        };
    }

    let range =
        prefixes.range_mut::<BStr, _>((Bound::Excluded(prefix.as_bstr()), Bound::Unbounded));
    for (longer_prefix, keys) in range {
        if !longer_prefix.starts_with(&prefix) {
            break;
        }
        keys.insert(key.clone());
    }
    prefixes
        .entry(prefix)
        .or_default()
        .extend(keys_from_shorter_prefixes);
}
fn recursively_remove_from_longer_prefixes<K: Ord>(
    prefix: &BStr,
    key: &K,
    prefixes: &mut BTreeMap<BString, BTreeSet<K>>,
) {
    let range =
        prefixes.range_mut::<BStr, _>((Bound::Included(prefix.as_bstr()), Bound::Unbounded));
    let mut to_remove = Vec::new();
    for (longer_prefix, keys) in range {
        if !longer_prefix.starts_with(prefix) {
            break;
        }
        let did_remove = keys.remove(key);
        debug_assert!(did_remove);
        if keys.is_empty() {
            to_remove.push(longer_prefix.clone());
        }
    }
    for longer_prefix in to_remove {
        prefixes.remove(&longer_prefix);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
