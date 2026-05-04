# Phase 7: WASM Build And Export Contract - Context

**Gathered:** 2026-05-02
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 7 turns the seeded Yune TypeDuck adapter into a reproducible browser/WASM build contract. It covers the intended Emscripten/WASM target, required `yune_typeduck_*` export list, local toolchain detection or documented blocker, native fallback validation when browser tooling is unavailable, and documentation of linker/export flags plus runtime file-layout assumptions.

This phase does not build the TypeScript wrapper package, does not orchestrate browser IDBFS persistence, does not clone or patch upstream TypeDuck-Web, and does not implement AI-native candidate/ranking behavior. Those remain Phase 8, Phase 9, Phase 10, and future AI-native milestone work respectively.

</domain>

<decisions>
## Implementation Decisions

### WASM Target Contract
- **D-01:** Treat `wasm32-unknown-emscripten` as the intended browser build target for this milestone because TypeDuck-Web-style integration needs Emscripten C ABI exports and filesystem/runtime hooks, not `wasm-bindgen` as the primary contract.
- **D-02:** Phase 7 should define a single documented build command or script path that attempts the Emscripten build and reports a reproducible local-toolchain blocker when Emscripten is unavailable. Missing Emscripten locally is not a failure if the blocker is deterministic and native adapter tests remain the fallback.
- **D-03:** Keep the Rust adapter in `crates/yune-rime-api` for this phase. Do not create a separate adapter crate unless planning proves the export/build contract cannot be expressed safely from the existing cdylib crate.

### Export Retention Verification
- **D-04:** The required browser export surface is the seeded `yune_typeduck_*` API: init, process-key, select-candidate, delete-candidate, flip-page, deploy, customize, cleanup, response-json, response-handled, and free-response.
- **D-05:** Add a deterministic symbol/export verification path that works in native mode and, when Emscripten output exists, checks the generated WASM/JS artifact for the same required symbol names.
- **D-06:** Export verification should be adapter-specific and must not broaden or mutate the existing librime-shaped `RimeApi` function table. The Phase 7 contract verifies symbol presence for JS callers; Phase 8 owns typed JS call ergonomics.

### Local Toolchain Fallback
- **D-07:** Native adapter contract tests in `crates/yune-rime-api/tests/typeduck_web.rs` remain the authoritative fallback when the local machine cannot build or run the browser target.
- **D-08:** Toolchain detection should fail with an actionable, reproducible message naming the missing Emscripten/Rust target component rather than silently skipping all validation.
- **D-09:** The planner may choose whether the detection lives in a small script, Cargo alias-like documented command, integration test helper, or docs-first command block, provided CI/local behavior is explicit and easy to rerun.

### Browser Constraint Documentation
- **D-10:** Documentation must make the one-active-process-global-service constraint visible: `yune_typeduck_cleanup` finalizes the process-global RIME service, and multiple simultaneous TypeDuck states with different dirs are not supported by this first contract.
- **D-11:** Documentation must state browser host assumptions for MEMFS/IDBFS paths: shared data, user data, and `user_data_dir/build` must exist before init; schema/dictionary assets must be preloaded; persistence sync remains a JS host responsibility until Phase 9.
- **D-12:** Document required linker/export flags and known host assumptions without requiring upstream TypeDuck-Web source access in Phase 7. Upstream clone/replace testing is explicitly Phase 10.

### Claude's Discretion
- Choose the exact script name, artifact inspection command, and docs location during planning as long as the commands are deterministic, preserve MSRV/workspace quality gates, and keep owned behavior out of `lib.rs` facades.
- Prefer small, reviewable build-contract checks over introducing a full browser bundler, npm package, or TypeDuck-Web checkout in this phase.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase Scope And Requirements
- `.planning/ROADMAP.md` — Phase 7 goal, success criteria, and plans `07-01` through `07-03` for WASM build and export contract.
- `.planning/REQUIREMENTS.md` — `TYPEDUCK-WASM-01` through `TYPEDUCK-WASM-03`, defining build/blocker reproducibility, export retention, and native fallback requirements.
- `.planning/STATE.md` — Current project position: Phase 7 ready to plan after TypeDuck adapter seed work.
- `.planning/PROJECT.md` — Compatibility, architecture, local-first, and facade ownership constraints that continue to apply.

