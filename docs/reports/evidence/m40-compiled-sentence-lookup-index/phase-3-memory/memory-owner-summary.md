# M40 Memory And Startup Owner Summary

Date: 2026-06-26

Scope: native engine only. This evidence does not claim browser, frontend,
product-delivery, packaging, or public-demo speed wins.

## Evidence Inputs

- Final native benchmark:
  [`../phase-4-final-native/`](../phase-4-final-native/)
- Baseline native benchmark:
  [`../phase-0-baseline/`](../phase-0-baseline/)
- Final storage status:
  [`../phase-4-final-native/product_path_status.csv`](../phase-4-final-native/product_path_status.csv)
- Final startup/session trace:
  [`../phase-4-final-native/startup_session_trace.csv`](../phase-4-final-native/startup_session_trace.csv)
- Final M37/M40 counters:
  [`../phase-4-final-native/m37_metrics.csv`](../phase-4-final-native/m37_metrics.csv)

## Track A Peak Guard

| Row | M39 guard | M40 final | Verdict |
| --- | ---: | ---: | --- |
| Track A max peak working set | `123,985,920 B` | `123,957,248 B` | Pass; final peak is slightly lower than M39 and below the `130,185,216 B` 5% guard. |
| 37-character row median working set | M40 baseline `112,103,424 B` | `114,704,384 B` | Accept; row working set moved up but peak stayed below the guard. |
| 59-character row median working set | M40 baseline `113,012,736 B` | `115,441,664 B` | Accept; row working set moved up but peak stayed below the guard. |

The final Track A peak is `123,957,248 B`, while the final same-run librime
Track A peaks are `13,971,456-17,358,848 B` depending row. M40 does not claim
memory parity with librime; it preserves the M39 no-regression guard while
adding a compact sentence lookup index.

## Sentence Index Retained Heap

| Owner | Retained shape | Evidence / bound | Verdict |
| --- | --- | --- | --- |
| `SentenceLookupIndex` exact ranges | `Box<[SentenceCodeRange]>`, one `u32 start` and one `u32 end` per distinct sentence-model code range | Upper bound from final `stored_entries=498,564`: `498,564 * 8 = 3,988,512 B`, before allocator overhead. Actual range count is lower when codes share entries. | Accept; bounded numeric ranges only. |
| Phrase/prefix index | Same sorted range index, narrowed by prefix lower bounds during `walk_from`; no persistent trie nodes | No cloned code/text/comment strings and no `HashSet<String>`/`HashMap<String, ...>` mirror. | Accept; borrowed/indexed over `entries_by_code`. |
| Existing model entries | Existing `entries_by_code: Vec<ModelEntry>` | M40 did not duplicate entry text/code payloads into the index. | Unchanged owner. |
| Character/vocabulary helpers | Existing `character_codes` and `vocabulary_first_codes` | M40 did not widen these helpers; they remain vocabulary/character support rather than sentence-index storage. | Unchanged owner. |
| Selected table/prism bytes | `mmap` selected artifacts | Final status reports table/prism mapping mode `mmap`, table heap mirror bytes `0`, and prism heap mirror bytes `0`. | Preserved. |

The implemented index is compact and schema-owned: it is built from the sorted
sentence-model entries during translator construction and stores numeric ranges
into that existing vector. It deliberately avoids an eager heap mirror of the
dictionary as cloned strings, `HashSet<String>` prefix membership, or a separate
large trie.

## Startup And First-Use Accounting

| Signal | Final value | Verdict |
| --- | ---: | --- |
| Startup/runtime-ready median | `23,934.200 us` | Pass; faster than M39 `25,048.200 us` and `0.913x` same-run librime. |
| Session create/select/destroy median | `23,994.000 us` | Pass; faster than M39 `25,255.500 us` and `0.934x` same-run librime. |
| Cold startup sentence-index build | `1` call, `5,144,800 ns` on startup sample `0` | Accounted in `phase-4-final-native/m37_metrics.csv`; warm startup/session rows are `0`. |
| Per-key `upstream_sentence_model_index_build_calls` | `0` in final key metrics | Pass; index construction is not hidden inside the measured key rows. |
| Startup trace owner | `translator_index_build` / `translator_install` events in `startup_session_trace.csv` | Accounted as startup/translator install work, then guarded by startup/session medians. |

The benchmark now records M37/M40 metrics for startup/session rows as well as
key rows. The cold index build appears in the first startup sample, warm reuse
appears as zero index-build calls, and key rows remain free of index-build
cost. Startup and session medians improved from M39.

## Storage Guard

Final `luna_pinyin` storage status:

- `selected_storage=rsmarisa_byte_backed`
- `table_mapping_mode=mmap`
- `prism_mapping_mode=mmap`
- `source_fallback=false`
- `table_heap_mirror_bytes=0`
- `prism_heap_mirror_bytes=0`
- `rsmarisa_status=ok`
- `rsmarisa_mapping_mode=mmap`
- `rsmarisa_num_keys=463586`

Final raw lookup rows also report positive `rsmarisa` exact and prefix counters
on every Track A target row.

## Verdict

M40 preserves the M39 memory/storage hot path while reducing the long-row
sentence lookup owner. The sentence index retains compact numeric ranges into
existing model entries, not cloned table strings or a separate dictionary heap
mirror. Whole-process memory remains a future engine gap, but it is not an M40
regression.
