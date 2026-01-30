//! Fast prefix-based prefiltering for router pattern matching.
//!
//! This crate provides efficient prefiltering of route matchers by extracting and indexing
//! literal prefixes from patterns. It enables quick elimination of non-matching routes
//! before running full pattern matching.
//!
//! # Examples
//!
//! ```
//! use router_prefilter::{RouterPrefilter, Matcher, MatcherVisitor, Case};
//!
//! struct RoutePattern {
//!     prefix: String,
//! }
//!
//! impl Matcher for RoutePattern {
//!     fn visit(&self, visitor: &mut MatcherVisitor) {
//!         visitor.visit_match_starts_with(&self.prefix, Case::Sensitive);
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

use inner_prefilter::InnerPrefilter;
use regex_syntax::hir::{Hir, literal};
use std::collections::{BTreeSet, btree_set};
use std::convert::Infallible;
use std::mem;

/// Specifies whether pattern matching is case-sensitive or case-insensitive.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Case {
    /// Pattern matching is case-sensitive.
    Sensitive,
    /// Pattern matching is case-insensitive.
    Insensitive,
}

/// Describes a pattern matcher that can be analyzed for prefix extraction.
///
/// Implementors use the [`MatcherVisitor`] to describe their matching logic,
/// allowing the prefilter to extract literal prefixes for fast filtering.
///
/// # Examples
///
/// ```
/// use router_prefilter::{Matcher, MatcherVisitor, Case};
///
/// struct PrefixMatcher {
///     prefix: String,
/// }
///
/// impl Matcher for PrefixMatcher {
///     fn visit(&self, visitor: &mut MatcherVisitor) {
///         visitor.visit_match_starts_with(&self.prefix, Case::Sensitive);
///     }
/// }
/// ```
pub trait Matcher {
    /// Visits this matcher using the provided visitor.
    ///
    /// Implementations should call appropriate visitor methods to describe
    /// their matching behavior.
    fn visit(&self, visitor: &mut MatcherVisitor);
}

impl<M: Matcher> Matcher for &M {
    fn visit(&self, visitor: &mut MatcherVisitor) {
        M::visit(self, visitor);
    }
}

/// A prefilter for quickly identifying potentially matching route patterns.
///
/// The prefilter analyzes route matchers to extract literal prefixes and builds
/// an efficient data structure for fast lookup. Routes without extractable
/// prefixes are tracked separately as always-possible matches.
///
/// # Examples
///
/// ```
/// use router_prefilter::{RouterPrefilter, Matcher, MatcherVisitor, Case};
///
/// struct Route {
///     path: String,
/// }
///
/// impl Matcher for Route {
///     fn visit(&self, visitor: &mut MatcherVisitor) {
///         visitor.visit_match_starts_with(&self.path, Case::Sensitive);
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

impl<K> Default for RouterPrefilter<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K> RouterPrefilter<K> {
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
    /// use router_prefilter::{RouterPrefilter, Matcher, MatcherVisitor, Case};
    ///
    /// struct Route(&'static str);
    ///
    /// impl Matcher for Route {
    ///     fn visit(&self, visitor: &mut MatcherVisitor) {
    ///         visitor.visit_match_starts_with(self.0, Case::Sensitive);
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
}

impl<K: Ord> RouterPrefilter<K> {
    /// Inserts a matcher with the given key.
    ///
    /// The matcher is analyzed to extract literal prefixes for fast filtering.
    /// If no prefixes can be extracted, the matcher is tracked as always-possible.
    ///
    /// # Examples
    ///
    /// ```
    /// use router_prefilter::{RouterPrefilter, Matcher, MatcherVisitor, Case};
    ///
    /// struct Route(&'static str);
    ///
    /// impl Matcher for Route {
    ///     fn visit(&self, visitor: &mut MatcherVisitor) {
    ///         visitor.visit_match_starts_with(self.0, Case::Sensitive);
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
    /// use router_prefilter::{RouterPrefilter, Matcher, MatcherVisitor, Case};
    ///
    /// struct Route(&'static str);
    ///
    /// impl Matcher for Route {
    ///     fn visit(&self, visitor: &mut MatcherVisitor) {
    ///         visitor.visit_match_starts_with(self.0, Case::Sensitive);
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
    /// use router_prefilter::{RouterPrefilter, Matcher, MatcherVisitor, Case};
    ///
    /// struct Route(&'static str);
    ///
    /// impl Matcher for Route {
    ///     fn visit(&self, visitor: &mut MatcherVisitor) {
    ///         visitor.visit_match_starts_with(self.0, Case::Sensitive);
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

fn extract_prefixes(hir: &Hir) -> Option<BTreeSet<Vec<u8>>> {
    if !hir
        .properties()
        .look_set_prefix()
        .contains_anchor_haystack()
    {
        return None;
    }
    let seq = literal::Extractor::new().extract(hir);
    seq.literals()
        .map(|literals| BTreeSet::from_iter(literals.iter().map(|lit| lit.as_bytes().to_vec())))
}

#[derive(Debug)]
struct Frame {
    and_literal_prefixes: Option<BTreeSet<Vec<u8>>>,
    or_literal_prefixes: Option<BTreeSet<Vec<u8>>>,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            and_literal_prefixes: None,
            or_literal_prefixes: Some(BTreeSet::new()),
        }
    }
}

