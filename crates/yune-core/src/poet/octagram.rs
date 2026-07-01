use std::mem;

use super::Grammar;
use crate::{DartsDoubleArray, DartsDoubleArrayError, MemoryOwnerClass, MemoryOwnerRow};

const FORMAT_MARKER: &[u8] = b"Rime::Grammar/1.0";
const FORMAT_LEN: usize = 32;
const CHECKSUM_OFFSET: usize = 32;
const DOUBLE_ARRAY_SIZE_OFFSET: usize = 36;
const DOUBLE_ARRAY_PTR_OFFSET: usize = 40;
const METADATA_LEN: usize = 44;
const VALUE_SCALE: f64 = 10_000.0;
const MAX_RESULTS: usize = 8;
const MAX_ENCODED_UNICODE: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OctagramGrammarConfig {
    pub collocation_max_length: usize,
    pub collocation_min_length: usize,
    pub collocation_penalty: f64,
    pub non_collocation_penalty: f64,
    pub weak_collocation_penalty: f64,
    pub rear_penalty: f64,
}

impl Default for OctagramGrammarConfig {
    fn default() -> Self {
        Self {
            collocation_max_length: 4,
            collocation_min_length: 3,
            collocation_penalty: -12.0,
            non_collocation_penalty: -12.0,
            weak_collocation_penalty: -24.0,
            rear_penalty: -18.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OctagramGrammarParseError {
    ShortHeader {
        len: usize,
    },
    InvalidFormat,
    ZeroDoubleArraySize,
    PayloadOutOfBounds {
        offset: i64,
        len: usize,
        file_len: usize,
    },
    PayloadMisaligned {
        len: usize,
    },
    InvalidDoubleArray(DartsDoubleArrayError),
}

#[derive(Clone, Debug, PartialEq)]
pub struct OctagramGrammar {
    trie: DartsDoubleArray,
    config: OctagramGrammarConfig,
    db_checksum: u32,
}

impl OctagramGrammar {
    pub fn from_bytes(
        bytes: &[u8],
        config: OctagramGrammarConfig,
    ) -> Result<Self, OctagramGrammarParseError> {
        if bytes.len() < METADATA_LEN {
            return Err(OctagramGrammarParseError::ShortHeader { len: bytes.len() });
        }
        if !valid_format_marker(&bytes[..FORMAT_LEN]) {
            return Err(OctagramGrammarParseError::InvalidFormat);
        }

        let db_checksum = read_u32_le(bytes, CHECKSUM_OFFSET);
        let double_array_size = read_u32_le(bytes, DOUBLE_ARRAY_SIZE_OFFSET) as usize;
        if double_array_size == 0 {
            return Err(OctagramGrammarParseError::ZeroDoubleArraySize);
        }
        let payload_len = double_array_size.checked_mul(mem::size_of::<u32>()).ok_or(
            OctagramGrammarParseError::PayloadOutOfBounds {
                offset: 0,
                len: usize::MAX,
                file_len: bytes.len(),
            },
        )?;
        if payload_len % mem::size_of::<u32>() != 0 {
            return Err(OctagramGrammarParseError::PayloadMisaligned { len: payload_len });
        }
        let relative_payload_offset = read_i32_le(bytes, DOUBLE_ARRAY_PTR_OFFSET);
        let payload_offset = DOUBLE_ARRAY_PTR_OFFSET as i64 + i64::from(relative_payload_offset);
        let payload_end = payload_offset.checked_add(payload_len as i64).ok_or(
            OctagramGrammarParseError::PayloadOutOfBounds {
                offset: payload_offset,
                len: payload_len,
                file_len: bytes.len(),
            },
        )?;
        if payload_offset < 0 || payload_end > bytes.len() as i64 {
            return Err(OctagramGrammarParseError::PayloadOutOfBounds {
                offset: payload_offset,
                len: payload_len,
                file_len: bytes.len(),
            });
        }

        let payload = &bytes[payload_offset as usize..payload_end as usize];
        let units = payload
            .chunks_exact(mem::size_of::<u32>())
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect::<Vec<_>>();
        let trie = DartsDoubleArray::from_units(units)
            .map_err(OctagramGrammarParseError::InvalidDoubleArray)?;
        Ok(Self {
            trie,
            config,
            db_checksum,
        })
    }

    #[must_use]
    pub const fn config(&self) -> OctagramGrammarConfig {
        self.config
    }

    #[must_use]
    pub const fn db_checksum(&self) -> u32 {
        self.db_checksum
    }

    #[must_use]
    pub fn memory_owner_row(&self) -> MemoryOwnerRow {
        MemoryOwnerRow::new(
            "poet.octagram_double_array",
            MemoryOwnerClass::HeapOwnedReducible,
            mem::size_of::<DartsDoubleArray>().saturating_add(
                self.trie
                    .units()
                    .len()
                    .saturating_mul(mem::size_of::<u32>()),
            ),
            self.trie.units().len(),
            "DartsDoubleArray",
            "octagram grammar double-array units parsed from .gram",
        )
    }
}

impl Grammar for OctagramGrammar {
    fn query(&self, context: &str, word: &str, is_rear: bool) -> f64 {
        let max_query_chars = self
            .config
            .collocation_max_length
            .saturating_sub(1)
            .min(MAX_ENCODED_UNICODE);
        if max_query_chars == 0 {
            return self.config.non_collocation_penalty;
        }
        if context.is_empty() {
            return self.config.non_collocation_penalty;
        }

        let word_head = first_n_chars(word, max_query_chars);
        let word_query = encode_octagram_key(&word_head);
        if word_query.is_empty() {
            return self.config.non_collocation_penalty;
        }

        let mut result = self.config.non_collocation_penalty;
        let context_tail = last_n_chars(context, max_query_chars);
        let context_query = encode_octagram_key(&context_tail);
        let context_starts = encoded_char_starts(&context_query);
        for (index, suffix_start) in context_starts.iter().enumerate() {
            let context_len = context_starts.len() - index;
            let context_suffix = &context_query[*suffix_start..];
            let mut key = Vec::with_capacity(context_suffix.len() + word_query.len());
            key.extend_from_slice(context_suffix);
            key.extend_from_slice(&word_query);
            let mut valid_results = 0usize;
            for matched in self.trie.common_prefix_search_bytes(&key) {
                if matched.length <= context_suffix.len() {
                    continue;
                }
                valid_results += 1;
                if valid_results > MAX_RESULTS {
                    break;
                }
                let word_match_bytes = matched.length - context_suffix.len();
                if word_match_bytes > word_query.len() {
                    continue;
                }
                let Some(word_match_len) = encoded_prefix_char_count(&word_query, word_match_bytes)
                else {
                    continue;
                };
                let collocation_len = context_len + word_match_len;
                let covers_whole_query = *suffix_start == 0 && word_match_bytes == word_query.len();
                let penalty = if collocation_len >= self.config.collocation_min_length
                    || covers_whole_query
                {
                    self.config.collocation_penalty
                } else {
                    self.config.weak_collocation_penalty
                };
                result = result.max(scale_value(matched.value) + penalty);
            }
        }

        if is_rear && word_head.chars().count() == word.chars().count() {
            let mut rear_key = word_query;
            rear_key.push(b'$');
            if let Some(value) = self.trie.exact_match_bytes(&rear_key) {
                result = result.max(scale_value(value) + self.config.rear_penalty);
            }
        }

        result
    }
}

#[must_use]
pub fn encode_octagram_key(text: &str) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(text.len());
    for ch in text.chars() {
        encode_code_point(ch as u32, &mut encoded);
    }
    encoded
}

fn encode_code_point(mut code_point: u32, encoded: &mut Vec<u8>) {
    if code_point < 0x80 {
        encoded.push(if code_point == 0 {
            0xe0
        } else {
            code_point as u8
        });
    } else if (0x4000..0xa000).contains(&code_point) {
        if code_point.trailing_zeros() >= 8 {
            encoded.push(0xe1);
            encoded.push(((code_point >> 8) + 0x40) as u8);
        } else {
            encoded.push(((code_point >> 8) + 0x40) as u8);
            encoded.push((code_point & 0xff) as u8);
        }
    } else {
        let mut bits = 32;
        while bits > 0 && (code_point & 0xfe00_0000) == 0 {
            bits -= 7;
            code_point <<= 7;
        }
        let mut bytes_to_encode = (bits + 6) / 7;
        encoded.push((0xe0 | bytes_to_encode) as u8);
        while bytes_to_encode > 0 {
            bytes_to_encode -= 1;
            encoded.push((((code_point >> 25) & 0x7f) | 0x80) as u8);
        }
    }
}

fn valid_format_marker(bytes: &[u8]) -> bool {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    &bytes[..end] == FORMAT_MARKER
}

fn read_u32_le(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn read_i32_le(bytes: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn scale_value(value: u32) -> f64 {
    f64::from(value) / VALUE_SCALE
}

fn first_n_chars(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

fn last_n_chars(text: &str, max: usize) -> String {
    let mut chars = text.chars().rev().take(max).collect::<Vec<_>>();
    chars.reverse();
    chars.into_iter().collect()
}

fn encoded_char_starts(encoded: &[u8]) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut index = 0usize;
    while index < encoded.len() {
        starts.push(index);
        index += encoded_char_width(encoded[index]);
    }
    starts
}

fn encoded_prefix_char_count(encoded: &[u8], length: usize) -> Option<usize> {
    if length > encoded.len() {
        return None;
    }
    let mut count = 0usize;
    let mut index = 0usize;
    while index < length {
        let width = encoded_char_width(encoded[index]);
        if index + width > length {
            return None;
        }
        index += width;
        count += 1;
    }
    Some(count)
}

fn encoded_char_width(first: u8) -> usize {
    if (first & 0x80) == 0 {
        1
    } else if (first & 0xf0) == 0xe0 {
        usize::from(first & 0x0f) + 1
    } else {
        2
    }
}
