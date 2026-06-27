# Reframed Comparison Review Snapshot

Date: 2026-06-27

Purpose: review the updated comparison framing after the My RIME Jyutping
confound was removed. Only `luna_pinyin` is treated as a fair cross-engine
comparison. Jyutping rows are Yune-only guard/correctness evidence.

## Native Track A Snapshot

Evidence: [`native-track-a/`](./native-track-a/)

Command:

```powershell
cargo build --release -p yune-rime-api
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot C:\Users\laubonghaudoi\Documents\GitHub\yune\docs\reports\evidence\reframed-comparison-review-2026-06-27\native-track-a -Iterations 9 -SessionIterations 60 -KeyIterations 80 -TrackAInputs n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru -TrackBInputs neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung
```

The wrapper command timed out after five minutes in Codex, but the native
benchmark process completed and wrote a complete `summary.csv`.

Track B caveat: this native run was Track-A-focused and did not pass
`-DeployProductBeforeBenchmark`. Its Track B rows are invalid product evidence:
`product_path_status.csv` records `compiled_ready=false`,
`selected_storage=unavailable`, and `source_fallback=true` for both
`jyut6ping3` dictionaries. The resulting `1,049,112,576 B` peak and
`510,925,748 B` `translator.entries_by_code` owner are source-YAML fallback
artifacts, not the byte-backed product path. The valid Track B native memory
snapshot remains M46's `504,627,200 B`, `source_fallback=false` run.

Read:

- Yune remains faster on seven Track A rows and slower on `hao`, `n`, and `ni`.
- Fresh short-key ratios: `hao 2.199x`, `n 3.534x`, `ni 3.698x`.
- Fresh faster rows: `zhongguo 0.363x`, `cszysmsrsd 0.437x`,
  `zybfshmsru 0.628x`, 59-character pinyin `0.700x`, startup `0.788x`,
  session `0.778x`, 37-character pinyin `0.959x`.
- Fresh peak memory: Yune `128,364,544 B`; same-run librime rows
  `14,004,224-17,989,632 B`.
- Do not cite this run's Track B native rows as product memory or owner
  evidence.

## Native Track B Clean Product Snapshot

Evidence: [`native-track-b-clean/`](./native-track-b-clean/)

Command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot C:\Users\laubonghaudoi\Documents\GitHub\yune\docs\reports\evidence\reframed-comparison-review-2026-06-27\native-track-b-clean -Iterations 9 -SessionIterations 60 -KeyIterations 80 -TrackAInputs ni -TrackBInputs h,ha,hai,hau,nei,ngo,neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung -DeployProductBeforeBenchmark
```

Read:

- Valid product path: `compiled_ready=true`, `selected_storage=byte_backed`,
  table/prism `mmap`, and `source_fallback=false` for both `jyut6ping3`
  dictionaries.
- Peak Track B working set: `504,676,352 B`; peak pagefile proxy:
  `536,322,048 B`.
- Steady Track B working set rows: `427,171,840-440,688,640 B`.
- Short rows: `h 1675.500 us`, `ha 1143.100 us`, `hai 778.000 us`,
  `hau 805.067 us`, `nei 397.267 us`, `ngo 581.400 us`.
- 50+ guard: `32.385 us`.

This clean rerun confirms M46's byte-backed Track B memory conclusion and
replaces the invalid Track B rows from `native-track-a/` for current product
Track B discussion.

## Browser Comparator Snapshot

Evidence:
[`../../../../apps/yune-web/e2e/results/reframed-comparison-review-2026-06-27/browser-comparator/`](../../../../apps/yune-web/e2e/results/reframed-comparison-review-2026-06-27/browser-comparator/)

Command:

```powershell
$env:YUNE_WEB_COMPARATOR_BASELINE='1'
$env:YUNE_WEB_COMPARATOR_INCLUDE_MY_RIME='1'
$env:YUNE_WEB_COMPARATOR_SAMPLES='7'
$env:YUNE_WEB_COMPARATOR_RESULT_ROOT='reframed-comparison-review-2026-06-27'
$env:YUNE_WEB_COMPARATOR_PHASE='browser-comparator'
npm.cmd --prefix apps\yune-web\e2e run test:e2e -- --grep "YUNE WEB COMPARATOR" --workers=1
```

Read:

- Fair Luna memory comparison: Yune-web `160.0 MiB`, My RIME `16.0 MiB`.
- Jyutping guard: Yune-web `893.1 MiB`; My RIME `68.0 MiB` remains
  Cantonese-only guard context, not a target floor.
- The generated comparator summary now includes `comparisonLane` with
  `fair comparison` for `luna_pinyin` and `guard` for Jyutping.

## Jyutping Guard Snapshot

Commands:

```powershell
cargo test -p yune-core --test cantonese_parity
$env:YUNE_WEB_JYUTPING_MEMORY_ATTRIBUTION='1'
npm.cmd --prefix apps\yune-web\e2e run test:e2e -- --grep "M46 JYUTPING MEMORY" --workers=1
```

Results:

- `cantonese_parity`: pass, `37` tests.
- Browser correctness evidence:
  [`../../../../apps/yune-web/e2e/results/yune-web-jyutping-memory-attribution/phase-0-current-runtime/`](../../../../apps/yune-web/e2e/results/yune-web-jyutping-memory-attribution/phase-0-current-runtime/)
- Clean Jyutping, Cangjie -> Luna -> Jyutping, and Jyutping -> Luna -> Jyutping
  all pass with `0` worker action errors and `893.1 MiB` max observed WASM.
