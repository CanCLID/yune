#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DartsDoubleArray {
    units: Vec<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DartsMatch {
    pub value: u32,
    pub length: usize,
}

pub(crate) const MAX_DARTS_PREDICTIVE_NODE_WORK: usize = 65_536;
pub(crate) const MAX_DARTS_PREDICTIVE_PATH_DEPTH: usize = 256;

#[derive(Clone, Copy)]
pub(crate) struct DartsPredictiveState {
    pub(crate) node_pos: usize,
    pub(crate) unit: u32,
    pub(crate) length: usize,
    pub(crate) parent: Option<usize>,
    pub(crate) cycle_closed: bool,
}

pub(crate) enum DartsPredictivePush {
    Accepted(usize),
    ClosedCycle,
    MalformedCycle,
    Exhausted,
}

pub(crate) fn push_darts_predictive_state(
    states: &mut Vec<DartsPredictiveState>,
    parent: usize,
    node_pos: usize,
    unit: u32,
    length: usize,
    leaf_pos: Option<usize>,
) -> DartsPredictivePush {
    if length > MAX_DARTS_PREDICTIVE_PATH_DEPTH || states.len() >= MAX_DARTS_PREDICTIVE_NODE_WORK {
        return DartsPredictivePush::Exhausted;
    }
    let Some(parent_state) = states.get(parent).copied() else {
        return DartsPredictivePush::Exhausted;
    };
    let mut repeated_ancestor = false;
    let mut leaf_overlaps_ancestor = false;
    let mut ancestor = Some(parent);
    while let Some(index) = ancestor {
        let Some(state) = states.get(index) else {
            return DartsPredictivePush::Exhausted;
        };
        if state.node_pos == node_pos {
            repeated_ancestor = true;
        }
        leaf_overlaps_ancestor |= leaf_pos == Some(state.node_pos);
        ancestor = state.parent;
    }
    // A DARTS value slot can resemble a labeled node during alphabet
    // enumeration. Pinned real artifacts contain one such ancestry closure
    // whose separate leaf remains valid, followed by finite side branches.
    // Admit that closure once. A leaf pointer that aliases the active path is
    // structurally malformed, while any later closure ends the bounded lap.
    if repeated_ancestor && leaf_overlaps_ancestor {
        return DartsPredictivePush::MalformedCycle;
    }
    if repeated_ancestor && parent_state.cycle_closed {
        return DartsPredictivePush::ClosedCycle;
    }
    let index = states.len();
    states.push(DartsPredictiveState {
        node_pos,
        unit,
        length,
        parent: Some(parent),
        cycle_closed: parent_state.cycle_closed || repeated_ancestor,
    });
    DartsPredictivePush::Accepted(index)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DartsDoubleArrayError {
    Empty,
    DuplicateKey,
    EmptyKey,
    ValueOutOfRange,
    OffsetOutOfRange,
}

pub(crate) trait DartsKeyValue {
    fn key_bytes(&self) -> &[u8];
    fn value(&self) -> u32;
}

#[derive(Clone, Copy)]
struct BorrowedDartsKey<'a> {
    key: &'a [u8],
    value: u32,
}

impl DartsKeyValue for BorrowedDartsKey<'_> {
    fn key_bytes(&self) -> &[u8] {
        self.key
    }

    fn value(&self) -> u32 {
        self.value
    }
}

#[derive(Clone, Copy)]
struct CompactTrieNode {
    value: u32,
    first_child: u32,
    child_count: u32,
}

impl Default for CompactTrieNode {
    fn default() -> Self {
        Self {
            value: u32::MAX,
            first_child: 0,
            child_count: 0,
        }
    }
}

#[derive(Clone, Copy, Default)]
struct CompactTrieEdge {
    child: u32,
    label: u8,
}

struct CompactTrie {
    nodes: Vec<CompactTrieNode>,
    edges: Vec<CompactTrieEdge>,
}

impl DartsDoubleArray {
    const HAS_LEAF: u32 = 1 << 8;
    const VALUE_MASK: u32 = (1 << 31) - 1;
    const LABEL_MASK: u32 = (1 << 31) | 0xff;
    const LARGE_OFFSET_THRESHOLD: u32 = 1 << 21;
    const MAX_OFFSET: u32 = 1 << 29;

