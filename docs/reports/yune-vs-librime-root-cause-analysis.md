# Current Yune Root-Cause Dashboard

Date: 2026-06-30

This report keeps only the current root-cause read. Older milestone narratives,
WEB-01/WEB-02/WEB-03 closeout detail, and superseded measurements remain in
[`history/2026-06-28-yune-vs-librime-root-cause-analysis-pre-current-dashboard.md`](./history/2026-06-28-yune-vs-librime-root-cause-analysis-pre-current-dashboard.md).

The native lane was refreshed by the M52 final same-run benchmark on
2026-06-30; browser rows are carried forward from the 2026-06-28 Playwright run.

## Technical Summary

- **Current native guardrail owner**: M52 added a committed Track A threshold
  artifact and a fail-on-regression mode to the native in-process benchmark. The
  final gate passes for `n`, `ni`, `hao`, the 37-character row, the existing
  59-character row, and the full-Luna Track A peak memory ceiling.
- **Current native latency disposition**: `n`, `hao`, and the 59-character row
  pass the strict `<=3.0x` same-run librime ratio target. `ni` (`3.143x`,
  `30.650 us` gap) and the 37-character row (`3.053x`, `601.967 us` gap)
  remain above the strict ratio target, but M52 closes them as
  bounded-microsecond ceilings because no owner evidence named a safe reduction
  worth destabilizing the oracle-sensitive translator/poet path.
- **Current native memory disposition**: Track A peak working set is
  `188.4 MB` versus librime's `17.3 MB` max peer peak. The named owners remain
  `poet.vocabulary` (`53.6 MB`) and `poet.entries_by_code` (`18.7 MB`) plus a
  process unclassified lower bound of `105.6 MB`. Native Track A `luna_pinyin`
  is an upstream comparison lane, not the current native TypeDuck product
  profile, so M52 closes the peak as a guardrailed comparison-lane memory watch.
- **Current browser fair memory owner**: the fair `luna_pinyin` browser gap is
  `64.0 MiB` Yune public demo versus `16.0 MiB` My RIME (carried 2026-06-28).
- **Current Jyutping launch state**: the shipping public-demo Jyutping path is
  byte-backed at `160.0 MiB`, not the old `893.1 MiB` source-fallback shape.

## Current Gap Map

| Area | Current root cause | Evidence | Current status |
| --- | --- | --- | --- |
| Native Track A guardrail | Benchmark lacked a committed regression ceiling for the current Luna comparison lane | M52 `track-a-thresholds.csv`; final `threshold-check.csv` all pass | closed by guardrail |
| Native `ni` | Exact-row scan under charset filtering without a retained acceptance index | `44.950 us` vs librime `14.300 us`; `3.143x`; `30.650 us` gap | bounded-microsecond ceiling |
| Native 37-char pinyin | Preset-vocabulary sentence-model graph cost | `895.178 us` vs librime `293.211 us`; `3.053x`; `601.967 us` gap | bounded-microsecond ceiling |
| Native Track A peak memory | Full upstream Luna preset-vocabulary/process residency | Yune peak `188.4 MB`; librime max peer peak `17.3 MB`; M52 ceiling `198.0 MB` | guardrailed comparison-lane watch |
| Browser `luna_pinyin` memory | Yune WASM/runtime floor still larger than My RIME | `64.0 MiB` vs `16.0 MiB`; same schema (carried) | blocker |
| Browser `luna_pinyin` startup | Yune public-demo startup still slower | `1000 ms` vs My RIME `634 ms` (carried) | watch |
| Browser Jyutping | Larger TypeDuck profile; not a peer-comparable lane | Yune `160.0 MiB`, My RIME Jyutping `68.0 MiB` on different dictionary (carried) | guard only |

![Current performance gaps by lane](./evidence/dashboard-visuals-2026-06-30/root-cause-gaps.svg)

## Native Track A Cause

