# M55 Phase 1 Native Track A Memory Attribution

Date: 2026-07-03

Status: green.

## Inputs

- Track A schema: `luna_pinyin`
- Shared data: `target/upstream-oracle/1.17.0/rime-shared`
- Prebuilt data: `target/upstream-oracle/1.17.0/rime-user/build`
- Probe sequence: `nihao`
- Baseline release owner profile: `../phase-0-baseline/gate-run-6-null-direct-take/memory-owner-profile.csv`

## Commands

```powershell
$shared=(Resolve-Path 'target\upstream-oracle\1.17.0\rime-shared').Path
$build=(Resolve-Path 'target\upstream-oracle\1.17.0\rime-user\build').Path
$env:YUNE_MEM_SCHEMA='luna_pinyin'
$env:YUNE_MEM_DEFAULT='luna_pinyin'
$env:YUNE_MEM_SHARED_DATA_DIR=$shared
$env:YUNE_MEM_PREBUILT_BUILD_DIR=$build
$env:YUNE_MEM_SKIP_DEPLOY='1'
$env:YUNE_MEM_SEQUENCE='nihao'
$env:YUNE_MEM_EVIDENCE_DIR='docs\reports\evidence\m55-native-match-or-beat\phase-1-attribution\native-memory-probe-luna'
cargo test -p yune-rime-api --test native_memory_probe -- --ignored --exact native_memory_probe_reports_working_set --nocapture
```

Attempted release-mode allocator probe:

```powershell
cargo test --release -p yune-rime-api --test native_memory_probe -- --ignored --exact native_memory_probe_reports_working_set --nocapture
```

Result: blocked. The release crate graph is built with `panic=abort`, while the ignored test harness requires `panic=unwind`. The allocator rows are therefore diagnostic classification evidence, not a release benchmark ceiling.

## Attribution

`attribution-summary.json` records:

- Phase 0 unclassified lower-bound proxy: `106,039,183 B`
- Diagnostic attributed bytes for that prior floor: `103,380,578 B`
- Coverage: `97.49%`
- Steady working set in the probe: `178,417,664 B`
- Peak working set in the probe: `193,585,152 B`
- Named heap owner bytes: `75,037,086 B`
- Named mmap-backed bytes: `13,044,872 B`
- Allocator live heap not in owner rows: `71,729,000 B`
- Resident process/runtime overhead: `18,606,706 B`
- Transient peak delta: `15,167,488 B`

`owner-budget.csv` is the Phase 1 budget table. The largest confirmed byte-backable owners remain:

- `poet.vocabulary`: `53,644,752 B`
- `poet.entries_by_code`: `18,694,662 B`

The top Phase 2b candidate is `allocator.live_heap_not_in_owner_rows` (`71,729,000 B`), but it must be split into concrete subowners before any reduction is claimed.

## Target Revision

The original provisional `60,000,000 B` Tier M memory bar is not supported by the named owners. The revised Tier M memory bar is `<=125,000,000 B` peak working set:

```text
188,600,320 B phase-0 peak
-72,370,289 B byte-backable poet owners
=116,230,031 B projected post-Phase-2 peak before mmap residency/remnant overhead
```

Phase 2b can target `<=110,000,000 B` only after concrete subowner evidence exists for allocator-live bytes or the transient peak.

## Phase Gate

The Phase 1 release ratchet gate passed:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 `
  -TrackAInputs "n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru" `
  -DeployProductBeforeBenchmark `
  -OutputRoot docs\reports\evidence\m55-native-match-or-beat\phase-1-attribution\ratchet-gate-1 `
  -TrackAThresholds docs\reports\evidence\m55-native-match-or-beat\thresholds\m55-thresholds.csv `
  -FailOnRegression
```

Result: `ratchet-gate-1/threshold-check.csv` has `23` pass rows and no failures. The inherited 59-character row remains tight: observed `2.447x`, ceiling `2.447x`.
