use super::compiled_table::CanonicalCodeSequence;
use super::{CompactTableByteSource, RimeCorrectionEntry, RimeToleranceRule};
use crate::dictionary::compiled::{
    parse_rime_format_version_for_payload, read_f32_le, read_i32_le, read_u32_le,
};
use crate::dictionary::double_array::{
    push_darts_predictive_state, DartsDoubleArray, DartsMatch, DartsPredictivePush,
    DartsPredictiveState,
};
use crate::{MemoryOwnerClass, MemoryOwnerRow};
use std::mem;
use std::ops::ControlFlow;
use std::sync::Arc;

const MAX_CORRECTION_COUNT: usize = 4096;
const MAX_TOLERANCE_RULE_COUNT: usize = 4096;
const MAX_TOLERANCE_CANDIDATE_COUNT: usize = 64;
const RIME_PRISM_HEADER_LEN: usize = 320;

#[derive(Clone, Debug, PartialEq)]
pub struct RimePrismBinPayload {
    pub dict_file_checksum: u32,
    pub schema_file_checksum: u32,
    pub num_syllables: u32,
    pub num_spellings: u32,
    pub double_array_size: u32,
    pub double_array: Option<DartsDoubleArray>,
    pub spelling_map: Vec<Vec<RimePrismSpellingDescriptor>>,
    pub corrections: Vec<RimeCorrectionEntry>,
    pub tolerance_rules: Vec<RimeToleranceRule>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RimePrismSpellingDescriptor {
    pub syllable_id: i32,
    pub spelling_type: i32,
    pub is_correction: bool,
    pub credibility: f32,
    pub tips: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PrismLookupCode<'a> {
    pub code: &'a str,
    pub syllable_id: usize,
    pub spelling_type: i32,
    pub normal: bool,
    pub abbreviation: bool,
    pub correction: bool,
    pub credibility: f32,
}

impl RimePrismBinPayload {
    #[must_use]
    pub fn memory_owner_rows(&self) -> Vec<MemoryOwnerRow> {
        let descriptor_count = self.spelling_map.iter().map(Vec::len).sum::<usize>();
        let tip_count = self
            .spelling_map
            .iter()
            .flatten()
            .filter(|descriptor| !descriptor.tips.is_empty())
            .count();
        vec![
            MemoryOwnerRow::new(
                "prism.double_array_units",
                MemoryOwnerClass::HeapOwnedRequired,
                estimate_double_array_units_bytes(self.double_array.as_ref()),
                self.double_array
                    .as_ref()
                    .map_or(0, |double_array| double_array.units().len()),
                "DartsDoubleArray Vec<u32>",
                "parsed prism double-array units retained on heap for spelling lookup",
            ),
            MemoryOwnerRow::new(
                "prism.spelling_map",
                MemoryOwnerClass::HeapOwnedRequired,
                estimate_spelling_map_bytes(&self.spelling_map, self.spelling_map.capacity()),
                descriptor_count,
                "Vec<Vec<RimePrismSpellingDescriptor>>",
                "parsed prism spelling descriptor vectors retained on heap",
            ),
            MemoryOwnerRow::new(
                "prism.corrections_tolerance",
                MemoryOwnerClass::HeapOwnedRequired,
                estimate_correction_tolerance_bytes(
                    &self.corrections,
                    self.corrections.capacity(),
                    &self.tolerance_rules,
                    self.tolerance_rules.capacity(),
                ),
                self.corrections
                    .len()
                    .saturating_add(self.tolerance_rules.len()),
                "Vec<RimeCorrectionEntry> + Vec<RimeToleranceRule>",
                "parsed prism correction/tolerance payload retained on heap",
            ),
            MemoryOwnerRow::new(
                "prism.tips_payload",
                MemoryOwnerClass::HeapOwnedRequired,
                estimate_tips_payload_bytes(&self.spelling_map),
                tip_count,
                "String payloads in RimePrismSpellingDescriptor",
                "parsed prism descriptor tips string payload retained on heap when present",
            ),
        ]
    }

    #[must_use]
    pub fn lookup_canonical_codes<'a>(
        &self,
        spelling: &str,
        syllabary_codes: &'a (impl CanonicalCodeSequence + ?Sized),
    ) -> Vec<PrismLookupCode<'a>> {
        self.lookup_canonical_codes_with_limit(spelling, syllabary_codes, usize::MAX)
    }

    #[must_use]
    pub fn lookup_canonical_codes_with_limit<'a>(
        &self,
        spelling: &str,
        syllabary_codes: &'a (impl CanonicalCodeSequence + ?Sized),
        limit: usize,
    ) -> Vec<PrismLookupCode<'a>> {
        let Some(spelling_index) = self
            .double_array
            .as_ref()
            .and_then(|double_array| double_array.exact_match(spelling))
        else {
            return Vec::new();
        };
        self.lookup_canonical_codes_for_index(spelling_index as usize, syllabary_codes, limit)
    }

    fn visit_canonical_codes<'a, B, F>(
        &self,
        spelling: &str,
        syllabary_codes: &'a (impl CanonicalCodeSequence + ?Sized),
        visitor: &mut F,
    ) -> ControlFlow<B>
    where
        F: FnMut(PrismLookupCode<'a>) -> ControlFlow<B>,
    {
        let Some(spelling_index) = self
            .double_array
            .as_ref()
            .and_then(|double_array| double_array.exact_match(spelling))
        else {
            return ControlFlow::Continue(());
        };
        self.visit_canonical_codes_for_index(spelling_index as usize, syllabary_codes, visitor)
    }

    fn visit_canonical_codes_for_index<'a, B, F>(
        &self,
        spelling_index: usize,
        syllabary_codes: &'a (impl CanonicalCodeSequence + ?Sized),
        visitor: &mut F,
    ) -> ControlFlow<B>
    where
        F: FnMut(PrismLookupCode<'a>) -> ControlFlow<B>,
    {
        for descriptor in self.spelling_map.get(spelling_index).into_iter().flatten() {
            let Some(syllable_index) = usize::try_from(descriptor.syllable_id).ok() else {
                continue;
            };
            let Some(code) = syllabary_codes.get(syllable_index) else {
                continue;
            };
            match visitor(PrismLookupCode {
                code,
                syllable_id: syllable_index,
                spelling_type: descriptor.spelling_type,
                normal: descriptor.spelling_type == 0,
                abbreviation: descriptor.spelling_type == 2,
                correction: descriptor.is_correction,
                credibility: descriptor.credibility,
            }) {
                ControlFlow::Continue(()) => {}
                ControlFlow::Break(value) => return ControlFlow::Break(value),
            }
        }
        ControlFlow::Continue(())
    }

    fn lookup_canonical_codes_for_index<'a>(
        &self,
        spelling_index: usize,
        syllabary_codes: &'a (impl CanonicalCodeSequence + ?Sized),
        limit: usize,
    ) -> Vec<PrismLookupCode<'a>> {
        self.spelling_map
            .get(spelling_index)
            .into_iter()
            .flatten()
            .filter_map(|descriptor| {
                let syllable_index = usize::try_from(descriptor.syllable_id).ok()?;
                let code = syllabary_codes.get(syllable_index)?;
                Some(PrismLookupCode {
                    code,
                    syllable_id: syllable_index,
                    spelling_type: descriptor.spelling_type,
                    normal: descriptor.spelling_type == 0,
                    abbreviation: descriptor.spelling_type == 2,
                    correction: descriptor.is_correction,
                    credibility: descriptor.credibility,
                })
            })
            .take(limit)
            .collect()
    }

    fn common_prefix_canonical_codes<'a>(
        &self,
        spelling: &str,
        syllabary_codes: &'a (impl CanonicalCodeSequence + ?Sized),
        limit: usize,
    ) -> Vec<(usize, PrismLookupCode<'a>)> {
        let Some(double_array) = self.double_array.as_ref() else {
            return Vec::new();
        };
        let mut lookups = Vec::new();
        for matched in double_array.common_prefix_search(spelling) {
            if !spelling.is_char_boundary(matched.length) {
                continue;
            }
            let remaining = limit.saturating_sub(lookups.len());
            if remaining == 0 {
                break;
            }
            lookups.extend(
                self.lookup_canonical_codes_for_index(
                    matched.value as usize,
                    syllabary_codes,
                    remaining,
                )
                .into_iter()
                .map(|lookup| (matched.length, lookup)),
            );
        }
        lookups
    }

    fn trailing_ascii_digit_prefix_canonical_codes<'a>(
        &self,
        spelling: &str,
        syllabary_codes: &'a (impl CanonicalCodeSequence + ?Sized),
        limit: usize,
    ) -> Vec<(usize, PrismLookupCode<'a>)> {
        let Some(double_array) = self.double_array.as_ref() else {
            return Vec::new();
        };
        let mut lookups = Vec::new();
        let mut probe = Vec::with_capacity(spelling.len().saturating_add(1));
        for (index, byte) in spelling.bytes().enumerate() {
            if !byte.is_ascii_alphabetic() {
                break;
            }
            probe.push(byte);
            let consumed = index + 1;
            probe.push(b'0');
            for tone in b'0'..=b'9' {
                probe[consumed] = tone;
                let Some(spelling_index) = double_array.exact_match_bytes(&probe) else {
                    continue;
                };
                let remaining = limit.saturating_sub(lookups.len());
                if remaining == 0 {
                    return lookups;
                }
                lookups.extend(
                    self.lookup_canonical_codes_for_index(
                        spelling_index as usize,
                        syllabary_codes,
                        remaining,
                    )
                    .into_iter()
                    .map(|lookup| (consumed, lookup)),
                );
            }
            probe.pop();
        }
        lookups
    }
}

