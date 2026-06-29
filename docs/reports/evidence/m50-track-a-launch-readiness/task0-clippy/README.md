# M50 Task 0 Clippy Gate Evidence

Date: 2026-06-29

Scope: native engine housekeeping only. This slice fixes the broad clippy gate
blocker introduced by the M49 MARISA traversal helper. No Track A latency,
memory, web, frontend, product, package, deployment, or iOS-device claim is made.

## Verified Failure

Before the fix:

```powershell
cargo clippy --workspace --all-targets -- -D warnings
```

failed on:

```text
crates\yune-core\src\dictionary\compiled_table.rs:2158
clippy::too_many_arguments
fn push_marisa_child_frames(...)
```

## Fix

`push_marisa_child_frames` now receives a private `MarisaChildFrameInput`
context struct instead of eight separate parameters. The change preserves MARISA
traversal order, prefix compatibility checks, and candidate output.

## Verification

Passed:

```powershell
cargo fmt --check
cargo test -p yune-core dictionary:: -- --nocapture
cargo clippy --workspace --all-targets -- -D warnings
```
