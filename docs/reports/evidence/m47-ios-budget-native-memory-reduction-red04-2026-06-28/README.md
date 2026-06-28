# M47 RED-04 Reverse/UI Optional Pack Evidence

Date: 2026-06-28

Harness: Windows native `RimeApi` lean memory probe for isolated `jyut6ping3_mobile`, with `YUNE_MEM_DEFAULT=jyut6ping3_mobile`, `YUNE_MEM_DISABLE_DICTIONARY_LOOKUP_RECORDS=1`, and `YUNE_MEM_DISABLE_COMPACT_LOOKUP_RECORDS=1`. The current run also sets `YUNE_MEM_DISABLE_LUNA_REVERSE_TRANSLATOR=1`, which patches only the temporary deployed schema to add `luna_pinyin/load_translator: false`. Values are Windows `WorkingSet64` / `PrivateUsage` proxy evidence, not iOS `phys_footprint`.

## Verdict

RED-04 succeeded as an optional keyboard-extension pack gate for the reverse/UI translator. A translator namespace can now set `load_translator: false` to skip installing that translator before any dictionary load. Default/public behavior remains eager; this opt-out is not behavior-preserving for the grave-prefix Luna reverse lookup path.

Steady memory moved from 69.2 MB WS / 29.7 MB private / 20.4 MB allocator-live to 58.5 MB WS / 23.3 MB private / 16.0 MB allocator-live. Peak working set did not improve: 80.7 MB before, 81.0 MB current.

This is still a Windows proxy result. It does not prove iOS keyboard-extension readiness: steady WS remains above the 48 MB target and peak remains above the 64 MB target.

## Owner Movement

| Owner | Before | Current | Verdict |
| --- | ---: | ---: | --- |
| `compact_table.storage` | 19,888,937 B mmap-backed | 15,248,382 B mmap-backed | secondary reverse table mapping omitted |
| `prism.spelling_map` | 11,955,056 B mmap-backed | 10,965,828 B mmap-backed | secondary reverse prism spelling-map bytes omitted |
| `prism.double_array_units` | 8,896,512 B mmap-backed | 8,388,608 B mmap-backed | secondary reverse prism double-array bytes omitted |
| `compact_table.syllabary_codes` | 4,850,892 B heap | 4,189,674 B heap | secondary reverse syllabary code Vec omitted |
| `compact_table.candidate_comment_payload` | 1,861,361 B shared/overlap | 1,516,569 B shared/overlap | secondary reverse comment payload omitted |
| `compact_table.candidate_text_payload` | 1,410,493 B shared/overlap | 1,058,723 B shared/overlap | secondary reverse text payload omitted |

The measured named-owner total dropped by 7,495,635 B. Heap-owned named owners dropped by 661,386 B; mmap/file-backed owner rows dropped by 6,137,687 B. This confirms RED-04 is mostly a clean mapping and small heap-owner reduction after RED-03 had already removed secondary lookup-record heap.

## Behavior Boundary

`load_translator: false` is an optional-pack gate, not a first-use lazy loader. With it enabled for `luna_pinyin`, the grave-prefix Mandarin reverse lookup UI is intentionally absent from that keyboard-profile run. The committed public schema bundle is not edited, and default/eager reverse lookup behavior remains covered by existing schema and yune-web tests.

Behavior-preserving first-use lazy loading remains a possible future improvement, but it needs a dedicated lazy static-table translator wrapper and first-prefix evidence for both latency and peak transient behavior.

## Remaining Blocker

After RED-04, isolated `jyut6ping3_mobile` with RED-01, RED-03, and RED-04 opt-outs is closer but still over budget:

- steady: 58.5 MB WS, 23.3 MB private, 16.0 MB allocator-live
- peak: 81.0 MB WS
- top named retained heap: `compact_table.syllabary_codes`, 4,189,674 B
- top mapped owners: primary compact table storage and primary prism byte-backed ranges

Recommended next branch: M47-RED-05 peak transient investigation. The peak remains around 81 MB even when the steady reverse/UI pack is omitted, so the next blocker is not a retained secondary translator owner.