### Seed Adapter Baseline
- `crates/yune-rime-api/src/typeduck_web.rs` — Seeded `yune_typeduck_*` C/WASM bridge, opaque state/response structs, JSON response ownership, deploy/customize wrappers, and cleanup behavior.
- `crates/yune-rime-api/tests/typeduck_web.rs` — Native adapter contract tests for lifecycle, JSON state, candidate actions, deploy/customize, null handling, and response freeing.
- `docs/typeduck-web-adapter.md` — Current browser filesystem contract, JS call shape, response ownership rules, and explicit deferred browser/TypeDuck-Web work.
- `crates/yune-rime-api/Cargo.toml` — Confirms `yune-rime-api` crate type and dependencies relevant to native cdylib/WASM build behavior.
- `crates/yune-rime-api/src/lib.rs` — Current facade/export wiring for the adapter module; Phase 7 should avoid adding owned build logic here.

### Prior Phase Context
- `.planning/phases/06-real-frontend-validation-and-benchmark/06-CONTEXT.md` — Phase 6 TypeDuck-Web validation order, wrapper/browser limits, ABI boundary, and benchmark/readiness decisions.
- `.planning/phases/05-userdb-and-scaling-hardening/05-CONTEXT.md` — Userdb lifecycle and quality gate constraints relevant to browser persistence assumptions.
- `.planning/phases/04-compiled-dictionary-data/04-CONTEXT.md` — Runtime dictionary and deployment data boundaries that browser asset preload must not bypass.

### Codebase Maps
- `.planning/codebase/STACK.md` — Rust workspace, Cargo build model, crate types, dependency/runtime stack, and lack of JS package tooling.
- `.planning/codebase/INTEGRATIONS.md` — RIME ABI, function table, runtime filesystem, deployment, notification, and packaging integration points.
- `.planning/codebase/ARCHITECTURE.md` — Layering, process-global runtime state, ABI unsafe boundary, facade constraints, and integration-test patterns.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/yune-rime-api/src/typeduck_web.rs`: Current adapter symbols are already isolated behind prefixed exports and can be used as the canonical export list for Phase 7.
- `crates/yune-rime-api/tests/typeduck_web.rs`: Native fallback tests already exercise the adapter without browser tooling and should remain the deterministic fallback path.
- `docs/typeduck-web-adapter.md`: Existing adapter documentation is the right place to extend export flags, build commands, local blocker behavior, and host assumptions.
- `crates/yune-rime-api/Cargo.toml`: Build contract work should preserve the crate's role as an `rlib`/`cdylib` ABI crate and avoid workspace-wide packaging churn.

### Established Patterns
- Frontend-facing compatibility stays in `crates/yune-rime-api`; `yune-core` should not learn about C pointers, Emscripten, or browser filesystem details.
- Process-wide RIME runtime state means adapter/browser lifecycle tests must be serialized where they initialize/finalize global service state.
- Compatibility slices use focused integration tests and documented reproducible blockers instead of requiring fragile external installations for every local run.
- `lib.rs` remains facade/export glue; build-contract scripts, docs, or tests should live outside facade logic.

### Integration Points
- The browser host will call direct `yune_typeduck_*` symbols from generated Emscripten output, while later Phase 8 TypeScript wraps those calls.
- Build validation connects to Cargo/Emscripten output and symbol inspection, not to TypeDuck-Web app source yet.
- Runtime file-layout validation connects to the adapter's `shared_data_dir`, `user_data_dir`, deploy/customize behavior, and the existing documentation's MEMFS/IDBFS assumptions.

</code_context>

<specifics>
## Specific Ideas

- The final milestone acceptance remains upstream TypeDuck-Web clone/replace/browser E2E in Phase 10; Phase 7 should not water that down, but it should not pull Phase 10 work forward.
- Treat missing local Emscripten as a documented blocker only if the command path proves what is missing and native adapter tests still run.
- Keep export verification centered on the `yune_typeduck_*` surface because TypeScript bridge memory ownership in Phase 8 depends on every owned response being paired with `yune_typeduck_free_response`.

</specifics>

<deferred>
## Deferred Ideas

- TypeScript wrapper types, response parsing/freeing enforcement, and browser keycode/mask mapping remain Phase 8.
- Browser virtual filesystem layout tests, asset preload orchestration, IDBFS sync, and stale/missing asset recovery remain Phase 9.
- Cloning upstream TypeDuck-Web, replacing its librime/WASM core with Yune, and real browser E2E validation remain Phase 10.
- AI-native provider, ranking, context, memory, and privacy work remains deferred until the TypeDuck-Web integration milestone produces a frontend exposure recommendation.

</deferred>

---

*Phase: 07-wasm-build-and-export-contract*
*Context gathered: 2026-05-02*
