# Requirements: Yune

**Defined:** 2026-04-28 **Core Value:** Yune should preserve predictable classic RIME input while making AI/LLM assistance a first-class, local-first, non-blocking source of candidates, ranking, context, and memory.

> **Note (2026-06-17):** The GSD `.planning/` system has been retired. This requirement list and its statuses are preserved here; the **Phase** references (e.g. in the Traceability table) are historical GSD labels — now only in git history — kept for context. The live roadmap is [`roadmap.md`](./roadmap.md); historical milestone context is in [`ledgers/milestone-history.md`](./ledgers/milestone-history.md); decisions are in [`decisions.md`](./decisions.md); conventions in [`conventions.md`](./conventions.md).

## v1 Requirements

Requirements for the compatibility milestone (historically GSD phases 1–5).

### CLI Frontend Surrogate

- [x] **CLI-01**: Developer can initialize `yune-rime-api` from `yune-cli` with explicit shared data and user data directories.
- [x] **CLI-02**: Developer can deploy and select schemas through the CLI using the RIME ABI path, not direct `yune-core` fixture setup.
- [x] **CLI-03**: Developer can create and destroy RIME sessions from the CLI and process interactive key events through `RimeProcessKey`.
- [x] **CLI-04**: Developer can render commit text, preedit, candidate page, highlight index, and status after each CLI key event.
- [x] **CLI-05**: Developer can replay transcript key sequences through the RIME ABI and compare the transcript against expected output.

### Frontend ABI Validation

- [x] **ABI-01**: Developer can run the current ABI against at least one real frontend client or native frontend-like loading path and record observed gaps.
- [x] **ABI-02**: Struct layout, lifetime, notification, deployment, and session gaps found by frontend validation have focused regression coverage.
- [x] **ABI-03**: Runtime resource IDs from C APIs and schema YAML reject path traversal, absolute paths, platform separators, and other non-logical IDs before filesystem joins.
- [x] **ABI-04**: Process-wide session, module, notification, switcher, and runtime state behavior remains deterministic under repeated initialize/finalize and session lifecycle operations.

### Schema Pipeline Depth

- [x] **SCHEMA-01**: `speller` behavior covers previous-match segment splitting and non-auto-commit composition behavior beyond current focused auto-commit paths.
- [x] **SCHEMA-02**: `editor`, `navigator`, and `selector` behavior covers deeper segment/selection span semantics and navigator fallback interactions beyond current focused overrides.
- [x] **SCHEMA-03**: `chord_composer`, `shape_processor`/`shape_formatter`, `punct_segmentor`, and `fallback_segmentor` behavior covers larger-chain and remaining lifecycle edge cases.
- [x] **SCHEMA-04**: Remaining librime gear behavior around `memory`, `poet`/`grammar`, `contextual_translation`, and `unity_table_encoder` has explicit compatibility increments or documented deferrals.
- [x] **SCHEMA-05**: Full spelling algebra, correction/tolerance search interaction, OpenCC conversion data, and distribution-scale processor/segmentor/translator/filter chains are compared directly against librime behavior.

### Dictionary And Compiled Data

- [x] **DATA-01**: Runtime dictionary loading can consume compiled `.table.bin`, `.prism.bin`, and `.reverse.bin` payloads beyond the current metadata slice.
- [x] **DATA-02**: Dictionary rebuild execution handles source-vs-prebuilt fallback, table/prism/reverse checksum decisions, pack checksum chaining, and compiled output freshness.
- [x] **DATA-03**: Stem-column data, reverse-db `dict_settings`, preset-vocabulary phrase injection, and UniTE-style encoder payloads are consumed where librime schemas rely on them.
- [x] **DATA-04**: Correction data and tolerance search inputs are represented in the compiled-data path sufficiently for schema-loaded lookup compatibility.

### User Dictionary Compatibility

- [x] **USERDB-01**: User dictionary storage supports librime-compatible LevelDB/userdb behavior or a documented compatible abstraction beyond the current plain text shim.
- [x] **USERDB-02**: Snapshot backup, restore, recovery, sync, and transaction rollback behavior match librime-observable semantics.
- [x] **USERDB-03**: Learning, frequency updates, predictive lookup, and backdated scan behavior are represented in runtime candidate ranking and userdb persistence.

### Engineering Structure And Quality

- [x] **QUAL-01**: Every new compatibility slice starts with an owning implementation module, owning test module, and explicit librime comparison target.
- [x] **QUAL-02**: `lib.rs` and `main.rs` remain facades/orchestration glue; temporary spike code is extracted before a second related behavior lands.
- [x] **QUAL-03**: Remaining oversized compatibility tests are split only along behavior ownership boundaries, without mixing mechanical moves and behavior changes.
- [x] **QUAL-04**: Quality gates for implementation phases include focused tests, `cargo fmt`, relevant `cargo test` targets, and workspace tests when shared behavior changes.

## v2 Requirements

Requirements for the next validation milestone before AI-native product work.

### Real Frontend Validation

- [x] **FRONTEND-VALIDATION-01**: A host-shaped native loader or real frontend path validates `rime_get_api`, setup, initialize, deploy, schema selection, session lifecycle, key processing, context/status reads, commit text, and teardown.
- [x] **FRONTEND-VALIDATION-02**: TypeDuck-Web-style browser/WebAssembly integration is attempted as a real application frontend path, with wrapper gaps and browser-specific limits documented.
- [x] **FRONTEND-VALIDATION-03**: Squirrel or a macOS frontend-shaped integration is attempted after the browser/WebAssembly path, with reproducible blockers documented if direct integration cannot run locally.
- [x] **FRONTEND-VALIDATION-04**: ibus-rime or fcitx-rime validation is scoped after the macOS path, with environment requirements and lifecycle differences documented.
- [x] **FRONTEND-VALIDATION-05**: Frontend-observed ABI/runtime mismatches are captured as notes, fixtures, or focused regression tests before being fixed.

### Frontend-Sensitive Benchmarks

- [x] **BENCH-01**: Benchmarks record baseline latency for session create/destroy, per-key `RimeProcessKey`, schema deployment, dictionary loading, and userdb learning/sync paths.
- [x] **BENCH-02**: Benchmark output is reproducible enough to compare future frontend or AI-native changes against the compatibility foundation baseline.

## TypeDuck-Web Browser Integration Requirements

Requirements for the next integration milestone. These requirements turn the Phase 6 TypeDuck-Web validation and the seed Rust adapter into a browser-usable path before AI-native product work begins.

**M9 completed real-assets validation.** The build-out (WASM export contract, TS bridge, browser filesystem) landed, and the WASM artifact now builds as loadable Emscripten `yune-typeduck.js`/`.wasm` with a Node smoke for one `yune_typeduck_*` call plus one `FS` operation. A post-review audit found the first WI-4 browser matrix used the placeholder echo path for candidate evidence. HR-1 proves the patched TypeDuck-Web worker can load real `jyut6ping3_mobile` assets and render `nei` candidates (`你`, `呢`, `尼`) in a real browser. HR-2 resolves the startup `setOption` export/wrapper/adapter gap, HR-3 proves browser `deploy()` returns true with real assets after adding the plain `jyut6ping3.schema.yaml` preload, and HR-4 proves live-worker persistence sync plus real reload survival. HR-5 reruns the full browser matrix against real assets, including paging, deletion, phrase commit, dictionary-panel rendering, and zero warning/error console entries after the post-review pure-modifier delete-path fix. Rich dictionary-comment byte parity is committed in `cantonese_parity`; the browser-shaped native rich-comment test also asserts the full real-assets path when local v1.1.2 oracle build assets are present. HR-6 locks the shared reverse-lookup joiner and schema-prompt bytes against the TypeDuck v1.1.2 oracle. HR-7 closes M9 with **GO WITH CONDITIONS** for gated AI-native frontend exposure.

### WASM Build And Export Contract

- [x] **TYPEDUCK-WASM-01**: Developer can build the TypeDuck adapter for the intended Emscripten/WASM target as a loadable JS+WASM module.
- [x] **TYPEDUCK-WASM-02**: The browser build preserves all required `yune_typeduck_*` exports for JS callers and exposes the Emscripten runtime methods needed by the TypeScript host.
- [x] **TYPEDUCK-WASM-03**: Native adapter contract tests remain the deterministic fallback when local browser/WASM tooling is unavailable.

### TypeScript Bridge And Runtime Package

- [x] **TYPEDUCK-JS-01**: A TypeScript wrapper exposes init, process-key, candidate action, deploy, customize, set-option, and cleanup operations.
- [x] **TYPEDUCK-JS-02**: The wrapper centralizes JSON parsing and pairs every owned adapter response with `yune_typeduck_free_response`.
- [x] **TYPEDUCK-JS-03**: Browser keycode/mask mapping is explicit and covered by deterministic tests.
- [x] **TYPEDUCK-JS-04**: Runtime lifecycle documentation makes the one-active-process-global-service constraint visible to TypeDuck-Web callers.

### Browser Filesystem And Persistence

- [x] **TYPEDUCK-FS-01**: Browser setup creates the expected shared data, user data, and deployed build directory layout before adapter init.
- [x] **TYPEDUCK-FS-02**: Schema and dictionary assets can be preloaded into the virtual filesystem before adapter init.
- [x] **TYPEDUCK-FS-03**: IDBFS or equivalent persistence syncs before init and after deploy, customize, and userdb mutations.
- [x] **TYPEDUCK-FS-04**: Missing assets, failed sync, and stale deployed config recovery paths are documented and tested where possible.

### TypeDuck-Web App Integration And E2E

- [x] **TYPEDUCK-E2E-01**: The upstream TypeDuck-Web repository is cloned or vendored in a reproducible test location, and its current librime/WASM bridge seam is identified.
- [x] **TYPEDUCK-E2E-02**: TypeDuck-Web is patched or configured so its input-engine binding calls the Yune TypeScript bridge instead of the original librime bridge, with candidate text/comment/highlight mapped from the runtime response shape.
- [x] **TYPEDUCK-E2E-03**: Real TypeDuck-Web browser validation covers composition, candidate paging, selection, deletion, commit output, deploy, customize, persistence smoke flows, and dictionary-panel rendering, with PASS evidence recorded from the HR-5 real-assets matrix. Rich dictionary-comment byte parity is committed in `cantonese_parity`; the browser-shaped native rich-comment test is explicitly skipped unless local v1.1.2 oracle build assets are present.
- [x] **TYPEDUCK-E2E-04**: Integration findings end with a go/no-go recommendation for exposing AI-native behavior through real frontends; HR-7 records **GO WITH CONDITIONS**.

## M12 Upstream Oracle And Behavioral Parity Requirements

**Status: complete.** Upstream `rime/librime 1.17.0` is the default core oracle target. TypeDuck `v1.1.2` remains a compatibility-profile oracle for TypeDuck-Web/Windows only. The official upstream Windows MSVC release binary is the behavioral-capture oracle; local source builds are a reproducibility check rather than the primary capture source. Later M17/M18 closeouts resolved the former sentence/lattice and processor blockers with fresh upstream fixtures.

- [x] **UPSTREAM-ORACLE-01**: Upstream `rime/librime 1.17.0` and commit `33e78140250125871856cdc5b42ddc6a5fcd3cd4` are pinned as the default core oracle in docs and fixture provenance.
- [x] **UPSTREAM-ORACLE-02**: Oracle fixture/golden naming distinguishes upstream core fixtures from TypeDuck profile fixtures, e.g. `upstream-1.17.0/` vs `typeduck-v1.1.2/`.
- [x] **UPSTREAM-AUDIT-01**: Existing compatibility coverage is audited for TypeDuck-only assumptions that should not define core Yune behavior.
- [x] **TYPEDUCK-PROFILE-01**: TypeDuck-specific ABI, comment, Cantonese/Jyutping, and native Windows frontend behavior remains documented and verified as profile-only; the default upstream core ABI remains separate.
- [x] **UPSTREAM-BEHAVIOR-01**: Upstream `luna_pinyin` behavioral fixtures are captured from the official `1.17.0` release binary for curated mechanics, full `ni` selection, action/paging/commit, reverse lookup, punctuation/symbols, option toggles, and later M17/M18 sentence/processor slices.
- [x] **UPSTREAM-BEHAVIOR-02**: Full-dictionary `ni` selection uses every exact-code `luna_pinyin.dict.yaml` row plus relevant `essay.txt` rows for in-scope candidates, with provenance checks preventing default/zero essay-weight ranking.
- [x] **UPSTREAM-BEHAVIOR-03**: Menu-dependent behavior is compared through Yune's real `Engine` path for paging, numeric selection, space commit, reverse lookup, punctuation, and supported option behavior.
- [x] **UPSTREAM-BEHAVIOR-04**: Unsupported upstream behavior remains explicit: former `zhongguo` sentence/lattice and punctuation processor blockers were closed by M17/M18 fixtures, while learned `.gram`/octagram grammar and contextual translation remain deferred until a named target needs them.
- [x] **UPSTREAM-BEHAVIOR-05**: `oracle_fixture_provenance` enforces non-circular fixture metadata, source-row policies, schema repository commits, capture commands, and absence of local absolute oracle-cache paths across all upstream `luna_pinyin` fixtures.
- [x] **UPSTREAM-BEHAVIOR-06**: M17 captures upstream `luna_pinyin` sentence and lattice goldens, implements the null-grammar poet path with `kPenalty = -13.815510557964274`, and keeps TypeDuck `jyut6ping3` sentence tuning isolated.

