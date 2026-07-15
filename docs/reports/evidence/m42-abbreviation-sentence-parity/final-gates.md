# M42 Final Gates

Date: 2026-06-26

Scope: native engine only. No TypeDuck Web, `typeduck.hk/web`, `yune-web`,
browser behavior, product behavior, or M16 rows are used as the M42 oracle.

## Phase 0 Decision

Phase 0 selected the implementation branch. Native upstream
`rime/librime 1.17.0` with `luna_pinyin` exported meaningful candidates for
both rows:

- `cszysmsrsd`
- `zybfshmsru`

Evidence:

- `phase-0-oracle/librime-1.17.0-candidate-output.json`
- `phase-0-oracle/yune-candidate-output.json`

## Candidate Output

Final native Yune output matches upstream first-page candidate text, comments,
order, context preedit, commit preview, page number, page size, and
`is_last_page` for both M42 rows.

Known metadata caveat: `RimeGetInput` remains Yune's raw keystroke buffer while
context preedit carries the segmented display string. This is not counted as a
candidate-output mismatch.

Evidence:

- `final-candidate-comparison/oracle-vs-yune-candidate-output.json`
- `final-candidate-comparison/oracle-vs-yune-candidate-output.md`

## Final Native Benchmark Rows

Same-run Track A native comparison:

| Row | Yune median | librime median | Ratio | Result |
| --- | ---: | ---: | ---: | --- |
| startup/runtime-ready | `23,856.300 us` | `31,421.900 us` | `0.759x` | Pass |
| session create/select/destroy | `23,776.500 us` | `27,766.600 us` | `0.856x` | Pass |
| `hao` | `38.800 us` | `11.333 us` | `3.424x` | Pass, under `5x` short-key guard |
| `ni` | `57.150 us` | `14.000 us` | `4.082x` | Pass, under `5x` short-key guard |
| `zhongguo` | `60.188 us` | `166.025 us` | `0.363x` | Pass |
| `ceshiyixiachangjushuruxingnengzenyang` | `278.438 us` | `290.873 us` | `0.957x` | Pass |
| `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` | `474.683 us` | `658.592 us` | `0.721x` | Pass |
| `cszysmsrsd` | `4,127.580 us` | `1,189.890 us` | `3.469x` | Behavior pass; latency blocker |
| `zybfshmsru` | `4,257.100 us` | `839.860 us` | `5.069x` | Behavior pass; latency blocker |

Track B guard:

| Row | Yune median | Yune p95 | Result |
| --- | ---: | ---: | --- |
| `neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung` | `186.513 us/op` | `204.680 us/op` | Guard only; no TypeDuck-profile speed claim |

Evidence:

- `final-native-benchmark/summary-comparison.csv`
- `final-native-benchmark/track-a-yune/summary.csv`
- `final-native-benchmark/track-a-librime-1.17.0/summary.csv`
- `final-native-benchmark/track-b-yune-guard/summary.csv`

## Owner Evidence

Short-key owner profile before optimization:

- `ni`: `56.25 us/op` process-key owner, `53.0 us/op` translator owner.
- `hao`: `38.2 us/op` process-key owner, `34.7 us/op` translator owner.

No short-key optimization was attempted in M42.

Abbreviation owner profile:

- `cszysmsrsd`: `4,126.81 us/op` process-key owner,
  `4,118.6 us/op` translator owner, `2,278.911 us/call` upstream sentence
  model owner, `1.8` model calls/op.
- `zybfshmsru`: `4,256.38 us/op` process-key owner,
  `4,246.34 us/op` translator owner, `2,348.833 us/call` upstream sentence
  model owner, `1.8` model calls/op.

Evidence:

- `final-native-benchmark/short-key-owner-profile.csv`
- `final-native-benchmark/abbreviation-owner-profile.csv`

## Storage And Memory

Track A storage/status:

- `selected_storage=rsmarisa_byte_backed`
- table/prism mapping mode: `mmap`
- selected table/prism heap mirror bytes: `0`
- `source_fallback=false`
- `rsmarisa_status=ok`
- `rsmarisa_mapping_mode=mmap`
- positive `rsmarisa` exact/prefix counters on target rows

Memory:

- Track A max peak working set: `119,775,232 B`
- M40 Track A peak baseline: `123,957,248 B`
- M40 5% guard max: `130,155,110 B`
- Track A 37-character working set: `113,610,752 B`
- Track A 59-character working set: `114,339,840 B`
- Track B guard median working set: `442,007,552 B`
- Track B guard max peak working set: `504,901,632 B`

Evidence:

- `final-native-benchmark/product_path_status.csv`
- Raw per-key counters and startup trace: archived by exact Git blob in the
  [evidence-pruning ledger](../../../ledgers/evidence-pruning/current-ledger.csv)
- `final-native-benchmark/track-a-yune/summary.csv`
- `final-native-benchmark/track-b-yune-guard/summary.csv`

## Final Quality Gates

Final commands:

- `cargo fmt --check` - pass
- `cargo clippy --workspace --all-targets -- -D warnings` - pass
- `cargo test -p yune-core abbreviation` - pass
- `cargo test -p yune-core --test upstream_luna_pinyin_parity` - pass
- `cargo test --workspace` - pass
- `git diff --check` - pass
- `cargo build --release -p yune-rime-api` - pass

During final verification, the first `cargo test --workspace` run exposed a
source-fixture regression in the normal full-pinyin sentence vocabulary path:
`zhongguo` returned no sentence candidates. Root cause was that the new
zero-weight character-code filter was applied to the shared character-code map;
source fixtures use missing weights as `0.0`. The final implementation splits
normal sentence character codes from abbreviation-only character codes, keeping
normal source/full-pinyin behavior intact while preserving the abbreviation
filter. The final workspace test rerun passes.

## Review Passes

Spec/requirement compliance:

- Phase 0 branch respected.
- M42 oracle is native upstream `rime/librime 1.17.0` with `luna_pinyin`.
- Candidate-output artifact exists for both M42 rows.
- M42 closes as behavior parity with a measured latency blocker, not as a speed
  win.
- No browser, product, TypeDuck Web, `typeduck.hk/web`, yune-web, or M16 oracle
  claim is made.

Code quality, ABI safety, and coverage:

- No `RimeApi` field order or default ABI widening.
- Abbreviation expansion is behind a separate code-span branch.
- Full-pinyin long rows remain on the M40 sentence lookup path.
- Abbreviation phrase vocabulary is target-scoped for the compiled M42 path.
- Focused poet/translator tests cover code-span abbreviation routing, ranking,
  zero-weight abbreviation filtering, and full-pinyin guard behavior.
- Full workspace tests pass after the shared source-vocabulary regression fix.