    pub fn build<K>(keys: &[(K, u32)]) -> Result<Self, DartsDoubleArrayError>
    where
        K: AsRef<str>,
    {
        let mut byte_keys = keys
            .iter()
            .map(|(key, value)| BorrowedDartsKey {
                key: key.as_ref().as_bytes(),
                value: *value,
            })
            .collect::<Vec<_>>();
        byte_keys.sort_unstable_by(|left, right| left.key.cmp(right.key));
        Self::build_sorted_key_values(&byte_keys)
    }

    pub fn build_bytes<K>(keys: &[(K, u32)]) -> Result<Self, DartsDoubleArrayError>
    where
        K: AsRef<[u8]>,
    {
        let mut byte_keys = keys
            .iter()
            .map(|(key, value)| BorrowedDartsKey {
                key: key.as_ref(),
                value: *value,
            })
            .collect::<Vec<_>>();
        byte_keys.sort_unstable_by(|left, right| left.key.cmp(right.key));
        Self::build_sorted_key_values(&byte_keys)
    }

    pub(crate) fn build_sorted_key_values<K>(keys: &[K]) -> Result<Self, DartsDoubleArrayError>
    where
        K: DartsKeyValue,
    {
        validate_sorted_keys(keys)?;
        let trie = CompactTrie::from_sorted_keys(keys)?;

        let mut builder = DartsBuilder {
            trie,
            units: vec![0],
            used: vec![true],
        };
        builder.assign(0, 0, 0)?;
        Ok(Self {
            units: builder.units,
        })
    }

    pub fn from_units(units: Vec<u32>) -> Result<Self, DartsDoubleArrayError> {
        if units.is_empty() {
            return Err(DartsDoubleArrayError::Empty);
        }
        Ok(Self { units })
    }

    #[must_use]
    pub fn units(&self) -> &[u32] {
        &self.units
    }

    #[must_use]
    pub(crate) fn units_capacity(&self) -> usize {
        self.units.capacity()
    }

    #[must_use]
    pub fn exact_match(&self, key: &str) -> Option<u32> {
        self.exact_match_bytes(key.as_bytes())
    }

    #[must_use]
    pub fn exact_match_bytes(&self, key: &[u8]) -> Option<u32> {
        let mut node_pos = 0usize;
        let mut unit = *self.units.get(node_pos)?;
        for byte in key {
            node_pos ^= usize::try_from(Self::offset(unit)).ok()? ^ usize::from(*byte);
            unit = *self.units.get(node_pos)?;
            if Self::label(unit) != u32::from(*byte) {
                return None;
            }
        }
        if !Self::has_leaf(unit) {
            return None;
        }
        let leaf_pos = node_pos ^ usize::try_from(Self::offset(unit)).ok()?;
        self.units.get(leaf_pos).map(|leaf| Self::value(*leaf))
    }

