# Requirements: Yune

**Defined:** 2026-04-28
**Core Value:** Yune should preserve predictable classic RIME input while making AI/LLM assistance a first-class, local-first, non-blocking source of candidates, ranking, context, and memory.

> **Note (2026-06-17):** The GSD `.planning/` system has been retired. This requirement
> list and its statuses are preserved here; the **Phase** references (e.g. in the
> Traceability table) are historical GSD labels — now only in git history — kept for
> context. The live roadmap is [`roadmap.md`](./roadmap.md); decisions are in
> [`decisions.md`](./decisions.md); conventions in [`CONVENTIONS.md`](./CONVENTIONS.md).

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

Requirements for the next integration milestone. These requirements turn the
Phase 6 TypeDuck-Web validation and the seed Rust adapter into a browser-usable
path before AI-native product work begins.

**M9 completed real-assets validation.** The build-out (WASM export contract,
TS bridge, browser filesystem) landed, and the WASM artifact now builds as
loadable Emscripten `yune-typeduck.js`/`.wasm` with a Node smoke for one
`yune_typeduck_*` call plus one `FS` operation. A post-review audit found the
first WI-4 browser matrix used the placeholder echo path for candidate evidence.
HR-1 proves the patched TypeDuck-Web worker can load real
`jyut6ping3_mobile` assets and render `nei` candidates (`你`, `呢`, `尼`) in a
real browser. HR-2 resolves the startup `setOption` export/wrapper/adapter gap,
HR-3 proves browser `deploy()` returns true with real assets after adding the
plain `jyut6ping3.schema.yaml` preload, and HR-4 proves live-worker persistence
sync plus real reload survival. HR-5 reruns the full browser matrix against real
assets, including paging, deletion, phrase commit, dictionary-panel rendering,
and zero warning/error console entries after the post-review pure-modifier
delete-path fix. Rich dictionary-comment byte parity is committed in
`cantonese_parity`; the browser-shaped native rich-comment test also asserts the
full real-assets path when local v1.1.2 oracle build assets are present. HR-6
locks the shared reverse-lookup joiner and schema-prompt bytes against the
TypeDuck v1.1.2 oracle. HR-7 closes
M9 with **GO WITH CONDITIONS** for gated AI-native frontend exposure.

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

**Status: complete.** Upstream `rime/librime 1.17.0` is the default core
oracle target. TypeDuck `v1.1.2` remains a compatibility-profile oracle for
TypeDuck-Web/Windows only. The official upstream Windows MSVC release binary is
the behavioral-capture oracle; local source builds are a reproducibility check
rather than the primary capture source. Later M17/M18 closeouts resolved the
former sentence/lattice and processor blockers with fresh upstream fixtures.

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

**Status: complete as a TypeDuck compatibility profile.** A first pass landed
(Phases 11-16), M9 web validation is complete, and the archived pre-M12 native
Windows package smoke has been superseded by current M10 T1/T2 profile package,
build/link, packaged lifecycle evidence, and stock TypeDuck-Windows real-server
IPC smoke evidence against `rime_get_typeduck_profile_api()`. The shared comment
requirement is covered for the current v1.1.2 oracle slices; captured Cantonese
engine fixtures are active, and the M10 T3 smoke now proves key input/output
through the native TypeDuck-Windows/weasel IPC path. This T3 proof is a stock
server/client IPC smoke, not an interactive TSF typing or visible candidate-panel
rendering smoke; those move to the Phase 2 Windows product/frontend track. These
requirements target that native path and no longer define Yune's active core
oracle milestone.

