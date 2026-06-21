# M22 — TypeDuck-Web Playground Feature-Completeness + Multi-Schema + Engine Debug Inspector Implementation Plan

> **Status:** Active · **Milestone:** M22 (Web playground build-out; M20 successor) · **Updated:** 2026-06-20 · **Type:** execution plan

> **For agentic workers:** implement task-by-task; steps use checkbox (`- [ ]`) syntax. Capture browser before/after evidence *before* claiming any ACTIVE control (M20 honesty gate is binding).

**Goal:** Surface *more* of Yune's engine inside this repo's internal patched TypeDuck-Web playground (`third_party/typeduck-web/`) as **honest controls** plus a **read-only debug inspector**, and load **more schemas** with a schema-switcher and reverse lookup — extending M20's canonical browser workbench without reopening M13, without changing the default `RimeApi`/`RimeCandidate` ABI, and without representing any unsupported behavior as working.

**Architecture:** M22 is the M20 successor (build-out), **not** M21 (M21 is the product-comparison protocol, complete once its gap-ledger is fully dispositioned). Three explicit, separately-tracked work buckets: (1) missing honest browser-safe toggles, (2) a read-only per-keystroke debug inspector, (3) multi-schema loading + schema switcher + reverse lookup. Runtime-changing controls flow through the existing `customize()` schema-key path (`adapter.ts:435-505`) or the `setOption()` session-option path (`adapter.ts:507-514`); schema switching flows through the existing `RimeSelectSchema` ABI slot (`abi.rs:301`) via a runtime re-init. The inspector adds only **opt-in debug JSON fields** in the TypeDuck response plus optional `yune_typeduck_*` read helpers; it must not widen `RimeCandidate`, reorder the `RimeApi` table, or move provider work into the per-key path. M18 can now build Yune-native `.table.bin`/`.prism.bin`/`.reverse.bin` artifacts; Bucket 3 should choose Yune-generated artifacts or pre-compiled upstream artifacts based on browser asset-budget and provenance needs.

**Tech Stack:** Rust `yune-core` / `yune-rime-api`, Emscripten WASM, `@yune-ime/typeduck-runtime`, TypeDuck-Web React/TypeScript, Playwright.

**Progress note (2026-06-20).** Bucket 2 is complete in `d548c9cf`
(`Implement M22 Bucket 2 debug inspector`). The inspector is opt-in/off by
default, preserves classic response identity when disabled, has committed
Playwright evidence under
`third_party/typeduck-web/e2e/results/m22-bucket2-inspector/`, and did not
change the default ABI layout files. Bucket 1 honest controls and Bucket 3
multi-schema/reverse-lookup work remain active.

---

## Grounded current state (cite-backed)

