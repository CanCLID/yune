---
gsd_state_version: 1.0
milestone: typeduck-windows-native-contract
milestone_name: TypeDuck-Windows Native IME Contract
status: execution
stopped_at: Ready to execute 11-01 Windows test baseline
last_updated: "2026-06-18T00:00:00.000Z"
last_activity: 2026-06-18
progress:
  total_phases: 16
  completed_phases: 10
  total_plans: 37
  completed_plans: 32
  percent: 86
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-04-28)

**Core value:** Existing RIME schemas and frontends should behave predictably through Yune's Rust implementation, with every compatibility difference measurable against librime before it is accepted.
**Current focus:** Phase 11 — Windows test baseline for the TypeDuck-Windows native IME contract

## Current Position

Phase: 11
Plan: 11-01 — Windows test baseline
Next phase: 12 — Fork Config List Append ABI
Status: Ready to execute
Last activity: 2026-06-18

Progress: [█████████░] 86%

## Performance Metrics

**Velocity:**

- Total plans completed: 24
- Planned Phase 06 plans: 4
- Average duration: -
- Total execution time: -

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01 | 3/3 | - | - |
| 02 | 3/3 | - | - |
| 03 | 4 | - | - |
| 04 | 4/4 | - | - |
| 05 | 4/4 | - | - |
| 06 | 0/4 | - | - |
| 07 | 3 | - | - |
| 09 | 3 | - | - |

**Recent Trend:**

- Last 5 plans: 02-01, 02-03, 02-02
- Trend: Phase 2 completed after verification gap closure

*Updated after each phase completion*
| Phase 03-schema-pipeline-depth P01 | 2h 5m | 3 tasks | 2 files |
| Phase 04-compiled-dictionary-data P03 | 7min | 3 tasks | 8 files |
| Phase 04-compiled-dictionary-data P04 | 16min | 3 tasks | 8 files |
| Phase 06 P03 | 27min | 3 tasks | 6 files |
| Phase 06-real-frontend-validation-and-benchmark P04 | 12min | 3 tasks | 7 files |
| Phase 07-wasm-build-and-export-contract P01 | 3min | 2 tasks | 2 files |
| Phase 07-wasm-build-and-export-contract P03 | 4min | 3 tasks | 6 files |
| Phase 09-browser-filesystem-and-persistence P03 | 5min | 3 tasks | 3 files |
| Phase 10 P04 | 8m 47s | 3 tasks | 2 files |
| Phase 11 P01 | - | pending | pending |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- Initialization: Existing `docs/analysis.md`, `docs/roadmap.md`,
  `docs/refactor-plan.md`, and `.planning/codebase/` are the source context for
  this GSD project.

- Initialization: External research was skipped for setup because current scope
  is driven by existing project docs and direct librime comparison.

- Initialization: Future compatibility slices must choose module ownership,
  test ownership, and librime comparison target before implementation.

- Phase 2: Native ABI validation uses a Cargo integration-test dynamic loader,
  resolves `rime_get_api`, and exercises the returned `RimeApi` function table
  against the real Cargo-built `yune-rime-api` cdylib.

- Phase 2: Runtime safety fixes are limited to ABI/runtime boundaries; schema
  semantics, compiled dictionary behavior, and userdb storage compatibility stay
  deferred to later phases.