    pub(crate) fn predictive_search_with_alphabet(
        &self,
        prefix: &[u8],
        alphabet: &[u8],
        limit: usize,
    ) -> Vec<DartsMatch> {
        let effective_limit = if limit == 0 { usize::MAX } else { limit };
        let mut node_pos = 0usize;
        let Some(mut unit) = self.units.get(node_pos).copied() else {
            return Vec::new();
        };
        let mut states = vec![DartsPredictiveState {
            node_pos,
            unit,
            length: 0,
            parent: None,
            cycle_closed: false,
        }];
        let mut state_index = 0usize;
        for byte in prefix {
            let Ok(offset) = usize::try_from(Self::offset(unit)) else {
                return Vec::new();
            };
            node_pos ^= offset ^ usize::from(*byte);
            let Some(next_unit) = self.units.get(node_pos).copied() else {
                return Vec::new();
            };
            if Self::label(next_unit) != u32::from(*byte) {
                return Vec::new();
            }
            let next_length = states[state_index].length.saturating_add(1);
            let next_leaf_pos = Self::has_leaf(next_unit)
                .then(|| node_pos ^ usize::try_from(Self::offset(next_unit)).unwrap_or(usize::MAX));
            let DartsPredictivePush::Accepted(next_state_index) = push_darts_predictive_state(
                &mut states,
                state_index,
                node_pos,
                next_unit,
                next_length,
                next_leaf_pos,
            ) else {
                return Vec::new();
            };
            state_index = next_state_index;
            unit = next_unit;
        }

        let mut matches = Vec::new();
        if Self::has_leaf(unit) {
            let Ok(offset) = usize::try_from(Self::offset(unit)) else {
                return matches;
            };
            if let Some(leaf) = self.units.get(node_pos ^ offset) {
                matches.push(DartsMatch {
                    value: Self::value(*leaf),
                    length: prefix.len(),
                });
                if matches.len() >= effective_limit {
                    return matches;
                }
            }
        }

        let mut queue = std::collections::VecDeque::from([state_index]);
        while let Some(parent_index) = queue.pop_front() {
            let parent = states[parent_index];
            let Ok(parent_offset) = usize::try_from(Self::offset(parent.unit)) else {
                continue;
            };
            for byte in alphabet.iter().copied().filter(|byte| *byte != 0) {
                let child_pos = parent.node_pos ^ parent_offset ^ usize::from(byte);
                let Some(child_unit) = self.units.get(child_pos).copied() else {
                    continue;
                };
                if Self::label(child_unit) != u32::from(byte) {
                    continue;
                }
                let child_len = parent.length.saturating_add(1);
                let child_leaf_pos = Self::has_leaf(child_unit).then(|| {
                    child_pos ^ usize::try_from(Self::offset(child_unit)).unwrap_or(usize::MAX)
                });
                let child_index = match push_darts_predictive_state(
                    &mut states,
                    parent_index,
                    child_pos,
                    child_unit,
                    child_len,
                    child_leaf_pos,
                ) {
                    DartsPredictivePush::Accepted(index) => Some(index),
                    // One bounded closure has already exposed its finite side
                    // branches. Preserve DARTS's terminal leaf observation,
                    // but do not enqueue another lap.
                    DartsPredictivePush::ClosedCycle => None,
                    // A leaf slot overlapping the active ancestry is a
                    // structurally malformed DARTS path, not a false-positive
                    // value-slot transition. Reject it before observing leaf.
                    DartsPredictivePush::MalformedCycle => return Vec::new(),
                    DartsPredictivePush::Exhausted => return Vec::new(),
                };
                if let Some(child_index) = child_index {
                    queue.push_back(child_index);
                }
                if !Self::has_leaf(child_unit) {
                    continue;
                }
                let Ok(child_offset) = usize::try_from(Self::offset(child_unit)) else {
                    continue;
                };
                if let Some(leaf) = self.units.get(child_pos ^ child_offset) {
                    matches.push(DartsMatch {
                        value: Self::value(*leaf),
                        length: child_len,
                    });
                    if matches.len() >= effective_limit {
                        return matches;
                    }
                }
            }
        }
        matches
    }

    #[must_use]
    pub fn common_prefix_search(&self, key: &str) -> Vec<DartsMatch> {
        self.common_prefix_search_bytes(key.as_bytes())
    }

    #[must_use]
    pub fn common_prefix_search_bytes(&self, key: &[u8]) -> Vec<DartsMatch> {
        self.common_prefix_search_bytes_from_prefix_with_limit(&[], key, usize::MAX)
    }

    #[must_use]
    pub(crate) fn common_prefix_search_bytes_from_prefix_with_limit(
        &self,
        prefix: &[u8],
        key: &[u8],
        limit: usize,
    ) -> Vec<DartsMatch> {
        let mut matches = Vec::new();
        if limit == 0 {
            return matches;
        }
        let mut node_pos = 0usize;
        let Some(mut unit) = self.units.get(node_pos).copied() else {
            return matches;
        };
        let Ok(offset) = usize::try_from(Self::offset(unit)) else {
            return matches;
        };
        node_pos ^= offset;

        for byte in prefix {
            node_pos ^= usize::from(*byte);
            let Some(next_unit) = self.units.get(node_pos).copied() else {
                return matches;
            };
            unit = next_unit;
            if Self::label(unit) != u32::from(*byte) {
                return matches;
            }
            let Ok(offset) = usize::try_from(Self::offset(unit)) else {
                return matches;
            };
            node_pos ^= offset;
        }

        for (index, byte) in key.iter().enumerate() {
            node_pos ^= usize::from(*byte);
            let Some(next_unit) = self.units.get(node_pos).copied() else {
                return matches;
            };
            unit = next_unit;
            if Self::label(unit) != u32::from(*byte) {
                return matches;
            }
            let Ok(offset) = usize::try_from(Self::offset(unit)) else {
                return matches;
            };
            node_pos ^= offset;
            if Self::has_leaf(unit) {
                if let Some(leaf) = self.units.get(node_pos) {
                    matches.push(DartsMatch {
                        value: Self::value(*leaf),
                        length: index + 1,
                    });
                    if matches.len() >= limit {
                        break;
                    }
                }
            }
        }
        matches
    }

