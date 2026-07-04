# M55 Final Verification Ratchet - No-Go

This directory records the fresh M55 closeout ratchet run on current code. The run is intentionally red and supports `../partial-closeout-2026-07-04.md`.

Command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 `
  -OutputRoot docs\reports\evidence\m55-native-match-or-beat\final\verification-ratchet-no-go-2026-07-04 `
  -Iterations 9 -SessionIterations 60 -KeyIterations 80 `
  -TrackAInputs "n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru" `
  -DeployProductBeforeBenchmark `
  -TrackAThresholds docs\reports\evidence\m55-native-match-or-beat\thresholds\m55-thresholds.csv `
  -FailOnRegression
```

Verdict: no-go. `threshold-check.csv` fails the 37-character Track A row, the 59-character Track A row, and the Track B product long-row latency guard. Track A peak working set remains below the current M55 memory ceiling, but no M55 threshold is handed over because the latency ratchet is red.
