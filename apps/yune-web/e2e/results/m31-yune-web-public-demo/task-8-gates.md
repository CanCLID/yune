# M31 Verification Gates

Date: 2026-06-24

Passed:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p yune-rime-api --test typeduck_web
npm.cmd --prefix packages\yune-typeduck-runtime test
npm.cmd --prefix packages\yune-typeduck-runtime run build
npm.cmd --prefix apps\yune-web\source run build
powershell -NoProfile -ExecutionPolicy Bypass -File apps\yune-web\public-demo\build.ps1
git diff --check
```

Focused local browser smoke:

```powershell
$env:YUNE_PUBLIC_DEMO_E2E='1'
$env:YUNE_WEB_APP_URL='http://127.0.0.1:8788'
$env:YUNE_WEB_EVIDENCE_DIR='../e2e/results'
npm.cmd --prefix apps\yune-web\e2e run test:e2e -- --grep "M31 PUBLIC"
```

Result: 3 passed.

Focused deployed browser smoke:

```powershell
$env:YUNE_PUBLIC_DEMO_E2E='1'
$env:YUNE_WEB_APP_URL='https://yune-web.pages.dev'
$env:YUNE_WEB_EVIDENCE_DIR='../e2e/results'
npm.cmd --prefix apps\yune-web\e2e run test:e2e -- --grep "M31 PUBLIC"
```

Result: 3 passed.

`yune-web` patch checks:

```powershell
git -C apps\yune-web\source apply --reverse --check ..\patches\yune-web-runtime.patch
git -C target\m31-yune-web-patch-check apply --check <repo>\apps\yune-web\patches\yune-web-runtime.patch
```

Result: both passed after initializing the clean checkout's `schema` submodule.

Production deploy:

- Pages project: `yune-web`.
- URL: `https://yune-web.pages.dev`.
- Deployment id: `d008e454-e852-4e62-ae80-5a2f5cb189f0`.
- Route: default Pages domain.
- Deploy method: Cloudflare connector API plus short-lived Pages upload token
  because local Wrangler remains unauthenticated.
