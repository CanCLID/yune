# M55 Phase 0 Baseline and Ratchet Attempt

Status: blocked / no-go at Phase 0 gate.

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

Baseline runs:

- `run-1/`
- `run-2/`
- `run-3/`

Ratchet artifacts:

- `noise-band.csv` records min/median/max/spread across the three baseline runs.
- `../thresholds/m55-thresholds.csv` is the M55 working threshold file attempted here. It imports the six M52 rows unchanged and adds:
  - three current win rows: `zhongguo`, `cszysmsrsd`, `zybfshmsru`;
  - startup/session absolute latency rows because their ratio bands exceeded the 10% stability target;
  - Track B product absolute regression rows for latency and memory.

Gate command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -TrackAInputs "n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru" -DeployProductBeforeBenchmark -TrackAThresholds docs\reports\evidence\m55-native-match-or-beat\thresholds\m55-thresholds.csv -FailOnRegression -OutputRoot docs\reports\evidence\m55-native-match-or-beat\phase-0-baseline\gate-run-1
```

Gate result:

- `gate-run-1/threshold-check.csv` failed.
- Failing inherited M52 rows:
  - `ceshiyixiachangjushuruxingnengzenyang`: observed `3.663x`, ceiling `3.267x`.
  - `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong`: observed `3.017x`, ceiling `2.447x`.
- Because M55 forbids optimization before the Phase 0 ratchet is green, Phase 1 was not started.
- Because the first real gate already failed, the synthetic-breach proof and second green gate run were not executed.

Conclusion:

M55 cannot proceed on this baseline without a reviewed plan decision. The current `main` baseline does not satisfy the inherited M52 long-input latency ceilings when run through the expanded M55 full-suite gate. Loosening those ceilings is explicitly forbidden by the active M55 plan, so this evidence records a Phase 0 no-go rather than a completed ratchet.
