# Roadmap: Yune

## Overview

This milestone turns Yune's focused compatibility surface into a stronger
frontend and data-compatibility validation track. It starts with a RIME
API-backed CLI frontend surrogate, uses that to harden ABI behavior against real
frontend expectations, then deepens schema, compiled dictionary, and user
dictionary compatibility while preserving the module boundaries created by the
recent refactor. AI-native input is the product direction after this foundation:
it should be planned as a separate layer of providers, rankers, context policy,
memory policy, and privacy controls rather than mixed into librime compatibility
work.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: CLI Frontend Surrogate** - Drive `yune-rime-api` from `yune-cli` and lock in structure rules for future slices.
- [x] **Phase 2: Native ABI Validation And Runtime Safety** - Exercise real frontend-like loading paths and harden ABI/resource boundaries.
- [x] **Phase 3: Schema Pipeline Depth** - Expand focused schema behavior toward deeper librime gear semantics.
- [x] **Phase 4: Compiled Dictionary Data** - Move from source dictionary parsing and metadata checks toward compiled payload consumption and rebuild execution.
- [x] **Phase 5: UserDB And Scaling Hardening** - Extend user dictionary compatibility and finish quality/test ownership cleanup for the milestone.
- [x] **Phase 6: Real Frontend Validation And Benchmark** - Exercise the compatibility foundation through real frontend lifecycle hosts and establish frontend-sensitive performance baselines before AI-native work.

## Phase Details

### Phase 1: CLI Frontend Surrogate
**Goal**: Developers can use `yune-cli` as a scriptable frontend surrogate that exercises `yune-rime-api` setup, schema selection, key processing, and transcript replay.
**Depends on**: Nothing (first phase)
**Requirements**: CLI-01, CLI-02, CLI-03, CLI-04, CLI-05, QUAL-01, QUAL-02
**Success Criteria** (what must be TRUE):
  1. Developer can initialize the RIME service from `yune-cli` with explicit shared/user data paths.
  2. Developer can deploy, select a schema, create a session, process keys, and destroy the session through ABI calls.
  3. Developer can inspect commit text, preedit, candidates, highlight index, and status after each CLI key event.
  4. Developer can replay a key transcript through the ABI and compare deterministic output.
  5. Every new behavior added in this phase lives in an owned module with matching focused tests, not in `main.rs` or `lib.rs`.
**Plans**: 3 plans

Plans:
- [x] 01-01: Implement RIME API service setup, schema deployment/selection, and session lifecycle in `crates/yune-cli/src/rime_frontend.rs`.
- [x] 01-02: Add interactive rendering and transcript replay output through `crates/yune-cli/src/render.rs` and `crates/yune-cli/src/transcript.rs`.
- [x] 01-03: Add focused CLI/ABI tests and document the module/test ownership rule for future compatibility slices.

### Phase 2: Native ABI Validation And Runtime Safety
**Goal**: The ABI surface is validated against at least one real frontend or native frontend-like loader, and runtime safety gaps discovered there are converted into tests and fixes.
**Depends on**: Phase 1
**Requirements**: ABI-01, ABI-02, ABI-03, ABI-04
**Success Criteria** (what must be TRUE):
  1. Developer can run a real frontend client or native frontend-like loader against the current ABI and capture failures as reproducible notes or fixtures.
  2. Struct layout, lifetime, notification, deployment, and session lifecycle gaps found during validation have focused regression coverage.
  3. Resource IDs from C APIs and schema YAML are rejected when they contain path traversal, absolute paths, separators, or other filesystem syntax.
  4. Repeated initialize/finalize, module, notification, switcher, and session lifecycle paths remain deterministic under the validation suite.
**Plans**: 3 plans

Plans:
- [x] 02-01: Build or run a native frontend validation harness and record observed ABI/frontend gaps.
- [x] 02-02: Fix and test lifecycle, notification, deployment, and session behavior exposed by native validation.
- [x] 02-03: Add logical resource-ID validation for config, dictionary, custom-settings, and userdb paths.

### Phase 3: Schema Pipeline Depth
**Goal**: Schema-loaded behavior covers deeper librime semantics across the processor, segmentor, translator, filter, and gear components that remain outside the current focused subset.
**Depends on**: Phase 2
**Requirements**: SCHEMA-01, SCHEMA-02, SCHEMA-03, SCHEMA-04, SCHEMA-05
**Success Criteria** (what must be TRUE):
  1. `speller` previous-match segment splitting and non-auto-commit composition behavior are covered by ABI-facing tests.
  2. `editor`, `navigator`, and `selector` segment/selection span behavior works across deeper candidate and segment interactions.
  3. `chord_composer`, shape, punctuation, and fallback segmentor behavior is tested in larger processing chains, not only isolated focused paths.
  4. `memory`, `poet`/`grammar`, `contextual_translation`, and `unity_table_encoder` each have either a compatibility increment or an explicit documented deferral.
  5. Larger distribution schema chains produce documented comparisons against librime for spelling algebra, OpenCC, and correction/tolerance behavior.