M52 deliberately did not change the translator or poet hot path. The owner
evidence did not name a small parity-safe reduction, and prior M49 evidence
already rejected the retained vocabulary prefix index shape because it added
about `35 MB` of heap.

Current native latency rows:

| Row | Yune median | librime median | Ratio | Current cause |
| --- | ---: | ---: | ---: | --- |
| `n` | `60.300 us` | `21.400 us` | `2.818x` | strict ratio pass |
| `ni` | `44.950 us` | `14.300 us` | `3.143x` | bounded exact-row scan under charset filtering |
| `hao` | `24.967 us` | `11.633 us` | `2.146x` | strict ratio pass |
| 37-char pinyin | `895.178 us` | `293.211 us` | `3.053x` | preset-vocabulary sentence-model graph cost |
| 59-char pinyin | `1,545.754 us` | `687.795 us` | `2.247x` | strict ratio pass |
| `zhongguo` (common word) | `46.150 us` | `166.600 us` | `0.277x` | Yune faster (carried 2026-06-29) |
| `cszysmsrsd` (10-char abbr) | `517.720 us` | `1,218.320 us` | `0.425x` | Yune faster (carried 2026-06-29) |
| `zybfshmsru` (8-char abbr) | `547.580 us` | `859.790 us` | `0.637x` | Yune faster (carried 2026-06-29) |

The slow rows are the short-key exact-row scan and the preset-vocabulary poet
graph (the `37`/`59-character` pinyin sentences). They are real but bounded:
Yune is actually faster than librime on `zhongguo` and the abbreviation rows,
where the compiled index and Rust path win by `120-701 us`. The short-key losses
are `13-39 us`, below any perceptible threshold; the `37`/`59-character` losses
are larger (`602`/`858 us`) but are the poet-graph cost, not a short-key issue.

![Native Track A latency across all input dimensions, Yune vs librime 1.17.0](./evidence/dashboard-visuals-2026-06-30/native-track-a-latency-ratios.svg)

The final raw lookup diagnostics keep the cost shape stable:

- `ni`: raw table lookup median `18.000 us`, translator median `41.600 us`,
  `182` table candidates, exact lookup calls per op `1.000`, owned candidates
  per op `7.000`.
- 37-character row: raw table lookup median `28.900 us`, translator median
  `890.984 us`, no direct table candidates, and context page candidates per op
  `0.135`.

## Native Memory Cause

Native Track A memory is now a guardrailed comparison-lane watch:

| Measurement | Current value | Read |
| --- | ---: | --- |
| Yune Track A max peak working set | `188.4 MB` | full Luna preset-vocabulary/process high-water |
| librime Track A max peer peak | `17.3 MB` | same-run peer scale |
| Maximum Track A private-bytes proxy | `194.3 MB` | Windows proxy, not iOS `phys_footprint` |
| `poet.vocabulary` | `53.6 MB` | retained full upstream Luna preset vocabulary |
| `poet.entries_by_code` | `18.7 MB` | retained upstream sentence-model entries |
| `poet.lookup_index` | `2.7 MB` | guarded M40 sentence lookup index |
| Process unclassified lower bound | `105.6 MB` | carried process proxy after named owners |

![Native Track A memory peak and named owners](./evidence/dashboard-visuals-2026-06-30/native-track-a-memory.svg)

This does **not** invalidate M47. M47's comments-intact
`jyut6ping3_mobile` keyboard profile remains the separate iOS-target lane and
reports about `22 MB` private in the lean native probe. The `188.4 MB` value
here is the full `luna_pinyin` Track A peer-comparison harness after M48 loaded
the upstream preset vocabulary.

The runtime path that loads these owners is schema selection for native
`luna_pinyin`: the schema install path recognizes the upstream script
translator/dictionary pair and installs the upstream sentence model with
`essay` preset vocabulary. Byte-backing `poet.vocabulary` would be a real
storage design change and could add per-lookup latency, so M52 leaves it as a
future owner only if native Luna becomes a shipping product profile.