- [x] **WIN-TEST-01**: Windows `cargo test --workspace` has a trustworthy green baseline, including portable signature timestamp shape and test-only poison-lock recovery.
- [x] **WIN-ABI-01**: `config_list_append_{string,bool,int,double}` helper behavior is implemented and exposed through the named, opt-in M19 TypeDuck-profile accessor; the default upstream `rime_get_api()` does not expose these fork-only slots.
- [x] **WIN-ORACLE-01**: The TypeDuck-HK/librime v1.1.2 binary and pinned schema are captured as a reproducible oracle, or a precise blocker is documented.
- [x] **WIN-COMMENT-01**: Candidate comment semantics match the v1.1.2 oracle for dictionary lookup payloads, reverse lookup joins, and prompt/schema identity. Dictionary lookup payload bytes, schema-prompt bytes, and reverse-lookup joiner coverage are oracle-backed.
- [x] **WIN-BUILD-01**: Yune produces a current TypeDuck-profile native Windows package (`rime.dll`, import `.lib`, upstream-shaped default headers, and `rime_typeduck_profile_api.h`) and the package script loads the packaged DLL through `rime_get_typeduck_profile_api()`.
- [x] **WIN-PARITY-01**: Cantonese/Jyutping parity regression coverage locks the captured v1.1.2 engine behavior in active `cantonese_parity` tests; schema-menu/userdb observations remain frontend/T3 evidence limits.
- [x] **WIN-FRONTEND-01**: TypeDuck-Windows builds/links against the Yune package and passes a stock real-server IPC smoke. Stock `TypeDuckServer.exe` starts from `output\`, loads packaged Yune `output\rime.dll`, and stock `TestTypeDuckIPC.exe /console` returns a nonzero session, sends `ngohaig` key events, and receives `status.schema_id=jyut6ping3` plus candidate/context data. Tracked evidence: `docs/plans/archive/m10-evidence/t3-stock-real-server/`. Caveat: interactive TSF typing, visible candidate-window rendering, and candidate-panel UI behavior are deferred to the Phase 2 Windows product/frontend track.

## Future Requirements

Deferred beyond the TypeDuck-Web browser integration milestone. Tracked but not in the current roadmap.

### Plugin Compatibility

- **PLUGIN-01**: Yune can load or adapt librime C++ plugin ABI extensions.
- **PLUGIN-02**: Lua, octagram, predict, proto, and other distribution plugin ecosystems have migration paths.

### Product Frontend

- **FRONTEND-01**: Yune ships a new graphical end-user frontend.
- **FRONTEND-02**: Yune-specific UI features expose optional AI ranking and contextual completion controls.

### iOS Keyboard Developer Track

- **IOS-DEV-01**: Yune provides a documented iOS package/host contract for
  keyboard developers, separate from the default upstream `RimeApi` table and
  without changing `RimeCandidate`.
- **IOS-DEV-02**: iOS resource deployment is explicit: schemas, dictionaries,
  OpenCC data, and userdb storage are bundled or generated in a sandbox-safe
  location without arbitrary filesystem paths or startup recompilation surprises.
- **IOS-DEV-03**: Swift/Obj-C integration defines keyboard-extension lifecycle,
  memory, persistence, and privacy constraints before TypeDuck iOS exposure is
  claimed.
- **IOS-DEV-04**: Mobile-specific behavior such as near-key correction maps or
  keyboard-layout differences is data/config-driven or UI-owned, not hardcoded
  as desktop-vs-mobile engine branches.

### AI Extension Layer

- [x] **AI-01**: Engine exposes an `AiCandidateProvider` interface and staged,
  input-keyed AI results without replacing classic translators. S1 implements
  this for the direct CLI mock path.
- [x] **AI-02**: Candidate ranking supports local model and rule-backed implementations with deterministic timeout/fallback behavior. S2 covers the background worker, input-keyed fallback, fixed-point confidence metadata, and confidence-ordered AI merge; S5 adds the local rule-backed provider.
- [x] **AI-03**: Contextual phrase and sentence completion can produce source-labeled AI candidates without allowing AI candidates to auto-commit by default. S1 covers source labeling and the no-default-auto-commit gate; S5 adds contextual local-model completions.
- [x] **AI-04**: Context providers define what app, field, preceding text, cursor, schema, and candidate-list data may be shared with AI providers. S3 implements `AiContext` plus `EngineAiContextProvider` snapshots.
- [x] **AI-05**: Memory store records user vocabulary, phrase preferences, and domain terms through explicit, inspectable, clearable policy. S4 implements `MemoryStore`, clear/disable controls, snapshot import/export, and `.ai-memory` namespace helpers.
- [x] **AI-06**: Privacy policy disables learning and remote calls for sensitive contexts and keeps classic input fully functional when AI is disabled. S3 blocks remote calls; S4 applies the same privacy gate to AI memory writes.
- [x] **AI-07**: CLI frontend surrogate can demonstrate AI candidate/ranking behavior with mock and local providers before native frontends expose it. S1 covers `yune-cli run --ai-provider mock`; S5 adds `--ai-provider local`.

## M13 AI-native Frontend Exposure Requirements

**Status: complete for TypeDuck-Web.** M13 exposes the M11 local AI layer through
TypeDuck-Web only, default-off and local-first, with the key path still
provider-free. Additional native frontend exposure remains future work.

- [x] **M13-AI-01**: `yune_typeduck_process_key` remains provider-free and classic-first; AI provider work runs only through the second-pass `yune_typeduck_stage_ai` path.
- [x] **M13-AI-02**: Browser AI is default-off, can be toggled without redeploy, and `set_ai_enabled(false)` clears any staged result for the current input.
- [x] **M13-AI-03**: AI candidates render after the classic top candidate, never at index 0, with source labels derived from engine snapshot data aligned to the rendered page; `RimeCandidate` and the upstream `RimeApi` table remain unchanged.
- [x] **M13-AI-04**: Browser commit safety preserves classic default commit behavior; AI rows never auto-commit and require explicit selection.
- [x] **M13-AI-05**: Explicit AI commits do not touch librime userdb; under the sensitive browser default, AI-memory learning is suppressed and no `.ai-memory` persistence is written.
- [x] **M13-AI-06**: Real TypeDuck-Web browser evidence covers AI-off byte identity, AI-on source-labeled second-pass rows, no auto-commit, explicit AI selection, and zero warning/error console entries.

## M14–M16 TypeDuck-Web Fork Parity Requirements

**Status: M14 capture complete; M15 engine parity complete; M16 browser validation complete with documented browser-surface limits.** Complete the TypeDuck
`jyut6ping3` target so the TypeDuck-Web example behaves like the fork.
Oracle-measured against TypeDuck-HK v1.1.2; `jyut6ping3` is dictionary-driven
and does **not** require the upstream language model (Track 2 / M17). See
roadmap M14–M16 and `decisions.md` D-27.

- [x] **TYPEDUCK-PARITY-01**: A v1.1.2 capture path is established by parameterizing the scenario-capable upstream probe's oracle identity (modules/distribution/provenance) — or a thin v1.1.2 wrapper — and Cantonese goldens are captured from the v1.1.2 oracle binary for `combine_candidates`, `show_full_code`, `enable_sentence`, completion/prediction, and correction at multiple input lengths.
- [x] **TYPEDUCK-PARITY-02**: The oracle-observable surface for schema-menu hiding (`hide_lone_schema`/`hide_caret`) is identified (config API, schema-list/switcher API, or TypeDuck-Web UI state) and emitted behavior is captured — not static config inspection alone.
- [x] **TYPEDUCK-PARITY-03**: A feasibility spike determines whether per-entry userdb pronunciations are capturable via the levers user-dict export/import/seed hooks; if not, the gap is documented as a fork-only deferral with the precise blocker.
- [x] **TYPEDUCK-PARITY-04**: `combine_candidates` (candidate grouping) and `show_full_code` (cangjie preedit algebra) are implemented and pass the captured goldens through Yune's real engine path.
- [x] **TYPEDUCK-PARITY-05**: `enable_sentence`, completion ranking, and correction/tolerance tuning are refined to pass the captured goldens.
- [x] **TYPEDUCK-PARITY-06**: OpenCC `hk2s` coverage is expanded from the built-in slice to the full conversion data the jyut6ping3 simplifier needs.
- [x] **TYPEDUCK-PARITY-07**: The TypeDuck-Web browser matrix passes for the app-exposed `jyut6ping3_mobile` surface plus M13 AI, while deploy-only variants (`common:/separate_candidates`, `common:/show_full_code`), schema-menu UI hiding, correction UI detail, and per-entry userdb pronunciation are explicitly documented as browser/userdb inspection limits backed by M14/M15 oracle evidence.

## Fork Parity Backlog — Cantonese engine-parity (complete)

**Status: complete.** Derived from the full Cantoboard + TypeDuck fork-vs-`1.17.0` audit in
[`fork-parity-ledger.md`](./fork-parity-ledger.md). These were genuine fork deltas Yune
needed to preserve or explicitly decline (distinct from the upstream-depth Track 2 M17–M19 work). M14–M16
closed the *captured* browser surface; these were the *uncaptured / partial* deltas the
goldens did not exercise. Each completed implementation was measured against the v1.1.2
oracle or closed by an explicit product decision.

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

**Status: complete.** M20 is a web/demo track for this repo's patched internal
TypeDuck-Web harness, not a reopened M13 and not the separately cloned
`TypeDuck-HK/TypeDuck-Web` product. It exposes already-supported Yune behavior
through honest UI controls and guided scenarios while preserving the M9/M13/M16
browser gates and the upstream-first ABI constraints. Browser evidence is under
`third_party/typeduck-web/e2e/results/m20-showcase-controls/`.

- [x] **M20-DEMO-01**: TypeDuck-Web exposes only runtime-backed active controls:
  schema/deploy-time controls flow through `customize()` plus deploy, live
  session controls flow through `setOption()`, display-only controls are grouped
  separately, and no new `RimeApi`, `RimeCandidate`, or `yune_typeduck_*` export
  is added for UI convenience.
- [x] **M20-DEMO-02**: Prediction controls are honest and profile-aligned:
  `prediction_never_first` defaults on, and the UI exposes one prediction
  threshold control because the frequency/weight config aliases drive the same
  engine threshold; the fine-grained threshold UI has a real-assets-calibrated
  `santai` cutoff plus documented range bounds in browser evidence.
- [x] **M20-DEMO-03**: Static or default-on engine features are guided
  scenarios, not fake toggles: long-entry prediction (`santai` -> `身體健康`),
  Cantonese fuzzy/容錯, letter-to-tone, reverse lookup/dictionary panels, and
  AI second-pass behavior are demonstrable without misrepresenting their
  configurability.
- [x] **M20-DEMO-04**: The internal TypeDuck-Web harness and
  `@yune-ime/typeduck-runtime` subtrees have local `AGENTS.md` guidance covering
  patch discipline, runtime wrapper boundaries, browser evidence, the
  control-honesty rule, and the distinction between the harness, the runtime
  bridge, and the real TypeDuck-Web web IME product.
- [x] **M20-DEMO-05**: Real browser evidence includes an honesty gate proving
  supported controls with visible before/after output where the
  `jyut6ping3_mobile` browser surface can render it: AI candidates,
  `combine_candidates`, `prediction_never_first`, prediction threshold, live
  `setOption()` controls, display-language/Jyutping rendering, and guided
  scenarios. Deploy-time controls whose current browser panel effect is not
  independently visible keep real persisted `jyut6ping3_mobile.custom.yaml`
  assertions, but are not counted as candidate-output proof. Input Memory has a
  visible learned-prediction on-state and an explicit browser-surface N/A for
  memory-off suppression; Auto-correction now has real `nri` browser
  before/after evidence, with correction off rendering the v1.1.2 prefix
  fallback rows and correction on rendering `你` first. The full oracle row set
  and commit previews remain engine-proven by `cantonese_parity`.
  `ascii_punct` now has M18 engine behavior but remains absent as a working
  browser toggle until a browser-visible evidence slice proves it. The fixed
  `jyut6ping3_mobile` browser schema lacks a
  `cangjie` namespace, so Reverse code display / Cangjie / `show_full_code` are
  labeled current-surface N/A rather than fake working toggles.
- [x] **M20-DEMO-06**: The internal TypeDuck-Web harness is documented and
  maintained as Yune's canonical browser playground: every browser-safe
  supported engine feature is reachable through an active control or guided
  scenario, and unsupported or deferred behavior is clearly absent or labeled
  rather than partially exposed.
- [x] **M20-DEMO-07**: Headline TypeDuck profile toggles are not lost in the
  playground: `translator/combine_candidates` is an active control whose
  UI default is documented as an M20 grouped-candidate demo choice while the raw
  mobile browser assets still record `common:/separate_candidates`, and
  `show_full_code` is either exercised through a browser-reachable Cangjie
  side-lookup scenario/control or explicitly recorded as N/A for the current
  `jyut6ping3_mobile`-only surface.

## M22 Web Playground Requirements

- [x] **M22-PLAY-01**: The internal TypeDuck-Web playground has an opt-in
  read-only engine inspector showing segment tags, candidate source/quality/
  preedit/comment details, spelling-algebra expansion, filter audit, prediction
  score/threshold data, and AI staging state.
- [x] **M22-PLAY-02**: The inspector is off by default, preserves classic
  candidate response identity when disabled, has committed browser evidence, and
  does not change the default `RimeApi`, `RimeCandidate`, or ABI layout files.
- [x] **M22-PLAY-03**: Remaining browser-safe controls
  (`traditionalization`, `extended_charset`, `disabled`, `dictionary_exclude`,
  and any `ascii_punct` exposure after M18) are exposed only when they clear the
  M20 honesty gate with real browser before/after evidence; otherwise they are
  documented as browser-surface N/A. M22 exposes the first four controls with
  status/candidate/persisted-config evidence and keeps `ascii_punct` absent.
- [x] **M22-PLAY-04**: The playground loads `jyut6ping3_mobile`, `cangjie5`,
  and `luna_pinyin` through a real schema switcher with reverse lookup for the
  new schemas, using generated or provenance-stamped compiled artifacts with
  measured browser asset sizes.

## M24 TypeDuck-Web Dogfooding Requirements

**Status: complete.** M24 closed the first manual dogfooding/demo-hardening
batch for the internal TypeDuck-Web playground. The closed issue ledger and
evidence index live in
[`plans/archive/m24-plan-typeduck-web-dogfooding.md`](./plans/archive/m24-plan-typeduck-web-dogfooding.md),
with browser evidence under
`third_party/typeduck-web/e2e/results/m24-dogfooding/`.

- [x] **M24-DOGFOOD-REQ-01**: The dogfood browser harness records issue-scoped
  evidence under `m24-dogfooding/<issue-id>/`, and startup evidence includes the
  worker phase markers, `yune-typeduck.js`/`.wasm` asset identity, and loaded
  shared schema assets.
- [x] **M24-DOGFOOD-REQ-02**: Candidate rendering strips literal `\f`, `\r`,
  and `\v` controls from visible rows while preserving dictionary parsing, and
  compound candidates keep row text compact with details in the dictionary panel.
- [x] **M24-DOGFOOD-REQ-03**: Browser/runtime correctness fixes remain
  fixture-backed where engine output is involved: `jigaajiusihaa` ordering is
  locked to TypeDuck `v1.1.2`, page-size customization writes `menu/page_size`,
  and the historical M24 Jyutping reverse lookup path uses packaged browser
  assets; M25 updates the current web profile to bare `` `zhe `` for
  `luna_pinyin`.
