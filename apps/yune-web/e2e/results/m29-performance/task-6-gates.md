# M29 Task 6 Gates

Date: 2026-06-22

## Rust

| Command | Result |
| --- | --- |
| `cargo fmt --check` | Passed. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed. |
| `cargo test -p yune-core --test upstream_luna_pinyin_parity` | Passed: 12 passed, 0 failed. |
| `cargo test -p yune-core --test cantonese_parity` | Passed: 35 passed, 0 failed, finished in 62.45s. |
| `cargo test -p yune-rime-api --test typeduck_web` | Passed: 28 passed, 0 failed, finished in 906.92s. |
| `cargo test --workspace` | Passed. The slow `typeduck_web` portion passed 28/28 in 903.33s; `yune-rime-api` unit coverage passed 290/290 with 1 ignored test. |

## Frontend Benchmark

Command:

```powershell
cmd /c "cargo bench -p yune-rime-api --bench frontend_baselines > target\m29-frontend-baselines-final.txt 2>&1"
```

Result: passed. Output: `target/m29-frontend-baselines-final.txt`.

Final key rows:

- `m29_single_startup_memory_jyut6ping3_mobile`: total `6,321.367ms`; before `4,739,072` bytes; after-ready `1,103,249,408` bytes; after-finalize `12,763,136` bytes; peak `1,124,024,320` bytes.
- `startup_real_jyut6ping3_mobile_runtime_ready`: median `6,279,927.300us`; p95 `6,343,815.200us`.
- `startup_trace_jyut6ping3_mobile_select_schema_total`: median `5,507,246us`; p95 `5,517,379us`; peak `1,791,193,088` bytes.
- `startup_trace_jyut6ping3_mobile_translator_install`: median `5,377,347us`; p95 `5,389,670us`.
- `startup_trace_jyut6ping3_mobile_spelling_algebra_expand`: median `5,049,599us`; p95 `5,080,961us`; working-set delta `606,056,448` bytes.
- `per_key_real_jyut6ping3_mobile_hai_full_abi`: median `15,886.967us`; p95 `17,403.400us`.
- `per_key_real_jyut6ping3_mobile_jigaajiusihaa_full_abi`: median `22,348.077us`; p95 `23,731.277us`.

Known stderr: Cargo repeated the existing output filename collision warnings for `yune_rime_api` release artifacts; the benchmark still completed successfully.

## TypeScript And Browser

| Command | Result |
| --- | --- |
| `npm.cmd --prefix packages/yune-typeduck-runtime test` | Passed: 5 files, 65 tests, 207ms. |
| `npm.cmd --prefix packages/yune-typeduck-runtime run build` | Passed. |
| `npm.cmd --prefix apps/yune-web/source run build` | Passed: Vite build completed in 685ms; `public/worker.js` built at 33.5KB. |
| `$env:YUNE_WEB_APP_URL='http://localhost:5173/web/'; $env:M29_EVIDENCE_LABEL='after'; npm.cmd --prefix apps\yune-web\e2e run test:e2e -- --grep "M29 PERF" --workers=1` | Passed: 1 Playwright test, 17.3s. |

M29 browser evidence files:

- `browser-startup-before.json`
- `browser-startup-after.json`
- `typing-keydown-to-paint-before.json`
- `typing-keydown-to-paint-after.json`
- `typing-attribution-before.json`
- `typing-attribution-after.json`

## TypeDuck-Web Patch And Diff Hygiene

| Check | Result |
| --- | --- |
| Regenerated `apps/yune-web/patches/yune-web-runtime.patch` from `apps/yune-web/source` with `--binary --submodule=diff`. | Passed; only expected line-ending warnings. |
| `git -C apps\yune-web\source apply --reverse --check ..\patches\yune-web-runtime.patch` | Passed. |
| Clean detached worktree plus `git apply --check apps/yune-web/patches/yune-web-runtime.patch` after `schema` submodule init. | Passed. |
| `git diff --check` | Passed; only expected line-ending notices for `docs/requirements.md` and `docs/roadmap.md`. |

## Closeout Notes

- M29 kept candidate ordering, Space/default-confirm behavior, comments, and ABI surfaces out of scope.
- The optimization is native-startup-owner evidence: browser startup and typing evidence are recorded as flat/mixed attribution, not as win claims.
