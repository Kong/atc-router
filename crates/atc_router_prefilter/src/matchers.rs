//! Matcher visitor pattern for extracting literal prefixes from route patterns.
//!
//! This module provides the visitor pattern infrastructure that allows route matchers
//! to describe their matching logic, enabling the prefilter to extract literal prefixes
//! for fast filtering.
//!
//! # Core Types
//!
//! - [`Matcher`] - Trait for types that can be analyzed for prefix extraction
//! - [`MatcherVisitor`] - Visitor that extracts literal prefixes from matcher patterns
//!
//! # Example
//!
//! ```
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
//! ```

use regex_syntax::hir::{Hir, literal};
use std::collections::BTreeSet;
use std::convert::Infallible;
use std::mem;

/// Describes a pattern matcher that can be analyzed for prefix extraction.
///
/// Implementors use the [`MatcherVisitor`] to describe their matching logic,
/// allowing the prefilter to extract literal prefixes for fast filtering.
///
/// # Examples
///
/// ```
/// use router_prefilter::matchers::{Matcher, MatcherVisitor};
///
/// struct PrefixMatcher {
///     prefix: String,
/// }
///
/// impl Matcher for PrefixMatcher {
///     fn visit(&self, visitor: &mut MatcherVisitor) {
///         visitor.visit_match_starts_with(&self.prefix);
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
        let prefixes = or_literal_prefixes?;
        if prefixes
            .first()
            .is_none_or(|shortest_prefix| shortest_prefix.is_empty())
        {
            return None;
        }
        Some(prefixes)
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
/// use router_prefilter::matchers::{Matcher, MatcherVisitor};
///
/// enum RouteMatcher {
///     And(Box<RouteMatcher>, Box<RouteMatcher>),
///     Or(Box<RouteMatcher>, Box<RouteMatcher>),
///     Regex(&'static str),
/// }
///
/// impl Matcher for RouteMatcher {
///     fn visit(&self, visitor: &mut MatcherVisitor) {
///         match self {
///             Self::And(lhs, rhs) => {
///                 visitor.visit_nested_start();
///                 lhs.visit(visitor);
///                 visitor.visit_nested_finish();
///                 visitor.visit_nested_start();
///                 rhs.visit(visitor);
///                 visitor.visit_nested_finish();
///             }
///             Self::Or(lhs, rhs) => {
///                 lhs.visit(visitor);
///                 visitor.visit_or_in();
///                 rhs.visit(visitor);
///             }
///             Self::Regex(regex) => visitor.visit_match_regex(regex),
///         }
///     }
/// }
/// ```
///
/// Complex pattern with nesting:
///
/// ```
/// use router_prefilter::matchers::{Matcher, MatcherVisitor};
///
/// struct VersionedRoute;
///
/// impl Matcher for VersionedRoute {
///     fn visit(&self, visitor: &mut MatcherVisitor) {
///         // /v && (/v1 || /v2)
///         visitor.visit_match_starts_with("/v");
///         visitor.visit_nested_start();
///         visitor.visit_match_starts_with("/v1");
///         visitor.visit_or_in();
///         visitor.visit_match_starts_with("/v2");
///         visitor.visit_nested_finish();
///     }
/// }
/// ```
#[derive(Debug)]
pub struct MatcherVisitor {
    frames: Vec<Frame>,
}

impl MatcherVisitor {
    pub(crate) fn new() -> Self {
        Self {
            frames: vec![Frame::default()],
        }
    }

    fn current_frame(&mut self) -> &mut Frame {
        self.frames.last_mut().unwrap()
    }

    pub(crate) fn finish(&mut self) -> Option<BTreeSet<Vec<u8>>> {
        let Self { frames } = self;
        let frame = match &mut frames[..] {
            [only_frame] => mem::take(only_frame),
            _ => {
                frames.clear();
                frames.push(Frame::default());
                panic!("mismatched nesting calls to MatcherVisitor")
            }
        };
        frame.finish()
    }

