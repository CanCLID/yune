# Phase 01: CLI Frontend Surrogate - Research

**Researched:** 2026-04-29
**Domain:** Rust CLI orchestration over a librime-shaped C ABI frontend surrogate [VERIFIED: .planning/ROADMAP.md; .planning/REQUIREMENTS.md; crates/yune-cli/src/main.rs; crates/yune-rime-api/src/api_table.rs]
**Confidence:** HIGH [VERIFIED: .planning/ROADMAP.md; .planning/REQUIREMENTS.md; crates/yune-cli/src/main.rs; crates/yune-rime-api/tests/frontend_client.rs]

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Add an explicit ABI-backed frontend path in `yune-cli` rather than silently replacing the current core-backed `run`/`check` behavior in the first implementation step. Keep the existing core fixture path available until ABI transcript checks provide a deterministic replacement.
- **D-02:** ABI/frontend commands must accept explicit `shared_data_dir` and `user_data_dir` inputs. Optional prebuilt, staging, and log directory flags may map directly onto `RimeTraits` when needed.
- **D-03:** Schema deployment and schema selection should be explicit operations in the frontend path. Hidden process-global RIME paths should not be required for deterministic CLI runs.

### ABI Lifecycle And Ownership
- **D-04:** The CLI frontend surrogate should drive `yune-rime-api` through the exported `RimeApi` function table from `rime_get_api`, matching the frontend-style test shape instead of calling `yune-core` directly.
- **D-05:** Unsafe pointer conversion, C string handling, ABI struct initialization, and allocation/free pairing belong in focused wrappers in `crates/yune-cli/src/rime_frontend.rs`.
- **D-06:** The frontend run lifecycle should initialize/setup traits, deploy or select schema as requested, create a session, process each key through `process_key`, read commit/context/status after each event, destroy the session, and clean up/finalize on every path that creates process-wide state.

### Transcript And Rendering Contract
- **D-07:** Transcript replay output should record ABI-visible state after each key event, not only a final snapshot. This makes frontend-surrogate behavior comparable event by event against librime.
- **D-08:** Deterministic transcript JSON should include the processed key and result, commits drained after the event, input/caret/preedit, candidate page fields, page/highlight metadata, select keys/labels when available, and status flags.
- **D-09:** Human-readable rendering and deterministic JSON serialization should stay separate: `render.rs` for operator-facing output, `transcript.rs` for the comparison contract.

### Test Ownership And Validation
- **D-10:** Keep `main.rs` as orchestration glue. Put command parsing in `args.rs`, ABI frontend lifecycle in `rime_frontend.rs`, human output in `render.rs`, deterministic transcript serialization in `transcript.rs`, and retained core fixture compatibility in `sample_core.rs`/`fixture.rs`.
- **D-11:** Add focused CLI tests for command parsing, transcript generation, and fixture comparison, plus ABI/frontend-style tests that use the same function-table and temp-runtime patterns as `crates/yune-rime-api/tests/frontend_client.rs`.
- **D-12:** Every behavior slice added in this phase needs an owning implementation module, owning test module, and explicit librime comparison target before implementation. Do not move unrelated code or split large tests mechanically unless this phase's transcript/replay design exposes a real ownership boundary.

### Compatibility Framing
- **D-13:** librime remains the external behavior oracle for user-visible transcript semantics, schema behavior, ABI lifecycle expectations, and migration-sensitive behavior.
- **D-14:** The CLI frontend is an intermediate validation layer. It should make ABI gaps measurable and scriptable, but Phase 2 remains responsible for native frontend-like loading paths and real frontend validation.

### the agent's Discretion
- Exact subcommand names, flag spelling, fixture naming, wrapper struct names, and small formatting details are left to the planner/executor, provided the decisions above remain true.
- The planner may decide whether deploy is a default step in a convenience command or an explicit subcommand, as long as transcript replay remains deterministic and runtime paths are explicit.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CLI-01 | Developer can initialize `yune-rime-api` from `yune-cli` with explicit shared data and user data directories. | CLI lifecycle, `RimeTraits`, runtime path handling, and `rime_get_api` setup ordering [VERIFIED: .planning/REQUIREMENTS.md; crates/yune-rime-api/src/runtime.rs; crates/yune-rime-api/tests/frontend_client.rs] |
| CLI-02 | Developer can deploy and select schemas through the CLI using the RIME ABI path, not direct `yune-core` fixture setup. | Deployment and schema-selection entry points, plus the current core-only CLI gap [VERIFIED: .planning/REQUIREMENTS.md; crates/yune-rime-api/src/deployment.rs; crates/yune-rime-api/src/schema_selection.rs; crates/yune-cli/src/main.rs] |
| CLI-03 | Developer can create and destroy RIME sessions from the CLI and process interactive key events through `RimeProcessKey`. | Session registry lifecycle, key dispatch, and session cleanup helpers [VERIFIED: .planning/REQUIREMENTS.md; crates/yune-rime-api/src/session.rs; crates/yune-rime-api/src/lib.rs] |
| CLI-04 | Developer can render commit text, preedit, candidate page, highlight index, and status after each CLI key event. | `RimeGetCommit`, `RimeGetContext`, `RimeGetStatus`, and transcript/render separation [VERIFIED: .planning/REQUIREMENTS.md; crates/yune-rime-api/src/context_api.rs; crates/yune-cli/src/transcript.rs; crates/yune-cli/src/render.rs] |
| CLI-05 | Developer can replay transcript key sequences through the RIME ABI and compare the transcript against expected output. | Existing fixture replay path, deterministic JSON serializer, and per-event ABI state capture [VERIFIED: .planning/REQUIREMENTS.md; crates/yune-cli/src/fixture.rs; crates/yune-cli/src/transcript.rs; .planning/ROADMAP.md] |
| QUAL-01 | Every new compatibility slice starts with an owning implementation module, owning test module, and explicit librime comparison target. | Existing phase and refactor rules already require module/test ownership [VERIFIED: .planning/REQUIREMENTS.md; .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md; docs/refactor-plan.md] |
| QUAL-02 | `lib.rs` and `main.rs` remain facades/orchestration glue; temporary spike code is extracted before a second related behavior lands. | Current CLI and ABI crate shapes already keep orchestration thin or intended-to-be-thin [VERIFIED: .planning/REQUIREMENTS.md; crates/yune-cli/src/main.rs; crates/yune-rime-api/src/lib.rs; docs/refactor-plan.md] |
</phase_requirements>

