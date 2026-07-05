# WEB-05 Controls Ledger

Date: 2026-07-05. Phase 0 deliverable of
[`plans/completed/web05-plan-harness-control-surface.md`](../../../plans/completed/web05-plan-harness-control-surface.md).

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

Public-demo posture is explicit for every `surface` row. `allowed` means the row may appear in the public demo because it is product-shaped state or an already-public setting. `hidden` means the row must be gated by the shared `IS_PUBLIC_DEMO` constant planned for Phase 1; these rows must not accidentally enlarge the public demo.

| # | Control / diagnostic | Seam (exists today) | Public demo | Why |
| - | --- | --- | --- | --- |
| S1 | `ascii_punct` toggle | live `setOption("ascii_punct")`; switch exists in every schema; status strip already shows the chip | allowed | The only schema switch with a status chip but no control. |
| S2 | Deploy-status dataset + visible state | `deployStatusChanged` listener -> add `<html>` `data-yune-deploy-status` + inspector text | allowed | Weakest-surfaced event: today only a transient spinner/toast; Playwright cannot assert deploy state. |
| S3 | Manual "Redeploy now" button | worker `customize()` + `deploy()` (auto-only today) | hidden | Deploy exists with no explicit trigger; needed for staleness/dogfood debugging, but manual admin controls must not grow the public demo. |
| S4 | Persistence-diagnostics inspector section | adapter `emitPersistenceDiagnostic` markers (already on `<html>` dataset + `__YUNE_WEB_DEBUG__`) | hidden | Rich persisted-vs-deployed config snapshots reachable only via console today; would have caught the M41 deploy-skip regression visually. |
| S5 | Deploy-cache stamp viewer + force-invalidate | adapter `.yune-deploy-stamp.json` / `isDeployCacheFresh` (cache-hit/miss already emitted as diagnostic) | hidden | Only current invalidation is the full hard reset; a scoped "invalidate deploy cache" is the missing middle, but cache invalidation is admin/debug power. |
| S6 | `optionChanged` observer (dataset + UI sync) | engine `option` notifications -> `optionChanged` listener (zero consumers today) | allowed | Engine-initiated option flips (incl. hotkeys) are invisible; wire via the existing-but-unused `useRimeOption` hook (`hooks.ts:58`). |
| S7 | Key-binder hotkey reference panel | schema `key_binder` bindings (Ctrl+Shift+2 ascii, Ctrl+Shift+3 full-shape, Ctrl+period ascii-punct, Ctrl+Shift+1/space variant cycle) | allowed | Real, persisted engine controls that nothing documents; pairs with S6 so flips are visible. |
| S8 | Free-form `dictionary_exclude` editor | `customize({dictionaryExclude: string[]})` (UI today = canned one-char preset per schema) | allowed | Seam accepts an arbitrary list; expose a real list editor, keep the preset as a quick option. |
| S9 | Injected-assets manifest diagnostic | worker `extraSharedAssets` writes (path-validated) | hidden | List what was written into the shared data dir per deploy (name/bytes); cheap staleness/debug aid, WEB-04-pattern data attributes. |
| S10 | Inspector render gaps: candidate `preedit` + `ai_confidence`; prediction `weight_threshold`/`above_threshold`; `segment.source` | already parsed into runtime types, simply not rendered | allowed | Zero new plumbing; render-only additions to existing inspector panels. |
| S11 | Raw response JSON viewer (dev-only) | response envelope already in hand in `rime.ts` | hidden | The raw engine JSON is not viewable anywhere; single collapsible inspector pane; demo-gated. |
| S12 | Free-form `set_option` + arbitrary-key `customize` console (dev-only) | `setOption(name, bool)` and `yune_web_customize(config_id, key, value)` accept arbitrary keys; UI drives only a fixed list | hidden | The definitional "surface all controls" item for engine debugging; strictly demo-gated, with a visible "modifies deployed config" warning for customize keys. |
| S13 | Error-detail surfacing | response `error` strings name the failing API slot; today reduced to a generic toast | hidden | Show actual error strings and last-N history in the dev harness; detailed error history is diagnostic surface and must stay out of the public demo. |
| S14 | Debug-URL reference panel (dev-only) | `?schema=`, `?debug`, `?wasmAttributionFamily=` already work URL-only | hidden | Document the reproduce-with-a-URL surface in the harness UI; debug URL reference is dev-power surface. |