- **M20 is the parent milestone.** M20 turned `third_party/typeduck-web/` into Yune's canonical internal browser playground and established the **honesty gate**: a visible ACTIVE control is allowed only when it demonstrably changes candidate output, committed output, status output, or persisted schema config; display controls must prove a visible rendering change; non-browser-safe or deferred behavior is a guided scenario or a documented browser-surface N/A, never a fake toggle (`docs/plans/archive/m20-plan-web-demo-showcase-controls.md`, `docs/roadmap.md:293-345`). M20 already wired `enable_completion`, `enable_correction`, `enable_sentence`, learning, `combine_candidates`, `prediction_never_first`, a measured prediction threshold, plus live `ascii_mode`/`full_shape`/`simplification`.
- **M20 explicitly left these browser-safe options unsurfaced** and recorded `show_full_code` / Cangjie / schema-switch as **browser-surface N/A only because `jyut6ping3_mobile` is the lone schema** (`docs/roadmap.md:326-337`). M22 closes those.
- **`engine.set_option()` already handles the Bucket-1 live toggles.** `crates/yune-core/src/engine.rs:273-286` maps `"disabled"`, `"simplification"|"simplified"`, `"traditionalization"|"traditional"`, `"ascii_mode"`, `"full_shape"`, and `"ascii_punct"` onto `status` flags and into the `options` map, then calls `refresh_candidates()`. `get_option()` mirrors them (`engine.rs:288-299`).
- **`extended_charset` is read by the always-on `CharsetFilter`.** `crates/yune-core/src/filter/mod.rs:56-70`: `CharsetFilter::apply_with_options()` only filters extended CJK when `options.get("extended_charset")` is false; flipping `extended_charset` via `setOption()` therefore changes which candidates survive — **but only when a `charset_filter`/`cjk_minifier` gear is installed for the active schema** (`schema_install.rs:160-165`, `471-476`). The `charset_filter` itself stays always-on (install-time), per the exclusion list.
- **`dictionary_exclude` is a deploy-time `customize()` key.** `crates/yune-rime-api/src/schema_install.rs:281-282` reads `<name_space>/dictionary_exclude` as a string list and the translator consumes it via `.with_dictionary_exclude(...)` (`schema_install.rs:297`). This is a `customize()` + `deploy()` control, not a live option.
- **Traditionalization is honestly conditional.** The `SimplifierFilter` runs only when `options.get(self.option_name)` is true (`filter/mod.rs:466-469`) and supports `SimplifiedToTraditional` / `s2t` conversion (`filter/mod.rs:429-449`, `597`). The current `jyut6ping3_mobile` simplifier is keyed on `"simplification"` with `hk2s` data; a `traditionalization` toggle is only browser-observable if a simplifier gear keyed on `"traditionalization"` with `s2t`/traditionalization data is installed for the active schema — otherwise it is status-only and must be recorded as a documented N/A, not a fake toggle.
- **`disabled` sets a status flag.** `engine.rs:276` sets `status.is_disabled` (`state.rs:249`) and `get_option("disabled")` reads it back; whether it produces a visible candidate/status before/after on the browser surface must be measured, or the control is recorded as status-only N/A.
- **Candidate SOURCE marshalling is AI-only today.** `crates/yune-rime-api/src/typeduck_web.rs:589-619` (`attach_candidate_sources`) and `source_label()` (`:621-629`) emit only `ai`/`ai:local`; all other `CandidateSource` variants (`echo`, `punct`, `table`, `user_table`, `completion`, `sentence`, `reverse_lookup`, `history`, `switch`, `unfold`, `schema` — `crates/yune-core/src/state.rs:28-75`) exist in `engine.context.candidates[].source` but are dropped. `CandidateSource::as_str()` (`state.rs:60-75`) already gives stable strings.
- **`copy_candidate()` emits only `text`+`comment`.** `typeduck_web.rs:582-587`. `Candidate` carries `preedit: Option<String>`, `quality: f32`, and structured `source` (`state.rs:1-8`); `AiConfidence` carries `basis_points` 0..10000 (`state.rs:104-143`). None are marshalled.
- **`segment_tags`, staged AI, and snapshot exist in the engine.** `engine.set_segment_tags()` (`engine.rs:305-308`), `staged_ai_result` + `ai_decision_for_current_input()` (`engine.rs:21,220-221`), and the session candidate snapshot path already used by `attach_candidate_sources` (`typeduck_web.rs:589-619`). The TypeDuck JSON response is assembled in `response()`/`copy_context()` (`typeduck_web.rs:493-570`).
- **`RimeCandidate.reserved` is the only fork-only extension slot.** `crates/yune-rime-api/src/abi.rs:57` (`pub reserved: *mut std::ffi::c_void`) — a null `void*` per upstream Rime ABI, safe to repurpose without changing the default table. `select_schema` already exists as a table slot (`abi.rs:301`).
- **Multi-schema assets are half-present.** `worker.ts:132` hard-codes `SCHEMA_ID = "jyut6ping3_mobile"`; `worker.ts:167-200` already loads `luna_pinyin.schema.yaml`/`.dict.yaml`, `cangjie5.schema.yaml`/`.dict.yaml`, and `cangjie3.*` as **source YAML extra-shared assets**, but the second `loadExtraSharedAssets([...], true)` binary block (`worker.ts:193-200`) lists **only `jyut6ping3*` compiled `.bin` artifacts**. Directory listing confirms `cangjie5.dict.yaml` (449 KB) and `luna_pinyin.dict.yaml` (471 KB) exist with **no** `.table.bin`/`.prism.bin`/`.reverse.bin`. The `jyut6ping3.schema.yaml` (`source/public/schema/jyut6ping3.schema.yaml`) already declares `luna_pinyin`/`cangjie5` as reverse-lookup namespaces with `prefix: "\`p"`/`"\`c"`, proving the reverse-lookup shape Yune already runs.
- **Reverse lookup is supported in-engine.** `ReverseLookupTranslator` (`crates/yune-core/src/translator/mod.rs:1184-1325`, codes joined `"; "` at `:1295`) and `ReverseLookupFilter` (`crates/yune-core/src/filter/mod.rs:723-800`); config loaders at `schema_install.rs:340-393` / `:590-630` / `:764-778`; the `"; "` joiner is a locked fork-parity behavior (`docs/fork-parity-ledger.md:105`, F5).
- **M18 removed the dictionary-compilation blocker.** `docs/plans/archive/m18-plan-deployment-and-processor-depth.md` added `build_prism_bin`/`build_table_bin`/`build_reverse_bin`, so M22 no longer has to rely on a precompiled-asset workaround. Bucket 3 should still record whether it uses Yune-generated artifacts or pre-compiled upstream artifacts, with provenance and measured browser asset sizes either way.
- **TypeDuck profile artifacts already proven (parallel oracle).** M19 onboards upstream `cangjie5` and `luna_pinyin` parity against the `1.17.0` oracle (`docs/plans/m19-plan-breadth-schemas.md`). M22 reuses those provenance-stamped schema/dictionary identities for the browser bundle; it does **not** reopen M19 capture.