    const fn unit(offset: u32, has_leaf: bool, label: u8) -> u32 {
        let encoded_offset = if offset >= Self::LARGE_OFFSET_THRESHOLD {
            ((offset >> 8) << 10) | (1 << 9)
        } else {
            offset << 10
        };
        encoded_offset | if has_leaf { Self::HAS_LEAF } else { 0 } | label as u32
    }

    const fn has_leaf(unit: u32) -> bool {
        ((unit >> 8) & 1) == 1
    }

    const fn value(unit: u32) -> u32 {
        unit & Self::VALUE_MASK
    }

    const fn label(unit: u32) -> u32 {
        unit & Self::LABEL_MASK
    }

    const fn offset(unit: u32) -> u32 {
        (unit >> 10) << ((unit & (1 << 9)) >> 6)
    }
}

fn validate_sorted_keys<K>(keys: &[K]) -> Result<(), DartsDoubleArrayError>
where
    K: DartsKeyValue,
{
    if keys.is_empty() {
        return Err(DartsDoubleArrayError::Empty);
    }
    let mut previous = None;
    for key in keys {
        if key.value() > DartsDoubleArray::VALUE_MASK {
            return Err(DartsDoubleArrayError::ValueOutOfRange);
        }
        let bytes = key.key_bytes();
        if bytes.is_empty() {
            return Err(DartsDoubleArrayError::EmptyKey);
        }
        if previous.is_some_and(|previous| previous == bytes) {
            return Err(DartsDoubleArrayError::DuplicateKey);
        }
        debug_assert!(previous.map_or(true, |previous| previous < bytes));
        previous = Some(bytes);
    }
    Ok(())
}

impl CompactTrie {
    fn from_sorted_keys<K>(keys: &[K]) -> Result<Self, DartsDoubleArrayError>
    where
        K: DartsKeyValue,
    {
        let mut trie = Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        };
        let root = trie.push_node(keys, 0)?;
        debug_assert_eq!(root, 0);
        Ok(trie)
    }

    fn push_node<K>(&mut self, keys: &[K], depth: usize) -> Result<u32, DartsDoubleArrayError>
    where
        K: DartsKeyValue,
    {
        let node_index = self.nodes.len();
        self.nodes.push(CompactTrieNode::default());
        let node_index_u32 =
            u32::try_from(node_index).map_err(|_| DartsDoubleArrayError::OffsetOutOfRange)?;

        let mut cursor = 0usize;
        if keys[0].key_bytes().len() == depth {
            self.nodes[node_index].value = keys[0].value();
            cursor = 1;
        }

        let mut child_count = 0usize;
        let mut group_start = cursor;
        while group_start < keys.len() {
            let key = keys[group_start].key_bytes();
            debug_assert!(key.len() > depth);
            let label = key[depth];
            child_count = child_count
                .checked_add(1)
                .ok_or(DartsDoubleArrayError::OffsetOutOfRange)?;
            group_start += 1;
            while group_start < keys.len()
                && keys[group_start].key_bytes().get(depth).copied() == Some(label)
            {
                group_start += 1;
            }
        }

        let first_child = self.edges.len();
        self.edges.resize(
            first_child
                .checked_add(child_count)
                .ok_or(DartsDoubleArrayError::OffsetOutOfRange)?,
            CompactTrieEdge::default(),
        );
        self.nodes[node_index].first_child =
            u32::try_from(first_child).map_err(|_| DartsDoubleArrayError::OffsetOutOfRange)?;
        self.nodes[node_index].child_count =
            u32::try_from(child_count).map_err(|_| DartsDoubleArrayError::OffsetOutOfRange)?;

        let mut edge_index = first_child;
        group_start = cursor;
        while group_start < keys.len() {
            let label = keys[group_start].key_bytes()[depth];
            let mut group_end = group_start + 1;
            while group_end < keys.len()
                && keys[group_end].key_bytes().get(depth).copied() == Some(label)
            {
                group_end += 1;
            }
            let child = self.push_node(&keys[group_start..group_end], depth + 1)?;
            self.edges[edge_index] = CompactTrieEdge { child, label };
            edge_index += 1;
            group_start = group_end;
        }
        Ok(node_index_u32)
    }

    fn value(&self, node_index: usize) -> Option<u32> {
        let value = self.nodes[node_index].value;
        (value != u32::MAX).then_some(value)
    }

    fn children(&self, node_index: usize) -> &[CompactTrieEdge] {
        let node = self.nodes[node_index];
        let start = node.first_child as usize;
        let end = start + node.child_count as usize;
        &self.edges[start..end]
    }
}

