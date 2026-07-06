# M59 Canonical Jyutping Reachability Parity Evidence

This bundle is for M59 only. Acceptance is canonical-first:

- Canonical Jyutping: Yune loaded with pinned upstream `rime/rime-cantonese` versus upstream `rime/librime 1.17.0` with the same pinned `rime-cantonese` data.
- Upstream Luna: Yune `luna_pinyin` versus upstream `rime/librime 1.17.0`.
- TypeDuck profile output is regression-guard only and is not used as the acceptance oracle.

## Phase 0

Baseline reproduction:

- `phase-0-baseline-ratchet-run1/` reproduced the red M55 ratchet after `c4336cd9`.
- Regression attribution: bounded Luna refresh was probing strict lookup prefixes even when prefix fallback was disabled; long TypeDuck/profile prefix fallback also inherited the wider short-input reachability cap.

Current-code ratchet proof after the typed-profile marker change:

- `phase-0-restored-ratchet-run8/` passed.
- `phase-0-restored-ratchet-run9/` passed.
- `phase-0-restored-ratchet-run7/` is retained as a failed noisy rerun: only the 59-character Luna Track A row exceeded the threshold (`1.683` observed versus `1.625` ceiling).

All three runs used:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot <run-dir> -Iterations 9 -SessionIterations 60 -KeyIterations 80 -TrackAInputs n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru -TrackBInputs neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung -DeployProductBeforeBenchmark -TrackAThresholds docs\reports\evidence\m55-native-match-or-beat\thresholds\m55-thresholds.csv -FailOnRegression
```

## Phase 1

Canonical Jyutping upstream capture:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\capture-upstream-rime-cantonese.ps1 -Output docs\reports\evidence\m59-canonical-jyutping-reachability-parity\phase-1\canonical-rime-cantonese-capture.json -Inputs bei,beingo,zijiguk,neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung -ReportedCaseInput zijiguk
```

The capture records:

- `rime/librime 1.17.0` at `33e78140250125871856cdc5b42ddc6a5fcd3cd4`.
- `rime/rime-cantonese` at `c99b16e44d2df77a5cb8fb0867dd2bab7a112cb0`.
- `rime.dll` SHA-256 `86b4c7357d4c6d293ce5589b234d8859ca2ac30923a03bedfa3926eeaf97fb0b`.
- `rime_deployer.exe` SHA-256 `3abb72b5bb56fcafcfe925d533ae5f832c68d5a0bc9952fd0eea0682fb1ab071`.

Yune canonical-loadability check:

- Staged pinned upstream rime-cantonese data under `target/m59-canonical-yune-load/shared`.
- Replaced only the staged `default.yaml` with a Yune-facing `schema_list` containing `jyut6ping3`, to avoid unrelated upstream default schemas that were not copied into this focused lane.
- The staged and deployed `jyut6ping3.schema.yaml` files have no `yune/profile` marker.
- `yune-canonical-rime-cantonese-load-bei.json` was captured with:

```powershell
cargo run -p yune-cli -- frontend --shared-data-dir target\m59-canonical-yune-load\shared --user-data-dir target\m59-canonical-yune-load\user --schema jyut6ping3 --sequence bei --output json
```

Dictionary provenance check:

- Pinned upstream/staged `jyut6ping3.dict.yaml` SHA-256: `4301001FB7BB52D5D1A9C032C519AC18BA50677E926E01006E34A48788385EFA`.
- Shipped web `apps/yune-web/public/schema/jyut6ping3.dict.yaml` SHA-256: `B0ABF4FBCBF18B8CF05F4689EA05A12BE6CF301F29A7316699E2B1AB9C24D172`.
- Conclusion: the shipped web dictionary is not the pinned canonical oracle data; M59 canonical diffing uses the staged pinned upstream lane.

Upstream Luna M59 captures:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\capture-upstream-luna-pinyin.ps1 -ScenarioInput docs\reports\evidence\m59-canonical-jyutping-reachability-parity\phase-1\luna-pinyin-m59-scenarios.json -Output docs\reports\evidence\m59-canonical-jyutping-reachability-parity\phase-1\upstream-luna-pinyin-m59-basic.json -SentenceExpandedOutput docs\reports\evidence\m59-canonical-jyutping-reachability-parity\phase-1\upstream-luna-pinyin-m59-sentence-expanded.json
```

The raw scenario snapshots are committed as `phase-1/upstream-luna-pinyin-m59-scenario-snapshots.json`.

## Phase 2

`phase-2/canonical-pre-fix-diff.json` is the frozen pre-fix diff. Raw Yune captures are committed beside it.

Named pre-fix gaps:

- Canonical `beingo`: upstream reaches `畀` at page 0/index 3 and `匕` at page 6/index 4; Yune did not reach either target in the captured pages.
- Canonical `zijiguk`: upstream has `諮議局` first and `諮` at page 45/index 2; Yune did not reach either target in the captured pages.
- Luna `ziyiju`: upstream reaches `諮` at page 5/index 3; Yune did not reach it in the captured pages.
- Luna `moboyi`: upstream selection flow commits `莫伯洢`; Yune committed `脈搏一`.