---

## Scope / Non-goals

**In scope.**

- **Bucket 1 — Missing honest toggles** (browser-safe, genuinely user-facing, not yet surfaced): `traditionalization` (live `setOption("traditionalization")`, conditioned on a real observable simplifier gear), `extended_charset` (live `setOption("extended_charset")`) **plus its `charset_filter`/`cjk_minifier` install dependency for the active schema**, `disabled` (live `setOption("disabled")`), and `dictionary_exclude` (deploy-time `customize()` + `deploy()`). Each ACTIVE control must clear the M20 honesty gate with real browser before/after evidence; otherwise it is a documented browser-surface N/A.
- **Bucket 2 — Read-only debug inspector (complete)**: shipped in `d548c9cf` as additive opt-in debug JSON plus a read-only browser panel. It shows segments + `segment_tags`, candidate source/quality/preedit/comment details, spelling-algebra expansion, filter audit, prediction scores vs the weight threshold, and AI staging; **zero default-ABI change**.
- **Bucket 3 — Multi-schema**: load THREE schemas in the playground — `jyut6ping3_mobile` + `cangjie5` + `luna_pinyin` — behind a schema-switcher UI wired through `RimeSelectSchema` (runtime re-init), with **reverse lookup enabled for both new schemas** (`cangjie5` and `luna_pinyin`). This unblocks `show_full_code` and the schema-switch surface (M20 browser-surface N/A) and gives M21 a multi-schema surface.
- Browser evidence under `third_party/typeduck-web/e2e/results/m22-playground-multischema-inspector/` for every ACTIVE control, every inspector field, and every schema switch.
- Patch regeneration + reverse/forward check of `patches/yune-typeduck-runtime.patch`.

**Non-goals.**

- **These must NOT become toggles** (inspect-only in Bucket 2, or left as always-on internals, because none has an honest user-facing before/after): `uniquifier_filter`, `single_char_filter`, `charset_filter` (**always-on** install gear; only its `extended_charset` *option* is a Bucket-1 toggle), schema-owned templates (`spelling_algebra`, `comment_format`, `preedit_format`, `tolerance_rules`, `prefix_suffix`, `segment_tags`), and internal `_`-prefixed options (`_vertical`, `_fold_options`, `_auto_commit`, `_chord_typing`, `_hide_candidate`). The inspector *shows* these; it does not expose them as controls.
- **`ascii_punct` is engine-backed after M18 but still needs browser evidence.** Do not expose it as a working toggle until Bucket 1 proves a browser-visible before/after through the M20 honesty gate.
- **No change to the default `RimeApi` table or `RimeCandidate` ABI.** No widening, no reorder, no new default-table slot. Inspector extensions are opt-in JSON + optional non-default `yune_typeduck_*` helpers only. (`abi.rs`, `api_table.rs`, `candidate_api.rs` stay diff-free.)
- **AI invariants unchanged:** AI default-off, classic-first, second-pass-only; `yune_typeduck_process_key` stays provider-free; provider work only behind `stage_ai`.
- **Upstream-first baseline preserved** outside the TypeDuck-Web/profile surface.
- **userdb persistence stays browser-limited** (in-memory / IndexedDB via IDBFS, not native file I/O); the inspector observes learning state but does not promise native-file persistence.
- Claiming browser multi-schema support without generated or provenance-stamped compiled artifacts. Bucket 3 can use M18's Yune-native writers or pre-compiled upstream artifacts, but must record provenance, measured sizes, and any per-schema blocker.
- Reopening M13 (AI exposure), M19 (oracle capture), or M21 (product-comparison protocol). M22 does not touch a separately cloned `TypeDuck-HK/TypeDuck-Web` product checkout.

