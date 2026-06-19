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
rather than the primary capture source. The expanded closeout keeps unsupported
language-model and processor-level behavior visible as ignored tests with
fixture-backed blocker reasons.

- [x] **UPSTREAM-ORACLE-01**: Upstream `rime/librime 1.17.0` and commit `33e78140250125871856cdc5b42ddc6a5fcd3cd4` are pinned as the default core oracle in docs and fixture provenance.
- [x] **UPSTREAM-ORACLE-02**: Oracle fixture/golden naming distinguishes upstream core fixtures from TypeDuck profile fixtures, e.g. `upstream-1.17.0/` vs `typeduck-v1.1.2/`.
- [x] **UPSTREAM-AUDIT-01**: Existing compatibility coverage is audited for TypeDuck-only assumptions that should not define core Yune behavior.
- [x] **TYPEDUCK-PROFILE-01**: TypeDuck-specific ABI, comment, and Cantonese/Jyutping behavior remains documented as profile-only and parked until explicitly resumed.
- [x] **UPSTREAM-BEHAVIOR-01**: Upstream `luna_pinyin` behavioral fixtures are captured from the official `1.17.0` release binary for curated mechanics, full `ni` selection, action/paging/commit, reverse lookup, punctuation/symbols, and option toggles.
- [x] **UPSTREAM-BEHAVIOR-02**: Full-dictionary `ni` selection uses every exact-code `luna_pinyin.dict.yaml` row plus relevant `essay.txt` rows for in-scope candidates, with provenance checks preventing default/zero essay-weight ranking.
- [x] **UPSTREAM-BEHAVIOR-03**: Menu-dependent behavior is compared through Yune's real `Engine` path for paging, numeric selection, space commit, reverse lookup, punctuation, and supported option behavior.
- [x] **UPSTREAM-BEHAVIOR-04**: Unsupported upstream behavior remains explicit: `zhongguo` phrase/language-model parity, `ascii_punct` processor bypass, and punctuation immediate commit are ignored tests with panic bodies and blocker reasons.
- [x] **UPSTREAM-BEHAVIOR-05**: `oracle_fixture_provenance` enforces non-circular fixture metadata, source-row policies, schema repository commits, capture commands, and absence of local absolute oracle-cache paths across all upstream `luna_pinyin` fixtures.

## TypeDuck-Windows Native IME Contract Requirements

**Status: parked as a TypeDuck compatibility profile.** A first pass landed
(Phases 11-16), M9 web validation is complete, and the native Windows package
smoke verified before M12. That smoke is now archived profile evidence, not an
active package gate against the default upstream `rime_get_api()`. The shared
comment requirement is covered for the current v1.1.2 oracle slices; remaining
Cantonese parity captures and a real TypeDuck-Windows frontend E2E stay explicit
blockers. These requirements target the native TypeDuck-Windows/weasel path and
no longer define Yune's active core oracle milestone.

- [x] **WIN-TEST-01**: Windows `cargo test --workspace` has a trustworthy green baseline, including portable signature timestamp shape and test-only poison-lock recovery.
- [x] **WIN-ABI-01**: `config_list_append_{string,bool,int,double}` helper behavior is implemented and covered as parked TypeDuck-profile implementation code; the default upstream `rime_get_api()` no longer exposes these fork-only slots.
- [x] **WIN-ORACLE-01**: The TypeDuck-HK/librime v1.1.2 binary and pinned schema are captured as a reproducible oracle, or a precise blocker is documented.
- [x] **WIN-COMMENT-01**: Candidate comment semantics match the v1.1.2 oracle for dictionary lookup payloads, reverse lookup joins, and prompt/schema identity. Dictionary lookup payload bytes, schema-prompt bytes, and reverse-lookup joiner coverage are oracle-backed.
- [x] **WIN-BUILD-01**: Yune had a pre-M12 native Windows `rime.dll`, import `.lib`, and compatible header package smoke; current packaging is parked and fails fast until a named TypeDuck profile ABI surface exists.
- [ ] **WIN-PARITY-01**: Cantonese/Jyutping parity regression coverage locks captured v1.1.2 behavior and records explicit ignored blockers for uncaptured fork-only cases. Full parity remains blocked by missing goldens.

