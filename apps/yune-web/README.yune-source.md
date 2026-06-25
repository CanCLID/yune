# yune-web Source Provenance

`apps/yune-web/` is now the tracked Vite + React harness for the Yune browser
demo. The historical TypeDuck-Web checkout remains useful only as provenance for
the app shell that was migrated here.

## Historical Upstream Pin

- **Upstream URL**: <https://github.com/TypeDuck-HK/TypeDuck-Web.git>
- **Branch**: main
- **Commit SHA**: 03f9afd2cf6ca75653197f2193f24d1cd0adbd83
- **Commit Timestamp**: 2024-11-17 10:48:01 +0800
- **Reference Path**: `apps/yune-web/source` when a local comparison checkout is
  needed

## Active Workflow

Install, build, and run from the app root:

```bash
npm --prefix apps/yune-web install
npm --prefix apps/yune-web run typecheck
npm --prefix apps/yune-web run build
npm --prefix apps/yune-web run start
```

The active runtime seam is `apps/yune-web/src/yune-integration/`. The retired
`apps/yune-web/patches/yune-web-runtime.patch` file is kept only as the migration
baseline from the old upstream-derived checkout.
