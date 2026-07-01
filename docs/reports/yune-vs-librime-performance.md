# Current Yune Performance Dashboard

Date: 2026-06-30

This dashboard shows the current benchmark state only. Older milestone closeout
narrative and superseded benchmark rows remain in
[`history/2026-06-28-yune-vs-librime-performance-pre-current-dashboard.md`](./history/2026-06-28-yune-vs-librime-performance-pre-current-dashboard.md).

The native Track A lane was refreshed by the M52 final same-run benchmark on
2026-06-30. The browser lane is carried forward from the 2026-06-28 Playwright
run and was not re-measured in this pass.

## Technical Summary

- **Native fair lane (`luna_pinyin`)**: the final M52 run records startup near
  parity at `1.113x` and session create/select/destroy at `1.001x` versus
  same-run upstream librime 1.17.0. The tracked key rows are guarded by the
  committed M52 threshold artifact and the final regression gate passes.
- **Native latency disposition**: `n` (`2.818x`), `hao` (`2.146x`), and the
  59-character row (`2.247x`) are inside the strict `<=3.0x` ratio target.
  `ni` (`3.143x`, `30.650 us` absolute gap) and the 37-character row
  (`3.053x`, `601.967 us` absolute gap) remain just above the strict ratio
  target and are closed by M52 as bounded-microsecond ceilings, not as product
  launch blockers.
- **Native Track A memory**: the final Track A high-water is
  `188,383,232 B` (`188.4 MB`) versus librime's `17,276,928 B` max peer peak in
  the same run. This full `luna_pinyin` preset-vocabulary shape is a comparison
  lane, not the current native TypeDuck product profile, and is now protected by
  the M52 `198,000,000 B` ceiling gate. It is separate from the M47
  `jyut6ping3_mobile` iOS-keyboard proxy.
- **Browser fair lane (`luna_pinyin`, carried 2026-06-28)**: Yune public demo
  uses `64.0 MiB` WASM peak versus My RIME `16.0 MiB` (`4.0x`). Yune is slower
  to ready (`1000 ms` vs `634 ms`), but faster on first input (`74 ms` vs
  `95 ms`).
- **Browser Jyutping (carried 2026-06-28)**: Yune public demo is byte-backed at
  `160.0 MiB` WASM peak. This remains a guard row, not a fair peer comparison,
  because My RIME's Jyutping uses a different Cantonese-only dictionary.

## Current Evidence Bundle

Fresh native source:
[`evidence/m52-track-a-guardrails-and-disposition/final-native-benchmark/`](./evidence/m52-track-a-guardrails-and-disposition/final-native-benchmark/).

M52 guardrail source of truth:
[`evidence/m52-track-a-guardrails-and-disposition/track-a-thresholds.csv`](./evidence/m52-track-a-guardrails-and-disposition/track-a-thresholds.csv).

Final regression-gate output:
[`evidence/m52-track-a-guardrails-and-disposition/final-native-benchmark/threshold-check.csv`](./evidence/m52-track-a-guardrails-and-disposition/final-native-benchmark/threshold-check.csv).

The normalized dashboard source from the previous browser-inclusive dashboard is
still available under
[`evidence/current-performance-dashboard-2026-06-29/`](./evidence/current-performance-dashboard-2026-06-29/);
browser rows in this file are carried from that evidence.

## Native Track A

| Dimension | Yune median | librime median | Yune / librime | Current read |
| --- | ---: | ---: | ---: | --- |
| startup | `24,139.200 us` | `21,686.900 us` | `1.113x` | near parity |
| session | `23,404.000 us` | `23,390.700 us` | `1.001x` | parity |
| `n` | `60.300 us` | `21.400 us` | `2.818x` | strict ratio pass; threshold pass |
| `ni` | `44.950 us` | `14.300 us` | `3.143x` | bounded-microsecond ceiling; threshold pass |
| `hao` | `24.967 us` | `11.633 us` | `2.146x` | strict ratio pass; threshold pass |
| 37-char pinyin | `895.178 us` | `293.211 us` | `3.053x` | bounded-microsecond ceiling; threshold pass |
| 59-char pinyin | `1,545.754 us` | `687.795 us` | `2.247x` | strict ratio pass; threshold pass |
| `zhongguo` (common word) | `46.150 us` | `166.600 us` | `0.277x` | Yune faster (carried 2026-06-29) |
| `cszysmsrsd` (10-char abbr) | `517.720 us` | `1,218.320 us` | `0.425x` | Yune faster (carried 2026-06-29) |
| `zybfshmsru` (8-char abbr) | `547.580 us` | `859.790 us` | `0.637x` | Yune faster (carried 2026-06-29) |

