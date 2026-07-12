use std::{fs, sync::Arc};

use yune_core::{
    parse_rime_prism_bin_payload, parse_rime_prism_runtime_payload,
    parse_rime_table_bin_advanced_data, CompactTableByteSource, CompactTableStore,
    RimePrismRuntimePayload,
};

#[derive(Debug)]
struct AuditBytes(Vec<u8>);

impl CompactTableByteSource for AuditBytes {
    fn bytes(&self) -> &[u8] {
        &self.0
    }

    fn storage_label(&self) -> &'static str {
        "external_audit"
    }

    fn mapping_mode(&self) -> &'static str {
        "owned_bytes"
    }
}

fn main() {
    let root = std::env::args()
        .nth(1)
        .expect("usage: m59-4d-kcount-audit <deployed-build-dir>");
    let table_bytes = fs::read(format!("{root}/luna_pinyin.table.bin"))
        .expect("read deployed luna table");
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("parse deployed luna table metadata");
    let table = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("open deployed compact luna table");
    let syllabary = table.syllabary_codes();

    let prism_bytes = fs::read(format!("{root}/luna_quanpin.prism.bin"))
        .expect("read deployed luna prism");
    let owned: RimePrismRuntimePayload = parse_rime_prism_bin_payload(&prism_bytes)
        .expect("parse owned prism")
        .into();
    let byte_backed = parse_rime_prism_runtime_payload(Arc::new(AuditBytes(prism_bytes)))
        .expect("parse byte-backed prism");

    let owned_rows = owned.predictive_canonical_codes_with_limit("k", syllabary, 512);
    let byte_rows = byte_backed.predictive_canonical_codes_with_limit("k", syllabary, 512);
    println!("syllabary_count={}", syllabary.len());
    println!("owned_canonical_k_count={}", owned_rows.len());
    println!("byte_backed_canonical_k_count={}", byte_rows.len());
    assert_eq!(owned_rows.len(), 504);
    assert_eq!(byte_rows.len(), 504);
    assert_eq!(owned_rows, byte_rows);
}
