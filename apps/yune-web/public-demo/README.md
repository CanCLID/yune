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

Before deploying a worker, engine, schema, or schema-delivery change, build the
source-current artifact and run the focused WEB-03 browser latency hard stop.
The runner starts its own exact `public-demo/dist` preview and rejects a dev
server or artifact without source/hash-bearing `build-info.json`. It also proves
the split Jyutping prism startup path. Release runs reject a dirty Git tree,
fully reconcile `public-demo/dist` to its deterministic artifact inventory, and
refetch/hash the served worker, app bundle, WASM, and schema manifest. The serial
Chromium matrix uses 4x main-thread CPU throttling plus loopback-only, synthetic
4x proportional ASCII-letter `processKey` service-time amplification,
self-verified on every timed key. This is a queue-stress profile rather than
empirical 4x-device proof. It covers the historical long Jyutping inputs, Luna
37/59 inputs, every public schema, and a TypeDuck row restored from browser
persistence after reload. The binding defaults are p95 `<= 750 ms`,
max `<= 1000 ms`, a sustained 250 ms key interval, and zero schema/split-part/
manifest requests during each timed typing window after its selected schema
reaches ready:

```powershell
npm.cmd --prefix apps/yune-web run build:public
npm.cmd --prefix apps/yune-web/e2e run test:e2e:input-latency:public
```

The pre-publish run also exercises the reported 47-key Jyutping input at an
unamplified 100 ms cadence. It binds every exact prefix, six-row candidate-page
shape, p95 `<= 150 ms`, max `<= 250 ms`, and max worker queue wait `<= 100 ms`.
It does not bind candidate text/order because no external oracle fixture exists
for that exact input. A delayed host timer never causes a short catch-up burst;
the original out-of-range gap stays red and blocks publication. The runner does
not retry a measured red.

Set `YUNE_WEB_LATENCY_OUTPUT_DIR` to preserve its JSON packet outside the
tracked tree. Rust/WASM changes must first rebuild and copy the source-current
Emscripten artifacts as the Cloudflare build below does. The Cloudflare build
runs this gate after packaging, requires its exact pinned-toolchain receipt, and
fails before publish on any red row. A direct post-deploy canary must set both
`YUNE_WEB_APP_URL=https://yune-web.pages.dev/` and
`YUNE_WEB_EXPECTED_SOURCE_COMMIT` to the full deployed commit before running
`test:e2e:input-latency`; it does not replace the build gate.

## Launch Assets

The public demo launch schema set is `jyut6ping3_mobile`, `cangjie5`, and
`luna_pinyin`. WEB-03 made the compiled-asset contract explicit: the public
schema manifest and worker asset lists include current `.table.bin`,
`.reverse.bin`, and `.prism.bin` payloads for the launch schemas and Jyutping
helper dictionaries, with launch prisms at `Rime::Prism/4.0`.

Current WEB-03 browser evidence shows the shipped Jyutping launch/full path
byte-backs and peaks at `160.0 MiB`; the old `893.1 MiB` row is retained only as
a synthetic no-launch-assets negative control. Evidence:
[`../../../docs/reports/evidence/web03-three-schema-launch-readiness/`](../../../docs/reports/evidence/web03-three-schema-launch-readiness/).

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
