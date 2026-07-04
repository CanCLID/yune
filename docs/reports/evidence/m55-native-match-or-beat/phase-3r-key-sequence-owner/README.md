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

- `../phase-3r-sentence-path-cache/m55-ratchet/track-a-yune/m37_metrics.csv`
- `../phase-3r-sentence-path-cache/m55-ratchet/summary-comparison.csv`

No new benchmark run was used for this diagnostic. Values in
`owner-summary.csv` are medians of each per-sample counter divided by
`operation_count`, grouped by input.

Reproduction shape:

```powershell
$rows = Import-Csv docs\reports\evidence\m55-native-match-or-beat\phase-3r-sentence-path-cache\m55-ratchet\track-a-yune\m37_metrics.csv |
  Where-Object { $_.workload -eq 'key_sequence_process_with_context' }
# For each input, group rows and compute median(counter / operation_count).
```

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
