# M46 Phase 0 Provenance

Date: 2026-06-27

## Repository

| Field | Value |
| --- | --- |
| Branch | `main` |
| Measurement commit | `47d136d4d57a8c88984b9802f2f1f19eacf49dca` |
| M45 commit | `fd192793c027cf0ce9c7830b789feef675072676` |
| WEB-01 commit | `47d136d4d57a8c88984b9802f2f1f19eacf49dca` |
| Plan | `docs/plans/active/m46-plan-jyutping-native-wasm-memory-attribution.md` |

The worktree was intentionally dirty for M46 instrumentation and evidence
capture. Pre-existing M45/WEB-01 report edits remained in place and were not
swept into any commit during Phase 0.

## Host

| Field | Value |
| --- | --- |
| Host | `DESKTOP-OHT2K15` |
| OS | `Microsoft Windows NT 10.0.26200.0` |
| PowerShell | `5.1.26100.8655` |
| Node | `v24.16.0` |
| npm | `11.13.0` |
| rustc | `rustc 1.96.0 (ac68faa20 2026-05-25)` |
| cargo | `cargo 1.96.0 (30a34c682 2026-05-25)` |
| Playwright Chromium | `149.0.7827.55` |

## Serialized Run Order

Native and browser memory benchmarks were not run concurrently.

1. Rust instrumentation tests and `cargo build --release -p yune-rime-api`.
2. Native Track B baseline into `phase-0-native/`.
3. `apps/yune-web` typecheck and production/public builds.
4. Browser single-schema WASM benchmark.
5. Browser asset-family attribution benchmark.
6. Browser schema-switch correctness capture.

The pre-browser process snapshot showed existing user/app Chrome and Node
processes on the machine, but no M46 native benchmark process was still running.

## Commands

```powershell
cargo build --release -p yune-rime-api
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot C:\Users\laubonghaudoi\Documents\GitHub\yune\docs\reports\evidence\m46-jyutping-native-wasm-memory-attribution\phase-0-native -Iterations 9 -SessionIterations 60 -KeyIterations 80 -TrackBInputs h,ha,hai,hau,nei,ngo,neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung -DeployProductBeforeBenchmark
```

```powershell
npm.cmd --prefix apps/yune-web run typecheck
npm.cmd --prefix apps/yune-web run build
npm.cmd --prefix apps/yune-web run build:public
```

```powershell
$env:YUNE_WEB_WASM_HEAP_BENCHMARK='1'
$env:YUNE_WEB_BENCHMARK_RESULT_ROOT='yune-web-jyutping-memory-attribution'
$env:YUNE_WEB_BENCHMARK_PHASE='phase-0-post-web01-single-schema'
$env:YUNE_WEB_BENCHMARK_SAMPLES='3'
npm.cmd --prefix apps/yune-web/e2e run test:e2e -- --grep "YUNE WEB WASM HEAP" --workers=1
```

```powershell
$env:YUNE_WEB_WASM_ATTRIBUTION='1'
$env:YUNE_WEB_WASM_ATTRIBUTION_RESULT_ROOT='yune-web-jyutping-memory-attribution\asset-family'
$env:YUNE_WEB_WASM_ATTRIBUTION_PHASE='phase-0-post-web01-asset-family'
npm.cmd --prefix apps/yune-web/e2e run test:e2e -- --grep "YUNE WEB WASM ATTRIBUTION" --workers=1
```

```powershell
$env:YUNE_WEB_JYUTPING_MEMORY_ATTRIBUTION='1'
$env:YUNE_WEB_JYUTPING_MEMORY_RESULT_ROOT='yune-web-jyutping-memory-attribution'
$env:YUNE_WEB_JYUTPING_MEMORY_PHASE='phase-0-schema-switch-current'
npm.cmd --prefix apps/yune-web/e2e run test:e2e -- --grep "M46 JYUTPING MEMORY" --workers=1
```
