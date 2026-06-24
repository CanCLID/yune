# yune-web Upstream-Derived Source

This directory documents the pinned upstream TypeDuck-Web checkout used as the historical source shell for `yune-web`.

## Repository Information

- **Upstream URL**: <https://github.com/TypeDuck-HK/TypeDuck-Web.git>
- **Branch**: main
- **Commit SHA**: 03f9afd2cf6ca75653197f2193f24d1cd0adbd83
- **Commit Timestamp**: 2024-11-17 10:48:01 +0800
- **Clone Path**: apps/yune-web/source

## Clone/Refresh Commands

### Initial Clone

```bash
mkdir -p apps/yune-web
git clone https://github.com/TypeDuck-HK/TypeDuck-Web.git apps/yune-web/source
```

### Refresh Existing Checkout

```bash
git -C apps/yune-web/source fetch --tags --prune
git -C apps/yune-web/source checkout main
git -C apps/yune-web/source reset --hard origin/main
```

## Setup Command

The upstream-derived source uses Bun as its package manager and build tool. After cloning, install dependencies:

```bash
cd apps/yune-web/source
bun install
```

## Build/Run Commands

Upstream package.json defines these scripts:

- `bun run worker` — Build worker script (esbuild src/worker.ts --outdir=public)
- `bun run start` — Start development server (vite --host)
- `bun run build` — Build production bundle
- `bun run wasm` — Build WASM bridge (scripts/build_wasm.ts)

## Source Status

Clone completed successfully. Git status shows clean checkout at pinned commit.

## Yune Integration Notes

This upstream checkout is used for:

- Identifying the current librime/WASM seam before Yune patching
- Testing Yune adapter integration through real yune-web flows
- Documenting minimal source changes needed for Yune runtime bridge

The upstream source remains unpatched during Phase 10 Plan 01. Later plans will implement the seam replacement.

---
**Pinned**: 2026-05-05T15:03:00Z
**Plan**: 10-01 (Upstream TypeDuck-Web source handling)
