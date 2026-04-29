# Requirements: Yune

**Defined:** 2026-04-28
**Core Value:** Yune should preserve predictable classic RIME input while making AI/LLM assistance a first-class, local-first, non-blocking source of candidates, ranking, context, and memory.

## v1 Requirements

Requirements for the current compatibility milestone. Each requirement maps to
exactly one roadmap phase.

### CLI Frontend Surrogate

- [ ] **CLI-01**: Developer can initialize `yune-rime-api` from `yune-cli` with explicit shared data and user data directories.
- [ ] **CLI-02**: Developer can deploy and select schemas through the CLI using the RIME ABI path, not direct `yune-core` fixture setup.
- [ ] **CLI-03**: Developer can create and destroy RIME sessions from the CLI and process interactive key events through `RimeProcessKey`.
- [ ] **CLI-04**: Developer can render commit text, preedit, candidate page, highlight index, and status after each CLI key event.
- [ ] **CLI-05**: Developer can replay transcript key sequences through the RIME ABI and compare the transcript against expected output.

### Frontend ABI Validation

- [ ] **ABI-01**: Developer can run the current ABI against at least one real frontend client or native frontend-like loading path and record observed gaps.
- [ ] **ABI-02**: Struct layout, lifetime, notification, deployment, and session gaps found by frontend validation have focused regression coverage.
- [ ] **ABI-03**: Runtime resource IDs from C APIs and schema YAML reject path traversal, absolute paths, platform separators, and other non-logical IDs before filesystem joins.
- [ ] **ABI-04**: Process-wide session, module, notification, switcher, and runtime state behavior remains deterministic under repeated initialize/finalize and session lifecycle operations.

### Schema Pipeline Depth

- [x] **SCHEMA-01**: `speller` behavior covers previous-match segment splitting and non-auto-commit composition behavior beyond current focused auto-commit paths.
- [x] **SCHEMA-02**: `editor`, `navigator`, and `selector` behavior covers deeper segment/selection span semantics and navigator fallback interactions beyond current focused overrides.
- [x] **SCHEMA-03**: `chord_composer`, `shape_processor`/`shape_formatter`, `punct_segmentor`, and `fallback_segmentor` behavior covers larger-chain and remaining lifecycle edge cases.
- [ ] **SCHEMA-04**: Remaining librime gear behavior around `memory`, `poet`/`grammar`, `contextual_translation`, and `unity_table_encoder` has explicit compatibility increments or documented deferrals.
- [ ] **SCHEMA-05**: Full spelling algebra, correction/tolerance search interaction, OpenCC conversion data, and distribution-scale processor/segmentor/translator/filter chains are compared directly against librime behavior.

### Dictionary And Compiled Data

- [ ] **DATA-01**: Runtime dictionary loading can consume compiled `.table.bin`, `.prism.bin`, and `.reverse.bin` payloads beyond the current metadata slice.
- [ ] **DATA-02**: Dictionary rebuild execution handles source-vs-prebuilt fallback, table/prism/reverse checksum decisions, pack checksum chaining, and compiled output freshness.
- [ ] **DATA-03**: Stem-column data, reverse-db `dict_settings`, preset-vocabulary phrase injection, and UniTE-style encoder payloads are consumed where librime schemas rely on them.
- [ ] **DATA-04**: Correction data and tolerance search inputs are represented in the compiled-data path sufficiently for schema-loaded lookup compatibility.

### User Dictionary Compatibility

- [ ] **USERDB-01**: User dictionary storage supports librime-compatible LevelDB/userdb behavior or a documented compatible abstraction beyond the current plain text shim.
- [ ] **USERDB-02**: Snapshot backup, restore, recovery, sync, and transaction rollback behavior match librime-observable semantics.
- [ ] **USERDB-03**: Learning, frequency updates, predictive lookup, and backdated scan behavior are represented in runtime candidate ranking and userdb persistence.

### Engineering Structure And Quality

