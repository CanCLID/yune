# Phase 1: CLI Frontend Surrogate - Context

**Gathered:** 2026-04-29
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 1 turns `yune-cli` into a scriptable frontend surrogate that exercises
`yune-rime-api` setup, schema deployment and selection, session lifecycle, key
processing, context/status/commit reads, rendering, and deterministic transcript
replay. The phase validates the ABI path from a CLI client; it does not claim
native frontend integration, build a graphical frontend, introduce AI behavior,
or expand compiled-data/userdb compatibility.

</domain>

<decisions>
## Implementation Decisions

### Command Surface And Runtime Inputs
- **D-01:** Add an explicit ABI-backed frontend path in `yune-cli` rather than
  silently replacing the current core-backed `run`/`check` behavior in the first
  implementation step. Keep the existing core fixture path available until ABI
  transcript checks provide a deterministic replacement.
- **D-02:** ABI/frontend commands must accept explicit `shared_data_dir` and
  `user_data_dir` inputs. Optional prebuilt, staging, and log directory flags may
  map directly onto `RimeTraits` when needed.
- **D-03:** Schema deployment and schema selection should be explicit operations
  in the frontend path. Hidden process-global RIME paths should not be required
  for deterministic CLI runs.

### ABI Lifecycle And Ownership
- **D-04:** The CLI frontend surrogate should drive `yune-rime-api` through the
  exported `RimeApi` function table from `rime_get_api`, matching the
  frontend-style test shape instead of calling `yune-core` directly.
- **D-05:** Unsafe pointer conversion, C string handling, ABI struct
  initialization, and allocation/free pairing belong in focused wrappers in
  `crates/yune-cli/src/rime_frontend.rs`.
- **D-06:** The frontend run lifecycle should initialize/setup traits, deploy or
  select schema as requested, create a session, process each key through
  `process_key`, read commit/context/status after each event, destroy the
  session, and clean up/finalize on every path that creates process-wide state.

### Transcript And Rendering Contract
- **D-07:** Transcript replay output should record ABI-visible state after each
  key event, not only a final snapshot. This makes frontend-surrogate behavior
  comparable event by event against librime.
- **D-08:** Deterministic transcript JSON should include the processed key and
  result, commits drained after the event, input/caret/preedit, candidate page
  fields, page/highlight metadata, select keys/labels when available, and status
  flags.
- **D-09:** Human-readable rendering and deterministic JSON serialization should
  stay separate: `render.rs` for operator-facing output, `transcript.rs` for the
  comparison contract.

### Test Ownership And Validation
- **D-10:** Keep `main.rs` as orchestration glue. Put command parsing in
  `args.rs`, ABI frontend lifecycle in `rime_frontend.rs`, human output in
  `render.rs`, deterministic transcript serialization in `transcript.rs`, and
  retained core fixture compatibility in `sample_core.rs`/`fixture.rs`.
- **D-11:** Add focused CLI tests for command parsing, transcript generation, and
  fixture comparison, plus ABI/frontend-style tests that use the same
  function-table and temp-runtime patterns as
  `crates/yune-rime-api/tests/frontend_client.rs`.
- **D-12:** Every behavior slice added in this phase needs an owning
  implementation module, owning test module, and explicit librime comparison
  target before implementation. Do not move unrelated code or split large tests
  mechanically unless this phase's transcript/replay design exposes a real
  ownership boundary.

### Compatibility Framing
- **D-13:** librime remains the external behavior oracle for user-visible
  transcript semantics, schema behavior, ABI lifecycle expectations, and
  migration-sensitive behavior.
- **D-14:** The CLI frontend is an intermediate validation layer. It should make
  ABI gaps measurable and scriptable, but Phase 2 remains responsible for native
  frontend-like loading paths and real frontend validation.

### Claude's Discretion
- Exact subcommand names, flag spelling, fixture naming, wrapper struct names,
  and small formatting details are left to the planner/executor, provided the
  decisions above remain true.
- The planner may decide whether deploy is a default step in a convenience
  command or an explicit subcommand, as long as transcript replay remains
  deterministic and runtime paths are explicit.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase Scope And Requirements
- `.planning/ROADMAP.md` - Phase 1 goal, success criteria, and planned work.
- `.planning/REQUIREMENTS.md` - `CLI-01` through `CLI-05` plus `QUAL-01` and
  `QUAL-02`, which define the acceptance surface for this phase.
