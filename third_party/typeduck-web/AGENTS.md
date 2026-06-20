# TypeDuck-Web Yune Integration Guide

This subtree is the internal browser demo/integration harness for Yune. The canonical source is the maintained patch in `patches/yune-typeduck-runtime.patch` plus the checked-out upstream app under `source/`.

It is not the shipping `TypeDuck-HK/TypeDuck-Web` product checkout, and it is not the reusable runtime package in `packages/yune-typeduck-runtime/`. Future product work may target a separate TypeDuck-Web clone, but M20 changes stay inside this harness and the Yune runtime bridge.

## Rules

- Keep TypeDuck-Web changes in the upstream app patch unless a file is intentionally Yune-owned (`yune-integration/`, `e2e/`, `README.yune-source.md`).
- Do not import from, re-pull, or edit a separately cloned `TypeDuck-HK/TypeDuck-Web` product checkout as part of this harness work.
- After editing `source/`, regenerate and reverse-check `patches/yune-typeduck-runtime.patch`.
- Preserve the native TypeDuck-Web fallback shape expected by the app: `Actions.processKey`, `stageAi`, candidate action methods, `customize`, `deploy`, and `setOption`.
- Use `customize()` for schema/deploy-time keys and `setOption()` for live session options. Do not add a new WASM export when the existing transport works.
- Every visible engine control must change candidate output, committed output, status output, or persisted config; display controls must change visible rendering. Do not expose `ascii_punct` as a working toggle until M18 implements the processor behavior.
- AI remains default-off, local-only, classic-first, and second-pass only. Do not move provider work into `processKey`.
- Browser validation must use real assets and committed Playwright/manual evidence.