#[derive(Debug)]
pub struct RimePrismRuntimePayload {
    storage: RimePrismRuntimeStorage,
}

#[derive(Debug)]
enum RimePrismRuntimeStorage {
    Owned(RimePrismBinPayload),
    ByteBacked(ByteBackedRimePrismPayload),
}

#[derive(Debug)]
struct ByteBackedRimePrismPayload {
    source: Arc<dyn CompactTableByteSource>,
    double_array: Option<ByteBackedPrismDoubleArray>,
    spelling_map: ByteBackedPrismSpellingMap,
    corrections: Vec<RimeCorrectionEntry>,
    tolerance_rules: Vec<RimeToleranceRule>,
}

#[derive(Debug)]
struct ByteBackedRimePrismLayout {
    double_array: Option<ByteBackedPrismDoubleArray>,
    spelling_map: ByteBackedPrismSpellingMap,
    corrections: Vec<RimeCorrectionEntry>,
    tolerance_rules: Vec<RimeToleranceRule>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ByteBackedPrismDoubleArray {
    offset: usize,
    unit_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ByteBackedPrismSpellingMap {
    Explicit {
        offset: usize,
        spelling_count: usize,
        descriptor_count: usize,
        raw_descriptor_bytes: usize,
        tips_payload_bytes: usize,
        tip_count: usize,
    },
    Identity {
        spelling_count: usize,
        num_syllables: usize,
    },
}

#[derive(Clone, Copy, Debug)]
struct RuntimePrismSpellingDescriptor {
    syllable_id: i32,
    spelling_type: i32,
    is_correction: bool,
    credibility: f32,
}

impl From<RimePrismBinPayload> for RimePrismRuntimePayload {
    fn from(payload: RimePrismBinPayload) -> Self {
        Self {
            storage: RimePrismRuntimeStorage::Owned(payload),
        }
    }
}

impl RimePrismRuntimePayload {
    #[must_use]
    pub(crate) fn has_byte_backed_identity_spelling_map(&self) -> bool {
        matches!(
            &self.storage,
            RimePrismRuntimeStorage::ByteBacked(ByteBackedRimePrismPayload {
                spelling_map: ByteBackedPrismSpellingMap::Identity { .. },
                ..
            })
        )
    }

    #[must_use]
    pub fn corrections(&self) -> &[RimeCorrectionEntry] {
        match &self.storage {
            RimePrismRuntimeStorage::Owned(payload) => &payload.corrections,
            RimePrismRuntimeStorage::ByteBacked(payload) => &payload.corrections,
        }
    }

    #[must_use]
    pub fn tolerance_rules(&self) -> &[RimeToleranceRule] {
        match &self.storage {
            RimePrismRuntimeStorage::Owned(payload) => &payload.tolerance_rules,
            RimePrismRuntimeStorage::ByteBacked(payload) => &payload.tolerance_rules,
        }
    }

    #[must_use]
    pub fn memory_owner_rows(&self) -> Vec<MemoryOwnerRow> {
        match &self.storage {
            RimePrismRuntimeStorage::Owned(payload) => payload.memory_owner_rows(),
            RimePrismRuntimeStorage::ByteBacked(payload) => payload.memory_owner_rows(),
        }
    }

    #[must_use]
    pub fn lookup_canonical_codes<'a>(
        &self,
        spelling: &str,
        syllabary_codes: &'a (impl CanonicalCodeSequence + ?Sized),
    ) -> Vec<PrismLookupCode<'a>> {
        self.lookup_canonical_codes_with_limit(spelling, syllabary_codes, usize::MAX)
    }

    #[must_use]
    pub fn lookup_canonical_codes_with_limit<'a>(
        &self,
        spelling: &str,
        syllabary_codes: &'a (impl CanonicalCodeSequence + ?Sized),
        limit: usize,
    ) -> Vec<PrismLookupCode<'a>> {
        match &self.storage {
            RimePrismRuntimeStorage::Owned(payload) => {
                payload.lookup_canonical_codes_with_limit(spelling, syllabary_codes, limit)
            }
            RimePrismRuntimeStorage::ByteBacked(payload) => {
                payload.lookup_canonical_codes_with_limit(spelling, syllabary_codes, limit)
            }
        }
    }

    /// Expands every prism key beginning with `prefix` in librime's breadth-
    /// first key order, then resolves each selected key to canonical syllable
    /// descriptors in source order. `limit` applies to prism keys before an
    /// invalid or caller-filtered descriptor can disappear; zero means
    /// unlimited, matching `Prism::ExpandSearch`.
    #[must_use]
    pub fn predictive_canonical_codes_with_limit<'a>(
        &self,
        prefix: &str,
        syllabary_codes: &'a (impl CanonicalCodeSequence + ?Sized),
        limit: usize,
    ) -> Vec<PrismLookupCode<'a>> {
        let alphabet = predictive_alphabet(syllabary_codes);
        let matches = match &self.storage {
            RimePrismRuntimeStorage::Owned(payload) => payload
                .double_array
                .as_ref()
                .map(|double_array| {
                    double_array.predictive_search_with_alphabet(
                        prefix.as_bytes(),
                        &alphabet,
                        limit,
                    )
                })
                .unwrap_or_default(),
            RimePrismRuntimeStorage::ByteBacked(payload) => payload
                .double_array
                .map(|double_array| {
                    double_array.predictive_search_with_alphabet(
                        payload.source.bytes(),
                        prefix.as_bytes(),
                        &alphabet,
                        limit,
                    )
                })
                .unwrap_or_default(),
        };

        let mut lookups = Vec::new();
        for matched in matches {
            let Ok(spelling_index) = usize::try_from(matched.value) else {
                continue;
            };
            match &self.storage {
                RimePrismRuntimeStorage::Owned(payload) => {
                    lookups.extend(payload.lookup_canonical_codes_for_index(
                        spelling_index,
                        syllabary_codes,
                        usize::MAX,
                    ))
                }
                RimePrismRuntimeStorage::ByteBacked(payload) => {
                    lookups.extend(payload.lookup_canonical_codes_for_index(
                        spelling_index,
                        syllabary_codes,
                        usize::MAX,
                    ))
                }
            }
        }
        lookups
    }

    /// Visits canonical codes for an exact deployed spelling in descriptor
    /// source order. Unlike the vector-returning lookup helpers, this path does
    /// not materialize every descriptor and permits the caller to stop early.
    pub(crate) fn visit_canonical_codes<'a, B>(
        &self,
        spelling: &str,
        syllabary_codes: &'a (impl CanonicalCodeSequence + ?Sized),
        mut visitor: impl FnMut(PrismLookupCode<'a>) -> ControlFlow<B>,
    ) -> ControlFlow<B> {
        match &self.storage {
            RimePrismRuntimeStorage::Owned(payload) => {
                payload.visit_canonical_codes(spelling, syllabary_codes, &mut visitor)
            }
            RimePrismRuntimeStorage::ByteBacked(payload) => {
                payload.visit_canonical_codes(spelling, syllabary_codes, &mut visitor)
            }
        }
    }

    /// Returns deployed spelling matches for every prism key that is a prefix
    /// of `spelling`. The Darts traversal is linear in the input length and
    /// avoids probing every UTF-8 boundary or materializing a global surface
    /// index for large identity prisms such as the tracked Stroke product.
    pub(crate) fn common_prefix_canonical_codes<'a>(
        &self,
        spelling: &str,
        syllabary_codes: &'a (impl CanonicalCodeSequence + ?Sized),
        limit: usize,
    ) -> Vec<(usize, PrismLookupCode<'a>)> {
        match &self.storage {
            RimePrismRuntimeStorage::Owned(payload) => {
                payload.common_prefix_canonical_codes(spelling, syllabary_codes, limit)
            }
            RimePrismRuntimeStorage::ByteBacked(payload) => {
                payload.common_prefix_canonical_codes(spelling, syllabary_codes, limit)
            }
        }
    }

    /// Returns canonical codes whose deployed identity spelling is the consumed
    /// input prefix plus one trailing ASCII tone digit. This preserves the
    /// historical no-algebra `bei` -> `bei2` surface without building a global
    /// normalized index for large identity prisms.
    pub(crate) fn trailing_ascii_digit_prefix_canonical_codes<'a>(
        &self,
        spelling: &str,
        syllabary_codes: &'a (impl CanonicalCodeSequence + ?Sized),
        limit: usize,
    ) -> Vec<(usize, PrismLookupCode<'a>)> {
        match &self.storage {
            RimePrismRuntimeStorage::Owned(payload) => payload
                .trailing_ascii_digit_prefix_canonical_codes(spelling, syllabary_codes, limit),
            RimePrismRuntimeStorage::ByteBacked(payload) => payload
                .trailing_ascii_digit_prefix_canonical_codes(spelling, syllabary_codes, limit),
        }
    }
}

