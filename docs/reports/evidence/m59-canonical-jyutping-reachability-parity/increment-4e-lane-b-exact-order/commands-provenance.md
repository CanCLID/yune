# Commands and provenance

All commands ran from clean source commit
`5879405c7b0f76af4dca7382f00b3e0605386f2c`. Build and measurement roots were
outside the tracked repository.

## Exact behavior gates

```powershell
$env:CARGO_TARGET_DIR='C:\m59-4e-target'
cargo test -p yune-rime-api --test yune_web m59_lane_b_product_matches_complete_pinned_librime_order -- --nocapture
cargo test -p yune-rime-api --test yune_web m59_luna_long_sentence_page_order_matches_pinned_oracle_on_byte_backed_product -- --nocapture
cargo test -p yune-rime-api --test yune_web m59_correction_oracle_source_and_compiled_deploy_paths_match_complete_order -- --nocapture
```

Lane B is bound to fixture SHA-256
`8ff369e1a78d0865055003433a258dcf4407136609d4ab8ab5a029c29b989273`.
The test reads the pinned fixture rather than copying or regenerating expected
candidate strings.

## Native WEB-04

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/verify-m59-web04-native.ps1 `
  -OutputRoot 'C:\m59-4e-web04-5879405c' `
  -WorkRoot 'C:\m59-4e-web04-clean-work' `
  -RepoRoot 'C:\Users\laubonghaudoi\Documents\GitHub\yune\' `
  -CargoTargetDir 'C:\m59-4e-release-target' `
  -ModelPath '<external pinned zh-hant-t-essay-bgw.gram>' `
  -ExpectedSourceCommit '5879405c7b0f76af4dca7382f00b3e0605386f2c'
```

Runner SHA-256:
`27547ec0a847477cd94adf45628790deb7a494a021dfec51d0cb536a3fc9a430`.

## Native performance

Each run used `scripts/benchmark-native-rime-inprocess.ps1`, the pinned oracle
root, the source-current release DLL, product deployment, iterations `9/60/80`,
the 17-input Track A list, and the declared Track B product row. The untouched
exact invocation for each round is preserved in that run's
`actual-invocation.txt` and `environment.txt`.

Run 1 built the harness. Runs 2-5 supplied:

```text
-PrebuiltNativeBenchmarkExecutable C:\m59-4e-perf-work-5879405c\run1\cargo-target\release\deps\native_inprocess_benchmark-5c1184aa464f2e15.exe
-PrebuiltNativeBenchmarkReceipt C:\m59-4e-perf-5879405c\run1\native-benchmark-build-receipt.txt
```

Aggregation command:

```powershell
python scripts/aggregate-native-ratchet.py `
  --thresholds docs/reports/evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv `
  --expected-runs 5 `
  --run C:\m59-4e-perf-5879405c\run1 `
  --run C:\m59-4e-perf-5879405c\run2 `
  --run C:\m59-4e-perf-5879405c\run3 `
  --run C:\m59-4e-perf-5879405c\run4 `
  --run C:\m59-4e-perf-5879405c\run5 `
  --output C:\m59-4e-perf-5879405c\aggregate
```

No executable, DLL, grammar model, or compiled/deployed asset is included in
this packet. Their identities are recorded by SHA-256 only.