## Native Track B Cause (product lane)

Track B is the native TypeDuck `jyut6ping3` product path and is a separate lane
from Track A: it has no librime peer, so the read is absolute cost plus the M47
byte-backing trajectory. M47 lookup-record/comment byte-backing collapsed the
owned heap, cutting key-sequence private bytes from `420.0 MB` to `183.9 MB`
and median working set from `440.1 MB` to `255.0 MB`. The `504.4 MB` process
peak is flat because it is dominated by the in-process deploy/compile transient
that loads both dictionaries, not by steady residency. These rows are carried
from the 2026-06-29 evidence; M52 ran `-SkipTrackB`.

![Native Track B memory, TypeDuck jyut6ping3 product path](./evidence/dashboard-visuals-2026-06-30/native-track-b-memory.svg)

## Browser Root Cause

Carried forward from the 2026-06-28 Playwright run.

The fair browser target is `luna_pinyin`, not Jyutping:

| Scenario | Ready | Input -> candidate | Commit | WASM peak | Resource payload | Read |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| Yune public demo `luna_pinyin` | `1000 ms` | `74 ms` | `107 ms` | `64.0 MiB` | `29.5 MiB` | fair Yune row |
| My RIME live `luna_pinyin` | `634 ms` | `95 ms` | `119 ms` | `16.0 MiB` | `8.5 MiB` | fair peer row |

The fair gap remains `4.0x`; startup and WASM memory are the browser-side
blockers. Jyutping remains a launch guard lane, not a peer lane, because the
dictionary families differ.

![Browser memory and payload by lane](./evidence/current-performance-dashboard-2026-06-29/visuals/current-browser-memory-payload.svg)

## Current Evidence

Key tables:

- [`final-native-benchmark/summary-comparison.csv`](./evidence/m52-track-a-guardrails-and-disposition/final-native-benchmark/summary-comparison.csv)
- [`final-native-benchmark/threshold-check.csv`](./evidence/m52-track-a-guardrails-and-disposition/final-native-benchmark/threshold-check.csv)
- [`final-native-benchmark/raw_lookup_microbench.csv`](./evidence/m52-track-a-guardrails-and-disposition/final-native-benchmark/raw_lookup_microbench.csv)
- [`final-native-benchmark/memory-owner-profile.csv`](./evidence/m52-track-a-guardrails-and-disposition/final-native-benchmark/memory-owner-profile.csv)
- [`track-a-thresholds.csv`](./evidence/m52-track-a-guardrails-and-disposition/track-a-thresholds.csv)

Browser evidence remains under
[`current-performance-dashboard-2026-06-29/`](./evidence/current-performance-dashboard-2026-06-29/).

## Next Diagnostic Order

| Rank | Work | Why this is next |
| ---: | --- | --- |
| 1 | Browser fair-lane memory floor on `luna_pinyin` | Same-schema browser gap is `64.0 MiB` vs `16.0 MiB`. |
| 2 | Browser startup phases | Yune public-demo `luna_pinyin` ready-to-input is `1000 ms` vs My RIME `634 ms`. |
| 3 | Native Track A memory owner work, only if product relevance changes | Full Luna Track A is now guardrailed at `188.4 MB` peak but is not the current native TypeDuck product profile. |
| 4 | Native `ni` exact-row scan, only with a tiny owner | M52 closes this as a `30.650 us` bounded ceiling and rejects retained heap indexes. |
| 5 | Native 37-character sentence-model cost, only with a tiny owner | M52 closes this as a `601.967 us` bounded ceiling and keeps poet parity intact. |

## History

Archived milestone-style report:
[`history/2026-06-28-yune-vs-librime-root-cause-analysis-pre-current-dashboard.md`](./history/2026-06-28-yune-vs-librime-root-cause-analysis-pre-current-dashboard.md).