impl Frame {
    fn finish(self) -> Option<BTreeSet<Vec<u8>>> {
        let Self {
            mut or_literal_prefixes,
            and_literal_prefixes,
        } = self;
        union_prefixes_limited(&mut or_literal_prefixes, and_literal_prefixes, 100);
        or_literal_prefixes
    }
}

/// Visitor for extracting literal prefixes from matcher patterns.
///
/// Supports AND, OR, and nested pattern combinations. Extracted prefixes
/// are used to build the prefilter's lookup structure.
///
/// Instances of this visitor are passed to [`Matcher::visit`] implementations.
///
/// # Examples
///
/// Basic usage with a simple prefix:
///
/// ```
/// use router_prefilter::{Matcher, MatcherVisitor, Case};
///
/// struct ApiRoute;
///
/// impl Matcher for ApiRoute {
///     fn visit(&self, visitor: &mut MatcherVisitor) {
///         visitor.visit_match_starts_with("/api", Case::Sensitive);
///     }
/// }
/// ```
///
/// Complex pattern with nesting:
///
/// ```
/// use router_prefilter::{Matcher, MatcherVisitor, Case};
///
/// struct VersionedRoute;
///
/// impl Matcher for VersionedRoute {
///     fn visit(&self, visitor: &mut MatcherVisitor) {
///         // /v && (/v1 || /v2)
///         visitor.visit_match_starts_with("/v", Case::Sensitive);
///         visitor.visit_nested_start();
///         visitor.visit_match_starts_with("/v1", Case::Sensitive);
///         visitor.visit_or_in();
///         visitor.visit_match_starts_with("/v2", Case::Sensitive);
///         visitor.visit_nested_finish();
///     }
/// }
/// ```
#[derive(Debug)]
pub struct MatcherVisitor {
    frames: Vec<Frame>,
}

fn union_prefixes_limited(
    lhs: &mut Option<BTreeSet<Vec<u8>>>,
    rhs: Option<BTreeSet<Vec<u8>>>,
    max_len: usize,
) {
    let Some(lhs_inner) = lhs else {
        return;
    };
    let Some(mut rhs_inner) = rhs else {
        *lhs = None;
        return;
    };
    if rhs_inner.len() > lhs_inner.len() {
        mem::swap(lhs_inner, &mut rhs_inner);
    }
    let mut len = lhs_inner.len();
    for v in rhs_inner {
        let did_insert = lhs_inner.insert(v);
        len += usize::from(did_insert);
        if len > max_len {
            *lhs = None;
            return;
        }
    }
}

fn intersect_prefix_expansions_into(
    dst: &mut BTreeSet<Vec<u8>>,
    lhs: &mut BTreeSet<Vec<u8>>,
    rhs: &mut BTreeSet<Vec<u8>>,
) -> Option<Infallible> {
    let mut l = lhs.pop_first()?;
    let mut r = rhs.pop_first()?;

    loop {
        while l <= r {
            if r.starts_with(&l) {
                dst.insert(r);
                r = rhs.pop_first()?;
            } else {
                l = lhs.pop_first()?;
            }
        }
        if l.starts_with(&r) {
            dst.insert(l);
            l = lhs.pop_first()?;
        } else {
            r = rhs.pop_first()?;
        }
    }
}

fn intersect_prefix_expansions(
    lhs: &mut Option<BTreeSet<Vec<u8>>>,
    rhs: Option<BTreeSet<Vec<u8>>>,
) {
    let Some(lhs) = lhs else {
        *lhs = rhs;
        return;
    };
    let Some(mut rhs) = rhs else {
        return;
    };

    let mut result = BTreeSet::new();
    _ = intersect_prefix_expansions_into(&mut result, lhs, &mut rhs);
    *lhs = result;
}

impl MatcherVisitor {
    fn new() -> Self {
        Self {
            frames: vec![Frame::default()],
        }
    }

    fn current_frame(&mut self) -> &mut Frame {
        self.frames.last_mut().unwrap()
    }

    fn finish(&mut self) -> literal::Seq {
        let Self { frames } = self;
        let frame = match &mut frames[..] {
            [only_frame] => mem::take(only_frame),
            _ => {
                frames.clear();
                frames.push(Frame::default());
                panic!("mismatched nesting calls to MatcherVisitor")
            }
        };
        frame.finish().map_or_else(literal::Seq::infinite, |set| {
            let mut seq = literal::Seq::new(set);
            seq.optimize_for_prefix_by_preference();
            seq
        })
    }

