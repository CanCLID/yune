# M27 Browser Startup After

> **Status:** Captured after M27 optimization and release WASM rebuild - **Milestone:** M27 (TypeDuck-Web startup/runtime init) - **Updated:** 2026-06-22 - **Type:** evidence

Command:

```powershell
$env:YUNE_WEB_APP_URL='http://localhost:5173/web/'; $env:M27_EVIDENCE_LABEL='after'; npm.cmd --prefix apps\yune-web\e2e run test:e2e -- --grep "M27 PERF"
```

Captured output:

- `browser-startup-after-after.json`
- `control-classification-after-after.json`

## Startup

| Path | M26 Baseline | M27 After | Change |
|---|---:|---:|---:|
| Browser fresh `startup:complete.totalMs` | about `10.7s` | `5.680s` | about `-46.9%` |
| Browser reload `startup:complete.totalMs` | about `10.4s` | `5.466s` | about `-47.4%` |

The final browser evidence uses:

- `m27EvidenceVersion`: `m27-startup-v1`
- `assetVersion`: `m27-startup-v1`
- `wasmBuildProfile`: `release`
- `wasmBinary`: `yune-typeduck.wasm`

## Control Boundary

`control-classification-after-after.json` records:

- `AI Candidates`: local runtime only; actions `customize`, `stageAi`; no deploy phases; loading dataset `false`; loading indicator count `0`.
- `Auto-correction`: deploy-backed; actions `customize`, `deploy`; schema deploy phases recorded.
- `ASCII mode`: live option; `setOption` actions; no deploy phases.
- `Candidate Menu Layout`: browser-only; no worker actions; no loading indicator.

## WASM Rebuild Note

The first local Playwright run after the JavaScript worker change still used the old release WASM asset and measured about `11.3s`. The final evidence above was captured only after rebuilding `typeduck_web_module` for `wasm32-unknown-emscripten --release`, copying the generated `yune-typeduck.js` and `yune-typeduck.wasm` into the TypeDuck-Web public assets, and bumping the browser asset query string to `m27-startup-v1`.
