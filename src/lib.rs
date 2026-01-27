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
//! let prefilter = RouterPrefilter::new(routes).unwrap();
//! let matches: Vec<_> = prefilter.possible_matches("/api/v1").collect();
//! assert_eq!(matches, vec![0]);
//! ```

#![warn(variant_size_differences)]
#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unnameable_types)]

mod inner_prefilter;

use inner_prefilter::InnerPrefilter;
use regex_syntax::hir::{Hir, literal};
use roaring::RoaringBitmap;
use std::collections::BTreeSet;
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

type Idx = u32;

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
/// let prefilter = RouterPrefilter::new(routes).unwrap();
/// let matches: Vec<_> = prefilter.possible_matches("/api/posts").collect();
/// assert!(matches.contains(&0));
/// ```
#[derive(Debug, Clone)]
pub struct RouterPrefilter {
    // Only includes indexes after prefilter starts
    always_possible_indexes: RoaringBitmap,
    first_prefiltered: Idx,
    prefilter: InnerPrefilter,
}

impl RouterPrefilter {
    /// Creates a new router prefilter from a collection of matchers.
    ///
    /// See also [`Self::with_max_unfiltered`], to configure a maximum number of unfiltered matchers
    ///
    /// # Examples
    ///
    /// ```
    /// use router_prefilter::{RouterPrefilter, Matcher, MatcherVisitor, Case};
    ///
    /// struct SimpleRoute(&'static str);
    ///
    /// impl Matcher for SimpleRoute {
    ///     fn visit(&self, visitor: &mut MatcherVisitor) {
    ///         visitor.visit_match_starts_with(self.0, Case::Sensitive);
    ///     }
    /// }
    ///
    /// let routes = vec![
    ///     SimpleRoute("/api"),
    ///     SimpleRoute("/users"),
    /// ];
    ///
    /// let prefilter = RouterPrefilter::new(routes);
    /// assert!(prefilter.is_some());
    /// ```
    ///
    /// Returns [`None`] if:
    /// - No matchers are provided
    /// - Too many matchers lack extractable prefixes
    /// - The internal prefilter fails to build
    pub fn new<M, I>(matchers: I) -> Option<Self>
    where
        M: Matcher,
        I: IntoIterator<Item = M>,
    {
        Self::with_max_unfiltered(matchers, usize::MAX)
    }

    /// Creates a new router prefilter with a custom unfiltered matcher limit.
    ///
    /// Analyzes each matcher to extract literal prefixes and builds an efficient
    /// lookup structure. Matchers without extractable prefixes are tracked as
    /// always-possible matches up to the `max_unfiltered` limit.
    ///
    /// # Examples
    ///
    /// ```
    /// use router_prefilter::{RouterPrefilter, Matcher, MatcherVisitor, Case};
    ///
    /// struct SimpleRoute(&'static str);
    ///
    /// impl Matcher for SimpleRoute {
    ///     fn visit(&self, visitor: &mut MatcherVisitor) {
    ///         visitor.visit_match_starts_with(self.0, Case::Sensitive);
    ///     }
    /// }
    ///
    /// let routes = vec![
    ///     SimpleRoute("/api"),
    ///     SimpleRoute("/users"),
    /// ];
    ///
    /// let prefilter = RouterPrefilter::with_max_unfiltered(routes, 100);
    /// assert!(prefilter.is_some());
    /// ```
    ///
    /// Returns [`None`] if:
    /// - No matchers are provided
    /// - All matchers lack extractable prefixes and exceed `max_unfiltered`
    /// - The internal prefilter fails to build
    pub fn with_max_unfiltered<M, I>(matchers: I, max_unfiltered: usize) -> Option<Self>
    where
        M: Matcher,
        I: IntoIterator<Item = M>,
    {
        let max_unfiltered = Idx::try_from(max_unfiltered).unwrap_or(Idx::MAX);

        let mut matchers = matchers.into_iter().enumerate();
        let mut extractor = MatcherVisitor::new();
        let mut num_unfiltered = 0;
        let (idx, first_prefixes) = loop {
            let (i, m) = matchers.next()?;
            m.visit(&mut extractor);
            let extracted_prefixes = extractor.finish();
            if extracted_prefixes.is_finite() {
                break (i, extracted_prefixes);
            }
            num_unfiltered += 1;
            if num_unfiltered >= max_unfiltered {
                return None;
            }
        };
        let first_prefiltered = num_unfiltered;

        let mut unfiltered = RoaringBitmap::new();
        let mut patterns = Vec::new();
        let mut pattern_indexes = Vec::new();

        for prefix in first_prefixes.literals().unwrap_or_default() {
            patterns.push(prefix.as_bytes().to_vec());
            pattern_indexes.push(Idx::try_from(idx).unwrap());
        }

        for (i, m) in matchers {
            m.visit(&mut extractor);
            let extracted_prefixes = extractor.finish();

            if let Some(prefixes) = extracted_prefixes.literals() {
                patterns.reserve(prefixes.len());
                pattern_indexes.reserve(prefixes.len());
                for prefix in prefixes {
                    patterns.push(prefix.as_bytes().to_vec());
                    pattern_indexes.push(Idx::try_from(i).unwrap());
                }
            } else {
                unfiltered.insert(i as u32);
                num_unfiltered += 1;
                if num_unfiltered >= max_unfiltered {
                    return None;
                }
            }
        }
        let prefilter = InnerPrefilter::new(&patterns, pattern_indexes)?;
        Some(Self {
            always_possible_indexes: unfiltered,
            first_prefiltered,
            prefilter,
        })
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
    /// let prefilter = RouterPrefilter::new(routes).unwrap();
    ///
    /// let matches: Vec<_> = prefilter.possible_matches("/api/v1").collect();
    /// assert_eq!(matches, vec![0]);
    /// ```
    #[must_use]
    pub fn possible_matches<'a>(&'a self, value: &'a str) -> RouterPrefilterIter<'a> {
        let value = value.as_bytes();
        RouterPrefilterIter(RouterPrefilterIterState::BeforePrefilter {
            i: 0,
            router_prefilter: self,
            s: value,
        })
    }
}

/// Iterator over matcher indexes that may match a given value.
///
/// Created by [`RouterPrefilter::possible_matches`]. Yields matcher indexes
/// in ascending order.
pub struct RouterPrefilterIter<'a>(RouterPrefilterIterState<'a>);