## Future Requirements

Deferred beyond the TypeDuck-Web browser integration milestone. Tracked but not in the current roadmap.

### Plugin Compatibility

- **PLUGIN-01**: Yune can load or adapt librime C++ plugin ABI extensions.
- **PLUGIN-02**: Lua, octagram, predict, proto, and other distribution plugin ecosystems have migration paths.

### Product Frontend

- **FRONTEND-01**: Yune ships a new graphical end-user frontend.
- **FRONTEND-02**: Yune-specific UI features expose optional AI ranking and contextual completion controls.

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
| WIN-ABI-01 | Phase 12 | Complete - direct helper coverage retained as parked TypeDuck-profile implementation; not exposed by default upstream `rime_get_api()` |
| WIN-ORACLE-01 | Phase 13 | Complete |
| WIN-COMMENT-01 | Phase 14 / 17 | Complete - dictionary payload, schema prompt, and joiner oracle covered |
| WIN-BUILD-01 | Phase 15 | Complete - archived pre-M12 package smoke; current script is parked until a named TypeDuck profile ABI exists |
| WIN-PARITY-01 | Phase 16 | Partial - ignored oracle cases still blocked |
| UPSTREAM-ORACLE-01 | M12 | Complete - upstream `1.17.0` provenance pinned as default core oracle |
| UPSTREAM-ORACLE-02 | M12 | Complete - fixture naming separates `upstream-1.17.0` and `typeduck-v1.1.2` goldens |
| UPSTREAM-AUDIT-01 | M12 | Complete - coverage audit captured in `docs/plans/archive/m12-coverage-audit.md` |
| TYPEDUCK-PROFILE-01 | M12 | Complete - TypeDuck-specific coverage remains profile-only and parked until explicitly resumed |
| UPSTREAM-BEHAVIOR-01 | M12 | Complete - six official-binary `luna_pinyin` fixture files are checked in under `upstream-1.17.0` |
| UPSTREAM-BEHAVIOR-02 | M12 | Complete - full `ni` selection fixture includes all exact dictionary rows and candidate essay rows |
| UPSTREAM-BEHAVIOR-03 | M12 | Complete - active parity tests drive real parser/dictionary/translator/filter/Engine paths |
| UPSTREAM-BEHAVIOR-04 | M12 | Complete - unsupported phrase/language-model and processor-only gaps are explicit ignored blockers |
| UPSTREAM-BEHAVIOR-05 | M12 | Complete - provenance test scans all upstream `luna_pinyin` fixtures and source policies |
| AI-01 | M11 S1 | Complete - staged provider interface in `yune-core` |
| AI-02 | M11 S2/S5 | Complete - worker/fallback/confidence merge plus local rule-backed provider |
| AI-03 | M11 S1/S5 | Complete - source-labeled contextual/local completions with no default AI auto-commit |
| AI-04 | M11 S3 | Complete - context snapshot provider covers app, field, preceding text, cursor, schema, and candidate count |
| AI-05 | M11 S4 | Complete - AI memory store records explicit AI selections, is inspectable/clearable/disable-able, and uses `.ai-memory` namespace helpers |
| AI-06 | M11 S3/S4 | Complete - default-sensitive privacy blocks remote calls and suppresses AI memory writes while classic input remains available |
| AI-07 | M11 S1/S5 | Complete - direct CLI demonstrates `--ai-provider mock` and `--ai-provider local` |

**Coverage:**
- v1 requirements: 25 total
- v2 validation requirements: 7 total
- TypeDuck-Web integration requirements: 15 total
- TypeDuck-Windows native IME requirements: 6 total
- M12 upstream oracle and behavioral parity requirements: 9 total, 9 complete
- Mapped to phases: 69
- Unmapped: 0

---
*Requirements defined: 2026-04-28*
*Last updated: 2026-06-19 - M12 upstream behavioral parity closeout complete; M10 TypeDuck-Windows remains parked as a TypeDuck compatibility profile until a named profile ABI surface exists*
