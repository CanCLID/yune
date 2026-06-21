use super::TableDictionary;
use crate::dictionary::table_writer::{
    put_c_string, put_len_string, put_u32_le, put_u32_le_extend,
};

pub fn build_reverse_bin(dict: &TableDictionary, dict_file_checksum: u32) -> Vec<u8> {
    let mut bytes = vec![0; 64];
    put_c_string(&mut bytes, 0, b"Rime::Reverse/4.0");
    put_u32_le(&mut bytes, 32, dict_file_checksum);
    bytes.extend_from_slice(b"YUNE-REVERSE\0");
    put_u32_le_extend(&mut bytes, dict.entries().len() as u32);
    for entry in dict.entries() {
        put_len_string(&mut bytes, &entry.code);
        put_len_string(&mut bytes, &entry.text);
    }

    put_u32_le_extend(&mut bytes, dict.dict_settings().len() as u32);
    for (key, value) in dict.dict_settings() {
        put_len_string(&mut bytes, key);
        put_len_string(&mut bytes, value);
    }

    let mut stems = dict.stems().iter().collect::<Vec<_>>();
    stems.sort_by(|left, right| left.0.cmp(right.0));
    put_u32_le_extend(&mut bytes, stems.len() as u32);
    for (text, values) in stems {
        put_len_string(&mut bytes, text);
        put_u32_le_extend(&mut bytes, values.len() as u32);
        for stem in values {
            put_len_string(&mut bytes, stem);
        }
    }
    bytes
}
