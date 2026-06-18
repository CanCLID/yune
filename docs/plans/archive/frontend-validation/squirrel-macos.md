# Squirrel/macOS Frontend Validation

> **Status:** Finished · **Milestone:** M8 / Phase 6 · **Closed:** 2026-05-01 · **Type:** validation record (archived)

## Target and plan decisions

This note records the Phase 06 D-04 Squirrel/macOS validation attempt after the
TypeDuck-Web path. It preserves the observed/source-modeled lifecycle evidence
per D-07 before any compatibility fix, and classifies the D-11 output as a
combination of a minimized call-sequence fixture and a documented blocker.

- Target: Squirrel/macOS native RIME frontend lifecycle shape.
- Source/version inspected or attempted: source-modeled lifecycle expectations
  from the public Squirrel/librime frontend shape, mapped to Yune-owned
  `RimeApi` calls.
- Reproduction artifact: `fixtures/frontend-traces/squirrel-lifecycle.json`.
- Regression coverage: `crates/yune-rime-api/tests/frontend_hosts/native_frontends.rs`.

## Local environment constraints

No checked-in regression test requires an installed Squirrel app bundle, Xcode UI
execution, input-method registration, macOS GUI automation, or a real user input
context. The direct app run remains a documented blocker because it needs local
packaging and OS input-method installation that is not deterministic in ordinary
Cargo test runs.

Attempted command status:

- Runnable regression command: `cargo test -p yune-rime-api --test frontend_hosts native_frontends -- --nocapture`.
- Direct Squirrel app command: not run as a mandatory gate; requires external
  app/input-method packaging and local registration outside the repository test
  trust boundary.

## ABI lifecycle call sequence

The preserved fixture models Squirrel/macOS at the RIME ABI boundary instead of
vendoring Squirrel source or bypassing through `yune-core`:

1. Resolve `rime_get_api` and validate the function-table data size.
2. Run app-level setup, deployer initialization, notification handler
   registration, initialize, maintenance/deploy, and join behavior.
3. Create a per-input-context session on focus-in.
4. Select the logical `squirrel_luna` schema.
5. Process key events through `RimeProcessKey`.
6. Read input, status, context, and commit through ABI calls.
7. Pair every successful status/context/commit read with `free_status`,
   `free_context`, and `free_commit`.
8. Replace and clear notification handlers, preserving observed option/schema
   notifications.
9. Clear composition on focus-out before destroying the input-context session.
10. Reject stale session use after destroy/finalize.
11. Exercise second input-context creation and teardown.
12. Run sync/reinitialize/finalize lifecycle checks.

## Expected behavior

Squirrel/macOS should be able to drive Yune through app-level setup and
initialization, per-input-context session create/destroy, schema selection, key
processing, context/status/commit reads, focus cleanup, notification handler
replacement, sync, reinitialize, and final teardown without relying on direct
`yune-core` calls or unchecked C pointers.

## Observed behavior or source-modeled gap

The minimized fixture completed through Yune's `RimeApi` with synthetic runtime
paths and logical resource IDs. No Yune ABI/runtime mismatch was found in this
source-modeled Squirrel lifecycle. The remaining gap is direct Squirrel bundle
execution: a real app run still needs external macOS input-method packaging and
local registration before it can be considered complete native frontend proof.

## Blocker and reproduction status

- Blocker status: documented blocker for direct Squirrel app execution.
- reproduction status: minimized call-sequence fixture plus documented blocker.
- D-11 output type: combination of minimized call-sequence fixture and documented
  blocker.
- D-07 preservation: the call sequence and blocker are checked in before any
  future fix is attempted.

## Sanitization

The note and fixture intentionally omit real user data directories, personal
paths, machine-specific identifiers, raw pointer values, timestamps, process IDs,
Cargo target paths, environment variables, and personal input data. The fixture
uses synthetic resource names and logical schema/userdata identifiers only.

## Out of scope

This plan does not claim complete native frontend compatibility, does not build a
new graphical frontend, does not implement AI-native provider/ranker/context or
memory work, and does not attempt full librime C++ plugin ABI compatibility.