Across the full input-dimension suite the picture is mixed, not slower
everywhere. Yune is faster than librime on `zhongguo` and the two abbreviation
rows (green below), where the compiled index and Rust path win by `120-701 us`.
It is slower on the short keys `n`/`ni`/`hao` (coral, but only `13-39 us`,
imperceptible) and on the `37`/`59-character` pinyin sentence rows
(`602`/`858 us`, the poet-graph cost). Startup and session are near parity and
run-noisy. The M52 gate rows carry M52 numbers; `zhongguo` and the abbreviation
rows are carried from the 2026-06-29 full suite.

![Native Track A latency across all input dimensions, Yune vs librime 1.17.0](./evidence/dashboard-visuals-2026-06-30/native-track-a-latency-ratios.svg)

## Native Track A Guardrail

| Guard | Observed | Ceiling | Status |
| --- | ---: | ---: | --- |
| `n` latency ratio | `2.818x` | `3.050x` | pass |
| `ni` latency ratio | `3.143x` | `3.223x` | pass |
| `hao` latency ratio | `2.146x` | `2.287x` | pass |
| 37-char latency ratio | `3.053x` | `3.267x` | pass |
| 59-char latency ratio | `2.247x` | `2.447x` | pass |
| Track A peak working set | `188,383,232 B` | `198,000,000 B` | pass |

The gate is run by
`scripts/benchmark-native-rime-inprocess.ps1 -TrackAThresholds <thresholds> -FailOnRegression`.
M52 intentionally did not add retained heap indexes or widen ABI surfaces.

## Native Track A Memory

| Measurement | Current value | Current read |
| --- | ---: | --- |
| Yune Track A peak working set | `188.4 MB` | comparison-lane high-water, now guardrailed |
| librime max peer peak | `17.3 MB` | same-run peer scale |
| Maximum Track A private-bytes proxy | `194.3 MB` | Windows proxy, not iOS `phys_footprint` |
| `poet.vocabulary` | `53.6 MB` | full upstream Luna preset vocabulary retained on heap |
| `poet.entries_by_code` | `18.7 MB` | upstream sentence-model entries retained on heap |
| Process unclassified lower bound | `105.6 MB` | measured process proxy after named owner accounting |

Native Track A `luna_pinyin` is kept as the upstream comparison lane. The current
native product target remains the TypeDuck/Jyutping profile lane, where M47's
lean probe reports the comments-intact keyboard profile at about `67 MB` working
set / `22 MB` private. These are separate lanes and are not interchangeable
memory claims.

![Native Track A memory peak and named owners](./evidence/dashboard-visuals-2026-06-30/native-track-a-memory.svg)

## Native Track B (TypeDuck `jyut6ping3` product)

Track B is the native TypeDuck/Jyutping product path. It has no librime peer
lane, because TypeDuck is a fork rather than upstream librime, so the read is
absolute memory cost and the M47 byte-backing trajectory, not a same-run ratio.
These rows are carried from the 2026-06-29 evidence; M52 ran `-SkipTrackB`.

