# M40 Final Gates

Date: 2026-06-26

Scope: native engine only. M40 does not optimize or claim `yune-web`, frontend,
product delivery, packaging, browser startup, browser typing, or public-demo
speed.

## Evidence

- Phase 0 baseline:
  [`phase-0-baseline/`](./phase-0-baseline/)
- Final native benchmark:
  [`phase-4-final-native/`](./phase-4-final-native/)
- Memory/startup summary:
  [`phase-3-memory/memory-owner-summary.md`](./phase-3-memory/memory-owner-summary.md)
- Final benchmark command:
  [`phase-4-final-native/commands.txt`](./phase-4-final-native/commands.txt)
- Final storage status:
  [`phase-4-final-native/product_path_status.csv`](./phase-4-final-native/product_path_status.csv)
- Final M40 counters:
  [`phase-4-final-native/m37_metrics.csv`](./phase-4-final-native/m37_metrics.csv)

## Final Native Rows

| Row | Yune median | librime median | Ratio / guard | Verdict |
| --- | ---: | ---: | ---: | --- |
| startup/runtime-ready | `23,934.200 us` | `26,218.400 us` | `0.913x` | Pass |
| session create/select/destroy | `23,994.000 us` | `25,700.000 us` | `0.934x` | Pass |
| `hao` | `38.200 us` | `11.800 us` | `3.237x` | Pass |
| `ni` | `56.850 us` | `14.700 us` | `3.867x` | Pass |
| `zhongguo` | `60.275 us` | `186.400 us` | `0.323x` | Pass |
| `ceshiyixiachangjushuruxingnengzenyang` | `289.914 us` | `295.800 us` | `0.980x` | Pass |
| `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` | `494.017 us` | `694.175 us` | `0.712x` | Pass |
| `cszysmsrsd` | `24.820 us` | `1,237.820 us` | N/A: `0` exported candidates | Behavior probe included; oracle output parity unverified. |
| `zybfshmsru` | `26.350 us` | `866.720 us` | N/A: `0` exported candidates | Behavior probe included; oracle output parity unverified. |
| Track B `neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung` | `196.387 us/op` | N/A | p95 `605.125 us/op` | Guard included; median is +`4.0%` vs M39 `188.857 us/op`; p95 has two measured Windows scheduling outliers and is recorded as a caveat. |

Both Track A long rows improved from M39 and are within `1.25x` of same-run
upstream librime:

- 37-character row: M39 `514.903 us` -> M40 `289.914 us`.
- 59-character row: M39 `917.961 us` -> M40 `494.017 us`.

## Four-Strategy Gate

Per-key counter medians are calculated from
[`phase-4-final-native/m37_metrics.csv`](./phase-4-final-native/m37_metrics.csv).

| Strategy | 37-character row | 59-character row | Verdict |
| --- | ---: | ---: | --- |
| A exact range index | `22.189` hits/key, `28.973` misses/key | `31.186` hits/key, `41.119` misses/key | Active |
| B reachable-vertex pruning | `9.595` reachable starts/key, `7.919` unreachable skips/key | `16.017` reachable starts/key, `13.508` unreachable skips/key | Active |
| C prefix filtering | `51.162` hits/key, `7.486` misses/key, `7.486` early breaks/key | `72.305` hits/key, `13.729` misses/key, `13.729` early breaks/key | Active |
| D phrase-index walk | `9.595` walks/key, `51.162` nodes/key, `22.189` emitted ranges/key | `16.017` walks/key, `72.305` nodes/key, `31.186` emitted ranges/key | Active |
| Old partition fallback | `0.000` calls/key | `0.000` calls/key | Removed from hot path |

## Owner Movement

| Row | Baseline prefix checks/key | Final prefix checks/key | Reduction | Baseline table entries/key | Final table entries/key | Reduction |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 37-character Track A row | `241.054` | `58.649` | `75.7%` | `3,564.216` | `111.486` | `96.9%` |
| 59-character Track A row | `608.576` | `86.034` | `85.9%` | `6,344.559` | `186.831` | `97.1%` |

The final owner counters exceed the `40%` reduction gate for the 59-character
row. The remaining long-row owner is still the sentence model/translator path,
but it is below same-run librime for both required long rows.

## M40-ENGINE-12 Incrementality Verdict

Cross-keystroke graph rebuild is measured and is not the top remaining long-row
owner after A/B/C/D:

| Row | Upstream sentence model | Graph rebuild | Translator median | Verdict |
| --- | ---: | ---: | ---: | --- |
| 37-character Track A row | `222.072 us/key` | `17.303 us/key` | `286.276 us` | Rebuild is not the top owner. |
| 59-character Track A row | `399.215 us/key` | `31.014 us/key` | `490.203 us` | Rebuild is not the top owner. |

No bounded incrementality path is implemented in M40 because the measured graph
rebuild cost is not the top remaining long-row owner. The final counters record
`incremental_reuse_hits=0`, `incremental_extend_ns=0`, and discarded rebuild
characters for each typed prefix.

## Storage, Memory, And Bounded Output

Final Track A storage:

- `selected_storage=rsmarisa_byte_backed`
- `table_mapping_mode=mmap`
- `prism_mapping_mode=mmap`
- `source_fallback=false`
- `table_heap_mirror_bytes=0`
- `prism_heap_mirror_bytes=0`
- `rsmarisa_status=ok`
- `rsmarisa_mapping_mode=mmap`
- `rsmarisa_num_keys=463586`
- positive runtime `rsmarisa` exact and prefix counters on all target rows

Final Track A peak working set is `123,957,248 B`, below the M39 final
`123,985,920 B` and below the 5% guard `130,185,216 B`.

Bounded output/context remains active in final raw lookup rows. The two long
Track A rows export only `0.135` and `0.085` context-page candidates per op on
average, with no full-list fallback becoming the owner.

## Quality Gates

Required final gates passed after the final native benchmark and documentation
updates:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `git diff --check`

## Verdict

M40 passes the native-engine closeout gates: all four sentence lookup
strategies are active, both long Track A rows are within same-run librime,
startup/session and short/medium rows remain guarded, mmap/`rsmarisa` selected
storage is preserved, memory does not regress, incomplete pinyin rows are
included as boundedness/behavior probes, the Track B 50+ guard row is
included, and cross-keystroke graph rebuild is measured as not the top
remaining long-row owner.

The incomplete-pinyin rows are not counted as same-output performance wins:
both rows export `0` ABI candidates in Yune. A future oracle-output check must
decide whether they should produce upstream `luna_pinyin` abbreviation
candidates or remain empty.
