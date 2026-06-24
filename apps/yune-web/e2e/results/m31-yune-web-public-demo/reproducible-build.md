# M31 Reproducible Build Evidence

Date: 2026-06-24

Command:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File apps\yune-web\public-demo\build.ps1
```

Result: passed.

The build script:

- Rebuilt `packages/yune-typeduck-runtime`.
- Bundled `source/src/worker.ts` with `YUNE_PUBLIC_DEMO_BUILD=true`.
- Built the Vite app with `VITE_YUNE_PUBLIC_DEMO=1`.
- Generated `apps/yune-web/public-demo/dist/`.
- Pruned the deployed schema directory to the checked-in
  `schema-asset-manifest.json`.
- Validated SHA-256 for every pinned schema asset.

Pinned schema payload:

- Files: 30.
- Bytes: 29,414,157.

Wrangler Pages command checked:

```powershell
npx.cmd wrangler pages deploy --help
```

Result: passed; Wrangler 4.104.0 exposes `--project-name`, `--branch`,
`--commit-hash`, `--commit-message`, and `--commit-dirty` for Pages deploys.

Production deploy:

- URL: `https://yune-web.pages.dev`.
- Pages project: `yune-web`.
- Deployment id: `d008e454-e852-4e62-ae80-5a2f5cb189f0`.
- Deploy method: Cloudflare connector API plus short-lived Pages upload token
  using the same Pages Direct Upload manifest shape as Wrangler, because local
  Wrangler was unauthenticated.
