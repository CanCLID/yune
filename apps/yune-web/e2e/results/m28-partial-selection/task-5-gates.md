# M28 Task 5 Gate Record

Date: 2026-06-22

Scope: M28 segment-aware TypeDuck partial selection closeout, after M27 startup/runtime-init closeout.

## Commands

- `cargo fmt --check` - pass.
- `cargo clippy --workspace --all-targets -- -D warnings` - pass.
- `cargo test -p yune-core --test upstream_luna_pinyin_parity` - pass, 12 tests.
- `cargo test -p yune-core --test cantonese_parity` - pass, 32 tests.
- `cargo test -p yune-rime-api --test typeduck_web` - pass, 26 tests.
- `cargo test --workspace` - pass.
- `cargo bench -p yune-rime-api --bench frontend_baselines` - pass. Latest M27/M28 closeout run kept `startup_real_jyut6ping3_mobile_runtime_ready` at about 6.28s median and `startup_trace_jyut6ping3_mobile_select_schema_total` at about 5.48s median.
- `npm.cmd --prefix packages/yune-typeduck-runtime test` - pass, 65 tests.
- `npm.cmd --prefix packages/yune-typeduck-runtime run build` - pass.
- `npm.cmd --prefix apps/yune-web/source run build` - pass.
- `npm.cmd --prefix apps/yune-web/e2e run test:e2e -- --grep "M27 PERF|M28 PARTIAL" --workers=1` with `YUNE_WEB_APP_URL=http://localhost:5173/web/` and `M27_EVIDENCE_LABEL=after-final` - pass, 3 browser tests.
- TypeDuck-Web patch regeneration from `apps/yune-web/source` - complete.
- TypeDuck-Web patch reverse check against current patched source - pass.
- TypeDuck-Web patch forward check against a clean `03f9afd` source worktree with the `schema` submodule initialized - pass.
- `git diff --check` - pass with Git line-ending warnings only.

## Evidence

- Oracle fixture: `crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-m28-partial-selection.json`.
- History classification: `apps/yune-web/e2e/results/m28-partial-selection/history-classification.md`.
- Oracle capture: `apps/yune-web/e2e/results/m28-partial-selection/oracle-capture.md`.
- Browser evidence: `apps/yune-web/e2e/results/m28-partial-selection/browser-evidence.md`.
- Browser JSON: `apps/yune-web/e2e/results/m28-partial-selection/browser-partial-selection.json`.

## Result

M28 is complete as a separate engine-correctness milestone. Selecting the prefix candidate `測` commits only the consumed span, keeps the remaining input composing, preserves FORK-PARITY-03 whole-sentence learning, and records browser evidence that the raw tail is not inserted.
