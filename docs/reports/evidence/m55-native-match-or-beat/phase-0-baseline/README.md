# M55 Phase 0 Baseline and Ratchet

Status: green after null-grammar fast-path repair.

Date: 2026-07-03.

Baseline:

- Repository: `C:\Users\laubonghaudoi\Documents\GitHub\yune`
- Branch: `main`
- Starting commit: `b05caede`
- `origin/main` fetched before execution; local `main` was already up to date.
- Required docs read before execution:
  - `AGENTS.md`
  - `docs/conventions.md`
  - `docs/roadmap.md`
  - `docs/plans/active/m55-plan-native-track-a-match-or-beat-program.md`

Oracle/product prerequisites:

- Reused local librime 1.17.0 oracle binary at `target/upstream-oracle/1.17.0/extract/dist/lib/rime.dll`.
- Populated ignored local upstream schema cache under `target/upstream-oracle/1.17.0/schema-src/` at:
  - `rime/rime-luna-pinyin` `18a80335c37522311f7cff02886cd81cec3b460a`
  - `rime/rime-prelude` `082425ea0684bca36474415d4a0e8db9b016487e`
  - `rime/rime-essay` `48c7538f0b760fcc8c9d6bf08711f82cfbd2e9ed`
  - `rime/rime-stroke` `3a4b0f4013e2b4c14b1e80c92b1d4723eb65f39c`
- Ran `powershell -ExecutionPolicy Bypass -File scripts\capture-upstream-luna-pinyin.ps1 -OracleRoot target\upstream-oracle\1.17.0` to provision `rime-shared` and `rime-user/build`.
- That provisioning rewrote existing upstream fixture files; those fixture changes were restored because they were not intentional M55 evidence.
- Product assets were present, so Track B was measured. `-SkipTrackB` was not used.

Baseline command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -TrackAInputs "n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru" -DeployProductBeforeBenchmark -OutputRoot docs\reports\evidence\m55-native-match-or-beat\phase-0-baseline\run-N
```

Initial baseline runs:

- `run-1/`
- `run-2/`
- `run-3/`

Ratchet artifacts:

- `noise-band.csv` records min/median/max/spread across the repaired fixed-path
  baseline runs used to refresh the M55-added rows.
- `../thresholds/m55-thresholds.csv` is the M55 working threshold file attempted here. It imports the six M52 rows unchanged and adds:
  - three current win rows: `zhongguo`, `cszysmsrsd`, `zybfshmsru`;
  - startup/session latency rows, using an absolute latency row where the ratio
    band exceeded the 10% stability target;
  - Track B product absolute regression rows for latency and memory.

Gate command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -TrackAInputs "n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru" -DeployProductBeforeBenchmark -TrackAThresholds docs\reports\evidence\m55-native-match-or-beat\thresholds\m55-thresholds.csv -FailOnRegression -OutputRoot docs\reports\evidence\m55-native-match-or-beat\phase-0-baseline\gate-run-1
```

Initial gate result:

- `gate-run-1/threshold-check.csv` failed.
- Failing inherited M52 rows:
  - `ceshiyixiachangjushuruxingnengzenyang`: observed `3.663x`, ceiling `3.267x`.
  - `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong`: observed `3.017x`, ceiling `2.447x`.
- Diagnosis: M54 had routed plain null-grammar Luna through grammar-aware
  sentence state. That preserved candidate output but added unnecessary grammar
  context maintenance and duplicate-text scans to the default-off path.
- Fix: restore the old null-grammar sentence path while keeping grammar-aware
  context/deduplication for Octagram-backed paths.

Diagnostic commands:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\m55-native-match-or-beat\phase-0-diagnostics\m52-shape-current-main -Iterations 9 -SessionIterations 60 -KeyIterations 80 -TrackAInputs n,ni,hao,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong -SkipTrackB -TrackAThresholds docs\reports\evidence\m52-track-a-guardrails-and-disposition\track-a-thresholds.csv -FailOnRegression
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\m55-native-match-or-beat\phase-0-diagnostics\m52-shape-null-grammar-fast-path -Iterations 9 -SessionIterations 60 -KeyIterations 80 -TrackAInputs n,ni,hao,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong -SkipTrackB -TrackAThresholds docs\reports\evidence\m52-track-a-guardrails-and-disposition\track-a-thresholds.csv -FailOnRegression
```

Diagnostic result:

- `phase-0-diagnostics/m52-shape-current-main/` failed the inherited M52 gate
  on the same long rows.
- `phase-0-diagnostics/m52-shape-null-grammar-fast-path/` passed the inherited
  M52 gate after restoring the null-grammar fast path.

Synthetic-breach proof:

- Threshold copy:
  `synthetic-breach-null-grammar-fast-path/m55-thresholds-synthetic-breach.csv`.
- Run:
  `synthetic-breach-null-grammar-fast-path/run-1/threshold-check.csv`.
- Expected result: non-zero exit with one intentional failure for each
  supported kind:
  - `latency_ratio`
  - `memory_peak`
  - `latency_absolute_us`
  - `memory_absolute_bytes`

Final green gates:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -TrackAInputs "n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru" -DeployProductBeforeBenchmark -TrackAThresholds docs\reports\evidence\m55-native-match-or-beat\thresholds\m55-thresholds.csv -FailOnRegression -OutputRoot docs\reports\evidence\m55-native-match-or-beat\phase-0-baseline\gate-run-5b-null-direct-take
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -TrackAInputs "n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru" -DeployProductBeforeBenchmark -TrackAThresholds docs\reports\evidence\m55-native-match-or-beat\thresholds\m55-thresholds.csv -FailOnRegression -OutputRoot docs\reports\evidence\m55-native-match-or-beat\phase-0-baseline\gate-run-6-null-direct-take
```

Final gate result:

- `gate-run-5b-null-direct-take/threshold-check.csv`: all rows pass.
- `gate-run-6-null-direct-take/threshold-check.csv`: all rows pass.
- Track B product assets were present and measured in both final gates.
- `-SkipTrackB` was not used.

Conclusion:

Phase 0 is now green. The earlier no-go remains as diagnostic evidence, but it
is superseded by the null-grammar fast-path repair plus the two final green
full-suite gates above. Phase 1 may begin from this ratchet; no later M55 phase
has started in this evidence bundle.