- 03-01: Non-auto previous-match splitting is owned in processors/speller.rs without lib.rs dispatch changes.
- 03-01: Existing editor/navigator/selector and chord/punctuation/fallback behavior is now locked by schema-loaded ABI fixtures.
- 04-03: Represent advanced source and compiled dictionary data on TableDictionary instead of parallel runtime-specific structs.
- 04-03: Use bounded local Yune fixture markers for Phase 04 advanced compiled payload parity while rejecting unsupported librime sections structurally.
- 04-03: Carry compiled reverse dict_settings into runtime table dictionaries so ReverseLookupTranslator observes source and compiled settings through the same API.
- 04-03: Keep LevelDB/userdb learning, predictive updates, plugin translators, and AI-native ranking out of DATA-03.
- 04-04: Correction/tolerance data is represented as normalized TableDictionary metadata and merged through compiled/source dictionary paths.
- 04-04: Lookup expansion preserves original input first, then correction canonicalization, then tolerance candidates.
- 04-04: Correction/tolerance parser counts are capped before allocation and malformed compiled sections fall back with structured diagnostics.
- 06-03: Squirrel/macOS validation is represented as a source-modeled RimeApi lifecycle fixture plus documented direct-run blocker rather than a mandatory app bundle or input-method registration step.
- 06-03: Linux ibus-rime and fcitx-rime validation remains follow-up documentation with safe ABI source-model markers in native_frontends.rs, not a required daemon dependency for cargo test.
- 06-03: Native frontend mismatch capture continues to reuse the Phase 06 host trace schema and sanitized fixture rules instead of inventing a new target-specific trace format.
- Frontend benchmark baselines use a dependency-free Cargo bench target instead of Criterion to preserve MSRV safety and avoid unnecessary benchmark infrastructure.
- BENCH-01/BENCH-02 measurements stay at the rime_get_api / RimeApi function-table boundary rather than direct yune-core calls.
- AI-native readiness is GO WITH CONDITIONS, based on Phase 6 validation and benchmarks while keeping providers, rankers, context policy, memory policy, and privacy controls out of scope.
- TypeDuck-Web adapter seed work added a Yune-owned `yune_typeduck_*` Rust C/WASM bridge, native adapter contract tests, and browser filesystem documentation before formal Phase 7 planning.
- TypeDuck-Web browser integration should proceed through WASM export contract, TypeScript bridge, browser filesystem persistence, and app-shaped E2E before AI-native frontend exposure.
- 07-01: Use scripts/typeduck-exports.txt as the canonical non-prefixed TypeDuck adapter export list.
- 07-01: Document wasm32-unknown-emscripten plus Emscripten EXPORTED_FUNCTIONS/EXPORTED_RUNTIME_METHODS as the browser build contract without changing lib.rs facade wiring.
- Treat missing browser schema/dictionary assets as an init-time failure before starting the process-global RIME service.
- Document Phase 7 as a handoff contract: one active process-global service, host-owned MEMFS/IDBFS layout and sync, and deterministic verified-or-blocked build output.
- Represent stale deployed config recovery as a deterministic test fixture over existing helpers instead of adding metadata heuristics the helper cannot know.
- Keep recovery documentation local-first and caller-owned: explicit assets, explicit sync boundaries, and no browser app/network/cache policy in Phase 09.
- Document userdb persistence as an explicit host sync boundary because current native exports do not expose userdb mutation notifications.
- D-12/TYPEDUCK-E2E-04: Final findings separate TypeDuck-Web app/source blockers, Yune adapter/runtime mismatches, and environment/tooling blockers.
- D-13/TYPEDUCK-E2E-04: Phase ends with NO-GO recommendation for AI-native frontend exposure due to browser validation blockers.
- D-14: AI-native provider calls, candidate generation, ranking, context, memory, privacy controls, and new first-party Yune frontend remain deferred.
- D-15/WIN-TEST-01: TypeDuck-Windows native IME is the next tracked milestone; first unblock Windows test trust before feature work.
- D-16/WIN-ABI-01: Fork-only config list append APIs are the first feature slice after the Windows baseline because they need no external oracle.
- D-17/WIN-ORACLE-01: Comment semantics and Cantonese/Jyutping parity must be driven by TypeDuck-HK/librime v1.1.2 goldens or documented blockers.

### Pending Todos

- Phase 11 — Windows Test Baseline — fix `librime_signature_modified_time` shape and test-only poison-lock recovery first.
- Phase 12 — Fork Config List Append ABI — implement `config_list_append_{string,bool,int,double}` through `RimeApi`.
- Acquire TypeDuck-HK/librime v1.1.2 plus the pinned TypeDuck schema before comment semantics or Cantonese parity changes.

### Blockers/Concerns

- TypeDuck-Web browser integration ended NO-GO for AI-native exposure because the WASM artifact/tooling path blocked real browser validation.
- TypeDuck-HK/librime v1.1.2 binary and pinned TypeDuck schema are required before Items 4 and 6 of the Windows contract.
- Native Windows packaging may require MSVC target/toolchain availability outside the Rust workspace.

## Deferred Items

Items acknowledged and carried forward from previous milestone close:

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Plugin compatibility | librime C++ plugin ABI, Lua, octagram, predict, proto | Deferred | Initialization |
| Product frontend | New graphical end-user frontend | Deferred | Initialization |
| AI extension layer | Production local model bridge and opt-in contextual suggestions | Deferred | Initialization |
| Schema semantics | Deeper librime gear behavior beyond ABI/runtime safety | Deferred | Phase 2 |
| Compiled data | `.table.bin`, `.prism.bin`, `.reverse.bin` payload consumption | Deferred | Phase 2 |
| UserDB storage | LevelDB/userdb compatibility beyond plain text shim | Deferred | Phase 2 |

## Session Continuity

Last session: 2026-06-18T00:00:00.000Z
Stopped at: Ready to execute 11-01 Windows test baseline
Resume file: None

**Completed Phase:** 10 (TypeDuck-Web App Integration And E2E) — 4 plans — 2026-05-05 — ended with NO-GO recommendation
**Next Phase:** 11 (Windows Test Baseline) — 1 plan — ready to execute

**Planned Milestone:** TypeDuck-Windows Native IME Contract — 6 phases — tracked in ROADMAP.md and REQUIREMENTS.md
