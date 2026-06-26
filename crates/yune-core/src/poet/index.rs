use std::cmp::Ordering;

use super::ModelEntry;

#[derive(Clone, Debug, Default)]
pub(super) struct SentenceLookupIndex {
    ranges: Box<[SentenceCodeRange]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SentenceCodeRange {
    start: u32,
    end: u32,
}

#[derive(Clone, Debug)]
pub(super) struct SentenceCodeSpan {
    pub(super) end: usize,
    pub(super) end_index: usize,
}

#[derive(Clone, Debug, Default)]
pub(super) struct SentencePhraseWalk {
    pub(super) spans: Vec<SentenceCodeSpan>,
    pub(super) prefix_hits: usize,
    pub(super) prefix_misses: usize,
    pub(super) prefix_early_breaks: usize,
    pub(super) exact_range_misses: usize,
    pub(super) nodes_visited: usize,
    pub(super) entry_ranges_emitted: usize,
}

impl SentenceLookupIndex {
    pub(super) fn build(entries: &[ModelEntry]) -> Self {
        let mut ranges = Vec::new();
        let mut start = 0usize;
        while start < entries.len() {
            let code = entries[start].code.as_str();
            let mut end = start + 1;
            while end < entries.len() && entries[end].code == code {
                end += 1;
            }
            ranges.push(SentenceCodeRange {
                start: u32::try_from(start).expect("sentence model range start exceeds u32"),
                end: u32::try_from(end).expect("sentence model range end exceeds u32"),
            });
            start = end;
        }
        Self {
            ranges: ranges.into_boxed_slice(),
        }
    }

    pub(super) fn entries_for_code<'a>(
        &self,
        entries: &'a [ModelEntry],
        code: &str,
    ) -> Option<&'a [ModelEntry]> {
        let index = self.find_exact_range(entries, code, 0, self.ranges.len())?;
        let range = self.ranges[index];
        Some(&entries[range.start as usize..range.end as usize])
    }

    pub(super) fn walk_from(
        &self,
        entries: &[ModelEntry],
        input: &str,
        boundaries: &[usize],
        start_index: usize,
    ) -> SentencePhraseWalk {
        let mut walk = SentencePhraseWalk::default();
        let mut range_start = 0usize;
        let mut range_end = self.ranges.len();
        for (end_index, end) in boundaries.iter().copied().enumerate().skip(start_index + 1) {
            let code = &input[boundaries[start_index]..end];
            let (next_start, next_end) = self.prefix_range(entries, code, range_start, range_end);
            if next_start == next_end {
                walk.prefix_misses += 1;
                walk.prefix_early_breaks += 1;
                break;
            }
            range_start = next_start;
            range_end = next_end;
            walk.prefix_hits += 1;
            walk.nodes_visited += 1;
            if self
                .find_exact_range(entries, code, range_start, range_end)
                .is_some()
            {
                walk.entry_ranges_emitted += 1;
                walk.spans.push(SentenceCodeSpan { end, end_index });
            } else {
                walk.exact_range_misses += 1;
            }
        }
        walk
    }

    fn find_exact_range(
        &self,
        entries: &[ModelEntry],
        code: &str,
        start: usize,
        end: usize,
    ) -> Option<usize> {
        let mut low = start;
        let mut high = end;
        while low < high {
            let mid = low + (high - low) / 2;
            match self.range_code(entries, mid).cmp(code) {
                Ordering::Less => low = mid + 1,
                Ordering::Equal => return Some(mid),
                Ordering::Greater => high = mid,
            }
        }
        None
    }

    fn prefix_range(
        &self,
        entries: &[ModelEntry],
        prefix: &str,
        start: usize,
        end: usize,
    ) -> (usize, usize) {
        let lower = self.lower_bound(entries, prefix, start, end);
        if lower == end || !self.range_code(entries, lower).starts_with(prefix) {
            return (lower, lower);
        }
        let upper = match next_prefix_bound(prefix) {
            Some(bound) => self.lower_bound(entries, &bound, lower, end),
            None => end,
        };
        (lower, upper)
    }

    fn lower_bound(&self, entries: &[ModelEntry], value: &str, start: usize, end: usize) -> usize {
        let mut low = start;
        let mut high = end;
        while low < high {
            let mid = low + (high - low) / 2;
            if self.range_code(entries, mid) < value {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        low
    }

    fn range_code<'a>(&self, entries: &'a [ModelEntry], index: usize) -> &'a str {
        entries[self.ranges[index].start as usize].code.as_str()
    }
}

fn next_prefix_bound(prefix: &str) -> Option<String> {
    let mut bytes = prefix.as_bytes().to_vec();
    for index in (0..bytes.len()).rev() {
        if bytes[index] != u8::MAX {
            bytes[index] += 1;
            bytes.truncate(index + 1);
            return String::from_utf8(bytes).ok();
        }
    }
    None
}
