# M55 Phase 2 Ratchet Gate 2 - Access-Path No-Go

This directory records the full M55 ratchet after bounded byte-backed poet
access-path work. The run is intentionally red and supports
`../access-path-followup-no-go-2026-07-03.md`.

Command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 `
  -OutputRoot docs\reports\evidence\m55-native-match-or-beat\phase-2-poet-storage\ratchet-gate-2-access-path-no-go `
  -Iterations 9 -SessionIterations 60 -KeyIterations 80 `
  -TrackAInputs "n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru" `
  -DeployProductBeforeBenchmark `
  -TrackAThresholds docs\reports\evidence\m55-native-match-or-beat\thresholds\m55-thresholds.csv `
  -FailOnRegression
```

Verdict: no-go. `threshold-check.csv` fails the 37-character Track A row, the
59-character Track A row, and the Track B product long-row latency guard. Track
A peak working set remains below the current memory ceiling.
