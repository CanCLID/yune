use super::{DartsDoubleArray, RimePrismSpellingDescriptor};
use crate::dictionary::table_writer::{
    append_c_string, put_c_string, put_f32_le, put_i32_le, put_offset, put_u32_le,
};
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};

const SPELLING_NORMAL: i32 = 0;
const SPELLING_FUZZY: i32 = 1;
const SPELLING_ABBREVIATION: i32 = 2;
const FUZZY_CREDIBILITY: f32 = -std::f32::consts::LN_2;
const ABBREVIATION_CREDIBILITY: f32 = -std::f32::consts::LN_2;
const CORRECTION_CREDIBILITY: f32 = -std::f32::consts::LN_10 * 2.0;

pub fn build_prism_bin(
    syllabary: &[String],
    algebra_formulas: &[String],
    dict_file_checksum: u32,
    schema_file_checksum: u32,
) -> Vec<u8> {
    let spelling_map = build_spelling_descriptors(syllabary, algebra_formulas);
    let keys = spelling_map
        .keys()
        .enumerate()
        .map(|(index, spelling)| (spelling.as_str(), index as u32))
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
    bytes
}

fn build_spelling_descriptors(
    syllabary: &[String],
    algebra_formulas: &[String],
) -> BTreeMap<String, Vec<RimePrismSpellingDescriptor>> {
    let mut entries = syllabary
        .iter()
        .enumerate()
        .map(|(index, syllable)| PrismSpelling {
            spelling: syllable.clone(),
            descriptor: RimePrismSpellingDescriptor {
                syllable_id: index as i32,
                spelling_type: SPELLING_NORMAL,
                is_correction: false,
                credibility: 0.0,
                tips: String::new(),
            },
        })
        .collect::<Vec<_>>();

    for formula in algebra_formulas {
        let Some(formula) = PrismFormula::parse(formula) else {
            continue;
        };
        entries = formula.apply(entries);
        dedupe_entries(&mut entries);
    }

    let mut map = BTreeMap::<String, Vec<RimePrismSpellingDescriptor>>::new();
    for entry in entries {
        if !entry.spelling.is_empty() {
            map.entry(entry.spelling)
                .or_default()
                .push(entry.descriptor);
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

#[derive(Clone)]
struct PrismSpelling {
    spelling: String,
    descriptor: RimePrismSpellingDescriptor,
}

enum PrismFormula {
    Xlit(Vec<(char, char)>),
    Transform {
        pattern: Regex,
        replacement: String,
        keep_original: bool,
        descriptor_update: DescriptorUpdate,
        erase: bool,
    },
}

#[derive(Clone, Copy)]
struct DescriptorUpdate {
    spelling_type: i32,
    is_correction: bool,
    credibility: f32,
}

impl DescriptorUpdate {
    const NORMAL: Self = Self {
        spelling_type: SPELLING_NORMAL,
        is_correction: false,
        credibility: 0.0,
    };
    const FUZZY: Self = Self {
        spelling_type: SPELLING_FUZZY,
        is_correction: false,
        credibility: FUZZY_CREDIBILITY,
    };
    const ABBREVIATION: Self = Self {
        spelling_type: SPELLING_ABBREVIATION,
        is_correction: false,
        credibility: ABBREVIATION_CREDIBILITY,
    };
    const CORRECTION: Self = Self {
        spelling_type: SPELLING_NORMAL,
        is_correction: true,
        credibility: CORRECTION_CREDIBILITY,
    };

    fn apply_to(self, descriptor: &mut RimePrismSpellingDescriptor) {
        descriptor.spelling_type = self.spelling_type;
        descriptor.is_correction = self.is_correction;
        descriptor.credibility = self.credibility;
    }
}

impl PrismFormula {
    fn parse(definition: &str) -> Option<Self> {
        let separator = definition.chars().find(|ch| !ch.is_ascii_lowercase())?;
        let args = definition.split(separator).collect::<Vec<_>>();
        match args.first().copied()? {
            "xlit" => {
                let left = args.get(1)?.chars().collect::<Vec<_>>();
                let right = args.get(2)?.chars().collect::<Vec<_>>();
                (left.len() == right.len())
                    .then(|| Self::Xlit(left.into_iter().zip(right).collect()))
            }
            "xform" => Self::transform(&args, false, DescriptorUpdate::NORMAL),
            "derive" => {
                let update = match args.get(3).copied() {
                    Some("fuzz") => DescriptorUpdate::FUZZY,
                    Some("abbrev") => DescriptorUpdate::ABBREVIATION,
                    Some("correction") => DescriptorUpdate::CORRECTION,
                    _ => DescriptorUpdate::NORMAL,
                };
                Self::transform(&args, true, update)
            }
            "fuzz" => Self::transform(&args, true, DescriptorUpdate::FUZZY),
            "abbrev" => Self::transform(&args, true, DescriptorUpdate::ABBREVIATION),
            "erase" => Some(Self::Transform {
                pattern: Regex::new(args.get(1)?).ok()?,
                replacement: String::new(),
                keep_original: false,
                descriptor_update: DescriptorUpdate::NORMAL,
                erase: true,
            }),
            _ => None,
        }
    }

    fn transform(
        args: &[&str],
        keep_original: bool,
        descriptor_update: DescriptorUpdate,
    ) -> Option<Self> {
        Some(Self::Transform {
            pattern: Regex::new(args.get(1)?).ok()?,
            replacement: (*args.get(2)?).to_string(),
            keep_original,
            descriptor_update,
            erase: false,
        })
    }

    fn apply(&self, entries: Vec<PrismSpelling>) -> Vec<PrismSpelling> {
        let mut output = Vec::new();
        for entry in entries {
            match self {
                Self::Xlit(map) => {
                    let mut modified = false;
                    let spelling = entry
                        .spelling
                        .chars()
                        .map(|ch| {
                            if let Some((_, replacement)) =
                                map.iter().find(|(source, _)| *source == ch)
                            {
                                modified = true;
                                *replacement
                            } else {
                                ch
                            }
                        })
                        .collect::<String>();
                    if modified {
                        output.push(PrismSpelling { spelling, ..entry });
                    } else {
                        output.push(entry);
                    }
                }
                Self::Transform {
                    pattern,
                    replacement,
                    keep_original,
                    descriptor_update,
                    erase,
                } => {
                    let transformed = pattern
                        .replace_all(&entry.spelling, replacement.as_str())
                        .into_owned();
                    let modified = transformed != entry.spelling;
                    if modified && *keep_original {
                        output.push(entry.clone());
                    }
                    if modified {
                        if !(*erase && transformed.is_empty()) {
                            let mut descriptor = entry.descriptor;
                            descriptor_update.apply_to(&mut descriptor);
                            output.push(PrismSpelling {
                                spelling: transformed,
                                descriptor,
                            });
                        }
                    } else {
                        output.push(entry);
                    }
                }
            }
        }
        output
    }
}

fn dedupe_entries(entries: &mut Vec<PrismSpelling>) {
    let mut seen = BTreeSet::new();
    entries.retain(|entry| {
        seen.insert((
            entry.spelling.clone(),
            entry.descriptor.syllable_id,
            entry.descriptor.spelling_type,
            entry.descriptor.is_correction,
            entry.descriptor.credibility.to_bits(),
        ))
    });
}
