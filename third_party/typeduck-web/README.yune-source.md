# TypeDuck-Web Upstream Source

This directory contains a pinned checkout of the upstream TypeDuck-Web repository for Yune integration testing.

## Repository Information

- **Upstream URL**: https://github.com/TypeDuck-HK/TypeDuck-Web.git
- **Branch**: main
- **Commit SHA**: 03f9afd2cf6ca75653197f2193f24d1cd0adbd83
- **Commit Timestamp**: 2024-11-17 10:48:01 +0800
- **Clone Path**: third_party/typeduck-web/source

## Clone/Refresh Commands

### Initial Clone

```bash
mkdir -p third_party/typeduck-web
git clone https://github.com/TypeDuck-HK/TypeDuck-Web.git third_party/typeduck-web/source
```

### Refresh Existing Checkout

```bash
git -C third_party/typeduck-web/source fetch --tags --prune
git -C third_party/typeduck-web/source checkout main
git -C third_party/typeduck-web/source reset --hard origin/main
```

## Setup Command

The upstream TypeDuck-Web uses Bun as its package manager and build tool. After cloning, install dependencies:

```bash
cd third_party/typeduck-web/source
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
- Testing Yune adapter integration through real TypeDuck-Web flows
- Documenting minimal source changes needed for Yune runtime bridge

The upstream source remains unpatched during Phase 10 Plan 01. Later plans will implement the seam replacement.

---
**Pinned**: 2026-05-05T15:03:00Z
**Plan**: 10-01 (Upstream TypeDuck-Web source handling)