    /// Begins a nested matching context.
    ///
    /// Use this to group patterns together for complex matching logic.
    /// Must be paired with [`visit_nested_finish`].
    ///
    /// Acts as a kind of "parenthesis" for matching logic.
    ///
    /// # Examples
    ///
    /// ```
    /// use router_prefilter::matchers::{Matcher, MatcherVisitor};
    ///
    /// struct NestedRoute;
    ///
    /// impl Matcher for NestedRoute {
    ///     fn visit(&self, visitor: &mut MatcherVisitor) {
    ///         // startsWith("/api") || (contains("abc") && contains("def"))
    ///         visitor.visit_match_starts_with("/api");
    ///         visitor.visit_or_in();
    ///         visitor.visit_nested_start();
    ///         visitor.visit_match_regex("abc");
    ///         visitor.visit_match_regex("def");
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
    /// Finalizes the pattern grouping started with [`visit_nested_start`].
    /// Must be paired with a preceding [`visit_nested_start`] call.
    ///
    /// The nested context's extracted prefixes are merged into the parent
    /// context using AND semantics.
    ///
    /// # Examples
    ///
    /// ```
    /// use router_prefilter::matchers::{Matcher, MatcherVisitor};
    ///
    /// struct NestedRoute;
    ///
    /// impl Matcher for NestedRoute {
    ///     fn visit(&self, visitor: &mut MatcherVisitor) {
    ///         // startsWith("/api") || (contains("abc") && contains("def"))
    ///         visitor.visit_match_starts_with("/api");
    ///         visitor.visit_or_in();
    ///         visitor.visit_nested_start();
    ///         visitor.visit_match_regex("abc");
    ///         visitor.visit_match_regex("def");
    ///         visitor.visit_nested_finish();
    ///     }
    /// }
    /// ```
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
    /// use router_prefilter::matchers::{Matcher, MatcherVisitor};
    ///
    /// struct MultiVersionRoute;
    ///
    /// impl Matcher for MultiVersionRoute {
    ///     fn visit(&self, visitor: &mut MatcherVisitor) {
    ///         // (startsWith("/v1") && contains("abc") || (startsWith("/v2") && contains("def"))
    ///         visitor.visit_match_starts_with("/v1");
    ///         visitor.visit_match_regex(r"abc");
    ///         visitor.visit_or_in();
    ///         visitor.visit_match_starts_with("/v2");
    ///         visitor.visit_match_regex(r"def");
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
    /// use router_prefilter::matchers::{Matcher, MatcherVisitor};
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
    /// # Examples
    ///
    /// ```
    /// use router_prefilter::matchers::{Matcher, MatcherVisitor};
    ///
    /// struct ExactRoute(&'static str);
    ///
    /// impl Matcher for ExactRoute {
    ///     fn visit(&self, visitor: &mut MatcherVisitor) {
    ///         visitor.visit_match_equals(self.0);
    ///     }
    /// }
    ///
    /// let route = ExactRoute("/api/users");
    /// ```
    pub fn visit_match_equals(&mut self, equals: &str) {
        // for our purposes, equality and starting with are the same
        self.visit_match_starts_with(equals);
    }