## Summary

Phase 1 should implement a new ABI-backed CLI path, not replace the existing core-backed fixture runner in place. The current CLI still routes `run` and `check` through `sample_core` and `fixture.rs`, while `crates/yune-cli/src/rime_frontend.rs` is only a placeholder; that means the phase work is to add a separate frontend-surrogate execution path that uses `rime_get_api` and `RimeApi` rather than `yune-core` directly [VERIFIED: crates/yune-cli/src/main.rs; crates/yune-cli/src/sample_core.rs; crates/yune-cli/src/rime_frontend.rs; crates/yune-rime-api/src/api_table.rs].

The phase should treat setup, deployment, schema selection, session lifecycle, per-key processing, and per-event state reads as one explicit lifecycle. The strongest evidence for that lifecycle comes from the existing frontend-style integration client and the ABI entrypoints themselves: `RimeSetup`/`RimeInitialize`, `RimeDeployWorkspace` or `RimeDeploySchema`, `RimeSelectSchema`, `RimeCreateSession`, `RimeProcessKey`, `RimeGetCommit`, `RimeGetContext`, `RimeGetStatus`, `RimeDestroySession`, and `RimeFinalize` are all already present and tested in the RIME API crate [VERIFIED: crates/yune-rime-api/src/runtime.rs; crates/yune-rime-api/src/deployment.rs; crates/yune-rime-api/src/schema_selection.rs; crates/yune-rime-api/src/session.rs; crates/yune-rime-api/src/lib.rs; crates/yune-rime-api/tests/frontend_client.rs].

The transcript contract also needs to change from a final-snapshot fixture to a per-event replay record. The current CLI serializer emits one final `FixtureOutput` with `schema_id`, `sequence`, `commits`, one `Snapshot`, and `Status`, while the phase decisions require recording ABI-visible state after each key event, including commit drains, preedit, highlighted candidate, candidate page metadata, select labels/keys, and status flags [VERIFIED: crates/yune-cli/src/transcript.rs; .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md].

