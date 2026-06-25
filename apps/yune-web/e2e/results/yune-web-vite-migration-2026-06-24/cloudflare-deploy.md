# Cloudflare Deploy Attempt

Date: 2026-06-24

## Target

- Project: `yune-web`
- Production URL: <https://yune-web.pages.dev>
- Account: `b8115041c042b125ed83f7aaf5f99373`

## Local Wrangler

- `npx wrangler --version`: `4.104.0`
- Initial `npx wrangler whoami`: failed because the local Wrangler token was expired and the environment was non-interactive.
- Retry `npx wrangler whoami`: passed after local auth refresh.

## API Connector Deployment Attempt

The Cloudflare API connector located the project and created a production
deployment using the Pages deployment API:

- Attempted deployment id: `05c64e7d-d6c0-427a-be6e-dc2b2e6ef5ed`
- Attempted deployment URL: <https://05c64e7d.yune-web.pages.dev>
- Initial Cloudflare status: `deploy` stage `success`

The deployment served HTTP 500 at both the production URL and immutable
deployment URL. The likely cause is that the connector-created deployment
accepted the manifest but did not complete the Pages asset upload/cache step
that Wrangler normally performs with the Pages upload JWT.

## Rollback

Production was rolled back immediately to the previous successful deployment:

- Restored deployment id: `363c6a01-d8a7-4a75-aec3-c1e3949107e8`
- Restored deployment URL: <https://363c6a01.yune-web.pages.dev>
- `curl -I https://yune-web.pages.dev/`: HTTP 200 after rollback

## Original Follow-up

Refresh local Wrangler auth with an interactive `wrangler login` or provide a
valid `CLOUDFLARE_API_TOKEN`, then deploy the already-built
`apps/yune-web/public-demo/dist` with:

```bash
npx wrangler pages deploy apps/yune-web/public-demo/dist --project-name yune-web --branch main
```

## Wrangler Retry

After local Wrangler auth was refreshed, the deploy succeeded with the explicit
account id because this login has access to two Cloudflare accounts:

```bash
CLOUDFLARE_ACCOUNT_ID=b8115041c042b125ed83f7aaf5f99373 \
  npx wrangler pages deploy apps/yune-web/public-demo/dist \
  --project-name yune-web --branch main
```

- Deployment URL: <https://bc0f80e7.yune-web.pages.dev>
- Upload result: `10 files` uploaded, `33` already uploaded.
- `curl -I https://yune-web.pages.dev/`: HTTP 200.
- `curl https://yune-web.pages.dev/build-info.json`: `builtAt`
  `2026-06-24T23:51:07.035Z`.
- `curl https://yune-web.pages.dev/yune-web.wasm`: HTTP 200,
  `application/wasm`.

## Runtime Compatibility Fix Redeploy

After deleting the duplicate Worker, the Pages site still served static assets,
but the browser worker failed to initialize the engine because the current app
expected `createYuneWebModule` / `yune_web_*` while the checked-in generated
WASM pair was still the older `createYuneTypeduckModule` / `yune_typeduck_*`
artifact. The fix keeps Pages as the hosting target and adds a temporary
compatibility bridge until a fresh Yune-named WASM artifact can be built.

- Deployment URL: <https://d50cda91.yune-web.pages.dev>
- `curl -I https://yune-web.pages.dev/`: HTTP 200.
- `curl https://yune-web.pages.dev/build-info.json`: `builtAt`
  `2026-06-25T00:09:45.948Z`.
- Real-browser production probe: typed `cak`, candidates appeared, loading
  cleared, and no console errors, page errors, request failures, or 4xx/5xx
  asset responses were captured.
