# Current Yune Performance Dashboard

Date: 2026-07-04 (corrective re-baseline)

This dashboard shows the current benchmark state only. Older milestone closeout
narrative and superseded benchmark rows remain in
[`history/2026-06-28-yune-vs-librime-performance-pre-current-dashboard.md`](./history/2026-06-28-yune-vs-librime-performance-pre-current-dashboard.md).

**Measurement note (load-bearing):** as of the 2026-07-04 corrective series the
native benchmark reads the context after **every keypress** inside the timed
loop, for Yune and librime alike — the shape every real frontend has. All
earlier `key_sequence_process_with_context` numbers (M52, the pre-corrective
M55 rows) were batch-shaped (one context read per sequence) and are **not
comparable** to the rows below. The pre-corrective M55 closeout numbers
(`0.237x`/`0.086x` long rows, `0.286x` startup) were artifacts of a since-
reverted key deferral and config cache measured under that batch shape; see
[`evidence/m55-native-match-or-beat/corrective-2026-07-04/`](./evidence/m55-native-match-or-beat/corrective-2026-07-04/).

## Technical Summary

- **Native Track A (`luna_pinyin`)**: M55 closes with real, honestly measured
  improvements and corrected claims. Versus the pre-M55 record the long
  sentence rows improved ~35% (37-char `3.05x -> 1.91x`, 59-char
  `2.25x -> 1.53x`), `ni`/`hao` improved (`3.14x -> 2.43x`,
  `2.15x -> 1.57x`), startup and session lifecycle are measured **faster than
  librime** (`0.90x`/`0.86x`, run-noisy), and the three win rows are kept and
  guarded `<1.00x`. Yune remains slower than librime on the short keys and
  both sentence rows.
- **Standing guardrail**: the corrective per-key
  [`m55-thresholds.csv`](./evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv)
  is the standing native gate (all dimensions ceilinged, wins locked
  `<1.00x`, Track B absolutes included), green twice consecutively
  (`gate-run-d/`, `gate-run-e/`). The M52 artifact and the pre-corrective M55
  artifact are batch-shaped history.
- **Native memory disposition**: the shipping default is the owned poet path
  at `185.7 MB` Track A peak (the latency ceilings bind). The `YUNE-POET/2`
  byte-backed path is a working, parity-preserving **opt-in**
  (`YUNE_POET_BYTE_BACKED=1`) at `~113.2 MB`, but it currently costs
  `4.6x`/`3.2x` on the long rows because the incremental sentence scratch only
  works on owned storage — porting it is the named future owner for
  reclaiming the memory win.
- **Candidate-output disclosure**: Yune matches librime's first candidate page
  on `ni`, `hao`, and both abbreviation rows; it **differs** on `n` and
  `zhongguo` (completion ranking) and on both long-sentence top candidates
  (sentence lattice; e.g. 37-char top `長足` vs librime `長句`). These are
  pre-existing gaps exposed by the M55 Phase 3R-0 oracle fixtures (13 rows are
  named blocked `#[ignore]` tests), not regressions from the corrective
  series. The oracle parity suites pass on their captured rows.
- **Browser fair lane (`luna_pinyin`, carried 2026-06-28)**: Yune public demo
  uses `64.0 MiB` WASM peak versus My RIME `16.0 MiB` (`4.0x`). Yune is slower
  to ready (`1000 ms` vs `634 ms`), but faster on first input (`74 ms` vs
  `95 ms`).

## Current Evidence Bundle

Corrective evidence root (decision runs, gate runs, README):
[`evidence/m55-native-match-or-beat/corrective-2026-07-04/`](./evidence/m55-native-match-or-beat/corrective-2026-07-04/).

Standing gate artifact:
[`evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv`](./evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv).

Consecutive green gate runs: `gate-run-d/` and `gate-run-e/` under the
corrective root. Browser rows are carried from
[`evidence/current-performance-dashboard-2026-06-29/`](./evidence/current-performance-dashboard-2026-06-29/).

## Native Track A

Corrective gate run D, same-run against upstream librime 1.17.0, context read
after every keypress:

| Dimension | Yune median | librime median | Yune / librime | Current read |
| --- | ---: | ---: | ---: | --- |
| startup | `22,428.100 us` | `25,061.500 us` | `0.895x` | faster; run-noisy, guarded at `1.091x` |
| session | `22,468.400 us` | `26,019.700 us` | `0.864x` | faster; guarded by absolute ceiling |
| `n` | `55.100 us` | `20.900 us` | `2.636x` | slower; +34 us absolute |
| `ni` | `42.450 us` | `17.450 us` | `2.433x` | slower; +25 us absolute |
| `hao` | `24.233 us` | `15.400 us` | `1.574x` | slower; +9 us absolute |
| 37-char pinyin | `571.684 us` | `298.859 us` | `1.913x` | slower; improved from `3.05x` pre-M55 |
| 59-char pinyin | `1,017.522 us` | `665.727 us` | `1.528x` | slower; improved from `2.25x` pre-M55 |
| `zhongguo` (common word) | `44.300 us` | `173.762 us` | `0.255x` | faster; win row, guarded `<1.00x` |
| `cszysmsrsd` (10-char abbr) | `454.040 us` | `1,190.230 us` | `0.381x` | faster; win row, guarded `<1.00x` |
| `zybfshmsru` (8-char abbr) | `469.340 us` | `832.090 us` | `0.564x` | faster; win row, guarded `<1.00x` |

The visualization below is regenerated from the corrective gate run D:

