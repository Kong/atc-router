use bstr::BString;
use std::collections::{BTreeMap, BTreeSet};
use std::iter;
use std::mem;

#[derive(Debug, Clone)]
struct RadixTrie<K> {
    keys: BTreeSet<K>,
    children: Vec<RadixLink<K>>,
}

#[derive(Debug, Clone)]
struct RadixLink<K> {
    ch: u8,
    rest: BString,
    child: RadixTrie<K>,
}

impl<K> RadixTrie<K> {
    fn new() -> Self {
        Self {
            keys: BTreeSet::new(),
            children: Vec::new(),
        }
    }

    /// Find the child index whose edge label starts with the given byte.
    fn find_child(&self, byte: u8) -> Result<usize, usize> {
        self.children.binary_search_by(|link| link.ch.cmp(&byte))
    }
}

impl<K: Ord> RadixTrie<K> {
    fn insert(&mut self, mut prefix: &[u8], key: K) {
        let mut node = self;
        while let Some(&first_char) = prefix.split_off_first() {
            let idx = match node.find_child(first_char) {
                Ok(idx) => idx,
                Err(idx) => {
                    node.children.insert(
                        idx,
                        RadixLink {
                            ch: first_char,
                            rest: BString::new(prefix.to_vec()),
                            child: RadixTrie::new(),
                        },
                    );
                    node = &mut node.children[idx].child;
                    break;
                }
            };

            let link = &mut node.children[idx];
            let common_len = common_prefix_len(&link.rest, prefix);

            if common_len < link.rest.len() {
                split_link(link, common_len);
            }

            prefix = &prefix[common_len..];
            node = &mut node.children[idx].child;
        }
        node.keys.insert(key);
    }

    fn remove(&mut self, mut prefix: &[u8], key: &K) {
        let Some(&first_char) = prefix.split_off_first() else {
            self.keys.remove(key);
            return;
        };
        let Ok(idx) = self.find_child(first_char) else {
            return;
        };

        let link = &mut self.children[idx];
        let Some((prefix_rest_begin, prefix_rest)) = prefix.split_at_checked(link.rest.len())
        else {
            return;
        };
        if prefix_rest_begin != link.rest.as_slice() {
            return;
        }

        link.child.remove(prefix_rest, key);

        // Clean up empty nodes.
        if link.child.keys.is_empty() && link.child.children.is_empty() {
            self.children.remove(idx);
        } else {
            try_compact_link(&mut self.children[idx]);
        }
    }

    fn collect_prefix_matches(&self, mut input: &[u8]) -> BTreeSet<&K> {
        let mut result = BTreeSet::new();
        let mut node = self;
        loop {
            result.extend(&node.keys);

            let Some(&first_char) = input.split_off_first() else {
                break;
            };

            let Ok(idx) = node.find_child(first_char) else {
                break;
            };

            let link = &node.children[idx];
            let Some((input_rest_begin, input_rest)) = input.split_at_checked(link.rest.len())
            else {
                break;
            };
            if input_rest_begin != link.rest.as_slice() {
                break;
            }

            input = input_rest;
            node = &link.child;
        }
        result
    }
}

fn try_compact_link<K>(link: &mut RadixLink<K>) {
    if link.child.keys.is_empty() && link.child.children.len() == 1 {
        let grandchild = link.child.children.pop().unwrap();
        link.rest.reserve(1 + grandchild.rest.len());
        link.rest.push(grandchild.ch);
        link.rest.extend_from_slice(&grandchild.rest);
        link.child = grandchild.child;
    }
}

fn split_link<K>(link: &mut RadixLink<K>, at: usize) {
    let tail = link.rest.split_off(at + 1);
    let ch = link.rest.pop().unwrap();
    let old_child = mem::replace(&mut link.child, RadixTrie::new());
    link.child.children.push(RadixLink {
        ch,
        rest: BString::new(tail),
        child: old_child,
    });
}

fn common_prefix_len(lhs: &[u8], rhs: &[u8]) -> usize {
    lhs.iter().zip(rhs).take_while(|(a, b)| a == b).count()
}

/// Internal prefix lookup structure using a radix trie for efficient prefix matching.
///
/// Stores prefixes mapped to sets of keys, with a reverse index for removal.
#[derive(Debug, Clone)]
pub(crate) struct InnerPrefilter<K> {
    prefix_map: RadixTrie<K>,
    key_to_prefixes: BTreeMap<K, Vec<BString>>,
}

