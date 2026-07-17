use super::compiled_table::CanonicalCodeSequence;
use super::{double_array::DartsKeyValue, DartsDoubleArray};
use crate::dictionary::table_writer::{
    append_c_string, put_c_string, put_f32_le, put_i32_le, put_offset, put_u32_le,
};
use crate::spelling_algebra::SpellingAlgebra;

#[derive(Clone, Copy, Debug, PartialEq)]
struct PrismBuildDescriptor {
    syllable_id: i32,
    spelling_type: i32,
    is_correction: bool,
    credibility: f32,
}

struct PrismSpellingRecord {
    spelling: String,
    descriptor: PrismBuildDescriptor,
}

struct PrismSpellingEntry {
    spelling: String,
    descriptor_start: u32,
    descriptor_count: u32,
    darts_value: u32,
}

impl DartsKeyValue for PrismSpellingEntry {
    fn key_bytes(&self) -> &[u8] {
        self.spelling.as_bytes()
    }

    fn value(&self) -> u32 {
        self.darts_value
    }
}

struct PrismSpellingMap {
    entries: Vec<PrismSpellingEntry>,
    descriptors: Vec<PrismBuildDescriptor>,
}

pub fn build_prism_bin(
    syllabary: &(impl CanonicalCodeSequence + ?Sized),
    algebra_formulas: &[String],
    dict_file_checksum: u32,
    schema_file_checksum: u32,
) -> Vec<u8> {
    let mut spelling_map = build_spelling_descriptors(syllabary, algebra_formulas);
    let identity_spelling_map = is_identity_spelling_map(&spelling_map, syllabary);
    spelling_map.assign_darts_values(identity_spelling_map);
    let double_array = if spelling_map.entries.is_empty() {
        None
    } else {
        Some(
            DartsDoubleArray::build_sorted_key_values(&spelling_map.entries)
                .expect("generated spelling keys should build"),
        )
    };

    let mut bytes = vec![0; 320];
    put_c_string(&mut bytes, 0, b"Rime::Prism/4.0");
    put_u32_le(&mut bytes, 32, dict_file_checksum);
    put_u32_le(&mut bytes, 36, schema_file_checksum);
    put_u32_le(&mut bytes, 40, syllabary.len() as u32);
    put_u32_le(&mut bytes, 44, spelling_map.entries.len() as u32);

    if let Some(double_array) = &double_array {
        let double_array_offset = bytes.len();
        for unit in double_array.units() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        put_u32_le(&mut bytes, 48, double_array.units().len() as u32);
        put_offset(&mut bytes, 52, double_array_offset);
    }

    // Pinned librime encodes a pure identity spelling map as a null pointer;
    // Darts values are then syllable ids directly. Preserve that representation
    // so large no-algebra schemas do not serialize or materialize one default
    // descriptor per code.
    if !identity_spelling_map {
        let spelling_map_offset = bytes.len();
        bytes.resize(spelling_map_offset + 4 + spelling_map.entries.len() * 8, 0);
        put_u32_le(
            &mut bytes,
            spelling_map_offset,
            spelling_map.entries.len() as u32,
        );

        let empty_tip_offset = append_c_string(&mut bytes, "");
        for (index, entry) in spelling_map.entries.iter().enumerate() {
            let descriptors = spelling_map.descriptors_for(entry);
            let item_offset = spelling_map_offset + 4 + index * 8;
            put_u32_le(&mut bytes, item_offset, descriptors.len() as u32);
            let descriptor_offset = bytes.len();
            bytes.resize(descriptor_offset + descriptors.len() * 16, 0);
            for (descriptor_index, descriptor) in descriptors.iter().enumerate() {
                let current = descriptor_offset + descriptor_index * 16;
                put_i32_le(&mut bytes, current, descriptor.syllable_id);
                let packed_type =
                    descriptor.spelling_type | if descriptor.is_correction { 1 << 30 } else { 0 };
                put_i32_le(&mut bytes, current + 4, packed_type);
                put_f32_le(&mut bytes, current + 8, descriptor.credibility);
                put_offset(&mut bytes, current + 12, empty_tip_offset);
            }
            put_offset(&mut bytes, item_offset + 4, descriptor_offset);
        }
        put_offset(&mut bytes, 56, spelling_map_offset);
    }
    bytes
}

fn is_identity_spelling_map(
    spelling_map: &PrismSpellingMap,
    syllabary: &(impl CanonicalCodeSequence + ?Sized),
) -> bool {
    spelling_map.entries.len() == syllabary.len()
        && spelling_map.entries.iter().all(|entry| {
            let descriptors = spelling_map.descriptors_for(entry);
            let [descriptor] = descriptors else {
                return false;
            };
            usize::try_from(descriptor.syllable_id)
                .ok()
                .and_then(|syllable_id| syllabary.get(syllable_id))
                .is_some_and(|canonical| canonical == entry.spelling)
                && descriptor.spelling_type == 0
                && !descriptor.is_correction
                && descriptor.credibility == 0.0
        })
}