- [x] **M24-DOGFOOD-REQ-04**: The TypeDuck-Web playground settings are
  Cantonese-first and grouped by engine/session/display/frontend purpose, with
  checklist display languages, real schema names, labeled engine status, and full
  Chinese typeface family names.
- [x] **M24-DOGFOOD-REQ-05**: The dogfood UI stack is Vite + React + Tailwind CSS
  plus small local components only; DaisyUI is removed from package metadata,
  Tailwind config, and local component class usage.

## M25 TypeDuck-Web Dogfooding Round 2 Requirements

**Status: complete.** M25 closed the second manual dogfooding round for the
internal TypeDuck-Web playground. The archived ledger and closeout evidence
index live in
[`plans/archive/m25-plan-typeduck-web-dogfooding-round-2.md`](./plans/archive/m25-plan-typeduck-web-dogfooding-round-2.md),
with browser evidence under
`third_party/typeduck-web/e2e/results/m25-dogfooding/`.

- [x] **M25-DOGFOOD-REQ-01**: Every closed M25 row has issue-scoped browser
  JSON/screenshot evidence, an owning Playwright or native test listed in the
  ledger closeout table, and TypeDuck-Web source changes regenerated into
  `third_party/typeduck-web/patches/yune-typeduck-runtime.patch` with
  reverse/forward patch checks.
