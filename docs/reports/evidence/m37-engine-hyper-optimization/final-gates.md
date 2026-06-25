# M37 Final Gates

Date: 2026-06-24

## Native Evidence

- Final native benchmark: `scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\m37-engine-hyper-optimization\phase-3-final-native -Iterations 5 -SessionIterations 20 -KeyIterations 20 -DeployProductBeforeBenchmark`
- Result: pass.
- Evidence: `phase-3-final-native/`
- Track B `hai`: M36 final `15,241.000us` median -> M37 final `8,336.800us` median.
- Track B product memory: M36 final peak `928,350,208 B` -> M37 final peak `504,377,344 B`.
- Selected product storage: `selected_storage=byte_backed`, `table_format=yune_no_marisa_compact`, `mapping_mode=mmap`, `source_fallback=false` for both `jyut6ping3` and `jyut6ping3_scolar`.
- `rsmarisa` probe: tried real `jyut6ping3` and `jyut6ping3_scolar` string-table payloads; both reported `rsmarisa_status=ok` and `rsmarisa_mapping_mode=mmap`.

## Rust Gates

- `cargo fmt --check`: pass.
- `cargo clippy --workspace --all-targets -- -D warnings`: pass.
- `cargo test --workspace`: pass. This covered upstream `luna_pinyin`, TypeDuck `jyut6ping3`, TypeDuck-Web, paging/selection, correction, prediction, learning, rich-comment, and TypeDuck-Windows boundary suites.
- `cargo test -p yune-core parses_compiled -- --nocapture`: pass, `5 passed`.
- Note: `cargo test -p yune-core compiled_payloads -- --nocapture` was attempted first and matched `0` tests in the current test names; it was superseded by the `parses_compiled` focused run above.

## Diff And Browser Gates

- M37 report visualization refresh: pass. Added `m37-product-latency-before-after.svg`, `m37-product-memory-before-after.svg`, `m37-track-a-latency-gap.svg`, and `m37-track-a-working-set-gap.svg`; XML parse and report-link checks passed.
- `git diff --check`: pass. Git emitted CRLF normalization warnings for `crates/yune-rime-api/Cargo.toml` and `crates/yune-rime-api/src/ffi_memory.rs`; no whitespace errors were reported.
- Runtime/browser gate: not run. M37 did not rebuild release WASM or run real browser proof, and the reports make no browser startup or typing claim.
