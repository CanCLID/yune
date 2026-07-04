# Current Yune Performance Dashboard

Date: 2026-07-04

This dashboard shows the current benchmark state only. Older milestone closeout
narrative and superseded benchmark rows remain in
[`history/2026-06-28-yune-vs-librime-performance-pre-current-dashboard.md`](./history/2026-06-28-yune-vs-librime-performance-pre-current-dashboard.md).

The native Track A lane was refreshed by the M55 final default-on ratchets on
2026-07-04. The browser lane is carried forward from the 2026-06-28 Playwright
run and was not re-measured in M55.

## Technical Summary

- **Native Track A (`luna_pinyin`)**: M55 is complete for its Tier M/bounded-gap
  target. The final product path consumes validated byte-backed Luna poet bytes
  by default, keeps the accepted Yune product-path candidate output unchanged,
  and passes the full M55 ratchet twice under
  `phase-5-final/default-on-ratchet-5-config-cache/` and
  `phase-5-final/default-on-ratchet-6-config-cache/`.
- **Standing guardrail**: M55's `m55-thresholds.csv` supersedes M52 as the
  broader native Track A regression gate. It covers startup, session lifecycle,
  eight Track A key rows, Track A peak memory, and Track B product absolute
  guards.
- **Native latency disposition**: final run 6 has `n` at `1.794x`, `ni` at
  `1.039x`, `hao` at `0.815x`, 37-character Luna at `0.237x`, and
  59-character Luna at `0.086x`. Only rows measured below `1.00x` are described
  as faster than librime; `n` is a bounded-gap pass, not a match-or-beat row.
- **Native memory disposition**: final run 6 reports Track A peak
  `113,397,760 B`, below both the tightened `118,831,104 B` M55 ceiling and the
  `125 MB` Tier M bar.
- **Browser fair lane (`luna_pinyin`, carried 2026-06-28)**: Yune public demo
  uses `64.0 MiB` WASM peak versus My RIME `16.0 MiB` (`4.0x`). Yune is slower
  to ready (`1000 ms` vs `634 ms`), but faster on first input (`74 ms` vs
  `95 ms`).

## Current Evidence Bundle

Final native source:
[`evidence/m55-native-match-or-beat/phase-5-final/default-on-ratchet-6-config-cache/`](./evidence/m55-native-match-or-beat/phase-5-final/default-on-ratchet-6-config-cache/).

Consecutive green companion run:
[`evidence/m55-native-match-or-beat/phase-5-final/default-on-ratchet-5-config-cache/`](./evidence/m55-native-match-or-beat/phase-5-final/default-on-ratchet-5-config-cache/).

M55 threshold artifact:
[`evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv`](./evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv).

Closeout note:
[`evidence/m55-native-match-or-beat/phase-5-final/closeout-2026-07-04.md`](./evidence/m55-native-match-or-beat/phase-5-final/closeout-2026-07-04.md).

The normalized dashboard source from the previous browser-inclusive dashboard is
still available under
[`evidence/current-performance-dashboard-2026-06-29/`](./evidence/current-performance-dashboard-2026-06-29/);
browser rows in this file are carried from that evidence.

## Native Track A

M55 final run 6, same-run against upstream librime 1.17.0:

| Dimension | Yune median | librime median | Yune / librime | Current read |
| --- | ---: | ---: | ---: | --- |
| startup | `6,567.800 us` | `22,945.800 us` | `0.286x` | faster in this native gate; startup remains local-host sensitive |
| session | `6,679.500 us` | `20,352.600 us` | `0.328x` | faster in this native gate |
| `n` | `36.600 us` | `20.400 us` | `1.794x` | Tier M bounded-gap pass; not match-or-beat |
| `ni` | `14.650 us` | `14.100 us` | `1.039x` | Tier M bounded-gap pass |
| `hao` | `9.267 us` | `11.367 us` | `0.815x` | faster than librime in this gate |
| 37-char pinyin | `66.665 us` | `281.559 us` | `0.237x` | faster than librime in this gate |
| 59-char pinyin | `55.641 us` | `644.025 us` | `0.086x` | faster than librime in this gate |
| `zhongguo` (common word) | `6.038 us` | `158.338 us` | `0.038x` | faster; win-row guard pass |
| `cszysmsrsd` (10-char abbr) | `103.950 us` | `1,159.300 us` | `0.090x` | faster; win-row guard pass |
| `zybfshmsru` (8-char abbr) | `100.700 us` | `817.200 us` | `0.123x` | faster; win-row guard pass |

The older 2026-06-30 visualization below is retained as a historical visual for
row shape, not as the current numeric source:

![Native Track A latency across all input dimensions, Yune vs librime 1.17.0](./evidence/dashboard-visuals-2026-06-30/native-track-a-latency-ratios.svg)

## Native Track A Guardrails

M55 final run 6 threshold gate:

| Guard | Observed | Ceiling | Status |
| --- | ---: | ---: | --- |
| `n` latency ratio | `1.794x` | `2.000x` | pass |
| `ni` latency ratio | `1.039x` | `2.000x` | pass |
| `hao` latency ratio | `0.815x` | `1.750x` | pass |
| 37-char latency ratio | `0.237x` | `1.500x` | pass |
| 59-char latency ratio | `0.086x` | `1.500x` | pass |
| `zhongguo` latency ratio | `0.038x` | `0.325x` | pass |
| `cszysmsrsd` latency ratio | `0.090x` | `0.532x` | pass |
| `zybfshmsru` latency ratio | `0.123x` | `0.770x` | pass |
| startup ratio | `0.286x` | `1.101x` | pass |
| session median | `6,679.500 us` | `25,533.310 us` | pass |
| Track A peak working set | `113,397,760 B` | `118,831,104 B` | pass |
| Track B product long-row latency | `316.282 us` | `375.253 us` | pass |

