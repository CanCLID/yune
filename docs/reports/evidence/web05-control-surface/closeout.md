# WEB-05 Control Surface Closeout

Date: 2026-07-05

Scope: `apps/yune-web` only. WEB-05 did not change `crates/` or
`packages/yune-web-runtime/`.

## Evidence Files

- `controls-ledger.md`: 108-row raw control/diagnostic inventory, including 14
  `surface` rows with explicit public-demo posture and 2 deferred rows.
- `default-behavior-baseline.json` and `default-behavior-post.json`: same-WASM
  default-behavior negative control for the standing smoke inputs.
- `control-surface-evidence.json`: dev-harness control group evidence for
  deploy status/manual deploy, deploy-cache invalidation, persistence
  diagnostics, injected-assets diagnostics, option synchronization, inspector
  render gaps, raw response visibility, free-form `set_option`, free-form
  `customize`, and debug URL/error-history surface.
- `public-demo-gating-evidence.json`: public-demo hidden-control assertion.

## Default Behavior

The WEB-05 Playwright negative control captured baseline and post-change rows
against the same WASM hash:

`cb05289d6a72562ed349d82e94eec0e5681f40fe681aa823107670be3c68dc22`

The baseline and post-change JSON snapshots compare equal. The covered smoke
inputs are `jyut6ping3` `ngo`, `jyut6ping3` `santai`, `luna_pinyin` `ni`, and
`luna_pinyin` `hao`; candidate text/source/preedit rows are unchanged.

## Public Demo

The public demo build uses the shared `IS_PUBLIC_DEMO` gate derived from
`import.meta.env.VITE_YUNE_PUBLIC_DEMO === "1"`. The demo-mode Playwright pass
asserted that these dev/admin selectors are absent from the built public demo:

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
`ascii_punct`, deploy status/cache summary, optionChanged-backed UI state,
hotkey reference, free-form dictionary exclude editor, and inspector render
fields.

## Deferred Rows

- `debug.storage`: runtime-lane deferred. The engine emits the storage block,
  but `packages/yune-web-runtime` drops it in `parseInspectorDebug`; WEB-05
  deliberately did not change that package. This is the exact future unlock for
  M57-style model-shape/storage diagnostics.
- Engine option read-back (`get_option`): engine-lane deferred. WEB-05 relies
  on existing `optionChanged` notifications and did not add a `yune_web_*`
  export.

## Gate Results

- `npm.cmd --prefix apps/yune-web run typecheck`: pass.
- `npm.cmd --prefix apps/yune-web/e2e run test:e2e` with dev server at
  `http://127.0.0.1:5173`: pass, 80 passed / 8 skipped.
- `npm.cmd --prefix apps/yune-web run build:public`: pass.
- `WEB05_PUBLIC_DEMO_E2E=1` public-demo Playwright pass against
  `apps/yune-web/public-demo/dist`: pass, hidden debug/admin controls absent.
- `git diff --check`: pass.
- Focused vitest: not run; WEB-05 added no new pure helper logic requiring a
  vitest-only unit.

## Notes

Manual redeploy reports `success` through the existing worker
`deployStatusChanged` seam, and deploy-cache invalidation is observable as
`data-yune-deploy-cache-fresh="false"`. The current cache-fresh snapshot also
remains `false` after a successful manual redeploy; WEB-05 records that exposed
stamp-freshness state honestly rather than changing runtime or engine policy in
this web-harness-only slice.
