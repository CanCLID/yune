# Phase 05 Quality Gates

Phase 05 closes userdb storage, classic learning/predictive flow, and test-ownership hardening. These gates are local to `05-userdb-and-scaling-hardening` and are intended for final phase closure and future milestone repeatability.

## Cargo Invocation Rule

Prefer the developer toolchain path when shell `PATH` is incomplete:

```bash
$HOME/.cargo/bin/cargo <command>
```

If that path is unavailable, use the same command through `cargo`.

## Focused Command Groups

### Storage lifecycle and userdb ABI

```bash
$HOME/.cargo/bin/cargo test -p yune-rime-api userdb
$HOME/.cargo/bin/cargo test -p yune-rime-api resource_id
```

Covers typed userdb records, logical dictionary-name validation, backup/restore, import/export, recovery, sync, and levers/userdb ABI behavior.

### Learning and predictive runtime flow

```bash
$HOME/.cargo/bin/cargo test -p yune-core userdb
$HOME/.cargo/bin/cargo test -p yune-rime-api userdb
$HOME/.cargo/bin/cargo test -p yune-rime-api --test frontend_client userdb
```

Covers commit-driven learning, persisted frequency/dee/tick updates, predictive lookup, candidate ordering, and frontend-style ABI replay.

### Core test ownership splits

```bash
$HOME/.cargo/bin/cargo test -p yune-core engine
$HOME/.cargo/bin/cargo test -p yune-core translator
$HOME/.cargo/bin/cargo test -p yune-core filter
```

Covers behavior-owned engine, translator, and filter tests after moving them out of the core facade.

### API/frontend test ownership splits

```bash
$HOME/.cargo/bin/cargo test -p yune-rime-api schema_selection
$HOME/.cargo/bin/cargo test -p yune-rime-api userdb
```

Covers schema selection/resource behavior and API userdb behavior in focused owner modules.

### Frontend ABI proof

```bash
$HOME/.cargo/bin/cargo test -p yune-rime-api --test frontend_client
```

Covers frontend-style API-table usage and keeps integration-style ABI behavior in the integration test rather than a unit-test dumping ground.

## D-15 Ownership Checklist

Every Phase 05 plan summary must name all three of the following before claiming closure:

1. **Implementation module owner** — the focused production module or facade seam that owns the behavior.
2. **Test module owner** — the behavior-owned unit or integration test module that verifies it.
3. **Librime comparison target** — the librime file, component, or observable behavior used as the compatibility oracle.

Do not mark a Phase 05 plan complete if a behavior change lacks this owner/test/oracle mapping.

## D-16 Final Gate Commands

Run these commands for final Phase 05 closure, using `$HOME/.cargo/bin/cargo` first and falling back to `cargo` only if needed:

```bash
$HOME/.cargo/bin/cargo fmt --check
$HOME/.cargo/bin/cargo test -p yune-core engine
$HOME/.cargo/bin/cargo test -p yune-core translator
$HOME/.cargo/bin/cargo test -p yune-core filter
$HOME/.cargo/bin/cargo test -p yune-rime-api userdb
$HOME/.cargo/bin/cargo test -p yune-rime-api schema_selection
$HOME/.cargo/bin/cargo test -p yune-rime-api --test frontend_client
$HOME/.cargo/bin/cargo test --workspace
$HOME/.cargo/bin/cargo clippy --workspace --all-targets -- -D warnings
```

## Mechanical Split Rule

Future test splits must be committed separately from semantic behavior changes. Mechanical movement may adjust imports, module declarations, and rustfmt output, but it must preserve assertions and observable behavior. If a move would require semantic behavior changes, leave the test in place and record the reason in the relevant summary.
