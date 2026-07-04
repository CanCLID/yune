# M55 Phase 3R Incremental Product Access Volume

Date: 2026-07-04

Verdict: diagnostic only. This is not a green performance checkpoint and does
not replace the current best committed ratchet:
`docs/reports/evidence/m55-native-match-or-beat/phase-3r-sentence-path-cache/`.

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

The harness explicitly sets `schema_id: luna_pinyin` and `enable_sentence:
true`, matching the native Luna bounded-refresh product path. Without those
settings the core helper either falls back to full-input model capture or skips
the upstream sentence model under bounded refresh, which is not the M55 path.

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

| input | storage | model_ns | graph_rebuild_ns | candidate_state_buckets | candidate_states_ranked | candidate_path_ns | candidate_merge_ns | graph_entries_inserted | dp_states_created | dp_beam_evictions |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 37-char | owned | `105,000` | `33,200` | `14` | `188` | `2,400` | `62,900` | `7` | `27` | `12` |
| 37-char | byte-backed | `359,600` | `155,200` | `0` | `0` | `0` | `0` | `96` | `319` | `130` |
| 59-char | owned | `198,200` | `59,900` | `24` | `338` | `3,800` | `127,300` | `7` | `35` | `20` |
| 59-char | byte-backed | `641,400` | `238,700` | `0` | `0` | `0` | `0` | `164` | `609` | `269` |

## Owner Update

The owned incremental final-key graph rebuild itself is now small compared with
total sentence-model time: owned graph rebuild is about `0.033 ms` on the
37-char row and `0.060 ms` on the 59-char row, while total owned model time is
about `0.105 ms` and `0.198 ms`. Candidate path conversion plus merge/rank is
about `0.065 ms` on the 37-char row and `0.131 ms` on the 59-char row.

The landed sentence-path cache moved path conversion out of the final-key owner:
path conversion fell from `27,900 ns` / `57,800 ns` in the previous capture to
`2,400 ns` / `3,800 ns`. On the final key alone, the remaining measured owned
extractor cost is candidate merge/rank over retained incremental states.

This final-key-only owner is not the whole strict benchmark owner. The follow-up
key-sequence owner correction in
`../phase-3r-key-sequence-owner/` shows that cumulative graph rebuild/extension
still dominates the full long-row key sequence. Byte-backed storage does not
use the owned incremental scratch path today, which is why its candidate
extraction counters are zero and its graph/DP counts remain much higher.

## Rejected Follow-Up Probe

A local final-end-only candidate-emission probe was rejected and removed. It
improved the 59-char quick probe to `1.557x` but worsened the 37-char row to
`2.055x` and also worsened `n`/`ni`/`hao`. The current best remains
`phase-3r-sentence-path-cache`.

A local sentence-end bucket-only merge probe was also rejected and removed. It
improved the 59-char quick probe to `1.552x` but worsened the 37-char row to
`2.032x`.