Manual standing gate command shape:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 `
  -OutputRoot docs\reports\evidence\<new-run> `
  -Iterations 9 -SessionIterations 60 -KeyIterations 80 `
  -TrackAInputs n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru `
  -TrackBInputs neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung `
  -DeployProductBeforeBenchmark `
  -TrackAThresholds docs\reports\evidence\m55-native-match-or-beat\thresholds\m55-thresholds.csv `
  -FailOnRegression
```

## Native Track A Memory

| Measurement | Current value | Current read |
| --- | ---: | --- |
| M55 final Track A peak working set | `113.4 MB` | final default-on product path; M55 guard pass |
| M55 tightened Track A peak ceiling | `118.8 MB` | below the `125 MB` Tier M bar |
| M52 Track A peak working set | `188.4 MB` | previous standing dashboard value |
| M52 librime max peer peak | `17.3 MB` | peer scale from the M52 standing guard |
| `poet.vocabulary` after byte-backing | `25.5 MB` | `mmap_file_backed` poet artifact bytes in diagnostic probe |
| `poet.entries_by_code` after byte-backing | `3.0 MB` | `mmap_file_backed` poet artifact bytes in diagnostic probe |

Native Track A `luna_pinyin` is kept as the upstream comparison lane. The
current native product target remains the TypeDuck/Jyutping profile lane, where
M47's lean probe reports the comments-intact keyboard profile at about `67 MB`
working set / `22 MB` private. These are separate lanes and are not
interchangeable memory claims.

The older memory visualization remains historical:

![Native Track A memory peak and named owners](./evidence/dashboard-visuals-2026-06-30/native-track-a-memory.svg)

## Native Track B (TypeDuck `jyut6ping3` product)

Track B is the native TypeDuck/Jyutping product path. It has no librime peer
lane, because TypeDuck is a fork rather than upstream librime, so the read is
absolute memory cost and the M47 byte-backing trajectory, not a same-run ratio.
M55 uses Track B only as a regression guard.

M55 final run 6 Track B guard:

| Dimension | Observed | Ceiling | Status |
| --- | ---: | ---: | --- |
| 50+ key-sequence latency | `316.282 us` | `375.253 us` | pass |
| key-sequence median working set | `80,150,528 B` | `280,518,656 B` | pass |
| key-sequence max peak working set | `511,053,824 B` | `564,065,075 B` | pass |
| key-sequence median private bytes | `36,982,784 B` | `200,620,851 B` | pass |
| session create/select/destroy | `31,279.700 us` | `109,795.290 us` | pass |
| startup warm runtime-ready | `30,114.800 us` | `107,085.000 us` | pass |

Historical Track B visuals from the previous dashboard:

![Native Track B memory, TypeDuck jyut6ping3 product path](./evidence/dashboard-visuals-2026-06-30/native-track-b-memory.svg)

![Native Track B lifecycle latency, TypeDuck jyut6ping3 product path](./evidence/dashboard-visuals-2026-06-30/native-track-b-latency.svg)

## Browser Peer Dashboard

Carried forward from the 2026-06-28 Playwright run.

| Scenario | Schema | Ready | Input -> candidate | Commit | WASM peak | Unique encoded resources | Validity |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- |
| Yune public demo | `luna_pinyin` | `1000 ms` | `74 ms` | `107 ms` | `64.0 MiB` | `29.5 MiB` | fair |
| My RIME live | `luna_pinyin` | `634 ms` | `95 ms` | `119 ms` | `16.0 MiB` | `8.5 MiB` | fair |
| Yune public demo | Jyutping | `1347 ms` | `103 ms` | `108 ms` | `160.0 MiB` | `72.2 MiB` | guard only |
| My RIME live | Jyutping | `998 ms` | `99 ms` | `114 ms` | `68.0 MiB` | `24.9 MiB` | guard only |

Browser visuals are carried unchanged from the 2026-06-28 Playwright run under
[`current-performance-dashboard-2026-06-29/visuals/`](./evidence/current-performance-dashboard-2026-06-29/visuals/).

## Remaining Current Gaps

| Rank | Gap | Current value | Next diagnostic target |
| ---: | --- | --- | --- |
| 1 | Browser `luna_pinyin` memory | `64.0 MiB` vs My RIME `16.0 MiB` | WASM runtime floor and public-demo resource/heap split |
| 2 | Browser `luna_pinyin` startup | `1000 ms` vs My RIME `634 ms` | startup asset/runtime phases after current public-demo build |
| 3 | Native Track A `n` short-key gap | `1.794x`, a Tier M pass but not match-or-beat | only if a future native owner wants the last microseconds |
| 4 | Native Track A memory peer gap | M55 final `113.4 MB` vs librime peer scale around `17 MB` | future native memory owner only with latency guard preserved |

## History

Older milestone closeout detail remains in:

- [`history/2026-06-28-yune-vs-librime-performance-pre-current-dashboard.md`](./history/2026-06-28-yune-vs-librime-performance-pre-current-dashboard.md)
- [`plans/completed/`](../plans/completed/)
- [`ledgers/milestone-history.md`](../ledgers/milestone-history.md)
