use regex_syntax::hir::{Hir, literal};
use roaring::RoaringBitmap;
use std::collections::{BTreeSet, HashMap};
use std::convert::Infallible;
use std::marker::PhantomData;
use std::mem;
use smallvec::SmallVec;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Case {
    Sensitive,
    Insensitive,
}

pub trait MatcherVisitor {
    fn visit_nested_start(&mut self);
    fn visit_nested_finish(&mut self);

    /// This method should be called between children of an `or` expression
    ///
    /// It is valid to call this even if no matches (regex, starts with) were visited:
    /// this can be the case if e.g. an `or` node is checking some other fields only.
    fn visit_or_in(&mut self);

    fn visit_match_regex(&mut self, regex: &str);
    fn visit_match_equals(&mut self, equals: &str, case: Case);
    fn visit_match_starts_with(&mut self, prefix: &str, case: Case);
}

pub trait Matcher {
    fn visit<V: MatcherVisitor>(&self, visitor: &mut V);
}

type Idx = u32;

#[derive(Debug, Default)]
pub struct RouterPrefilterBuilder {
    next_idx: Idx,
    always_possible_indexes: RoaringBitmap,
    possible_matches: Vec<(String, Idx)>,
}

impl RouterPrefilterBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_matcher<M: Matcher>(&mut self, matcher: &M) {
        let idx = self.next_idx;
        self.next_idx += 1;

        let mut extractor = PrefixExtractorVisitor::new();
        matcher.visit(&mut extractor);
        let extracted_prefixes = extractor.finish();

        if let Some(prefixes) = extracted_prefixes.literals() {
            for prefix in prefixes {
                self.possible_matches
                    .push((str::from_utf8(prefix.as_bytes()).unwrap().to_owned(), idx));
            }
        } else {
            self.always_possible_indexes.insert(idx);
        }
    }

    pub fn build(self) -> Option<RouterPrefilter> {
        let Self {
            next_idx: _,
            always_possible_indexes,
            possible_matches,
        } = self;
        if possible_matches.is_empty() {
            return None;
        }
        let pattern_to_key = possible_matches
            .iter()
            .enumerate()
            .map(|(i, &(_, idx))| (aho_corasick::PatternID::must(i), idx))
            .collect();
        let prefilter = aho_corasick::AhoCorasickBuilder::new()
            .start_kind(aho_corasick::StartKind::Anchored)
            .build(possible_matches.iter().map(|(pat, _)| pat));
        let prefilter = match prefilter {
            Ok(prefilter) => prefilter,
            Err(_) => return None,
        };
        Some(RouterPrefilter {
            always_possible_indexes,
            prefilter,
            pattern_to_index: pattern_to_key,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RouterPrefilter {
    always_possible_indexes: RoaringBitmap,
    prefilter: aho_corasick::AhoCorasick,
    pattern_to_index: HashMap<aho_corasick::PatternID, Idx>,
}

impl RouterPrefilter {
    pub fn possible_matches<'a>(&'a self, value: &'a str) -> RouterPrefilterIter<'a> {
        let mut possible_indexes = self.always_possible_indexes.clone();
        let mut state = aho_corasick::automaton::OverlappingState::start();

        loop {
            self.prefilter.find_overlapping(
                aho_corasick::Input::new(value).anchored(aho_corasick::Anchored::Yes),
                &mut state,
            );
            match state.get_match() {
                Some(m) => {
                    possible_indexes.insert(self.pattern_to_index[&m.pattern()]);
                }
                None => break,
            }
        }

        RouterPrefilterIter {
            indexes: possible_indexes.into_iter(),
            _phantom: PhantomData,
        }
    }
}

pub struct RouterPrefilterIter<'a> {
    indexes: roaring::bitmap::IntoIter,
    // We don't currently borrow anything, but make sure we could without a backward incompatible change
    _phantom: PhantomData<(&'a RouterPrefilter, &'a str)>,
}

impl Iterator for RouterPrefilterIter<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        self.indexes.next().map(|idx| usize::try_from(idx).unwrap())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.indexes.size_hint()
    }

    fn fold<B, F>(self, init: B, mut f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        self.indexes
            .fold(init, move |acc, idx| f(acc, usize::try_from(idx).unwrap()))
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

#[derive(Debug)]
struct PrefixExtractorVisitor {
    frames: SmallVec<[Frame; 4]>,
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

impl PrefixExtractorVisitor {
    fn new() -> Self {
        Self {
            frames: smallvec::smallvec![Frame::default()],
        }
    }

    fn current_frame(&mut self) -> &mut Frame {
        self.frames.last_mut().unwrap()
    }

    fn finish(self) -> literal::Seq {
        let Self { mut frames } = self;
        assert_eq!(frames.len(), 1);
        let frame = frames.pop().unwrap();
        frame
            .finish()
            .map(|set| {
                let mut seq = literal::Seq::new(set);
                seq.optimize_for_prefix_by_preference();
                seq
            })
            .unwrap_or_else(literal::Seq::infinite)
    }
}

impl MatcherVisitor for PrefixExtractorVisitor {
    fn visit_nested_start(&mut self) {
        self.frames.push(Frame::default());
    }

    fn visit_nested_finish(&mut self) {
        let frame = self
            .frames
            .pop()
            .expect("every finish should match with a start");
        let new_inner = frame.finish();
        intersect_prefix_expansions(&mut self.current_frame().and_literal_prefixes, new_inner);
    }

    fn visit_or_in(&mut self) {
        let frame = self.current_frame();
        let new_and = frame.and_literal_prefixes.take();
        union_prefixes_limited(&mut frame.or_literal_prefixes, new_and, 100);
    }

    fn visit_match_regex(&mut self, regex: &str) {
        // TODO: Should we return an error? Or panic instead?
        let hir = regex_syntax::parse(regex).unwrap_or_else(|_| Hir::fail());
        let current = &mut self.frames.last_mut().unwrap().and_literal_prefixes;
        let new_prefixes = extract_prefixes(&hir);
        intersect_prefix_expansions(current, new_prefixes);
    }

    fn visit_match_equals(&mut self, equals: &str, case: Case) {
        // for our purposes, equality and starting with are the same
        self.visit_match_starts_with(equals, case);
    }

    fn visit_match_starts_with(&mut self, prefix: &str, case: Case) {
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
        let mut extractor = PrefixExtractorVisitor::new();
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
