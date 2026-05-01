# Phase 6: Real Frontend Validation And Benchmark - Context

**Gathered:** 2026-05-01
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 6 validates the completed compatibility foundation through real frontend-shaped lifecycle hosts and records frontend-sensitive performance baselines before AI-native product work begins. The phase covers native host-shaped loading, TypeDuck-Web-style browser/WebAssembly validation, macOS/Squirrel-shaped validation or reproducible blockers, scoped Linux frontend follow-up, regression capture for observed ABI/runtime mismatches, and benchmark baselines for session, key-processing, deployment/dictionary, and userdb paths.

This phase does not design the AI-native provider/ranker/context/memory layer, does not build a new graphical frontend, and does not attempt full librime C++ plugin ABI compatibility.

</domain>

<decisions>
## Implementation Decisions

### Validation Target Order
- **D-01:** Start with the existing Cargo-built `yune-rime-api` cdylib dynamic-loader path and expand it into a host-shaped native validation harness that records lifecycle call traces.
- **D-02:** Treat TypeDuck-Web as the first real application frontend validation target after the native loader harness, because its wrapper exercises `rime_get_api`, setup, initialize, notification handling, deploy/maintenance, session creation, key simulation, context/commit reads, candidate selection/deletion, page changes, customization, and browser-local persistence.
- **D-03:** TypeDuck-Web validation is additive, not a replacement for native IME validation; browser/WebAssembly does not cover Squirrel, ibus, fcitx, dynamic-library loading, threading, packaging, or OS input-context behavior.
- **D-04:** Attempt Squirrel/macOS validation after the TypeDuck-Web path because the current development environment is macOS; if direct integration cannot run locally, capture reproducible blockers and build fixtures around the mismatches that can be reproduced.
- **D-05:** Scope ibus-rime/fcitx-rime validation after macOS validation, with environment requirements documented instead of forcing Linux setup into the earliest plan.

### Frontend Lifecycle Coverage
- **D-06:** The host-shaped validation harness must cover `rime_get_api`, setup, initialize, deploy or maintenance/start behavior, schema selection, session create/destroy, key processing, context/status reads, commit reads, notification handler replacement, repeated initialize/finalize, stale sessions, and teardown.
- **D-07:** Prefer lifecycle call-trace notes and focused regression tests over broad frontend emulation. If a frontend-observed mismatch appears, capture the call sequence and expected/observed behavior before fixing it.
- **D-08:** Keep validation anchored at the RIME ABI boundary in `crates/yune-rime-api`; do not move C ABI allocation ownership into `yune-core`, and do not bypass schema installation or runtime path APIs.

### TypeDuck-Web Integration Shape
- **D-09:** Research and planning should inspect TypeDuck-Web's current wrapper shape and map its calls onto Yune's ABI, but Yune should not vendor TypeDuck-Web code unless a plan explicitly justifies a bounded fixture or adapter.
- **D-10:** Browser/WebAssembly findings should be documented as wrapper or browser-specific limits when they arise from Emscripten worker lifecycle, IDBFS persistence, or unavailable native dynamic loading rather than from Yune's ABI behavior.
- **D-11:** The TypeDuck-Web path should produce either a runnable reproduction, a minimized call-sequence fixture, or a documented blocker; all three are valid outputs if they are reproducible.

### Benchmark Baselines
- **D-12:** Add frontend-sensitive benchmark baselines only after the validation paths identify the hot lifecycle surfaces to measure.
- **D-13:** Benchmark coverage must include session create/destroy, per-key `RimeProcessKey` for simple ASCII and schema-loaded lookup paths, schema deployment/dictionary loading, and userdb learning/sync paths.
- **D-14:** Benchmark output should be reproducible enough for future comparison against frontend and AI-native changes, but this phase should avoid premature benchmark infrastructure beyond what supports the required baseline.

### AI-Native Readiness Gate
- **D-15:** End the phase with a go/no-go recommendation for beginning AI-native candidate/ranking design.
- **D-16:** The recommendation should be based on frontend lifecycle compatibility, documented wrapper/native blockers, and benchmark baselines, not on AI feature desirability.

### Claude's Discretion
- Choose the exact trace format, benchmark harness structure, and fixture organization during planning, provided they remain easy to run locally and keep owned behavior out of `lib.rs`/`main.rs` facades.
- Choose focused regression tests for observed frontend gaps instead of broad end-to-end frontend automation when the latter would require fragile external setup.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Milestone Scope
- `.planning/ROADMAP.md` — Phase 6 goal, requirements mapping, success criteria, and four planned work items.
- `.planning/REQUIREMENTS.md` — v2 frontend validation and benchmark requirements: `FRONTEND-VALIDATION-01` through `FRONTEND-VALIDATION-05`, `BENCH-01`, and `BENCH-02`.
- `.planning/STATE.md` — Current milestone state and Phase 6 resume point.
- `.planning/PROJECT.md` — Project constraints: librime as oracle, frontend validation is not complete, AI-native work remains layered/deferred.

