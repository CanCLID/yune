# WEB-05 Controls Ledger

Date: 2026-07-05. Phase 0 deliverable of
[`plans/active/web05-plan-harness-control-surface.md`](../../../plans/active/web05-plan-harness-control-surface.md).

Inventory method: three parallel read-only sweeps over (1) the worker
action/message protocol (`worker.ts`, `rime.ts`, `types.ts`), (2) the
engine-side web surface (`web_runtime.rs` 14 exports, `set_option` names,
schema `switches:`, adapter customize keys), and (3) the current UI + demo
gating (`App.tsx`, `Preferences.tsx`, `Toolbar`/`SchemaSwitcher`, `hooks.ts`,
`consts.ts`, gating flags). 108 raw rows, consolidated below.

Dispositions: `surface` (WEB-05 Phase 1 work), `already-surfaced`,
`engine-lane-deferred` (needs a new `yune_web_*` export),
`runtime-lane-deferred` (needs a `packages/yune-web-runtime` change), and
`no-surface` (deliberate decision with reason — an honest extension of the
plan's vocabulary so every row is dispositioned rather than fictionally
surfaced).

## 1. `surface` — the Phase 1 work list

| # | Control / diagnostic | Seam (exists today) | Why |
| - | --- | --- | --- |
| S1 | `ascii_punct` toggle | live `setOption("ascii_punct")`; switch exists in every schema; status strip already shows the chip | The only schema switch with a status chip but no control. |
| S2 | Deploy-status dataset + visible state | `deployStatusChanged` listener → add `<html>` `data-yune-deploy-status` + inspector text | Weakest-surfaced event: today only a transient spinner/toast; Playwright cannot assert deploy state. |
| S3 | Manual "Redeploy now" button | worker `customize()` + `deploy()` (auto-only today) | Deploy exists with no explicit trigger; needed for staleness/dogfood debugging. |
| S4 | Persistence-diagnostics inspector section | adapter `emitPersistenceDiagnostic` markers (already on `<html>` dataset + `__YUNE_WEB_DEBUG__`) | Rich persisted-vs-deployed config snapshots reachable only via console today; would have caught the M41 deploy-skip regression visually. |
| S5 | Deploy-cache stamp viewer + force-invalidate | adapter `.yune-deploy-stamp.json` / `isDeployCacheFresh` (cache-hit/miss already emitted as diagnostic) | Only current invalidation is the full hard reset; a scoped "invalidate deploy cache" is the missing middle. |
| S6 | `optionChanged` observer (dataset + UI sync) | engine `option` notifications → `optionChanged` listener (zero consumers today) | Engine-initiated option flips (incl. hotkeys) are invisible; wire via the existing-but-unused `useRimeOption` hook (`hooks.ts:58`). |
| S7 | Key-binder hotkey reference panel | schema `key_binder` bindings (Ctrl+Shift+2 ascii, Ctrl+Shift+3 full-shape, Ctrl+period ascii-punct, Ctrl+Shift+1/space variant cycle) | Real, persisted engine controls that nothing documents; pairs with S6 so flips are visible. |
| S8 | Free-form `dictionary_exclude` editor | `customize({dictionaryExclude: string[]})` (UI today = canned one-char preset per schema) | Seam accepts an arbitrary list; expose a real list editor, keep the preset as a quick option. |
| S9 | Injected-assets manifest diagnostic | worker `extraSharedAssets` writes (path-validated) | List what was written into the shared data dir per deploy (name/bytes); cheap staleness/debug aid, WEB-04-pattern data attributes. |
| S10 | Inspector render gaps: candidate `preedit` + `ai_confidence`; prediction `weight_threshold`/`above_threshold`; `segment.source` | already parsed into runtime types, simply not rendered | Zero new plumbing; render-only additions to existing inspector panels. |
| S11 | Raw response JSON viewer (dev-only) | response envelope already in hand in `rime.ts` | The raw engine JSON is not viewable anywhere; single collapsible inspector pane; **demo-gated**. |
| S12 | Free-form `set_option` + arbitrary-key `customize` console (dev-only) | `setOption(name, bool)` and `yune_web_customize(config_id, key, value)` accept arbitrary keys; UI drives only a fixed list | The definitional "surface all controls" item for engine debugging; **strictly demo-gated**, with a visible "modifies deployed config" warning for customize keys. |
| S13 | Error-detail surfacing | response `error` strings name the failing API slot; today reduced to a generic toast | Show the actual error string in the toast/inspector; store last-N (dataset already keeps `data-yune-last-action-error`). |
| S14 | Debug-URL reference panel (dev-only) | `?schema=`, `?debug`, `?wasmAttributionFamily=` already work URL-only | Document the reproduce-with-a-URL surface in the harness UI. |

## 2. `already-surfaced` (no Phase 1 work; verified location noted)

Worker actions: `selectSchema` (SchemaSwitcher), `processKey`/`selectCandidate`/`deleteCandidate` (long-press)/`flipPage` (CandidatePanel), `stageAi` (auto when `enableAI`), `customize`+`deploy` (auto on preference change), `getUserdbSnapshot`/`importUserdb` (YuneUserdbViewer incl. export download), hard reset (Preferences danger button).

