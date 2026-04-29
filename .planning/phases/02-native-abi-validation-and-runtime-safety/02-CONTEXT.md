# Phase 2: Native ABI Validation And Runtime Safety - Context

**Gathered:** 2026-04-29
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 2 proves that `yune-rime-api` can be loaded and exercised through a native frontend-like ABI path, then converts the runtime-safety gaps exposed there into focused regression coverage and fixes. The phase covers dynamic loading, function-table lifecycle use, ABI layout/lifetime safety, notification/deployment/session determinism, and logical resource-ID validation. It does not build a graphical frontend, expand schema semantics beyond ABI safety, implement compiled dictionary payload consumption, or replace userdb storage.

</domain>

<decisions>
## Implementation Decisions

### Validation Target
- **D-01:** The minimum native/frontend-like validation target is a dynamic loader harness, not only the existing synthetic Rust frontend client and not a full real frontend integration first.
- **D-02:** The loader must exercise `rime_get_api` and drive the exported `RimeApi` function table first, because that is the primary frontend-facing access pattern this phase needs to prove.
- **D-03:** The loader should be integrated as a Rust test harness or focused test helper so it can run in normal Cargo workflows and produce repeatable regression evidence.
- **D-04:** Phase 2 should require a real loadable dynamic-library artifact for `yune-rime-api`; adding a harness that only skips because no dynamic artifact exists is not sufficient.

### Gap Handling
- **D-05:** When dynamic-loader validation exposes an ABI/frontend gap, first capture the observed behavior as a focused failing regression test, then fix it when the gap is inside Phase 2 scope.
- **D-06:** Immediate fixes in this phase should be limited to ABI safety and runtime safety: layout, lifetime, loading, notification, deployment, session lifecycle, process-global determinism, and resource-ID safety.
- **D-07:** Gaps outside Phase 2, such as deeper schema semantics, compiled dictionary behavior, or userdb storage compatibility, should be recorded as structured findings with observed behavior, expected librime/frontend behavior when known, scope decision, and target future phase.
- **D-08:** Failure to load the dynamic artifact on the current platform is a Phase 2 blocker unless a concrete platform limitation is documented with an equivalent replacement validation path.

### Lifecycle Stress
- **D-09:** Lifecycle validation should stress the core process-global paths most likely to affect native frontends: repeated setup/initialize/finalize, session create/destroy/cleanup-all, schema switching, deployment, and notification registration.
- **D-10:** Repeated lifecycle tests should use small deterministic loop counts that are appropriate for normal `cargo test`, not heavy stress/benchmark loops.
- **D-11:** Multi-threaded frontend-style calls should be documented as a finding unless the loader exposes a concrete concurrency issue; deterministic process-wide lifecycle behavior is the priority for this phase.
- **D-12:** Notification behavior should validate deterministic callback order around the deployment/schema lifecycle paths exercised by the loader, not only callback presence.

### Claude's Discretion
- Exact dynamic-loader crate/test organization, helper naming, loop counts, and structured finding file format are left to the planner/executor, provided the decisions above remain true.
- The planner may decide whether resource-ID validation is implemented as shared helper functions, boundary-specific validators, or a small logical-ID module, as long as all Phase 2 resource-ID requirements are covered.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase Scope And Requirements
- `.planning/ROADMAP.md` — Phase 2 goal, success criteria, and planned work.
- `.planning/REQUIREMENTS.md` — `ABI-01` through `ABI-04`, which define native/frontend-like validation, gap regression coverage, resource-ID validation, and deterministic process-wide runtime behavior.
- `.planning/PROJECT.md` — Project constraints: librime as compatibility oracle, typed Rust architecture, frontend validation caveat, runtime resource IDs as logical IDs, and quality gates.

### Prior Phase Context
- `.planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md` — Phase 1 boundary and decisions, especially that the CLI surrogate is not proof of native frontend integration and Phase 2 owns native/frontend-like loading paths.
- `.planning/phases/01-cli-frontend-surrogate/VERIFICATION.md` — Confirms Phase 1 delivered the ABI-backed CLI surrogate that Phase 2 can build from.
- `.planning/phases/01-cli-frontend-surrogate/01-SECURITY.md` — Confirms Phase 1 schema ID validation and fixture/replay safety decisions; Phase 2 must extend resource-ID safety beyond CLI schema IDs.