## TypeDuck-Windows Native IME Contract Requirements

**Status: complete as a TypeDuck compatibility profile.** A first pass landed (Phases 11-16), M9 web validation is complete, and the archived pre-M12 native Windows package smoke has been superseded by current M10 T1/T2 profile package, build/link, packaged lifecycle evidence, and stock TypeDuck-Windows real-server IPC smoke evidence against `rime_get_typeduck_profile_api()`. The shared comment requirement is covered for the current v1.1.2 oracle slices; captured Cantonese engine fixtures are active, and the M10 T3 smoke now proves key input/output through the native TypeDuck-Windows/weasel IPC path. This T3 proof is a stock server/client IPC smoke, not an interactive TSF typing or visible candidate-panel rendering smoke; those move to the Phase 2 Windows product/frontend track. These requirements target that native path and no longer define Yune's active core oracle milestone.

- [x] **WIN-TEST-01**: Windows `cargo test --workspace` has a trustworthy green baseline, including portable signature timestamp shape and test-only poison-lock recovery.
- [x] **WIN-ABI-01**: `config_list_append_{string,bool,int,double}` helper behavior is implemented and exposed through the named, opt-in M19 TypeDuck-profile accessor; the default upstream `rime_get_api()` does not expose these fork-only slots.
- [x] **WIN-ORACLE-01**: The TypeDuck-HK/librime v1.1.2 binary and pinned schema are captured as a reproducible oracle, or a precise blocker is documented.
- [x] **WIN-COMMENT-01**: Candidate comment semantics match the v1.1.2 oracle for dictionary lookup payloads, reverse lookup joins, and prompt/schema identity. Dictionary lookup payload bytes, schema-prompt bytes, and reverse-lookup joiner coverage are oracle-backed.
- [x] **WIN-BUILD-01**: Yune produces a current TypeDuck-profile native Windows package (`rime.dll`, import `.lib`, upstream-shaped default headers, and `rime_typeduck_profile_api.h`) and the package script loads the packaged DLL through `rime_get_typeduck_profile_api()`.
- [x] **WIN-PARITY-01**: Cantonese/Jyutping parity regression coverage locks the captured v1.1.2 engine behavior in active `cantonese_parity` tests; schema-menu/userdb observations remain frontend/T3 evidence limits.
- [x] **WIN-FRONTEND-01**: TypeDuck-Windows builds/links against the Yune package and passes a stock real-server IPC smoke. Stock `TypeDuckServer.exe` starts from `output\`, loads packaged Yune `output\rime.dll`, and stock `TestTypeDuckIPC.exe /console` returns a nonzero session, sends `ngohaig` key events, and receives `status.schema_id=jyut6ping3` plus candidate/context data. Tracked evidence: `docs/plans/completed/m10-evidence/t3-stock-real-server/`. Caveat: interactive TSF typing, visible candidate-window rendering, and candidate-panel UI behavior are deferred to the Phase 2 Windows product/frontend track.

## P2-WIN-02 TypeDuck Windows Boundary Compatibility Requirements

**Status: complete; Yune boundary fixed and non-Yune TSF input-delivery blocker classified.** P2-WIN-02 closes the Yune-side raw TypeDuck `jyut6ping3` `ngohaig` boundary bug found by TypeDuck-Windows Phase 0C without widening the default upstream ABI. Evidence: `docs/reports/evidence/p2-win02-boundary-compat-2026-06-22/`; plan: [`docs/plans/completed/p2-win02-plan-typeduck-boundary-compat.md`](./plans/completed/p2-win02-plan-typeduck-boundary-compat.md).

- [x] **P2-WIN02-BOUNDARY-01**: Phase 0C `ngohaig` raw comment evidence is promoted into a Yune-owned TypeDuck `v1.1.2` fixture with locked provenance and native parity tests.
- [x] **P2-WIN02-BOUNDARY-02**: Yune emits TypeDuck `v1.1.2` rich `\f\r1,` comment bytes for the Windows-facing `jyut6ping3` `ngohaig` path through both core and Rime ABI tests.
- [x] **P2-WIN02-BOUNDARY-03**: Compiled TypeDuck `dictionary_lookup_filter` side dictionaries preserve lookup records, and workspace deployment rebuilds those side artifacts instead of relying on stale external compiled data.
- [x] **P2-WIN02-BOUNDARY-04**: The TypeDuck-Windows uninitialized `RimeConfig` boundary is tolerated without freeing foreign pointers, and repeated session/schema lifecycle remains responsive in focused ABI tests.
- [x] **P2-WIN02-BOUNDARY-05**: The rebuilt TypeDuck Windows package passes the packaged DLL smoke, direct `RimeCandidate.comment` byte probe, TypeDuck-Web regression gate, and stock TypeDuck-Windows IPC smoke with rich comments.
- [x] **P2-WIN02-BOUNDARY-06**: Interactive Notepad TSF smoke proves candidate commit, produces a newly classified non-Yune blocker with committed evidence, or the user explicitly accepts IPC-only closure. The approved reruns produced the newly classified non-Yune blocker path: session-scoped TypeDuck activation succeeded and the Yune-backed server stayed alive, but Notepad still received raw ASCII, so the remaining issue belongs to TSF input-delivery/frontend-shell work.

## Future Requirements

Deferred beyond the TypeDuck-Web browser integration milestone. Tracked but not in the current roadmap.

### Plugin Compatibility

- **PLUGIN-01**: Yune can load or adapt librime C++ plugin ABI extensions.
- **PLUGIN-02**: Lua, octagram, predict, proto, and other distribution plugin ecosystems have migration paths.

### Product Frontend

- **FRONTEND-01**: Yune ships a new graphical end-user frontend.
- **FRONTEND-02**: Yune-specific UI features expose optional AI ranking and contextual completion controls.

### iOS Keyboard Developer Track

- **IOS-DEV-01**: Yune provides a documented iOS package/host contract for keyboard developers, separate from the default upstream `RimeApi` table and without changing `RimeCandidate`.
- **IOS-DEV-02**: iOS resource deployment is explicit: schemas, dictionaries, OpenCC data, and userdb storage are bundled or generated in a sandbox-safe location without arbitrary filesystem paths or startup recompilation surprises.
- **IOS-DEV-03**: Swift/Obj-C integration defines keyboard-extension lifecycle, memory, persistence, and privacy constraints before TypeDuck iOS exposure is claimed.
- **IOS-DEV-04**: Mobile-specific behavior such as near-key correction maps or keyboard-layout differences is data/config-driven or UI-owned, not hardcoded as desktop-vs-mobile engine branches.

### AI Extension Layer

- [x] **AI-01**: Engine exposes an `AiCandidateProvider` interface and staged, input-keyed AI results without replacing classic translators. S1 implements this for the direct CLI mock path.
- [x] **AI-02**: Candidate ranking supports local model and rule-backed implementations with deterministic timeout/fallback behavior. S2 covers the background worker, input-keyed fallback, fixed-point confidence metadata, and confidence-ordered AI merge; S5 adds the local rule-backed provider.
- [x] **AI-03**: Contextual phrase and sentence completion can produce source-labeled AI candidates without allowing AI candidates to auto-commit by default. S1 covers source labeling and the no-default-auto-commit gate; S5 adds contextual local-model completions.
- [x] **AI-04**: Context providers define what app, field, preceding text, cursor, schema, and candidate-list data may be shared with AI providers. S3 implements `AiContext` plus `EngineAiContextProvider` snapshots.
- [x] **AI-05**: Memory store records user vocabulary, phrase preferences, and domain terms through explicit, inspectable, clearable policy. S4 implements `MemoryStore`, clear/disable controls, snapshot import/export, and `.ai-memory` namespace helpers.
- [x] **AI-06**: Privacy policy disables learning and remote calls for sensitive contexts and keeps classic input fully functional when AI is disabled. S3 blocks remote calls; S4 applies the same privacy gate to AI memory writes.
- [x] **AI-07**: CLI frontend surrogate can demonstrate AI candidate/ranking behavior with mock and local providers before native frontends expose it. S1 covers `yune-cli run --ai-provider mock`; S5 adds `--ai-provider local`.

## M13 AI-native Frontend Exposure Requirements

**Status: complete for TypeDuck-Web.** M13 exposes the M11 local AI layer through TypeDuck-Web only, default-off and local-first, with the key path still provider-free. Additional native frontend exposure remains future work.

- [x] **M13-AI-01**: `yune_typeduck_process_key` remains provider-free and classic-first; AI provider work runs only through the second-pass `yune_typeduck_stage_ai` path.
- [x] **M13-AI-02**: Browser AI is default-off, can be toggled without redeploy, and `set_ai_enabled(false)` clears any staged result for the current input.
- [x] **M13-AI-03**: AI candidates render after the classic top candidate, never at index 0, with source labels derived from engine snapshot data aligned to the rendered page; `RimeCandidate` and the upstream `RimeApi` table remain unchanged.
- [x] **M13-AI-04**: Browser commit safety preserves classic default commit behavior; AI rows never auto-commit and require explicit selection.
- [x] **M13-AI-05**: Explicit AI commits do not touch librime userdb; under the sensitive browser default, AI-memory learning is suppressed and no `.ai-memory` persistence is written.
- [x] **M13-AI-06**: Real TypeDuck-Web browser evidence covers AI-off byte identity, AI-on source-labeled second-pass rows, no auto-commit, explicit AI selection, and zero warning/error console entries.

## M14–M16 TypeDuck-Web Fork Parity Requirements

**Status: M14 capture complete; M15 engine parity complete; M16 browser validation complete with documented browser-surface limits.** Complete the TypeDuck `jyut6ping3` target so the TypeDuck-Web example behaves like the fork. Oracle-measured against TypeDuck-HK v1.1.2; `jyut6ping3` is dictionary-driven and does **not** require the upstream language model (Track 2 / M17). See roadmap M14–M16 and `decisions.md` D-27.

- [x] **TYPEDUCK-PARITY-01**: A v1.1.2 capture path is established by parameterizing the scenario-capable upstream probe's oracle identity (modules/distribution/provenance) — or a thin v1.1.2 wrapper — and Cantonese goldens are captured from the v1.1.2 oracle binary for `combine_candidates`, `show_full_code`, `enable_sentence`, completion/prediction, and correction at multiple input lengths.
- [x] **TYPEDUCK-PARITY-02**: The oracle-observable surface for schema-menu hiding (`hide_lone_schema`/`hide_caret`) is identified (config API, schema-list/switcher API, or TypeDuck-Web UI state) and emitted behavior is captured — not static config inspection alone.
- [x] **TYPEDUCK-PARITY-03**: A feasibility spike determines whether per-entry userdb pronunciations are capturable via the levers user-dict export/import/seed hooks; if not, the gap is documented as a fork-only deferral with the precise blocker.
- [x] **TYPEDUCK-PARITY-04**: `combine_candidates` (candidate grouping) and `show_full_code` (cangjie preedit algebra) are implemented and pass the captured goldens through Yune's real engine path.
- [x] **TYPEDUCK-PARITY-05**: `enable_sentence`, completion ranking, and correction/tolerance tuning are refined to pass the captured goldens.
- [x] **TYPEDUCK-PARITY-06**: OpenCC `hk2s` coverage is expanded from the built-in slice to the full conversion data the jyut6ping3 simplifier needs.
- [x] **TYPEDUCK-PARITY-07**: The TypeDuck-Web browser matrix passes for the app-exposed `jyut6ping3_mobile` surface plus M13 AI, while deploy-only variants (`common:/separate_candidates`, `common:/show_full_code`), schema-menu UI hiding, correction UI detail, and per-entry userdb pronunciation are explicitly documented as browser/userdb inspection limits backed by M14/M15 oracle evidence.

## Fork Parity Backlog — Cantonese engine-parity (complete)

**Status: complete.** Derived from the full Cantoboard + TypeDuck fork-vs-`1.17.0` audit in [`ledgers/fork-parity-ledger.md`](./ledgers/fork-parity-ledger.md). These were genuine fork deltas Yune needed to preserve or explicitly decline (distinct from the upstream-depth Track 2 M17–M19 work). M14–M16 closed the _captured_ browser surface; these were the _uncaptured / partial_ deltas the goldens did not exercise. Each completed implementation was measured against the v1.1.2 oracle or closed by an explicit product decision.

- [x] **FORK-PARITY-01**: The Cantonese 容錯 (fuzzy) spelling-algebra ruleset (`lv1_laanjam`, `lv2_upper`, `shortcuts`, `lv2_lower`, abbreviation — including the `ng→m` rule behind the F1 `m` case) runs on the real ~127k-entry `jyut6ping3` dictionary, with a real-dictionary golden.
- [x] **FORK-PARITY-02**: `PreferUserPhrase` weighted gate — a user-dictionary phrase outranks a competing system phrase only with a longer code, or equal-length code and weight ≥ the system phrase.
- [x] **FORK-PARITY-03**: Per-entry userdb element/full-code pronunciation recovery, including multi-syllable sentence commits preserving all primary lookup codes.
- [x] **FORK-PARITY-04**: `hide_lone_schema` — suppress the schema switcher when only one schema exists (`838e3d41`).
- [x] **FORK-PARITY-05**: Correction fidelity — edit-distance-scaled penalty + discard non-minimal-distance corrections (`kCorrection`, `81e13724`), an `enable_correction` gate independent of `enable_completion` (`585f4656`), and restricting corrections to normal spellings (`733eedc8`→`2f79c3ab`).
- [x] **FORK-PARITY-06**: `letter_to_tone`/`tone_to_letter` — type `v`/`x`/`q` for tones via the TypeDuck profile's `preedit_format` path.
- [x] **FORK-PARITY-07**: TypeDuck-profile `全形`/`半形` state labels (vs upstream `全角`/`半角`) — schema-asset/golden change only, no Rust change.
- [x] **FORK-PARITY-08**: Product decision and implementation: do **not** chase full TypeDuck prediction-ranking byte parity; preserve upstream `1.17.0` long-entry completion (`santai` can surface `身體健康`) and expose profile controls for `prediction_never_first` plus raw-weight/frequency thresholds.
- [x] **FORK-PARITY-09**: Product decision: `display_languages` gloss-column selection lives in TypeDuck-Web UI; the engine continues to emit stable, ordered lookup payloads without adding engine-side language filtering.

## M20 Web Demo Showcase Controls Requirements

**Status: complete.** M20 is a web/demo track for this repo's patched internal TypeDuck-Web harness, not a reopened M13 and not the separately cloned `TypeDuck-HK/TypeDuck-Web` product. It exposes already-supported Yune behavior through honest UI controls and guided scenarios while preserving the M9/M13/M16 browser gates and the upstream-first ABI constraints. Browser evidence is under `apps/yune-web/e2e/results/m20-showcase-controls/`.

- [x] **M20-DEMO-01**: TypeDuck-Web exposes only runtime-backed active controls: schema/deploy-time controls flow through `customize()` plus deploy, live session controls flow through `setOption()`, display-only controls are grouped separately, and no new `RimeApi`, `RimeCandidate`, or `yune_typeduck_*` export is added for UI convenience.
- [x] **M20-DEMO-02**: Prediction controls are honest and profile-aligned: `prediction_never_first` defaults on, and the UI exposes one prediction threshold control because the frequency/weight config aliases drive the same engine threshold; the fine-grained threshold UI has a real-assets-calibrated `santai` cutoff plus documented range bounds in browser evidence.
- [x] **M20-DEMO-03**: Static or default-on engine features are guided scenarios, not fake toggles: long-entry prediction (`santai` -> `身體健康`), Cantonese fuzzy/容錯, letter-to-tone, reverse lookup/dictionary panels, and AI second-pass behavior are demonstrable without misrepresenting their configurability.
- [x] **M20-DEMO-04**: The internal TypeDuck-Web harness and `@yune-ime/typeduck-runtime` subtrees have local `AGENTS.md` guidance covering patch discipline, runtime wrapper boundaries, browser evidence, the control-honesty rule, and the distinction between the harness, the runtime bridge, and the real TypeDuck-Web web IME product.
- [x] **M20-DEMO-05**: Real browser evidence includes an honesty gate proving supported controls with visible before/after output where the `jyut6ping3_mobile` browser surface can render it: AI candidates, `combine_candidates`, `prediction_never_first`, prediction threshold, live `setOption()` controls, display-language/Jyutping rendering, and guided scenarios. Deploy-time controls whose current browser panel effect is not independently visible keep real persisted `jyut6ping3_mobile.custom.yaml` assertions, but are not counted as candidate-output proof. Input Memory has a visible learned-prediction on-state and an explicit browser-surface N/A for memory-off suppression; Auto-correction now has real `nri` browser before/after evidence, with correction off rendering the v1.1.2 prefix fallback rows and correction on rendering `你` first. The full oracle row set and commit previews remain engine-proven by `cantonese_parity`. `ascii_punct` now has M18 engine behavior but remains absent as a working browser toggle until a browser-visible evidence slice proves it. The fixed `jyut6ping3_mobile` browser schema lacks a `cangjie` namespace, so Reverse code display / Cangjie / `show_full_code` are labeled current-surface N/A rather than fake working toggles.
- [x] **M20-DEMO-06**: The internal TypeDuck-Web harness is documented and maintained as Yune's canonical browser playground: every browser-safe supported engine feature is reachable through an active control or guided scenario, and unsupported or deferred behavior is clearly absent or labeled rather than partially exposed.
- [x] **M20-DEMO-07**: Headline TypeDuck profile toggles are not lost in the playground: `translator/combine_candidates` is an active control whose UI default is documented as an M20 grouped-candidate demo choice while the raw mobile browser assets still record `common:/separate_candidates`, and `show_full_code` is either exercised through a browser-reachable Cangjie side-lookup scenario/control or explicitly recorded as N/A for the current `jyut6ping3_mobile`-only surface.

## M22 Web Playground Requirements

- [x] **M22-PLAY-01**: The internal TypeDuck-Web playground has an opt-in read-only engine inspector showing segment tags, candidate source/quality/ preedit/comment details, spelling-algebra expansion, filter audit, prediction score/threshold data, and AI staging state.
- [x] **M22-PLAY-02**: The inspector is off by default, preserves classic candidate response identity when disabled, has committed browser evidence, and does not change the default `RimeApi`, `RimeCandidate`, or ABI layout files.
- [x] **M22-PLAY-03**: Remaining browser-safe controls (`traditionalization`, `extended_charset`, `disabled`, `dictionary_exclude`, and any `ascii_punct` exposure after M18) are exposed only when they clear the M20 honesty gate with real browser before/after evidence; otherwise they are documented as browser-surface N/A. M22 exposes the first four controls with status/candidate/persisted-config evidence and keeps `ascii_punct` absent.
- [x] **M22-PLAY-04**: The playground loads `jyut6ping3_mobile`, `cangjie5`, and `luna_pinyin` through a real schema switcher with reverse lookup for the new schemas, using generated or provenance-stamped compiled artifacts with measured browser asset sizes.

## M24 TypeDuck-Web Dogfooding Requirements

**Status: complete.** M24 closed the first manual dogfooding/demo-hardening batch for the internal TypeDuck-Web playground. The closed issue ledger and evidence index live in [`plans/completed/m24-plan-typeduck-web-dogfooding.md`](./plans/completed/m24-plan-typeduck-web-dogfooding.md), with browser evidence under `apps/yune-web/e2e/results/m24-dogfooding/`.

- [x] **M24-DOGFOOD-REQ-01**: The dogfood browser harness records issue-scoped evidence under `m24-dogfooding/<issue-id>/`, and startup evidence includes the worker phase markers, `yune-typeduck.js`/`.wasm` asset identity, and loaded shared schema assets.
- [x] **M24-DOGFOOD-REQ-02**: Candidate rendering strips literal `\f`, `\r`, and `\v` controls from visible rows while preserving dictionary parsing, and compound candidates keep row text compact with details in the dictionary panel.
- [x] **M24-DOGFOOD-REQ-03**: Browser/runtime correctness fixes remain fixture-backed where engine output is involved: `jigaajiusihaa` ordering is locked to TypeDuck `v1.1.2`, page-size customization writes `menu/page_size`, and the historical M24 Jyutping reverse lookup path uses packaged browser assets; M25 updates the current web profile to bare `` `zhe `` for `luna_pinyin`.
- [x] **M24-DOGFOOD-REQ-04**: The TypeDuck-Web playground settings are Cantonese-first and grouped by engine/session/display/frontend purpose, with checklist display languages, real schema names, labeled engine status, and full Chinese typeface family names.
- [x] **M24-DOGFOOD-REQ-05**: The dogfood UI stack is Vite + React + Tailwind CSS plus small local components only; DaisyUI is removed from package metadata, Tailwind config, and local component class usage.

