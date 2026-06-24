# M31 Delivery And Cache Evidence

Date: 2026-06-24

Local preview URL: `http://127.0.0.1:8788/`
Deployed URL: `https://yune-web.pages.dev`

Focused browser smoke:

```powershell
$env:YUNE_PUBLIC_DEMO_E2E='1'
$env:YUNE_WEB_APP_URL='http://127.0.0.1:8788'
$env:YUNE_WEB_EVIDENCE_DIR='../e2e/results'
npm.cmd --prefix apps\yune-web\e2e run test:e2e -- --grep "M31 PUBLIC"
```

Result: 3 passed.

The same focused smoke passed against the deployed Cloudflare URL:

```powershell
$env:YUNE_PUBLIC_DEMO_E2E='1'
$env:YUNE_WEB_APP_URL='https://yune-web.pages.dev'
$env:YUNE_WEB_EVIDENCE_DIR='../e2e/results'
npm.cmd --prefix apps\yune-web\e2e run test:e2e -- --grep "M31 PUBLIC"
```

Result: 3 passed.

Evidence files:

- `startup-assets-cache.json`
- `screenshot-startup-assets-cache.png`

Observed behavior:

- Startup marker includes `m31EvidenceVersion: "m31-yune-web-public-demo-v1"`.
- Startup marker includes `publicDemo: true`.
- Loaded shared assets exclude `cangjie`, `loengfan`, `10keys`, and
  `longpress`.
- Cache Storage warm reload records asset cache hits:
  `31` hits and `0` misses on the deployed warm reload evidence.

Claim boundary:

This evidence proves public delivery pruning and warm-cache reuse. It does not
claim a browser startup or typing speed win.

Measured caveat:

The current `jyut6ping3_mobile` schema still requires Luna and scholar lookup
assets for the public behavior it exposes. M31 prunes non-public schema
families and uses content-addressed cache keys, but does not rewrite the schema
to defer every reverse-lookup dependency until first backtick input.