- `.planning/PROJECT.md` - Project constraints: librime as compatibility oracle,
  typed Rust architecture, frontend validation caveat, and module-boundary rule.

### Compatibility Strategy
- `docs/analysis.md` - Rationale for the CLI frontend surrogate and the
  distinction between ABI-surrogate validation and native frontend proof.
- `docs/roadmap.md` - Current "Next" guidance for building `yune-cli` into a
  RIME API-backed frontend surrogate.
- `docs/refactor-plan.md` - Phase 4 CLI module ownership, refactor rules, and
  completion criteria for keeping `main.rs`/`lib.rs` as facades.

### Codebase Maps
- `.planning/codebase/STRUCTURE.md` - Current crate/module layout and where new
  CLI/RIME API code belongs.
- `.planning/codebase/TESTING.md` - Focused Rust test patterns, frontend-style
  function-table tests, temp runtime setup, and fixture conventions.
- `.planning/codebase/INTEGRATIONS.md` - RIME ABI function-table integration
  points, runtime paths, deployment/config inputs, and local storage behavior.
- `.planning/codebase/CONCERNS.md` - Relevant risks: placeholder CLI frontend,
  ABI ownership, global runtime state, resource path validation, and large
  frontend compatibility suites.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/yune-cli/src/args.rs`: Existing command parser to extend without
  mixing parsing into `main.rs`.
- `crates/yune-cli/src/render.rs`: Existing human output module; use it for
  interactive/frontend display instead of printing from ABI wrappers.
- `crates/yune-cli/src/transcript.rs`: Existing deterministic JSON serializer;
  extend or adapt it for ABI event transcripts.
- `crates/yune-cli/src/fixture.rs`: Existing fixture comparison flow to preserve
  while ABI-backed transcript checks are introduced.
- `crates/yune-cli/src/sample_core.rs`: Current core-backed sample runner;
  retain as compatibility scaffolding until the ABI path replaces its fixture
  role.
- `crates/yune-cli/src/rime_frontend.rs`: Placeholder module intended to own
  RIME API setup, session lifecycle, key processing, and ABI state reads.
- `crates/yune-rime-api/src/api_table.rs`: Builds the `RimeApi` function table
  that the CLI surrogate should call.
- `crates/yune-rime-api/tests/frontend_client.rs`: Existing function-table
  client patterns for setup, temp runtime data, schema lists, modules, levers,
  context/status/commit reads, and cleanup.

### Established Patterns
- ABI-facing code validates pointers, initializes versioned structs with
  positive `data_size`, and pairs caller-owned allocations with matching free
  functions.
- Runtime/session tests serialize process-wide global state with a test guard
  and use unique temp directories for shared/user/staging data.
- CLI code returns `Result<(), String>` at the boundary and keeps stdout/stderr
  only in `main.rs`/rendering paths.
- Deterministic fixture output is handcrafted JSON with stable ordering rather
  than serde-driven output.
- New compatibility slices should choose owning implementation module, owning
  test module, and librime comparison target before code changes.

### Integration Points
- `crates/yune-cli/Cargo.toml` currently depends only on `yune-core`; the
  frontend surrogate will need to integrate `yune-rime-api`.
- `crates/yune-cli/src/main.rs` dispatches parsed commands and should remain
  thin.
- `RimeSetup`/`RimeInitialize`, `RimeDeployWorkspace`/schema deployment,
  `RimeSelectSchema`, `RimeCreateSession`, `RimeProcessKey`, `RimeGetCommit`,
  `RimeGetContext`, `RimeGetStatus`, and their free/cleanup counterparts are
  the core ABI path for this phase.
- `crates/yune-rime-api/src/context_api.rs`, `session.rs`,
  `schema_selection.rs`, and `deployment.rs` define the session and state-read
  behavior the CLI must exercise.

</code_context>

<specifics>
## Specific Ideas

No user-provided product examples were supplied in this auto run. The decisions
above are based on the roadmap, requirements, current code shape, and existing
project docs.

</specifics>

<deferred>
## Deferred Ideas

- Native frontend validation with Squirrel, Weasel, ibus-rime, fcitx-rime, or
  fcitx5-rime belongs to Phase 2.
- A graphical end-user frontend remains out of scope for the current milestone.
- AI-native candidate/provider/ranking/context behavior belongs to the future
  AI-native input layer milestone.
- Compiled dictionary payload consumption and user dictionary storage
  compatibility belong to later phases.

</deferred>

---

*Phase: 01-cli-frontend-surrogate*
*Context gathered: 2026-04-29*