    /// Begins a nested matching context.
    ///
    /// Use this to group patterns together for complex matching logic.
    /// Must be paired with [`visit_nested_finish`].
    ///
    /// # Examples
    ///
    /// ```
    /// use router_prefilter::{Matcher, MatcherVisitor, Case};
    ///
    /// struct NestedRoute;
    ///
    /// impl Matcher for NestedRoute {
    ///     fn visit(&self, visitor: &mut MatcherVisitor) {
    ///         visitor.visit_nested_start();
    ///         visitor.visit_match_starts_with("/api", Case::Sensitive);
    ///         visitor.visit_nested_finish();
    ///     }
    /// }
    /// ```
    ///
    /// [`visit_nested_finish`]: MatcherVisitor::visit_nested_finish
    pub fn visit_nested_start(&mut self) {
        self.frames.push(Frame::default());
    }

    /// Completes a nested matching context.
    ///
    /// # Panics
    ///
    /// Panics if called without a matching [`visit_nested_start`].
    ///
    /// [`visit_nested_start`]: MatcherVisitor::visit_nested_start
    pub fn visit_nested_finish(&mut self) {
        let frame = self
            .frames
            .pop()
            .expect("every finish should match with a start");
        let new_inner = frame.finish();
        intersect_prefix_expansions(&mut self.current_frame().and_literal_prefixes, new_inner);
    }

    /// Marks an OR boundary in the current matching context.
    ///
    /// Use this to separate alternative patterns that should be treated
    /// as different matching possibilities.
    ///
    /// # Examples
    ///
    /// ```
    /// use router_prefilter::{Matcher, MatcherVisitor, Case};
    ///
    /// struct MultiVersionRoute;
    ///
    /// impl Matcher for MultiVersionRoute {
    ///     fn visit(&self, visitor: &mut MatcherVisitor) {
    ///         visitor.visit_match_starts_with("/v1", Case::Sensitive);
    ///         visitor.visit_or_in();
    ///         visitor.visit_match_starts_with("/v2", Case::Sensitive);
    ///     }
    /// }
    /// ```
    pub fn visit_or_in(&mut self) {
        let frame = self.current_frame();
        let new_and = frame.and_literal_prefixes.take();
        union_prefixes_limited(&mut frame.or_literal_prefixes, new_and, 100);
    }

    /// Processes a regex pattern to extract literal prefixes.
    ///
    /// Parses the regex and extracts any literal prefixes that can be used
    /// for prefiltering. Only anchored patterns yield extractable prefixes.
    ///
    /// # Examples
    ///
    /// ```
    /// use router_prefilter::{Matcher, MatcherVisitor};
    ///
    /// struct RegexRoute(&'static str);
    ///
    /// impl Matcher for RegexRoute {
    ///     fn visit(&self, visitor: &mut MatcherVisitor) {
    ///         visitor.visit_match_regex(self.0);
    ///     }
    /// }
    ///
    /// let route = RegexRoute("^/api/.*");
    /// ```
    pub fn visit_match_regex(&mut self, regex: &str) {
        let hir = regex_syntax::parse(regex).unwrap_or_else(|_| Hir::fail());
        let current = &mut self.frames.last_mut().unwrap().and_literal_prefixes;
        let new_prefixes = extract_prefixes(&hir);
        intersect_prefix_expansions(current, new_prefixes);
    }

    /// Processes an exact equality match pattern.
    ///
    /// For prefiltering purposes, exact equality matches are treated the same
    /// as prefix matches, since any string equal to the pattern also starts
    /// with it.
    ///
    /// # Examples
    ///
    /// ```
    /// use router_prefilter::{Matcher, MatcherVisitor, Case};
    ///
    /// struct ExactRoute(&'static str);
    ///
    /// impl Matcher for ExactRoute {
    ///     fn visit(&self, visitor: &mut MatcherVisitor) {
    ///         visitor.visit_match_equals(self.0, Case::Sensitive);
    ///     }
    /// }
    ///
    /// let route = ExactRoute("/api/users");
    /// ```
    pub fn visit_match_equals(&mut self, equals: &str, case: Case) {
        // for our purposes, equality and starting with are the same
        self.visit_match_starts_with(equals, case);
    }

    /// Processes a prefix match pattern.
    ///
    /// Case-sensitive matches can be optimized for prefiltering.
    /// Case-insensitive matches are currently not optimized.
    ///
    /// # Examples
    ///
    /// ```
    /// use router_prefilter::{Matcher, MatcherVisitor, Case};
    ///
    /// struct PrefixRoute(&'static str);
    ///
    /// impl Matcher for PrefixRoute {
    ///     fn visit(&self, visitor: &mut MatcherVisitor) {
    ///         visitor.visit_match_starts_with(self.0, Case::Sensitive);
    ///     }
    /// }
    ///
    /// let route = PrefixRoute("/api");
    /// ```
    pub fn visit_match_starts_with(&mut self, prefix: &str, case: Case) {
        if case != Case::Sensitive {
            // in the future, we might want to see if we can use aho-corasick's ability to do
            // ascii case-insensitive matching, but for now, we can't optimize it
            return;
        }
        let new_prefixes = Some(BTreeSet::from([prefix.as_bytes().to_vec()]));
        let current = &mut self.frames.last_mut().unwrap().and_literal_prefixes;
        intersect_prefix_expansions(current, new_prefixes);
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
                visitor.visit_match_starts_with(prefix, Case::Sensitive);
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