## 2. `already-surfaced` (no Phase 1 work; verified location noted)

Worker actions: `selectSchema` (SchemaSwitcher), `processKey`/`selectCandidate`/`deleteCandidate` (long-press)/`flipPage` (CandidatePanel), `stageAi` (auto when `enableAI`), `customize`+`deploy` (auto on preference change), `getUserdbSnapshot`/`importUserdb` (YuneUserdbViewer incl. export download), hard reset (Preferences danger button).

Live options with UI: `ascii_mode` (toolbar 中/英 + prefs), `full_shape` (全/半), output-standard group (`zh_hans`/`zh_hant_hk`/`zh_hant_tw`/`variants_hk`/`trad_tw`/`simplification` via cycle button + radio), `extended_charset`, `disabled`, `yune_inspector` (inspector checkbox).

Deploy-time preferences with UI: `pageSize`, `enableCompletion`, `enableCorrection`, `enableSentence`, `enableLearning` (fans out to `enable_user_dict`+`encode_commit_history`), `combineCandidates`, `predictionNeverFirst`, `predictionThreshold`, `dictionaryExclude` (preset only — see S8), `isCangjie5` (toolbar segment).

Diagnostics with UI/dataset: startup `initialized` (+heap seed), `schemaChanged` (`<html>` dataset), grammar diagnostic (WEB-04 metric row + dataset — the pattern template), inspector metrics strip (lookup ms, heap, peak, AI ms, candidates, userdb rows — always visible), inspector debug panel (segments, algebra, filter audit, prediction table, AI staging), status strip (`data-yune-status-*`), per-action timing/error diagnostics (`data-yune-action-*` last-100/25), persistence diagnostics dataset (console/dataset only — UI is S4), `yune-startup`/`yune-persistence` diagnostic sources, candidate source labels (AI badge always; others inspector-gated), reverse-lookup trigger summary (toolbar), memory snapshots per action.

## 3. Deferred rows (named, out of WEB-05 scope by plan rule)

| Row | Lane | What / why it matters |
| --- | --- | --- |
| Storage debug block (`source_fallback` deferrals, selected-storage rows with `mapping_mode`/`byte_source_len`/`stored_entry_count`, `memory_owner_rows`) | **runtime-lane-deferred** | The engine emits it in inspector debug (`web_runtime.rs:698-730`), but `packages/yune-web-runtime` `parseInspectorDebug` rebuilds the object **without** the `storage` field, silently dropping it. This remains deferred by WEB-05's hard scope rule: do not fix `parseInspectorDebug` here. Coordination note for M57: this is the exact future unlock for model-shape/storage diagnostics, including sentence-model byte-backed ownership and selected-storage inspection. |
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

- App-side demo flag: `IS_PUBLIC_DEMO` in `consts.ts`, derived from `import.meta.env.VITE_YUNE_PUBLIC_DEMO === "1"`. WEB-05 gates the new dev-power controls on this shared constant. Do not use `import.meta.env.DEV` for visible-control gating.
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

## Appendix A: raw inventory table

The table below is the raw 108-row inventory behind the grouped sections above. `Public demo` is precise for the 14 `surface` rows; other rows are marked as `pre-existing`, `n/a`, or `hidden` where the row is only a gating/mechanism record.

