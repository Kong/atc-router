//! Fast prefix-based prefiltering for router pattern matching.
//!
//! This crate provides efficient prefiltering of route matchers by extracting and indexing
//! literal prefixes from patterns. It enables quick elimination of non-matching routes
//! before running full route matching.
//!
//! # Examples
//!
//! ```
//! use router_prefilter::RouterPrefilter;
//! use router_prefilter::matchers::{Matcher, MatcherVisitor};
//!
//! struct RoutePattern {
//!     prefix: String,
//! }
//!
//! impl Matcher for RoutePattern {
//!     fn visit(&self, visitor: &mut MatcherVisitor) {
//!         visitor.visit_match_starts_with(&self.prefix);
//!     }
//! }
//!
//! let routes = vec![
//!     RoutePattern { prefix: "/api".to_string() },
//!     RoutePattern { prefix: "/users".to_string() },
//! ];
//!
//! let mut prefilter = RouterPrefilter::new();
//! for (i, route) in routes.into_iter().enumerate() {
//!     prefilter.insert(i, route);
//! }
//! let matches: Vec<_> = prefilter.possible_matches("/api/v1").collect();
//! assert_eq!(matches, vec![&0]);
//! ```

#![warn(variant_size_differences)]
#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unnameable_types)]

mod inner_prefilter;
pub mod matchers;

use crate::matchers::{Matcher, MatcherVisitor};
use inner_prefilter::InnerPrefilter;
use std::collections::{BTreeSet, btree_set};

/// A prefilter for quickly identifying potentially matching route patterns.
///
/// The prefilter analyzes route matchers to extract literal prefixes and builds
/// an efficient data structure for fast lookup. Routes without extractable
/// prefixes are tracked separately as always-possible matches.
///
/// # Examples
///
/// ```
/// use router_prefilter::RouterPrefilter;
/// use router_prefilter::matchers::{Matcher, MatcherVisitor};
///
/// struct Route {
///     path: String,
/// }
///
/// impl Matcher for Route {
///     fn visit(&self, visitor: &mut MatcherVisitor) {
///         visitor.visit_match_starts_with(&self.path);
///     }
/// }
///
/// let routes = vec![
///     Route { path: "/api".to_string() },
///     Route { path: "/users".to_string() },
/// ];
///
/// let mut prefilter = RouterPrefilter::new();
/// for (i, route) in routes.into_iter().enumerate() {
///     prefilter.insert(i, route);
/// }
/// let matches: Vec<_> = prefilter.possible_matches("/api/posts").collect();
/// assert!(matches.contains(&&0));
/// ```
#[derive(Debug)]
pub struct RouterPrefilter<K> {
    // Only includes indexes after prefilter starts
    always_possible: BTreeSet<K>,
    prefilter: InnerPrefilter<K>,

    matcher_visitor: MatcherVisitor,
}

impl<K: Clone> Clone for RouterPrefilter<K> {
    fn clone(&self) -> Self {
        Self {
            always_possible: self.always_possible.clone(),
            prefilter: self.prefilter.clone(),

            matcher_visitor: MatcherVisitor::new(),
        }
    }
}

impl<K: Ord> Default for RouterPrefilter<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Ord> RouterPrefilter<K> {
    /// Creates a new empty prefilter.
    ///
    /// # Examples
    ///
    /// ```
    /// use router_prefilter::RouterPrefilter;
    ///
    /// let prefilter: RouterPrefilter<usize> = RouterPrefilter::new();
    /// ```
    pub fn new() -> Self {
        Self {
            always_possible: BTreeSet::new(),
            prefilter: InnerPrefilter::new(),

            matcher_visitor: MatcherVisitor::new(),
        }
    }

    /// Returns whether this prefilter can perform filtering.
    ///
    /// Returns `true` if at least one matcher has been inserted with extractable
    /// prefixes. Returns `false` if the prefilter is empty or all matchers lack
    /// extractable prefixes.
    ///
    /// # Examples
    ///
    /// ```
    /// use router_prefilter::RouterPrefilter;
    /// use router_prefilter::matchers::{Matcher, MatcherVisitor};
    ///
    /// struct Route(&'static str);
    ///
    /// impl Matcher for Route {
    ///     fn visit(&self, visitor: &mut MatcherVisitor) {
    ///         visitor.visit_match_starts_with(self.0);
    ///     }
    /// }
    ///
    /// let mut prefilter = RouterPrefilter::new();
    /// assert!(!prefilter.can_prefilter());
    ///
    /// prefilter.insert(0, Route("/api"));
    /// assert!(prefilter.can_prefilter());
    /// ```
    pub fn can_prefilter(&self) -> bool {
        !self.prefilter.is_empty()
    }

