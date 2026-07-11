use super::{DartsDoubleArray, RimePrismSpellingDescriptor};
use crate::dictionary::table_writer::{
    append_c_string, put_c_string, put_f32_le, put_i32_le, put_offset, put_u32_le,
};
use crate::spelling_algebra::SpellingAlgebra;
use std::collections::BTreeMap;

pub fn build_prism_bin(
    syllabary: &[String],
    algebra_formulas: &[String],
    dict_file_checksum: u32,
    schema_file_checksum: u32,
) -> Vec<u8> {
    let spelling_map = build_spelling_descriptors(syllabary, algebra_formulas);
    let identity_spelling_map = is_identity_spelling_map(&spelling_map, syllabary);
    let keys = spelling_map
        .iter()
        .enumerate()
        .map(|(index, (spelling, descriptors))| {
            let value = if identity_spelling_map {
                u32::try_from(descriptors[0].syllable_id)
                    .expect("identity spelling syllable id should be nonnegative")
            } else {
                index as u32
            };
            (spelling.as_str(), value)
        })
        .collect::<Vec<_>>();
    let double_array = if keys.is_empty() {
        None
    } else {
        Some(DartsDoubleArray::build(&keys).expect("generated spelling keys should build"))
    };

    let mut bytes = vec![0; 320];
    put_c_string(&mut bytes, 0, b"Rime::Prism/4.0");
    put_u32_le(&mut bytes, 32, dict_file_checksum);
    put_u32_le(&mut bytes, 36, schema_file_checksum);
    put_u32_le(&mut bytes, 40, syllabary.len() as u32);
    put_u32_le(&mut bytes, 44, spelling_map.len() as u32);

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
        bytes.resize(spelling_map_offset + 4 + spelling_map.len() * 8, 0);
        put_u32_le(&mut bytes, spelling_map_offset, spelling_map.len() as u32);

        let empty_tip_offset = append_c_string(&mut bytes, "");
        for (index, descriptors) in spelling_map.values().enumerate() {
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
    spelling_map: &BTreeMap<String, Vec<RimePrismSpellingDescriptor>>,
    syllabary: &[String],
) -> bool {
    spelling_map.len() == syllabary.len()
        && spelling_map.iter().all(|(spelling, descriptors)| {
            let [descriptor] = descriptors.as_slice() else {
                return false;
            };
            usize::try_from(descriptor.syllable_id)
                .ok()
                .and_then(|syllable_id| syllabary.get(syllable_id))
                .is_some_and(|canonical| canonical == spelling)
                && descriptor.spelling_type == 0
                && !descriptor.is_correction
                && descriptor.credibility == 0.0
                && descriptor.tips.is_empty()
        })
}

fn build_spelling_descriptors(
    syllabary: &[String],
    algebra_formulas: &[String],
) -> BTreeMap<String, Vec<RimePrismSpellingDescriptor>> {
    let algebra = SpellingAlgebra::parse(algebra_formulas);
    let mut map = BTreeMap::<String, Vec<RimePrismSpellingDescriptor>>::new();
    for (index, syllable) in syllabary.iter().enumerate() {
        for variant in algebra.expand_deployed_spelling_variants(syllable) {
            if variant.code.is_empty() {
                continue;
            }
            let properties = variant.properties;
            map.entry(variant.code)
                .or_default()
                .push(RimePrismSpellingDescriptor {
                    syllable_id: index as i32,
                    spelling_type: properties.spelling_type as i32,
                    is_correction: properties.is_correction,
                    // librime composes `SpellingProperties::credibility` as
                    // double and narrows only into the on-disk float field.
                    credibility: properties.credibility as f32,
                    tips: String::new(),
                });
        }
    }
    for descriptors in map.values_mut() {
        descriptors.sort_by(|left, right| {
            left.syllable_id
                .cmp(&right.syllable_id)
                .then(left.spelling_type.cmp(&right.spelling_type))
                .then(left.is_correction.cmp(&right.is_correction))
                .then(left.credibility.to_bits().cmp(&right.credibility.to_bits()))
        });
    }
    map
}