**Plans**: 4 plans

Plans:
- [x] 03-01: Expand speller, editor, navigator, selector, chord, shape, punctuation, and fallback processor coverage.
- [x] 03-02: Add compatibility decisions or increments for remaining librime gear components.
- [x] 03-03: Compare larger distribution schema chains against librime and convert differences into focused fixtures.
- [x] 03-04: Broaden spelling algebra, correction/tolerance, and OpenCC behavior where current focused coverage is insufficient.

### Phase 4: Compiled Dictionary Data
**Goal**: Dictionary loading and rebuild behavior move beyond source parsing and metadata checks toward compiled librime data compatibility.
**Depends on**: Phase 3
**Requirements**: DATA-01, DATA-02, DATA-03, DATA-04
**Success Criteria** (what must be TRUE):
  1. Runtime dictionary loading can consume compiled `.table.bin`, `.prism.bin`, and `.reverse.bin` payloads beyond checksum metadata.
  2. Rebuild execution handles source-vs-prebuilt fallback, table/prism/reverse freshness, and pack checksum chaining.
  3. Stem-column data, reverse-db `dict_settings`, preset vocabulary injection, and UniTE-style encoder payloads are consumed where schemas rely on them.
  4. Correction data and tolerance search inputs are represented in the compiled-data path and covered by schema-loaded lookup tests.
**Plans**: 4 plans

Plans:
- [x] 04-01: Implement compiled table/prism/reverse payload readers and runtime fallback from source dictionaries.
- [x] 04-02: Implement rebuild execution and pack checksum chaining around the existing rebuild-plan primitive.
- [x] 04-03: Consume stem, `dict_settings`, preset vocabulary, and UniTE encoder payloads in reverse/encoder paths.
- [x] 04-04: Represent correction/tolerance data in compiled lookup and validate behavior against librime.

### Phase 5: UserDB And Scaling Hardening
**Goal**: User dictionary behavior and remaining quality concerns are strong enough for longer-running frontend-style sessions and future milestone planning.
**Depends on**: Phase 4
**Requirements**: USERDB-01, USERDB-02, USERDB-03, QUAL-03, QUAL-04
**Success Criteria** (what must be TRUE):
  1. User dictionary storage has a librime-compatible LevelDB/userdb path or a documented compatible abstraction beyond the plain text shim.
  2. Snapshot backup, restore, recovery, sync, and transaction rollback behavior match librime-observable semantics.
  3. Learning, frequency updates, predictive lookup, and backdated scan behavior are represented in runtime candidate ranking and persistence.
  4. Remaining oversized compatibility tests are split along ownership boundaries where that reduces future risk.
  5. Implementation phases close with focused tests, formatting, relevant package tests, and workspace tests when shared behavior changes.
**Plans**: 4 plans

Plans:
- [x] 05-01: Add userdb storage, snapshot, recovery, sync, and rollback compatibility beyond plain text shims.
- [x] 05-02: Add learning, frequency update, predictive lookup, and backdated scan behavior to runtime candidate/userdb flow.
- [x] 05-03: Split remaining core oversized tests where useful by behavior ownership.
- [x] 05-04: Split remaining API/frontend tests where useful and codify final Phase 05 quality gates.

### Phase 6: Real Frontend Validation And Benchmark
**Goal**: Yune's RIME ABI is exercised by real frontend lifecycle hosts, TypeDuck-Web-style browser/WebAssembly integration, or host-shaped validation harnesses, and frontend-sensitive performance baselines are recorded before AI-native work begins.
**Depends on**: Phase 5
**Requirements**: FRONTEND-VALIDATION-01, FRONTEND-VALIDATION-02, FRONTEND-VALIDATION-03, FRONTEND-VALIDATION-04, FRONTEND-VALIDATION-05, BENCH-01, BENCH-02
**Success Criteria** (what must be TRUE):
  1. A host-shaped native loader or real frontend integration validates `rime_get_api`, setup, initialize, deploy, schema selection, session lifecycle, key processing, context/status reads, commits, and teardown.
  2. TypeDuck-Web-style browser/WebAssembly validation is attempted as the first real application frontend path and its browser-specific limits are documented.
  3. At least one macOS Squirrel-shaped validation path is attempted or documented with reproducible blockers before Linux frontend validation is expanded.
  4. Any frontend-observed ABI/runtime mismatch is captured as notes, fixtures, or focused regression tests.
  5. Benchmarks record baseline latency for session lifecycle, per-key processing, schema deployment/dictionary loading, and userdb learning/sync paths.
  6. The phase ends with a go/no-go recommendation for starting AI-native candidate/ranking design.