impl Iterator for RouterPrefilterIter<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0 {
            RouterPrefilterIterState::BeforePrefilter {
                ref mut i,
                router_prefilter,
                s,
            } => {
                let idx = *i;
                if idx >= router_prefilter.first_prefiltered {
                    let mut it = make_combined_prefilter_iter(router_prefilter, s);
                    let result = it.next().map(|i| usize::try_from(i).unwrap());
                    self.0 = RouterPrefilterIterState::Both(it);
                    return result;
                }
                *i = idx + 1;
                Some(usize::try_from(idx).unwrap())
            }
            RouterPrefilterIterState::Both(ref mut inner) => {
                inner.next().map(|i| usize::try_from(i).unwrap())
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.0 {
            RouterPrefilterIterState::BeforePrefilter {
                i,
                router_prefilter,
                s: _,
            } => {
                let min_size = router_prefilter.first_prefiltered - i;
                (min_size as usize, None)
            }
            RouterPrefilterIterState::Both(ref inner) => inner.size_hint(),
        }
    }

    fn fold<B, F>(self, mut init: B, mut f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        let both_iter = match self.0 {
            RouterPrefilterIterState::BeforePrefilter {
                i,
                router_prefilter,
                s,
            } => {
                init = (i..router_prefilter.first_prefiltered)
                    .map(|idx| usize::try_from(idx).unwrap())
                    .fold(init, &mut f);
                make_combined_prefilter_iter(router_prefilter, s)
            }
            RouterPrefilterIterState::Both(it) => it,
        };
        both_iter
            .map(|idx| usize::try_from(idx).unwrap())
            .fold(init, &mut f)
    }
}
fn make_combined_prefilter_iter(
    router_prefilter: &RouterPrefilter,
    s: &[u8],
) -> roaring::bitmap::IntoIter {
    let mut indexes = router_prefilter.always_possible_indexes.clone();
    if let Some(prefilter_indexes) = router_prefilter.prefilter.check(s) {
        indexes |= prefilter_indexes;
    }
    indexes.into_iter()
}

enum RouterPrefilterIterState<'a> {
    BeforePrefilter {
        i: Idx,
        router_prefilter: &'a RouterPrefilter,
        s: &'a [u8],
    },
    Both(roaring::bitmap::IntoIter),
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
        let frame = frames.pop().unwrap();
        assert!(frames.is_empty());
        frames.push(Frame::default());
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
        // TODO: Should we return an error? Or panic instead?
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

        let prefilter = RouterPrefilter::new(matchers).unwrap();
        let matches: Vec<_> = prefilter.possible_matches("/api/test").collect();

        assert_eq!(matches, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_max_unfiltered_limit() {
        let matchers = vec![
            TestMatcher::without_prefix(),
            TestMatcher::without_prefix(),
            TestMatcher::without_prefix(),
            TestMatcher::with_prefix("/api"),
        ];

        // Should succeed with limit of 4 (allows 3 unfiltered + 1 with prefix)
        let prefilter = RouterPrefilter::with_max_unfiltered(matchers.clone(), 4);
        assert!(prefilter.is_some());

        // Should fail with limit of 2 (only allows 2 unfiltered, but we have 3)
        let prefilter = RouterPrefilter::with_max_unfiltered(matchers, 2);
        assert!(prefilter.is_none());
    }
}