**Primary recommendation:** implement a narrow `rime_frontend.rs` wrapper around `rime_get_api`, keep the old core-backed runner as a fallback path, and make transcript replay capture ABI state after every key event so the CLI can compare directly against librime-observable behavior [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md; crates/yune-cli/src/main.rs; crates/yune-rime-api/tests/frontend_client.rs].

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| CLI command parsing and dispatch | Browser / Client equivalent: `yune-cli` binary [VERIFIED: crates/yune-cli/src/main.rs; crates/yune-cli/src/args.rs] | API / Backend: `yune-rime-api` function table [VERIFIED: crates/yune-rime-api/src/api_table.rs] | The CLI owns user-facing flags and command dispatch, while the ABI crate owns the state machine the CLI must drive [VERIFIED: crates/yune-cli/src/main.rs; crates/yune-cli/src/args.rs; crates/yune-rime-api/src/api_table.rs]. |
| Runtime setup and path resolution | API / Backend: `yune-rime-api::runtime` [VERIFIED: crates/yune-rime-api/src/runtime.rs] | CLI / Client: `rime_frontend.rs` wrapper [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md] | `RimeSetup` stores process-wide runtime traits and derives directories from `RimeTraits`; the CLI should only supply those inputs and report errors [VERIFIED: crates/yune-rime-api/src/runtime.rs]. |
| Schema deployment and selection | API / Backend: `yune-rime-api::deployment` and `schema_selection` [VERIFIED: crates/yune-rime-api/src/deployment.rs; crates/yune-rime-api/src/schema_selection.rs] | CLI / Client: explicit command flow [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md] | The ABI crate already owns deploy/select behavior; the CLI should invoke those calls explicitly instead of fabricating a core-only shortcut [VERIFIED: crates/yune-rime-api/src/deployment.rs; crates/yune-rime-api/src/schema_selection.rs]. |
| Session lifecycle and key processing | API / Backend: `yune-rime-api::session` and `RimeProcessKey` [VERIFIED: crates/yune-rime-api/src/session.rs; crates/yune-rime-api/src/lib.rs] | CLI / Client: event loop and cleanup sequencing [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md] | Session creation, lookup, cleanup, and the key-processing dispatch already live in the ABI crate, so the CLI should orchestrate rather than reimplement them [VERIFIED: crates/yune-rime-api/src/session.rs; crates/yune-rime-api/src/lib.rs]. |
| Commit/context/status rendering | CLI / Client: transcript and human render modules [VERIFIED: crates/yune-cli/src/transcript.rs; crates/yune-cli/src/render.rs] | API / Backend: context/status/commit C APIs [VERIFIED: crates/yune-rime-api/src/context_api.rs] | The CLI owns presentation and deterministic serialization, while the ABI crate owns the data to present [VERIFIED: crates/yune-cli/src/transcript.rs; crates/yune-cli/src/render.rs; crates/yune-rime-api/src/context_api.rs]. |
| Fixture comparison and replay | CLI / Client: `fixture.rs` and transcript replay [VERIFIED: crates/yune-cli/src/fixture.rs; crates/yune-cli/src/transcript.rs] | API / Backend: per-event state reads [VERIFIED: crates/yune-rime-api/src/context_api.rs] | Existing fixture comparison already lives in the CLI, so the ABI-backed replay should plug into that contract rather than creating a second comparison path [VERIFIED: crates/yune-cli/src/fixture.rs; crates/yune-cli/src/transcript.rs]. |

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Rust / Cargo workspace | `1.95.0` toolchain in this environment; workspace MSRV is `1.76` [VERIFIED: rustc --version; cargo --version; Cargo.toml] | Build and test the workspace, CLI, and ABI crates [VERIFIED: Cargo.toml] | This is the project language/runtime and the only build system present [VERIFIED: Cargo.toml; .planning/codebase/STRUCTURE.md]. |
| `yune-cli` | `0.1.0` [VERIFIED: crates/yune-cli/Cargo.toml; Cargo.lock] | CLI orchestration, command parsing, human output, transcript replay, and fixture comparison [VERIFIED: crates/yune-cli/src/main.rs; crates/yune-cli/src/fixture.rs; crates/yune-cli/src/transcript.rs; crates/yune-cli/src/render.rs] | Phase 1 owns the CLI surrogate, so this crate is the primary implementation target [VERIFIED: .planning/ROADMAP.md; .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md]. |
| `yune-rime-api` | `0.1.0` [VERIFIED: crates/yune-rime-api/Cargo.toml; Cargo.lock] | Librime-shaped C ABI, runtime setup, deployment, schema selection, session lifecycle, and context/status/commit reads [VERIFIED: crates/yune-rime-api/src/api_table.rs; crates/yune-rime-api/src/runtime.rs; crates/yune-rime-api/src/deployment.rs; crates/yune-rime-api/src/context_api.rs] | The surrogate must exercise the actual ABI surface to stay comparable with frontend-style tests and librime [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md; crates/yune-rime-api/tests/frontend_client.rs]. |
| `yune-core` | `0.1.0` [VERIFIED: Cargo.lock; .planning/codebase/STRUCTURE.md] | Retained core-backed fixture runner and deterministic sample engine path [VERIFIED: crates/yune-cli/src/sample_core.rs; crates/yune-cli/src/fixture.rs] | The core path remains as a compatibility fallback until ABI transcript checks are deterministic [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md]. |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `serde_yaml` | `0.9.34+deprecated` [VERIFIED: Cargo.lock] | RIME config/runtime YAML parsing and writing in the ABI crate [VERIFIED: crates/yune-rime-api/src/runtime.rs; crates/yune-rime-api/src/config_api.rs; crates/yune-rime-api/src/deployment.rs] | Use whenever the CLI surrogate needs to prepare or inspect RIME YAML-backed state through the ABI layer [VERIFIED: crates/yune-rime-api/src/runtime.rs]. |
| `regex` | `1.12.3` [VERIFIED: Cargo.lock] | Pattern handling inside the ABI/runtime and core stack [VERIFIED: .planning/codebase/STRUCTURE.md; .planning/codebase/STACK.md] | Use indirectly through the ABI/core crates; do not add new regex-based CLI parsing unless the existing code path requires it [VERIFIED: .planning/codebase/STACK.md]. |
| `libc` | `0.2.186` [VERIFIED: Cargo.lock] | Unix-specific ABI compatibility helpers and version-signature formatting [VERIFIED: .planning/codebase/STACK.md; crates/yune-rime-api/src/lib.rs] | Keep as a transitive ABI dependency; the CLI should not depend on it directly unless the wrapper needs FFI utilities [VERIFIED: crates/yune-rime-api/src/lib.rs]. |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| ABI-backed CLI surrogate | Keep the existing `yune-core` fixture runner as the only CLI path [VERIFIED: crates/yune-cli/src/main.rs; crates/yune-cli/src/sample_core.rs] | Easier short-term, but it does not exercise the ABI lifecycle, schema selection, or frontend-style state reads that Phase 1 is supposed to validate [VERIFIED: .planning/ROADMAP.md; .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md]. |
| Explicit setup/select commands | Hide deployment and schema selection behind process-global defaults [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md] | Faster to script, but it breaks deterministic CLI runs and weakens the ABI comparison target [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md]. |
| Final snapshot JSON only | Capture ABI-visible state after each key event [VERIFIED: crates/yune-cli/src/transcript.rs; .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md] | Simpler serializer, but it cannot compare event-by-event behavior against librime and misses per-event drift [VERIFIED: crates/yune-cli/src/transcript.rs; .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md]. |

