# M55 Phase 3R Key-Sequence Owner Correction

Date: 2026-07-04

Verdict: diagnostic only. This replaces the next-owner conclusion from the
final-key-only access-volume follow-up; it does not replace the current best
green checkpoint:
`docs/reports/evidence/m55-native-match-or-beat/phase-3r-sentence-path-cache/`.

## Why This Capture Exists

The previous incremental product-path diagnostic measured only the final
keypress after feeding the prefix with metrics disabled. That answered what the
last key costs after the scratch has been warmed, but the strict M55 benchmark
measures the whole key sequence. The whole sequence still spends most of its
time in cumulative upstream sentence graph rebuild/extension work across all
prefixes.

## Source

This is derived from the committed strict ratchet metrics:

- Raw per-key counters: archived by exact Git blob in the
  [evidence-pruning ledger](../../../../ledgers/evidence-pruning/current-ledger.csv)
- `../phase-3r-sentence-path-cache/m55-ratchet/summary-comparison.csv`

No new benchmark run was used for this diagnostic. Values in
`owner-summary.csv` are medians of each per-sample counter divided by
`operation_count`, grouped by input.

To recompute the owner table, recover the archived raw leaf using the generic
`git show` recipe in the evidence-pruning ledger README, select workload
`key_sequence_process_with_context`, then group by input and compute the median
of each counter divided by `operation_count`.

## Key Rows

| input | process key us/op | translator us/op | upstream model us/op | graph rebuild us/op | candidate path us/op | candidate merge us/op |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 37-char | `575.724` | `572.234` | `524.012` | `515.030` | `0.000` | `0.000` |
| 59-char | `991.538` | `987.667` | `925.330` | `911.192` | `0.000` | `0.000` |
| `n` | `53.200` | `51.200` | `0.000` | `0.000` | `0.000` | `0.000` |
| `ni` | `40.800` | `38.625` | `0.000` | `0.000` | `0.000` | `0.000` |
| `hao` | `22.517` | `20.267` | `0.000` | `0.000` | `0.000` | `0.000` |

The release benchmark does not populate the debug-only candidate extraction
subcounters, so `candidate_path_us_per_op` and `candidate_merge_us_per_op` are
zero here. The release-visible owner is still clear: on the long rows,
`upstream_sentence_model_graph_rebuild_ns` accounts for nearly all
`upstream_sentence_model_ns`, and upstream sentence model time accounts for most
translator time.

## Decision

The next Phase 3R-2 implementation target is cumulative per-prefix graph
extension/rebuild work across the full key sequence. The final-key-only
diagnostic remains useful for warmed-scratch behavior, but it should not drive
the next optimization by itself.

Rejected candidate-extraction probes after the sentence-path cache support this
correction: simple candidate/key clone avoidance and a retained merged candidate
map both preserved product-path output but failed to replace the current best.

## Per-Key Fixture Diagnostic

`per-key-volume.csv` is a follow-up fixture-dictionary diagnostic captured from
the same M55 37-char and 59-char inputs, one row per keypress, for the owned and
byte-backed sentence engines. It is not a replacement for the release ratchet
above: the CSV uses the expanded oracle fixture dictionary, not the deployed
full dictionary. Its purpose is to show how cumulative model work is distributed
across the incremental key sequence after the sentence-path cache.

Capture command:

```powershell
$env:YUNE_M55_PHASE3R_KEY_SEQUENCE_VOLUME_CSV = (Join-Path (Resolve-Path '.').Path 'docs\reports\evidence\m55-native-match-or-beat\phase-3r-key-sequence-owner\per-key-volume.csv')
cargo test -p yune-core --test upstream_luna_pinyin_parity capture_phase3r_incremental_key_sequence_volume_csv -- --ignored --nocapture
Remove-Item Env:\YUNE_M55_PHASE3R_KEY_SEQUENCE_VOLUME_CSV
```

Aggregate fixture totals:

| input | storage | rows | model ns | graph rebuild ns | incremental extend ns | index probes | graph entries | DP states | beam evictions | candidate merge ns |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 37-char | owned | 37 | `1,727,900` | `537,200` | `465,500` | `260` | `96` | `319` | `130` | `941,900` |
| 37-char | byte-backed | 37 | `6,499,100` | `3,220,500` | `0` | `923` | `1,789` | `4,967` | `1,691` | `0` |
| 59-char | owned | 59 | `4,914,500` | `1,258,900` | `1,233,200` | `788` | `164` | `609` | `269` | `3,278,800` |
| 59-char | byte-backed | 59 | `19,614,800` | `7,906,000` | `0` | `2,670` | `5,387` | `17,661` | `6,754` | `0` |

The owned path's fixture totals confirm that the sentence-path cache is taking
effect: it inserts far fewer graph entries and DP states than the byte-backed
builder. The remaining product-path problem is therefore not a single cold final
keypress. The sequence still pays repeated per-prefix upstream sentence work
while the user types, and the authoritative release counters above show that the
full-dictionary product path is still dominated by cumulative graph rebuild time.

The largest owned fixture rows are late prefixes in the 59-char input, especially
prefixes ending in `...caineng`, `...caine`, `...cainen`, and the final
`...cainengyong`. These are the rows to inspect first when the next optimization
tries to reduce cumulative per-prefix graph extension cost without changing
candidate order, scoring, beam behavior, or ABI.
