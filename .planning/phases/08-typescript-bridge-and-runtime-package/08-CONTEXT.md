# Phase 8: TypeScript Bridge And Runtime Package - Context

**Gathered:** 2026-05-04
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 8 turns the Phase 7 `yune_typeduck_*` C/WASM adapter contract into a typed TypeScript bridge that browser code can call safely. It owns the wrapper API surface for init, key processing, candidate actions, deploy, customize, cleanup, response JSON parsing, response-free pairing, deterministic browser keycode/mask mapping, and caller-visible lifecycle constraints.

This phase does not mount MEMFS/IDBFS, preload schema or dictionary assets, synchronize persistence, clone or patch upstream TypeDuck-Web, run real browser E2E, introduce multi-instance isolation, or add AI-native ranking/provider behavior. Those remain Phase 9, Phase 10, and the future AI-native milestone.

</domain>

<decisions>
## Implementation Decisions

### Wrapper Location And Tooling
- **D-01:** Add a minimal repository-owned TypeScript bridge package rather than embedding TypeScript snippets only in docs. The package should be small enough to test deterministically in Node without requiring a real Emscripten browser build.
- **D-02:** Because the repo currently has no JS/TS tooling files, Phase 8 may introduce the smallest necessary TypeScript test/build setup for the wrapper. Avoid broad frontend app scaffolding, bundler config for TypeDuck-Web, or browser E2E infrastructure.
- **D-03:** Keep the TypeScript bridge adapter-shaped and independent from `RimeApi`; it wraps only the Phase 7 `yune_typeduck_*` export list and must not expose librime-shaped function-table details to browser callers.

### TypeScript API Shape
- **D-04:** The wrapper should expose typed operations for `init`, `processKey`, `selectCandidate`, `deleteCandidate`, `flipPage`, `deploy`, `customize`, and `cleanup`, matching Phase 8 requirements TYPEDUCK-JS-01 through TYPEDUCK-JS-04.
- **D-05:** Treat the Emscripten `Module` object as an injected dependency with a narrow typed interface (`cwrap`/`UTF8ToString` or equivalent). Tests should use a fake module so Phase 8 does not depend on local Emscripten output or upstream TypeDuck-Web source.
- **D-06:** State pointers and response pointers should remain opaque numbers at the JS boundary. The wrapper may return small TypeScript classes/objects around those pointers, but callers should not manipulate raw C pointer lifetimes directly except through the wrapper contract.

### Response Ownership And JSON Parsing
- **D-07:** Centralize all response handling in one wrapper path: call the adapter operation, read `yune_typeduck_response_json`, copy/parse the JSON string, read handled state if needed, and always call `yune_typeduck_free_response` exactly once for non-null owned responses.
- **D-08:** Null response pointers and null JSON pointers are wrapper-level errors. The TypeScript layer should surface deterministic errors rather than fabricating empty candidate lists, because missing assets and lifecycle problems must stay visible before Phase 9 recovery work.
- **D-09:** Define TypeScript response types from the documented JSON response shape in `docs/typeduck-web-adapter.md`: `handled`, `commits`, optional/nullable `context`, optional/nullable `status`, and optional `error`. Keep parsing permissive enough for nullable context/status but strict enough to catch non-object or malformed JSON.

### Browser Key Mapping
- **D-10:** Keycode/mask mapping must be explicit in Phase 8 and covered by deterministic tests. The mapping should convert browser `KeyboardEvent`-like inputs into the integer keycode/mask pair passed to `yune_typeduck_process_key`.
- **D-11:** Start with the key paths needed by TypeDuck-Web integration: printable character keys, Enter/Backspace/Escape/Space, arrow keys, PageUp/PageDown, number selection keys, and common modifier masks. Avoid trying to model every platform/browser edge case before Phase 10 observes the real app seam.
- **D-12:** Keep the mapping function independently testable without DOM globals by accepting a narrow event-like object instead of requiring real `KeyboardEvent` instances.

### Runtime Lifecycle Contract
- **D-13:** The wrapper must make the one-active-process-global Yune/RIME service constraint visible. `cleanup` finalizes process-global service state through `yune_typeduck_cleanup`; callers should not create multiple simultaneous states with different dirs in one Module instance.
- **D-14:** The wrapper should guard against obvious misuse within one wrapper instance: operations after cleanup should fail deterministically, cleanup should be idempotent at the TypeScript layer, and init failure should throw or return a typed failure before any state object is exposed.
- **D-15:** Browser filesystem setup remains host responsibility. The wrapper can accept `sharedDataDir`, `userDataDir`, and `schemaId`, but it must not mount IDBFS, preload assets, sync persistence, or hide missing-asset failures in Phase 8.

### Claude's Discretion
- Choose exact package path, file names, and JS test runner during planning, as long as the result is minimal, deterministic, and does not turn Phase 8 into a browser app or TypeDuck-Web checkout.
- Prefer a fake Emscripten Module contract in tests over installing Emscripten or building WASM during Phase 8.
- Prefer small typed interfaces and focused tests over large generated bindings. Generated or build-system-specific TypeDuck-Web integration remains Phase 10.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase Scope And Requirements
- `.planning/ROADMAP.md` — Phase 8 goal, success criteria, and planned slices `08-01` through `08-03` for TypeScript wrapper, tests, and lifecycle documentation.
- `.planning/REQUIREMENTS.md` — `TYPEDUCK-JS-01` through `TYPEDUCK-JS-04`, defining wrapper operations, response ownership, key mapping, and lifecycle documentation.
- `.planning/STATE.md` — Current project position after Phase 7 completion and next-phase routing.
- `.planning/PROJECT.md` — Compatibility, architecture, quality, security, and AI-native deferral constraints.

