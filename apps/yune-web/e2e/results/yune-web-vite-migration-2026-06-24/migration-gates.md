# yune-web Vite Migration Gates

Date: 2026-06-24

## Passed

| Gate | Result |
| --- | --- |
| `npm --prefix packages/yune-typeduck-runtime test` | PASS, 5 files / 65 tests |
| `npm --prefix packages/yune-typeduck-runtime run build` | PASS |
| `npm --prefix apps/yune-web install` | PASS, with npm audit reporting 1 moderate and 1 high advisory |
| `npm --prefix apps/yune-web run typecheck` | PASS |
| `npm --prefix apps/yune-web run build` | PASS |
| `npm --prefix apps/yune-web run build:public` | PASS, built `apps/yune-web/public-demo/dist` and validated pinned schema hashes |
| Browser readiness probe against `http://127.0.0.1:5174/` | PASS |
| `YUNE_WEB_APP_URL=http://127.0.0.1:5174 npm --prefix apps/yune-web/e2e run test:e2e -- --grep "M25 DOGFOOD-11" --workers=1` | PASS, proves row-level inline definitions render for dictionary-backed candidates |

## Blocked Or Failed

| Gate | Result |
| --- | --- |
| `scripts/typeduck-wasm-build.sh` | BLOCKED: local Rust toolchain was initially absent; after installing a minimal stable toolchain and target, the script hung because `emcc -dumpversion` did not return. Existing generated `yune-typeduck.js` and `yune-typeduck.wasm` from the reference checkout were copied into `apps/yune-web/public/` for build and browser validation. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File apps/yune-web/public-demo/build.ps1` | NOT RUN: neither `powershell` nor `pwsh` is installed on this macOS environment. The equivalent dependency-free Node build script `npm --prefix apps/yune-web run build:public` passed. |
| `YUNE_WEB_APP_URL=http://127.0.0.1:5174 npm --prefix apps/yune-web/e2e run test:e2e:smoke` | FAIL: 4 passed before failures. Failures were legacy broad smoke cases: M20 learned prediction candidate did not appear, M16 hit a WASM allocation abort, and M22 timed out switching to Cangjie after Luna. |
| `YUNE_PUBLIC_DEMO_E2E=1 YUNE_WEB_APP_URL=https://yune-web.pages.dev npm --prefix apps/yune-web/e2e run test:e2e -- --grep @public-smoke --workers=1` | NOT PASSED: first run was interrupted because the attempted deployment returned HTTP 500. Production was rolled back before completing the smoke. |
