# yune-web Integration Guide

This subtree is the canonical `yune-web` browser demo/integration harness for Yune. The reproducible source state is the maintained patch in `patches/yune-web-runtime.patch`, the checked-out upstream-derived app under `source/`, and the checked-in public build/evidence files.

The shell is derived from the historical `TypeDuck-HK/TypeDuck-Web` app with project-owner approval. It is not the shipping TypeDuck-Web product checkout, and it is not the reusable runtime package in `packages/yune-typeduck-runtime/`. Future product work may target a separate TypeDuck-Web clone, but app/demo work in this repo should use `apps/yune-web`.

## Rules

- Keep yune-web source changes in the upstream-derived app patch unless a file is intentionally Yune-owned (`yune-integration/`, `e2e/`, `README.yune-source.md`, `public-demo/`).
- Do not import from, re-pull, or edit a separately cloned `TypeDuck-HK/TypeDuck-Web` product checkout as part of this harness work.
- After editing `source/`, regenerate and reverse-check `patches/yune-web-runtime.patch`.
- Preserve the upstream-derived Actions fallback shape expected by the app: `Actions.processKey`, `stageAi`, candidate action methods, `customize`, `deploy`, and `setOption`.
- Use `customize()` for schema/deploy-time keys and `setOption()` for live session options. Do not add a new WASM export when the existing transport works.
- Every visible engine control must change candidate output, committed output, status output, or persisted config; display controls must change visible rendering. Do not expose `ascii_punct` as a working toggle until M18 implements the processor behavior.
- AI remains default-off, local-only, classic-first, and second-pass only. Do not move provider work into `processKey`.
- Browser validation must use real assets and committed Playwright/manual evidence.