---

## Tasks

### Bucket 1 — Missing honest toggles

#### Task 1 — Bucket-1 preference model + adapter mapping

**Files:** `third_party/typeduck-web/source/src/types.ts`, `consts.ts`, `source/src/yune-integration/adapter.ts`, `yune-integration/adapter-filesystem.test.ts`

- [ ] Extend `RimePreferences` (`types.ts`) and `DEFAULT_PREFERENCES` (`consts.ts`) with: `isTraditionalization: boolean` (default `false`), `isExtendedCharset: boolean` (default `false`), `isDisabled: boolean` (default `false`), and `dictionaryExclude: string[]` (default `[]`). Do **not** add an `ascii_punct` preference.
- [ ] In `adapter.ts`, route the three live options through the existing `setOption()` path used by `ascii_mode`/`full_shape`/`simplification`: `setOption("traditionalization", v)`, `setOption("extended_charset", v)`, `setOption("disabled", v)`. These map onto `engine.set_option()` (`engine.rs:273-286`) and the `CharsetFilter` options read (`filter/mod.rs:65-69`) with no new export.
- [ ] In `customize(preferences)` (`adapter.ts:435-505`), map `dictionaryExclude` to the deploy-time key the translator already consumes: `customizeSetting("translator/dictionary_exclude", JSON.stringify(preferences.dictionaryExclude))` (or the schema list encoding the existing `schema_string_list` reader at `schema_install.rs:281` expects — verify the encoding against `dictionary_exclude` parsing before finalizing). Trigger `deploy()`.
- [ ] **Honesty pre-check before exposing each control:** run a real-browser probe for each. (a) `traditionalization`: confirm the active schema has a simplifier gear keyed on `"traditionalization"`/`s2t` that visibly rewrites candidate text; if `jyut6ping3_mobile` only ships the `hk2s` `"simplification"` simplifier, **either** add an honest traditionalization simplifier gear to the loaded schema **or** record `traditionalization` as a documented browser-surface N/A (status-only) — do not ship a dead toggle. (b) `extended_charset`: confirm a `charset_filter`/`cjk_minifier` gear is installed for the active schema (`schema_install.rs:471-476`) so flipping the option changes the surviving candidate set; if not installed, record N/A. (c) `disabled`: confirm a visible candidate/status before/after; otherwise record status-only N/A.
- [ ] Extend `adapter-filesystem.test.ts` so the fake runtime receives the new `setOption`/`customize` calls with the exact keys (`traditionalization`, `extended_charset`, `disabled`, `translator/dictionary_exclude`).
- [ ] **Acceptance:** `npm --prefix packages/yune-typeduck-runtime test` passes; the adapter test asserts each Bucket-1 mapping; the honesty pre-check result (ACTIVE vs documented N/A) is recorded per control in the evidence dir.

#### Task 2 — Bucket-1 UI controls (honest, grouped, ascii_punct still deferred)

**Files:** `source/src/App.tsx`, `source/src/Preferences.tsx`, `source/src/Inputs.tsx` (if a list editor is needed)