**Installation:**
```bash
cargo add yune-rime-api --path crates/yune-rime-api
```
[VERIFIED: crates/yune-cli/Cargo.toml; .planning/ROADMAP.md]

## Architecture Patterns

### System Architecture Diagram

```text
CLI args / transcript replay
  -> crates/yune-cli/src/args.rs
  -> crates/yune-cli/src/rime_frontend.rs
     -> rime_get_api() / RimeApi function table
     -> RimeSetup / RimeInitialize
     -> RimeDeployWorkspace or RimeDeploySchema
     -> RimeSelectSchema
     -> RimeCreateSession
     -> per-key RimeProcessKey loop
        -> RimeGetCommit / RimeGetContext / RimeGetStatus
        -> transcript.rs deterministic JSON
        -> render.rs human output
     -> RimeDestroySession
     -> RimeFinalize
  -> fixture.rs expected-vs-actual comparison
  -> stdout / stderr
```
[VERIFIED: crates/yune-cli/src/main.rs; crates/yune-cli/src/fixture.rs; crates/yune-cli/src/transcript.rs; crates/yune-cli/src/render.rs; crates/yune-rime-api/src/api_table.rs; crates/yune-rime-api/src/runtime.rs; crates/yune-rime-api/src/deployment.rs; crates/yune-rime-api/src/schema_selection.rs; crates/yune-rime-api/src/session.rs; crates/yune-rime-api/src/context_api.rs]

### Recommended Project Structure

```text
crates/yune-cli/src/
├── args.rs          # command parsing and flag validation [VERIFIED: crates/yune-cli/src/args.rs]
├── rime_frontend.rs # ABI-backed setup, deploy/select, session lifecycle, key loop [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md]
├── render.rs        # human-facing event rendering [VERIFIED: crates/yune-cli/src/render.rs]
├── transcript.rs    # deterministic JSON transcript contract [VERIFIED: crates/yune-cli/src/transcript.rs]
├── sample_core.rs   # retained core-backed fixture path [VERIFIED: crates/yune-cli/src/sample_core.rs]
├── fixture.rs       # fixture comparison/replay harness [VERIFIED: crates/yune-cli/src/fixture.rs]
└── main.rs          # orchestration glue only [VERIFIED: crates/yune-cli/src/main.rs]
```

### Pattern 1: Explicit ABI Lifecycle Wrapper
**What:** Put all unsafe pointer conversion, C string conversion, function-table lookup, and free pairing in one focused wrapper module [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md; crates/yune-rime-api/src/ffi_memory.rs].
**When to use:** Whenever the CLI needs to call `RimeSetup`, `RimeDeployWorkspace`, `RimeSelectSchema`, `RimeCreateSession`, `RimeProcessKey`, `RimeGetCommit`, `RimeGetContext`, `RimeGetStatus`, and the matching destroy/free functions [VERIFIED: crates/yune-rime-api/src/runtime.rs; crates/yune-rime-api/src/deployment.rs; crates/yune-rime-api/src/schema_selection.rs; crates/yune-rime-api/src/session.rs; crates/yune-rime-api/src/context_api.rs; crates/yune-rime-api/src/ffi_memory.rs].
**Example:**
```rust
// Source: [VERIFIED: crates/yune-rime-api/tests/frontend_client.rs] + [VERIFIED: crates/yune-rime-api/src/lib.rs]
let api = unsafe { &*rime_get_api() };
let setup = api.setup.expect("setup is required");
let create_session = api.create_session.expect("session creation is required");
let process_key = api.process_key.expect("key processing is required");
let get_commit = api.get_commit.expect("commit reads are required");
let get_context = api.get_context.expect("context reads are required");
let get_status = api.get_status.expect("status reads are required");

unsafe { setup(&traits) };
unsafe { (api.initialize.expect("initialize is required"))(&traits) };
let session_id = create_session();
unsafe { (api.select_schema.expect("schema selection is required"))(session_id, schema_id_ptr) };
let accepted = process_key(session_id, keycode, mask);
unsafe { get_commit(session_id, &mut commit) };
unsafe { get_context(session_id, &mut context) };
unsafe { get_status(session_id, &mut status) };
unsafe { (api.destroy_session.expect("destroy is required"))(session_id) };
(api.finalize.expect("finalize is required"))();
```

