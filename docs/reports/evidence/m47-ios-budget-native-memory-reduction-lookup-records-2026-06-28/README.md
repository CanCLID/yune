# M47 RED-03 Compact Lookup-Record Evidence

Date: 2026-06-28

Harness: Windows native `RimeApi` lean memory probe for isolated `jyut6ping3_mobile`, with `YUNE_MEM_DEFAULT=jyut6ping3_mobile` and `YUNE_MEM_DISABLE_DICTIONARY_LOOKUP_RECORDS=1`. The current run also sets `YUNE_MEM_DISABLE_COMPACT_LOOKUP_RECORDS=1`, which patches only the temporary deployed schema to add `translator/load_lookup_records: false` and `luna_pinyin/load_lookup_records: false`. Values are Windows `WorkingSet64` / `PrivateUsage` proxy evidence, not iOS `phys_footprint`.

## Verdict

RED-03 succeeded as a native runtime heap reduction for the keyboard-profile measurement. Compact table advanced payload parsing now keeps lookup records by default, but can validate and walk the `YUNE-LOOKUP` payload without retaining the `HashMap<String, Vec<DictionaryLookupRecord>>` when a translator namespace sets `load_lookup_records: false`.

Steady memory moved from 137.8 MB WS / 102.8 MB private / 66.6 MB allocator-live to 69.4 MB WS / 30.2 MB private / 20.4 MB allocator-live. Peak working set moved from 172.1 MB to 80.8 MB.

This is still a Windows proxy result. It does not prove iOS keyboard-extension readiness: steady WS remains above the 48 MB target and peak remains above the 64 MB target.

## Owner Movement

| Owner | Before | Current | Verdict |
| --- | ---: | ---: | --- |
| `compact_table.lookup_records` primary `jyut6ping3_mobile` | 31,920,140 B / 127,144 records | 48 B / 0 records | skipped in keyboard-profile run |
| `compact_table.lookup_records` secondary `luna_pinyin_yune_reverse` | 13,769,158 B / 70,807 records | 48 B / 0 records | skipped in keyboard-profile run |
| `compact_table.lookup_records` total | 45,689,298 B / 197,951 records | 96 B / 0 records | measured RED-03 owner removed |
| `compact_table.syllabary_codes` total | 4,850,892 B | 4,850,892 B | top remaining heap owner |
| `compact_table.storage` total | 19,888,937 B mmap-backed | 19,888,937 B mmap-backed | unchanged raw table mappings |
| `prism.spelling_map` total | 11,955,056 B mmap-backed | 11,955,056 B mmap-backed | unchanged RED-02 byte-backed payload |
| `prism.double_array_units` total | 8,896,512 B mmap-backed | 8,896,512 B mmap-backed | unchanged RED-02 byte-backed payload |

The measured heap-owned named owner total dropped by 45,689,202 B. Mmap/file-backed owner rows stayed flat at 40,740,505 B, confirming this branch removed live retained heap rather than file mappings.

## Path Safety

Default/public behavior remains eager. The new parser option defaults to `load_lookup_records: true`; the committed public schema bundle is not edited. The opt-out is schema-config driven and was applied only to the native probe's temporary deployed schema for this keyboard-profile evidence run.

The skipped records are not required for normal unprefixed Jyutping candidate generation. Candidate text and code/comment payloads remain byte-backed in `compact_table.storage`, and the RED-01 `dictionary_lookup_filter/load_lookup_records: false` path still separately controls dictionary-panel/comment enrichment.

## Remaining Blocker

After RED-03, isolated `jyut6ping3_mobile` with RED-01 and RED-03 opt-outs is close but still over budget:

- steady: 69.4 MB WS, 30.2 MB private, 20.4 MB allocator-live
- peak: 80.8 MB WS
- top named retained heap: `compact_table.syllabary_codes`, 4,850,892 B total
- top mapped owners: compact table storage, prism spelling-map bytes, and prism double-array bytes

Recommended next branch: M47-RED-04 reverse/UI lazy loading, because the `luna_pinyin_yune_reverse` translator and related mapped table/prism payloads are still loaded at `create_session()` for a grave-prefix UI path rather than normal unprefixed Jyutping typing. Keep peak visible; the next branch must not hide the 80.8 MB cold-start peak.