- [x] **M25-DOGFOOD-REQ-02**: Browser startup uses the release-mode WASM build,
  records phase timing evidence, reuses fresh deploy state instead of forcing
  schema invalidation, and normal typing no longer shows the global loading
  state.
- [x] **M25-DOGFOOD-REQ-03**: Page size is an obvious 3-10 setting wired to
  `menu/page_size`; native and browser tests prove the candidate panel cap and
  page navigation at 3, 9, and 10 visible rows.
- [x] **M25-DOGFOOD-REQ-04**: The current Jyutping web profile uses bare
  `` ` `` for `luna_pinyin` reverse lookup, removes the vestigial bare
  `reverse_lookup` slot, keeps retained Loengfan/Cangjie secondary lookups on
  explicit non-bare triggers such as `` `vl`` / `` `vc``, and shows the trigger
  map in the web UI.
- [x] **M25-DOGFOOD-REQ-05**: The schema selector, Luna visible name, Cangjie
  version control, Display/Live settings order, and IME Settings alignment are
  browser-evidenced across desktop and narrow viewports.
- [x] **M25-DOGFOOD-REQ-06**: Binary dogfood controls use checkbox-style
  affordances, Candidate Menu Layout uses radio choices, and the UI stack
  remains Vite + React + Tailwind CSS plus small local components only.

**Follow-on (no requirement IDs):** [`M21`](./plans/archive/m21-plan-typeduck-web-product-comparison.md) is complete as a post-M20 *comparison protocol* and hard-oracle closeout. It compared the Yune harness against the deployed `typeduck.hk/web` product as a behavior/feel target, but the `v1.1.2` fixtures remained the hard oracle. The final gap ledger has no remaining hard-oracle action rows: M21-GAP-01 is closed by `jyut6ping3-m21-sentence-composition.json`, M21-GAP-02 is closed by `jyut6ping3-m21-prediction-ranking.json` plus real `nri` browser before/after evidence, and `jyut6ping3-m21-closeout.json` locks the remaining baseline/fuzzy/sentence/`hk2s`/tone-letter/paging rows including the final `m` and `mgoi` fixes.

## Out of Scope

Explicitly excluded from the current milestone.

| Feature | Reason |
|---------|--------|
| Full librime C++ plugin ABI compatibility | Expensive and not yet required by a concrete frontend or distribution migration path |
| Cloud inference as a required dependency | Classic input behavior must remain local-first and predictable |
| New GUI frontend | Native frontend integration should validate the ABI first; `yune-cli` is only a frontend surrogate |
| Behavior changes during mechanical refactors | Compatibility work needs measurable, reviewable behavior slices |
| 100% feature parity with librime internals | The oracle is a behavioral floor, not a feature target; a librime feature is implemented only when a named target schema/frontend needs it (see roadmap "Compatibility goal" and `decisions.md` D-25) |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
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
| UPSTREAM-ORACLE-01 | M12 | Complete - upstream `1.17.0` provenance pinned as default core oracle |
| UPSTREAM-ORACLE-02 | M12 | Complete - fixture naming separates `upstream-1.17.0` and `typeduck-v1.1.2` goldens |
| UPSTREAM-AUDIT-01 | M12 | Complete - coverage audit captured in `docs/plans/archive/m12-audit-coverage.md` |
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

**Coverage:**
- v1 requirements: 25 total
- v2 validation requirements: 7 total
- TypeDuck-Web integration requirements: 15 total
- TypeDuck-Windows native IME requirements: 7 total
- M12/M17/M18 upstream oracle and behavioral parity requirements: 10 total, 10 complete
- M13 AI-native frontend exposure requirements: 6 total, 6 complete
- M14–M16 TypeDuck-Web fork parity requirements: 7 total, 7 complete (M16 complete with explicit browser/userdb inspection limits)
- Fork parity backlog (Cantonese engine-parity, vs upstream 1.17.0): 9 total, 9 complete; see [`fork-parity-ledger.md`](./fork-parity-ledger.md)
- M20 web demo showcase controls requirements: 7 total, 7 complete
- M19 schema breadth and TypeDuck-profile ABI requirements: 5 total, 5 complete
- M22 web playground requirements: 4 total, 4 complete
- M24 TypeDuck-Web dogfooding requirements: 5 total, 5 complete
- M25 TypeDuck-Web dogfooding round 2 requirements: 6 total, 6 complete
- Mapped to phases: 120
- Unmapped: 0

---
*Requirements defined: 2026-04-28*
*Last updated: 2026-06-21 - M25 TypeDuck-Web dogfooding round 2 is complete with 10/10 rows closed, browser evidence, and regenerated TypeDuck-Web patch checks; M24 TypeDuck-Web dogfooding remains complete with 13/13 rows closed and browser evidence; M19 schema breadth and the named TypeDuck-profile ABI accessor are complete; M23 architecture hardening and M18 deployment/processor depth are complete; all M22 TypeDuck-Web playground buckets are complete with browser evidence; M21 TypeDuck-Web product comparison is complete as a hard-oracle closeout; M20 Web Demo Showcase Controls remain complete as a separate internal web/demo track; M10 TypeDuck-Windows is complete as a TypeDuck compatibility profile with T1/T2 package/profile smoke, build/link evidence, and stock T3 TypeDuckServer/TestTypeDuckIPC real-server IPC smoke*