    /// Returns the number of routes with extractable prefixes.
    ///
    /// A "prefilterable" route is one from which literal prefixes can be
    /// extracted for fast filtering. Routes without extractable prefixes
    /// are tracked separately as always-possible matches and are not
    /// counted by this method.
    ///
    /// A pattern must be anchored at the start and begin with literal
    /// characters to have an extractable prefix.
    ///
    /// # Examples
    ///
    /// ```
    /// use router_prefilter::RouterPrefilter;
    /// use router_prefilter::matchers::{Matcher, MatcherVisitor};
    ///
    /// struct Route {
    ///     pattern: &'static str,
    /// }
    ///
    /// impl Matcher for Route {
    ///     fn visit(&self, visitor: &mut MatcherVisitor) {
    ///         visitor.visit_match_regex(self.pattern);
    ///     }
    /// }
    ///
    /// let mut prefilter = RouterPrefilter::new();
    ///
    /// // Anchored with literal prefix - prefilterable
    /// prefilter.insert(0, Route { pattern: r"^/api/.*" });
    /// prefilter.insert(1, Route { pattern: r"^/users/\d+$" });
    ///
    /// // Anchored but no literal prefix - not prefilterable
    /// prefilter.insert(2, Route { pattern: r"^.*abc" });
    /// prefilter.insert(3, Route { pattern: r"^\d+/api" });
    ///
    /// // Not anchored - not prefilterable
    /// prefilter.insert(4, Route { pattern: r"/abc/def" });
    ///
    /// // Only routes 0 and 1 have extractable literal prefixes
    /// assert_eq!(prefilter.prefilterable_routes(), 2);
    /// ```
    pub fn prefilterable_routes(&self) -> usize {
        self.prefilter.num_routes()
    }

    /// Returns the total number of routes in the prefilter.
    ///
    /// This includes both routes with extractable prefixes and routes
    /// tracked as always-possible matches.
    ///
    /// # Examples
    ///
    /// ```
    /// use router_prefilter::RouterPrefilter;
    /// use router_prefilter::matchers::{Matcher, MatcherVisitor};
    ///
    /// struct Route {
    ///     pattern: &'static str,
    /// }
    ///
    /// impl Matcher for Route {
    ///     fn visit(&self, visitor: &mut MatcherVisitor) {
    ///         visitor.visit_match_regex(self.pattern);
    ///     }
    /// }
    ///
    /// let mut prefilter = RouterPrefilter::new();
    /// prefilter.insert(0, Route { pattern: r"^/api/.*" });
    /// prefilter.insert(1, Route { pattern: r"^.*abc" });
    ///
    /// assert_eq!(prefilter.len(), 2);
    /// ```
    pub fn len(&self) -> usize {
        self.prefilter.num_routes() + self.always_possible.len()
    }

    /// Returns whether the prefilter contains any routes.
    ///
    /// # Examples
    ///
    /// ```
    /// use router_prefilter::RouterPrefilter;
    /// use router_prefilter::matchers::{Matcher, MatcherVisitor};
    ///
    /// struct Route {
    ///     pattern: &'static str,
    /// }
    ///
    /// impl Matcher for Route {
    ///     fn visit(&self, visitor: &mut MatcherVisitor) {
    ///         visitor.visit_match_regex(self.pattern);
    ///     }
    /// }
    ///
    /// let mut prefilter: RouterPrefilter<usize> = RouterPrefilter::new();
    /// assert!(prefilter.is_empty());
    ///
    /// prefilter.insert(0, Route { pattern: r"^/api/.*" });
    /// assert!(!prefilter.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.always_possible.is_empty() && self.prefilter.is_empty()
    }

    /// Inserts a matcher with the given key.
    ///
    /// The matcher is analyzed to extract literal prefixes for fast filtering.
    /// If no prefixes can be extracted, the matcher is tracked as always-possible.
    ///
    /// # Examples
    ///
    /// ```
    /// use router_prefilter::RouterPrefilter;
    /// use router_prefilter::matchers::{Matcher, MatcherVisitor};
    ///
    /// struct Route(&'static str);
    ///
    /// impl Matcher for Route {
    ///     fn visit(&self, visitor: &mut MatcherVisitor) {
    ///         visitor.visit_match_starts_with(self.0);
    ///     }
    /// }
    ///
    /// let mut prefilter = RouterPrefilter::new();
    /// prefilter.insert(0, Route("/api"));
    /// prefilter.insert(1, Route("/users"));
    /// ```
    pub fn insert<M: Matcher>(&mut self, key: K, matcher: M)
    where
        K: Clone,
    {
        matcher.visit(&mut self.matcher_visitor);
        let seq = self.matcher_visitor.finish();
        if let Some(literals) = seq.literals() {
            let prefixes = literals.iter().map(|lit| lit.as_bytes().to_vec()).collect();
            self.prefilter.insert(key, prefixes);
        } else {
            self.always_possible.insert(key);
        }
    }

    /// Removes a matcher by key.
    ///
    /// # Examples
    ///
    /// ```
    /// use router_prefilter::RouterPrefilter;
    /// use router_prefilter::matchers::{Matcher, MatcherVisitor};
    ///
    /// struct Route(&'static str);
    ///
    /// impl Matcher for Route {
    ///     fn visit(&self, visitor: &mut MatcherVisitor) {
    ///         visitor.visit_match_starts_with(self.0);
    ///     }
    /// }
    ///
    /// let mut prefilter = RouterPrefilter::new();
    /// prefilter.insert(0, Route("/api"));
    /// prefilter.remove(&0);
    /// ```
    pub fn remove(&mut self, key: &K) {
        self.always_possible.remove(key);
        self.prefilter.remove(key);
    }

    /// Returns an iterator over matcher indexes that may match the given value.
    ///
    /// # Examples
    ///
    /// ```
    /// use router_prefilter::RouterPrefilter;
    /// use router_prefilter::matchers::{Matcher, MatcherVisitor};
    ///
    /// struct Route(&'static str);
    ///
    /// impl Matcher for Route {
    ///     fn visit(&self, visitor: &mut MatcherVisitor) {
    ///         visitor.visit_match_starts_with(self.0);
    ///     }
    /// }
    ///
    /// let routes = vec![Route("/api"), Route("/users")];
    /// let mut prefilter = RouterPrefilter::new();
    /// for (i, route) in routes.into_iter().enumerate() {
    ///     prefilter.insert(i, route);
    /// }
    ///
    /// let matches: Vec<_> = prefilter.possible_matches("/api/v1").collect();
    /// assert_eq!(matches, vec![&0]);
    /// ```
    #[must_use]
    #[doc(alias = "iter")]
    pub fn possible_matches<'a>(&'a self, value: &'a str) -> RouterPrefilterIter<'a, K> {
        let value = value.as_bytes();
        let inner = match self.prefilter.check(value) {
            Some(prefiltered) => {
                RouterPrefilterIterState::Union(prefiltered.union(&self.always_possible))
            }
            None => RouterPrefilterIterState::OnlyAlways(self.always_possible.iter()),
        };
        RouterPrefilterIter(inner)
    }
}

