// Switch between prefilter implementations by changing the path:
// - "inner_prefilter.rs" - AhoCorasick-based implementation
// - "inner_prefilter_btree.rs" - BTreeMap-based implementation
// - "inner_prefilter_fst.rs" - FST-based implementation
#[path = "inner_prefilter_btree.rs"]
mod inner_prefilter;

use inner_prefilter::AhoCorasickPrefilter;
use regex_syntax::hir::{Hir, literal};
use roaring::RoaringBitmap;
use roaring::bitmap::IntoIter;
use std::collections::BTreeSet;
use std::convert::Infallible;
use std::mem;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Case {
    Sensitive,
    Insensitive,
}

pub trait Matcher {
    fn visit(&self, visitor: &mut MatcherVisitor);
}

impl<M: Matcher> Matcher for &M {
    fn visit(&self, visitor: &mut MatcherVisitor) {
        M::visit(self, visitor);
    }
}

type Idx = u32;

#[derive(Debug, Clone)]
pub struct RouterPrefilter {
    always_possible_indexes: RoaringBitmap,
    prefilter: AhoCorasickPrefilter,
}

impl RouterPrefilter {
    pub fn new<M, I>(matchers: I, max_unfiltered: usize) -> Option<Self>
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
        let mut unfiltered = RoaringBitmap::new();
        unfiltered.insert_range(..num_unfiltered);

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
        let prefilter = AhoCorasickPrefilter::new(&patterns, pattern_indexes)?;
        Some(Self {
            always_possible_indexes: unfiltered,
            prefilter,
        })
    }

    pub fn possible_matches<'a>(&'a self, value: &'a str) -> RouterPrefilterIter<'a> {
        let value = value.as_bytes();
        let first_filtered_idx = self.prefilter.first_index();
        if self.always_possible_indexes.min().unwrap_or(0) >= first_filtered_idx {
            RouterPrefilterIter(RouterPrefilterIterState::Both(
                make_combined_prefilter_iter(&self, value),
            ))
        } else {
            RouterPrefilterIter(RouterPrefilterIterState::BeforePrefilter {
                unfiltered_it: self.always_possible_indexes.range(..first_filtered_idx),
                router_prefilter: self,
                s: value,
            })
        }
    }
}

pub struct RouterPrefilterIter<'a>(RouterPrefilterIterState<'a>);

impl Iterator for RouterPrefilterIter<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0 {
            RouterPrefilterIterState::BeforePrefilter {
                ref mut unfiltered_it,
                router_prefilter,
                s,
            } => {
                let Some(idx) = unfiltered_it.next() else {
                    let mut it = make_combined_prefilter_iter(&router_prefilter, s);
                    let result = it.next().map(|i| usize::try_from(i).unwrap());
                    self.0 = RouterPrefilterIterState::Both(it);
                    return result;
                };
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
                ref unfiltered_it, ..
            } => (unfiltered_it.len(), None),
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
                unfiltered_it,
                router_prefilter,
                s,
            } => {
                init = unfiltered_it
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
fn make_combined_prefilter_iter(router_prefilter: &RouterPrefilter, s: &[u8]) -> IntoIter {
    let first_idx = router_prefilter.prefilter.first_index();
    let mut indexes = router_prefilter.prefilter.check(s);
    if router_prefilter
        .always_possible_indexes
        .max()
        .is_some_and(|max| max > first_idx)
    {
        indexes |= &router_prefilter.always_possible_indexes;
    }
    indexes.into_range(first_idx..)
}

enum RouterPrefilterIterState<'a> {
    BeforePrefilter {
        unfiltered_it: roaring::bitmap::Iter<'a>,
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
        frame
            .finish()
            .map(|set| {
                let mut seq = literal::Seq::new(set);
                seq.optimize_for_prefix_by_preference();
                seq
            })
            .unwrap_or_else(literal::Seq::infinite)
    }

    pub fn visit_nested_start(&mut self) {
        self.frames.push(Frame::default());
    }

    pub fn visit_nested_finish(&mut self) {
        let frame = self
            .frames
            .pop()
            .expect("every finish should match with a start");
        let new_inner = frame.finish();
        intersect_prefix_expansions(&mut self.current_frame().and_literal_prefixes, new_inner);
    }

    pub fn visit_or_in(&mut self) {
        let frame = self.current_frame();
        let new_and = frame.and_literal_prefixes.take();
        union_prefixes_limited(&mut frame.or_literal_prefixes, new_and, 100);
    }

    pub fn visit_match_regex(&mut self, regex: &str) {
        // TODO: Should we return an error? Or panic instead?
        let hir = regex_syntax::parse(regex).unwrap_or_else(|_| Hir::fail());
        let current = &mut self.frames.last_mut().unwrap().and_literal_prefixes;
        let new_prefixes = extract_prefixes(&hir);
        intersect_prefix_expansions(current, new_prefixes);
    }

    pub fn visit_match_equals(&mut self, equals: &str, case: Case) {
        // for our purposes, equality and starting with are the same
        self.visit_match_starts_with(equals, case);
    }

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

    #[test]
    fn extractor() {
        let mut extractor = MatcherVisitor::new();
        extractor.visit_match_starts_with("/opt/123", Case::Sensitive);
        extractor.visit_match_starts_with("/opt/123", Case::Sensitive);
        extractor.visit_or_in();
        extractor.visit_match_regex("abc");
        extractor.visit_match_regex("(^/opt|^/bob)/abcd.*");
        extractor.visit_match_regex("^/bob");

        let values = extractor.finish();
        panic!("{:?}", values);
    }
}