| Row | Name | Seam | Current surfaced location | Public demo | Disposition | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| A001 | `ascii_punct` toggle | `setOption("ascii_punct")`; schema switch; status bit | Status strip only | allowed | surface | Add a live control and keep status synchronized. |
| A002 | Deploy-status dataset + visible state | `deployStatusChanged` listener | Spinner/toast only | allowed | surface | Add `<html>` dataset plus inspector-visible state. |
| A003 | Manual redeploy | `customize()` + `deploy()` worker actions | Auto-deploy only | hidden | surface | Admin/debug control; gate from public demo. |
| A004 | Persistence diagnostics panel | `yune-persistence` diagnostics; debug helper | Dataset and console only | hidden | surface | Display persisted/deployed settings in dev harness. |
| A005 | Deploy-cache viewer/invalidate | `.yune-deploy-stamp.json`; cache-hit/miss diagnostics | Dataset/console only | hidden | surface | Scoped invalidation stays dev-harness-only. |
| A006 | `optionChanged` observer | Engine notification -> `optionChanged` listener | Listener exists; no UI consumer | allowed | surface | Use existing `useRimeOption` path. |
| A007 | Key-binder hotkey reference | Schema `key_binder` bindings | Not surfaced | allowed | surface | Product-shaped reference for existing controls. |
| A008 | Free-form `dictionary_exclude` editor | `customize({ dictionaryExclude })` | Canned preset toggle only | allowed | surface | Keep preset and add explicit list editor. |
| A009 | Injected-assets manifest | Worker `extraSharedAssets` write path | Not surfaced | hidden | surface | Diagnostic asset list; gate from public demo. |
| A010 | Inspector render gaps | Runtime-parsed debug/candidate fields | Partially rendered inspector | allowed | surface | Render candidate preedit, AI confidence, threshold, above-threshold, and segment source. |
| A011 | Raw response JSON viewer | Response envelope in `rime.ts` | Not surfaced | hidden | surface | Dev-harness-only raw engine payload. |
| A012 | Free-form `set_option` / `customize` console | Existing actions accept arbitrary names/keys | Not surfaced | hidden | surface | Strict debug-power surface; gate from public demo. |
| A013 | Detailed action error history | `data-yune-last-action-error`; last-25 errors | Dataset and console only | hidden | surface | Show actual error details in dev harness, not public demo. |
| A014 | Debug URL reference | `?schema=`, `?debug`, `?wasmAttributionFamily=` | URL-only | hidden | surface | Dev-harness reproduction reference. |
| A015 | Schema selection action | `selectSchema(schemaId)` | `SchemaSwitcher` | pre-existing | already-surfaced | Existing primary schema control. |
| A016 | Schema URL selection | `?schema=` read in worker | URL behavior | pre-existing | already-surfaced | Deep-link schema selection. |
| A017 | Schema change event | `schemaChanged` listener | `<html>` dataset and React state | pre-existing | already-surfaced | Existing WEB-04-style dataset. |
| A018 | Schema option registry | `SCHEMA_OPTIONS` / `PUBLIC_SCHEMA_OPTIONS` | `SchemaSwitcher` | pre-existing | already-surfaced | Demo filtering is already present. |
| A019 | Key processing | `processKey(input)` | Compose text area and candidates | pre-existing | already-surfaced | Core user path. |
| A020 | Candidate selection | `selectCandidate(index)` | Candidate panel click/keyboard | pre-existing | already-surfaced | Core user path. |
| A021 | Candidate deletion | `deleteCandidate(index)` | Long-press candidate action | pre-existing | already-surfaced | Existing candidate action. |
| A022 | Page flipping | `flipPage(backward)` | Candidate panel paging | pre-existing | already-surfaced | Existing paging control. |
| A023 | Paging disabled state | Response page state | Candidate panel prev/next state | pre-existing | already-surfaced | Observable UI state for paging. |
| A024 | AI staging action | `stageAi()` | Auto second pass when AI enabled | pre-existing | already-surfaced | Default remains off. |
| A025 | AI enable preference | `customize({ enableAI })` | Preferences engine section | pre-existing | already-surfaced | Local-only, default-off. |
| A026 | Userdb snapshot | `getUserdbSnapshot()` | `YuneUserdbViewer` | pre-existing | already-surfaced | Existing user-data diagnostic. |
| A027 | Userdb import | `importUserdb(rawText)` | `YuneUserdbViewer` import | pre-existing | already-surfaced | Existing user-data mutation. |
| A028 | Userdb raw export | Snapshot `rawText` | `YuneUserdbViewer` download | pre-existing | already-surfaced | Existing export surface. |
| A029 | Userdb refresh | Snapshot action | `YuneUserdbViewer` refresh | pre-existing | already-surfaced | Existing refresh control. |
| A030 | Hard reset storage | `resetYuneWebStorage()` | Preferences danger button | pre-existing | already-surfaced | Existing broad reset; not the scoped deploy-cache invalidator. |
| A031 | Deploy-time customize | `customize(preferences)` | Auto on preference changes | pre-existing | already-surfaced | Fixed-key mapper in app adapter. |
| A032 | Deploy action | `deploy()` | Auto after customize | pre-existing | already-surfaced | Manual trigger is A003. |
| A033 | Deploy transient state | `deployStatusChanged` | Spinner/toast | pre-existing | already-surfaced | Dataset/inspector hardening is A002. |
| A034 | Page size | `menu/page_size` customize key | Preferences range | pre-existing | already-surfaced | Deploy-time setting. |
| A035 | Completion | `enable_completion` customize key | Preferences toggle | pre-existing | already-surfaced | Deploy-time setting. |
| A036 | Correction | `enable_correction` customize key | Preferences toggle | pre-existing | already-surfaced | Deploy-time setting. |
| A037 | Sentence mode | `enable_sentence` customize key | Preferences toggle | pre-existing | already-surfaced | Deploy-time setting. |
| A038 | User dictionary | `enable_user_dict` customize key | Preferences toggle | pre-existing | already-surfaced | Learning on/off. |
| A039 | Commit-history encoding | `encode_commit_history` customize fanout | Preferences learning toggle | pre-existing | already-surfaced | Same UI as A038. |
| A040 | Combine same-text candidates | `combine_candidates` customize key | Preferences toggle | pre-existing | already-surfaced | Deploy-time setting. |
| A041 | Prediction never first | `prediction/never_first` customize key | Preferences toggle | pre-existing | already-surfaced | Deploy-time setting. |
| A042 | Prediction threshold | `prediction/weight_threshold` customize key | Preferences range | pre-existing | already-surfaced | Render threshold details in A010. |
| A043 | Dictionary exclude preset | `dictionary_exclude` customize key | Preferences preset toggle | pre-existing | already-surfaced | Free-form editor is A008. |
| A044 | Cangjie version | `isCangjie5` customize path | Toolbar segment | pre-existing | already-surfaced | Deploy-time control. |
| A045 | ASCII mode toolbar | `setOption("ascii_mode")` | Toolbar mode segment | pre-existing | already-surfaced | Existing live option. |
| A046 | ASCII mode preference | `setOption("ascii_mode")` | Preferences session section | pre-existing | already-surfaced | Same live option as A045. |
| A047 | Full-shape toolbar | `setOption("full_shape")` | Toolbar width segment | pre-existing | already-surfaced | Existing live option. |
| A048 | Full-shape preference | `setOption("full_shape")` | Preferences session section | pre-existing | already-surfaced | Same live option as A047. |
| A049 | Output standard toolbar | Output-standard cycle | Toolbar button | pre-existing | already-surfaced | Existing live option group. |
| A050 | Output standard radio | Output-standard radio group | Preferences session section | pre-existing | already-surfaced | Existing live option group. |
| A051 | `variants_hk` option | Output-standard mapper | Toolbar/preferences | pre-existing | already-surfaced | TypeDuck HK output path. |
| A052 | `trad_tw` option | Output-standard mapper | Toolbar/preferences | pre-existing | already-surfaced | TypeDuck Taiwan output path. |
| A053 | `simplification` option | Output-standard mapper | Toolbar/preferences | pre-existing | already-surfaced | TypeDuck simplified output path. |
| A054 | `zh_hans` option | Output-standard mapper | Toolbar/preferences | pre-existing | already-surfaced | Luna simplified output path. |
| A055 | `zh_hant_hk` option | Output-standard mapper | Toolbar/preferences | pre-existing | already-surfaced | Luna HK output path. |
| A056 | `zh_hant_tw` option | Output-standard mapper | Toolbar/preferences | pre-existing | already-surfaced | Luna Taiwan output path. |
| A057 | Extended charset | `setOption("extended_charset")` | Preferences session section | pre-existing | already-surfaced | Existing live option. |
| A058 | Disabled mode | `setOption("disabled")` | Preferences session section | pre-existing | already-surfaced | Existing live option. |
| A059 | Inspector option | `setOption("yune_inspector")` | Inspector TRACE toggle | pre-existing | already-surfaced | Enables debug payloads. |
| A060 | Inspector panel visibility | React state + inspector option | Inspector gate | pre-existing | already-surfaced | Existing panel shell. |
| A061 | Status schema | Response `status` | `YuneStatusStrip` | pre-existing | already-surfaced | Schema id/name chips. |
| A062 | Status disabled/composing | Response `status` | `YuneStatusStrip` | pre-existing | already-surfaced | Live status chips. |
| A063 | Status mode/width | Response `status` | `YuneStatusStrip` | pre-existing | already-surfaced | ASCII/full-shape chips. |
| A064 | Status output/traditional/punct | Response `status` | `YuneStatusStrip` | pre-existing | already-surfaced | Includes ASCII punctuation chip. |
| A065 | Candidate source labels | Response candidate source | Candidate badges/inspector-gated labels | pre-existing | already-surfaced | AI badge always; other labels inspector-gated. |
| A066 | Reverse lookup support | Schema metadata | Toolbar reverse summary | pre-existing | already-surfaced | Existing reference for reverse triggers. |
| A067 | Dictionary panel | Candidate details | `DictionaryPanel` | pre-existing | already-surfaced | Existing detailed candidate view. |
| A068 | Dictionary details display | Display preference | Preferences display section | pre-existing | already-surfaced | Display-only setting. |
| A069 | Candidate romanization | Display preference | Preferences display section | pre-existing | already-surfaced | Display-only setting. |
| A070 | Reverse code display | Display preference | Preferences display section | pre-existing | already-surfaced | Display-only setting. |
| A071 | Candidate layout | Display preference | Preferences display section | pre-existing | already-surfaced | Display-only setting. |
| A072 | Display languages | Display preference | Preferences display section | pre-existing | already-surfaced | Display-only setting. |
| A073 | Main display language | Display preference | Derived in preferences | pre-existing | already-surfaced | Kept coherent with selected display languages. |
| A074 | UI language | Local storage preference | Header language switcher | pre-existing | already-surfaced | UI-only. |
| A075 | Theme | Local/browser preference | Header theme switcher | pre-existing | already-surfaced | UI-only. |
| A076 | Chinese typeface | Display preference | Preferences display section | pre-existing | already-surfaced | UI-only rendering. |
| A077 | Fixed floating panel | Display preference | Compose panel toggle | pre-existing | already-surfaced | UI-only layout. |
| A078 | Startup initialized state | `initialized` listener | `<html>` dataset + startup state | pre-existing | already-surfaced | Existing startup diagnostic. |
| A079 | Loading state | App loading tracker | Overlay + `<html>` dataset | pre-existing | already-surfaced | Existing busy indicator. |
| A080 | Startup failure | Worker error/init failure | Toast + failed overlay | pre-existing | already-surfaced | Existing error path. |
| A081 | Startup heap seed | Init memory snapshot | Metrics strip | pre-existing | already-surfaced | Existing memory metric seed. |
| A082 | Lookup latency metric | Result/action timing | Metrics strip | pre-existing | already-surfaced | Existing inspector metric. |
| A083 | WASM heap metric | Memory snapshot | Metrics strip | pre-existing | already-surfaced | Existing inspector metric. |
| A084 | Peak WASM heap metric | Memory snapshot | Metrics strip | pre-existing | already-surfaced | Existing inspector metric. |
| A085 | AI rerank metric | AI result timing | Metrics strip | pre-existing | already-surfaced | Existing inspector metric. |
| A086 | Candidate count metric | Result candidates | Metrics strip | pre-existing | already-surfaced | Existing inspector metric. |
| A087 | Userdb row metric | Userdb snapshot | Metrics strip | pre-existing | already-surfaced | Existing inspector metric. |
| A088 | Grammar diagnostic dataset | `grammarDiagnosticChanged` | `<html>` dataset | pre-existing | already-surfaced | WEB-04 diagnostic pattern. |
| A089 | Grammar diagnostic metric | `grammarDiagnosticChanged` | Metrics strip | pre-existing | already-surfaced | WEB-04 metric row. |
| A090 | Inspector debug block | `debug` in result context | `YuneInspector` segments/algebra/filters/prediction/AI | pre-existing | already-surfaced | Render gaps are A010. |
| A091 | Action timing diagnostics | `rime.ts` action wrapper | `data-yune-action-diagnostics` + debug helper | pre-existing | already-surfaced | Last 100 actions. |
| A092 | Action error diagnostics | `rime.ts` action wrapper | `data-yune-action-errors` + `data-yune-last-action-error` | pre-existing | already-surfaced | Last 25 errors; detailed UI is A013. |
| A093 | Persistence/startup diagnostics | Worker/adapter diagnostic messages | `data-yune-persistence-diagnostics` + debug helper | pre-existing | already-surfaced | UI panel is A004. |
| A094 | `debug.storage` inspector block | Engine debug JSON emits `storage`; runtime parser drops it | Not surfaced | n/a | runtime-lane-deferred | Keep deferred for M57/model-shape diagnostics; requires runtime package change. |
| A095 | Engine option read-back | Missing `get_option` web export | Not surfaced | n/a | engine-lane-deferred | S6 covers practical two-way sync; new export is out of WEB-05 scope. |
| A096 | `soft_cursor` | Hardcoded `setOption("soft_cursor", true)` | Not surfaced | n/a | no-surface | Fixed invariant; surfacing would invite a no-op-like toggle. |
| A097 | `traditionalization` | Hardcoded `setOption("traditionalization", false)` | Not surfaced | n/a | no-surface | Appears inert in current tree. |
| A098 | Legacy options bitmap | `RimePreferences.options` | Not surfaced | n/a | no-surface | Dead field; cleanup candidate, not a control. |
| A099 | `select_keys` | Parsed schema metadata | Not surfaced directly | n/a | no-surface | Redundant with rendered select labels. |
| A100 | Octagram grammar weighting toggle | Schema-fixed grammar profile | Dedicated schema option | n/a | no-surface | WEB-04 design makes the profile the toggle. |
| A101 | Display preferences as engine controls | Local display state | Preferences display section | n/a | no-surface | Already surfaced as display UI; not an engine control. |
| A102 | UI language/theme as engine controls | Local UI state | Header controls | n/a | no-surface | Already surfaced as UI; no engine behavior. |
| A103 | Worker stderr exact `?debug` gate | Worker URL check | Not surfaced | n/a | no-surface | Latent follow-up only; not needed for WEB-05 control surface. |
| A104 | App public-demo flag | `IS_PUBLIC_DEMO` derived from `import.meta.env.VITE_YUNE_PUBLIC_DEMO === "1"` | `consts.ts`, consumed by schema gating and WEB-05 controls | hidden | gating-mechanism | Shared public-demo gate implemented in Phase 1. |
| A105 | Worker public-demo define | `YUNE_PUBLIC_DEMO_BUILD` | Worker public-demo branch | hidden | gating-mechanism | Existing esbuild define in `public-demo/build.mjs`. |
| A106 | Public schema option filter | `PUBLIC_SCHEMA_OPTIONS` | Schema switcher/hooks | pre-existing | gating-mechanism | Keeps octagram out of public demo. |
| A107 | Debug console logging gate | `import.meta.env.DEV` or `?debug` | Console behavior only | pre-existing | gating-mechanism | Not a visible-control gate; do not use for WEB-05 controls. |
| A108 | Debug helper global | `__YUNE_WEB_DEBUG__` | Browser console helper | pre-existing | gating-mechanism | Pre-existing in public demo; WEB-05 must not add new ungated debug UI. |

## Phase 0 gate

Ledger committed; no code changes in this phase (typecheck untouched).
Counts: 14 `surface` rows, ~75 already-surfaced, 2 deferred (1 runtime-lane,
1 engine-lane), 8 no-surface, 5 gating/mechanism rows.
