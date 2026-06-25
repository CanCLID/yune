# M38 Marisa Table Fixture

Date: 2026-06-24

This fixture records the upstream `luna_pinyin` deployed table used by the M38
isolated native engine benchmark. It is not an extracted-payload-only probe:
the final Yune Track A hot path selected the deployed
`luna_pinyin.table.bin` through `CompactTableStore::from_table_bin_byte_source`
with `selected_storage=rsmarisa_byte_backed`.

Final evidence:

- Final run: `docs/reports/evidence/m38-engine-performance-parity/phase-3-final-native/`
- Status CSV: `phase-3-final-native/product_path_status.csv`
- Raw lookup CSV: `phase-3-final-native/raw_lookup_microbench.csv`
- Per-key counters: `phase-3-final-native/m37_metrics.csv`

Selected fixture row:

| Field | Value |
| --- | --- |
| Schema | `luna_pinyin` |
| Dictionary / prism | `luna_pinyin` / `luna_pinyin` |
| Table path | `target/native-inprocess/phase-3-final-native/track-a-yune/user/build/luna_pinyin.table.bin` |
| Prism path | `target/native-inprocess/phase-3-final-native/track-a-yune/user/build/luna_pinyin.prism.bin` |
| Table format | `rime_marisa_string_table:1574520` |
| Selected storage | `rsmarisa_byte_backed` |
| Table mapping mode | `mmap` |
| Prism mapping mode | `mmap` |
| Source fallback | `false` |
| Table byte source length | `13013460` |
| Stored entries | `498564` |
| Table heap mirror bytes | `0` |
| Prism heap mirror bytes | `0` |
| rsmarisa status | `ok` |
| rsmarisa mapping mode | `mmap` |
| rsmarisa tries / keys | `3` / `463586` |
| Sample key | `a` |

The source dictionary checksum (`0x16ad0e3e`) does not match the deployed table
checksum (`0xb967cfef`) because Yune's current source checksum covers the local
`luna_pinyin.dict.yaml`, while the upstream deployed table checksum includes
imported dictionary material. M38 accepts this only for the upstream
`luna_pinyin` marisa table path and records the status as
`accepted_upstream_marisa_import_checksum`.
