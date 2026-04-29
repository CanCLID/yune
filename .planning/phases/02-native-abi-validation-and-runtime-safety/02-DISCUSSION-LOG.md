# Phase 2: Native ABI Validation And Runtime Safety - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-29
**Phase:** 02-native-abi-validation-and-runtime-safety
**Areas discussed:** Validation target, Gap handling, Lifecycle stress

---

## Validation Target

### Minimum native/frontend-like validation target

| Option | Description | Selected |
|--------|-------------|----------|
| Dynamic loader | Build a small native dynamic-loader harness that resolves exported ABI symbols/function tables from the compiled library. | ✓ |
| Synthetic client | Extend the existing Rust frontend-style client tests only. | |
| Real frontend first | Try Squirrel/ibus/fcitx-style integration first. | |

**User's choice:** Dynamic loader
**Notes:** This is the minimum proof for Phase 2.

### ABI access path

| Option | Description | Selected |
|--------|-------------|----------|
| Function table | Load `rime_get_api`, call through `RimeApi`, and validate frontend-visible lifecycle through the same table real frontends prefer. | ✓ |
| Direct symbols | Resolve individual `Rime*` exports first. | |
| Both paths | Validate function table plus selected direct symbols. | |

**User's choice:** Function table
**Notes:** Function-table validation is the primary path.

### Repository integration

| Option | Description | Selected |
|--------|-------------|----------|
| Rust test harness | Add a Rust integration test or helper that loads the compiled dynamic library where supported. | ✓ |
| Standalone tool | Add a small separate validation binary/script. | |
| Docs-only runbook | Document manual loader commands and capture findings. | |

**User's choice:** Rust test harness
**Notes:** Keeps validation in Cargo workflows.

### Dynamic artifact requirement

| Option | Description | Selected |
|--------|-------------|----------|
| Require artifact | Update `yune-rime-api` packaging as needed so the loader test has a real dylib/cdylib to load. | ✓ |
| Harness only | Add the harness but skip when no dynamic artifact exists. | |
| Manual artifact | Keep crate config unchanged and document how to build a local artifact manually. | |

**User's choice:** Require artifact
**Notes:** A real loadable artifact is required for Phase 2.

---

## Gap Handling

### First action for observed gaps

| Option | Description | Selected |
|--------|-------------|----------|
| Test then fix | Turn each observed gap into a focused failing regression test, then fix it in the same phase when it is inside Phase 2 scope. | ✓ |
| Notes then triage | Record findings first, then decide which to fix. | |
| Fix immediately | Patch obvious gaps as they appear, then add tests afterward. | |

**User's choice:** Test then fix
**Notes:** Regression shape should be preserved before remediation.

### Fix vs defer boundary

| Option | Description | Selected |
|--------|-------------|----------|
| ABI safety only | Fix layout, lifetime, loading, notification, deployment, session lifecycle, and resource-ID safety gaps; defer schema semantics or dictionary/userdb gaps. | ✓ |
| All observed | Fix anything the loader or frontend-like path reveals. | |
| Critical only | Fix crashes/security issues only; document functional compatibility gaps. | |

**User's choice:** ABI safety only
**Notes:** Keeps Phase 2 scoped to ABI/runtime safety.

### Deferred gap recording

| Option | Description | Selected |
|--------|-------------|----------|
| Structured notes | Write structured findings with observed behavior, expected librime/frontend behavior when known, scope decision, and target future phase. | ✓ |
| Backlog only | Create backlog bullets only. | |
| Fixture stubs | Add ignored/pending tests for deferred gaps. | |

**User's choice:** Structured notes
**Notes:** Downstream phases need evidence, not only backlog labels.

### Loader cannot run

| Option | Description | Selected |
|--------|-------------|----------|
| Block phase | Treat failure to load the ABI artifact as a Phase 2 blocker unless a concrete platform limitation is documented with a replacement validation path. | ✓ |
| Fallback client | Fall back to the existing frontend-style Rust client and record the loader as a follow-up. | |
| Docs fallback | Document the loader limitation and continue with safety fixes. | |

**User's choice:** Block phase
**Notes:** Dynamic loading is the phase's proof point.

---

## Lifecycle Stress

### Paths to stress

| Option | Description | Selected |
|--------|-------------|----------|
| Core globals | Repeated setup/initialize/finalize, session create/destroy/cleanup-all, schema switch, deployment, and notification registration. | ✓ |
| All ABI state | Include modules, levers/switcher, config handles, userdb helpers, and candidate iterators too. | |
| Loader only | Focus lifecycle stress only around load/unload and function table calls. | |

**User's choice:** Core globals
**Notes:** Targets process-wide state most likely to break frontends.

### Repeat intensity

| Option | Description | Selected |
|--------|-------------|----------|
| Small loops | Deterministic small loop counts that run quickly in normal `cargo test`. | ✓ |
| Heavy stress | Large loops and possibly threaded scenarios. | |
| One repeat | Only validate a second run after cleanup/finalize. | |

**User's choice:** Small loops
**Notes:** Tests should be quick and deterministic, not stress benchmarks.

### Multi-threaded frontend-style calls

| Option | Description | Selected |
|--------|-------------|----------|
| Document only | Document concurrency as a finding unless the dynamic loader exposes a concrete issue. | ✓ |
| Basic threads | Add a minimal multi-thread smoke test around separate sessions. | |
| Full concurrency | Stress callbacks, sessions, and schema switching across threads. | |

**User's choice:** Document only
**Notes:** Concurrency is not the primary Phase 2 validation target unless a concrete issue appears.

### Notification validation

| Option | Description | Selected |
|--------|-------------|----------|
| Callback order | Register a callback and assert deterministic deployment/schema notification labels around the lifecycle paths the loader exercises. | ✓ |
| Presence only | Only assert callbacks can be registered and invoked at least once. | |
| Defer details | Leave notification timing/order to a later frontend integration phase. | |

**User's choice:** Callback order
**Notes:** Phase 2 explicitly covers notification gaps, so callback order should be validated.

---

## Claude's Discretion

- Exact dynamic-loader crate/test organization, helper naming, loop counts, structured finding file format, and resource-ID helper organization are left to planning/execution.

## Deferred Ideas

- Real native frontend integrations are beyond the minimum target unless needed by loader findings.
- Full multi-threaded frontend stress is deferred unless concrete loader evidence requires it.
- Schema semantics, compiled dictionary payloads, and userdb behavior remain later-phase work.
