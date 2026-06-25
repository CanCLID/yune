# yune-web Integration Layer

This directory contains the Yune-owned integration layer that bridges the
yune-web browser harness to the repository-owned
`@yune-ime/yune-web-runtime` package.

## Components

- `adapter.ts` owns the active `YuneWebRuntime` lifecycle, translates browser
  worker actions to the runtime API, and maps runtime responses back to the
  app's `RimeResult` shape.
- `assets.ts` loads the explicit schema, dictionary, OpenCC, and deployed-build
  assets from `apps/yune-web/public/schema/`.
- `response.ts` translates `YuneWebResponse` data into the app-visible result
  contract.

## Active Build Flow

From the repository root:

```bash
npm --prefix packages/yune-web-runtime run build
npm --prefix apps/yune-web install
npm --prefix apps/yune-web run build
```

For a fresh browser runtime artifact, build the Emscripten pair and copy it into
the app public directory:

```bash
scripts/yune-web-wasm-build.sh
cp target/wasm32-unknown-emscripten/release/yune-web.js apps/yune-web/public/
cp target/wasm32-unknown-emscripten/release/yune-web.wasm apps/yune-web/public/
```

`apps/yune-web/public/yune-web.js`, `apps/yune-web/public/yune-web.wasm`,
`apps/yune-web/public/worker.js`, and build output are generated artifacts and
stay out of git.

## Contract Notes

- Runtime resource identifiers are logical asset IDs, not arbitrary filesystem
  paths.
- `customize()` handles schema/deploy-time keys; `setOption()` handles live
  session options.
- The AI pass remains default-off, local-only, and second-pass over classic
  candidates.
- The top-level `apps/yune-web/yune-integration/` directory is retained only as
  compatibility re-exports for older repo references.
