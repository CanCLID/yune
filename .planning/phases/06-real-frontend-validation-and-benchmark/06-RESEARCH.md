# Phase 06: Real Frontend Validation And Benchmark - Research

**Researched:** 2026-05-01
**Domain:** RIME C ABI frontend lifecycle validation, browser/WebAssembly wrapper validation, native frontend-shaped host tests, and frontend-sensitive benchmark baselines
**Confidence:** HIGH for ABI lifecycle and host-shape findings; MEDIUM for benchmark dependency choice and real frontend tool availability

## User Constraints

Locked Phase 6 decisions come from `.planning/phases/06-real-frontend-validation-and-benchmark/06-CONTEXT.md` and must be preserved during planning:

- Start with the existing Cargo-built `yune-rime-api` cdylib dynamic-loader path and expand it into a host-shaped native validation harness with lifecycle call traces.
- Treat TypeDuck-Web as the first real application frontend validation target after the native loader harness.
- Do not treat TypeDuck-Web browser/WebAssembly success as a substitute for native IME lifecycle validation.
- Attempt Squirrel/macOS validation after TypeDuck-Web; if direct integration cannot run locally, capture reproducible blockers and minimized fixtures.
- Scope ibus-rime/fcitx-rime validation after macOS validation with environment requirements documented.
- Capture frontend-observed ABI/runtime mismatches as notes, fixtures, or focused tests before fixes.
- Add frontend-sensitive benchmark baselines for session lifecycle, per-key processing, deployment/dictionary loading, and userdb learning/sync.
- End the phase with a go/no-go recommendation for beginning AI-native candidate/ranking design.

## Project Constraints

- No `CLAUDE.md`, `.claude/skills/`, or `.agents/skills/` project-local instructions were found in the main working tree.
- `Cargo.toml` sets Rust edition 2021 and `rust-version = "1.76"`; new dependencies must respect the workspace MSRV.
- `crates/yune-rime-api/Cargo.toml` already declares `crate-type = ["rlib", "cdylib"]` and uses `libloading = "0.8"` as a dev-dependency.
- `docs/real-frontend-validation-plan.md` and `docs/compat-foundation-summary.md` exist in the main working tree and are canonical Phase 6 inputs.
- The GSD config has Nyquist validation disabled for this run, so no `VALIDATION.md` artifact is required.
- Security enforcement is enabled by default in the workflow; Phase 6 plans should include threat models around FFI ownership, local trace data, and resource-id/path boundaries.

## Summary

Phase 06 should be planned as a validation-and-measurement phase, not a compatibility-fix phase. The core implementation strategy is to extend the existing dynamic-loader/frontend-style ABI test surface into reusable host-shaped lifecycle scenarios, use TypeDuck-Web and native frontend sources as lifecycle models, capture mismatches before fixes, and then record reproducible ABI-level benchmark baselines.

The existing `crates/yune-rime-api/tests/dynamic_loader.rs` is the strongest starting point because it already discovers/builds the Cargo cdylib, loads it with `libloading`, resolves `rime_get_api`, checks the returned `RimeApi` table, and drives setup, initialize, deploy, session, schema, key, context, status, commit, notification, destroy, cleanup, and finalize behavior. `crates/yune-rime-api/tests/frontend_client.rs` provides complementary in-process API-table helpers for config, schema, levers, userdb, modules, context/status/commit, and notification behavior.

Real frontend validation should be split into two layers:

1. Mandatory source-modeled Rust traces that run through Yune's ABI boundary and are stable in normal development.
2. Optional/manual real frontend attempts for TypeDuck-Web, Squirrel/macOS, and Linux frontends that produce reproducible blocker notes when the required toolchain or OS integration is unavailable.

Benchmarks should measure public ABI flows rather than direct `yune-core` calls. Direct core benchmarks would miss function-table lookup, session registry behavior, runtime path setup, deployment, context/status/commit copying, free calls, and userdb persistence costs that frontends actually observe.

## Phase Requirements

| ID | Planning Support |
|----|------------------|
| FRONTEND-VALIDATION-01 | Extend `dynamic_loader.rs` or adjacent integration-test helpers into host-shaped cdylib validation covering `rime_get_api`, setup, initialize, deploy/maintenance, schema selection, session lifecycle, key processing, context/status/commit reads, notifications, and teardown. |
| FRONTEND-VALIDATION-02 | Map TypeDuck-Web's wrapper lifecycle to Yune ABI calls and produce a runnable reproduction, minimized call-sequence fixture, or documented blocker. |
| FRONTEND-VALIDATION-03 | Attempt Squirrel/macOS validation after TypeDuck-Web; capture direct-run blockers and convert source-modeled lifecycle differences into reproducible fixtures. |
| FRONTEND-VALIDATION-04 | Scope ibus/fcitx follow-up after macOS validation; document Linux environment requirements and source-modeled lifecycle differences without requiring Linux daemons in normal tests. |
| FRONTEND-VALIDATION-05 | Introduce a mismatch capture format and require observed ABI/runtime gaps to be captured as notes, fixtures, or focused tests before fixes. |
| BENCH-01 | Add benchmark baselines for session create/destroy, per-key `RimeProcessKey`, schema deployment/dictionary loading, and userdb learning/sync paths. |
| BENCH-02 | Make benchmark output reproducible and comparable for future frontend or AI-native changes. |