fn build_spelling_descriptors(
    syllabary: &(impl CanonicalCodeSequence + ?Sized),
    algebra_formulas: &[String],
) -> PrismSpellingMap {
    let algebra = SpellingAlgebra::parse(algebra_formulas);
    let mut records = Vec::new();
    for index in 0..syllabary.len() {
        let syllable = syllabary
            .get(index)
            .expect("syllabary index comes from its reported length");
        for variant in algebra.expand_deployed_spelling_variants(syllable) {
            if variant.code.is_empty() {
                continue;
            }
            let properties = variant.properties;
            records.push(PrismSpellingRecord {
                spelling: variant.code,
                descriptor: PrismBuildDescriptor {
                    syllable_id: index as i32,
                    spelling_type: properties.spelling_type as i32,
                    is_correction: properties.is_correction,
                    // librime composes `SpellingProperties::credibility` as
                    // double and narrows only into the on-disk float field.
                    credibility: properties.credibility as f32,
                },
            });
        }
    }

    records.sort_unstable_by(|left, right| {
        left.spelling
            .cmp(&right.spelling)
            .then_with(|| compare_build_descriptors(&left.descriptor, &right.descriptor))
    });
    let spelling_count = records
        .iter()
        .enumerate()
        .filter(|(index, record)| *index == 0 || records[*index - 1].spelling != record.spelling)
        .count();
    let mut entries = Vec::<PrismSpellingEntry>::with_capacity(spelling_count);
    let mut descriptors = Vec::with_capacity(records.len());
    for record in records {
        if entries
            .last()
            .map_or(true, |entry| entry.spelling != record.spelling)
        {
            entries.push(PrismSpellingEntry {
                spelling: record.spelling,
                descriptor_start: u32::try_from(descriptors.len())
                    .expect("generated descriptor offset should fit u32"),
                descriptor_count: 0,
                darts_value: 0,
            });
        }
        descriptors.push(record.descriptor);
        let entry = entries
            .last_mut()
            .expect("a generated descriptor should own a spelling entry");
        entry.descriptor_count = entry
            .descriptor_count
            .checked_add(1)
            .expect("generated descriptor count should fit u32");
    }
    PrismSpellingMap {
        entries,
        descriptors,
    }
}

fn compare_build_descriptors(
    left: &PrismBuildDescriptor,
    right: &PrismBuildDescriptor,
) -> std::cmp::Ordering {
    left.syllable_id
        .cmp(&right.syllable_id)
        .then(left.spelling_type.cmp(&right.spelling_type))
        .then(left.is_correction.cmp(&right.is_correction))
        .then(left.credibility.to_bits().cmp(&right.credibility.to_bits()))
}

impl PrismSpellingMap {
    fn descriptors_for(&self, entry: &PrismSpellingEntry) -> &[PrismBuildDescriptor] {
        let start = entry.descriptor_start as usize;
        let end = start + entry.descriptor_count as usize;
        &self.descriptors[start..end]
    }

    fn assign_darts_values(&mut self, identity_spelling_map: bool) {
        for (index, entry) in self.entries.iter_mut().enumerate() {
            entry.darts_value = if identity_spelling_map {
                let descriptor = self.descriptors[entry.descriptor_start as usize];
                u32::try_from(descriptor.syllable_id)
                    .expect("identity spelling syllable id should be nonnegative")
            } else {
                u32::try_from(index).expect("generated spelling index should fit u32")
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{build_spelling_descriptors, is_identity_spelling_map};

    #[test]
    fn flat_spelling_descriptors_group_collisions_in_canonical_order() {
        let syllabary = vec!["za".to_owned(), "xa".to_owned()];
        let formulas = vec!["derive/^z/x/".to_owned()];
        let mut map = build_spelling_descriptors(&syllabary, &formulas);

        assert_eq!(
            map.entries
                .iter()
                .map(|entry| entry.spelling.as_str())
                .collect::<Vec<_>>(),
            vec!["xa", "za"]
        );
        assert_eq!(
            map.descriptors_for(&map.entries[0])
                .iter()
                .map(|descriptor| descriptor.syllable_id)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert_eq!(
            map.descriptors_for(&map.entries[1])
                .iter()
                .map(|descriptor| descriptor.syllable_id)
                .collect::<Vec<_>>(),
            vec![0]
        );
        assert!(!is_identity_spelling_map(&map, &syllabary));

        map.assign_darts_values(false);
        assert_eq!(
            map.entries
                .iter()
                .map(|entry| entry.darts_value)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
    }
}
