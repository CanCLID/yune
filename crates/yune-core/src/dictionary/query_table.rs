use std::borrow::Cow;
use std::collections::{btree_map, BTreeMap};
use std::slice;

use crate::{Candidate, CandidateSource};

#[derive(Clone)]
pub(crate) struct LookupCandidate<'a> {
    text: Cow<'a, str>,
    raw_comment: Cow<'a, str>,
    raw_quality: f32,
    source_hint: CandidateSource,
}

impl<'a> LookupCandidate<'a> {
    #[must_use]
    pub(crate) fn new(
        text: impl Into<Cow<'a, str>>,
        raw_comment: impl Into<Cow<'a, str>>,
        raw_quality: f32,
        source_hint: CandidateSource,
    ) -> Self {
        crate::m37_record_lookup_view();
        Self {
            text: text.into(),
            raw_comment: raw_comment.into(),
            raw_quality,
            source_hint,
        }
    }

    #[must_use]
    pub(crate) fn from_candidate(candidate: &'a Candidate) -> Self {
        Self::new(
            &candidate.text,
            &candidate.comment,
            candidate.quality,
            candidate.source.clone(),
        )
    }

    #[must_use]
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub(crate) fn raw_comment(&self) -> &str {
        &self.raw_comment
    }

    #[must_use]
    pub(crate) fn raw_quality(&self) -> f32 {
        self.raw_quality
    }

    #[must_use]
    pub(crate) fn source_hint(&self) -> CandidateSource {
        self.source_hint.clone()
    }

    #[must_use]
    pub(crate) fn to_candidate(&self) -> Candidate {
        crate::m37_record_owned_candidate_materialized();
        Candidate {
            text: self.text.to_string(),
            comment: self.raw_comment.to_string(),
            preedit: None,
            source: self.source_hint(),
            quality: self.raw_quality,
        }
    }
}

pub(crate) struct LookupCandidateEntry<'a> {
    code: Cow<'a, str>,
    candidate: LookupCandidate<'a>,
}

impl<'a> LookupCandidateEntry<'a> {
    #[must_use]
    pub(crate) fn new(code: impl Into<Cow<'a, str>>, candidate: LookupCandidate<'a>) -> Self {
        Self {
            code: code.into(),
            candidate,
        }
    }

    #[must_use]
    pub(crate) fn into_parts(self) -> (Cow<'a, str>, LookupCandidate<'a>) {
        (self.code, self.candidate)
    }
}

pub(crate) trait TableLookup {
    type ExactCandidates<'a>: Iterator<Item = LookupCandidate<'a>>
    where
        Self: 'a;
    type PrefixCandidates<'a>: Iterator<Item = LookupCandidateEntry<'a>>
    where
        Self: 'a;
    type AllCodes<'a>: Iterator<Item = Cow<'a, str>>
    where
        Self: 'a;

    fn has_code(&self, code: &str) -> bool;

    fn exact_candidates<'a>(&'a self, code: &'a str) -> Self::ExactCandidates<'a>;

    fn prefix_candidates<'a>(&'a self, prefix: &'a str) -> Self::PrefixCandidates<'a>;

    #[allow(dead_code)]
    fn all_codes(&self) -> Self::AllCodes<'_>;
}

pub(crate) struct HeapExactCandidates<'a> {
    inner: Option<slice::Iter<'a, Candidate>>,
}

impl<'a> Iterator for HeapExactCandidates<'a> {
    type Item = LookupCandidate<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .as_mut()
            .and_then(Iterator::next)
            .map(LookupCandidate::from_candidate)
    }
}

pub(crate) struct HeapPrefixCandidates<'a> {
    prefix: &'a str,
    inner: btree_map::Range<'a, String, Vec<Candidate>>,
    current_code: Option<&'a str>,
    current_candidates: Option<slice::Iter<'a, Candidate>>,
    done: bool,
}

impl<'a> Iterator for HeapPrefixCandidates<'a> {
    type Item = LookupCandidateEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let (Some(code), Some(candidates)) =
                (self.current_code, self.current_candidates.as_mut())
            {
                if let Some(candidate) = candidates.next() {
                    return Some(LookupCandidateEntry::new(
                        code,
                        LookupCandidate::from_candidate(candidate),
                    ));
                }
                self.current_code = None;
                self.current_candidates = None;
            }

            if self.done {
                return None;
            }
            let Some((code, candidates)) = self.inner.next() else {
                self.done = true;
                return None;
            };
            if !code.starts_with(self.prefix) {
                self.done = true;
                return None;
            }
            self.current_code = Some(code.as_str());
            self.current_candidates = Some(candidates.iter());
        }
    }
}

pub(crate) struct HeapAllCodes<'a> {
    inner: btree_map::Keys<'a, String, Vec<Candidate>>,
}

impl<'a> Iterator for HeapAllCodes<'a> {
    type Item = Cow<'a, str>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|code| Cow::Borrowed(code.as_str()))
    }
}

impl TableLookup for BTreeMap<String, Vec<Candidate>> {
    type ExactCandidates<'a> = HeapExactCandidates<'a>;
    type PrefixCandidates<'a> = HeapPrefixCandidates<'a>;
    type AllCodes<'a> = HeapAllCodes<'a>;

    fn has_code(&self, code: &str) -> bool {
        self.contains_key(code)
    }

    fn exact_candidates<'a>(&'a self, code: &'a str) -> Self::ExactCandidates<'a> {
        HeapExactCandidates {
            inner: self.get(code).map(|candidates| candidates.iter()),
        }
    }

    fn prefix_candidates<'a>(&'a self, prefix: &'a str) -> Self::PrefixCandidates<'a> {
        HeapPrefixCandidates {
            prefix,
            inner: self.range(prefix.to_owned()..),
            current_code: None,
            current_candidates: None,
            done: false,
        }
    }

    fn all_codes(&self) -> Self::AllCodes<'_> {
        HeapAllCodes { inner: self.keys() }
    }
}