### Pattern 2: Per-Event Transcript Capture
**What:** Record the processed key, acceptance result, drained commit text, candidate page data, preedit/input/caret, and status after every key event instead of only at the end [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md].
**When to use:** For replay mode, fixture comparison, regression capture, and librime comparison of CLI-visible state [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md; crates/yune-cli/src/transcript.rs; crates/yune-cli/src/fixture.rs].
**Example:**
```rust
// Source: [VERIFIED: crates/yune-cli/src/transcript.rs] + [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md]
// One event record should contain:
// - key / result
// - commit(s) drained after the key
// - input, caret, preedit
// - candidate page fields and highlight index
// - select keys / select labels when available
// - status flags
```

### Pattern 3: Dual-Path CLI
**What:** Keep the old core-backed runner intact while adding a separate ABI-backed frontend surrogate path [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md; crates/yune-cli/src/main.rs; crates/yune-cli/src/sample_core.rs].
**When to use:** During the first phase when ABI transcript checks are still being established and the fallback path is still needed for deterministic fixtures [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md].
**Example:**
```rust
// Source: [VERIFIED: crates/yune-cli/src/main.rs]
match Command::parse(&args)? {
    Command::Run { .. } => {
        // Phase 1 should route one subcommand through rime_frontend.rs
        // while keeping sample_core.rs available as a compatibility path.
    }
    Command::Check { .. } => {
        // Keep fixture comparison separate from the ABI wrapper.
    }
    Command::Help => {}
}
```

### Anti-Patterns to Avoid
- **Mixing the ABI path into `main.rs`:** it keeps unsafe lifecycle handling and parsing glue in one file and makes review harder [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md; docs/refactor-plan.md].
- **Calling `yune-core` directly for the surrogate path:** it bypasses the ABI surface that Phase 1 is supposed to validate [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md; crates/yune-cli/src/sample_core.rs].
- **Serializing only a final snapshot:** it loses event-by-event comparison against librime [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md; crates/yune-cli/src/transcript.rs].
- **Hiding runtime directories behind defaults:** it makes the CLI non-deterministic and undermines explicit setup requirements [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md; crates/yune-rime-api/src/runtime.rs].

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| ABI lifecycle orchestration | Ad hoc direct calls into `yune-core` from the CLI | `rime_get_api` plus a focused `rime_frontend.rs` wrapper [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md; crates/yune-rime-api/src/api_table.rs] | The ABI layer already defines the compatibility contract and ownership boundaries [VERIFIED: crates/yune-rime-api/src/api_table.rs; crates/yune-rime-api/src/ffi_memory.rs]. |
| Unsafe pointer and C string handling | Scatter `unsafe` conversions across `main.rs`, `render.rs`, or `fixture.rs` | Centralize FFI handling in `rime_frontend.rs` and reuse existing ABI cleanup patterns [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md; crates/yune-rime-api/src/ffi_memory.rs] | Keeps lifetime and free-pairing rules auditable in one place [VERIFIED: crates/yune-rime-api/src/ffi_memory.rs]. |
| Transcript serialization | Hand-roll a second JSON serializer in the CLI entry point | Extend `transcript.rs` as the deterministic comparison contract [VERIFIED: crates/yune-cli/src/transcript.rs; .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md] | The current serializer already enforces field order and stable formatting [VERIFIED: crates/yune-cli/src/transcript.rs]. |
| Fixture replay and comparison | Recreate fixture parsing/comparison logic in the ABI wrapper | Keep `fixture.rs` as the compare harness and feed it the replay transcript [VERIFIED: crates/yune-cli/src/fixture.rs; crates/yune-cli/src/transcript.rs] | Prevents duplication and preserves the existing checked-in fixture contract [VERIFIED: crates/yune-cli/src/fixture.rs]. |
| Session and runtime cleanup | Rely on implicit process teardown | Call `RimeDestroySession`, `RimeFinalize`, and the existing cleanup/reset helpers explicitly [VERIFIED: crates/yune-rime-api/src/session.rs; crates/yune-rime-api/src/deployment.rs; crates/yune-rime-api/src/runtime.rs] | The ABI crate keeps process-wide state in mutex-protected globals that must be reset deliberately [VERIFIED: crates/yune-rime-api/src/session.rs; crates/yune-rime-api/src/runtime.rs]. |

**Key insight:** the CLI surrogate is a compatibility harness, not a second engine. The more logic that stays in the ABI wrapper and existing transcript/render/fixture modules, the easier it is to compare against librime and to keep `main.rs` thin [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md; docs/refactor-plan.md].

## Common Pitfalls