    /// Processes a prefix match pattern.
    ///
    /// # Examples
    ///
    /// ```
    /// use router_prefilter::matchers::{Matcher, MatcherVisitor};
    ///
    /// struct PrefixRoute(&'static str);
    ///
    /// impl Matcher for PrefixRoute {
    ///     fn visit(&self, visitor: &mut MatcherVisitor) {
    ///         visitor.visit_match_starts_with(self.0);
    ///     }
    /// }
    ///
    /// let route = PrefixRoute("/api");
    /// ```
    pub fn visit_match_starts_with(&mut self, prefix: &str) {
        let new_prefixes = Some(BTreeSet::from([prefix.as_bytes().to_vec()]));
        let current = &mut self.frames.last_mut().unwrap().and_literal_prefixes;
        intersect_prefix_expansions(current, new_prefixes);
    }
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

fn extract_prefixes(hir: &Hir) -> Option<BTreeSet<Vec<u8>>> {
    if !hir
        .properties()
        .look_set_prefix()
        .contains_anchor_haystack()
    {
        return None;
    }
    let seq = literal::Extractor::new().extract(hir);
    seq.literals().map(|literals| {
        literals
            .iter()
            .map(|lit| lit.as_bytes().to_vec())
            .collect::<BTreeSet<_>>()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SimpleMatcher(&'static str);

    impl Matcher for SimpleMatcher {
        fn visit(&self, visitor: &mut MatcherVisitor) {
            visitor.visit_match_starts_with(self.0);
        }
    }

    #[test]
    fn test_matcher_by_reference() {
        let matcher = SimpleMatcher("/api");
        let mut visitor = MatcherVisitor::new();
        #[expect(clippy::needless_borrow)]
        (&matcher).visit(&mut visitor);
        let prefixes = visitor.finish();
        assert!(prefixes.is_some());
    }

    #[test]
    fn test_visit_match_regex_anchored() {
        let mut visitor = MatcherVisitor::new();
        visitor.visit_match_regex(r"^/api/.*");
        let prefixes = visitor.finish().unwrap();
        assert_eq!(prefixes.len(), 1);
        assert!(prefixes.contains(b"/api/".as_slice()));
    }

    #[test]
    fn test_visit_match_regex_unanchored() {
        let mut visitor = MatcherVisitor::new();
        visitor.visit_match_regex(r"/api/.*");
        // Unanchored regex should not extract prefixes
        assert!(visitor.finish().is_none());
    }

    #[test]
    fn test_visit_match_equals() {
        let mut visitor = MatcherVisitor::new();
        visitor.visit_match_equals("/api/users");
        let prefixes = visitor.finish().unwrap();
        assert_eq!(prefixes.len(), 1);
        assert!(prefixes.contains(b"/api/users".as_slice()));
    }

    #[test]
    fn test_visit_nested() {
        let mut visitor = MatcherVisitor::new();
        visitor.visit_nested_start();
        visitor.visit_match_starts_with("/api");
        visitor.visit_nested_finish();
        let prefixes = visitor.finish().unwrap();
        assert_eq!(prefixes.len(), 1);
        assert!(prefixes.contains(b"/api".as_slice()));
    }

    #[test]
    fn test_visit_or_in() {
        let mut visitor = MatcherVisitor::new();
        visitor.visit_match_starts_with("/v1");
        visitor.visit_or_in();
        visitor.visit_match_starts_with("/v2");
        let prefixes = visitor.finish().unwrap();
        assert_eq!(prefixes.len(), 2);
        assert!(prefixes.contains(b"/v1".as_slice()));
        assert!(prefixes.contains(b"/v2".as_slice()));
    }

    #[test]
    fn test_nested_with_or() {
        let mut visitor = MatcherVisitor::new();
        // Match: /v && (/v1 || /v2)
        visitor.visit_match_starts_with("/v");
        visitor.visit_nested_start();
        visitor.visit_match_starts_with("/v1");
        visitor.visit_or_in();
        visitor.visit_match_starts_with("/v2");
        visitor.visit_nested_finish();
        let prefixes = visitor.finish().unwrap();
        // Should have /v1 and /v2 (both start with /v)
        assert_eq!(prefixes.len(), 2);
        assert!(prefixes.contains(b"/v1".as_slice()));
        assert!(prefixes.contains(b"/v2".as_slice()));
    }

    #[test]
    fn test_union_prefixes_limited_exceeds_max() {
        let mut lhs = Some(BTreeSet::new());
        let mut rhs = BTreeSet::new();

        // Fill lhs with 60 items
        for i in 0..60 {
            lhs.as_mut().unwrap().insert(vec![i]);
        }

        // Fill rhs with 60 items
        for i in 60..120 {
            rhs.insert(vec![i]);
        }

        union_prefixes_limited(&mut lhs, Some(rhs), 100);
        // Should exceed limit and become None
        assert!(lhs.is_none());
    }

    #[test]
    fn test_intersect_prefix_expansions_both_none() {
        let mut lhs = None;
        let rhs = None;
        intersect_prefix_expansions(&mut lhs, rhs);
        assert!(lhs.is_none());
    }

    #[test]
    fn test_intersect_prefix_expansions_lhs_none() {
        let mut lhs = None;
        let mut rhs = BTreeSet::new();
        rhs.insert(b"/api".to_vec());
        intersect_prefix_expansions(&mut lhs, Some(rhs.clone()));
        assert_eq!(lhs, Some(rhs));
    }

    #[test]
    fn test_intersect_prefix_expansions_rhs_none() {
        let mut lhs_set = BTreeSet::new();
        lhs_set.insert(b"/api".to_vec());
        let mut lhs = Some(lhs_set.clone());
        intersect_prefix_expansions(&mut lhs, None);
        // Should remain unchanged when rhs is None
        assert_eq!(lhs, Some(lhs_set));
    }

    #[test]
    fn test_intersect_prefix_expansions_with_values() {
        // Test that intersect finds elements where one is a prefix of the other
        let mut lhs_set = BTreeSet::new();
        lhs_set.insert(b"/a".to_vec());

        let mut rhs_set = BTreeSet::new();
        rhs_set.insert(b"/api".to_vec());

        let mut lhs = Some(lhs_set);
        intersect_prefix_expansions(&mut lhs, Some(rhs_set));

        // /a is a prefix of /api, so /api should be in the result
        let result = lhs.unwrap();
        assert!(result.contains(b"/api".as_slice()));
    }

    #[test]
    fn test_extract_prefixes_anchored() {
        let hir = regex_syntax::parse(r"^/api/.*").unwrap();
        let prefixes = extract_prefixes(&hir);
        assert!(prefixes.is_some());
        let prefixes = prefixes.unwrap();
        assert!(prefixes.contains(b"/api/".as_slice()));
    }

    #[test]
    fn test_extract_prefixes_unanchored() {
        let hir = regex_syntax::parse(r"/api/.*").unwrap();
        let prefixes = extract_prefixes(&hir);
        // Unanchored patterns should return None
        assert!(prefixes.is_none());
    }
}