- [ ] Render `traditionalization`, `extended_charset`, `disabled` toggles only for controls that passed the Task-1 honesty pre-check; render `dictionary_exclude` as a small comma/line list editor. Group them under the existing engine-controls section, separate from display-only controls.
- [ ] Wire live toggles into the existing `setOption()` effect in `App.tsx` (the effect family added in M20 for `ascii_mode`/`full_shape`/`simplification`) and the `dictionary_exclude` editor into the `customize()`/`deploy()` effect; add the new keys to the effect dependency lists.
- [ ] Keep `ascii_punct` absent from active controls unless the Bucket-1 honesty gate proves a browser-visible before/after for the now engine-backed behavior. If it is mentioned before that evidence exists, label it as "engine-backed; browser evidence pending" and render no working toggle.
- [ ] For any control the honesty pre-check downgraded to N/A, render it as an explicitly labeled, disabled/annotated row (or omit it) — never as an interactive toggle that does nothing.
- [ ] **Acceptance:** the app builds (`npm run build` in `source/`, or the checkout's Bun script); every rendered Bucket-1 toggle is backed by a measured before/after; `ascii_punct` renders no working toggle.

#### Task 3 — Bucket-1 control honesty tests

**Files:** `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`, `e2e/yune-browser-smoke.md`

- [ ] Add each shipped Bucket-1 ACTIVE control to the existing `ACTIVE_SHOWCASE_CONTROLS` honesty list and assert it is visible; assert `getByLabel(/ascii_punct/i)` has count 0.
- [ ] Add one before/after test per shipped ACTIVE control: `extended_charset` (a candidate containing extended-CJK appears only when on, via the `CharsetFilter` path); `traditionalization` (a candidate's text/comment changes to traditional form only when on, **only if** a real traditionalization gear was added); `disabled` (visible candidate/status before/after); `dictionary_exclude` (an entry from an excluded dictionary disappears after deploy). Every assertion compares candidate text / commit / status / persisted config — `consoleErrors == []` alone is insufficient (M20 rule).
- [ ] For each control recorded as N/A, the spec/evidence must explicitly state the documented browser-surface N/A rather than asserting a fake effect.
- [ ] Add an M22 Bucket-1 section to `yune-browser-smoke.md`.
- [ ] **Acceptance:** the honesty gate passes; each ACTIVE Bucket-1 control has committed before/after browser evidence; each N/A is documented.

### Bucket 2 — Engine debug inspector (READ-ONLY, complete)

Closed by `d548c9cf`. The checklist below is retained as the completed
acceptance record; future M22 workers should not redo this bucket except for
regression fixes.

#### Task 4 — Additive engine-state marshalling (no default-ABI change)

**Files:** `crates/yune-rime-api/src/typeduck_web.rs`, additive `Engine` accessors in `crates/yune-core/src/engine.rs`, `crates/yune-rime-api/tests/typeduck_web.rs`

- [ ] Extend `attach_candidate_sources()` / `source_label()` (`typeduck_web.rs:589-629`) to emit the **full** `CandidateSource` set using `CandidateSource::as_str()` (`state.rs:60-75`): `echo`, `punct`, `table`, `user_table`, `completion`, `sentence`, `reverse_lookup`, `history`, `switch`, `unfold`, `schema`, `ai`/`ai:local`. Keep the AI label exactly as today.
- [ ] Extend `copy_candidate()` (`typeduck_web.rs:582-587`) with **optional** debug-only JSON fields, gated behind an inspector/debug flag so default responses are byte-identical: `source` (string), `quality` (f32), `preedit` (if `Some`), and `ai_confidence` (0.0..1.0 from `AiConfidence::as_score()`, `state.rs:140-142`, when the source is AI).
- [ ] Extend `copy_context()` (`typeduck_web.rs:552-570`) with optional `segments` (array of `{start, end, tag, source}` computed from `Context.segment_tags` + input offsets via an additive `Engine` accessor), `ai_staging` (`{state: "off"|"pending"|"ready", reason?, for_input?}` derived from `ai_decision_for_current_input()` / `staged_ai_result`, `engine.rs:21,220-221`), and a `filter_audit` array (filter name + before/after counts) behind the same debug flag. Add additive `Engine` accessor methods for segment tags, the filter pipeline order, the prediction-weight-threshold value in effect, and (read-only) any learned-entry lookup — do **not** make existing private fields public beyond a read accessor.
- [ ] Add an opt-in `yune_typeduck_set_inspector_enabled(enabled)` (or a per-call debug parameter) so the inspector is **off by default** and the classic/default response stays byte-identical. If candidate-level metadata needs a C handle, use only `RimeCandidate.reserved` (`abi.rs:57`) with a new **non-default** `yune_typeduck_candidate_metadata()` helper — never a new default-table slot.
- [ ] **Acceptance:** `cargo test -p yune-rime-api --test typeduck_web` passes; a new test proves the response is byte-identical with the inspector off and carries the extra fields with it on; `git diff -- crates/yune-rime-api/src/abi.rs crates/yune-rime-api/src/api_table.rs crates/yune-rime-api/src/candidate_api.rs` is empty.

#### Task 5 — Spelling-algebra + prediction-score inspector data

**Files:** `crates/yune-core/src/engine.rs` / translator accessors, `crates/yune-rime-api/src/typeduck_web.rs`

- [ ] Surface spelling-algebra code expansion for the current input as read-only inspector data (which algebra projections produced the active code forms) without changing candidate ranking. Honest-scope note: per-candidate "which formula matched" requires new tracking; if that is heavier than additive, expose the **algebra rule list + the expanded code set** for the input rather than per-candidate provenance, and record the per-candidate-formula attribution as a documented inspector limitation.
- [ ] Surface prediction scores vs the weight threshold: emit each prediction candidate's `quality` alongside the effective `prediction_weight_threshold` so the inspector can show which rows are above/below the cutoff (the same threshold the M20 control sets, `schema_install.rs:211-227`).
- [ ] **Acceptance:** inspector JSON carries algebra-expansion and prediction-score-vs-threshold data; ranking/candidate output is unchanged (assert classic output byte-identity with inspector on/off).

#### Task 6 — Inspector UI panel (read-only)

**Files:** `source/src/yune-integration/response.ts`, new `source/src/YuneInspector.tsx`, `source/src/App.tsx`

- [ ] Pass the new optional debug fields through `translateResponse()` (`response.ts:26-64`) without altering the existing `RimeResult` shape used by the candidate panel (add an optional `debug` sub-object).
- [ ] Build a read-only `YuneInspector.tsx` panel showing, per keystroke: segments + `segment_tags`, each candidate's source/quality/comment/preedit, spelling-algebra expansion, filter pipeline, prediction scores vs threshold, and AI staging. **No toggles** in this panel — it is observation only. The honesty-gate exclusion-list features (uniquifier/single_char/always-on charset_filter, schema-owned templates, `_`-prefixed options) are *shown here* rather than fake-toggled elsewhere.
- [ ] Gate the panel behind an inspector on/off switch that calls `yune_typeduck_set_inspector_enabled`; default off.
- [ ] **Acceptance:** the app builds; the panel renders live inspector data for `nei`/`santai`; classic candidate output is unchanged whether the inspector is on or off.

#### Task 7 — Inspector browser evidence

**Files:** `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`, `e2e/yune-browser-smoke.md`

- [ ] Add Playwright assertions that, with the inspector on, typing `nei`/`santai`/`mgoi` renders correct per-candidate `source` labels (e.g. `table`, `sentence`, `completion`), a non-empty segments list, prediction-score-vs-threshold data for `santai`, and AI staging state transitioning `off → pending → ready` when AI is enabled.
- [ ] Assert the inspector is read-only: no candidate output difference between inspector-on and inspector-off for the same input.
- [ ] **Acceptance:** committed inspector evidence (screenshots + JSON snapshots) under the M22 evidence dir; classic byte-identity proven.

### Bucket 3 — Multi-schema + reverse lookup

#### Task 8 — Pre-compiled upstream artifacts for cangjie5 + luna_pinyin

**Files:** `third_party/typeduck-web/source/public/schema/` (add `.bin` artifacts), `scripts/` capture wrapper notes

- [ ] **Honesty gate (artifact provenance):** M18 can now build `.table.bin`/`.prism.bin`/`.reverse.bin`, so choose either Yune-generated artifacts or pre-compiled upstream artifacts for `cangjie5` and `luna_pinyin`. Stamp provenance to match the M19 schema-data identities, record which producer was used, and add `cangjie5.table.bin`, `cangjie5.prism.bin`, `cangjie5.reverse.bin`, `luna_pinyin.table.bin`, `luna_pinyin.prism.bin`, `luna_pinyin.reverse.bin` or a precise per-schema blocker.
- [ ] **Asset-budget risk (luna_pinyin):** `luna_pinyin.dict.yaml` is 471 KB source; its compiled `.table.bin` size is unknown (estimate 1–5 MB by analogy to `jyut6ping3.table.bin` 4.2 MB). Record measured `.bin` sizes and the total multi-schema bundle size; if the bundle threatens the browser WASM memory budget, document the size and consider lazy per-schema asset fetch (fetch a schema's `.bin` only on first switch) rather than eager bundling.
- [ ] If an upstream-`.bin` cannot be produced for a schema, record a precise blocker and ship that schema as **source-only / N/A** rather than a half-loaded broken schema.
- [ ] **Acceptance:** the three schemas have compiled artifacts checked in with provenance, or a recorded per-schema blocker; measured sizes documented.

#### Task 9 — Multi-schema worker loading + schema switcher

**Files:** `source/src/worker.ts`, `source/src/yune-integration/adapter.ts`, new `source/src/SchemaSelector.tsx`, `source/src/Preferences.tsx`/`App.tsx`

- [ ] Replace the single `SCHEMA_ID` constant (`worker.ts:132`) with a schema list `["jyut6ping3_mobile", "cangjie5", "luna_pinyin"]`; add the new schemas' compiled `.bin` to the binary `loadExtraSharedAssets([...], true)` block (`worker.ts:193-200`) alongside the existing source-YAML entries (`worker.ts:178-186`).
- [ ] Wire a schema switch through the existing `RimeSelectSchema` ABI slot (`abi.rs:301`). Per the research, the single active `TypeDuckRuntime` enforces one-active-runtime-per-Module (`adapter.ts` init/cleanup), so a switch must either (a) call `select_schema` on the live session if multi-schema deploy supports it, or (b) `cleanup()` + re-`init()` with the new `schemaId`. Choose the path that keeps userdb/IndexedDB state consistent and record which path is used.
- [ ] Add a `SchemaSelector.tsx` UI (a real schema switcher, not the existing Cangjie-3/5 sub-toggle) that drives the switch and updates status (`schema_id`/`schema_name` already in the status response, `response.ts` status fields).
- [ ] **Acceptance:** the playground lists three schemas; switching changes the active schema (status `schema_id` updates) and produces schema-correct candidates (`luna_pinyin` pinyin candidates, `cangjie5` shape-code candidates).

#### Task 10 — Reverse lookup for cangjie5 and luna_pinyin

**Files:** schema config for the loaded `cangjie5`/`luna_pinyin` browser schemas, `source/public/schema/` reverse dicts/`.reverse.bin`

- [ ] Enable reverse lookup for **both** new schemas using the in-engine `ReverseLookupTranslator`/`ReverseLookupFilter` already supported (`translator/mod.rs:1184-1325`, `filter/mod.rs:723-800`; loaders `schema_install.rs:340-393`/`:590-630`/`:764-778`). Provide each schema a `reverse_lookup` namespace (dictionary, `prefix`, optional `suffix`, optional `comment_format`) — mirror the existing `jyut6ping3.schema.yaml` reverse-lookup shape (`prefix: "\`c"`/`"\`p"`). Ship a generated or provenance-stamped `.reverse.bin` from Task 8.
- [ ] Preserve the locked `"; "` reverse-lookup joiner (`fork-parity-ledger.md:105`, F5; `translator/mod.rs:1295`) so reverse lookup shows candidate text + looked-up code(s).
- [ ] **Honesty note:** reverse-dict availability for `luna_pinyin` is uncertain (the live fixture has no reverse lookup defined); if no upstream reverse dict can be produced, record reverse lookup for that schema as N/A rather than shipping a dead `\`` trigger.
- [ ] **Acceptance:** typing the reverse-lookup trigger in `cangjie5` and `luna_pinyin` returns candidates with both the looked-up code and the schema comment; or a documented per-schema N/A.

#### Task 11 — Multi-schema + show_full_code + schema-switch browser evidence

**Files:** `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`, `e2e/yune-browser-smoke.md`

- [ ] Add Playwright tests that switch to `cangjie5` and assert `cangjie/show_full_code` now produces a real, browser-reachable before/after on side-lookup comments — converting the M20 browser-surface N/A (`docs/roadmap.md:336-337`) into real evidence.
- [ ] Assert the schema switcher is visible and functional now that more than one schema is loaded — converting M20's "hide lone schema" / switcher-N/A into a live switch with status before/after.
- [ ] Assert reverse lookup works for both new schemas (or document the per-schema N/A).
- [ ] **Acceptance:** `show_full_code` and the schema-switch surface have real browser before/after evidence; M20's two N/A items are explicitly resolved.

### Cross-cutting

#### Task 12 — Patch regeneration + reverse/forward check

**Files:** `third_party/typeduck-web/patches/yune-typeduck-runtime.patch`

- [ ] Regenerate the maintained TypeDuck-Web source patch after all `source/src/*` and `source/public/schema/*` changes; reverse-check (`git apply --reverse --check`) and forward-check (`git apply --check`) on a clean source tree.
- [ ] **Acceptance:** both checks exit 0.

#### Task 13 — Final verification + invariants

**Files:** evidence under `third_party/typeduck-web/e2e/results/m22-playground-multischema-inspector/`; `docs/roadmap.md`; `docs/requirements.md`

- [ ] `cargo fmt`; `cargo test -p yune-rime-api --test typeduck_web`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `npm --prefix packages/yune-typeduck-runtime test`; `npm --prefix packages/yune-typeduck-runtime run build`.
- [ ] Run the real TypeDuck-Web Playwright suite plus the `e2e/yune-browser-smoke.md` M22 procedure; capture evidence (run log, console log, JSON state snapshots, screenshots for each ACTIVE Bucket-1 control, each inspector field, each schema switch, and reverse lookup).
- [ ] **ABI no-diff gate:** `git diff -- crates/yune-rime-api/src/abi.rs crates/yune-rime-api/src/api_table.rs crates/yune-rime-api/src/candidate_api.rs` must be empty.
- [ ] **AI invariant gate:** re-run the M13 AI-off byte-identity and second-pass scenarios; confirm `process_key` stays provider-free and the inspector did not change AI defaults.
- [ ] `git diff --check` (whitespace) clean.
- [ ] Update `docs/roadmap.md` (move M22 into Completed/Planned as appropriate) and `docs/requirements.md` (add `M22-PLAY-0x` rows for the three buckets) only to reflect real landed status.
- [ ] **Acceptance:** all Rust/TS/browser gates pass or record a precise blocker; default ABI byte-for-byte unchanged; AI defaults unchanged.

---

## Acceptance criteria

- **Bucket 1:** every shipped ACTIVE toggle (`traditionalization`/`extended_charset`/`disabled`/`dictionary_exclude`, and `ascii_punct` only if browser evidence proves the M18 engine-backed behavior is visible) has real browser before/after evidence; any control that could not be made browser-observable is a documented browser-surface N/A, not a fake toggle.
- **Bucket 2:** the read-only inspector shows segments + `segment_tags`, full candidate SOURCE coverage, comments/preedit/quality, spelling-algebra expansion, the filter pipeline, prediction scores vs the weight threshold, and AI staging; it is opt-in and off by default; classic candidate output is byte-identical with the inspector on or off; the honesty-gate exclusion-list features are inspected, not toggled.
- **Bucket 3:** three schemas (`jyut6ping3_mobile` + `cangjie5` + `luna_pinyin`) load with a working schema switcher (via `RimeSelectSchema`/re-init) and reverse lookup enabled for both new schemas (or a documented per-schema N/A); `show_full_code` and the schema-switch surface — both M20 browser-surface N/A — now have real browser before/after evidence; new schemas ship with generated or provenance-stamped compiled artifacts and measured sizes.
- **Invariants:** default `RimeApi` table and `RimeCandidate` unchanged (`abi.rs`/`api_table.rs`/`candidate_api.rs` diff-free); AI default-off / classic-first / second-pass-only preserved; upstream-first baseline preserved; `ascii_punct` is not exposed as a working browser toggle without fresh before/after evidence; userdb persistence stays browser-limited (IndexedDB/in-memory).
- **Honesty:** every ACTIVE control changes candidate/commit/status/persisted config; every display change proves a rendering diff; everything else is a guided scenario or documented N/A.
- The maintained TypeDuck-Web patch reverse- and forward-checks cleanly; full Rust/TS/browser gates pass or record a precise blocker.

---

## Deliverables

- **Bucket 1 toggles:** extended `RimePreferences`/`DEFAULT_PREFERENCES` (`types.ts`, `consts.ts`); adapter `setOption`/`customize` mappings for `traditionalization`/`extended_charset`/`disabled`/`dictionary_exclude` (`source/src/yune-integration/adapter.ts`) + `adapter-filesystem.test.ts` coverage; honest grouped UI controls (`App.tsx`, `Preferences.tsx`, `Inputs.tsx`).
- **Bucket 2 inspector:** additive full-source + debug-field marshalling and optional `yune_typeduck_set_inspector_enabled` / `yune_typeduck_candidate_metadata` helpers (`crates/yune-rime-api/src/typeduck_web.rs`); additive read-only `Engine` accessors (`crates/yune-core/src/engine.rs`); `YuneInspector.tsx` read-only panel + `response.ts` debug passthrough; tests in `crates/yune-rime-api/tests/typeduck_web.rs`.
- **Bucket 3 multi-schema:** generated or provenance-stamped `cangjie5.*`/`luna_pinyin.*` `.table.bin`/`.prism.bin`/`.reverse.bin` artifacts under `source/public/schema/`; multi-schema worker loading (`worker.ts`); `SchemaSelector.tsx` + `RimeSelectSchema` wiring; reverse-lookup config for both new schemas.
- **Evidence:** `third_party/typeduck-web/e2e/yune-typeduck.spec.ts` honesty/inspector/multi-schema assertions; `e2e/yune-browser-smoke.md` M22 section; committed evidence under `third_party/typeduck-web/e2e/results/m22-playground-multischema-inspector/` (per-control before/after, inspector snapshots, schema-switch + reverse-lookup screenshots, ABI no-diff log, AI-invariant log).
- **Patch + docs:** regenerated `patches/yune-typeduck-runtime.patch`; `docs/roadmap.md` + `docs/requirements.md` (`M22-PLAY-0x`) updated to landed status.
- **This plan:** `docs/plans/m22-plan-web-playground-multischema-inspector.md`.
