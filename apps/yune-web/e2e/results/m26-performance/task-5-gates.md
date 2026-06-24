# M26 Task 5 Gate Evidence

> **Status:** Passed - **Milestone:** M26 (performance hardening) - **Updated:** 2026-06-22 - **Type:** evidence

Fresh closeout commands were rerun from `C:\Users\laubonghaudoi\Documents\GitHub\yune` after the benchmark harness, TypeDuck-Web diagnostics, dynamic-correction optimization, documentation, and patch regeneration changes were in place.

| Gate | Result |
| --- | --- |
| `cargo fmt --check` | Passed. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed. |
| `cargo test -p yune-core --test cantonese_parity` | Passed: 30 passed, 0 failed. |
| `cargo test -p yune-core --test upstream_luna_pinyin_parity` | Passed: 12 passed, 0 failed. |
| `cargo test -p yune-rime-api --test typeduck_web` | Passed: 25 passed, 0 failed. |
| `cargo test --workspace` | Passed: exit 0; the output included the full workspace suite, including the repeated 25-test native TypeDuck-Web adapter gate. |
| `cargo bench -p yune-rime-api --bench frontend_baselines` | Passed: exit 0. Cargo printed known output filename collision warnings for `yune_rime_api` artifacts, but the benchmark ran and emitted the M26 real-asset rows. |
| `npm.cmd --prefix packages/yune-typeduck-runtime test` | Passed: 5 files, 65 tests. |
| `npm.cmd --prefix packages/yune-typeduck-runtime run build` | Passed. |
| `npm.cmd --prefix apps/yune-web/source run build` | Passed. |
| `$env:M26_EVIDENCE_LABEL='after'; npm.cmd --prefix apps/yune-web/e2e run test:e2e -- --grep "M26 PERF" --workers=1` | Passed: 2 passed; refreshed `startup-attribution-after.json` and `typing-keydown-to-paint-after.json`. |
| TypeDuck-Web patch regeneration | Passed: regenerated `apps/yune-web/patches/yune-web-runtime.patch` with `git -C apps/yune-web/source diff HEAD --submodule=diff --binary --output=..\patches\yune-web-runtime.patch`. |
| TypeDuck-Web patch reverse check | Passed: `git apply --reverse --check ..\patches\yune-web-runtime.patch` from the patched source checkout. |
| TypeDuck-Web patch forward check | Passed: temporary clean worktree at the locked TypeDuck-Web revision `03f9afd2cf6ca75653197f2193f24d1cd0adbd83`, recursive submodule init/update, then `git apply --check` with the regenerated patch. The temporary worktree was removed. |
| `git diff --check` | Passed after this file was written. |

The native benchmark after-run includes the optimized dynamic-correction stress row `per_key_real_jyut6ping3_mobile_jigaajiusihaa_correction_engine_only` at median `122207.885us` and p95 `123790.292us` in this fresh run. The checked-in before/after summaries remain `native-before.md` and `native-after.md`; browser after evidence remains separate because those numbers include browser, WASM, worker, React, and paint-proxy latency rather than native engine latency.
