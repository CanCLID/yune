# M30 Browser Evidence After Lever A

Date: 2026-06-22

## Fresh WASM Assets

The Rust change was rebuilt for browser evidence before the after-run.

Command:

```powershell
& 'C:\Program Files\Git\bin\bash.exe' -lc 'cd /c/Users/laubonghaudoi/Documents/GitHub/yune && export EMSDK=/c/Users/laubonghaudoi/Documents/GitHub/yune/target/emsdk && export EMSDK_NODE=/c/Users/laubonghaudoi/Documents/GitHub/yune/target/emsdk/node/22.16.0_64bit/bin/node.exe && export PATH=/c/Users/laubonghaudoi/Documents/GitHub/yune/target/emsdk/upstream/emscripten:/c/Users/laubonghaudoi/Documents/GitHub/yune/target/emsdk/upstream/bin:/c/Users/laubonghaudoi/Documents/GitHub/yune/target/emsdk/node/22.16.0_64bit/bin:$PATH && scripts/typeduck-wasm-build.sh'
```

Result: passed. The script verified native exports, built the `wasm32-unknown-emscripten` release module, verified the browser module smoke (`yune_typeduck_response_handled` plus `FS` write/read), and reported the release `yune-typeduck.js` / `yune-typeduck.wasm` artifacts. `wasm-opt` post-optimization was skipped because it could not validate this Emscripten module; this is a warning from the existing script path, not a build failure.

The generated release files were copied into the ignored local TypeDuck-Web public asset directory for the browser run:

- `apps/yune-web/source/public/yune-typeduck.js`
- `apps/yune-web/source/public/yune-typeduck.wasm`

These public asset copies are ignored local runtime inputs, not tracked deliverables.

## Browser Command

```powershell
$env:YUNE_WEB_APP_URL='http://localhost:5173/web/'; $env:M29_EVIDENCE_LABEL='m30-after'; npm.cmd --prefix apps\yune-web\e2e run test:e2e -- --grep "M29 PERF" --workers=1
```

Result: passed, `1` Playwright test.

Copied evidence:

- `browser-startup-after.json`
- `typing-keydown-to-paint-after.json`
- `typing-attribution-after.json`

## Browser Before/After

| Scenario | Before | After | Reading |
| --- | ---: | ---: | --- |
| Fresh startup | `5,256ms` | `5,321ms` | flat/slightly slower browser result |
| Reload startup | `5,096ms` | `5,179ms` | flat/slightly slower browser result |
| `hai` total p95 | `51ms` | `55ms` | native/worker moved `25ms` -> `29ms`; not an engine win claim |
| `longPhrase` total p95 | `71ms` | `73ms` | native/worker moved `46ms` -> `55ms`; browser/native attribution is noisy |
| `longComposition` total p95 | `50ms` | `50ms` | native/worker moved `35ms` -> `44ms` |
| `paging` total p95 | `5ms` | `5ms` | flat |
| `reverseLookup` total p95 | `14ms` | `14ms` | flat |

## Reading

The browser after-run is a regression guard, not the success metric for Lever A. Lever A's measured win is native memory pressure (`1,103,331,328` -> `838,209,536` after-ready bytes) and a smaller native startup reduction. Browser startup and typing stayed flat/noisy after fresh WASM rebuild, so M30 should not claim a browser latency win.
