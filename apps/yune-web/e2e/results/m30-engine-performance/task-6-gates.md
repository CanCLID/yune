# M30 Task 6 Gates

Date: 2026-06-22

## Commands

| Command | Result |
| --- | --- |
| `cargo fmt --check` | Passed. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed. |
| `cargo test -p yune-core --test upstream_luna_pinyin_parity` | Passed before final gate set: `12 passed`. Covered again by `cargo test --workspace`. |
| `cargo test -p yune-core --test cantonese_parity -- m28_followup` | Passed before final gate set: `3 passed`. Covered again by `cargo test --workspace`. |
| `cargo test -p yune-rime-api --test typeduck_web` | Passed after final Lever A patch via the WASM-build fallback run: `28 passed`. Covered again by `cargo test --workspace`. |
| `cargo test --workspace` | Passed. Includes the slow TypeDuck-Web real-assets test file: `typeduck_web.rs` reported `28 passed`; core library unit/integration output included `290 passed, 1 ignored`; other workspace suites passed. |
| `cmd /c "cargo bench -p yune-rime-api --bench frontend_baselines > target\m30-frontend-baselines-final.txt 2>&1"` | Passed. Cargo printed the known `yune_rime_api` output filename collision warnings and exited 0. |
| `npm.cmd --prefix packages/yune-typeduck-runtime test` | Passed: `5` test files, `65` tests. |
| `npm.cmd --prefix packages/yune-typeduck-runtime run build` | Passed. |
| `npm.cmd --prefix apps/yune-web/source run build` | Passed. Vite production build and worker bundle completed. |
| `& 'C:\Program Files\Git\bin\bash.exe' -lc 'cd /c/Users/laubonghaudoi/Documents/GitHub/yune && export EMSDK=/c/Users/laubonghaudoi/Documents/GitHub/yune/target/emsdk && export EMSDK_NODE=/c/Users/laubonghaudoi/Documents/GitHub/yune/target/emsdk/node/22.16.0_64bit/bin/node.exe && export PATH=/c/Users/laubonghaudoi/Documents/GitHub/yune/target/emsdk/upstream/emscripten:/c/Users/laubonghaudoi/Documents/GitHub/yune/target/emsdk/upstream/bin:/c/Users/laubonghaudoi/Documents/GitHub/yune/target/emsdk/node/22.16.0_64bit/bin:$PATH && scripts/typeduck-wasm-build.sh'` | Passed. Native exports verified; release `wasm32-unknown-emscripten` module built; JS glue scan/export fallback passed; browser module smoke verified one `yune_typeduck_*` export plus Emscripten `FS` write/read. Existing script skipped `wasm-opt` because it could not validate this Emscripten module. |
| `$env:YUNE_WEB_APP_URL='http://localhost:5173/web/'; $env:M29_EVIDENCE_LABEL='m30-after'; npm.cmd --prefix apps\yune-web\e2e run test:e2e -- --grep "M29 PERF" --workers=1` | Passed: `1` Playwright test. Evidence copied to `browser-startup-after.json`, `typing-keydown-to-paint-after.json`, and `typing-attribution-after.json`. |
| `git diff --check` | Passed. Git printed CRLF-to-LF working-copy warnings for docs files only. |

## Final Benchmark Snapshot

Final benchmark output: `target/m30-frontend-baselines-final.txt`.

| Metric | M30 before | Final gate |
| --- | ---: | ---: |
| Single-startup after-ready bytes | `1,103,331,328` | `839,217,152` |
| Single-startup peak bytes | `1,123,745,792` | `1,026,129,920` |
| `startup_real_jyut6ping3_mobile_runtime_ready` median | `6,242,614.900us` | `6,120,732.800us` |
| `startup_trace_jyut6ping3_mobile_spelling_algebra_expand` median | `5,015,352us` | `4,879,981us` |

The final benchmark confirms the memory win. Startup and per-key rows remain noisier than memory measurements, so M30 does not claim a browser latency or typing win.

## Browser Evidence

Browser after evidence used freshly rebuilt release WASM assets copied into the ignored local TypeDuck-Web public directory. The checked-in evidence is:

- `browser-after.md`
- `browser-startup-before.json`
- `browser-startup-after.json`
- `typing-keydown-to-paint-before.json`
- `typing-keydown-to-paint-after.json`
- `typing-attribution-before.json`
- `typing-attribution-after.json`

Browser before/after stayed flat/noisy:

| Scenario | Before | After |
| --- | ---: | ---: |
| Fresh startup | `5,256ms` | `5,321ms` |
| Reload startup | `5,096ms` | `5,179ms` |
| `hai` total p95 | `51ms` | `55ms` |
| `longPhrase` total p95 | `71ms` | `73ms` |
| `longComposition` total p95 | `50ms` | `50ms` |
| `paging` total p95 | `5ms` | `5ms` |
| `reverseLookup` total p95 | `14ms` | `14ms` |

## Patch Discipline

No tracked files under `apps/yune-web/source/` were edited. The release `yune-typeduck.js` and `yune-typeduck.wasm` files were copied into the ignored local `source/public/` directory only so Playwright could measure fresh Rust code. Because no tracked TypeDuck-Web source patch changed, `apps/yune-web/patches/yune-web-runtime.patch` did not need regeneration or reverse/forward checks for M30.