**Plans**: 4 plans

Plans:
- [x] 06-01: Build the host-shaped native frontend validation harness and capture lifecycle call traces.
- [x] 06-02: Validate the TypeDuck-Web browser/WebAssembly integration path and capture frontend wrapper gaps.
- [x] 06-03: Attempt Squirrel/macOS native frontend validation and convert observed gaps into reproducible fixtures.
- [x] 06-04: Add frontend-sensitive benchmark baselines and write the AI-native readiness recommendation.

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 -> 4 -> 5 -> 6

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. CLI Frontend Surrogate | 3/3 | Complete | 2026-04-29 |
| 2. Native ABI Validation And Runtime Safety | 3/3 | Complete | 2026-04-29 |
| 3. Schema Pipeline Depth | 4/4 | Complete | 2026-04-29 |
| 4. Compiled Dictionary Data | 4/4 | Complete | 2026-04-29 |
| 5. UserDB And Scaling Hardening | 4/4 | Complete | 2026-04-30 |
| 6. Real Frontend Validation And Benchmark | 4/4 | Complete | 2026-05-01 |
| 7. WASM Build And Export Contract | 3/3 | Complete | 2026-05-05 |
| 8. TypeScript Bridge And Runtime Package | 3/3 | Complete | 2026-05-05 |
| 9. Browser Filesystem And Persistence | 3/3 | Complete | 2026-05-05 |

## Next Milestone: TypeDuck-Web Browser Integration

Phase 6 proved that TypeDuck-Web-style lifecycle expectations can be modeled at
the RIME ABI boundary. The next milestone turns that validation into a practical
browser integration path: keep the Rust adapter stable, make the WASM/export
contract reproducible, add the TypeScript bridge, and prove the flow inside a
browser-like host before AI-native work depends on frontend plumbing.

### Seed Work: Yune TypeDuck Adapter

**Status**: Implemented before milestone planning.

Completed seed work:
- `crates/yune-rime-api/src/typeduck_web.rs` exports the `yune_typeduck_*` C/WASM bridge.
- `crates/yune-rime-api/tests/typeduck_web.rs` covers native adapter lifecycle, JSON responses, candidate actions, deploy/customize, null handling, and response freeing.
- `docs/typeduck-web-adapter.md` documents the browser filesystem contract and JS call shape.

### Phase 7: WASM Build And Export Contract

**Goal**: The TypeDuck adapter can be built for the browser target with a stable, documented symbol/export contract.
**Depends on**: Phase 6 + TypeDuck adapter seed work
**Requirements**: TYPEDUCK-WASM-01, TYPEDUCK-WASM-02, TYPEDUCK-WASM-03
**Success Criteria** (what must be TRUE):
  1. Developer can run a documented build command for the intended Emscripten/WASM target or get a reproducible local-toolchain blocker.
  2. The build contract preserves all `yune_typeduck_*` symbols needed by JS callers.
  3. The adapter's native contract tests remain the fallback validation path when Emscripten is unavailable locally.
  4. Documentation states required linker/export flags, runtime file layout, and known host assumptions.
**Plans**: 3 plans

Plans:
- [x] 07-01: Define the Emscripten/WASM build target, export list, and local-toolchain detection path.
- [x] 07-02: Add build-script or documented command coverage that verifies adapter symbol availability.
- [x] 07-03: Extend adapter contract tests/docs for browser target constraints and fallback blockers.

### Phase 8: TypeScript Bridge And Runtime Package

**Goal**: Browser code can call the Yune TypeDuck adapter through a typed JS/TypeScript wrapper with correct memory ownership.
**Depends on**: Phase 7
**Requirements**: TYPEDUCK-JS-01, TYPEDUCK-JS-02, TYPEDUCK-JS-03, TYPEDUCK-JS-04
**Success Criteria** (what must be TRUE):
  1. A TypeScript wrapper exposes init, process-key, candidate action, deploy, customize, and cleanup operations.
  2. JSON response parsing and `yune_typeduck_free_response` pairing are enforced in one wrapper path.
  3. Keycode/mask mapping is explicit and covered by deterministic tests.
  4. The wrapper makes process-global runtime limitations visible to callers.
**Plans**: 3 plans

Plans:
- [ ] 08-01: Add TypeScript types and wrapper functions around the `yune_typeduck_*` symbols.
- [ ] 08-02: Add wrapper-level tests for response parsing/freeing, error/null handling, and key mapping.
- [ ] 08-03: Document runtime lifecycle and one-active-service constraints for TypeDuck-Web callers.

### Phase 9: Browser Filesystem And Persistence