## M25 TypeDuck-Web Dogfooding Round 2 Requirements

**Status: complete.** M25 closed the second manual dogfooding round for the internal TypeDuck-Web playground. The completed ledger and closeout evidence index live in [`plans/completed/m25-plan-typeduck-web-dogfooding-round-2.md`](./plans/completed/m25-plan-typeduck-web-dogfooding-round-2.md), with browser evidence under `apps/yune-web/e2e/results/m25-dogfooding/`.

- [x] **M25-DOGFOOD-REQ-01**: Every closed M25 row has issue-scoped browser JSON/screenshot evidence, an owning Playwright or native test listed in the ledger closeout table, and TypeDuck-Web source changes regenerated into `apps/yune-web/patches/yune-web-runtime.patch` with reverse/forward patch checks.
- [x] **M25-DOGFOOD-REQ-02**: Browser startup uses the release-mode WASM build, records phase timing evidence, reuses fresh deploy state instead of forcing schema invalidation, and normal typing no longer shows the global loading state.
- [x] **M25-DOGFOOD-REQ-03**: Page size is an obvious 3-10 setting wired to `menu/page_size`; native and browser tests prove the candidate panel cap and page navigation at 3, 9, and 10 visible rows.
- [x] **M25-DOGFOOD-REQ-04**: The current Jyutping web profile uses bare `` ` `` for `luna_pinyin` reverse lookup, removes the vestigial bare `reverse_lookup` slot, keeps retained Loengfan/Cangjie secondary lookups on explicit non-bare triggers such as `` `vl`` / `` `vc``, and shows the trigger map in the web UI.
- [x] **M25-DOGFOOD-REQ-05**: The schema selector, Luna visible name, Cangjie version control, Display/Live settings order, and IME Settings alignment are browser-evidenced across desktop and narrow viewports.
- [x] **M25-DOGFOOD-REQ-06**: Binary dogfood controls use checkbox-style affordances, Candidate Menu Layout uses radio choices, and the UI stack remains Vite + React + Tailwind CSS plus small local components only.

## M26 Performance Hardening Requirements

**Status: complete.** M26 turned the post-M25 performance review into a measurement-first hardening milestone. It separates native engine cost from browser/WASM/worker/React latency before any optimization claims are accepted.

- [x] **M26-PERF-REQ-01**: Native large-real-asset benchmarks cover `jyut6ping3_mobile`, `luna_pinyin`, representative `cangjie5`, and the TypeDuck dynamic-correction path, reporting median, p95, p99, max, cold-first-key versus warm steady-state, operation count, full-ABI versus engine-only cost, and allocation/RSS notes. Evidence: `apps/yune-web/e2e/results/m26-performance/native-before.md` and `apps/yune-web/e2e/results/m26-performance/native-after.md`.
- [x] **M26-PERF-REQ-02**: Browser diagnostics record keydown-to-paint or the closest browser-supported proxy for normal typing, long phrases, page changes, reverse lookup, and cold/warm startup, without treating browser-only numbers as native engine latency. Evidence: `apps/yune-web/e2e/results/m26-performance/typing-keydown-to-paint-before.json` and `apps/yune-web/e2e/results/m26-performance/typing-keydown-to-paint-after.json`.
- [x] **M26-PERF-REQ-03**: Startup diagnostics attribute the current coarse TypeDuck-Web `runtime:initialized` interval into worker/package load, WASM module creation, filesystem mount/sync, schema asset deploy/reuse, `TypeDuckRuntime.init`, schema selection, and startup complete buckets. Evidence: `apps/yune-web/e2e/results/m26-performance/startup-attribution-before.json` and `apps/yune-web/e2e/results/m26-performance/startup-attribution-after.json`.
- [x] **M26-PERF-REQ-04**: One measured optimization landed with before/after native and browser evidence. The largest measured owner was startup/schema-selection/runtime initialization, now closed by the named M27 follow-up [`docs/plans/completed/m27-plan-typeduck-web-startup-runtime-init.md`](./plans/completed/m27-plan-typeduck-web-startup-runtime-init.md). The landed lower-risk M26 slice targets the measured TypeDuck dynamic-correction stress owner: `per_key_real_jyut6ping3_mobile_jigaajiusihaa_correction_engine_only` improved from median `451490.692us` / p95 `467909.308us` to median `121712.662us` / p95 `124420.115us` by pruning impossible-length candidates before the restricted-distance matrix. Evidence: `apps/yune-web/e2e/results/m26-performance/optimization-choice.md`.
- [x] **M26-PERF-REQ-05**: Compatibility gates remain green: upstream `luna_pinyin`, Cantonese parity, native `typeduck_web`, TypeScript runtime tests/build, TypeDuck-Web build, focused M26 browser evidence, and TypeDuck-Web patch reverse/forward checks. Evidence: `apps/yune-web/e2e/results/m26-performance/task-5-gates.md` and `apps/yune-web/e2e/results/m26-performance/patch-checks.md`.

## M27 TypeDuck-Web Startup Runtime Init Requirements

**Status: complete.** M27 closed the startup/schema-selection/runtime-init owner measured by M26 and the TypeDuck-Web engine-control update surface with native/browser path reconciliation, native owner spans, Windows process memory, browser-to-native mapping, a measured spelling-algebra startup optimization, and live-vs-deploy control classification evidence.

- [x] **M27-STARTUP-REQ-01**: Native startup benchmarks reconcile the browser-paid path against native rows and split the browser-paid `jyut6ping3_mobile` path into observable owners. Evidence: `apps/yune-web/e2e/results/m27-startup-runtime/startup-path-reconciliation.md`, `apps/yune-web/e2e/results/m27-startup-runtime/native-startup-before.md`, and `apps/yune-web/e2e/results/m27-startup-runtime/native-startup-after.md`.
- [x] **M27-STARTUP-REQ-02**: Native startup evidence includes real Windows process-memory metrics, including working-set deltas and peak working-set bytes for startup spans. Evidence: `apps/yune-web/e2e/results/m27-startup-runtime/native-startup-before.md` and `apps/yune-web/e2e/results/m27-startup-runtime/native-startup-after.md`.
- [x] **M27-STARTUP-REQ-03**: Browser startup evidence records fresh and reload paths, preserves the M26 startup markers, adds `m27EvidenceVersion`, and maps browser `schema:select` / `runtime:init` timing back to native startup owners without treating browser timing as native engine timing. Evidence: `apps/yune-web/e2e/results/m27-startup-runtime/browser-startup-after.md` and `apps/yune-web/e2e/results/m27-startup-runtime/browser-startup-after-after.json`.
- [x] **M27-STARTUP-REQ-04**: The evidenced top startup owner was materially reduced by caching spelling-algebra expansions per original code. Native `startup_real_jyut6ping3_mobile_runtime_ready` improved from about `15.55s` median to about `6.35s`, and browser fresh/reload startup improved to `5.680s` / `5.466s`. Evidence: `apps/yune-web/e2e/results/m27-startup-runtime/optimization-choice.md`, `apps/yune-web/e2e/results/m27-startup-runtime/native-startup-after.md`, and `apps/yune-web/e2e/results/m27-startup-runtime/browser-startup-after.md`.
- [x] **M27-STARTUP-REQ-05**: Compatibility gates remain green: upstream `luna_pinyin`, Cantonese parity, native `typeduck_web`, workspace tests, frontend benchmarks, TypeScript runtime tests/build, TypeDuck-Web build, focused M27 startup/control browser evidence, TypeDuck-Web patch checks, and `git diff --check`. Evidence: `apps/yune-web/e2e/results/m27-startup-runtime/task-5-gates.md` and `apps/yune-web/e2e/results/m27-startup-runtime/patch-checks.md`.
- [x] **M27-STARTUP-REQ-06**: TypeDuck-Web engine controls are classified as live, browser-only, deploy-time, or local-runtime-only. AI candidates no longer use the page-wide loading wrapper and do not emit runtime init, schema select, deploy markers, or visible loading; deploy-backed controls remain measured separately. Evidence: `apps/yune-web/e2e/results/m27-startup-runtime/control-classification-before.md` and `apps/yune-web/e2e/results/m27-startup-runtime/control-classification-after-after.json`.

## M28 TypeDuck Partial Candidate Selection Requirements

**Status: complete.** M28 closed segment-aware partial candidate selection as a separate engine-correctness milestone after M27. The v1.1.2 oracle remains authoritative where it diverges from the user-feel target.

- [x] **M28-PARTIAL-REQ-01**: Git history and code evidence classify the `caksijathaacoenggeoizi` -> select `測` behavior as previously missing support, not a recent regression. Evidence: `apps/yune-web/e2e/results/m28-partial-selection/history-classification.md`.
- [x] **M28-PARTIAL-REQ-02**: TypeDuck-HK/librime `v1.1.2` oracle fixture captures partial-selection behavior for `caksijathaacoenggeoizi`, including first committed text, remaining input/preedit, next candidates, and final oracle flow. Evidence: `crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-m28-partial-selection.json` and `apps/yune-web/e2e/results/m28-partial-selection/oracle-capture.md`.
- [x] **M28-PARTIAL-REQ-03**: Native `yune-core` and `yune-rime-api` tests cover segment-aware partial commit/recomposition and preserve FORK-PARITY-03 userdb pronunciation recovery: whole-sentence commits keep full primary codes, while true partial commits record only the consumed span. Evidence: `crates/yune-core/tests/cantonese_parity.rs`, `crates/yune-rime-api/tests/typeduck_web.rs`, and `apps/yune-web/e2e/results/m28-partial-selection/task-5-gates.md`.
- [x] **M28-PARTIAL-REQ-04**: TypeDuck-Web browser evidence proves selecting `測` does not commit raw `sijathaacoenggeoizi`, continues through the captured component flow, and records that the user-feel `測試一下長句子` target is not the TypeDuck v1.1.2 oracle flow. Evidence: `apps/yune-web/e2e/results/m28-partial-selection/browser-partial-selection.json` and `apps/yune-web/e2e/results/m28-partial-selection/browser-evidence.md`.
- [x] **M28-PARTIAL-REQ-05**: Full compatibility gates remain green: `cargo fmt --check`, workspace clippy, upstream `luna_pinyin`, `cantonese_parity`, `typeduck_web`, workspace tests, frontend benchmarks, TypeScript runtime tests/build, TypeDuck-Web build/evidence, patch checks, and `git diff --check`. Evidence: `apps/yune-web/e2e/results/m28-partial-selection/task-5-gates.md`.

## M28 Follow-up Upstream Jyutping Composition Requirements

**Status: complete.** This follow-up closes the post-M28 dogfood gaps for Space/default-confirm partial recomposition and upstream-style Jyutping long composition. Evidence: `apps/yune-web/e2e/results/m28-follow-up-upstream-jyutping/`; plan: [`docs/plans/completed/m28-follow-up-plan-upstream-jyutping-composition.md`](./plans/completed/m28-follow-up-plan-upstream-jyutping-composition.md).

- [x] **M28F-UPSTREAM-REQ-01**: Space/default-confirm for `caksijathaacoenggeoizi` commits only the consumed prefix candidate and keeps the remaining input composing; it never commits `測sijathaacoenggeoizi`.
- [x] **M28F-UPSTREAM-REQ-02**: A checked-in hybrid upstream-librime-engine Jyutping fixture captures `caksijathaacoenggeoizi` composition/ranking with provenance: upstream engine repository/tag/commit, pinned Jyutping schema/dictionary source repository/commit, upstream deploy command, capture command, options, and candidate rows. The fixture contains no local absolute paths and lives outside the pure `upstream-1.17.0` fixture family.
- [x] **M28F-UPSTREAM-REQ-03**: `docs/decisions.md` records a narrow decision that this Jyutping long-composition/ranking slice follows the captured-and-accepted hybrid fixture over TypeDuck v1.1.2 when they disagree, while TypeDuck v1.1.2 remains the compatibility oracle for profile ABI/comment surfaces and the hybrid fixture explicitly excludes dictionary-comment payloads.
- [x] **M28F-UPSTREAM-REQ-04**: Native tests follow the accepted captured ordering for this case: sentence/lattice candidate first when enabled, fixture-captured fallback rows after it, and no invented phrase-prefix row when upstream did not capture one.
- [x] **M28F-UPSTREAM-REQ-05**: TypeDuck-Web browser evidence covers auto-composition off plus Space/default-confirm, and auto-composition on plus first-page ranking, without raw-tail commits.
- [x] **M28F-UPSTREAM-REQ-06**: Full compatibility gates remain green: Rust fmt/clippy/tests, upstream `luna_pinyin`, `cantonese_parity`, `typeduck_web`, TypeScript runtime tests/build, TypeDuck-Web build/evidence, patch checks if source changes, and `git diff --check`.

## M29 Startup Memory And Typing Performance Requirements

**Status: complete.** M29 refreshed post-M28-follow-up startup, memory, and typing evidence; classified the M27-style `1.79GB` peak as repeated-benchmark high-water with real single-startup ready pressure around `1.10GB`; reduced the measured `spelling_algebra_expand` startup owner by avoiding no-op regex replacement allocation; and kept typing as attribution evidence because the fresh owner profile was mixed and already much smaller than startup. Evidence: `apps/yune-web/e2e/results/m29-performance/`; plan: [`docs/plans/completed/m29-plan-startup-memory-typing-performance.md`](./plans/completed/m29-plan-startup-memory-typing-performance.md).

- [x] **M29-PERF-REQ-01**: Fresh M29 baselines re-run native startup benchmarks, browser startup evidence, and browser keydown-to-paint typing evidence on the current post-M28-follow-up code.
- [x] **M29-PERF-REQ-02**: Memory evidence classifies the M27 `1.79GB` peak as per-startup pressure, benchmark cumulative high-water, or unresolved with a precise blocker and next measurement.
- [x] **M29-PERF-REQ-03**: Startup attribution identifies the top remaining owner, expected to be `spelling_algebra_expand` unless fresh evidence proves otherwise, before any startup optimization is implemented.
- [x] **M29-PERF-REQ-04**: Typing attribution identifies the top owner for normal and long-phrase keydown-to-paint latency across browser, worker, serialization, native/WASM processing, and render spans.
- [x] **M29-PERF-REQ-05**: At least one startup or typing optimization lands with before/after native and browser evidence, or the chosen owner is closed with an evidence-backed reason it cannot be reduced safely in this milestone.
- [x] **M29-PERF-REQ-06**: Full gates remain green: Rust fmt/clippy/tests, frontend benchmark, TypeScript runtime tests/build, TypeDuck-Web build, focused M29 Playwright evidence, patch checks if source changes, and `git diff --check`.

## M30 Engine Representation Performance Requirements

**Status: complete.** M30 closed as an engine-only follow-up after M29. It accepted Lever A: the duplicate steady-state expanded-entry vector is removed for spelling-algebra-backed translators, the final `entries_by_code` map is built by moving `Candidate` values, and TypeDuck row order is preserved through a builder-only source stream. Single-startup ready pressure improved from `1,103,331,328` bytes to `838,209,536` bytes in the Lever A run and `839,217,152` bytes in the final gate. Native runtime-ready startup median was noisy but improved versus baseline in both after-runs (`6,242,614.900us` -> `5,952,128.400us` in the Lever A run; `6,120,732.800us` in the final gate). Browser startup and typing stayed flat/noisy after fresh WASM rebuild, so M30 records no browser latency win. Evidence: `apps/yune-web/e2e/results/m30-engine-performance/`; plan: [`docs/plans/completed/m30-plan-engine-representation-performance.md`](./plans/completed/m30-plan-engine-representation-performance.md).

- [x] **M30-PERF-REQ-01**: Fresh M30 native and browser baselines were captured before implementation, including single-startup memory, startup owner spans, watched `hai`/`jigaajiusihaa` key rows, and browser attribution. Evidence: `apps/yune-web/e2e/results/m30-engine-performance/native-before.md` and `apps/yune-web/e2e/results/m30-engine-performance/m30-baseline.md`.
- [x] **M30-PERF-REQ-02**: M29 evidence markdown tables were reconciled against the committed startup/typing JSON before M30 optimization claims. Evidence: `apps/yune-web/e2e/results/m30-engine-performance/m29-evidence-check.md`.
- [x] **M30-PERF-REQ-03**: Expanded-table Lever A landed with before/after evidence proving reduced startup memory and no accepted behavior changes. Evidence: `apps/yune-web/e2e/results/m30-engine-performance/lever-a.md`.
- [x] **M30-PERF-REQ-04**: Internal string-sharing / compact abbreviation representation was deferred after Lever A because the accepted slice already delivered a large memory win and the watched per-key rows did not justify a broader candidate-payload rewrite in M30.
- [x] **M30-PERF-REQ-05**: Long-input sentence-lattice backpointers were deferred because the Lever A after-run kept `jigaajiusihaa` rows flat/noisy rather than identifying sentence DP as the next M30 hot owner.
- [x] **M30-PERF-REQ-06**: Correction-stress indexing was deferred because M26 had already reduced the correction stress path and M30's correction-on row stayed flat after Lever A.
- [x] **M30-PERF-REQ-07**: Full gates remained green: Rust fmt/clippy/tests, frontend benchmark, TypeScript runtime tests/build, TypeDuck-Web build, focused browser performance evidence, no tracked TypeDuck-Web source patch change, and `git diff --check`. Evidence: `apps/yune-web/e2e/results/m30-engine-performance/task-6-gates.md`.

**Follow-on (no requirement IDs):** [`M21`](./plans/completed/m21-plan-typeduck-web-product-comparison.md) is complete as a post-M20 _comparison protocol_ and hard-oracle closeout. It compared the Yune harness against the deployed `typeduck.hk/web` product as a behavior/feel target, but the `v1.1.2` fixtures remained the hard oracle. The final gap ledger has no remaining hard-oracle action rows: M21-GAP-01 is closed by `jyut6ping3-m21-sentence-composition.json`, M21-GAP-02 is closed by `jyut6ping3-m21-prediction-ranking.json` plus real `nri` browser before/after evidence, and `jyut6ping3-m21-closeout.json` locks the remaining baseline/fuzzy/sentence/`hk2s`/tone-letter/paging rows including the final `m` and `mgoi` fixes.

## M37 Engine Hyper-Optimization Requirements

**Status: complete.** M37 closed the engine hyper-optimization gates from [`plans/completed/m37-plan-engine-hyper-optimization.md`](./plans/completed/m37-plan-engine-hyper-optimization.md). Evidence: `docs/reports/evidence/m37-engine-hyper-optimization/`, especially `phase-0-baseline/`, `phase-1-page-bounded-sentence/`, `phase-3-final-native/`, `rsmarisa-path.md`, and `storage-path.md`.

- [x] **M37-ENGINE-01**: Phase 0 evidence splits Track B `hai` across key-path counters and records a product memory-owner table. `hai` was attributed to full product candidate materialization/filtering, and product memory was attributed to the M36 owned no-marisa table row mirror plus retained compiled payload state.
- [x] **M37-ENGINE-02**: The final product storage path is byte-backed and native-mapped. `rsmarisa 0.4.2` was tried against actual `jyut6ping3` and `jyut6ping3_scolar` marisa string-table data and mmaped both; the selected route is a mapped Yune-readable table because the full `rsmarisa` hot path still needs a multi-level phrase-index adapter.
- [x] **M37-ENGINE-03**: Final product status proves fresh table/prism/reverse artifacts, no `SourceFallback`, `selected_storage=byte_backed`, `table_format=yune_no_marisa_compact`, and `mapping_mode=mmap`.
- [x] **M37-ENGINE-04**: Default Track B product rows prove page-bounded ordinary `RimeProcessKey` + `RimeGetContext` materialization. Final `hai` builds 52 owned candidates, sorts/stores 48, page-clones 5, and exports 5, instead of the phase-0 19,918 owned candidates and 11,289 sorted/stored rows.
- [x] **M37-ENGINE-05**: `RimeGetContext` uses page snapshots for page-only reads and no longer requires a full `Engine::snapshot()` candidate-list clone.
- [x] **M37-ENGINE-06**: Behavior gates remained byte-identical across the focused upstream, TypeDuck, paging/selection, correction, prediction, learning, and rich-comment gates run for M37. Runtime/browser gates were not used for performance claims because no browser speed claim was made.
- [x] **M37-ENGINE-07**: Track B `hai` moved from the M36 final `15,241.000us` median to `8,336.800us` (`-45.3%`) and is no longer unexplained; the residual owner is lookup-view scanning.
- [x] **M37-ENGINE-08**: Track B product median working set moved from the M36 final about `777 MB` row plateau to about `365-369 MB`, and peak moved from `928,350,208 B` to `504,377,344 B`. Track A working-set attribution was refreshed in the final native run.
- [x] **M37-ENGINE-09**: The final native product path reports mmap/file-backed loading for the selected hot storage bytes. `rsmarisa` probe evidence also reports `rsmarisa_mapping_mode=mmap` for both real marisa string-table payloads.
- [x] **M37-ENGINE-10**: Public claims remain separated: Track A remains comparison evidence, Track B remains product before/after evidence, and no browser startup/typing claim is made without rebuilt release WASM and real browser evidence.
- [x] **M37-ENGINE-11**: Final quality gates are recorded in the completed M37 plan. Runtime/browser gates were N/A for performance claims because M37 did not change runtime-visible browser files or make browser speed claims.

## M38 Engine Performance Parity Requirements

**Status: complete.** M38 closed as the pure isolated-engine parity milestone in [`plans/completed/m38-plan-engine-performance-parity.md`](./plans/completed/m38-plan-engine-performance-parity.md). Closeout evidence lives under [`reports/evidence/m38-engine-performance-parity/`](./reports/evidence/m38-engine-performance-parity/), with the final same-run native benchmark in `phase-5-final-native/` and final gate summary in `final-gates.md`.

- [x] **M38-ENGINE-01**: Final M38 claims are native isolated-engine claims only. Product rows remain regression/status context and no frontend, browser, packaging, deployment, or public-delivery speed claim is made.
- [x] **M38-ENGINE-02**: Phase 0 reran fresh same-machine Yune and upstream librime `1.17.0` startup/runtime-ready, session create/select/destroy, `hao`, `ni`, `zhongguo`, working set, and peak working set before implementation.
- [x] **M38-ENGINE-03**: Phase 0 and final evidence attribute startup/session and per-key owners, including selected backend, mapping mode, heap mirror bytes, `rsmarisa` calls, raw prism/table lookup, translator production, context export, memory, and ABI allocation bytes.
- [x] **M38-ENGINE-04**: Final Track A status proves `selected_storage=rsmarisa_byte_backed` over the deployed `luna_pinyin` marisa table, with positive `rsmarisa` exact/prefix counters and zero ordinary no-marisa compact fallback.
- [x] **M38-ENGINE-05**: Final selected Track A table and prism bytes are mmap-backed; table/prism heap mirror bytes are `0`.
- [x] **M38-ENGINE-06**: Final startup/runtime-ready and session medians are within `1.25x` of same-run upstream librime: startup `0.959x`, session `0.867x`.
- [x] **M38-ENGINE-07**: Final `hao`, `ni`, and `zhongguo` rows are each within `5x` of same-run upstream librime: `3.415x`, `3.969x`, and `0.354x`.
- [x] **M38-ENGINE-08**: Final evidence includes Yune-only raw prism, raw table, raw `rsmarisa`, translator, and context-export microbench rows.
- [x] **M38-ENGINE-09**: Ordinary Track A first-page reads are page-bounded; final counters show bounded reads, owned candidates/page clones scaled to page plus bounded surplus, and no full-list fallback on the target rows.
- [x] **M38-ENGINE-10**: Final evidence reports working set, peak working set, allocation counters, and the remaining whole-process memory gap while preserving zero selected table/prism heap mirror bytes.
- [x] **M38-ENGINE-11**: Upstream `luna_pinyin` behavior and touched compatibility paths are covered by focused tests plus `cargo test --workspace`.
- [x] **M38-ENGINE-12**: Final reports make only native isolated-engine claims and explicitly exclude frontend/browser/product/delivery speed claims.
- [x] **M38-ENGINE-13**: Closeout records final quality gates: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, focused engine and touched compatibility tests, `cargo test --workspace`, final native benchmark, report checks, and final `git diff --check`.

## Post-M38 Engine Performance Follow-Up Requirements

**Status: draft.** These requirements do not reopen M38. The post-M38 baseline
in [`reports/evidence/post-m38-long-input-baseline/baseline-native/`](./reports/evidence/post-m38-long-input-baseline/baseline-native/)
and the 59-character stress run in
[`reports/evidence/post-m38-long-input-baseline/stress-59-native/`](./reports/evidence/post-m38-long-input-baseline/stress-59-native/)
show that uninterrupted long input is not in parity. These requirements define
evidence that the next engine-performance milestone must keep before claiming
broader typing parity.

- [ ] **POST-M38-PERF-01**: The next same-run native Yune-versus-librime
  benchmark keeps the M38 Track A rows (`hao`, `ni`, `zhongguo`) and includes
  the required long continuous pinyin rows
  `ceshiyixiachangjushuruxingnengzenyang` and
  `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong`.
- [ ] **POST-M38-PERF-02**: The long-input rows record the same evidence shape
  as the M38 target rows: Yune/librime medians and ratios, selected backend,
  mapping mode, heap mirror bytes, rsmarisa/no-marisa fallback counters, raw
  prism/table lookup, translator production, context export, memory,
  allocation, and ABI export counters.
- [ ] **POST-M38-PERF-03**: Any long-input optimization claim identifies whether
  the owner is lookup enumeration, sentence/full-list behavior, paging/context
  growth, ABI export, userdb/filter/ranker work, or another measured bucket.
- [ ] **POST-M38-PERF-04**: Long-composition translator attribution splits the
  current top-level translator bucket into inner owners, including
  sentence/full-list fallback, `StaticTableTranslator::sentence_candidate`,
  substring exact/prefix lookup loops, path cloning, sorting, context/export,
  and any userdb/filter/ranker work that fires for the long row.
- [ ] **POST-M38-PERF-05**: Memory follow-up uses the post-M38 working-set/peak
  baseline as the comparison anchor and adds heap-owner attribution before any
  memory reduction claim is accepted.
- [ ] **POST-M38-PERF-06**: Long uninterrupted input is treated as a primary
  engine requirement, not an optional stress test. The next milestone records a
  length-curve benchmark around short, medium, 37-character, 50+ character, and
  59-character inputs, and it does not claim broader typing parity unless the
  50+ character row is brought into the agreed same-run librime ratio gate or
  closed by an explicit measured no-go.
- [ ] **POST-M38-PERF-07**: The next engine-performance milestone blocks
  cross-dimension regressions: startup/session, short-input latency, long-input
  latency, mmap/`rsmarisa` activation, bounded output, working set, peak memory,
  and upstream-observable behavior must all be reported and must not regress
  outside the milestone's explicit gates.

## Out of Scope

Explicitly excluded from the current milestone.

| Feature | Reason |
| --- | --- |
| Full librime C++ plugin ABI compatibility | Expensive and not yet required by a concrete frontend or distribution migration path |
| Cloud inference as a required dependency | Classic input behavior must remain local-first and predictable |
| New GUI frontend | Native frontend integration should validate the ABI first; `yune-cli` is only a frontend surrogate |
| Behavior changes during mechanical refactors | Compatibility work needs measurable, reviewable behavior slices |
| 100% feature parity with librime internals | The oracle is a behavioral floor, not a feature target; a librime feature is implemented only when a named target schema/frontend needs it (see roadmap "Scope Ledger" and `decisions.md` D-25) |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
| --- | --- | --- |
| CLI-01 | Phase 1 | Complete |
| CLI-02 | Phase 1 | Complete |
| CLI-03 | Phase 1 | Complete |
| CLI-04 | Phase 1 | Complete |
| CLI-05 | Phase 1 | Complete |
| ABI-01 | Phase 2 | Complete |
| ABI-02 | Phase 2 | Complete |
| ABI-03 | Phase 2 | Complete |
| ABI-04 | Phase 2 | Complete |
| SCHEMA-01 | Phase 3 | Complete |
| SCHEMA-02 | Phase 3 | Complete |
| SCHEMA-03 | Phase 3 | Complete |
| SCHEMA-04 | Phase 3 | Complete |
| SCHEMA-05 | Phase 3 | Complete |
| DATA-01 | Phase 4 | Complete |
| DATA-02 | Phase 4 | Complete |
| DATA-03 | Phase 4 | Complete |
| DATA-04 | Phase 4 | Complete |
| USERDB-01 | Phase 5 | Complete |
| USERDB-02 | Phase 5 | Complete |
| USERDB-03 | Phase 5 | Complete |
| QUAL-01 | Phase 1 | Complete |
| QUAL-02 | Phase 1 | Complete |
| QUAL-03 | Phase 5 | Complete |
| QUAL-04 | Phase 5 | Complete |
| FRONTEND-VALIDATION-01 | Phase 6 | Complete |
| FRONTEND-VALIDATION-02 | Phase 6 | Complete |
| FRONTEND-VALIDATION-03 | Phase 6 | Complete |
| FRONTEND-VALIDATION-04 | Phase 6 | Complete |
| FRONTEND-VALIDATION-05 | Phase 6 | Complete |
| BENCH-01 | Phase 6 | Complete |
| BENCH-02 | Phase 6 | Complete |
| TYPEDUCK-WASM-01 | Phase 7 | Complete |
| TYPEDUCK-WASM-02 | Phase 7 | Complete |
| TYPEDUCK-WASM-03 | Phase 7 | Complete |
| TYPEDUCK-JS-01 | Phase 8 | Complete |
| TYPEDUCK-JS-02 | Phase 8 | Complete |
| TYPEDUCK-JS-03 | Phase 8 | Complete |
| TYPEDUCK-JS-04 | Phase 8 | Complete |
| TYPEDUCK-FS-01 | Phase 9 | Complete |
| TYPEDUCK-FS-02 | Phase 9 | Complete |
| TYPEDUCK-FS-03 | Phase 9 | Complete |
| TYPEDUCK-FS-04 | Phase 9 | Complete |
| TYPEDUCK-E2E-01 | Phase 10 | Complete |
| TYPEDUCK-E2E-02 | Phase 10 | Complete |
| TYPEDUCK-E2E-03 | Phase 10 / 17 | Complete - HR-5 real-assets browser matrix passes; rich comment byte parity is committed in `cantonese_parity` |
| TYPEDUCK-E2E-04 | Phase 10 / 17 | Complete - HR-7 records GO WITH CONDITIONS |
| WIN-TEST-01 | Phase 11 | Complete |
| WIN-ABI-01 | Phase 12 / M19 | Complete - helper coverage retained and exposed through named `rime_get_typeduck_profile_api()`; not exposed by default upstream `rime_get_api()` |
| WIN-ORACLE-01 | Phase 13 | Complete |
| WIN-COMMENT-01 | Phase 14 / 17 | Complete - dictionary payload, schema prompt, and joiner oracle covered |
| WIN-BUILD-01 | Phase 15 / M10 | Complete - current TypeDuck-profile package/header smoke and packaged DLL profile lifecycle pass |
| WIN-PARITY-01 | Phase 16 / M10 | Complete - captured v1.1.2 engine behavior is active; frontend-only schema-menu/userdb observations remain T3 evidence scope |
| WIN-FRONTEND-01 | M10 | Complete - T1 build/link and stock T3 TypeDuckServer/TestTypeDuckIPC real-server IPC smoke pass against the Yune package; interactive TSF typing and visible candidate-panel rendering are Phase 2 frontend gates |
| P2-WIN02-BOUNDARY-01 | P2-WIN-02 | Complete - Phase 0C `ngohaig` fixture and provenance are checked in |
| P2-WIN02-BOUNDARY-02 | P2-WIN-02 | Complete - core and ABI tests assert TypeDuck `\f\r1,` rich comments |
| P2-WIN02-BOUNDARY-03 | P2-WIN-02 | Complete - compiled lookup records and deployment side-dictionary rebuild are covered |
| P2-WIN02-BOUNDARY-04 | P2-WIN-02 | Complete - uninitialized config boundary and repeated lifecycle tests pass |
| P2-WIN02-BOUNDARY-05 | P2-WIN-02 | Complete - package, direct DLL probe, TypeDuck-Web gate, and stock IPC smoke pass |
| P2-WIN02-BOUNDARY-06 | P2-WIN-02 | Complete - approved Notepad TSF reruns classify the remaining raw-ASCII behavior as non-Yune TSF input-delivery/frontend-shell work |
| UPSTREAM-ORACLE-01 | M12 | Complete - upstream `1.17.0` provenance pinned as default core oracle |
| UPSTREAM-ORACLE-02 | M12 | Complete - fixture naming separates `upstream-1.17.0` and `typeduck-v1.1.2` goldens |
| UPSTREAM-AUDIT-01 | M12 | Complete - coverage audit captured in `docs/plans/completed/m12-audit-coverage.md` |
| TYPEDUCK-PROFILE-01 | M12/M10 | Complete - TypeDuck-specific coverage remains profile-only; M10 verifies the native Windows frontend path without widening the default ABI |
| UPSTREAM-BEHAVIOR-01 | M12 | Complete - six official-binary `luna_pinyin` fixture files are checked in under `upstream-1.17.0` |
| UPSTREAM-BEHAVIOR-02 | M12 | Complete - full `ni` selection fixture includes all exact dictionary rows and candidate essay rows |
| UPSTREAM-BEHAVIOR-03 | M12 | Complete - active parity tests drive real parser/dictionary/translator/filter/Engine paths |
| UPSTREAM-BEHAVIOR-04 | M12/M17/M18 | Complete - former sentence/lattice and processor blockers are fixture-backed; learned grammar/contextual paths remain deferred |
| UPSTREAM-BEHAVIOR-05 | M12 | Complete - provenance test scans all upstream `luna_pinyin` fixtures and source policies |
| UPSTREAM-BEHAVIOR-06 | M17 | Complete - upstream `luna_pinyin` sentence/lattice fixtures and null-grammar poet path are active |
| AI-01 | M11 S1 | Complete - staged provider interface in `yune-core` |
| AI-02 | M11 S2/S5 | Complete - worker/fallback/confidence merge plus local rule-backed provider |
| AI-03 | M11 S1/S5 | Complete - source-labeled contextual/local completions with no default AI auto-commit |
| AI-04 | M11 S3 | Complete - context snapshot provider covers app, field, preceding text, cursor, schema, and candidate count |
| AI-05 | M11 S4 | Complete - AI memory store records explicit AI selections, is inspectable/clearable/disable-able, and uses `.ai-memory` namespace helpers |
| AI-06 | M11 S3/S4 | Complete - default-sensitive privacy blocks remote calls and suppresses AI memory writes while classic input remains available |
| AI-07 | M11 S1/S5 | Complete - direct CLI demonstrates `--ai-provider mock` and `--ai-provider local` |
| M13-AI-01 | M13 | Complete - `process_key` stays provider-free; `stage_ai` owns the local provider pass |
| M13-AI-02 | M13 | Complete - default-off browser toggle and disable-clears-staged-row behavior covered |
| M13-AI-03 | M13 | Complete - source labels flow from engine snapshot data without ABI/table changes |
| M13-AI-04 | M13 | Complete - browser/default commit remains classic; AI selection is explicit |
| M13-AI-05 | M13 | Complete - AI commits skip userdb and sensitive default suppresses AI memory learning |
| M13-AI-06 | M13 | Complete - real TypeDuck-Web M13 Playwright evidence covers the safety scenarios |
| TYPEDUCK-PARITY-01 | M14 | Complete - v1.1.2 wrapper + Cantonese option/completion/correction goldens captured |
| TYPEDUCK-PARITY-02 | M14 | Complete - emitted schema-list surface captured; UI hiding assertion deferred to M16 |
| TYPEDUCK-PARITY-03 | M14 | Complete - levers export spike captured a learned `nei5` userdb row |
| TYPEDUCK-PARITY-04 | M15 | Complete - combine_candidates + show_full_code pass M14-backed real-engine assertions |
| TYPEDUCK-PARITY-05 | M15 | Complete - enable_sentence/completion/correction parity assertions are active |
| TYPEDUCK-PARITY-06 | M15 | Complete - checked-in OpenCC source dictionaries drive `hk2s` simplification |
| TYPEDUCK-PARITY-07 | M16 | Complete with conditions - real TypeDuck-Web Playwright matrix covers app-exposed Cantonese paths plus M13 AI; deploy-only/UI/userdb gaps are explicit |
| FORK-PARITY-01 | backlog | Complete - 容錯 ruleset runs on the real ~127k jyut6ping3 dictionary with golden coverage |
| FORK-PARITY-02 | backlog | Complete - weighted PreferUserPhrase gate implemented |
| FORK-PARITY-03 | backlog | Complete - per-entry userdb pronunciation recovery, including multi-syllable sentence codes |
| FORK-PARITY-04 | backlog | Complete - `hide_lone_schema` implemented |
| FORK-PARITY-05 | backlog | Complete - correction edit-distance/min-distance/enable_correction gate/normal-only behavior implemented |
| FORK-PARITY-06 | backlog | Complete - TypeDuck letter-tone preedit path implemented |
| FORK-PARITY-07 | backlog | Complete - TypeDuck-profile `全形`/`半形` labels locked |
| FORK-PARITY-08 | backlog | Complete - upstream ranking accepted except the oracle-backed TypeDuck `jyut6ping3` prediction-count limit; long-entry prediction preserved with threshold and never-first controls implemented |
| FORK-PARITY-09 | backlog | Complete - UI-side `display_languages` decision recorded |
| M20-DEMO-01 | M20 | Complete - controls use existing customize/deploy and setOption paths while preserving ABI/export boundaries |
| M20-DEMO-02 | M20 | Complete - prediction never-first defaults on; real-assets-calibrated threshold control filters `santai` predictions with documented range bounds |
| M20-DEMO-03 | M20 | Complete - static/default-on features use guided scenarios, not fake toggles |
| M20-DEMO-04 | M20 | Complete - local AGENTS guidance added for internal TypeDuck-Web harness, runtime package, and product-surface distinction |
| M20-DEMO-05 | M20 | Complete - browser honesty gate separates visible before/after controls from explicit browser-surface N/A for Input Memory off-state and current-schema Cangjie/show_full_code limits; Auto-correction `nri` now has real browser before/after evidence; `ascii_punct` still needs browser-visible evidence before becoming a working web toggle |
| M20-DEMO-06 | M20 | Complete - internal TypeDuck-Web harness is documented as the canonical browser playground for supported engine features |
| M20-DEMO-07 | M20 | Complete - documented demo-default `combine_candidates` active control plus current-schema `show_full_code`/Cangjie N/A evidence |
| M19-BREADTH-01 | M19 | Complete - generalized upstream schema capture recipe and provenance guard added |
| M19-BREADTH-02 | M19 | Complete - `double_pinyin` upstream 1.17.0 fixture and owning parity test added |
| M19-BREADTH-03 | M19 | Complete - `cangjie5` upstream 1.17.0 fixture and owning parity test added |
| M19-BREADTH-04 | M19 | Complete - `bopomofo` upstream 1.17.0 fixture and owning parity test added |
| M19-ABI-01 | M19 | Complete - named TypeDuck-profile ABI accessor exposes list-append slots while default `rime_get_api()` remains upstream-shaped |
| M22-PLAY-01 | M22 Bucket 2 | Complete - opt-in read-only inspector exposes engine debug data in the TypeDuck-Web playground |
| M22-PLAY-02 | M22 Bucket 2 | Complete - inspector is default-off, response-identity tested, browser-evidenced, and ABI-layout neutral |
| M22-PLAY-03 | M22 Bucket 1 | Complete - traditionalization, disabled, extended_charset, and dictionary_exclude have browser evidence; ascii_punct remains absent without browser-visible before/after evidence |
| M22-PLAY-04 | M22 Bucket 3 | Complete - jyut6ping3_mobile, cangjie5, and luna_pinyin load through a real schema switcher; cangjie5 and luna_pinyin reverse lookup are active with measured browser asset sizes |
| M24-DOGFOOD-REQ-01 | M24 | Complete - issue-scoped M24 browser evidence and startup asset markers are recorded |
| M24-DOGFOOD-REQ-02 | M24 | Complete - comment controls are hidden and compound candidate details live in the dictionary panel |
| M24-DOGFOOD-REQ-03 | M24 | Complete - ordering, page-size, and Jyutping reverse lookup are fixture/native/browser guarded |
| M24-DOGFOOD-REQ-04 | M24 | Complete - Cantonese-first grouped settings, schema names, status labels, and typeface picker are browser-evidenced |
| M24-DOGFOOD-REQ-05 | M24 | Complete - DaisyUI removed; local Tailwind components build and pass browser evidence |
| M25-DOGFOOD-REQ-01 | M25 | Complete - issue-scoped M25 browser evidence, owning tests, and regenerated patch checks are recorded |
| M25-DOGFOOD-REQ-02 | M25 | Complete - release browser WASM, deploy reuse, phase timing, and typing/loading separation are browser-evidenced |
| M25-DOGFOOD-REQ-03 | M25 | Complete - 3-10 page-size customization and candidate caps are native/browser tested |
| M25-DOGFOOD-REQ-04 | M25 | Complete - bare-grave Luna reverse lookup and explicit `vl`/`vc` side lookup triggers are native/browser tested |
| M25-DOGFOOD-REQ-05 | M25 | Complete - top-control layout, settings order, and alignment are browser-evidenced |
| M25-DOGFOOD-REQ-06 | M25 | Complete - checkbox and radio affordances preserve the local Tailwind component stack |
| M26-PERF-REQ-01 | M26 | Complete - native large-real-asset benchmark coverage with cold/warm, full-ABI/engine-only, correction-path, and memory/allocation reporting |
| M26-PERF-REQ-02 | M26 | Complete - browser keydown-to-paint instrumentation for normal typing, long phrases, paging, and reverse lookup |
| M26-PERF-REQ-03 | M26 | Complete - startup attribution below `runtime:initialized` |
| M26-PERF-REQ-04 | M26 | Complete - startup owner deferred to M27; measured TypeDuck dynamic-correction optimization landed with before/after evidence |
| M26-PERF-REQ-05 | M26 | Complete - compatibility, integration, and TypeDuck-Web patch discipline gates remain green |
| M27-STARTUP-REQ-01 | M27 | Complete - browser-paid startup-path reconciliation and native sub-attribution by owner |
| M27-STARTUP-REQ-02 | M27 | Complete - hard Windows process-memory evidence recorded |
| M27-STARTUP-REQ-03 | M27 | Complete - browser fresh/reload startup evidence mapped to native owners |
| M27-STARTUP-REQ-04 | M27 | Complete - measured top-owner startup bottleneck materially reduced with timing, memory, and browser before-after evidence |
| M27-STARTUP-REQ-05 | M27 | Complete - compatibility, integration, benchmark, browser, and patch discipline gates remain green |
| M27-STARTUP-REQ-06 | M27 | Complete - engine controls classified as live, browser-only, deploy-time, or local-runtime-only with marker evidence |
| M28-PARTIAL-REQ-01 | M28 | Complete - history evidence classifies partial selection as previously missing support |
| M28-PARTIAL-REQ-02 | M28 | Complete - capture-not-confirm TypeDuck v1.1.2 oracle fixture for partial selection |
| M28-PARTIAL-REQ-03 | M28 | Complete - native engine/API tests cover segment-aware partial commit and FORK-PARITY-03 learning preservation |
| M28-PARTIAL-REQ-04 | M28 | Complete - TypeDuck-Web browser evidence covers continued selection and raw-tail guard |
| M28-PARTIAL-REQ-05 | M28 | Complete - compatibility and integration gates remain green |
| M28F-UPSTREAM-REQ-01 | M28 follow-up | Complete - Space/default-confirm uses scoped consumed-span recomposition |
| M28F-UPSTREAM-REQ-02 | M28 follow-up | Complete - hybrid upstream-engine Jyutping fixture with pinned source-YAML provenance and no local paths |
| M28F-UPSTREAM-REQ-03 | M28 follow-up | Complete - narrow decision for accepted hybrid fixture, live-site exclusion, and comment-scope exclusion |
| M28F-UPSTREAM-REQ-04 | M28 follow-up | Complete - native ordering tests follow the accepted upstream-Jyutping fixture and preserve TypeDuck profile guards |
| M28F-UPSTREAM-REQ-05 | M28 follow-up | Complete - TypeDuck-Web browser evidence for Space/default-confirm and ranking |
| M28F-UPSTREAM-REQ-06 | M28 follow-up | Complete - full compatibility and integration gates |
| M29-PERF-REQ-01 | M29 | Complete - fresh native startup, browser startup, and typing baselines |
| M29-PERF-REQ-02 | M29 | Complete - `1.79GB` peak classified as repeated-benchmark high-water, with real single-startup ready pressure around `1.10GB` |
| M29-PERF-REQ-03 | M29 | Complete - `spelling_algebra_expand` remained the top startup owner before optimization |
| M29-PERF-REQ-04 | M29 | Complete - browser keydown-to-paint attribution recorded worker/native, response mapping, React update, and paint-proxy owners |
| M29-PERF-REQ-05 | M29 | Complete - no-op spelling-algebra replacement allocation avoidance reduced the native startup owner with before/after evidence |
| M29-PERF-REQ-06 | M29 | Complete - full compatibility and integration gates |
| M30-PERF-REQ-01 | M30 | Complete - fresh native/browser baselines captured before implementation |
| M30-PERF-REQ-02 | M30 | Complete - M29 markdown tables reconciled to committed JSON |
| M30-PERF-REQ-03 | M30 | Complete - expanded-table Lever A landed with memory/startup evidence |
| M30-PERF-REQ-04 | M30 | Complete - shared-payload rewrite deferred after Lever A evidence |
| M30-PERF-REQ-05 | M30 | Complete - sentence-DP backpointers deferred because long-input rows did not justify the rewrite |
| M30-PERF-REQ-06 | M30 | Complete - correction-stress indexing deferred because correction rows stayed flat |
| M30-PERF-REQ-07 | M30 | Complete - full compatibility and integration gates |
| M33-PERF-REQ-01 | M33 | Complete - fresh native and cross-engine baselines captured before M33 claims; evidence under `docs/reports/evidence/m33-2026-06-23/` |
| M33-PERF-REQ-02 | M33 | Complete - comparison fairness enforced by lazy reverse lookup, so no headline compares luna-plus-stroke Yune startup against luna-only librime |
| M33-PERF-REQ-03 | M33 | Complete - lazy reverse-lookup loading preserves first-use reverse lookup behavior and removes eager `stroke` startup/session/resident asymmetry |
| M33-PERF-REQ-04 | M33 | Complete - build-once dictionary translator sharing reduces repeated schema/session cost with byte-identical candidates and source invalidation coverage |
| M33-PERF-REQ-05 | M33 | Complete - lazy spelling-algebra lookup was spike-gated and deferred because prism-only lookup is insufficient; byte-identical lookup needs a queryable table+prism path |
| M33-PERF-REQ-06 | M33 | Complete - mmap compiled artifacts deferred by stop gate; low-risk slice fixed warm re-select/session while remaining gaps are cold startup, footprint, and per-key candidate-pipeline/storage representation |
| M33-PERF-REQ-07 | M33 | Complete - public performance report, root-cause report, README, roadmap, requirements, and archived plan now use the fair M33 rerun with cold/warm and peak-memory caveats; no chart SVG generated |
| M33-PERF-REQ-08 | M33 | Complete - full Rust compatibility, ABI, benchmark, and diff gates run for M33; no frontend/browser claim made because no frontend/WASM path changed |
| M31-PUBLIC-REQ-00 | M31 | Complete - public identity and provenance are recorded in `public-demo/PROVENANCE.md`, the app is named `yune-web`, and the page identifies itself as a Yune engine demo through a TypeDuck-Web-derived harness |
| M31-PUBLIC-REQ-01 | M31 | Complete - P2-WIN-02 is complete, P2-WIN-01 priority is unaffected, and M31 stayed scoped to browser delivery/UI/devops without widening Windows or default ABI surfaces |
| M31-PUBLIC-REQ-02 | M31 | Complete - the public demo exposes only Hong Kong Traditional plus `hk2s` Simplified, both backed by native/runtime/browser evidence; unsupported OpenCC standards are absent |
| M31-PUBLIC-REQ-03 | M31 | Complete - the output-standard control changes candidate output through Yune's `simplification` option, not browser-only postprocessing |
| M31-PUBLIC-REQ-04 | M31 | Complete - `public-demo/build.ps1`, the checked-in patch, bridge, WASM, schema asset manifest, and evidence reproduce the deployable `yune-web` package from checked-in Yune state |
| M31-PUBLIC-REQ-05 | M31 | Complete - Cloudflare local preview and deployed smoke passed for app boot, WASM load, schema asset load, `jyut6ping3_mobile` typing, output-standard toggle, and root routing at `https://yune-web.pages.dev` |
| M31-PUBLIC-REQ-06 | M31 | Complete - AI remains default-off/local-only with no remote calls, telemetry, secrets, or third-party model keys in the public smoke; AI-off classic output is preserved |
| M31-PUBLIC-REQ-07 | M31 | Complete - public payload is measured, pruned, and documented: one public schema surface, 41 deployed files, 32,147,434 bytes, and non-public Cangjie/Loengfan/side-layout assets excluded |
| M31-PUBLIC-REQ-08 | M31 | Complete with measured caveat - public boot is selected-schema-only for `jyut6ping3_mobile` and excludes non-public schema families, but the current TypeDuck product schema still boots Luna and scholar lookup assets required by exposed behavior instead of fully deferring every reverse-lookup dependency |
| M31-PUBLIC-REQ-09 | M31 | Complete with measured caveat - schema/runtime assets have SHA-256 metadata and Cache Storage warm-cache evidence (`31` hits, `0` misses on warm reload); reverse-lookup-specific cold-first-use deferral remains future work for the current schema dependency set |
| M31-PUBLIC-REQ-10 | M31 | Complete - Pages Direct Upload, browser Cache Storage, cold/warm startup markers, and deployed smoke evidence are recorded; no browser-startup or typing speed win is claimed |
| M31-PUBLIC-REQ-11 | M31 | Complete - WASM/download-size work is reported only as delivery packaging/pruning; Rust engine latency claims remain in M34/M36/M37 performance tracks |
| M31-PUBLIC-REQ-12 | M31 | Complete - Rust, runtime, TypeDuck-Web build, local/deployed Playwright, patch reverse/forward, Wrangler dry-run, and `git diff --check` gates passed for M31 |
| M31-PUBLIC-REQ-13 | M31 | Complete - public-facing strings, package name, deployment config, docs, evidence labels, route copy, and repo-owned app path use `yune-web`; the upstream-derived source checkout lives under `apps/yune-web/source/` |
| M34-PERF-REQ-01 | M34 | Complete - fresh native and fair cross-engine M33-surface baselines captured under `docs/reports/evidence/m34-queryable-table-prism/` |
| M34-PERF-REQ-02 | M34 | Complete - attribution identified full-result materialization/context work as the accepted Lever A owner; a temporary `ni`/`hao` diagnostic artifact records lookup, prefix scan, eligibility probe, eager materialization, sort, and bounded materialization spans, with no diagnostic instrumentation retained |
| M34-PERF-REQ-03 | M34 | Complete - full-list readers were audited, including correction, sentence, prefix fallback, prediction-never-first, filters/rankers, ABI context, and candidate-list iterators |
| M34-PERF-REQ-04 | M34 | Complete - internal bounded request/result contract added without changing public C ABI; eager translation remains the fallback |
| M34-PERF-REQ-05 | M34 | Complete - safe short `luna_pinyin` first-page refresh uses bounded materialization with lazy full-list expansion; full-list filters/rankers/userdb paths fallback eager |
| M34-PERF-REQ-06 | M34 | Complete - internal heap-backed `TableLookup` abstraction covers exact, prefix, and all-code queries with focused tests |
| M34-PERF-REQ-07 | M34 | Closed by stop-gate - compiled `.table.bin` query storage was not implemented because current readers still materialize owned dictionaries and candidate payload parity work remains |
| M34-PERF-REQ-08 | M34 | Closed by stop-gate - prism+table candidate integration was not implemented because prism does not carry candidate payload bytes and needs a table-backed payload query path first |
| M34-PERF-REQ-09 | M34 | Complete - TypeDuck `jyut6ping3` behavior stayed byte-identical under `cantonese_parity` and `typeduck_web`; default/profile ABI isolation preserved |
| M34-PERF-REQ-10 | M34 | Complete - native `luna_pinyin` `ni` full-ABI improved `1,760.250us` -> `1,132.950us`; TypeDuck full-ABI watch rows stayed within the accepted guard |
| M34-PERF-REQ-11 | M34 | Closed by stop-gate - memory/cold-start claims are separated; mmap/borrowed storage was not attempted because queryable table/prism storage did not land |
| M34-PERF-REQ-12 | M34 | Complete - Rust, focused parity, workspace, benchmark, report, and diff gates run; no runtime/browser gates needed because those paths did not change |
| M34-PERF-REQ-13 | M34 | Complete - `my_rime` reference split recorded; engine data-path lessons stay in engine work and delivery/cache lessons route to M31 |
| M35-PERF-REQ-01 | M35 | Complete - fresh M35 native and fair cross-engine baselines captured under `docs/reports/evidence/m35-compact-table-prism-storage/` for startup/session, memory, `ni`, `hao`, `zhongguo`, and TypeDuck watch rows |
| M35-PERF-REQ-02 | M35 | Complete - `TableLookup` now returns `LookupCandidate` / `LookupCandidateEntry` views instead of heap `&[Candidate]` slices, materializing owned `Candidate` values only at selected or compatibility boundaries |
| M35-PERF-REQ-03 | M35 | Complete - heap-backed behavior through the candidate-view API is covered by focused lookup tests and existing upstream/TypeDuck parity gates |
| M35-PERF-REQ-04 | M35 | Complete - `CompactTableStore` preserves text, raw code/comment, raw weight/order, correction/tolerance data, and advanced dictionary payloads; TypeDuck lookup-record compact enablement remains guarded by no-go |
| M35-PERF-REQ-05 | M35 | Complete - compact exact, prefix, and all-code queries reproduce heap-backed output in focused tests, and upstream `luna_pinyin` parity passes with compact storage active |
| M35-PERF-REQ-06 | M35 | Complete - prism lookup is used only for spelling/canonical-code discovery while compact table storage supplies payloads; upstream `luna_pinyin` no longer re-expands heap aliases |
| M35-PERF-REQ-07 | M35 | Complete - `StaticTableTranslator` chooses private heap or boxed compact storage; compact-active upstream `luna_pinyin` does not retain heap `entries_by_code` |
| M35-PERF-REQ-08 | M35 | Complete - compact runtime storage is enabled for safe upstream `luna_pinyin`; TypeDuck `jyut6ping3` stays on documented heap fallback because profile invariants are broader |
| M35-PERF-REQ-09 | M35 | Complete with stretch no-go - native short-input rows improved (`hao_engine_only` `1092.879us` -> `750.517us`, `ni_engine_only` `891.791us` -> `697.044us`), but low-hundreds stretch remains a measured future owner |
| M35-PERF-REQ-10 | M35 | Complete with stretch no-go - memory attribution separates upstream and product schemas; upstream dictionary-specific `translator_install` delta dropped `37556224` -> `9822208` bytes, while fair-harness `40-70 MB` whole-process peak stretch remains deferred |
| M35-PERF-REQ-11 | M35 | Closed by no-go - mmap/borrowed storage was not attempted because compact owned storage removed the upstream expansion delta and the remaining whole-process peak needs a separate borrowed/demand-paged design |
| M35-PERF-REQ-12 | M35 | Complete - TypeDuck profile rows remain byte-identical under `cantonese_parity` and `typeduck_web`; full-ABI watch rows stayed within the 10% guard while heap fallback remains active |
| M35-PERF-REQ-13 | M35 | Complete - performance and root-cause reports separate native engine-only, full-ABI, fair cross-engine, memory, and browser-delivery claims; cross-engine ratios are not the headline |
| M35-PERF-REQ-14 | M35 | Complete - Rust, focused parity, workspace, benchmark, fair cross-engine, docs/report, and diff gates pass; runtime/browser gates are N/A because no runtime/browser-visible files changed |
| M36-PERF-REQ-01 | M36 | Complete - native in-process harness records Track A (`luna_pinyin` Yune vs librime) separately from Track B (`jyut6ping3_mobile` Yune before/after), with startup, session, per-key, resident working-set, and peak working-set rows under `docs/reports/evidence/m36-product-path/` |
| M36-PERF-REQ-02 | M36 | Complete - strategy evidence separates product owners from comparison-only rows; Track A ratios remain caveats and Track B before/after rows are the only TypeDuck product performance headline |
| M36-PERF-REQ-03 | M36 | Complete - product path status CSV records stale unsupported shipped product blobs at baseline and final `compiled_ready=true` rebuilt table/prism/reverse artifacts for both `jyut6ping3` and `jyut6ping3_scolar` |
| M36-PERF-REQ-04 | M36 | Closed by no-go - no browser/WASM/runtime-visible files changed, so browser free wins, `INITIAL_MEMORY`, bundle gating, and delivery/cache claims remain M31 work with browser smoke required before any claim |
| M36-PERF-REQ-05 | M36 | Closed by no-go - standalone byte-arena/StringId interning was not the selected product owner after attribution; the accepted storage win is compiled-active no-marisa product artifacts plus compact storage, with API-boundary owned candidates preserved |
| M36-PERF-REQ-06 | M36 | Complete with measured `rsmarisa` no-go - actual shipped `jyut6ping3` and `jyut6ping3_scolar` blobs are stale and unsupported as a table/prism/reverse set, so M36 lands schema-scoped Yune-readable no-marisa re-emitted artifacts with no final runtime `SourceFallback` |
| M36-PERF-REQ-07 | M36 | Complete - product translator installation preserves prism payloads on compiled product loads, writes configured prism stems such as `jyut6ping3_mobile`, and keeps TypeDuck rich comments, lookup records, correction/tolerance, partial selection, long composition, and userdb gates green |
| M36-PERF-REQ-08 | M36 | Closed by no-go - bounded/lazy candidate windows were not generalized to TypeDuck product rows because whole-list, paging, filters/rankers, correction/tolerance, context, and userdb invariants remain broader than the safe upstream subset |
| M36-PERF-REQ-09 | M36 | Complete - performance/root-cause reports, checked-in charts, and evidence docs separate native vs browser, product vs comparison, memory vs latency, and landed wins vs no-go strategies; no "matched librime", "faster than librime", or browser claim is made |
| M36-PERF-REQ-10 | M36 | Complete - final fmt/clippy/focused parity/workspace tests, `typeduck_web`, `frontend_baselines`, native M36 benchmark evidence, report SVG/XML checks, `git diff --check`, and completed-plan updates are recorded; runtime/browser/patch gates are N/A because no runtime-visible files changed |
| M37-ENGINE-01 | M37 | Complete - Phase 0 attribution explains Track B `hai` materialization/filtering owners and product memory owners; evidence in `phase-0-baseline/` |
| M37-ENGINE-02 | M37 | Complete - selected product storage is byte-backed/mapped, `rsmarisa` was tried on real product marisa payloads, and memory moved materially |
| M37-ENGINE-03 | M37 | Complete - final product status proves fresh artifacts, no `SourceFallback`, and active byte-backed mapped product path |
| M37-ENGINE-04 | M37 | Complete - default Track B product rows prove page-bounded materialization in `m37_metrics.csv` |
| M37-ENGINE-05 | M37 | Complete - `RimeGetContext` exports page reads through page snapshot counters without full candidate-list cloning |
| M37-ENGINE-06 | M37 | Complete - focused upstream and TypeDuck behavior gates remained byte-identical; browser speed gates were N/A because no browser claim was made |
| M37-ENGINE-07 | M37 | Complete - Track B `hai` moved from `15,241.000us` to `8,336.800us` and residual owner is named |
| M37-ENGINE-08 | M37 | Complete - Track B product memory moved from the M36 baseline and Track A working-set evidence was refreshed |
| M37-ENGINE-09 | M37 | Complete - final native product path reports `mapping_mode=mmap`; `rsmarisa` probes also report `mmap` |
| M37-ENGINE-10 | M37 | Complete - reports separate native/browser and Track A/Track B claims; no browser speed claim is made |
| M37-ENGINE-11 | M37 | Complete - final quality gates are recorded in the completed M37 plan; runtime/browser gates were N/A for performance claims |
| M38-ENGINE-01 | M38 | Complete - final claims are native isolated-engine only; product/frontend/browser rows are not used as closeout evidence |
| M38-ENGINE-02 | M38 | Complete - phase 0 recorded fresh same-run upstream librime and Yune baseline rows |
| M38-ENGINE-03 | M38 | Complete - owner attribution recorded for lifecycle, lookup, materialization, context, memory, and allocation rows |
| M38-ENGINE-04 | M38 | Complete - final Track A hot path uses `rsmarisa_byte_backed` deployed table lookup with positive counters and zero no-marisa fallback |
| M38-ENGINE-05 | M38 | Complete - final selected table/prism bytes are mmap-backed with table/prism heap mirror bytes `0` |
| M38-ENGINE-06 | M38 | Complete - final startup/session medians are within `1.25x` of same-run librime |
| M38-ENGINE-07 | M38 | Complete - final `hao`, `ni`, and `zhongguo` rows are each within `5x` of same-run librime |
| M38-ENGINE-08 | M38 | Complete - raw prism, raw table, raw `rsmarisa`, translator, and context-export rows recorded |
| M38-ENGINE-09 | M38 | Complete - first-page reads are page-bounded for target rows with fallback counters reported |
| M38-ENGINE-10 | M38 | Complete - working set, peak working set, allocation counters, and remaining memory gap are reported honestly |
| M38-ENGINE-11 | M38 | Complete - touched upstream `luna_pinyin` behavior and shared compatibility tests are green |
| M38-ENGINE-12 | M38 | Complete - final reports make only native isolated-engine claims |
| M38-ENGINE-13 | M38 | Complete - fmt, clippy, focused tests, workspace tests, final native benchmark, report checks, and `git diff --check` recorded |
| POST-M38-PERF-01 | Post-M38 | Draft - next same-run native benchmark must include required long continuous pinyin rows `ceshiyixiachangjushuruxingnengzenyang` and `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` |
| POST-M38-PERF-02 | Post-M38 | Draft - long-input rows must carry the same owner/status/memory/allocation evidence as the M38 rows |
| POST-M38-PERF-03 | Post-M38 | Draft - long-input optimization claims must name the measured owner |
| POST-M38-PERF-04 | Post-M38 | Draft - long-composition translator attribution must split the current top-level translator bucket into inner owners |
| POST-M38-PERF-05 | Post-M38 | Draft - memory follow-up must use the post-M38 working-set/peak baseline and add heap-owner attribution |
| POST-M38-PERF-06 | Post-M38 | Draft - long uninterrupted 50+ character input is a primary engine requirement and must be covered by a length-curve benchmark plus an explicit parity/no-go gate |
| POST-M38-PERF-07 | Post-M38 | Draft - next engine-performance milestone must report and block regressions across startup/session, short input, long input, mmap/rsmarisa, bounded output, memory, and behavior |