| Dimension | 2026-06-28 | 2026-06-29 | Read |
| --- | ---: | ---: | --- |
| peak working set (process) | `504.4 MB` | `504.4 MB` | flat; in-process deploy/compile transient over both dictionaries |
| median working set (key sequence) | `440.1 MB` | `255.0 MB` | resident fell with byte-backed records |
| private bytes (key sequence) | `420.0 MB` | `183.9 MB` | owned heap collapsed by M47 lookup/comment byte-backing |
| private bytes (session) | `405.8 MB` | `178.8 MB` | steady private dropped with byte-backed records |
| key-sequence latency | `341.674 us` | `336.490 us` | product key latency essentially flat |
| startup warm runtime-ready | `124,505.300 us` | `90,795.000 us` | faster with byte-backed records |
| session create/select/destroy | `141,590.400 us` | `92,093.100 us` | faster with byte-backed records |

![Native Track B memory, TypeDuck jyut6ping3 product path](./evidence/dashboard-visuals-2026-06-30/native-track-b-memory.svg)

![Native Track B lifecycle latency, TypeDuck jyut6ping3 product path](./evidence/dashboard-visuals-2026-06-30/native-track-b-latency.svg)

Source: [`current-native-track-b.csv`](./evidence/current-performance-dashboard-2026-06-29/current-native-track-b.csv).
This is the M47/M46 native product lane and must not be conflated with the
Track A `luna_pinyin` comparison lane above or with the iOS `phys_footprint`
target.

## Browser Peer Dashboard

Carried forward from the 2026-06-28 Playwright run.

| Scenario | Schema | Ready | Input -> candidate | Commit | WASM peak | Unique encoded resources | Validity |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- |
| Yune public demo | `luna_pinyin` | `1000 ms` | `74 ms` | `107 ms` | `64.0 MiB` | `29.5 MiB` | fair |
| My RIME live | `luna_pinyin` | `634 ms` | `95 ms` | `119 ms` | `16.0 MiB` | `8.5 MiB` | fair |
| Yune public demo | Jyutping | `1347 ms` | `103 ms` | `108 ms` | `160.0 MiB` | `72.2 MiB` | guard only |
| My RIME live | Jyutping | `998 ms` | `99 ms` | `114 ms` | `68.0 MiB` | `24.9 MiB` | guard only |

![Browser peer latency: Yune public demo vs My RIME](./evidence/current-performance-dashboard-2026-06-29/visuals/current-browser-peer-latency.svg)

![Browser memory and payload by lane](./evidence/current-performance-dashboard-2026-06-29/visuals/current-browser-memory-payload.svg)

![Yune browser input latency suite](./evidence/current-performance-dashboard-2026-06-29/visuals/current-yune-browser-input-latency.svg)

Browser visuals are carried unchanged from the 2026-06-28 Playwright run under
[`current-performance-dashboard-2026-06-29/visuals/`](./evidence/current-performance-dashboard-2026-06-29/visuals/).

## Remaining Current Gaps

| Rank | Gap | Current value | Next diagnostic target |
| ---: | --- | --- | --- |
| 1 | Browser `luna_pinyin` memory | `64.0 MiB` vs My RIME `16.0 MiB` | WASM runtime floor and public-demo resource/heap split |
| 2 | Browser `luna_pinyin` startup | `1000 ms` vs My RIME `634 ms` | startup asset/runtime phases after current public-demo build |
| 3 | Native Track A full-Luna memory watch | `188.4 MB` vs librime max peer peak `17.3 MB`; M52 ceiling `198.0 MB` | future owner work only if native Luna becomes a shipping product profile |
| 4 | Native `ni` latency watch | `44.950 us` vs `14.300 us`; threshold ceiling `3.223x` | bounded exact-row scan under charset filtering, no retained heap prefix index |
| 5 | Native 37-char latency watch | `895.178 us` vs `293.211 us`; threshold ceiling `3.267x` | preset-vocabulary sentence-model graph cost, no parity-risk shortcut |

![Current performance gaps by lane](./evidence/dashboard-visuals-2026-06-30/root-cause-gaps.svg)

## History

Older milestone closeout detail remains in:

- [`history/2026-06-28-yune-vs-librime-performance-pre-current-dashboard.md`](./history/2026-06-28-yune-vs-librime-performance-pre-current-dashboard.md)
- [`plans/completed/`](../plans/completed/)
- [`ledgers/milestone-history.md`](../ledgers/milestone-history.md)
