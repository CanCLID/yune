# WEB-05 Control Surface Closeout

Date: 2026-07-05

Scope: `apps/yune-web` only. WEB-05 did not change `crates/` or
`packages/yune-web-runtime/`.

## Evidence Files

- `controls-ledger.md`: 108-row raw control/diagnostic inventory, including 14
  `surface` rows with explicit public-demo posture and 2 deferred rows.
- `yune-web-spec-rebaseline-audit.md`: separates WEB-05 selector/helper
  maintenance from behavior assertion changes in `yune-web.spec.ts`.
- `default-behavior-baseline.json` and `default-behavior-post.json`: same-WASM
  default-behavior negative control for the standing smoke inputs, with
  baseline provenance from parent `a87c6b88a9702a48a13d42dde74498ff03f56b01`.
- `control-surface-evidence.json`: dev-harness control group evidence for
  deploy status/manual deploy, deploy-cache invalidation, persistence
  diagnostics, injected-assets diagnostics, option synchronization, inspector
  render gaps, raw response visibility, free-form `set_option`, free-form
  `customize`, and debug URL/error-history surface.
- `public-demo-gating-evidence.json`: public-demo hidden-control assertion plus
  data-surface checks for WEB-05 raw action results and cache/asset actions.

## Default Behavior

The WEB-05 Playwright negative control now captures:

- baseline from parent `a87c6b88a9702a48a13d42dde74498ff03f56b01` served from
  `http://127.0.0.1:5174` with the same built WASM copied into the parent
  worktree's untracked `target/` path;
- post-change from the corrected WEB-05 worktree served from
  `http://127.0.0.1:5173`.

Both runs use the same WASM hash:

`cb05289d6a72562ed349d82e94eec0e5681f40fe681aa823107670be3c68dc22`

The test compares the WASM hash, runtime marker, and candidate/preedit
snapshots. The covered smoke inputs are `jyut6ping3` `ngo`,
`jyut6ping3` `santai`, `luna_pinyin` `ni`, and `luna_pinyin` `hao`;
candidate text/source/preedit rows are unchanged.

## Public Demo

The public demo build uses the shared `IS_PUBLIC_DEMO` gate derived from
`import.meta.env.VITE_YUNE_PUBLIC_DEMO === "1"`. WEB-05 now gates both the
visible debug/admin cards and the new hidden data surface:

- `rime.ts` does not write `data-yune-last-action-result` in public-demo builds.
- `YuneControlSurface` does not call `deployCacheSnapshot()` or
  `injectedAssetsManifest()` in public-demo builds.

The demo-mode Playwright pass asserts that these dev/admin selectors are absent
from the built public demo:

- `[data-yune-control-redeploy]`
- `[data-yune-control-invalidate-deploy-cache]`
- `[data-yune-persistence-diagnostics-panel]`
- `[data-yune-injected-assets]`
- `[data-yune-raw-response-viewer]`
- `[data-yune-freeform-set-option]`
- `[data-yune-freeform-customize]`
- `[data-yune-debug-url-reference]`
- `[data-yune-action-error-history]`

Allowed public-demo rows remain product-shaped controls or references:
`ascii_punct`, deploy status, `ascii_punct` optionChanged-backed UI state,
hotkey reference, free-form dictionary exclude editor, and inspector render
fields. Deploy-cache details, raw responses, injected-assets diagnostics, and
detailed action-error history remain hidden and no longer run their WEB-05 data
pulls in public-demo mode.

## Deferred Rows

- `debug.storage`: runtime-lane deferred. The engine emits the storage block,
  but `packages/yune-web-runtime` drops it in `parseInspectorDebug`; WEB-05
  deliberately did not change that package. This is the exact future unlock for
  M57-style model-shape/storage diagnostics.
- Engine option read-back (`get_option`): engine-lane deferred. WEB-05 relies
  on existing `optionChanged` notifications and did not add a `yune_web_*`
  export.

## Named Follow-Ups

- `WEB05-FOLLOWUP-DEPLOY-CACHE-PERSISTED-CUSTOM-CONFIG`: current DOGFOOD-01
  evidence records a deliberate `deploy:cache-miss` after changing page size to
  7, because persisted custom config exists while the pre-deploy schema snapshot
  still reports `menu/page_size=6`. WEB-05 keeps this visible rather than
  changing deploy-cache policy in the harness-only corrective pass.
- `WEB05-FOLLOWUP-EXTENDED-CHARSET-BROWSER-EFFECT`: current `cangjie5` browser
  schema lacks `charset_filter`/`cjk_minifier`, so Extended charset is a visible
  pre-existing toggle but the U+2330A candidate set is unchanged off/on. A later
  slice should install a real filter gear or demote the toggle's active-control
  claim.

## Gate Results

- `npm.cmd --prefix apps/yune-web run typecheck`: pass.
- Same-WASM WEB-05 negative control
  `npm.cmd --prefix apps/yune-web/e2e run test:e2e -- --grep "WEB-05 same-WASM default behavior"`:
  pass, 1 passed.
- `npm.cmd --prefix apps/yune-web/e2e run test:e2e` with dev server at
  `http://127.0.0.1:5173`: pass, 80 passed / 8 skipped.
- `npm.cmd --prefix apps/yune-web run build:public`: pass.
- `WEB05_PUBLIC_DEMO_E2E=1` public-demo Playwright pass against
  `apps/yune-web/public-demo/dist`: pass, 1 passed; hidden debug/admin controls
  and WEB-05 raw/cache/asset data surfaces absent.
- `YUNE_PUBLIC_DEMO_E2E=1` M31 `@public-smoke` lane against
  `apps/yune-web/public-demo/dist`: pass, 4 passed.
- `git diff --check`: pass.
- `git diff --name-only -- crates packages/yune-web-runtime`: empty.
- Focused vitest: not run; WEB-05 added no new pure helper logic requiring a
  vitest-only unit.

## Notes

The local `apps/yune-web/source/` checkout can contain its own `node_modules`.
WEB-05 now constrains the harness Vite optimizer to the harness `index.html`
entry and dedupes `react`/`react-dom`, so the local upstream checkout cannot
prebundle `react-dom` from `source/node_modules` while the harness imports
`react` from its own dependency tree.

Manual redeploy reports `success` through the existing worker
`deployStatusChanged` seam, and deploy-cache invalidation is observable as
`data-yune-deploy-cache-fresh="false"`. The current cache-fresh snapshot also
remains `false` after a successful manual redeploy when persisted custom config
is present; WEB-05 records that exposed stamp-freshness state honestly rather
than changing runtime or engine policy in this web-harness-only slice.
