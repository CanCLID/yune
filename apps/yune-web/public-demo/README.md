# yune-web Public Demo

`yune-web` is the public Yune browser demo built from the canonical
`apps/yune-web/source/` upstream-derived harness. Public UI, deployment config,
evidence, docs, and the repo-owned app path use `yune-web`.

Build the deployable static artifact from checked-in Yune state:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File apps\yune-web\public-demo\build.ps1
```

The script rebuilds `@yune-ime/typeduck-runtime`, bundles the worker with the
public-demo flag, runs the Vite public build, copies only the pinned public
schema assets listed in `schema-asset-manifest.json`, validates every SHA-256,
and writes `apps/yune-web/public-demo/dist/`.

Deploy with Wrangler Pages after the local preview and M31 evidence gates pass:

```powershell
npx.cmd wrangler pages deploy apps\yune-web\public-demo\dist --project-name yune-web --branch main
```

No Cloudflare account id, token, or secret belongs in this directory.

M31 deployed the public demo to:

<https://yune-web.pages.dev>

Local Wrangler was unauthenticated during closeout, so the production deploy was
performed through the installed Cloudflare connector plus a short-lived Pages
upload token using the same Pages Direct Upload manifest shape as Wrangler.