**Goal**: TypeDuck-Web-style browser storage can provide Yune with shared data, user data, deployed configs, customization patches, and userdb persistence.
**Depends on**: Phase 8
**Requirements**: TYPEDUCK-FS-01, TYPEDUCK-FS-02, TYPEDUCK-FS-03, TYPEDUCK-FS-04
**Success Criteria** (what must be TRUE):
  1. Browser host setup creates the expected `shared_data_dir`, `user_data_dir`, and `user_data_dir/build` layout.
  2. Schema and dictionary assets can be preloaded before adapter init.
  3. IDBFS or equivalent persistence sync happens before init and after deploy/customize/userdb mutations.
  4. Failure and recovery paths are documented for missing assets, failed sync, and stale deployed configs.
**Plans**: 3 plans

Plans:
- [x] 09-01-PLAN.md — Filesystem layout and explicit asset preload helpers.
- [x] 09-02-PLAN.md — Persistence sync wrappers around init and runtime mutations.
- [x] 09-03-PLAN.md — Failure-mode tests, recovery docs, and final scope gates.

### Phase 10: TypeDuck-Web App Integration And E2E

**Goal**: The upstream TypeDuck-Web application can be cloned, pointed at Yune instead of its librime/WASM core, and exercised through real browser flows.
**Depends on**: Phase 9
**Requirements**: TYPEDUCK-E2E-01, TYPEDUCK-E2E-02, TYPEDUCK-E2E-03, TYPEDUCK-E2E-04
**Success Criteria** (what must be TRUE):
  1. The TypeDuck-Web repository is cloned or vendored in a reproducible test location, and its current librime/WASM bridge is identified.
  2. The TypeDuck-Web input-engine binding is patched or configured to call the Yune TypeScript bridge instead of the original librime bridge.
  3. Real TypeDuck-Web browser validation covers composition, candidate paging, selection, deletion, commit output, deploy, customize, and persistence smoke flows.
  4. Any blocker from TypeDuck-Web source structure, build system, worker isolation, browser APIs, or Yune adapter mismatch is recorded reproducibly.
  5. The milestone ends with a go/no-go recommendation for starting AI-native frontend exposure.
**Plans**: 4 plans

Plans:
- [ ] 10-01: Clone TypeDuck-Web, inspect its librime/WASM bridge, and document the replacement seam for Yune.
- [ ] 10-02: Patch or configure TypeDuck-Web so its input-engine binding calls the Yune TypeScript bridge.
- [ ] 10-03: Add real TypeDuck-Web browser E2E coverage for composition, candidate actions, deploy/customize, and persistence smoke flows.
- [ ] 10-04: Write TypeDuck-Web integration findings and the AI-native frontend exposure recommendation.

## Future Milestone: AI-Native Input Layer

This milestone is intentionally not folded into the compatibility or TypeDuck-Web
integration milestones. Compatibility keeps classic input measurable and stable;
TypeDuck-Web integration proves frontend plumbing; the AI-native milestone
defines behavior that librime cannot serve as an oracle for.

### Candidate Provider Architecture

**Goal**: AI can provide candidates without replacing classic translators.

Expected requirements:
- `AiCandidateProvider` or equivalent provider interface receives bounded input
  context and returns source-labeled candidates.
- AI candidates use explicit source metadata and confidence/latency metadata.
- Classic candidates remain available when AI is disabled, pending, or failed.
- AI candidates do not auto-commit by default.

### Non-Blocking Ranking And Merge Policy

**Goal**: AI can rerank or merge candidates without adding typing latency.

Expected requirements:
- Ranking has a strict timeout budget and deterministic fallback.
- Late AI results are safe to discard or apply only at stable UI boundaries.
- Merge policy defines ordering between table, completion, sentence, userdb, and
  AI candidates.
- Tests use mock providers so behavior remains deterministic.

### Context And Privacy Policy

**Goal**: Yune can use context without turning the input method into an
uncontrolled data exfiltration path.

Expected requirements:
- Context providers classify app, field, preceding text, cursor state, schema,
  and candidate-list data by sensitivity.
- Sensitive contexts disable learning and remote calls.
- Users can inspect, clear, and disable memory.
- Remote LLM calls are optional enhancements; baseline AI-native behavior should
  work with local/mock providers.

### Memory And Personalization

**Goal**: Yune can learn useful language preferences while keeping user control.

Expected requirements:
- Memory store captures user vocabulary, phrase preferences, domain terms,
  code/project names, and style preferences through explicit policy.
- Memory updates are separated from librime-compatible userdb behavior until the
  interaction contract is clear.
- Personalization can influence ranking and completion without corrupting
  classic dictionary/userdb compatibility.

### CLI Playground Before Native Exposure

**Goal**: AI-native behavior is observable in the CLI frontend surrogate before
native frontends depend on it.

Expected requirements:
- CLI can enable mock/local AI providers per run.
- Transcript output records AI source, timeout/fallback decisions, and merge
  results.
- Native frontends keep AI disabled by default until the CLI behavior is stable.