/// Iterator over matcher indexes that may match a given value.
///
/// Created by [`RouterPrefilter::possible_matches`]. Yields matcher indexes
/// in ascending order.
pub struct RouterPrefilterIter<'a, K>(RouterPrefilterIterState<'a, K>);

enum RouterPrefilterIterState<'a, K> {
    OnlyAlways(btree_set::Iter<'a, K>),
    Union(btree_set::Union<'a, K>),
}

impl<'a, K: Ord> Iterator for RouterPrefilterIter<'a, K> {
    type Item = &'a K;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            RouterPrefilterIterState::OnlyAlways(inner) => inner.next(),
            RouterPrefilterIterState::Union(inner) => inner.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            RouterPrefilterIterState::OnlyAlways(inner) => inner.size_hint(),
            RouterPrefilterIterState::Union(inner) => inner.size_hint(),
        }
    }

    fn fold<B, F>(self, init: B, f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        match self.0 {
            RouterPrefilterIterState::OnlyAlways(inner) => inner.fold(init, f),
            RouterPrefilterIterState::Union(inner) => inner.fold(init, f),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct TestMatcher {
        prefix: Option<&'static str>,
    }

    impl TestMatcher {
        fn with_prefix(prefix: &'static str) -> Self {
            Self {
                prefix: Some(prefix),
            }
        }

        fn without_prefix() -> Self {
            Self { prefix: None }
        }
    }

    impl Matcher for TestMatcher {
        fn visit(&self, visitor: &mut MatcherVisitor) {
            if let Some(prefix) = self.prefix {
                visitor.visit_match_starts_with(prefix);
            }
        }
    }

    #[test]
    fn test_iterator_no_skips_before_prefilter() {
        let matchers = vec![
            TestMatcher::without_prefix(),
            TestMatcher::without_prefix(),
            TestMatcher::without_prefix(),
            TestMatcher::without_prefix(),
            TestMatcher::with_prefix("/api"),
            TestMatcher::with_prefix("/users"),
        ];

        let mut prefilter = RouterPrefilter::new();
        for (i, matcher) in matchers.into_iter().enumerate() {
            prefilter.insert(i, matcher);
        }
        let matches: Vec<_> = prefilter.possible_matches("/api/test").collect();

        assert_eq!(matches, vec![&0, &1, &2, &3, &4]);
    }

    #[test]
    fn test_mixed_matchers() {
        let matchers = vec![
            TestMatcher::without_prefix(),
            TestMatcher::without_prefix(),
            TestMatcher::without_prefix(),
            TestMatcher::with_prefix("/api"),
        ];

        let mut prefilter = RouterPrefilter::new();
        for (i, matcher) in matchers.into_iter().enumerate() {
            prefilter.insert(i, matcher);
        }

        let matches: Vec<_> = prefilter.possible_matches("/api/test").collect();
        assert_eq!(matches, vec![&0, &1, &2, &3]);

        let matches: Vec<_> = prefilter.possible_matches("/other/path").collect();
        assert_eq!(matches, vec![&0, &1, &2]);
    }
}
