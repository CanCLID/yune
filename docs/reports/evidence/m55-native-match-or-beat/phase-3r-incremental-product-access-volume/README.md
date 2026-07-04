# M55 Phase 3R Incremental Product Access Volume

Date: 2026-07-04

Verdict: diagnostic only. This is not a green performance checkpoint and does
not replace the current best committed ratchet:
`docs/reports/evidence/m55-native-match-or-beat/phase-3r-incremental-prefix-state/`.

## Why This Capture Exists

The earlier Phase 3R access-volume capture used
`UpstreamSentenceModel::candidates_for_input`, which measures the full-input
model API. The native product path uses incremental engine state through
`Engine::process_key_sequence` and `UpstreamSentenceModel::candidates_for_input_with_limit_and_scratch`.
That means the earlier diagnostic missed the retained scratch path used by
actual key-by-key input.

This capture drives the product path by processing the input prefix with
metrics disabled, enabling and resetting metrics, and then processing only the
final key. The CSV therefore captures the final-keypress work for the 37-char
and 59-char M55 rows on both owned and byte-backed storage.

## Reproduction

From the repository root:

```powershell
$env:YUNE_M55_PHASE3R_INCREMENTAL_VOLUME_CSV = (Join-Path (Resolve-Path '.').Path 'docs\reports\evidence\m55-native-match-or-beat\phase-3r-incremental-product-access-volume\access-volume.csv')
cargo test -p yune-core --test upstream_luna_pinyin_parity capture_phase3r_incremental_product_access_volume_csv -- --ignored --nocapture
Remove-Item Env:\YUNE_M55_PHASE3R_INCREMENTAL_VOLUME_CSV
```

Captured output:

- `access-volume.csv`

## Key Rows

| input | storage | model_ns | graph_rebuild_ns | graph_entries_inserted | dp_states_created | dp_beam_evictions |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| 37-char | owned | `10,449,600` | `88,900` | `96` | `5,060` | `1,700` |
| 37-char | byte-backed | `11,386,400` | `209,400` | `96` | `5,060` | `1,700` |
| 59-char | owned | `27,697,500` | `119,700` | `164` | `10,853` | `4,494` |
| 59-char | byte-backed | `27,046,500` | `260,200` | `164` | `10,853` | `4,494` |

## Owner Update

The incremental final-key graph rebuild itself is now small compared with total
sentence-model time: owned graph rebuild is about `0.089 ms` on the 37-char row
and `0.120 ms` on the 59-char row, while total owned model time is about
`10.450 ms` and `27.698 ms`.

This changes the next Phase 3R-2 target from graph construction volume to the
candidate extraction path over retained incremental states. Byte-backed storage
still adds graph-rebuild cost, but that cost is not the dominant owner at this
checkpoint.

## Rejected Follow-Up Probe

A local final-end-only candidate-emission probe was rejected and removed. It
improved the 59-char quick probe to `1.557x` but worsened the 37-char row to
`2.055x` and also worsened `n`/`ni`/`hao`. The current best remains
`phase-3r-incremental-prefix-state`.
