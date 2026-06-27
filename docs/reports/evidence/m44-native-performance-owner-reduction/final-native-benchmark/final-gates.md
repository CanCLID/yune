# M44 Final Gates

Status: passed with measured `ni` and Track A memory blockers

M44 closes as a partial native/profile performance reduction. It does not claim
full performance success, browser/frontend speed, packaging/deployment speed,
public-demo speed, or broad product-delivery speed.

## Final Native Rows

Same-run Track A oracle: upstream `rime/librime 1.17.0` with `luna_pinyin`.

| Row | Yune median | librime median | Ratio / result |
| --- | ---: | ---: | --- |
| startup/runtime-ready | `24,367.100 us` | `30,845.200 us` | `0.790x`; guard pass |
| session create/select/destroy | `23,431.200 us` | `28,775.300 us` | `0.814x`; guard pass |
| `hao` | `24.700 us` | `11.633 us` | `2.123x`; target pass |
| `ni` | `49.450 us` | `14.400 us` | `3.434x`; target miss |
| `zhongguo` | `60.338 us` | `170.812 us` | `0.353x`; guard pass |
| `ceshiyixiachangjushuruxingnengzenyang` | `291.546 us` | `294.995 us` | `0.988x`; guard pass |
| `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` | `502.766 us` | `680.368 us` | `0.739x`; guard pass |
| `cszysmsrsd` | `545.020 us` | `1,224.850 us` | `0.445x`; target pass |
| `zybfshmsru` | `540.970 us` | `853.120 us` | `0.634x`; target pass |

Track A peak working set is `127,619,072 B`, above the M44 memory target
`<=107,797,708 B` and still above the historical M42 `+5%` ceiling
`125,763,994 B`. Memory remains a measured blocker.

## Track B Product-Profile Rows

| Row | Phase 0 median | Final median | Improvement | Final exact lookups/key |
| --- | ---: | ---: | ---: | ---: |
| `h` | `21,832.900 us` | `1,669.600 us` | `92.4%` | `1` |
| `ha` | `11,328.800 us` | `1,141.650 us` | `89.9%` | `2` |
| `hai` | `7,550.433 us` | `763.933 us` | `89.9%` | `3` |
| `hau` | `7,617.567 us` | `774.600 us` | `89.8%` | `3` |
| `nei` | `3,572.400 us` | `398.933 us` | `88.8%` | `3` |
| `ngo` | `3,752.900 us` | `573.333 us` | `84.7%` | `3` |
| 50+ guard | `189.026 us` | `32.462 us` | `82.8%` | `119` |

Track B deployed storage remains `source_fallback=false`.

## Behavior And Storage Gates

- Candidate-output comparison for abbreviation rows `cszysmsrsd` and
  `zybfshmsru`: pass for text, comments, order, preedit, commit preview, and
  first-page metadata. This artifact is scoped to abbreviation rows; it is not
  a standalone oracle-vs-Yune candidate dump for `hao`, `ni`, or Track B short
  rows.
- `hao` and `ni`: `upstream_sentence_model_calls=0`.
- Both M40 full-pinyin long rows: all abbreviation-only counters are `0`, and
  short-key counters are `0`.
- Track A storage: `selected_storage=rsmarisa_byte_backed`, table/prism
  mapping mode `mmap`, table/prism heap mirrors `0`, `source_fallback=false`,
  `rsmarisa_status=ok`, and positive `rsmarisa` counters.
- Track B storage: `selected_storage=byte_backed`, table/prism mapping mode
  `mmap`, heap mirrors `0`, and `source_fallback=false`.
- First-page output and `RimeGetContext` stay page-bounded.

## Review Follow-Up

Two read-only review passes were run before final closeout.

- Requirement-compliance review found stale plan/ledger/final-gate state and an
  ambiguous long-row short-key counter. The plan was moved to completed, the
  ledger and final-gates record were added, and the long-row short-key counter
  issue was fixed and tested.
- Code-quality review found that abbreviation graph-build timing was recorded
  on the normal full-pinyin path, that abbreviation ranking needed bounded
  overfetch before de-duplication, that short-key refresh needed a filter
  cushion, and that Track B pruning was too broad. The metric path, ranking
  bound, short-key refresh, and Track B prefix gate were tightened and covered
  by focused tests.
- External M44 review follow-up found three extra evidence gaps. The M44 visual
  concern was stale: checked-in M44 SVGs are linked from the reports. The real
  short-key risk was fixed by falling back to a complete refresh when permitted
  filters under-fill an incomplete bounded first page. Track B bounded pruning
  now has a focused bounded-vs-full output preservation test for `nei` and
  `ngo`. The requirements traceability table now marks M44 complete with
  measured `ni` and memory blockers instead of planned.

## Evidence Artifacts

- Phase 0 benchmark:
  `docs/reports/evidence/m44-native-performance-owner-reduction/phase-0-native-benchmark/`
- Final benchmark:
  `docs/reports/evidence/m44-native-performance-owner-reduction/final-native-benchmark/`
- Final metrics:
  `docs/reports/evidence/m44-native-performance-owner-reduction/final-native-benchmark/m37_metrics.csv`
- Storage/status:
  `docs/reports/evidence/m44-native-performance-owner-reduction/final-native-benchmark/product_path_status.csv`
- Memory owner profile:
  `docs/reports/evidence/m44-native-performance-owner-reduction/final-native-benchmark/memory-owner-profile.csv`
- Candidate output for abbreviation rows:
  `docs/reports/evidence/m44-native-performance-owner-reduction/final-native-benchmark/oracle-vs-yune-candidate-output.md`
- Visual evidence:
  `docs/reports/evidence/m44-native-performance-owner-reduction/visuals/`

## Required Commands

| Command | Result |
| --- | --- |
| `cargo fmt --check` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo test --workspace` | Pass |
| `git diff --check` | Pass |

Focused checks also run:

- `cargo test -p yune-rime-api m37_metrics_exports_snapshot_json_for_loaded_benchmarks`
- `cargo test -p yune-core bounded_compact_translator_uses_prism_abbreviation_spans_for_sentence_model`
- `cargo test -p yune-core long_luna_rows_do_not_record_m44_short_key_metrics`
- `cargo test -p yune-core short_luna_key_refresh_uses_first_page_bound_and_completes_on_page_turn`
- `cargo test -p yune-core short_luna_key_refresh_falls_back_when_filter_surplus_underfills_first_page`
- `cargo test -p yune-core bounded_typeduck_profile_request_records_m44_track_b_owner_metrics`
- `cargo test -p yune-core bounded_typeduck_short_prefix_pruning_matches_full_translation_for_target_rows`

Post-review follow-up verification for the added guard and documentation
corrections:

| Command | Result |
| --- | --- |
| `cargo fmt --check` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo test -p yune-core` | Pass |
| `cargo test -p yune-rime-api m37_metrics_exports_snapshot_json_for_loaded_benchmarks` | Pass |
| `cargo test -p yune-rime-api --test yune_web` | Pass; 31 tests passed in about 702 seconds after the bounded fallback was narrowed to the Track A `luna_pinyin` short-key path. |
| `cargo test --workspace` | Pass; includes the slow real-asset `yune_web` integration suite. |
| `git diff --check` | Pass |