### Pitfall 1: Process-Wide State Leakage
**What goes wrong:** sessions, runtime paths, notifications, and module/state caches leak between runs or tests [VERIFIED: crates/yune-rime-api/src/session.rs; crates/yune-rime-api/src/runtime.rs; crates/yune-rime-api/src/notifications.rs; crates/yune-rime-api/src/modules.rs].
**Why it happens:** the ABI crate keeps these as process-wide singletons behind `OnceLock`/`Mutex` and an atomic service flag [VERIFIED: crates/yune-rime-api/src/session.rs; crates/yune-rime-api/src/runtime.rs; crates/yune-rime-api/src/api_table.rs].
**How to avoid:** initialize with explicit traits, destroy the session, and finalize/reset on every CLI run path [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md; crates/yune-rime-api/src/deployment.rs; crates/yune-rime-api/src/session.rs].
**Warning signs:** flaky tests, stale schema names, repeated notification events, or sessions that survive a supposed finalize [VERIFIED: crates/yune-rime-api/src/tests/session_api.rs; crates/yune-rime-api/src/tests/runtime.rs].

### Pitfall 2: Final-Snapshot-Only Replay
**What goes wrong:** the CLI compares only the terminal output and misses per-key drift [VERIFIED: crates/yune-cli/src/transcript.rs; .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md].
**Why it happens:** the current serializer and fixture harness were designed for the core-backed runner, not a per-event ABI transcript [VERIFIED: crates/yune-cli/src/transcript.rs; crates/yune-cli/src/fixture.rs].
**How to avoid:** emit event records after every key and include the drained commit plus current context/status data [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md].
**Warning signs:** a transcript that only changes at the end of input, or a fixture diff that cannot show which key changed behavior [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md; crates/yune-cli/src/fixture.rs].

### Pitfall 3: Implicit Runtime Paths
**What goes wrong:** the CLI quietly depends on process-global defaults instead of explicit shared/user directories [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md; crates/yune-rime-api/src/runtime.rs].
**Why it happens:** `RimeSetup` can derive defaults from `RimeTraits`, and existing code will happily fall back when fields are omitted [VERIFIED: crates/yune-rime-api/src/runtime.rs].
**How to avoid:** require explicit `shared_data_dir` and `user_data_dir` inputs for the surrogate path and map optional prebuilt/staging/log paths deliberately [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md; crates/yune-rime-api/src/runtime.rs].
**Warning signs:** CLI runs that differ based on prior process state or machine-local directories [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md].

### Pitfall 4: Ownership Mismatch at the FFI Boundary
**What goes wrong:** context, status, commit, candidate-list, or schema-list memory is freed with the wrong helper or not freed at all [VERIFIED: crates/yune-rime-api/src/ffi_memory.rs; crates/yune-rime-api/src/context_api.rs; crates/yune-rime-api/src/candidate_api.rs].
**Why it happens:** the ABI uses `CString::into_raw`, `Box::into_raw`, and `Vec::from_raw_parts` on caller-owned storage [VERIFIED: crates/yune-rime-api/src/ffi_memory.rs; crates/yune-rime-api/src/context_api.rs; crates/yune-rime-api/src/candidate_api.rs].
**How to avoid:** keep paired free functions close to the allocation sites and let the wrapper own the call ordering [VERIFIED: crates/yune-rime-api/src/ffi_memory.rs; .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md].
**Warning signs:** crashes after repeated replay, double-free symptoms, or fields that appear to remain populated after cleanup [VERIFIED: crates/yune-rime-api/src/ffi_memory.rs].

### Pitfall 5: Mixing Presentation With Comparison
**What goes wrong:** human output and deterministic transcript output diverge or become order-dependent [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md; crates/yune-cli/src/render.rs; crates/yune-cli/src/transcript.rs].
**Why it happens:** both outputs are tempting to assemble from the same event loop if modules are not separated [VERIFIED: crates/yune-cli/src/main.rs; crates/yune-cli/src/render.rs; crates/yune-cli/src/transcript.rs].
**How to avoid:** keep `render.rs` and `transcript.rs` separate and feed both from one shared event model [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md].
**Warning signs:** tests that only pass when stdout formatting and JSON field order happen to match [VERIFIED: crates/yune-cli/src/transcript.rs; crates/yune-cli/src/fixture.rs].

## Code Examples

Verified patterns from local sources:

### ABI Lifecycle Skeleton
```rust
// Source: [VERIFIED: crates/yune-rime-api/tests/frontend_client.rs] + [VERIFIED: crates/yune-rime-api/src/lib.rs]
let api = unsafe { &*rime_get_api() };
let setup = api.setup.expect("setup");
let initialize = api.initialize.expect("initialize");
let deploy_workspace = api.deploy.expect("deploy");
let select_schema = api.select_schema.expect("select_schema");
let create_session = api.create_session.expect("create_session");
let process_key = api.process_key.expect("process_key");
let get_commit = api.get_commit.expect("get_commit");
let get_context = api.get_context.expect("get_context");
let get_status = api.get_status.expect("get_status");
let destroy_session = api.destroy_session.expect("destroy_session");
let finalize = api.finalize.expect("finalize");

unsafe { setup(&traits) };
unsafe { initialize(&traits) };
assert!(unsafe { deploy_workspace() } != 0);
let session_id = create_session();
assert!(unsafe { select_schema(session_id, schema_ptr) } != 0);
assert!(process_key(session_id, keycode, mask) != 0);
unsafe { get_commit(session_id, &mut commit) };
unsafe { get_context(session_id, &mut context) };
unsafe { get_status(session_id, &mut status) };
assert!(destroy_session(session_id) != 0);
finalize();
```

