# Current Yune Performance Dashboard

Date: 2026-07-06 (M58 corrective closeout at `f780410c`; standing gate remains
the 2026-07-04 corrective re-baseline and was re-run green for M58)

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

**macOS verification note:** M57 fixed a macOS-only Yune sentence-model
construction defect in the Track A verification bundle. The macOS
`rime_deployer`-compiled upstream Luna MARISA table uses checksum pair
`0xb3d4e98e` / `0x29d56c89`; after M57, Yune accepts that target-scoped pair,
keeps compact storage active, and restores the expected model shape
(`332,604` compact codes, `513,353` expanded sentence entries, 11-row
abbreviation vocabulary). Evidence:
[`evidence/m57-macos-track-a-sentence-model-parity/`](./evidence/m57-macos-track-a-sentence-model-parity/).

**M58 Jyutping/profile note:** M58 completed the upstream Jyutping oracle
rebase at `f780410c`. Canonical `jyut6ping3` candidate behavior now uses
upstream `rime/librime 1.17.0` plus pinned `rime/rime-cantonese`; the
user-specified `zijiguk` / `諮議局` capture returns `諮議局` first, so no
canonical candidate bug was reproduced and no canonical fix was derived. The
shipped `yune-web` TypeDuck/profile lane had separate reachability bugs:
`beingo` / `畀` at TypeDuck/profile index 6 and `zi` / `諮` at index 27. Those
were fixed by restoring `畀	bei2	200000`, retaining one TypeDuck/profile page
for short `jyut6ping3_mobile` reported/profile inputs, and widening prefix
fallback only on that short-input path, without first-page promotion. No schema
id split, profile predicate change, userdb migration, or ABI widening landed;
`jyut6ping3_typeduck` remains the preferred future TypeDuck profile id pending
explicit sign-off.

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
  (`gate-run-d/`, `gate-run-e/`) and re-run green at M56 closeout under
  [`m56-productization-hardening/final/ratchet-run/`](./evidence/m56-productization-hardening/final/ratchet-run/)
  and again at M58 closeout under
  [`m58-jyutping-exact-before-fuzzy/phase-2b/m55-product-ratchet-corrective-final-pass2/`](./evidence/m58-jyutping-exact-before-fuzzy/phase-2b/m55-product-ratchet-corrective-final-pass2/).
  The M56 and M58 runs are guard proofs, not performance rebaselines: some
  short-key and sentence-row ratios drift upward but remain inside the
  committed ceilings.
  The M52 artifact and the pre-corrective M55 artifact are batch-shaped
  history.
- **macOS verification repair**: M57 repaired the macOS Track A bundle so it is
  no longer a false contradiction of the M55 corrective record. The two full
  macOS passes now keep Luna on `rsmarisa_byte_backed` storage, report
  `compact_all_codes_count=332604` and
  `compact_expanded_table_entries=513353`, and match local librime first-page
  candidates for `cszysmsrsd` and `zybfshmsru`. This is a comparability repair,
  not a new cross-platform performance headline.
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
  (sentence lattice; see the captured candidate snapshots). These are
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

M57 macOS verification repair evidence:
[`evidence/m57-macos-track-a-sentence-model-parity/`](./evidence/m57-macos-track-a-sentence-model-parity/).

M58 Jyutping/profile corrective closeout evidence:
[`evidence/m58-jyutping-exact-before-fuzzy/`](./evidence/m58-jyutping-exact-before-fuzzy/).

Standing gate artifact:
[`evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv`](./evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv).

Consecutive green gate runs: `gate-run-d/` and `gate-run-e/` under the
corrective root. Latest closeout proof:
[`evidence/m58-jyutping-exact-before-fuzzy/phase-2b/m55-product-ratchet-corrective-final-pass2/threshold-check.csv`](./evidence/m58-jyutping-exact-before-fuzzy/phase-2b/m55-product-ratchet-corrective-final-pass2/threshold-check.csv).
Browser rows are carried from
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

## Native Track A — Windows vs macOS (cross-platform, post-M57)

With M57's macOS repair landed, macOS is a fair Track A lane and can be shown
next to the Windows standing gate. **This is a two-machine comparison** — a
Windows x86 desktop (M55 corrective gate run D) versus an Apple Silicon MacBook
Air (M57 full-pass-1; full-pass-2 gives the reproducibility range) — so any
difference is dominated by hardware and toolchain, not by the OS name.

| Dimension | Yune Win | librime Win | Win ratio | Yune mac | librime mac | mac ratio |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| startup (runtime-ready) | `22,428 us` | `25,062 us` | `0.895x` | `18,111 us` | `31,463 us` | `0.576x` |
| session lifecycle | `22,468 us` | `26,020 us` | `0.864x` | `18,818 us` | `30,066 us` | `0.626x` |
| `zhongguo` (common word) † | `44.3 us` | `173.8 us` | `0.255x` | `42.9 us` | `110.6 us` | `0.388x` (`0.16–0.39x`) |
| `cszysmsrsd` (10-char abbr) | `454.0 us` | `1,190.2 us` | `0.381x` | `419.8 us` | `834.0 us` | `0.503x` |
| `zybfshmsru` (8-char abbr) | `469.3 us` | `832.1 us` | `0.564x` | `461.0 us` | `576.7 us` | `0.799x` |
| `hao` (short key) † | `24.2 us` | `15.4 us` | `1.574x` | `26.0 us` | `15.3 us` | `1.697x` (`1.15–1.70x`) |
| `ni` (short key) † | `42.5 us` | `17.5 us` | `2.433x` | `46.1 us` | `18.3 us` | `2.522x` (`2.52–4.10x`) |
| `n` (short key) † | `55.1 us` | `20.9 us` | `2.636x` | `64.5 us` | `27.0 us` | `2.385x` (`2.39–4.32x`) |
| 59-char pinyin | `1,017.5 us` | `665.7 us` | `1.528x` | `943.7 us` | `402.3 us` | `2.346x` |
| 37-char pinyin | `571.7 us` | `298.9 us` | `1.913x` | `512.6 us` | `174.6 us` | `2.936x` |

