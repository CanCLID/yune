# M55 Phase 2 Access-Path Follow-Up No-Go - 2026-07-03

Verdict: Phase 2 remains partial/no-go. The byte-backed poet access path was
improved, but the full ratchet is still red. No Phase 3 or Phase 4 work has
started from this state.

## Follow-Up Change Under Test

The retained-heap poet payload owners were already moved to `mmap_file_backed`
in the earlier Phase 2 slice. This follow-up tested bounded access-path work
allowed by the Phase 2 no-go clause:

- byte-backed vocabulary prefix checks read chars lazily from the validated
  `YUNE-POET/1` bytes instead of materializing the full char vector per row;
- byte-backed phrase-code derivation advances by input byte offset instead of
  repeatedly checking the full accumulated prefix string;
- accepted vocabulary text and weight are read once per accepted row, then
  reused for emitted graph edges;
- vocabulary index lists are cached only within one graph rebuild, keyed by the
  input code slice. This is transient request-local scratch, not a retained heap
  index.

Rejected transient experiments:

- retained hot cache under 1 MB: worse because synchronization dominated the
  hot path;
- single-pass derive-without-prefix-gate: worse because misses did too much
  derivation work;
- early batched vocabulary view including text: worse because it decoded text
  before the prefix gate rejected rows.

## Evidence

Primary follow-up run:

```text
docs/reports/evidence/m55-native-match-or-beat/phase-2-poet-storage/ratchet-gate-2-access-path-no-go/
```

Exact gate command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 `
  -OutputRoot docs\reports\evidence\m55-native-match-or-beat\phase-2-poet-storage\ratchet-gate-2-access-path-no-go `
  -Iterations 9 -SessionIterations 60 -KeyIterations 80 `
  -TrackAInputs "n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru" `
  -DeployProductBeforeBenchmark `
  -TrackAThresholds docs\reports\evidence\m55-native-match-or-beat\thresholds\m55-thresholds.csv `
  -FailOnRegression
```

The gate exited non-zero. `threshold-check.csv` records:

| Row | Observed | Ceiling | Result |
| --- | ---: | ---: | --- |
| `ceshiyixiachangjushuruxingnengzenyang` | `5.618x` | `3.267x` | fail |
| `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` | `3.870x` | `2.447x` | fail |
| Track B product long row | `387.059 us` | `375.253 us` | fail |

Passing guarded rows in the same run:

| Row | Observed | Ceiling |
| --- | ---: | ---: |
| `n` | `2.745x` | `3.050x` |
| `ni` | `2.979x` | `3.223x` |
| `hao` | `2.075x` | `2.287x` |
| `zhongguo` | `0.265x` | `0.325x` |
| `cszysmsrsd` | `0.480x` | `0.532x` |
| `zybfshmsru` | `0.698x` | `0.770x` |
| startup ratio | `0.928x` | `1.101x` |
| session absolute | `23697.500 us` | `25533.310 us` |
| Track A peak working set | `110440448 B` | `198000000 B` |

Compared with `ratchet-gate-1`, the two long Track A Yune medians improved, but
not enough to satisfy the gate:

| Row | Gate 1 Yune median | Gate 2 Yune median |
| --- | ---: | ---: |
| 37-char row | `1818.973 us` | `1637.727 us` |
| 59-char row | `2912.697 us` | `2581.449 us` |

The Track B product latency guard remains red and was worse in this same-run
gate (`378.449 us` in gate 1, `387.059 us` in gate 2).

## Decision

This is not a Phase 2 green result. No memory ceiling is tightened, no latency
ceiling is loosened, and M55 is not closed. The follow-up establishes that
reasonable Phase 2 access-path work improved the long Track A medians but did
not make byte-backed poet storage compatible with the current latency ratchet.

Per the active plan, later graph constant-factor or short-key index phases must
not be started from this state without an explicit follow-up decision that
acknowledges this Phase 2 partial/no-go.
