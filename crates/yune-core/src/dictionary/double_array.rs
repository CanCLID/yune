use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DartsDoubleArray {
    units: Vec<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DartsMatch {
    pub value: u32,
    pub length: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DartsDoubleArrayError {
    Empty,
    DuplicateKey,
    EmptyKey,
    ValueOutOfRange,
    OffsetOutOfRange,
}

#[derive(Default)]
struct TrieNode {
    value: Option<u32>,
    children: BTreeMap<u8, usize>,
}

impl DartsDoubleArray {
    const HAS_LEAF: u32 = 1 << 8;
    const VALUE_MASK: u32 = (1 << 31) - 1;
    const LABEL_MASK: u32 = (1 << 31) | 0xff;

    pub fn build<K>(keys: &[(K, u32)]) -> Result<Self, DartsDoubleArrayError>
    where
        K: AsRef<str>,
    {
        if keys.is_empty() {
            return Err(DartsDoubleArrayError::Empty);
        }
        let mut trie = vec![TrieNode::default()];
        for (key, value) in keys {
            if *value > Self::VALUE_MASK {
                return Err(DartsDoubleArrayError::ValueOutOfRange);
            }
            let key = key.as_ref();
            if key.is_empty() {
                return Err(DartsDoubleArrayError::EmptyKey);
            }
            let mut node = 0usize;
            for byte in key.bytes() {
                if let Some(next) = trie[node].children.get(&byte).copied() {
                    node = next;
                } else {
                    let next = trie.len();
                    trie.push(TrieNode::default());
                    trie[node].children.insert(byte, next);
                    node = next;
                }
            }
            if trie[node].value.replace(*value).is_some() {
                return Err(DartsDoubleArrayError::DuplicateKey);
            }
        }

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
    pub fn exact_match(&self, key: &str) -> Option<u32> {
        let mut node_pos = 0usize;
        let mut unit = *self.units.get(node_pos)?;
        for byte in key.bytes() {
            node_pos ^= usize::try_from(Self::offset(unit)).ok()? ^ usize::from(byte);
            unit = *self.units.get(node_pos)?;
            if Self::label(unit) != u32::from(byte) {
                return None;
            }
        }
        if !Self::has_leaf(unit) {
            return None;
        }
        let leaf_pos = node_pos ^ usize::try_from(Self::offset(unit)).ok()?;
        self.units.get(leaf_pos).map(|leaf| Self::value(*leaf))
    }

    #[must_use]
    pub fn common_prefix_search(&self, key: &str) -> Vec<DartsMatch> {
        let mut matches = Vec::new();
        let mut node_pos = 0usize;
        let Some(mut unit) = self.units.get(node_pos).copied() else {
            return matches;
        };
        let Ok(root_offset) = usize::try_from(Self::offset(unit)) else {
            return matches;
        };
        node_pos ^= root_offset;
        for (index, byte) in key.bytes().enumerate() {
            node_pos ^= usize::from(byte);
            let Some(next_unit) = self.units.get(node_pos).copied() else {
                return matches;
            };
            unit = next_unit;
            if Self::label(unit) != u32::from(byte) {
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
                }
            }
        }
        matches
    }

    const fn unit(offset: u32, has_leaf: bool, label: u8) -> u32 {
        (offset << 10) | if has_leaf { Self::HAS_LEAF } else { 0 } | label as u32
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

struct DartsBuilder {
    trie: Vec<TrieNode>,
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
            DartsDoubleArray::unit(offset, self.trie[trie_index].value.is_some(), label);

        if let Some(value) = self.trie[trie_index].value {
            let leaf_index = array_index ^ usize::try_from(offset).unwrap();
            self.reserve(leaf_index);
            self.units[leaf_index] = value;
        }

        let children = self.trie[trie_index]
            .children
            .iter()
            .map(|(byte, child)| (*byte, *child))
            .collect::<Vec<_>>();
        for (byte, child) in children {
            let child_index = array_index ^ usize::try_from(offset).unwrap() ^ usize::from(byte);
            self.assign(child, child_index, byte)?;
        }
        Ok(())
    }

    fn target_labels(&self, trie_index: usize) -> Vec<Option<u8>> {
        let mut labels = Vec::new();
        if self.trie[trie_index].value.is_some() {
            labels.push(None);
        }
        labels.extend(self.trie[trie_index].children.keys().copied().map(Some));
        labels
    }

    fn find_offset(
        &self,
        array_index: usize,
        labels: &[Option<u8>],
    ) -> Result<u32, DartsDoubleArrayError> {
        for offset in 1usize.. {
            if offset > (u32::MAX >> 10) as usize {
                return Err(DartsDoubleArrayError::OffsetOutOfRange);
            }
            if labels.iter().all(|label| {
                let target = array_index ^ offset ^ label.map_or(0usize, usize::from);
                target != array_index && !self.used.get(target).copied().unwrap_or(false)
            }) {
                return u32::try_from(offset).map_err(|_| DartsDoubleArrayError::OffsetOutOfRange);
            }
        }
        Err(DartsDoubleArrayError::OffsetOutOfRange)
    }

    fn reserve(&mut self, index: usize) {
        if self.units.len() <= index {
            self.units.resize(index + 1, 0);
            self.used.resize(index + 1, false);
        }
        self.used[index] = true;
    }
}