`†` the macOS *ratio* on these rows is noise-affected and shown as a range
(pass-1↔pass-2): for `hao`/`ni`/`n` librime's absolute sits at the
microbenchmark noise floor (single-digit-to-low-tens of `us`), and for
`zhongguo` librime's own median swings run-to-run (`111 → 264 us`). Only the
sentence, abbreviation, and lifecycle rows have a stable enough librime baseline
to compare cross-platform.

![Native Track A latency ratio, Windows vs macOS](./evidence/dashboard-visuals-2026-07-05-cross-platform/native-track-a-latency-windows-vs-macos.svg)

How to read the cross-platform rows:

- **Yune's own per-key latency is comparable across the two machines** and
  reproduces between the macOS passes — 37-char `572 us` (Win) → `513 us` (mac);
  `cszysmsrsd` `454 us` → `420 us`; `zybfshmsru` `469 us` → `461 us`. Yune is
  not slower on macOS.
- **The ratio widens on the sentence rows** (37-char `1.913x` → `2.936x`)
  because **librime is faster on the Apple Silicon machine** (37-char librime
  `299 us` (Win) → `175 us` (mac)), not because Yune slowed down. Dividing by a
  faster librime inflates the ratio.
- **The win rows stay wins on both machines** (`zhongguo`, `cszysmsrsd`,
  `zybfshmsru` all `<1.00x`), and startup/session stay faster than librime on
  both (run-noisy, as flagged).
- **Candidate parity holds on macOS post-M57**: Yune matches librime's first
  page on `ni`, `hao`, and both abbreviation rows; `zhongguo`, `n`, and the two
  long-sentence tops differ (for `zhongguo` only the top candidate matches — the
  rest of the page differs) — the same pre-existing lattice/completion gaps
  disclosed for Windows, not a macOS regression.

**Memory (indicative only — the counters differ across platforms).** Windows
reports working-set / private bytes; macOS reports resident size. Track A peak:
Yune `185.7 MB` (Win working set) / `~228 MB` (mac resident); librime `17.2 MB`
(Win) / `~16 MB` (mac) — the same order-of-magnitude peer-scale direction on
both (`~11x` Win, `~14x` mac), but the absolute megabytes are not directly
comparable.

**Track B (TypeDuck profile product path) on macOS post-M57.** Key-sequence
latency `~289 us` (mac) vs `315 us` (Win); session/startup `~29 ms` (mac) vs
`~35 ms` (Win). Track B memory is not compared cross-platform (macOS resident
plus deploy/compile transient is not the Windows working-set metric). This is
profile/product guard evidence, not canonical `rime-cantonese` candidate-order
oracle evidence. M58 completed the schema/profile blast-radius
audit and did not implement a split; the preferred future TypeDuck multilingual
id remains `jyut6ping3_typeduck` pending explicit sign-off.

## Native Track A Guardrails

Corrective gate run D against the standing artifact (run E and the M56
closeout ratchet repeat green):

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

Latest M58 closeout ratchet read: all `23` rows pass, but the short-key rows
and sentence rows still have limited headroom (`n` `2.770x` / `2.890x`, `ni`
`2.494x` / `2.666x`, `hao` `1.654x` / `1.731x`, 37-char `2.022x` / `2.094x`,
59-char `1.567x` / `1.625x`, Track B long row `335.823 us` / `347.975 us`).
This is guard proof, not a new performance headline; do not summarize it as
"no measurable performance cost."

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

## Native Track B (TypeDuck Profile Product Path)

Track B is the native TypeDuck/Jyutping profile product path and regression
guard lane (no librime peer). Current evidence uses historical
`jyut6ping3_mobile` asset names; it should be read as TypeDuck profile evidence
and future schema-split work should present that lane as
`jyut6ping3_typeduck` only after explicit sign-off. M58 completed the
blast-radius audit and did not implement the split.
It is mode-independent for the poet default (sentence is off in the mobile
profile). Latest M58 final-pass ratchet:

| Dimension | Observed | Ceiling | Status |
| --- | ---: | ---: | --- |
| 50+ key-sequence latency | `335.823 us` | `347.975 us` | pass (pre-M55 source baseline `341.139 us`) |
| key-sequence median working set | `79,511,552 B` | `88,012,390 B` | pass |
| key-sequence max peak working set | `510,885,888 B` | `562,033,050 B` | pass (deploy/compile transient) |
| key-sequence median private bytes | `35,426,304 B` | `39,460,045 B` | pass |
| session create/select/destroy | `36,098.200 us` | `39,289.800 us` | pass (~3x better than the Phase 0-era `99.8 ms` source baseline) |
| startup warm runtime-ready | `35,459.000 us` | `38,825.050 us` | pass (~3x better than the Phase 0-era `97.4 ms` source baseline) |

The visualizations below are carried from the 2026-07-04 standing-gate
dashboard and remain directional; the M58 table above is the current Track B
ratchet read.

![Native Track B memory, TypeDuck profile product path](./evidence/dashboard-visuals-2026-07-04/native-track-b-memory.svg)

![Native Track B lifecycle latency, TypeDuck profile product path](./evidence/dashboard-visuals-2026-07-04/native-track-b-latency.svg)

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
- M57 macOS Track A verification repair:
  [`evidence/m57-macos-track-a-sentence-model-parity/`](./evidence/m57-macos-track-a-sentence-model-parity/).