**Coverage:**

- v1 requirements: 25 total
- v2 validation requirements: 7 total
- TypeDuck-Web integration requirements: 15 total
- TypeDuck-Windows native IME requirements: 7 total
- M12/M17/M18 upstream oracle and behavioral parity requirements: 10 total, 10 complete
- M13 AI-native frontend exposure requirements: 6 total, 6 complete
- M14–M16 TypeDuck-Web fork parity requirements: 7 total, 7 complete (M16 complete with explicit browser/userdb inspection limits)
- Fork parity backlog (Cantonese engine-parity, vs upstream 1.17.0): 9 total, 9 complete; see [`ledgers/fork-parity-ledger.md`](./ledgers/fork-parity-ledger.md)
- M20 web demo showcase controls requirements: 7 total, 7 complete
- M19 schema breadth and TypeDuck-profile ABI requirements: 5 total, 5 complete
- M22 web playground requirements: 4 total, 4 complete
- M24 TypeDuck-Web dogfooding requirements: 5 total, 5 complete
- M25 TypeDuck-Web dogfooding round 2 requirements: 6 total, 6 complete
- M26 performance hardening requirements: 5 total, 5 complete
- M27 TypeDuck-Web startup runtime init and control-classification requirements: 6 total, 6 complete
- M28 TypeDuck partial candidate selection requirements: 5 total, 5 complete
- M28 follow-up upstream Jyutping composition requirements: 6 total, 6 complete, 0 draft
- M29 startup memory and typing performance requirements: 6 total, 6 complete, 0 draft
- M30 engine representation performance requirements: 7 total, 7 complete, 0 draft
- P2-WIN-02 TypeDuck Windows boundary compatibility requirements: 6 total, 6 complete, 0 open
- M33 engine native lookup performance requirements: 8 total, 8 complete, 0 draft
- M31 `yune-web` public demo readiness requirements: 14 total, 12 complete, 2 complete with measured caveat, 0 draft
- M34 lazy candidate pipeline and queryable table+prism performance requirements: 13 total, 10 implemented/complete, 3 closed by stop-gate, 0 draft
- M35 compact table+prism runtime storage performance requirements: 14 total, 13 complete, 1 closed by no-go, 0 draft
- M36 product-path engine optimization requirements: 10 total, 7 complete, 3 closed by no-go, 0 draft
- M37 engine hyper-optimization requirements: 11 total, 11 complete, 0 planned
- M38 engine performance parity requirements: 13 total, 13 complete, 0 active
- Post-M38 engine performance follow-up requirements: 7 total, 0 complete, 7 draft
- Mapped to phases: 251
- Unmapped: 0