- [ ] **QUAL-01**: Every new compatibility slice starts with an owning implementation module, owning test module, and explicit librime comparison target.
- [ ] **QUAL-02**: `lib.rs` and `main.rs` remain facades/orchestration glue; temporary spike code is extracted before a second related behavior lands.
- [ ] **QUAL-03**: Remaining oversized compatibility tests are split only along behavior ownership boundaries, without mixing mechanical moves and behavior changes.
- [ ] **QUAL-04**: Quality gates for implementation phases include focused tests, `cargo fmt`, relevant `cargo test` targets, and workspace tests when shared behavior changes.

## v2 Requirements

Deferred to future releases. Tracked but not in the current roadmap.

### Plugin Compatibility

- **PLUGIN-01**: Yune can load or adapt librime C++ plugin ABI extensions.
- **PLUGIN-02**: Lua, octagram, predict, proto, and other distribution plugin ecosystems have migration paths.

### Product Frontend

- **FRONTEND-01**: Yune ships a new graphical end-user frontend.
- **FRONTEND-02**: Yune-specific UI features expose optional AI ranking and contextual completion controls.

### AI Extension Layer

- **AI-01**: Engine exposes an `AiCandidateProvider` or equivalent interface that can provide candidates without replacing classic translators.
- **AI-02**: Candidate ranking supports local model and rule-backed implementations with deterministic timeout/fallback behavior.
- **AI-03**: Contextual phrase and sentence completion can produce source-labeled AI candidates without allowing AI candidates to auto-commit by default.
- **AI-04**: Context providers define what app, field, preceding text, cursor, schema, and candidate-list data may be shared with AI providers.
- **AI-05**: Memory store records user vocabulary, phrase preferences, and domain terms through explicit, inspectable, clearable policy.
- **AI-06**: Privacy policy disables learning and remote calls for sensitive contexts and keeps classic input fully functional when AI is disabled.
- **AI-07**: CLI frontend surrogate can demonstrate AI candidate/ranking behavior with mock and local providers before native frontends expose it.

## Out of Scope

Explicitly excluded from the current milestone.

| Feature | Reason |
|---------|--------|
| Full librime C++ plugin ABI compatibility | Expensive and not yet required by a concrete frontend or distribution migration path |
| Cloud inference as a required dependency | Classic input behavior must remain local-first and predictable |
| New GUI frontend | Native frontend integration should validate the ABI first; `yune-cli` is only a frontend surrogate |
| Behavior changes during mechanical refactors | Compatibility work needs measurable, reviewable behavior slices |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| CLI-01 | Phase 1 | Pending |
| CLI-02 | Phase 1 | Pending |
| CLI-03 | Phase 1 | Pending |
| CLI-04 | Phase 1 | Pending |
| CLI-05 | Phase 1 | Pending |
| ABI-01 | Phase 2 | Pending |
| ABI-02 | Phase 2 | Pending |
| ABI-03 | Phase 2 | Pending |
| ABI-04 | Phase 2 | Pending |
| SCHEMA-01 | Phase 3 | Pending |
| SCHEMA-02 | Phase 3 | Pending |
| SCHEMA-03 | Phase 3 | Pending |
| SCHEMA-04 | Phase 3 | Pending |
| SCHEMA-05 | Phase 3 | Pending |
| DATA-01 | Phase 4 | Pending |
| DATA-02 | Phase 4 | Pending |
| DATA-03 | Phase 4 | Pending |
| DATA-04 | Phase 4 | Pending |
| USERDB-01 | Phase 5 | Pending |
| USERDB-02 | Phase 5 | Pending |
| USERDB-03 | Phase 5 | Pending |
| QUAL-01 | Phase 1 | Pending |
| QUAL-02 | Phase 1 | Pending |
| QUAL-03 | Phase 5 | Pending |
| QUAL-04 | Phase 5 | Pending |

**Coverage:**
- v1 requirements: 25 total
- Mapped to phases: 25
- Unmapped: 0

---
*Requirements defined: 2026-04-28*
*Last updated: 2026-04-28 after roadmap traceability mapping*