### Codebase Maps
- `.planning/codebase/INTEGRATIONS.md` — RIME ABI function table, exported C ABI modules, runtime paths, notification callbacks, module registry, and current packaging note that `yune-rime-api` lacks dynamic-library crate type.
- `.planning/codebase/ARCHITECTURE.md` — ABI layer responsibilities, process-wide state, key/session/schema/deployment flows, unsafe-boundary constraints, and anti-patterns.
- `.planning/codebase/CONCERNS.md` — High-priority concerns for native frontend validation, resource path validation, FFI ownership, global process state, notification/deployment behavior, and concurrency caveats.

### Compatibility Strategy
- `docs/analysis.md` — Librime compatibility rationale and known gaps around frontend validation and runtime behavior.
- `docs/roadmap.md` — Prior compatibility roadmap and native validation direction.
- `docs/refactor-plan.md` — Module/test ownership rules that still constrain Phase 2 implementation.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/yune-rime-api/src/api_table.rs`: Builds the `RimeApi` function table that the dynamic-loader harness must resolve and exercise.
- `crates/yune-rime-api/src/abi.rs`: Defines the ABI structs, layout, callbacks, and function table shapes that loader/lifecycle tests must treat as the external contract.
- `crates/yune-rime-api/src/session.rs`: Owns process-wide session IDs, lifecycle checks, cleanup, and session registry behavior for repeated lifecycle validation.
- `crates/yune-rime-api/src/runtime.rs`: Owns process-wide runtime paths and resource-root resolution, including areas that need logical resource-ID validation before filesystem joins.
- `crates/yune-rime-api/src/deployment.rs`: Owns initialize/maintenance/deploy/sync behavior and notification-triggering runtime paths.
- `crates/yune-rime-api/src/notifications.rs`: Owns notification handler registration and callback invocation paths.
- `crates/yune-rime-api/tests/frontend_client.rs`: Existing frontend-style client patterns for function-table setup, temp runtime roots, deployment, schema selection, context/status/commit reads, and cleanup.
- `crates/yune-cli/src/rime_frontend.rs`: Phase 1 ABI-backed CLI surrogate that can provide scriptable lifecycle expectations but is not sufficient native loading evidence.

### Established Patterns
- ABI-facing code validates pointers, initializes versioned structs with positive `data_size`, and pairs caller-owned allocations with matching free functions.
- Tests that touch process-wide RIME state use serialized guards and unique temp shared/user/staging directories.
- CLI and test output should remain deterministic and avoid environment-derived values unless a structured finding intentionally records a platform limitation.
- New compatibility slices should choose an owning implementation module, owning test module, and librime comparison target before code changes.

### Integration Points
- `crates/yune-rime-api/Cargo.toml` likely needs crate-type/package changes so a real loadable dynamic artifact exists for the loader harness.
- Dynamic-loader tests should connect at the compiled artifact boundary, resolve `rime_get_api`, and then exercise the function table rather than calling Rust functions directly.
- Resource-ID validation should cover config IDs, schema/dictionary names, custom config IDs, and userdb names before they are joined onto runtime roots.
- Notification-order validation should register callbacks before deployment/schema lifecycle operations and assert deterministic callback labels/order for the exercised paths.

</code_context>

<specifics>
## Specific Ideas

- Minimum proof is a Rust dynamic-loader harness loading a real `yune-rime-api` dynamic artifact and driving `RimeApi` through `rime_get_api`.
- Loader failure on the current platform blocks Phase 2 unless an equivalent replacement validation path is documented.
- Observed native/frontend-like gaps should become failing tests first, then fixes if they are ABI/runtime-safety gaps.

</specifics>

<deferred>
## Deferred Ideas

- Real Squirrel, Weasel, ibus-rime, fcitx-rime, or fcitx5-rime integration remains beyond the minimum Phase 2 validation target unless the dynamic loader makes it necessary.
- Full multi-threaded frontend stress is deferred unless a concrete issue appears during dynamic-loader validation.
- Deeper schema semantics belong to Phase 3.
- Compiled dictionary payload consumption belongs to Phase 4.
- Userdb storage/learning compatibility belongs to Phase 5.

</deferred>

---

*Phase: 02-native-abi-validation-and-runtime-safety*
*Context gathered: 2026-04-29*