fn predictive_alphabet(syllabary_codes: &(impl CanonicalCodeSequence + ?Sized)) -> Vec<u8> {
    let mut alphabet = (0..syllabary_codes.len())
        .filter_map(|index| syllabary_codes.get(index))
        .flat_map(str::bytes)
        .filter(|byte| *byte != 0)
        .collect::<Vec<_>>();
    alphabet.sort_unstable();
    alphabet.dedup();
    alphabet
}

impl ByteBackedRimePrismPayload {
    fn memory_owner_rows(&self) -> Vec<MemoryOwnerRow> {
        let source_class = prism_byte_source_class(self.source.as_ref());
        let source_label = format!(
            "{}:{}",
            self.source.storage_label(),
            self.source.mapping_mode()
        );
        vec![
            MemoryOwnerRow::new(
                "prism.double_array_units",
                source_class,
                self.double_array
                    .map_or(0, ByteBackedPrismDoubleArray::byte_len),
                self.double_array
                    .map_or(0, |double_array| double_array.unit_count),
                source_label.clone(),
                "prism double-array units are read directly from the byte source",
            ),
            MemoryOwnerRow::new(
                "prism.spelling_map",
                source_class,
                self.spelling_map.byte_len(),
                self.spelling_map.descriptor_count(),
                source_label.clone(),
                "prism spelling descriptors are read lazily from the byte source",
            ),
            MemoryOwnerRow::new(
                "prism.corrections_tolerance",
                MemoryOwnerClass::HeapOwnedRequired,
                estimate_correction_tolerance_bytes(
                    &self.corrections,
                    self.corrections.capacity(),
                    &self.tolerance_rules,
                    self.tolerance_rules.capacity(),
                ),
                self.corrections
                    .len()
                    .saturating_add(self.tolerance_rules.len()),
                "Vec<RimeCorrectionEntry> + Vec<RimeToleranceRule>",
                "parsed prism correction/tolerance payload retained on heap",
            ),
            MemoryOwnerRow::new(
                "prism.tips_payload",
                source_class,
                self.spelling_map.tips_payload_bytes(),
                self.spelling_map.tip_count(),
                source_label,
                "prism descriptor tips remain in the byte source",
            ),
        ]
    }

    fn lookup_canonical_codes_with_limit<'a>(
        &self,
        spelling: &str,
        syllabary_codes: &'a (impl CanonicalCodeSequence + ?Sized),
        limit: usize,
    ) -> Vec<PrismLookupCode<'a>> {
        let Some(spelling_index) = self
            .double_array
            .as_ref()
            .and_then(|double_array| double_array.exact_match(self.source.bytes(), spelling))
        else {
            return Vec::new();
        };
        let Ok(spelling_index) = usize::try_from(spelling_index) else {
            return Vec::new();
        };
        self.lookup_canonical_codes_for_index(spelling_index, syllabary_codes, limit)
    }

    fn visit_canonical_codes<'a, B, F>(
        &self,
        spelling: &str,
        syllabary_codes: &'a (impl CanonicalCodeSequence + ?Sized),
        visitor: &mut F,
    ) -> ControlFlow<B>
    where
        F: FnMut(PrismLookupCode<'a>) -> ControlFlow<B>,
    {
        let Some(spelling_index) = self
            .double_array
            .as_ref()
            .and_then(|double_array| double_array.exact_match(self.source.bytes(), spelling))
        else {
            return ControlFlow::Continue(());
        };
        let Ok(spelling_index) = usize::try_from(spelling_index) else {
            return ControlFlow::Continue(());
        };
        self.visit_canonical_codes_for_index(spelling_index, syllabary_codes, visitor)
    }

    fn visit_canonical_codes_for_index<'a, B, F>(
        &self,
        spelling_index: usize,
        syllabary_codes: &'a (impl CanonicalCodeSequence + ?Sized),
        visitor: &mut F,
    ) -> ControlFlow<B>
    where
        F: FnMut(PrismLookupCode<'a>) -> ControlFlow<B>,
    {
        if let ByteBackedPrismSpellingMap::Identity {
            spelling_count,
            num_syllables,
        } = self.spelling_map
        {
            if spelling_index >= spelling_count
                || spelling_index >= num_syllables
                || spelling_index >= syllabary_codes.len()
            {
                return ControlFlow::Continue(());
            }
            return visitor(PrismLookupCode {
                code: syllabary_codes
                    .get(spelling_index)
                    .expect("identity spelling index was bounds-checked"),
                syllable_id: spelling_index,
                spelling_type: 0,
                normal: true,
                abbreviation: false,
                correction: false,
                credibility: 0.0,
            });
        }
        let Some((descriptor_offset, descriptor_count)) = self
            .spelling_map
            .descriptor_header(self.source.bytes(), spelling_index)
        else {
            return ControlFlow::Continue(());
        };
        for index in 0..descriptor_count {
            let Some(descriptor) =
                read_runtime_spelling_descriptor(self.source.bytes(), descriptor_offset, index)
            else {
                return ControlFlow::Continue(());
            };
            let Some(syllable_index) = usize::try_from(descriptor.syllable_id).ok() else {
                continue;
            };
            let Some(code) = syllabary_codes.get(syllable_index) else {
                continue;
            };
            match visitor(PrismLookupCode {
                code,
                syllable_id: syllable_index,
                spelling_type: descriptor.spelling_type,
                normal: descriptor.spelling_type == 0,
                abbreviation: descriptor.spelling_type == 2,
                correction: descriptor.is_correction,
                credibility: descriptor.credibility,
            }) {
                ControlFlow::Continue(()) => {}
                ControlFlow::Break(value) => return ControlFlow::Break(value),
            }
        }
        ControlFlow::Continue(())
    }

    fn lookup_canonical_codes_for_index<'a>(
        &self,
        spelling_index: usize,
        syllabary_codes: &'a (impl CanonicalCodeSequence + ?Sized),
        limit: usize,
    ) -> Vec<PrismLookupCode<'a>> {
        if let ByteBackedPrismSpellingMap::Identity {
            spelling_count,
            num_syllables,
        } = self.spelling_map
        {
            if limit == 0
                || spelling_index >= spelling_count
                || spelling_index >= num_syllables
                || spelling_index >= syllabary_codes.len()
            {
                return Vec::new();
            }
            return vec![PrismLookupCode {
                code: syllabary_codes
                    .get(spelling_index)
                    .expect("identity spelling index was bounds-checked"),
                syllable_id: spelling_index,
                spelling_type: 0,
                normal: true,
                abbreviation: false,
                correction: false,
                credibility: 0.0,
            }];
        }
        let Some((descriptor_offset, descriptor_count)) = self
            .spelling_map
            .descriptor_header(self.source.bytes(), spelling_index)
        else {
            return Vec::new();
        };
        let mut lookups = Vec::new();
        for index in 0..descriptor_count {
            let Some(descriptor) =
                read_runtime_spelling_descriptor(self.source.bytes(), descriptor_offset, index)
            else {
                return Vec::new();
            };
            let Some(syllable_index) = usize::try_from(descriptor.syllable_id).ok() else {
                continue;
            };
            let Some(code) = syllabary_codes.get(syllable_index) else {
                continue;
            };
            lookups.push(PrismLookupCode {
                code,
                syllable_id: syllable_index,
                spelling_type: descriptor.spelling_type,
                normal: descriptor.spelling_type == 0,
                abbreviation: descriptor.spelling_type == 2,
                correction: descriptor.is_correction,
                credibility: descriptor.credibility,
            });
            if lookups.len() == limit {
                break;
            }
        }
        lookups
    }

    fn common_prefix_canonical_codes<'a>(
        &self,
        spelling: &str,
        syllabary_codes: &'a (impl CanonicalCodeSequence + ?Sized),
        limit: usize,
    ) -> Vec<(usize, PrismLookupCode<'a>)> {
        let Some(double_array) = self.double_array else {
            return Vec::new();
        };
        let mut lookups = Vec::new();
        for matched in double_array.common_prefix_search(self.source.bytes(), spelling) {
            if !spelling.is_char_boundary(matched.length) {
                continue;
            }
            let remaining = limit.saturating_sub(lookups.len());
            if remaining == 0 {
                break;
            }
            let Ok(spelling_index) = usize::try_from(matched.value) else {
                continue;
            };
            lookups.extend(
                self.lookup_canonical_codes_for_index(spelling_index, syllabary_codes, remaining)
                    .into_iter()
                    .map(|lookup| (matched.length, lookup)),
            );
        }
        lookups
    }

    fn trailing_ascii_digit_prefix_canonical_codes<'a>(
        &self,
        spelling: &str,
        syllabary_codes: &'a (impl CanonicalCodeSequence + ?Sized),
        limit: usize,
    ) -> Vec<(usize, PrismLookupCode<'a>)> {
        let Some(double_array) = self.double_array else {
            return Vec::new();
        };
        let mut lookups = Vec::new();
        let mut probe = Vec::with_capacity(spelling.len().saturating_add(1));
        for (index, byte) in spelling.bytes().enumerate() {
            if !byte.is_ascii_alphabetic() {
                break;
            }
            probe.push(byte);
            let consumed = index + 1;
            probe.push(b'0');
            for tone in b'0'..=b'9' {
                probe[consumed] = tone;
                let Some(spelling_index) =
                    double_array.exact_match_bytes(self.source.bytes(), &probe)
                else {
                    continue;
                };
                let remaining = limit.saturating_sub(lookups.len());
                if remaining == 0 {
                    return lookups;
                }
                let Ok(spelling_index) = usize::try_from(spelling_index) else {
                    continue;
                };
                lookups.extend(
                    self.lookup_canonical_codes_for_index(
                        spelling_index,
                        syllabary_codes,
                        remaining,
                    )
                    .into_iter()
                    .map(|lookup| (consumed, lookup)),
                );
            }
            probe.pop();
        }
        lookups
    }
}