### Frontend Validation Direction
- `docs/real-frontend-validation-plan.md` — Priority order, TypeDuck-Web target rationale, validation scenarios, benchmark scenarios, and expected outputs.
- `docs/compat-foundation-summary.md` — Completed compatibility foundation scope and explicit boundaries before Phase 6.

### Existing ABI And Frontend Test Surface
- `crates/yune-rime-api/tests/dynamic_loader.rs` — Current Cargo-built cdylib dynamic-loader harness and native loader lifecycle coverage.
- `crates/yune-rime-api/tests/frontend_client.rs` — Existing frontend-style API-table tests for schema lists, modules, config, context/status/commit, userdb, and related frontend calls.
- `crates/yune-rime-api/Cargo.toml` — Confirms `yune-rime-api` builds as both `rlib` and `cdylib` and uses `libloading` for loader tests.
- `crates/yune-rime-api/src/api_table.rs` — Builds the frontend-facing `RimeApi` and levers function tables.
- `crates/yune-rime-api/src/abi.rs` — Defines the `#[repr(C)]` ABI structs and function pointer shapes frontend hosts consume.
- `crates/yune-rime-api/src/lib.rs` — Exports the librime-shaped ABI entrypoints and key-processing/session orchestration.

### Prior Phase Context
- `.planning/phases/02-native-abi-validation-and-runtime-safety/02-CONTEXT.md` — Prior native ABI validation decisions and boundaries.
- `.planning/phases/05-userdb-and-scaling-hardening/05-CONTEXT.md` — Userdb lifecycle, sync, learning, and quality gate context that benchmark work must respect.

### Codebase Maps
- `.planning/codebase/STACK.md` — Workspace, crate, dynamic-library, and test stack summary.
- `.planning/codebase/INTEGRATIONS.md` — RIME frontend ABI, notification, module, filesystem, userdb, and runtime integration points.
- `.planning/codebase/ARCHITECTURE.md` — Layering, facade constraints, RIME ABI data flow, and anti-patterns relevant to Phase 6.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/yune-rime-api/tests/dynamic_loader.rs`: Existing dynamic loader harness already discovers/builds the Cargo cdylib, resolves `rime_get_api`, drives setup/initialize/deploy/session/key/context/status/commit/teardown, and records notifications.
- `crates/yune-rime-api/tests/frontend_client.rs`: Existing frontend-style API-table tests provide reusable helpers for traits, context/status/commit structs, notification capture, schema/config setup, module registration, and userdb-facing frontend behavior.
- `crates/yune-rime-api/src/api_table.rs` and `crates/yune-rime-api/src/abi.rs`: The stable ABI table and layout definitions are the correct validation boundary for host-shaped and frontend-shaped tests.
- `crates/yune-cli/src/rime_frontend.rs` and `crates/yune-cli/tests/frontend_surrogate.rs`: CLI frontend-surrogate behavior can inform transcript/call-sequence expectations, but Phase 6 should not treat CLI success as real frontend proof.

### Established Patterns
- Frontend-facing compatibility lives under `crates/yune-rime-api`; `yune-core` remains a deterministic Rust engine without C pointer ownership.
- Process-wide runtime, sessions, notification handlers, module registries, and API tables use guarded global state, so repeated initialize/finalize and session lifecycle validation must serialize tests where needed.
- Existing C ABI tests prefer focused Rust integration tests and in-process fixtures over requiring fragile external frontend installations for every regression.
- `lib.rs` and `main.rs` are facades/orchestration glue; new harness, trace, or benchmark behavior should live in owned modules or integration-test files.

### Integration Points
- Native loader validation connects through `rime_get_api` and the `RimeApi` function table from the compiled cdylib.
- TypeDuck-Web validation maps its worker/C++ wrapper calls to Yune's ABI setup, lifecycle, simulated key sequence or key processing, context/commit, candidate navigation, customization, deployment, and persistence paths.
- Squirrel/macOS validation connects through dynamic-library loading and native frontend lifecycle expectations; blockers should become reproducible notes or minimized fixtures.
- Benchmarking connects to the same ABI/runtime paths rather than only `yune-core`, because Phase 6 measures frontend-sensitive behavior.

</code_context>

<specifics>
## Specific Ideas

- Use TypeDuck-Web as a real application frontend validation target, but do not let browser/WASM success stand in for native IME lifecycle validation.
- Prefer a short, reproducible validation note or fixture for each frontend-observed gap before applying fixes.
- Use Phase 6 to decide whether AI-native design can start; keep AI implementation itself out of this phase.

</specifics>

<deferred>
## Deferred Ideas

- Full AI-native candidate provider, ranking, context policy, memory policy, and privacy controls remain deferred to the AI-native milestone after Phase 6 readiness.
- Full librime C++ plugin ABI compatibility remains deferred until a concrete frontend or distribution migration path requires it.
- A new graphical end-user frontend remains out of scope; Phase 6 validates compatibility through existing/front-end-shaped hosts.

</deferred>

---

*Phase: 6-Real Frontend Validation And Benchmark*
*Context gathered: 2026-05-01*