struct DartsBuilder {
    trie: CompactTrie,
    units: Vec<u32>,
    used: Vec<bool>,
}

impl DartsBuilder {
    fn assign(
        &mut self,
        trie_index: usize,
        array_index: usize,
        label: u8,
    ) -> Result<(), DartsDoubleArrayError> {
        let targets = self.target_labels(trie_index);
        let offset = self.find_offset(array_index, &targets)?;
        self.reserve(array_index);
        self.units[array_index] =
            DartsDoubleArray::unit(offset, self.trie.value(trie_index).is_some(), label);

        if let Some(value) = self.trie.value(trie_index) {
            let leaf_index = array_index ^ usize::try_from(offset).unwrap();
            self.reserve(leaf_index);
            self.units[leaf_index] = value;
        }

        let children = self.trie.children(trie_index).to_vec();
        // Reserve every sibling slot before recursing. `find_offset` only checked
        // that these slots were free at this instant; without reserving them now, a
        // child's own subtree could be placed into a not-yet-assigned sibling's slot
        // and corrupt the trie (producing out-of-range `exact_match` values for some
        // keys). Reserving them up front keeps each sibling's slot exclusive.
        let offset_index = usize::try_from(offset).unwrap();
        for child in &children {
            self.reserve(array_index ^ offset_index ^ usize::from(child.label));
        }
        for child in children {
            let child_index = array_index ^ offset_index ^ usize::from(child.label);
            self.assign(child.child as usize, child_index, child.label)?;
        }
        Ok(())
    }

    fn target_labels(&self, trie_index: usize) -> Vec<Option<u8>> {
        let mut labels = Vec::new();
        if self.trie.value(trie_index).is_some() {
            labels.push(None);
        }
        labels.extend(
            self.trie
                .children(trie_index)
                .iter()
                .map(|child| Some(child.label)),
        );
        labels
    }

    fn find_offset(
        &self,
        array_index: usize,
        labels: &[Option<u8>],
    ) -> Result<u32, DartsDoubleArrayError> {
        let mut offset = 1usize;
        loop {
            if offset >= DartsDoubleArray::MAX_OFFSET as usize {
                return Err(DartsDoubleArrayError::OffsetOutOfRange);
            }
            if offset >= DartsDoubleArray::LARGE_OFFSET_THRESHOLD as usize && offset & 0xff != 0 {
                offset = offset
                    .checked_add(0xff)
                    .map(|offset| offset & !0xff)
                    .ok_or(DartsDoubleArrayError::OffsetOutOfRange)?;
                continue;
            }
            if labels.iter().all(|label| {
                let target = array_index ^ offset ^ label.map_or(0usize, usize::from);
                target != array_index && !self.used.get(target).copied().unwrap_or(false)
            }) {
                return u32::try_from(offset).map_err(|_| DartsDoubleArrayError::OffsetOutOfRange);
            }
            offset = offset
                .checked_add(1)
                .ok_or(DartsDoubleArrayError::OffsetOutOfRange)?;
        }
    }

    fn reserve(&mut self, index: usize) {
        if self.units.len() <= index {
            self.units.resize(index + 1, 0);
            self.used.resize(index + 1, false);
        }
        self.used[index] = true;
    }
}

#[cfg(test)]
mod large_offset_encoding_tests {
    use super::{DartsDoubleArray, DartsMatch, MAX_DARTS_PREDICTIVE_PATH_DEPTH};

