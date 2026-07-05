# WEB-05 `yune-web.spec.ts` Rebaseline Audit

Date: 2026-07-05

Scope: audit of the broad `apps/yune-web/e2e/yune-web.spec.ts` changes that
landed with WEB-05. This note separates selector/helper maintenance from
behavior assertions so WEB-05 does not silently absorb unrelated behavior drift.

## Selector / Helper Maintenance

Accepted as harness maintenance:

| Area | Change | Disposition |
| --- | --- | --- |
| Compose input locator | Replaced broad `input[type='text'], textarea` probes with the compose textarea helper. | Accepted; avoids unrelated form fields added by WEB-05 controls. |
| Input helper | Hardened focus/type helper against disabled/loading textarea states. | Accepted; no behavior expectation changed. |
| Selector scoping | Scoped top-control, schema, DaisyUI/class, and localized-title selectors. | Accepted; prevents false positives after additional control cards and bilingual labels. |
| Serial mode | Kept the broad browser spec serial to avoid origin/persistence races between dogfood scenarios. | Accepted; test scheduling only. |

## Behavior Assertion Changes

| Area | WEB-05-era change | Corrective disposition |
| --- | --- | --- |
| M25 deploy cache | The old `deploy:cache-hit` expectation was weakened to `hit|miss`. | Replaced with an explicit `deploy:cache-miss` assertion plus persisted/deployed page-size mismatch proof. Named follow-up: `WEB05-FOLLOWUP-DEPLOY-CACHE-PERSISTED-CUSTOM-CONFIG`. |
| M22 extended charset | The test asserted U+2330A exists both before and after enabling Extended charset. | Replaced with an honest N/A assertion: current `cangjie5.schema.yaml` has no `charset_filter`/`cjk_minifier`, so the rare candidate set is unchanged off/on. Named follow-up: `WEB05-FOLLOWUP-EXTENDED-CHARSET-BROWSER-EFFECT`. |
| M31 public identity | Public smoke still expected visible `yune-web`, but the banner had only the localized engine title. | Restored visible `yune-web` product text in the banner through `uiText.header.product`; M31 public smoke remains valid. |
| WEB-05 default behavior | Baseline was captured from the WEB-05 tip rather than the parent harness. | Recaptured `default-behavior-baseline.json` from parent `a87c6b88a9702a48a13d42dde74498ff03f56b01` on port 5174 and current post from the corrective worktree on port 5173, both using the same WASM SHA. |
| Public-demo debug data | Hidden cards still allowed raw action-result datasets and cache/asset snapshot actions to run in public-demo builds. | Gated `data-yune-last-action-result` and skipped WEB-05 cache/asset remote pulls when `IS_PUBLIC_DEMO`; public-demo evidence now checks DOM absence and data-surface/action absence. |
| Older dogfood rows | Several historical rows changed inputs or loosened candidate/layout assertions (`nei` to `ngo`, AI/guided scenario rows, layout details). | Not counted as WEB-05 behavior proof. They remain current-harness assertions unless a later milestone restores their old oracle rows with fresh evidence. |

## Residual Follow-Ups

- `WEB05-FOLLOWUP-DEPLOY-CACHE-PERSISTED-CUSTOM-CONFIG`: decide whether a
  persisted custom config should become cache-fresh after redeploy, or whether
  the harness should expose a separate "custom config requires deploy" state.
- `WEB05-FOLLOWUP-EXTENDED-CHARSET-BROWSER-EFFECT`: either install a real
  browser schema filter gear for Extended charset or demote the visible toggle
  from active browser-control claims.
