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
refetch/hash the served worker, app bundle, WASM, and schema manifest. The
single-worker Chromium matrix uses 4x main-thread CPU throttling plus
loopback-only, synthetic
4x proportional ASCII-letter `processKey` service-time amplification,
self-verified on every timed key. This is a queue-stress profile rather than
empirical 4x-device proof. It covers the historical long Jyutping inputs, Luna
37/59 inputs, every public schema, and a TypeDuck row restored from browser
persistence after reload. The binding defaults are p95 `<= 750 ms`,
max `<= 1000 ms`, a sustained 250 ms key interval, and zero schema/split-part/
manifest requests during each timed typing window after its selected schema
reaches ready. The binding build-plus-gate entrypoint is:

```bash
bash apps/yune-web/public-demo/cloudflare-pages-build.sh
```

Calling `build:public` and the inner latency runner separately is diagnostic
unless the caller supplies the same exact toolchain and receipt environment as
that entrypoint; an ambient-toolchain artifact cannot produce a release pass.

The pre-publish run also exercises the reported 47-key Jyutping input at an
unamplified 100 ms cadence. It binds every exact prefix, six-row candidate-page
shape, p95 `<= 150 ms`, max `<= 250 ms`, and max worker queue wait `<= 100 ms`.
It does not bind candidate text/order because no external oracle fixture exists
for that exact input. A delayed host timer never causes a short catch-up burst;
the original out-of-range gap stays red and blocks publication. The runner does
not retry a measured red. The 4x and exact 1x measurements use independent
failure semantics in the same single-worker run. The exact-input canary runs
first, so a later 4x red cannot suppress its receipt; any red still makes the
build fail. Latency-gate failure logs retain gzip/base64 chunks of the complete
JSON receipts plus hashes of their exact bytes.

Set `YUNE_WEB_LATENCY_OUTPUT_DIR` to preserve its JSON packet outside the
tracked tree. Rust/WASM changes must first rebuild and copy the source-current
Emscripten artifacts as the release build below does. The release workflow
builds with pinned Rust `1.96.1`, Emscripten `4.0.23`, and the Emscripten SDK's
Node `22.16.0`, rejects ambient Rust compiler flags, requires that exact
toolchain receipt, and fails before production upload on any red row. A manual
deployed diagnostic sets both
`YUNE_WEB_APP_URL=https://yune-web.pages.dev/` and
`YUNE_WEB_EXPECTED_SOURCE_COMMIT` to the full deployed commit before running
`test:e2e:input-latency`; routine releases instead run the canary against the
candidate preview, then verify exact source and artifact hashes on production.

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

Deploy with Wrangler Pages only from a certified artifact:

```powershell
npx.cmd wrangler pages deploy apps\yune-web\public-demo\dist --project-name yune-web --branch main
```

The compatibility entrypoint reproduces the pinned build and binding gate
locally:

```bash
bash apps/yune-web/public-demo/cloudflare-pages-build.sh
```

The deployment-maintenance path is
`.github/workflows/deploy-yune-web.yml`. It runs on every `main` push so a
documentation-only change receives an explicit successful no-op. A
release-affecting change is built and measured once without secrets, sealed as
a SHA-256-addressed archive, direct-uploaded to a preview, exercised by the
source-pinned deployed canary, and then direct-uploaded unchanged to production.
The final production check compares source, toolchain, manifest, WASM, schema,
and runtime hashes with that archive. Measured reds have zero retries and never
reach production.

Cloudflare project settings:

- Production branch: `main`
- Migration interlock: automatic production and preview deployments are
  disabled; each credentialed upload checks the Pages API and stops if either
  setting is re-enabled
- Production delivery: Wrangler direct upload from the source-pinned GitHub
  workflow

GitHub Actions deploys use the provisioned, branch-restricted
`yune-web-preview` and `yune-web-production` environments, each with these
environment secrets:

- `CLOUDFLARE_ACCOUNT_ID`: Cloudflare account id that owns the `yune-web` Pages project.
- `CLOUDFLARE_API_TOKEN`: Cloudflare API token with Pages edit access for that account.

No Cloudflare account id, token, or secret belongs in this directory.

M31 deployed the public demo to:

<https://yune-web.pages.dev>

WEB03-11 closed at clean source `ef485b10`: the exact build entrypoint passed
the binding 8-scenario / 186-key gate, the Git-integrated Pages deployment
succeeded, and the source-pinned production canary passed. The public runtime
keeps the status header but disables hidden diagnostics polling and excludes
the development cockpit from the typing path.

After migration activation, production releases are triggered automatically by
release-affecting pushes to `main`. A manual workflow run is allowed only as an
explicitly named premeasurement setup retry; a measured red is never rerun or
discarded. Manual Wrangler uploads remain an emergency fallback and must use an
already-certified archive.