    #[test]
    fn compact_trie_preserves_frozen_layout_across_input_order() {
        let unordered =
            DartsDoubleArray::build(&[("ba", 7), ("a", 0), ("ang", 4), ("bai", 9), ("an", 3)])
                .unwrap();
        let sorted =
            DartsDoubleArray::build(&[("a", 0), ("an", 3), ("ang", 4), ("ba", 7), ("bai", 9)])
                .unwrap();
        assert_eq!(unordered.units(), sorted.units());

        let mut expected = vec![0; 108];
        for (index, unit) in [
            (0, 1024),
            (2, 7),
            (3, 1377),
            (14, 3),
            (15, 1390),
            (96, 1377),
            (99, 1122),
            (104, 4),
            (105, 1383),
            (106, 9),
            (107, 1385),
        ] {
            expected[index] = unit;
        }
        assert_eq!(unordered.units(), expected);
    }

    #[test]
    fn darts_large_offsets_round_trip_through_the_standard_shift_flag() {
        for offset in [
            DartsDoubleArray::LARGE_OFFSET_THRESHOLD - 1,
            DartsDoubleArray::LARGE_OFFSET_THRESHOLD,
            DartsDoubleArray::LARGE_OFFSET_THRESHOLD + 0x100,
            DartsDoubleArray::MAX_OFFSET - 0x100,
        ] {
            let unit = DartsDoubleArray::unit(offset, true, b'z');
            assert_eq!(DartsDoubleArray::offset(unit), offset);
            assert!(DartsDoubleArray::has_leaf(unit));
            assert_eq!(DartsDoubleArray::label(unit), u32::from(b'z'));
            if offset >= DartsDoubleArray::LARGE_OFFSET_THRESHOLD {
                assert_ne!(unit & (1 << 9), 0);
            }
        }
    }

    #[test]
    fn predictive_search_rejects_a_malformed_owned_cycle() {
        let mut units = vec![0; 98];
        units[0] = DartsDoubleArray::unit(1, false, 0);
        // For the `a` transition, offset 96 alternates positions 96 and 97.
        // A valid DARTS trie never revisits a node; external bytes can.
        units[96] = DartsDoubleArray::unit(96, false, b'a');
        units[97] = DartsDoubleArray::unit(96, false, b'a');
        let malformed =
            DartsDoubleArray::from_units(units).expect("units are structurally present");

        assert!(malformed
            .predictive_search_with_alphabet(b"", b"a", 0)
            .is_empty());
    }

    #[test]
    fn predictive_search_rejects_a_leaf_bearing_owned_cycle_before_production_limit() {
        let mut units = vec![0; 98];
        units[0] = DartsDoubleArray::unit(1, false, 0);
        units[96] = DartsDoubleArray::unit(96, true, b'a');
        units[97] = DartsDoubleArray::unit(96, true, b'a');
        let malformed =
            DartsDoubleArray::from_units(units).expect("units are structurally present");

        assert!(malformed
            .predictive_search_with_alphabet(b"", b"a", 512)
            .is_empty());
    }

    #[test]
    fn predictive_search_preserves_path_dependent_repeated_owned_states() {
        let mut units = vec![0; 124];
        units[0] = DartsDoubleArray::unit(1, false, 0);
        units[96] = DartsDoubleArray::unit(126, false, b'a');
        units[99] = DartsDoubleArray::unit(125, false, b'b');
        // Both `az` and `bz` reach slot 100. Predictive DARTS traversal is
        // path-dependent, so a global visited set would incorrectly drop one.
        units[100] = DartsDoubleArray::unit(1, true, b'z');
        units[101] = 42;
        let repeated = DartsDoubleArray::from_units(units).expect("units are structurally present");

        assert_eq!(
            repeated.predictive_search_with_alphabet(b"", b"abz", 0),
            [
                DartsMatch {
                    value: 42,
                    length: 2,
                },
                DartsMatch {
                    value: 42,
                    length: 2,
                },
            ]
        );
    }

    #[test]
    fn predictive_search_rejects_owned_paths_over_the_depth_bound() {
        let key = "a".repeat(MAX_DARTS_PREDICTIVE_PATH_DEPTH + 1);
        let deep = DartsDoubleArray::build(&[(key, 7)]).expect("deep test key should build");

        assert!(deep
            .predictive_search_with_alphabet(b"", b"a", 512)
            .is_empty());
    }
}
