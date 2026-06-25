# yune-web Public Demo

`yune-web` is the public Yune browser demo built from the canonical tracked app
under `apps/yune-web/`. Public UI, deployment config, evidence, docs, and the
repo-owned app path use `yune-web`.

Build the deployable static artifact from checked-in Yune state:

```bash
npm --prefix apps/yune-web run build:public
```

The Windows-compatible wrapper runs the same build flow:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File apps\yune-web\public-demo\build.ps1
```

The script rebuilds `@yune-ime/yune-web-runtime`, bundles the worker with the
public-demo flag, runs the Vite public build, copies only the pinned public
schema assets listed in `schema-asset-manifest.json`, validates every SHA-256,
and writes `apps/yune-web/public-demo/dist/`.

Deploy with Wrangler Pages after the local preview and M31 evidence gates pass:

```powershell
npx.cmd wrangler pages deploy apps\yune-web\public-demo\dist --project-name yune-web --branch main
```

Cloudflare Pages Git integration uses the repository build script:

```bash
bash apps/yune-web/public-demo/cloudflare-pages-build.sh
```

Cloudflare project settings:

- Production branch: `main`
- Build command: `bash apps/yune-web/public-demo/cloudflare-pages-build.sh`
- Build output directory: `apps/yune-web/public-demo/dist`
- Root directory: repository root

GitHub Actions deploys require repository secrets:

- `CLOUDFLARE_ACCOUNT_ID`: Cloudflare account id that owns the `yune-web` Pages project.
- `CLOUDFLARE_API_TOKEN`: Cloudflare API token with Pages edit access for that account.

No Cloudflare account id, token, or secret belongs in this directory.

M31 deployed the public demo to:

<https://yune-web.pages.dev>

Production deploys are triggered automatically by pushes to `main`. Manual
Wrangler direct uploads are retained only as an emergency fallback.
