# M56 Phase 3 ABI Abuse and Panic Boundary Evidence

## Scope

Phase 3 covers the `#[no_mangle] extern "C"` exports recorded by the Phase 0 ABI
entry-point ledger:

- Ledger: `docs/reports/evidence/m56-productization-hardening/phase-0-inventory/abi-entry-ledger.csv`
- Export count: 162
- Guard policy: every discovered export must enter through `crate::ffi_guard`
  before executing implementation logic.

The Phase 0 ledger remained a read-only inventory artifact. Guard conformance is
ratcheted by the Phase 3 abuse suite, which scans the current source tree and
compares the discovered export count against the Phase 0 ledger.

## Commands

```powershell
cargo test -p yune-rime-api --lib abi:: -- --nocapture
cargo test -p yune-rime-api session_registry_poison_recovery_preserves_followup_happy_path -- --nocapture
cargo test -p yune-rime-api --test abi_abuse -- --nocapture
$env:RUST_BACKTRACE='1'; cargo test -p yune-rime-api --test abi_abuse -- --nocapture; Remove-Item Env:RUST_BACKTRACE
```

## Results

- `cargo test -p yune-rime-api --lib abi:: -- --nocapture`
  - Result: `2 passed; 0 failed`
- `cargo test -p yune-rime-api session_registry_poison_recovery_preserves_followup_happy_path -- --nocapture`
  - Result: `1 passed; 0 failed`
  - Covered: a deliberately poisoned session-registry mutex is recovered by the
    public ABI path, and a follow-up create/find/destroy lifecycle still works.
- `cargo test -p yune-rime-api --test abi_abuse -- --nocapture`
  - Result: `5 passed; 0 failed`
  - Covered:
    - source inventory count stays in lockstep with the Phase 0 ABI ledger;
    - every `#[no_mangle]` export body contains an `ffi_guard` entry point;
    - null and out-of-order calls return failure values without poisoning a
      follow-up session lifecycle;
    - cross-thread session lookup remains tolerated;
    - random logical strings passed through task/key/schema ABI calls do not
      panic.
- `RUST_BACKTRACE=1` ABI abuse rerun
  - Result: `5 passed; 0 failed`
  - Covered: the full ABI abuse suite completes without a happy-path guard
    panic/backtrace.

## Crash and Threading Policy

- ABI calls must not unwind across the C boundary in test/unwind builds.
- Guarded panic fallback values are conservative failure values: `FALSE`, `0`,
  or null pointer/null string-slice as appropriate for the slot.
- Session and runtime APIs remain process-global and internally synchronized;
  M56 does not widen the threading contract beyond tolerating cross-thread calls
  without panicking or poisoning the runtime.
- Session-registry poison must not break a subsequent ordinary ABI lifecycle
  call in the same process. Exhaustive poisoning of every process-global lock is
  recorded as follow-up hardening, not claimed as M56 coverage.

## Release Panic Strategy

M56 keeps the existing release profile:

```toml
[profile.release]
panic = "abort"
```

Reasoning:

- The `yune-rime-api` crate builds an `rlib` and `cdylib`; release builds should
  not rely on Rust unwinding across foreign callers.
- The M56 guard suite proves the intended no-reachable-panic ABI behavior in
  unwind-capable test builds.
- Changing the release profile to unwind would broaden runtime semantics and
  require a separate size/latency/product validation pass. M56 preserves no ABI
  widening and no behavior change on defined happy paths.

The release decision is therefore: keep `panic = "abort"` for release artifacts,
and ratchet debug/test panic-boundary coverage through the ABI abuse suite.