## Recommended Architecture

```text
Frontend host model fixture
  |
  | host-shaped scenario: setup/init/focus/key/status/context/deploy/finalize
  v
Dynamic loader / API-table harness
  |-- discovers Cargo-built cdylib when validating native loading
  |-- resolves rime_get_api
  |-- records ordered call trace + return values + notifications
  v
RimeApi function table boundary
  |-- setup / initialize / finalize
  |-- create/find/destroy session
  |-- process_key / simulate_key_sequence / candidate actions
  |-- get_commit / get_status / get_context + free_*
  |-- deploy / maintenance / sync / levers / config
  v
Yune runtime and session registry
  |-- runtime traits and paths
  |-- schema deployment and selection
  |-- per-session engine and processors
  |-- userdb and dictionary load paths
  v
Trace artifacts, focused tests, blocker notes, and benchmark baselines
```

### Suggested File Ownership

The planner may choose exact names, but should keep this ownership shape:

- `crates/yune-rime-api/tests/dynamic_loader.rs` — keep or extend cdylib discovery and native-loader smoke coverage.
- `crates/yune-rime-api/tests/frontend_hosts/` or equivalent integration-test helper module — shared trace model and host-shaped scenarios.
- `crates/yune-rime-api/tests/frontend_client.rs` — reuse helpers/patterns for in-process API-table behavior where appropriate.
- `fixtures/frontend-traces/` or `docs/frontend-validation/` — reproducible trace/blocker artifacts for frontend-observed gaps.
- `crates/yune-rime-api/benches/` or a small benchmark binary/test target — ABI-driven benchmark baselines, depending on dependency choice.
- `docs/real-frontend-validation-plan.md` — update with outcomes only if the implementation produces manual validation notes or readiness recommendations.

## Host Lifecycle Models

### Native loader / host-shaped baseline

Use the existing dynamic-loader harness as the mandatory baseline. It should cover:

- cdylib discovery/build under the Cargo target directory;
- `rime_get_api` symbol resolution;
- `RimeApi.data_size` validation;
- required function pointer availability;
- setup/initialize/finalize;
- notification handler registration/replacement;
- deploy or maintenance/start behavior;
- create/find/destroy/cleanup sessions;
- select schema;
- process keys;
- read and free status/context/commit;
- validate stale sessions after teardown/restart.

### TypeDuck-Web

TypeDuck-Web is useful because its `wasm/api.cpp` wrapper shape exercises a real app-style lifecycle:

- `rime_get_api` once into a global pointer;
- setup with shared and user data directories;
- initialize and notification handler registration;
- quick start or maintenance/deploy restart;
- one global session;
- key input through `simulate_key_sequence`/wrapper equivalent;
- context and commit reads serialized for JavaScript;
- candidate selection/deletion and page movement;
- customization and deploy;
- browser-local persistence through worker/IDBFS.

Planning should not require vendoring TypeDuck-Web or running Emscripten in every regression. Valid outputs are: a runnable local reproduction, a minimized Yune call-sequence fixture, or a documented blocker with exact missing tool/environment details.

### Squirrel/macOS

Squirrel/macOS validation should focus on lifecycle shape rather than product UI:

- setup/initialize at app level;
- per-input-context session creation;
- key event processing through RIME;
- commit/status/context reads in the order a frontend expects;
- focus/deactivation composition handling;
- deploy/sync/finalize/reinitialize behavior;
- notification callback behavior.

If direct Squirrel integration cannot run locally, the plan should produce a blocker note and a source-modeled fixture for the observed lifecycle expectations.

### ibus-rime / fcitx-rime

Linux frontend validation should be scoped after macOS:

- document required OS packages, daemon/session setup, and build/runtime requirements;
- map focus/reset/property/key-processing/status/context/commit lifecycles from source or docs;
- do not make `ibus`, `fcitx5`, or desktop daemons required for `cargo test --workspace`;
- create follow-up notes or fixtures that make future Linux validation reproducible.

## Benchmark Guidance

Benchmarks should be ABI-observed and reproducible. Required baseline categories:

