# M27 Task 5 Gates

> **Status:** Passed - **Milestone:** M27 (TypeDuck-Web startup/runtime init) - **Updated:** 2026-06-22 - **Type:** evidence

M27 closeout was verified on Windows after the startup optimization, worker evidence marker, AI-control loading fix, TypeDuck-Web patch regeneration, and release WASM rebuild.

## Results

| Gate | Result | Evidence |
|---|---|---|
| `cargo fmt --check` | PASS | Formatting check completed after M27 code changes. |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS | Workspace clippy passed after fixing the benchmark ignored-unit pattern. |
| `cargo test -p yune-core --test cantonese_parity` | PASS | 30 tests passed. |
| `cargo test -p yune-core --test upstream_luna_pinyin_parity` | PASS | 12 tests passed. |
| `cargo test -p yune-rime-api --test typeduck_web` | PASS | 25 tests passed. |
| `cargo test --workspace` | PASS | Workspace test suite passed, including the `typeduck_web` integration test. |
| `cargo bench -p yune-rime-api --bench frontend_baselines` | PASS | Clean rerun captured in `target/m27-frontend-baselines-after-final.txt`. |
| `npm.cmd --prefix packages/yune-typeduck-runtime test` | PASS | 5 files / 65 tests passed. |
| `npm.cmd --prefix packages/yune-typeduck-runtime run build` | PASS | TypeScript build passed. |
| `npm.cmd --prefix apps/yune-web/source run build` | PASS | Worker and Vite production build passed. |
| Focused M27 Playwright evidence | PASS | `browser-startup-after-after.json` and `control-classification-after-after.json`; 2 focused M27 tests passed. |
| TypeDuck-Web patch checks | PASS | Reverse and clean forward checks recorded in `patch-checks.md`. |
| `git diff --check` | PASS | Whitespace check passed after M27 archive/doc updates. |

## Closeout Notes

- Native startup attribution now records observable startup owners plus Windows working-set and peak-working-set metrics.
- The evidenced top owner, `spelling_algebra_expand`, was reduced by caching expanded code variants per original code before candidate materialization.
- Browser startup evidence improved from the M26 baseline of about `10.7s` fresh / `10.4s` reload to `5.680s` fresh / `5.466s` reload after the release WASM rebuild.
- AI Candidates no longer use the page-wide loading wrapper and do not emit deploy or runtime reinitialization markers; deploy-backed controls remain classified separately.