---

_Requirements defined: 2026-04-28_ _Last updated: 2026-06-25 - M38 engine performance parity is complete with same-run upstream librime evidence, mmap-backed selected table/prism bytes, real `rsmarisa` Track A hot-path lookup, page-bounded first-page iteration, memory/allocation attribution, startup/session within `1.25x`, `hao`/`ni`/`zhongguo` within `5x`, behavior gates, honest native-only claims, and final quality gates recorded. The post-M38 long-input baseline adds `ceshiyixiachangjushuruxingnengzenyang` and shows broader typing parity is not closed (`412,192.727us` Yune versus `294.151us` librime); the 59-character stress run adds `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` and records `1,202,404.588us` Yune versus `702.212us` librime. Draft follow-up requirements now require keeping both long rows, treating 50+ uninterrupted input as a primary engine requirement, splitting the long-composition translator owner, adding a length-curve benchmark, blocking cross-dimension regressions, and adding heap-owner attribution before a memory reduction claim. `roadmap.md` is now a current-state dashboard and the historical milestone ledger lives in `ledgers/milestone-history.md`. M37 engine hyper-optimization is complete with latency and memory attribution, byte-backed/native-mapped product storage, real `rsmarisa` product probes, fresh compiled artifacts, page-bounded materialization/context export, `hai` movement, product memory movement, behavior parity, honest claims, and final quality gates recorded in evidence. M31 remains complete as the `yune-web` public demo readiness milestone with browser delivery claims scoped to packaging/pruning/cache evidence, not startup/typing wins. M36 remains complete as the product-path engine optimization milestone after M35, with Track A/Track B and browser-delivery caveats preserved. M35 remains complete as the compact table+prism runtime storage milestone. M33, M34, P2-WIN-02, M30, M29, M28 follow-up, M28, M27, M26, M25, M24, M19, M23, M18, M22, M21, M20, and M10 remain complete as previously recorded._