1. Session create/destroy latency.
2. Per-key `RimeProcessKey` latency for simple ASCII and schema-loaded table lookup, including context/status/commit/free cycles where relevant.
3. Schema deployment and dictionary load latency for representative minimal and schema-loaded data.
4. Userdb learning, backup/restore, and sync latency with controlled record counts.

Dependency guidance:

- Latest `criterion` versions may exceed the workspace MSRV; verify compatibility before adding a benchmark dependency.
- If `criterion` is not MSRV-safe, use a dependency-free timing harness that writes deterministic baseline JSON/Markdown with clear limitations and avoids statistical claims.
- Do not benchmark `Engine::process_key_event` directly for BENCH-01/BENCH-02 unless it is explicitly supplementary and not the frontend baseline.

Baseline output should include enough metadata to compare future runs:

- benchmark name;
- git commit or phase label;
- operation count;
- schema/fixture name;
- data size or record count;
- measured duration/unit;
- toolchain/platform when available;
- whether the run is debug or release.

## Security And Safety Considerations

Phase 6 plans should include threat-model coverage for:

- FFI pointer ownership: every successful `get_context`, `get_status`, `get_commit`, iterator, or list read must pair with the correct `free_*` call before pointers are discarded.
- Null function pointers: host scenarios should report missing required functions as validation blockers instead of panicking through unchecked calls.
- Session lifecycle misuse: stale sessions after finalize/deploy/destroy should be tested deterministically.
- Local data leakage: trace and blocker artifacts must use synthetic temp directories and fixtures, not real user dictionaries or local personal paths.
- Resource IDs: validation and benchmarks must continue to treat schema/config/userdb IDs as logical IDs, not filesystem paths.

## Common Pitfalls

- Confusing CLI surrogate success with real frontend validation.
- Treating TypeDuck-Web browser/WASM behavior as proof of native frontend compatibility.
- Making Emscripten, full Xcode, IBus, or Fcitx daemons mandatory for ordinary tests.
- Fixing frontend-observed mismatches before preserving them as notes/fixtures/tests.
- Benchmarking the wrong layer by calling `yune-core` directly instead of the RIME ABI.
- Adding latest benchmark dependencies without MSRV review.
- Expanding Phase 6 into AI-native provider/ranking implementation instead of ending with a readiness recommendation.

## Planning Recommendations

A four-plan split matches the roadmap and dependency order:

1. **06-01 Native host trace harness** — extend dynamic loader into reusable host-shaped lifecycle validation and trace/mismatch capture.
2. **06-02 TypeDuck-Web validation** — map and attempt the browser/WASM wrapper path; save runnable reproduction, fixture, or blocker.
3. **06-03 Squirrel/macOS and Linux scope** — attempt or source-model macOS validation; convert observed gaps into fixtures; document ibus/fcitx follow-up requirements.
4. **06-04 Benchmarks and readiness** — add ABI-level baseline benchmarks and write the AI-native go/no-go recommendation.

Wave dependencies should be sequential where evidence is required:

- Wave 1: native host trace harness.
- Wave 2: TypeDuck-Web validation using the trace/mismatch format from Wave 1.
- Wave 3: Squirrel/macOS validation and Linux scoping using the same format.
- Wave 4: benchmarks and readiness recommendation after validation surfaces are known.

## Sources

### Repository sources

- `.planning/phases/06-real-frontend-validation-and-benchmark/06-CONTEXT.md` — locked Phase 6 decisions.
- `.planning/ROADMAP.md` — Phase 6 plans and success criteria.
- `.planning/REQUIREMENTS.md` — Phase 6 requirements.
- `.planning/STATE.md` — current milestone state.
- `docs/real-frontend-validation-plan.md` — validation priority order and scenarios.
- `docs/compat-foundation-summary.md` — completed compatibility foundation and boundaries.
- `crates/yune-rime-api/tests/dynamic_loader.rs` — existing cdylib dynamic-loader harness.
- `crates/yune-rime-api/tests/frontend_client.rs` — existing frontend-style API-table coverage.
- `crates/yune-rime-api/Cargo.toml` — cdylib and dev-dependency configuration.
- `.planning/codebase/STACK.md`, `.planning/codebase/INTEGRATIONS.md`, `.planning/codebase/ARCHITECTURE.md` — stack, integration, and architecture maps.

### External sources to verify during implementation/research follow-up

- `librime` public `rime_api.h` for ABI struct/function contracts.
- TypeDuck-Web `wasm/api.cpp`, `src/worker.ts`, and `src/rime.ts` for wrapper lifecycle and browser persistence behavior.
- Squirrel macOS frontend sources for app/controller lifecycle.
- ibus-rime and fcitx5-rime sources for Linux frontend lifecycle and follow-up scope.

## RESEARCH COMPLETE