impl<K> InnerPrefilter<K> {
    pub(crate) fn new() -> Self {
        Self {
            prefix_map: RadixTrie::new(),
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
                self.prefix_map.remove(&prefix, &key);
            }
        }
        let prefixes_len = prefixes.len();
        // Use repeat_n to avoid cloning the last iteration
        for (prefix, key) in prefixes.into_iter().zip(iter::repeat_n(key, prefixes_len)) {
            self.prefix_map.insert(&prefix, key);
        }
    }

    /// Removes a key and all its associated prefixes from the prefilter.
    pub(crate) fn remove(&mut self, key: &K) {
        let Some(prefixes) = self.key_to_prefixes.remove(key) else {
            return;
        };
        for prefix in prefixes {
            self.prefix_map.remove(&prefix, key);
        }
    }

    pub(crate) fn clear(&mut self) {
        self.key_to_prefixes.clear();
        self.prefix_map = RadixTrie::new();
    }

    /// Checks bytes against the prefilter, returning a set of possible matcher keys.
    pub(crate) fn check(&self, bytes: &[u8]) -> BTreeSet<&K> {
        self.prefix_map.collect_prefix_matches(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_patterns() {
        let prefilter = InnerPrefilter::<u8>::new();
        assert_eq!(prefilter.check(b""), BTreeSet::new());
    }

    #[test]
    fn test_simple_match() {
        let patterns = vec![b"/api/users".to_vec(), b"/api/posts".to_vec()];
        let mut prefilter = InnerPrefilter::new();
        for (i, pattern) in patterns.into_iter().enumerate() {
            prefilter.insert(i, vec![pattern]);
        }

        let result = prefilter.check(b"/api/users/123");
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

        let result = prefilter.check(b"/api/v1/users");
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

        let result = prefilter.check(b"/api/v1");
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

        let result = prefilter.check(b"/abc/def");
        assert!(result.contains(&0));
        assert!(result.contains(&1));
        assert!(result.contains(&2));
        assert!(result.contains(&3));

        let result = prefilter.check(b"/ab");
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
        let result = prefilter.check(b"/target/resource");

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

        let result = prefilter.check(b"/api/users/123");
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
        let result = prefilter.check(b"/api/v1/users");
        assert!(result.contains(&0)); // "/api" still matches
        assert!(!result.contains(&1)); // "/api/v1" was removed

        // Remove all routes
        prefilter.remove(&0);
        prefilter.remove(&2);
        assert!(prefilter.is_empty());
    }

    #[test]
    fn test_remove_compaction() {
        let mut prefilter = InnerPrefilter::new();
        // Build a trie with structure: root -> "a" -> "b" -> "c" (key 0)
        //                                          -> "x" (key 1)
        // Removing key 1 should compact "a"+"b" into "ab" since the "b"
        // node would have no keys and one child.
        prefilter.insert(0, vec![b"abc".to_vec()]);
        prefilter.insert(1, vec![b"abx".to_vec()]);
        prefilter.insert(2, vec![b"a".to_vec()]);

        // Verify all match before removal
        assert!(prefilter.check(b"abc_more").contains(&0));
        assert!(prefilter.check(b"abx_more").contains(&1));

        // Remove key 1 — "ab" node now has one child "c", should compact
        prefilter.remove(&1);
        // Key 0 must still work after compaction
        assert!(prefilter.check(b"abc_more").contains(&0));
        assert!(prefilter.check(b"a_more").contains(&2));
        assert!(!prefilter.check(b"abx_more").contains(&1));

        // Remove key 2, then key 0 — trie should be fully empty
        prefilter.remove(&2);
        prefilter.remove(&0);
        assert!(prefilter.is_empty());
    }

    #[test]
    fn test_edge_split_insert() {
        let mut prefilter = InnerPrefilter::new();
        // Insert "abcdef" then "abcxyz" — forces a split at "abc"
        prefilter.insert(0, vec![b"abcdef".to_vec()]);
        prefilter.insert(1, vec![b"abcxyz".to_vec()]);

        assert!(prefilter.check(b"abcdef_more").contains(&0));
        assert!(prefilter.check(b"abcxyz_more").contains(&1));
        assert!(!prefilter.check(b"abc").contains(&0));
        assert!(!prefilter.check(b"abc").contains(&1));

        // Insert "abc" — key at the split point itself
        prefilter.insert(2, vec![b"abc".to_vec()]);
        let result = prefilter.check(b"abcdef_more");
        assert!(result.contains(&0));
        assert!(result.contains(&2));
        assert!(!result.contains(&1));

        // Insert "ab" — forces another split higher up
        prefilter.insert(3, vec![b"ab".to_vec()]);
        let result = prefilter.check(b"abcxyz_more");
        assert!(result.contains(&1));
        assert!(result.contains(&2));
        assert!(result.contains(&3));
        assert!(!result.contains(&0));
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