### Phase 7 Handoff
- `.planning/phases/07-wasm-build-and-export-contract/07-CONTEXT.md` — Locked decisions for Emscripten target, export list, native fallback, one-active-service lifecycle, and MEMFS/IDBFS host assumptions.
- `.planning/phases/07-wasm-build-and-export-contract/07-VERIFICATION.md` — Evidence that Phase 7 build/export contract and native fallback tests passed locally.
- `scripts/typeduck-exports.txt` — Canonical 11-symbol `yune_typeduck_*` export list that the TypeScript bridge should wrap.
- `scripts/typeduck-wasm-build.sh` — Reproducible browser build/blocker command path; Phase 8 tests should not duplicate its symbol-verification job.

### Adapter Contract
- `crates/yune-rime-api/src/typeduck_web.rs` — Source of the C/WASM bridge symbols, opaque state/response pointers, response ownership, JSON serialization, deploy/customize wrappers, and cleanup behavior.
- `crates/yune-rime-api/tests/typeduck_web.rs` — Native fallback tests demonstrating operation flows, response copying/freeing, null handling, browser host layout constraints, and process-global serialization.
- `docs/typeduck-web-adapter.md` — Documented exported symbols, response JSON shape, ownership rules, suggested JS flow, browser filesystem contract, build/export contract, and current out-of-scope items.
- `crates/yune-rime-api/Cargo.toml` — Confirms the adapter crate remains the Rust ABI crate; Phase 8 should avoid changing Rust crate shape unless planning proves it necessary.

### Codebase Maps
- `.planning/codebase/STACK.md` — Rust workspace and absence of existing JS package tooling.
- `.planning/codebase/INTEGRATIONS.md` — ABI, runtime filesystem, deployment, and frontend integration points relevant to browser callers.
- `.planning/codebase/ARCHITECTURE.md` — Process-global runtime state, ABI unsafe boundary, facade constraints, and integration-test patterns.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `scripts/typeduck-exports.txt`: Provides the exact adapter function list that the TypeScript bridge should bind.
- `docs/typeduck-web-adapter.md`: Provides the response JSON schema, suggested JS flow, lifecycle warning, and filesystem boundaries that wrapper docs/tests should mirror.
- `crates/yune-rime-api/src/typeduck_web.rs`: Defines the real operation semantics and pointer ownership contract the TypeScript wrapper must enforce.
- `crates/yune-rime-api/tests/typeduck_web.rs`: Shows the safe response pattern: copy JSON text before `yune_typeduck_free_response` and serialize process-global lifecycle tests.

### Established Patterns
- Frontend/browser compatibility surfaces belong at the `yune-rime-api` boundary or adjacent adapter tooling, not in `yune-core`.
- Process-global service state is a real contract constraint, not a temporary test artifact.
- Missing browser tooling should not block deterministic local tests; Phase 8 should use fake-module tests for wrapper behavior.
- New compatibility slices should have an owning implementation module and owning test module rather than putting behavior into facade files.

### Integration Points
- The TypeScript wrapper calls Emscripten-exported `yune_typeduck_*` symbols by name through an injected Module-like object.
- The wrapper receives browser-host paths and schema IDs but leaves actual virtual filesystem creation and persistence sync to Phase 9 host code.
- Key mapping feeds integer `keycode` and `mask` values to `yune_typeduck_process_key`; Phase 8 owns this deterministic translation layer.
- TypeDuck-Web app binding replacement is not done here, but Phase 8 should produce an API that Phase 10 can plug into the upstream app seam.

</code_context>

<specifics>
## Specific Ideas

- Use the `docs/typeduck-web-adapter.md` suggested JS flow as the behavioral seed, but replace raw `Module.cwrap` snippets with reusable typed wrapper functions.
- Model response ownership with a helper such as `withResponse`/`readResponse` so every operation goes through one free-pairing path.
- Tests should prove the fake response pointer is freed on successful parse and on parse/error paths.
- The wrapper should expose lifecycle limits in both types/docs and runtime behavior: after `cleanup`, further operation calls should fail clearly.

</specifics>

<deferred>
## Deferred Ideas

- Browser virtual filesystem layout creation, schema/dictionary asset preload, IDBFS sync, missing asset recovery, and stale deployed config recovery remain Phase 9.
- Cloning upstream TypeDuck-Web, identifying its current librime/WASM bridge seam, patching it to use Yune, and running real browser flows remain Phase 10.
- Multi-instance isolation beyond one active process-global Yune/RIME service remains out of scope for this milestone unless a later TypeDuck-Web integration blocker requires it.
- AI-native provider, ranking, context, memory, privacy, and frontend exposure behavior remains deferred until TypeDuck-Web integration produces a go/no-go recommendation.

</deferred>

---

*Phase: 08-typescript-bridge-and-runtime-package*
*Context gathered: 2026-05-04*