![Native Track A latency across all input dimensions, Yune vs librime 1.17.0](./evidence/dashboard-visuals-2026-07-04/native-track-a-latency-ratios.svg)

## Native Track A Guardrails

Corrective gate run D against the standing artifact (run E repeats green):

| Guard | Observed | Ceiling | Status |
| --- | ---: | ---: | --- |
| `n` latency ratio | `2.636x` | `2.890x` | pass |
| `ni` latency ratio | `2.433x` | `2.666x` | pass |
| `hao` latency ratio | `1.574x` | `1.731x` | pass |
| 37-char latency ratio | `1.913x` | `2.094x` | pass |
| 59-char latency ratio | `1.528x` | `1.625x` | pass |
| `zhongguo` win row | `0.255x` | `0.323x` (`<1.00x` locked) | pass |
| `cszysmsrsd` win row | `0.381x` | `0.474x` (`<1.00x` locked) | pass |
| `zybfshmsru` win row | `0.564x` | `0.695x` (`<1.00x` locked) | pass |
| startup ratio | `0.895x` | `1.091x` | pass |
| session median | `22,468.400 us` | `25,470.280 us` | pass |
| Track A peak working set | `185,749,504 B` | `195,028,378 B` | pass |
| Track B product long-row latency | `315.356 us` | `347.975 us` | pass |

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
| Track A peak working set (shipping default, owned poet) | `185.7 MB` | latency ceilings bind; guarded at `195.0 MB` |
| Track A peak working set (`YUNE_POET_BYTE_BACKED=1` opt-in) | `113.2 MB` | real, parity-preserving, but fails the long-row latency ceilings (scratch not yet byte-backed) |
| librime max peer peak (same run) | `13.5 MB` | peer scale |
| `poet.vocabulary` / `poet.entries_by_code` (opt-in mode) | `25.5 MB` / `3.0 MB` | `mmap_file_backed` in the `YUNE-POET/2` artifact |

Native Track A `luna_pinyin` is kept as the upstream comparison lane. The
current native product target remains the TypeDuck/Jyutping profile lane, where
M47's lean probe reports the comments-intact keyboard profile at about `67 MB`
working set / `22 MB` private. These are separate lanes and are not
interchangeable memory claims.

![Native Track A memory peak and named owners](./evidence/dashboard-visuals-2026-07-04/native-track-a-memory.svg)

## Native Track B (TypeDuck `jyut6ping3` product)

Track B is the native TypeDuck/Jyutping product path and regression guard lane
(no librime peer). It is mode-independent for the poet default (sentence is off
in the mobile profile). Corrective gate run D:

| Dimension | Observed | Ceiling | Status |
| --- | ---: | ---: | --- |
| 50+ key-sequence latency | `315.356 us` | `347.975 us` | pass (pre-M55 source baseline `341.139 us`) |
| key-sequence median working set | `79,953,920 B` | `88,012,390 B` | pass |
| key-sequence max peak working set | `510,672,896 B` | `562,033,050 B` | pass (deploy/compile transient) |
| key-sequence median private bytes | `35,733,504 B` | `39,460,045 B` | pass |
| session create/select/destroy | `35,364.100 us` | `39,289.800 us` | pass (~3x better than the Phase 0-era `99.8 ms` source baseline) |
| startup warm runtime-ready | `34,732.800 us` | `38,825.050 us` | pass (~3x better than the Phase 0-era `97.4 ms` source baseline) |

![Native Track B memory, TypeDuck jyut6ping3 product path](./evidence/dashboard-visuals-2026-07-04/native-track-b-memory.svg)

![Native Track B lifecycle latency, TypeDuck jyut6ping3 product path](./evidence/dashboard-visuals-2026-07-04/native-track-b-latency.svg)

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
| 1 | Native sentence-lattice candidate divergence | 37/59-char top candidates and `n`/`zhongguo` pages differ from librime (pre-existing; 13 blocked oracle fixture rows) | future parity milestone over the Phase 3R-0 expanded fixtures |
| 2 | Browser `luna_pinyin` memory | `64.0 MiB` vs My RIME `16.0 MiB` | WASM runtime floor and public-demo resource/heap split |
| 3 | Browser `luna_pinyin` startup | `1000 ms` vs My RIME `634 ms` | startup asset/runtime phases after current public-demo build |
| 4 | Native long-row latency | 37-char `1.913x`, 59-char `1.528x` | poet graph constant factors; original Tier M bar was `1.50x` |
| 5 | Native short keys | `n` `2.636x`, `ni` `2.433x`, `hao` `1.574x` (`+9-34 us` absolute) | compile-time short-key index without retained heap or output change |
| 6 | Native Track A memory peer gap | default `185.7 MB` vs librime `13.5 MB`; opt-in byte-backed `113.2 MB` | port the incremental sentence scratch to byte-backed storage, then re-decide the default |

## History

Older milestone closeout detail remains in:

- [`history/2026-06-28-yune-vs-librime-performance-pre-current-dashboard.md`](./history/2026-06-28-yune-vs-librime-performance-pre-current-dashboard.md)
- [`plans/completed/`](../plans/completed/)
- [`ledgers/milestone-history.md`](../ledgers/milestone-history.md)
- The pre-corrective 2026-07-04 dashboard state (batch-shaped M55 closeout
  numbers) is preserved in git history at commit `531dbcf2` and analyzed in
  [`evidence/m55-native-match-or-beat/corrective-2026-07-04/README.md`](./evidence/m55-native-match-or-beat/corrective-2026-07-04/README.md).
