# yune-web UX redesign evidence

Date: 2026-06-24

Public URL: <https://yune-web.pages.dev>
Cloudflare Pages project: yune-web
Deployment: 363c6a01-d8a7-4a75-aec3-c1e3949107e8
Branch: main

## Build and patch gates

- PASS: `git -C apps\yune-web\source apply --reverse --check ..\patches\yune-web-runtime.patch`
- PASS: clean forward patch check from `apps\yune-web\source` HEAD into a temporary worktree
- PASS: `npm.cmd --prefix apps\yune-web\source run build`
- PASS: `powershell -NoProfile -ExecutionPolicy Bypass -File apps\yune-web\public-demo\build.ps1`
- PASS: final `dist` hash comparison matched `pages-upload-manifest.json`

## Test gates

- PASS: `cargo fmt --check`
- PASS: `cargo clippy --workspace --all-targets -- -D warnings`
- PASS: `cargo test --workspace`
- PASS: `cargo test -p yune-rime-api --test typeduck_web`
- PASS: `npm.cmd --prefix packages\yune-typeduck-runtime test`
- PASS: `npm.cmd --prefix packages\yune-typeduck-runtime run build`
- PASS: local preview `@public-smoke`: 4 passed
- PASS: deployed `@public-smoke` against `https://yune-web.pages.dev`: 4 passed

## Deployment evidence

- Asset upload summary: `pages-upload-summary.json`
- Uploaded manifest: `pages-upload-manifest.json`
- Cloudflare deployment stage: `deploy` / `success`
- Production URL tested: <https://yune-web.pages.dev>

## UX evidence

- Reference capture: `reference-yuen-harness.png`
- Light-mode implementation capture: `screenshot-local-light-reference-flow.png`
- Local smoke capture: `screenshot-local-ux-redesign-smoke.png`
- Deployed smoke capture: `screenshot-ux-redesign-smoke.png`
- Local smoke data: `local-ux-redesign-smoke.json`
- Deployed smoke data: `ux-redesign-smoke.json`

The redesigned public surface shows the primary header label `新韻輸入法引擎`
with visible canonical `yune-web` identity. The public build hides unsupported
schema and OpenCC controls, keeps AI off by default, records no blocked remote
telemetry calls, and reports honest runtime metrics (`off`, `N/A`, or measured
values only).