impl ByteBackedPrismDoubleArray {
    const HAS_LEAF: u32 = 1 << 8;
    const VALUE_MASK: u32 = (1 << 31) - 1;
    const LABEL_MASK: u32 = (1 << 31) | 0xff;

    const fn byte_len(self) -> usize {
        self.unit_count.saturating_mul(mem::size_of::<u32>())
    }

    fn unit(self, bytes: &[u8], index: usize) -> Option<u32> {
        if index >= self.unit_count {
            return None;
        }
        let offset = self.offset.checked_add(index.checked_mul(4)?)?;
        let chunk = bytes.get(offset..offset.checked_add(4)?)?;
        Some(u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
    }

    fn exact_match(self, bytes: &[u8], key: &str) -> Option<u32> {
        self.exact_match_bytes(bytes, key.as_bytes())
    }

    fn exact_match_bytes(self, bytes: &[u8], key: &[u8]) -> Option<u32> {
        let mut node_pos = 0usize;
        let mut unit = self.unit(bytes, node_pos)?;
        for byte in key {
            node_pos ^= usize::try_from(Self::offset(unit)).ok()? ^ usize::from(*byte);
            unit = self.unit(bytes, node_pos)?;
            if Self::label(unit) != u32::from(*byte) {
                return None;
            }
        }
        if !Self::has_leaf(unit) {
            return None;
        }
        let leaf_pos = node_pos ^ usize::try_from(Self::offset(unit)).ok()?;
        self.unit(bytes, leaf_pos).map(Self::value)
    }

    fn predictive_search_with_alphabet(
        self,
        bytes: &[u8],
        prefix: &[u8],
        alphabet: &[u8],
        limit: usize,
    ) -> Vec<DartsMatch> {
        let effective_limit = if limit == 0 { usize::MAX } else { limit };
        let mut node_pos = 0usize;
        let Some(mut unit) = self.unit(bytes, node_pos) else {
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
            let Some(next_unit) = self.unit(bytes, node_pos) else {
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
            if let Some(leaf) = self.unit(bytes, node_pos ^ offset) {
                matches.push(DartsMatch {
                    value: Self::value(leaf),
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
                let Some(child_unit) = self.unit(bytes, child_pos) else {
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
                if let Some(leaf) = self.unit(bytes, child_pos ^ child_offset) {
                    matches.push(DartsMatch {
                        value: Self::value(leaf),
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

    fn common_prefix_search(self, bytes: &[u8], key: &str) -> Vec<DartsMatch> {
        let mut matches = Vec::new();
        let mut node_pos = 0usize;
        let Some(mut unit) = self.unit(bytes, node_pos) else {
            return matches;
        };
        let Ok(offset) = usize::try_from(Self::offset(unit)) else {
            return matches;
        };
        node_pos ^= offset;

        for (index, byte) in key.bytes().enumerate() {
            node_pos ^= usize::from(byte);
            let Some(next_unit) = self.unit(bytes, node_pos) else {
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
                if let Some(leaf) = self.unit(bytes, node_pos) {
                    matches.push(DartsMatch {
                        value: Self::value(leaf),
                        length: index + 1,
                    });
                }
            }
        }
        matches
    }

    const fn has_leaf(unit: u32) -> bool {
        (unit & Self::HAS_LEAF) != 0
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

impl ByteBackedPrismSpellingMap {
    const ITEM_SIZE: usize = 8;
    const DESCRIPTOR_SIZE: usize = 16;

    const fn byte_len(self) -> usize {
        match self {
            Self::Explicit {
                spelling_count,
                raw_descriptor_bytes,
                tips_payload_bytes,
                ..
            } => 4usize
                .saturating_add(spelling_count.saturating_mul(Self::ITEM_SIZE))
                .saturating_add(raw_descriptor_bytes)
                .saturating_add(tips_payload_bytes),
            Self::Identity { .. } => 0,
        }
    }

    fn descriptor_header(self, bytes: &[u8], index: usize) -> Option<(usize, usize)> {
        let Self::Explicit {
            offset,
            spelling_count,
            ..
        } = self
        else {
            return None;
        };
        if index >= spelling_count {
            return None;
        }
        let start = offset.checked_add(4)?;
        let item_offset = start.checked_add(index.checked_mul(Self::ITEM_SIZE)?)?;
        let descriptor_count = read_count(bytes, item_offset).ok()?;
        let descriptor_offset = read_offset_ptr(bytes, item_offset.checked_add(4)?).ok()??;
        Some((descriptor_offset, descriptor_count))
    }

    const fn descriptor_count(self) -> usize {
        match self {
            Self::Explicit {
                descriptor_count, ..
            } => descriptor_count,
            Self::Identity { .. } => 0,
        }
    }

    const fn tips_payload_bytes(self) -> usize {
        match self {
            Self::Explicit {
                tips_payload_bytes, ..
            } => tips_payload_bytes,
            Self::Identity { .. } => 0,
        }
    }

    const fn tip_count(self) -> usize {
        match self {
            Self::Explicit { tip_count, .. } => tip_count,
            Self::Identity { .. } => 0,
        }
    }
}

fn prism_byte_source_class(source: &dyn CompactTableByteSource) -> MemoryOwnerClass {
    if source.mapping_mode() == "mmap" {
        MemoryOwnerClass::MmapFileBacked
    } else {
        MemoryOwnerClass::HeapOwnedGuarded
    }
}

fn estimate_double_array_units_bytes(double_array: Option<&DartsDoubleArray>) -> usize {
    double_array.map_or(0, |double_array| {
        mem::size_of::<DartsDoubleArray>().saturating_add(
            double_array
                .units_capacity()
                .saturating_mul(mem::size_of::<u32>()),
        )
    })
}

fn estimate_spelling_map_bytes(
    map: &[Vec<RimePrismSpellingDescriptor>],
    outer_capacity: usize,
) -> usize {
    mem::size_of::<Vec<Vec<RimePrismSpellingDescriptor>>>()
        .saturating_add(
            outer_capacity.saturating_mul(mem::size_of::<Vec<RimePrismSpellingDescriptor>>()),
        )
        .saturating_add(
            map.iter()
                .map(|descriptors| {
                    descriptors
                        .capacity()
                        .saturating_mul(mem::size_of::<RimePrismSpellingDescriptor>())
                })
                .sum::<usize>(),
        )
}

fn estimate_correction_tolerance_bytes(
    corrections: &[RimeCorrectionEntry],
    correction_capacity: usize,
    tolerance_rules: &[RimeToleranceRule],
    tolerance_rule_capacity: usize,
) -> usize {
    mem::size_of::<Vec<RimeCorrectionEntry>>()
        .saturating_add(correction_capacity.saturating_mul(mem::size_of::<RimeCorrectionEntry>()))
        .saturating_add(
            corrections
                .iter()
                .map(|entry| {
                    entry
                        .observed_input
                        .capacity()
                        .saturating_add(entry.canonical_code.capacity())
                })
                .sum::<usize>(),
        )
        .saturating_add(mem::size_of::<Vec<RimeToleranceRule>>())
        .saturating_add(tolerance_rule_capacity.saturating_mul(mem::size_of::<RimeToleranceRule>()))
        .saturating_add(
            tolerance_rules
                .iter()
                .map(|rule| {
                    rule.near_code
                        .capacity()
                        .saturating_add(
                            rule.candidate_codes
                                .capacity()
                                .saturating_mul(mem::size_of::<String>()),
                        )
                        .saturating_add(
                            rule.candidate_codes
                                .iter()
                                .map(String::capacity)
                                .sum::<usize>(),
                        )
                })
                .sum::<usize>(),
        )
}

fn estimate_tips_payload_bytes(map: &[Vec<RimePrismSpellingDescriptor>]) -> usize {
    map.iter()
        .flatten()
        .map(|descriptor| descriptor.tips.capacity())
        .sum()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RimePrismBinParseError {
    TooShort,
    InvalidFormat,
    UnsupportedVersion,
    MissingRequiredSection,
    OutOfBounds,
    InvalidLength,
    InvalidCount,
    InvalidUtf8,
    UnsupportedSection { role: String },
}

pub fn parse_rime_prism_bin_payload(
    bytes: impl AsRef<[u8]>,
) -> Result<RimePrismBinPayload, RimePrismBinParseError> {
    let bytes = bytes.as_ref();
    ensure_len(bytes, 320)?;
    let version = parse_rime_format_version_for_payload(bytes, b"Rime::Prism/")
        .map_err(map_metadata_error)?;
    if version < 4.0 - f64::EPSILON {
        return Err(RimePrismBinParseError::UnsupportedVersion);
    }
    let double_array_offset = read_offset_ptr(bytes, 52)?;
    let spelling_map_offset = read_offset_ptr(bytes, 56)?;
    let correction_offset = read_yune_payload_offset(bytes, 60, b"YUNE-CORR\0")?;
    let tolerance_offset = read_yune_payload_offset(bytes, 64, b"YUNE-TOL\0")?;
    let double_array_size = read_u32_le(bytes, 48).map_err(map_metadata_error)?;
    let double_array = read_double_array(bytes, double_array_offset, double_array_size)?;

    let num_syllables = read_u32_le(bytes, 40).map_err(map_metadata_error)?;
    let num_spellings = read_u32_le(bytes, 44).map_err(map_metadata_error)?;
    let spelling_map = match spelling_map_offset {
        Some(offset) => read_spelling_map(bytes, offset)?,
        None => identity_spelling_map(num_syllables, num_spellings, double_array_size)?,
    };

    Ok(RimePrismBinPayload {
        dict_file_checksum: read_u32_le(bytes, 32).map_err(map_metadata_error)?,
        schema_file_checksum: read_u32_le(bytes, 36).map_err(map_metadata_error)?,
        num_syllables,
        num_spellings,
        double_array_size,
        double_array,
        spelling_map,
        corrections: correction_offset
            .map(|offset| read_corrections(bytes, offset))
            .transpose()?
            .unwrap_or_default(),
        tolerance_rules: tolerance_offset
            .map(|offset| read_tolerance_rules(bytes, offset))
            .transpose()?
            .unwrap_or_default(),
    })
}

pub fn parse_rime_prism_runtime_payload(
    source: Arc<dyn CompactTableByteSource>,
) -> Result<RimePrismRuntimePayload, RimePrismBinParseError> {
    let layout = read_byte_backed_prism_layout(source.bytes())?;
    Ok(byte_backed_runtime_payload(source, layout))
}

/// Parses and validates a prism through a temporary byte source while retaining
/// a distinct source for runtime lookups.
///
/// Both sources must describe the same complete file length and 320-byte Rime
/// header. The returned payload stores file-relative layout only and retains
/// `runtime_source`; `validation_source` can therefore be dropped immediately
/// after this call. The ordinary [`parse_rime_prism_runtime_payload`] entry
/// point remains the single-source path for owned bytes and WASM.
#[doc(hidden)]
pub fn parse_rime_prism_runtime_payload_with_validation_source(
    runtime_source: Arc<dyn CompactTableByteSource>,
    validation_source: &dyn CompactTableByteSource,
) -> Result<RimePrismRuntimePayload, RimePrismBinParseError> {
    let runtime_bytes = runtime_source.bytes();
    let validation_bytes = validation_source.bytes();
    ensure_len(runtime_bytes, RIME_PRISM_HEADER_LEN)?;
    ensure_len(validation_bytes, RIME_PRISM_HEADER_LEN)?;
    if runtime_bytes.len() != validation_bytes.len() {
        return Err(RimePrismBinParseError::InvalidLength);
    }
    if runtime_bytes[..RIME_PRISM_HEADER_LEN] != validation_bytes[..RIME_PRISM_HEADER_LEN] {
        return Err(RimePrismBinParseError::InvalidFormat);
    }

    let layout = read_byte_backed_prism_layout(validation_bytes)?;
    Ok(byte_backed_runtime_payload(runtime_source, layout))
}

fn read_byte_backed_prism_layout(
    bytes: &[u8],
) -> Result<ByteBackedRimePrismLayout, RimePrismBinParseError> {
    ensure_len(bytes, RIME_PRISM_HEADER_LEN)?;
    let version = parse_rime_format_version_for_payload(bytes, b"Rime::Prism/")
        .map_err(map_metadata_error)?;
    if version < 4.0 - f64::EPSILON {
        return Err(RimePrismBinParseError::UnsupportedVersion);
    }
    let double_array_offset = read_offset_ptr(bytes, 52)?;
    let spelling_map_offset = read_offset_ptr(bytes, 56)?;
    let correction_offset = read_yune_payload_offset(bytes, 60, b"YUNE-CORR\0")?;
    let tolerance_offset = read_yune_payload_offset(bytes, 64, b"YUNE-TOL\0")?;
    let double_array_size = read_u32_le(bytes, 48).map_err(map_metadata_error)?;
    let double_array =
        read_byte_backed_double_array(bytes, double_array_offset, double_array_size)?;
    let num_syllables = read_u32_le(bytes, 40).map_err(map_metadata_error)?;
    let num_spellings = read_u32_le(bytes, 44).map_err(map_metadata_error)?;
    let spelling_map = match spelling_map_offset {
        Some(offset) => read_byte_backed_spelling_map(bytes, offset)?,
        None => byte_backed_identity_spelling_map(num_syllables, num_spellings, double_array_size)?,
    };
    let corrections = correction_offset
        .map(|offset| read_corrections(bytes, offset))
        .transpose()?
        .unwrap_or_default();
    let tolerance_rules = tolerance_offset
        .map(|offset| read_tolerance_rules(bytes, offset))
        .transpose()?
        .unwrap_or_default();

    Ok(ByteBackedRimePrismLayout {
        double_array,
        spelling_map,
        corrections,
        tolerance_rules,
    })
}

fn byte_backed_runtime_payload(
    source: Arc<dyn CompactTableByteSource>,
    layout: ByteBackedRimePrismLayout,
) -> RimePrismRuntimePayload {
    RimePrismRuntimePayload {
        storage: RimePrismRuntimeStorage::ByteBacked(ByteBackedRimePrismPayload {
            source,
            double_array: layout.double_array,
            spelling_map: layout.spelling_map,
            corrections: layout.corrections,
            tolerance_rules: layout.tolerance_rules,
        }),
    }
}

fn read_double_array(
    bytes: &[u8],
    offset: Option<usize>,
    size: u32,
) -> Result<Option<DartsDoubleArray>, RimePrismBinParseError> {
    let Some(offset) = offset else {
        if size == 0 {
            return Ok(None);
        }
        return Err(RimePrismBinParseError::MissingRequiredSection);
    };
    if size == 0 {
        return Err(RimePrismBinParseError::InvalidCount);
    }
    let size = usize::try_from(size).map_err(|_| RimePrismBinParseError::InvalidCount)?;
    let byte_len = size
        .checked_mul(4)
        .ok_or(RimePrismBinParseError::InvalidCount)?;
    let end = offset
        .checked_add(byte_len)
        .ok_or(RimePrismBinParseError::OutOfBounds)?;
    if end > bytes.len() {
        return Err(RimePrismBinParseError::OutOfBounds);
    }
    let units = bytes[offset..end]
        .chunks_exact(4)
        .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect::<Vec<_>>();
    DartsDoubleArray::from_units(units)
        .map(Some)
        .map_err(|_| RimePrismBinParseError::InvalidCount)
}

fn read_byte_backed_double_array(
    bytes: &[u8],
    offset: Option<usize>,
    size: u32,
) -> Result<Option<ByteBackedPrismDoubleArray>, RimePrismBinParseError> {
    let Some(offset) = offset else {
        if size == 0 {
            return Ok(None);
        }
        return Err(RimePrismBinParseError::MissingRequiredSection);
    };
    if size == 0 {
        return Err(RimePrismBinParseError::InvalidCount);
    }
    let unit_count = usize::try_from(size).map_err(|_| RimePrismBinParseError::InvalidCount)?;
    let byte_len = unit_count
        .checked_mul(4)
        .ok_or(RimePrismBinParseError::InvalidCount)?;
    let end = offset
        .checked_add(byte_len)
        .ok_or(RimePrismBinParseError::OutOfBounds)?;
    if end > bytes.len() {
        return Err(RimePrismBinParseError::OutOfBounds);
    }
    Ok(Some(ByteBackedPrismDoubleArray { offset, unit_count }))
}

fn read_corrections(
    bytes: &[u8],
    offset: usize,
) -> Result<Vec<RimeCorrectionEntry>, RimePrismBinParseError> {
    let payload = bytes
        .get(offset..)
        .ok_or(RimePrismBinParseError::OutOfBounds)?;
    if !payload.starts_with(b"YUNE-CORR\0") {
        return Err(RimePrismBinParseError::UnsupportedSection {
            role: "correction payload".to_owned(),
        });
    }
    let mut cursor = offset
        .checked_add(b"YUNE-CORR\0".len())
        .ok_or(RimePrismBinParseError::OutOfBounds)?;
    let count = read_count(bytes, cursor)?;
    if count > MAX_CORRECTION_COUNT {
        return Err(RimePrismBinParseError::InvalidCount);
    }
    cursor = cursor
        .checked_add(4)
        .ok_or(RimePrismBinParseError::OutOfBounds)?;
    let mut corrections = Vec::with_capacity(count);
    for _ in 0..count {
        let (observed_input, next) = read_len_string(bytes, cursor)?;
        cursor = next;
        let (canonical_code, next) = read_len_string(bytes, cursor)?;
        cursor = next;
        corrections.push(RimeCorrectionEntry::new(observed_input, canonical_code));
    }
    Ok(corrections)
}

fn read_tolerance_rules(
    bytes: &[u8],
    offset: usize,
) -> Result<Vec<RimeToleranceRule>, RimePrismBinParseError> {
    let payload = bytes
        .get(offset..)
        .ok_or(RimePrismBinParseError::OutOfBounds)?;
    if !payload.starts_with(b"YUNE-TOL\0") {
        return Err(RimePrismBinParseError::UnsupportedSection {
            role: "tolerance payload".to_owned(),
        });
    }
    let mut cursor = offset
        .checked_add(b"YUNE-TOL\0".len())
        .ok_or(RimePrismBinParseError::OutOfBounds)?;
    let count = read_count(bytes, cursor)?;
    if count > MAX_TOLERANCE_RULE_COUNT {
        return Err(RimePrismBinParseError::InvalidCount);
    }
    cursor = cursor
        .checked_add(4)
        .ok_or(RimePrismBinParseError::OutOfBounds)?;
    let mut rules = Vec::with_capacity(count);
    for _ in 0..count {
        let (near_code, next) = read_len_string(bytes, cursor)?;
        cursor = next;
        let candidate_count = read_count(bytes, cursor)?;
        if candidate_count > MAX_TOLERANCE_CANDIDATE_COUNT {
            return Err(RimePrismBinParseError::InvalidCount);
        }
        cursor = cursor
            .checked_add(4)
            .ok_or(RimePrismBinParseError::OutOfBounds)?;
        let mut candidate_codes = Vec::with_capacity(candidate_count);
        for _ in 0..candidate_count {
            let (candidate_code, next) = read_len_string(bytes, cursor)?;
            cursor = next;
            candidate_codes.push(candidate_code);
        }
        rules.push(RimeToleranceRule::new(near_code, candidate_codes));
    }
    Ok(rules)
}

fn read_spelling_map(
    bytes: &[u8],
    offset: usize,
) -> Result<Vec<Vec<RimePrismSpellingDescriptor>>, RimePrismBinParseError> {
    let count = read_count(bytes, offset)?;
    let start = offset
        .checked_add(4)
        .ok_or(RimePrismBinParseError::OutOfBounds)?;
    let item_size = 8usize;
    let total = count
        .checked_mul(item_size)
        .and_then(|len| start.checked_add(len))
        .ok_or(RimePrismBinParseError::InvalidCount)?;
    if total > bytes.len() {
        return Err(RimePrismBinParseError::OutOfBounds);
    }

    let mut map = Vec::with_capacity(count);
    for index in 0..count {
        let item_offset = start
            .checked_add(
                index
                    .checked_mul(item_size)
                    .ok_or(RimePrismBinParseError::InvalidCount)?,
            )
            .ok_or(RimePrismBinParseError::OutOfBounds)?;
        let descriptor_count = read_count(bytes, item_offset)?;
        let descriptor_offset = read_offset_ptr(bytes, item_offset + 4)?
            .ok_or(RimePrismBinParseError::MissingRequiredSection)?;
        map.push(read_spelling_descriptors(
            bytes,
            descriptor_offset,
            descriptor_count,
        )?);
    }
    Ok(map)
}

fn identity_spelling_map(
    num_syllables: u32,
    num_spellings: u32,
    double_array_size: u32,
) -> Result<Vec<Vec<RimePrismSpellingDescriptor>>, RimePrismBinParseError> {
    if num_spellings > num_syllables
        || num_spellings > double_array_size
        || num_spellings > i32::MAX as u32
    {
        return Err(RimePrismBinParseError::InvalidCount);
    }
    Ok((0..num_spellings)
        .map(|spelling_id| {
            vec![RimePrismSpellingDescriptor {
                syllable_id: spelling_id as i32,
                spelling_type: 0,
                is_correction: false,
                credibility: 0.0,
                tips: String::new(),
            }]
        })
        .collect())
}

fn read_byte_backed_spelling_map(
    bytes: &[u8],
    offset: usize,
) -> Result<ByteBackedPrismSpellingMap, RimePrismBinParseError> {
    let spelling_count = read_count(bytes, offset)?;
    let start = offset
        .checked_add(4)
        .ok_or(RimePrismBinParseError::OutOfBounds)?;
    let total = spelling_count
        .checked_mul(ByteBackedPrismSpellingMap::ITEM_SIZE)
        .and_then(|len| start.checked_add(len))
        .ok_or(RimePrismBinParseError::InvalidCount)?;
    if total > bytes.len() {
        return Err(RimePrismBinParseError::OutOfBounds);
    }

    let mut descriptor_count = 0usize;
    let mut raw_descriptor_bytes = 0usize;
    let mut tips_payload_bytes = 0usize;
    let mut tip_count = 0usize;
    for index in 0..spelling_count {
        let item_offset = start
            .checked_add(
                index
                    .checked_mul(ByteBackedPrismSpellingMap::ITEM_SIZE)
                    .ok_or(RimePrismBinParseError::InvalidCount)?,
            )
            .ok_or(RimePrismBinParseError::OutOfBounds)?;
        let descriptors = read_count(bytes, item_offset)?;
        let descriptor_offset = read_offset_ptr(bytes, item_offset + 4)?
            .ok_or(RimePrismBinParseError::MissingRequiredSection)?;
        validate_spelling_descriptor_range(bytes, descriptor_offset, descriptors)?;
        descriptor_count = descriptor_count
            .checked_add(descriptors)
            .ok_or(RimePrismBinParseError::InvalidCount)?;
        raw_descriptor_bytes = raw_descriptor_bytes
            .checked_add(
                descriptors
                    .checked_mul(ByteBackedPrismSpellingMap::DESCRIPTOR_SIZE)
                    .ok_or(RimePrismBinParseError::InvalidCount)?,
            )
            .ok_or(RimePrismBinParseError::InvalidCount)?;
        for descriptor_index in 0..descriptors {
            let descriptor_base = descriptor_offset
                .checked_add(
                    descriptor_index
                        .checked_mul(ByteBackedPrismSpellingMap::DESCRIPTOR_SIZE)
                        .ok_or(RimePrismBinParseError::InvalidCount)?,
                )
                .ok_or(RimePrismBinParseError::OutOfBounds)?;
            if let Some(tip_len) = read_string_payload_len(bytes, descriptor_base + 12)? {
                if tip_len > 0 {
                    tip_count = tip_count.saturating_add(1);
                    tips_payload_bytes = tips_payload_bytes.saturating_add(tip_len);
                }
            }
        }
    }

    Ok(ByteBackedPrismSpellingMap::Explicit {
        offset,
        spelling_count,
        descriptor_count,
        raw_descriptor_bytes,
        tips_payload_bytes,
        tip_count,
    })
}

fn byte_backed_identity_spelling_map(
    num_syllables: u32,
    num_spellings: u32,
    double_array_size: u32,
) -> Result<ByteBackedPrismSpellingMap, RimePrismBinParseError> {
    if num_spellings > num_syllables
        || num_spellings > double_array_size
        || num_spellings > i32::MAX as u32
    {
        return Err(RimePrismBinParseError::InvalidCount);
    }
    Ok(ByteBackedPrismSpellingMap::Identity {
        spelling_count: num_spellings as usize,
        num_syllables: num_syllables as usize,
    })
}

fn read_spelling_descriptors(
    bytes: &[u8],
    offset: usize,
    count: usize,
) -> Result<Vec<RimePrismSpellingDescriptor>, RimePrismBinParseError> {
    let descriptor_size = 16usize;
    let total = count
        .checked_mul(descriptor_size)
        .and_then(|len| offset.checked_add(len))
        .ok_or(RimePrismBinParseError::InvalidCount)?;
    if total > bytes.len() {
        return Err(RimePrismBinParseError::OutOfBounds);
    }

    let mut descriptors = Vec::with_capacity(count);
    for index in 0..count {
        let descriptor_offset = offset
            .checked_add(
                index
                    .checked_mul(descriptor_size)
                    .ok_or(RimePrismBinParseError::InvalidCount)?,
            )
            .ok_or(RimePrismBinParseError::OutOfBounds)?;
        let packed_type = read_i32_le(bytes, descriptor_offset + 4).map_err(map_metadata_error)?;
        descriptors.push(RimePrismSpellingDescriptor {
            syllable_id: read_i32_le(bytes, descriptor_offset).map_err(map_metadata_error)?,
            spelling_type: packed_type & !(1 << 30),
            is_correction: packed_type & (1 << 30) != 0,
            credibility: read_f32_le(bytes, descriptor_offset + 8).map_err(map_metadata_error)?,
            tips: read_string(bytes, descriptor_offset + 12)?,
        });
    }
    Ok(descriptors)
}

fn validate_spelling_descriptor_range(
    bytes: &[u8],
    offset: usize,
    count: usize,
) -> Result<(), RimePrismBinParseError> {
    let total = count
        .checked_mul(ByteBackedPrismSpellingMap::DESCRIPTOR_SIZE)
        .and_then(|len| offset.checked_add(len))
        .ok_or(RimePrismBinParseError::InvalidCount)?;
    if total > bytes.len() {
        return Err(RimePrismBinParseError::OutOfBounds);
    }
    Ok(())
}

fn read_runtime_spelling_descriptor(
    bytes: &[u8],
    offset: usize,
    index: usize,
) -> Option<RuntimePrismSpellingDescriptor> {
    let descriptor_offset =
        offset.checked_add(index.checked_mul(ByteBackedPrismSpellingMap::DESCRIPTOR_SIZE)?)?;
    let packed_type = read_i32_le(bytes, descriptor_offset.checked_add(4)?).ok()?;
    Some(RuntimePrismSpellingDescriptor {
        syllable_id: read_i32_le(bytes, descriptor_offset).ok()?,
        spelling_type: packed_type & !(1 << 30),
        is_correction: packed_type & (1 << 30) != 0,
        credibility: read_f32_le(bytes, descriptor_offset.checked_add(8)?).ok()?,
    })
}

fn read_string(bytes: &[u8], offset: usize) -> Result<String, RimePrismBinParseError> {
    let Some(string_offset) = read_offset_ptr(bytes, offset)? else {
        return Ok(String::new());
    };
    if string_offset >= bytes.len() {
        return Err(RimePrismBinParseError::OutOfBounds);
    }
    let end = bytes[string_offset..]
        .iter()
        .position(|byte| *byte == 0)
        .map(|position| string_offset + position)
        .ok_or(RimePrismBinParseError::InvalidLength)?;
    std::str::from_utf8(&bytes[string_offset..end])
        .map(str::to_owned)
        .map_err(|_| RimePrismBinParseError::InvalidUtf8)
}

fn read_string_payload_len(
    bytes: &[u8],
    offset: usize,
) -> Result<Option<usize>, RimePrismBinParseError> {
    let Some(string_offset) = read_offset_ptr(bytes, offset)? else {
        return Ok(None);
    };
    if string_offset >= bytes.len() {
        return Err(RimePrismBinParseError::OutOfBounds);
    }
    let len = bytes[string_offset..]
        .iter()
        .position(|byte| *byte == 0)
        .ok_or(RimePrismBinParseError::InvalidLength)?;
    std::str::from_utf8(&bytes[string_offset..string_offset + len])
        .map_err(|_| RimePrismBinParseError::InvalidUtf8)?;
    Ok(Some(len))
}

fn read_len_string(bytes: &[u8], offset: usize) -> Result<(String, usize), RimePrismBinParseError> {
    let len = read_count(bytes, offset)?;
    let start = offset
        .checked_add(4)
        .ok_or(RimePrismBinParseError::OutOfBounds)?;
    let end = start
        .checked_add(len)
        .ok_or(RimePrismBinParseError::InvalidLength)?;
    if end > bytes.len() {
        return Err(RimePrismBinParseError::OutOfBounds);
    }
    let value = std::str::from_utf8(&bytes[start..end])
        .map(str::to_owned)
        .map_err(|_| RimePrismBinParseError::InvalidUtf8)?;
    Ok((value, end))
}

fn read_offset_ptr(
    bytes: &[u8],
    field_offset: usize,
) -> Result<Option<usize>, RimePrismBinParseError> {
    let raw = read_i32_le(bytes, field_offset).map_err(map_metadata_error)?;
    if raw == 0 {
        return Ok(None);
    }
    let target = field_offset
        .checked_add_signed(raw as isize)
        .ok_or(RimePrismBinParseError::OutOfBounds)?;
    if target >= bytes.len() {
        return Err(RimePrismBinParseError::OutOfBounds);
    }
    Ok(Some(target))
}

fn read_yune_payload_offset(
    bytes: &[u8],
    field_offset: usize,
    marker: &[u8],
) -> Result<Option<usize>, RimePrismBinParseError> {
    let raw = read_i32_le(bytes, field_offset).map_err(map_metadata_error)?;
    if raw == 0 {
        return Ok(None);
    }
    let Some(target) = field_offset.checked_add_signed(raw as isize) else {
        return Ok(None);
    };
    if target >= bytes.len() {
        return Ok(None);
    }
    if bytes[target..].starts_with(marker) || bytes[target..].starts_with(b"YUNE-") {
        Ok(Some(target))
    } else {
        Ok(None)
    }
}

fn read_count(bytes: &[u8], offset: usize) -> Result<usize, RimePrismBinParseError> {
    let count = read_u32_le(bytes, offset).map_err(map_metadata_error)?;
    usize::try_from(count).map_err(|_| RimePrismBinParseError::InvalidCount)
}

fn ensure_len(bytes: &[u8], len: usize) -> Result<(), RimePrismBinParseError> {
    if bytes.len() < len {
        return Err(RimePrismBinParseError::TooShort);
    }
    Ok(())
}

fn map_metadata_error(error: super::RimeCompiledMetadataError) -> RimePrismBinParseError {
    match error {
        super::RimeCompiledMetadataError::TooShort => RimePrismBinParseError::TooShort,
        super::RimeCompiledMetadataError::InvalidFormat => RimePrismBinParseError::InvalidFormat,
        super::RimeCompiledMetadataError::UnsupportedVersion => {
            RimePrismBinParseError::UnsupportedVersion
        }
        super::RimeCompiledMetadataError::MissingRequiredSection => {
            RimePrismBinParseError::MissingRequiredSection
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::build_prism_bin;

    #[derive(Debug)]
    struct TestPrismByteSource {
        bytes: Arc<[u8]>,
    }

    impl CompactTableByteSource for TestPrismByteSource {
        fn bytes(&self) -> &[u8] {
            &self.bytes
        }

        fn storage_label(&self) -> &'static str {
            "byte_backed"
        }

        fn mapping_mode(&self) -> &'static str {
            "test"
        }
    }

    fn test_source(bytes: impl Into<Arc<[u8]>>) -> Arc<dyn CompactTableByteSource> {
        Arc::new(TestPrismByteSource {
            bytes: bytes.into(),
        })
    }

    fn explicit_prism_fixture() -> (Vec<String>, Vec<u8>) {
        let syllabary_codes = ["ai", "an", "ao"].map(str::to_owned).to_vec();
        let algebra_formulas = [
            "derive/^ai$/a/".to_owned(),
            "derive/^an$/a/abbrev".to_owned(),
            "derive/^ao$/a/".to_owned(),
        ];
        let mut bytes = build_prism_bin(&syllabary_codes, &algebra_formulas, 1, 2);
        let descriptor_offset = descriptor_offset_for_spelling(&bytes, "a");
        let tip_offset = bytes.len();
        bytes.extend_from_slice(b"tip\0");
        put_relative_offset(&mut bytes, descriptor_offset + 12, tip_offset);
        (syllabary_codes, bytes)
    }

    fn descriptor_offset_for_spelling(bytes: &[u8], spelling: &str) -> usize {
        let payload = parse_rime_prism_bin_payload(bytes).expect("fixture prism should parse");
        let spelling_index = payload
            .double_array
            .as_ref()
            .and_then(|double_array| double_array.exact_match(spelling))
            .expect("fixture spelling should be indexed") as usize;
        let map_offset = read_offset_ptr(bytes, 56)
            .expect("fixture map pointer should parse")
            .expect("fixture should use an explicit spelling map");
        let item_offset = map_offset + 4 + spelling_index * ByteBackedPrismSpellingMap::ITEM_SIZE;
        read_offset_ptr(bytes, item_offset + 4)
            .expect("fixture descriptor pointer should parse")
            .expect("fixture descriptor pointer should be present")
    }

    fn put_relative_offset(bytes: &mut [u8], field_offset: usize, target: usize) {
        let relative = i32::try_from(target as isize - field_offset as isize)
            .expect("fixture offset should fit i32");
        bytes[field_offset..field_offset + 4].copy_from_slice(&relative.to_le_bytes());
    }

    fn byte_backed_payload(runtime: &RimePrismRuntimePayload) -> &ByteBackedRimePrismPayload {
        match &runtime.storage {
            RimePrismRuntimeStorage::ByteBacked(payload) => payload,
            RimePrismRuntimeStorage::Owned(_) => panic!("fixture should use byte-backed storage"),
        }
    }

    fn lookup_snapshot(
        runtime: &RimePrismRuntimePayload,
        syllabary_codes: &[String],
    ) -> Vec<(String, bool, bool, u32)> {
        runtime
            .lookup_canonical_codes("a", syllabary_codes)
            .into_iter()
            .map(|lookup| {
                (
                    lookup.code.to_owned(),
                    lookup.abbreviation,
                    lookup.correction,
                    lookup.credibility.to_bits(),
                )
            })
            .collect()
    }

    #[test]
    fn predictive_search_rejects_a_malformed_byte_backed_cycle() {
        let mut units = vec![0u32; 98];
        units[0] = 1 << 10;
        // For the `a` transition, offset 96 alternates positions 96 and 97.
        // The byte-backed reader must reject the repeated node rather than
        // enqueueing the malformed external graph forever.
        units[96] = (96 << 10) | u32::from(b'a');
        units[97] = (96 << 10) | u32::from(b'a');
        let bytes = units
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect::<Vec<_>>();
        let malformed = ByteBackedPrismDoubleArray {
            offset: 0,
            unit_count: 98,
        };

        assert!(malformed
            .predictive_search_with_alphabet(&bytes, b"", b"a", 0)
            .is_empty());
    }

    #[test]
    fn predictive_search_rejects_a_leaf_bearing_byte_cycle_before_production_limit() {
        let mut units = vec![0u32; 98];
        units[0] = 1 << 10;
        units[96] = (96 << 10) | ByteBackedPrismDoubleArray::HAS_LEAF | u32::from(b'a');
        units[97] = (96 << 10) | ByteBackedPrismDoubleArray::HAS_LEAF | u32::from(b'a');
        let bytes = units
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect::<Vec<_>>();
        let malformed = ByteBackedPrismDoubleArray {
            offset: 0,
            unit_count: 98,
        };

        assert!(malformed
            .predictive_search_with_alphabet(&bytes, b"", b"a", 512)
            .is_empty());
    }

    #[test]
    fn predictive_search_preserves_path_dependent_repeated_byte_backed_states() {
        let mut units = vec![0u32; 124];
        units[0] = 1 << 10;
        units[96] = (126 << 10) | u32::from(b'a');
        units[99] = (125 << 10) | u32::from(b'b');
        units[100] = (1 << 10) | ByteBackedPrismDoubleArray::HAS_LEAF | u32::from(b'z');
        units[101] = 42;
        let bytes = units
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect::<Vec<_>>();
        let repeated = ByteBackedPrismDoubleArray {
            offset: 0,
            unit_count: 124,
        };

        assert_eq!(
            repeated.predictive_search_with_alphabet(&bytes, b"", b"abz", 0),
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
    fn predictive_search_rejects_byte_backed_paths_over_the_depth_bound() {
        let key = "a".repeat(crate::dictionary::double_array::MAX_DARTS_PREDICTIVE_PATH_DEPTH + 1);
        let deep = DartsDoubleArray::build(&[(key, 7)]).expect("deep test key should build");
        let bytes = deep
            .units()
            .iter()
            .flat_map(|unit| unit.to_le_bytes())
            .collect::<Vec<_>>();
        let byte_backed = ByteBackedPrismDoubleArray {
            offset: 0,
            unit_count: deep.units().len(),
        };

        assert!(byte_backed
            .predictive_search_with_alphabet(&bytes, b"", b"a", 512)
            .is_empty());
    }

    fn runtime_payloads(
        syllabary_codes: &[String],
        algebra_formulas: &[String],
    ) -> [(&'static str, RimePrismRuntimePayload); 2] {
        let bytes = build_prism_bin(syllabary_codes, algebra_formulas, 1, 2);
        let owned = parse_rime_prism_bin_payload(&bytes)
            .map(RimePrismRuntimePayload::from)
            .expect("generated owned prism should parse");
        let byte_backed = parse_rime_prism_runtime_payload(Arc::new(TestPrismByteSource {
            bytes: Arc::from(bytes),
        }))
        .expect("generated byte-backed prism should parse");
        [("owned", owned), ("byte-backed", byte_backed)]
    }

    #[test]
    fn dual_source_parse_matches_single_source_layout_order_and_diagnostics() {
        let (syllabary_codes, bytes) = explicit_prism_fixture();
        let single = parse_rime_prism_runtime_payload(test_source(bytes.clone()))
            .expect("single-source runtime prism should parse");
        let validation_source = TestPrismByteSource {
            bytes: Arc::from(bytes.clone()),
        };
        let dual = parse_rime_prism_runtime_payload_with_validation_source(
            test_source(bytes),
            &validation_source,
        )
        .expect("dual-source runtime prism should parse");

        let single_payload = byte_backed_payload(&single);
        let dual_payload = byte_backed_payload(&dual);
        assert_eq!(dual_payload.double_array, single_payload.double_array);
        assert_eq!(dual_payload.spelling_map, single_payload.spelling_map);
        assert_eq!(dual_payload.corrections, single_payload.corrections);
        assert_eq!(dual_payload.tolerance_rules, single_payload.tolerance_rules);
        assert_eq!(
            lookup_snapshot(&dual, &syllabary_codes),
            lookup_snapshot(&single, &syllabary_codes)
        );
        assert_eq!(dual.memory_owner_rows(), single.memory_owner_rows());
        let tips = dual
            .memory_owner_rows()
            .into_iter()
            .find(|row| row.owner == "prism.tips_payload")
            .expect("tip diagnostics should be present");
        assert_eq!((tips.estimated_bytes, tips.item_count), (3, 1));
    }

    #[test]
    fn dual_source_parse_rejects_length_header_and_malformed_tip_mismatches() {
        let (_, bytes) = explicit_prism_fixture();

        let mut shorter = bytes.clone();
        shorter.pop();
        let shorter_source = TestPrismByteSource {
            bytes: Arc::from(shorter),
        };
        assert!(matches!(
            parse_rime_prism_runtime_payload_with_validation_source(
                test_source(bytes.clone()),
                &shorter_source,
            ),
            Err(RimePrismBinParseError::InvalidLength)
        ));

        let mut changed_header = bytes.clone();
        changed_header[32] ^= 1;
        let changed_header_source = TestPrismByteSource {
            bytes: Arc::from(changed_header),
        };
        assert!(matches!(
            parse_rime_prism_runtime_payload_with_validation_source(
                test_source(bytes.clone()),
                &changed_header_source,
            ),
            Err(RimePrismBinParseError::InvalidFormat)
        ));

        let mut malformed_tip = bytes.clone();
        let tip_pointer = descriptor_offset_for_spelling(&malformed_tip, "a") + 12;
        malformed_tip[tip_pointer..tip_pointer + 4].copy_from_slice(&i32::MAX.to_le_bytes());
        let malformed_tip_source = TestPrismByteSource {
            bytes: Arc::from(malformed_tip.clone()),
        };
        assert!(matches!(
            parse_rime_prism_runtime_payload_with_validation_source(
                test_source(bytes),
                &malformed_tip_source,
            ),
            Err(RimePrismBinParseError::OutOfBounds)
        ));
        assert!(matches!(
            parse_rime_prism_runtime_payload(test_source(malformed_tip)),
            Err(RimePrismBinParseError::OutOfBounds)
        ));
    }

    #[test]
    fn dual_source_runtime_uses_retained_source_after_validation_drops() {
        let (syllabary_codes, validation_bytes) = explicit_prism_fixture();
        let mut runtime_bytes = validation_bytes.clone();
        let first_descriptor = descriptor_offset_for_spelling(&runtime_bytes, "a");
        runtime_bytes[first_descriptor..first_descriptor + 4].copy_from_slice(&1i32.to_le_bytes());
        let validation_source = TestPrismByteSource {
            bytes: Arc::from(validation_bytes),
        };
        let runtime = parse_rime_prism_runtime_payload_with_validation_source(
            test_source(runtime_bytes),
            &validation_source,
        )
        .expect("matching file layout should parse through validation source");
        drop(validation_source);

        let resolved = lookup_snapshot(&runtime, &syllabary_codes);
        assert_eq!(resolved[0].0, "an");
        assert_eq!(resolved[1].0, "an");
        assert_eq!(resolved[2].0, "ao");
    }

    #[test]
    fn canonical_code_visitor_preserves_source_order_and_stops_early() {
        let syllabary_codes = ["ai", "an", "ao"].map(str::to_owned);
        let algebra_formulas =
            ["derive/^ai$/a/", "derive/^an$/a/abbrev", "derive/^ao$/a/"].map(str::to_owned);

        for (storage, runtime) in runtime_payloads(&syllabary_codes, &algebra_formulas) {
            let existing = runtime
                .lookup_canonical_codes("a", &syllabary_codes)
                .into_iter()
                .map(|lookup| {
                    (
                        lookup.code.to_owned(),
                        lookup.abbreviation,
                        lookup.correction,
                        lookup.credibility.to_bits(),
                    )
                })
                .collect::<Vec<_>>();
            assert_eq!(
                existing
                    .iter()
                    .map(|(code, abbreviation, _, _)| (code.as_str(), *abbreviation))
                    .collect::<Vec<_>>(),
                [("ai", false), ("an", true), ("ao", false)],
                "existing lookup order should remain unchanged for {storage} storage"
            );

            let mut visited = Vec::new();
            let outcome = runtime.visit_canonical_codes("a", &syllabary_codes, |lookup| {
                visited.push((
                    lookup.code.to_owned(),
                    lookup.abbreviation,
                    lookup.correction,
                    lookup.credibility.to_bits(),
                ));
                if visited.len() == 2 {
                    ControlFlow::Break("enough")
                } else {
                    ControlFlow::Continue(())
                }
            });

            assert_eq!(outcome, ControlFlow::Break("enough"), "{storage}");
            assert_eq!(
                visited,
                existing[..2],
                "visitor should expose source order without reading the trailing descriptor for {storage} storage"
            );
        }
    }

    #[test]
    fn canonical_code_visitor_supports_identity_maps_and_missing_spellings() {
        let syllabary_codes = ["z", "a"].map(str::to_owned);

        for (storage, runtime) in runtime_payloads(&syllabary_codes, &[]) {
            let mut visited = Vec::new();
            let outcome: ControlFlow<()> =
                runtime.visit_canonical_codes("a", &syllabary_codes, |lookup| {
                    visited.push((lookup.code.to_owned(), lookup.abbreviation));
                    ControlFlow::Continue(())
                });
            assert_eq!(outcome, ControlFlow::Continue(()), "{storage}");
            assert_eq!(visited, [("a".to_owned(), false)], "{storage}");

            let mut invoked = false;
            let missing: ControlFlow<()> =
                runtime.visit_canonical_codes("missing", &syllabary_codes, |_| {
                    invoked = true;
                    ControlFlow::Break(())
                });
            assert_eq!(missing, ControlFlow::Continue(()), "{storage}");
            assert!(
                !invoked,
                "missing spelling should not invoke {storage} visitor"
            );
        }
    }
}
