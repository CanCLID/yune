# Phase 1: CLI Frontend Surrogate - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution
> agents. Decisions are captured in CONTEXT.md - this log preserves the
> alternatives considered.

**Date:** 2026-04-29
**Phase:** 1-CLI Frontend Surrogate
**Areas discussed:** Command surface and runtime inputs, ABI lifecycle and
ownership, Transcript and rendering contract, Test ownership and validation

---

## Command Surface And Runtime Inputs

| Option | Description | Selected |
|--------|-------------|----------|
| Explicit ABI frontend path | Add ABI-backed commands while retaining current core fixtures until deterministic replacement. | yes |
| Immediate replacement | Change existing `run`/`check` behavior to ABI mode immediately. | |
| Separate binary | Create a second CLI binary for ABI frontend behavior. | |

**User's choice:** Auto-selected recommended default: explicit ABI frontend path.
**Notes:** The current CLI path is core-backed and fixtures already depend on it,
so the safer phase decision is to add the ABI path explicitly before replacing
fixture behavior.

| Option | Description | Selected |
|--------|-------------|----------|
| Explicit runtime dirs | Require shared/user data directories for ABI commands, with optional prebuilt/staging/log dirs. | yes |
| Implicit defaults | Let the CLI infer RIME paths from process state or host defaults. | |
| Environment variables only | Configure the ABI frontend entirely through environment variables. | |

**User's choice:** Auto-selected recommended default: explicit runtime dirs.
**Notes:** This directly satisfies `CLI-01` and keeps transcript runs
deterministic.

---

## ABI Lifecycle And Ownership

| Option | Description | Selected |
|--------|-------------|----------|
| `RimeApi` function table | Use `rime_get_api` and call through the frontend-style function table. | yes |
| Direct exported `Rime*` calls | Call individual exported symbols directly from Rust. | |
| Direct `yune-core` calls | Continue bypassing the ABI with direct core engine use. | |

**User's choice:** Auto-selected recommended default: `RimeApi` function table.
**Notes:** This mirrors `crates/yune-rime-api/tests/frontend_client.rs` and
keeps the CLI closer to actual frontend integration behavior.

| Option | Description | Selected |
|--------|-------------|----------|
| Focused `rime_frontend.rs` wrappers | Isolate unsafe ABI pointers and free-call pairing in the reserved module. | yes |
| Inline in `main.rs` | Put ABI setup and reads directly in CLI orchestration. | |
| Spread across rendering/transcript modules | Let render/transcript code dereference ABI structures directly. | |

**User's choice:** Auto-selected recommended default: focused
`rime_frontend.rs` wrappers.
**Notes:** This preserves the refactor rule that `main.rs` remains glue and
keeps ABI ownership separate from presentation.

---

## Transcript And Rendering Contract

| Option | Description | Selected |
|--------|-------------|----------|
| Event-by-event state | Record ABI-visible state after every processed key. | yes |
| Final snapshot only | Keep the current final-state fixture shape. | |
| Human output only | Use CLI display output as the only replay artifact. | |

**User's choice:** Auto-selected recommended default: event-by-event state.
**Notes:** Phase 1 needs frontend-surrogate validation, so replay must expose
state transitions rather than only the final engine snapshot.

| Option | Description | Selected |
|--------|-------------|----------|
| Full ABI-visible transcript fields | Include processed key/result, commits, composition, menu, candidates, labels, and status. | yes |
| Minimal current fixture fields | Preserve only the fields currently emitted by the core-backed fixture output. | |
| Raw debug dump | Serialize ABI structs without a stable comparison contract. | |

**User's choice:** Auto-selected recommended default: full ABI-visible
transcript fields.
**Notes:** This captures the Phase 1 requirements for commit text, preedit,
candidate page, highlight index, and status after each key event.

---

## Test Ownership And Validation

| Option | Description | Selected |
|--------|-------------|----------|
| Existing focused modules | Keep parsing, ABI lifecycle, rendering, transcript, and fixture behavior in their prepared modules. | yes |
| Single CLI implementation file | Add the frontend surrogate mostly in `main.rs`. | |
| New nested frontend crate | Add a new crate before the CLI surface requires one. | |

**User's choice:** Auto-selected recommended default: existing focused modules.
**Notes:** This follows `docs/refactor-plan.md` and prevents `main.rs` from
becoming the new compatibility catch-all.

| Option | Description | Selected |
|--------|-------------|----------|
| Focused CLI and function-table validation | Add CLI tests plus frontend-style ABI tests using temp runtime data. | yes |
| Only unit tests | Test parser/output helpers without driving the ABI lifecycle. | |
| Only manual CLI checks | Rely on operator usage rather than deterministic tests. | |

**User's choice:** Auto-selected recommended default: focused CLI and
function-table validation.
**Notes:** This is the smallest validation layer that proves the surrogate uses
the ABI path while keeping native frontend validation for Phase 2.

---

## the agent's Discretion

- Exact command names and flag spelling.
- Fixture file naming and transcript file organization.
- Internal wrapper type names.
- Whether deploy is a default step in one convenience command or an explicit
  subcommand, as long as runtime paths are explicit and transcript replay is
  deterministic.

## Deferred Ideas

- Native frontend validation with Squirrel, Weasel, ibus-rime, fcitx-rime, or
  fcitx5-rime belongs to Phase 2.
- A graphical end-user frontend remains out of scope for this milestone.
- AI-native provider/ranking/context behavior belongs to a future product
  milestone.
- Compiled dictionary payload consumption and user dictionary storage
  compatibility belong to later phases.
