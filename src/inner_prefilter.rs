use aho_corasick;
use roaring::RoaringBitmap;

type Idx = u32;

#[derive(Debug, Clone)]
pub struct AhoCorasickPrefilter {
    automaton: aho_corasick::AhoCorasick,
    pattern_to_index: Vec<Idx>,
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

        let automaton = aho_corasick::AhoCorasickBuilder::new()
            .start_kind(aho_corasick::StartKind::Anchored)
            .build(patterns)
            .ok()?;

        Some(Self {
            automaton,
            pattern_to_index: pattern_indexes,
        })
    }

    /// Checks bytes against the prefilter, returning a bitmap of possible matcher indexes.
    pub fn check(&self, bytes: &[u8]) -> RoaringBitmap {
        let mut possible_indexes = RoaringBitmap::new();
        let mut state = aho_corasick::automaton::OverlappingState::start();
        let input = aho_corasick::Input::new(bytes).anchored(aho_corasick::Anchored::Yes);

        loop {
            self.automaton.find_overlapping(input.clone(), &mut state);
            match state.get_match() {
                Some(m) => {
                    possible_indexes.insert(self.pattern_to_index[m.pattern().as_usize()]);
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
        self.pattern_to_index[0]
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
}