### Deterministic Transcript Shape
```rust
// Source: [VERIFIED: crates/yune-cli/src/transcript.rs] + [VERIFIED: crates/yune-cli/src/fixture.rs]
// Current fixture output is a single final snapshot:
// schema_id, sequence, commits, context snapshot, and status.
// Phase 1 should extend this into per-event records while keeping
// deterministic field order and stable JSON formatting.
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Core-only CLI fixture runner | ABI-backed frontend surrogate plus retained fallback path [VERIFIED: crates/yune-cli/src/main.rs; crates/yune-cli/src/sample_core.rs; .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md] | Phase 1 planning [VERIFIED: .planning/ROADMAP.md; .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md] | The CLI can now validate the real ABI lifecycle instead of only the core engine path [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md]. |
| Final snapshot JSON | Per-event transcript replay [VERIFIED: crates/yune-cli/src/transcript.rs; .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md] | Phase 1 decision [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md] | Behavior differences become visible at the key-event boundary rather than only at the end of the sequence [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md]. |
| Implicit runtime defaults | Explicit `shared_data_dir` / `user_data_dir` inputs [VERIFIED: crates/yune-rime-api/src/runtime.rs; .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md] | Phase 1 decision [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md] | Deterministic CLI runs become possible without relying on hidden process-global state [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md]. |

**Deprecated/outdated:**
- `yune-cli` as a pure `yune-core` fixture runner is no longer the only CLI shape for this milestone; it becomes the retained compatibility path, not the surrogate path [VERIFIED: crates/yune-cli/src/main.rs; crates/yune-cli/src/sample_core.rs; .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md].

## Assumptions Log

> No claims in this research are tagged `[ASSUMED]`; all recommendations are grounded in the phase context, roadmap, codebase, or test artifacts [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md; .planning/ROADMAP.md; crates/yune-cli/src/main.rs; crates/yune-rime-api/tests/frontend_client.rs].

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| None | No assumed claims were needed. | N/A | N/A |

## Open Questions (RESOLVED)

1. **RESOLVED: What exact command surface should the ABI-backed surrogate expose?**
   - Resolution: use an explicit ABI-backed frontend command path while retaining existing core-backed `run`/`check` behavior. The planner may choose exact names, but the planned surface uses separate frontend execution and frontend fixture-check paths so deploy/select/runtime inputs stay deterministic [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md; .planning/phases/01-cli-frontend-surrogate/01-01-PLAN.md; .planning/phases/01-cli-frontend-surrogate/01-02-PLAN.md].
   - Outcome: deploy/select may be part of the frontend lifecycle or an explicit subcommand, but every deterministic run must accept explicit runtime directories and must not rely on hidden process-global defaults.

2. **RESOLVED: Should replay mode print human output as well as JSON?**
   - Resolution: deterministic replay comparison is JSON-only on stdout; human-readable rendering is a separate output mode/path that consumes the same owned frontend event data [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md; .planning/phases/01-cli-frontend-surrogate/01-UI-SPEC.md; .planning/phases/01-cli-frontend-surrogate/01-02-PLAN.md].
   - Outcome: successful machine-readable commands emit only JSON to stdout, and successful human-readable commands emit only plain transcript text to stdout. No single successful deterministic fixture path mixes both streams.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| `rustc` | Workspace build and CLI/ABI implementation | ✓ | `1.95.0` [VERIFIED: rustc --version] | — |
| `cargo` | Build, test, and lint commands | ✓ | `1.95.0` [VERIFIED: cargo --version] | — |
| `rg` | Fast codebase inspection during planning/execution | ✓ | `15.1.0` [VERIFIED: rg --version] | `grep` / `find` |

**Missing dependencies with no fallback:**
- None [VERIFIED: rustc --version; cargo --version].

**Missing dependencies with fallback:**
- None [VERIFIED: rustc --version; cargo --version].

## Validation Architecture

> Skipped: `.planning/config.json` sets `workflow.nyquist_validation` to `false` [VERIFIED: .planning/config.json].

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no | No auth boundary exists in this phase; the CLI and ABI calls are local and in-process [VERIFIED: .planning/codebase/INTEGRATIONS.md]. |
| V3 Session Management | yes | Process-wide session lifecycle in `session.rs`, with explicit create/destroy/finalize/reset sequencing [VERIFIED: crates/yune-rime-api/src/session.rs; crates/yune-rime-api/src/deployment.rs]. |
| V4 Access Control | no | There is no role-based or user-authz model in the current CLI/ABI surrogate [VERIFIED: .planning/codebase/INTEGRATIONS.md]. |
| V5 Input Validation | yes | Validate CLI flags, schema IDs, runtime resource IDs, C strings, and replay input before crossing the ABI boundary [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md; .planning/codebase/CONCERNS.md]. |
| V6 Cryptography | no | This phase does not introduce cryptographic operations [VERIFIED: .planning/ROADMAP.md; .planning/REQUIREMENTS.md]. |

### Known Threat Patterns for Rust CLI + ABI Surrogate

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Runtime resource ID path traversal | Tampering | Treat schema/config/userdb identifiers as logical IDs and reject separators, `..`, absolute paths, drive prefixes, and similar path syntax before joining [VERIFIED: .planning/codebase/CONCERNS.md; .planning/REQUIREMENTS.md]. |
| FFI ownership/lifetime mismatch | Tampering / DoS | Keep allocation/free pairs centralized and never duplicate cleanup logic across the CLI wrapper [VERIFIED: crates/yune-rime-api/src/ffi_memory.rs]. |
| Process-wide state leakage | DoS / Tampering | Reset sessions, runtime traits, and notification state explicitly between runs and tests [VERIFIED: crates/yune-rime-api/src/session.rs; crates/yune-rime-api/src/runtime.rs; crates/yune-rime-api/src/notifications.rs]. |
| Unsafe C string handling | Tampering | Validate NUL-terminated inputs at the boundary and keep conversions in one wrapper module [VERIFIED: crates/yune-rime-api/src/runtime.rs; crates/yune-rime-api/src/context_api.rs; .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md]. |

## Sources

### Primary (HIGH confidence)
- `.planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md` - locked decisions, scope, and deferred ideas [VERIFIED: file].
- `.planning/ROADMAP.md` - phase goal, success criteria, and planned work [VERIFIED: file].
- `.planning/REQUIREMENTS.md` - CLI-01 through CLI-05 plus QUAL-01 and QUAL-02 [VERIFIED: file].
- `.planning/config.json` - validation toggle and workflow settings [VERIFIED: file].
- `.planning/codebase/STRUCTURE.md` - module ownership and file layout [VERIFIED: file].
- `.planning/codebase/TESTING.md` - test harness, fixture, and frontend-style test patterns [VERIFIED: file].
- `.planning/codebase/INTEGRATIONS.md` - ABI integration points and runtime storage behavior [VERIFIED: file].
- `.planning/codebase/CONCERNS.md` - CLI placeholder, ownership, state, and security risks [VERIFIED: file].
- `crates/yune-cli/src/main.rs` - current CLI orchestration path [VERIFIED: file].
- `crates/yune-cli/src/args.rs` - command parsing [VERIFIED: file].
- `crates/yune-cli/src/sample_core.rs` - retained core-backed fixture path [VERIFIED: file].
- `crates/yune-cli/src/fixture.rs` - existing replay/comparison harness [VERIFIED: file].
- `crates/yune-cli/src/transcript.rs` - deterministic JSON serializer [VERIFIED: file].
- `crates/yune-cli/src/render.rs` - human-facing output [VERIFIED: file].
- `crates/yune-cli/src/rime_frontend.rs` - placeholder for the ABI-backed frontend wrapper [VERIFIED: file].
- `crates/yune-rime-api/src/api_table.rs` - `rime_get_api` and function-table construction [VERIFIED: file].
- `crates/yune-rime-api/src/runtime.rs` - `RimeSetup` and runtime trait/path handling [VERIFIED: file].
- `crates/yune-rime-api/src/deployment.rs` - initialize/finalize/deploy/sync paths [VERIFIED: file].
- `crates/yune-rime-api/src/schema_selection.rs` - schema application and reset behavior [VERIFIED: file].
- `crates/yune-rime-api/src/session.rs` - session registry and lifecycle [VERIFIED: file].
- `crates/yune-rime-api/src/context_api.rs` - commit/context/status reads [VERIFIED: file].
- `crates/yune-rime-api/src/ffi_memory.rs` - ownership cleanup helpers [VERIFIED: file].
- `crates/yune-rime-api/tests/frontend_client.rs` - frontend-style function-table integration pattern [VERIFIED: file].
- `Cargo.toml` and `Cargo.lock` - workspace metadata and locked dependency versions [VERIFIED: files].

### Secondary (MEDIUM confidence)
- None - no web/third-party sources were needed for this phase [VERIFIED: session tools and repo files].

### Tertiary (LOW confidence)
- None [VERIFIED: session tools and repo files].

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - versions and crate ownership were verified from the workspace manifests and lockfile [VERIFIED: Cargo.toml; Cargo.lock; crates/yune-cli/Cargo.toml; crates/yune-rime-api/Cargo.toml].
- Architecture: HIGH - phase decisions and current code shape agree on the same module boundaries [VERIFIED: .planning/phases/01-cli-frontend-surrogate/01-CONTEXT.md; crates/yune-cli/src/main.rs; crates/yune-rime-api/tests/frontend_client.rs].
- Pitfalls: HIGH - the risks are explicitly documented in the codebase concerns and confirmed by current file structure [VERIFIED: .planning/codebase/CONCERNS.md; crates/yune-cli/src/main.rs; crates/yune-cli/src/transcript.rs].

**Research date:** 2026-04-29
**Valid until:** 2026-05-29
