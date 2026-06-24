# M31 Cloudflare Smoke

Date: 2026-06-24

Current docs checked:

- Pages Direct Upload:
  <https://developers.cloudflare.com/pages/get-started/direct-upload/>
- Wrangler Pages commands:
  <https://developers.cloudflare.com/workers/wrangler/commands/pages/>
- Pages local development:
  <https://developers.cloudflare.com/pages/functions/local-development/>

Config selected:

- Target: Cloudflare Pages Direct Upload.
- Project: `yune-web`.
- Public URL: <https://yune-web.pages.dev>.
- Config file: `apps/yune-web/public-demo/wrangler.jsonc`.
- `pages_build_output_dir`: `./dist`.
- Compatibility date: `2026-06-24`.
- SPA fallback: Pages native SPA rendering; the deploy has no top-level
  `404.html`.

Wrangler:

- Version: 4.104.0.
- `wrangler whoami`: unauthenticated locally.
- Production deploy path: Cloudflare connector API plus short-lived Pages upload
  token, matching Wrangler Pages Direct Upload manifest behavior.

Local Pages preview:

```powershell
npx.cmd wrangler pages dev apps\yune-web\public-demo\dist --port 8788 --ip 127.0.0.1 --compatibility-date=2026-06-24
```

Result: passed; `GET /` returned 200.

Browser smoke:

- `M31 PUBLIC yune-web exposes only supported public controls @smoke`: passed.
- `M31 PUBLIC startup uses pruned public assets and warm cache @smoke`: passed.
- `M31 PUBLIC hk2s output standard is browser-visible and AI stays default-off @smoke`: passed.

Production deploy:

- Pages project: `yune-web`.
- URL: <https://yune-web.pages.dev>.
- Deployment id: `d008e454-e852-4e62-ae80-5a2f5cb189f0`.
- Environment: `production`.
- Final stage: `deploy` / `success`.

Production URL smoke:

- `GET /`: 200.
- `GET /schema-asset-manifest.json`: 200.
- `GET /yune-typeduck.wasm`: 200.

Deployed browser smoke:

```powershell
$env:YUNE_PUBLIC_DEMO_E2E='1'
$env:YUNE_WEB_APP_URL='https://yune-web.pages.dev'
$env:YUNE_WEB_EVIDENCE_DIR='../e2e/results'
npm.cmd --prefix apps\yune-web\e2e run test:e2e -- --grep "M31 PUBLIC"
```

Result: 3 passed.

No Cloudflare token, upload token, account id, or secret is committed.