Live options with UI: `ascii_mode` (toolbar 中/英 + prefs), `full_shape` (全/半), output-standard group (`zh_hans`/`zh_hant_hk`/`zh_hant_tw`/`variants_hk`/`trad_tw`/`simplification` via cycle button + radio), `extended_charset`, `disabled`, `yune_inspector` (inspector checkbox).

Deploy-time preferences with UI: `pageSize`, `enableCompletion`, `enableCorrection`, `enableSentence`, `enableLearning` (fans out to `enable_user_dict`+`encode_commit_history`), `combineCandidates`, `predictionNeverFirst`, `predictionThreshold`, `dictionaryExclude` (preset only — see S8), `isCangjie5` (toolbar segment).

Diagnostics with UI/dataset: startup `initialized` (+heap seed), `schemaChanged` (`<html>` dataset), grammar diagnostic (WEB-04 metric row + dataset — the pattern template), inspector metrics strip (lookup ms, heap, peak, AI ms, candidates, userdb rows — always visible), inspector debug panel (segments, algebra, filter audit, prediction table, AI staging), status strip (`data-yune-status-*`), per-action timing/error diagnostics (`data-yune-action-*` last-100/25), persistence diagnostics dataset (console/dataset only — UI is S4), `yune-startup`/`yune-persistence` diagnostic sources, candidate source labels (AI badge always; others inspector-gated), reverse-lookup trigger summary (toolbar), memory snapshots per action.

## 3. Deferred rows (named, out of WEB-05 scope by plan rule)

| Row | Lane | What / why it matters |
| --- | --- | --- |
| Storage debug block (`source_fallback` deferrals, selected-storage rows with `mapping_mode`/`byte_source_len`/`stored_entry_count`, `memory_owner_rows`) | **runtime-lane-deferred** | The engine emits it in inspector debug (`web_runtime.rs:698-730`), but `packages/yune-web-runtime` `parseInspectorDebug` rebuilds the object **without** the `storage` field, silently dropping it. This is the WEB-02-893MiB-class diagnostic and — post-M57 — the storage/model-shape diagnostic lane. Highest-value single deferred item; needs one runtime change + version bump. |
| Engine option read-back (`get_option`) | **engine-lane-deferred** | The web ABI is set-only; two-way state relies on `optionChanged` notifications (S6 covers the practical need). A read-back export would require touching `scripts/yune-web-exports.txt` — defer unless S6 proves insufficient. |

## 4. `no-surface` (deliberate, with reasons)

| Row | Reason |
| --- | --- |
| `soft_cursor` (fixed `true`), `traditionalization` (fixed `false`) | Hardcoded in the live-options pass; `traditionalization` appears to have **no consumer in crates/** (inert). Surfacing invites no-op toggles; recorded instead as a cleanup candidate. |
| Legacy `RimePreferences.options` bitmap | Dead field, never read by `customize()`; cleanup candidate, not a control. |
| Display-only preferences (typefaces, candidate layout, romanization, languages), UI language/theme | Non-engine by design; explicitly "does not affect engine output". |
| `select_keys` (parsed, unused) | Redundant with `select_labels` already rendered. |
| Octagram grammar weighting toggle | Schema-fixed by WEB-04 design (dedicated profile IS the toggle). |

## 5. Gating mechanisms (constraints for Phase 1)

- App-side demo flag: `import.meta.env.VITE_YUNE_PUBLIC_DEMO === "1"` (inline at `SchemaSwitcher.tsx:20`, `hooks.ts:247`). **Phase 1 must hoist a shared `IS_PUBLIC_DEMO` into `consts.ts`** and gate S11/S12/S14 (and any other debug-power control) on it. Do not use `import.meta.env.DEV`.
- Worker-side demo define: `YUNE_PUBLIC_DEMO_BUILD` (esbuild define in `public-demo/build.mjs:83`).
- Observed posture: several debug surfaces (`__YUNE_WEB_DEBUG__`, the `data-yune-*` action/persistence datasets, inspector, userdb viewer) ship ungated in the demo today. Pre-existing, out of WEB-05 scope to change; WEB-05 adds no *new* ungated debug surface regardless.

## 6. Inventory corrections & observations

- There is **no `postMessage` diagnostic source `yune-octagram`** in the
  current tree (the WEB-04-era duplicate channel was removed); the octagram
  diagnostic flows solely through `grammarDiagnosticChanged`.
- `useRimeOption` (`hooks.ts:58`) is a complete, unused two-way option hook —
  Phase 1 should build S1/S6 on it rather than new plumbing.
- Latent nit (follow-up, not WEB-05): the demo worker's stderr gate compares
  `location.search === "?debug"` against the **worker** URL, which always
  carries `?v=&schema=` — the gate is effectively unreachable.
- `jyut6ping3` UI id maps to the `jyut6ping3_mobile` runtime schema; octagram
  model failure falls back to `luna_pinyin` with the diagnostic carrying
  requested-vs-effective ids (WEB-04 semantics) — e2e assertions in Phase 2
  must key on the diagnostic, not the selector state.

## Phase 0 gate

Ledger committed; no code changes in this phase (typecheck untouched).
Counts: 14 `surface` rows, ~75 already-surfaced, 2 deferred (1 runtime-lane,
1 engine-lane), 8 no-surface, 5 gating/mechanism rows.
