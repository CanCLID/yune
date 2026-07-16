# yune-web Integration Guide

This subtree is the canonical `yune-web` browser demo/integration harness for Yune. The tracked Vite app lives directly under `apps/yune-web/` (`src/`, `public/`, `index.html`, and app-local config). The historical upstream-derived checkout under `source/` is ignored and kept only as a reference snapshot.

The shell is derived from the historical `TypeDuck-HK/TypeDuck-Web` app with project-owner approval. It is not the shipping TypeDuck-Web product checkout, and it is not the reusable runtime package in `packages/yune-web-runtime/`. Future product work may target a separate TypeDuck-Web clone, but app/demo work in this repo should use `apps/yune-web`.

## Rules

- Keep yune-web source changes in the tracked app root. The active Yune runtime seam is `src/yune-integration/`; the top-level `yune-integration/` files are compatibility re-exports for older references.
- Do not import from, re-pull, or edit a separately cloned `TypeDuck-HK/TypeDuck-Web` product checkout as part of this harness work.
- Do not regenerate `patches/yune-web-runtime.patch` for normal app work; it is a retired migration baseline.
- Preserve the upstream-derived Actions fallback shape expected by the app: `Actions.processKey`, `stageAi`, candidate action methods, `customize`, `deploy`, and `setOption`.
- Use `customize()` for schema/deploy-time keys and `setOption()` for live session options. Do not add a new WASM export when the existing transport works.
- Every visible engine control must change candidate output, committed output, status output, or persisted config; display controls must change visible rendering. Do not expose `ascii_punct` as a working toggle until M18 implements the processor behavior.
- AI remains default-off, local-only, classic-first, and second-pass only. Do not move provider work into `processKey`.
- Browser validation must use real assets and committed Playwright/manual evidence.
- Changes to the worker, WASM/engine key path, launch schema assets, or public
  packaging must run the dedicated serial input-latency hard stop against the
  source-current built public artifact before production publish:
  `npm --prefix apps/yune-web/e2e run test:e2e:input-latency:public`. Keep its
  binding low-end defaults: 4x main-thread Chromium throttling plus synthetic
  4x proportional ASCII-letter `processKey` service-time amplification,
  self-verified on every timed key. The worker hook is loopback-only; this is a
  queue-stress profile, not empirical 4x-device proof. Preserve optional JSON
  evidence outside the tracked tree. Release evidence requires a clean Git tree
  and a recomputed full public-artifact inventory; the served worker, app bundle,
  WASM, and schema manifest must match it. The source-pinned GitHub release
  workflow is the maintenance release boundary: it runs this gate before any
  production upload, preserves every red, exercises the certified bytes through
  a Cloudflare preview canary, and promotes the same hashed archive to production.
  Cloudflare Git auto-builds are disabled before the first direct-upload
  workflow is pushed, and each upload must reassert that setting through the
  Pages API. A post-upload artifact check does not replace
  the binding pre-publish gate.
- For local-only yune-web tweaks, do not run lint, typecheck, unit tests, Playwright, or browser smoke by default. Run them only when the user explicitly asks for verification or when pushing changes; if changes are not being pushed, skip tests and lint.
