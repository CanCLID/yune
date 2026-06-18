# Linux Frontend Validation Follow-up

> **Status:** Finished · **Milestone:** M8 / Phase 6 · **Closed:** 2026-05-01 · **Type:** validation record (archived)

## Scope and plan decisions

Per D-05, ibus-rime and fcitx-rime validation is scoped after the macOS/Squirrel
path. Per D-07, future Linux frontend fixes must first preserve the exact call
sequence, expected behavior, observed behavior, blocker, and reproduction status
as notes, fixtures, or focused tests.

The ordinary regression suite must keep Linux desktop daemons not mandatory. The
source-modeled coverage that is safe on macOS/developer machines lives in
`crates/yune-rime-api/tests/frontend_hosts/native_frontends.rs`, with the
Squirrel fixture carrying Linux follow-up call-sequence markers for focus/reset,
status/context/commit read ordering, and notification handling.

## ibus-rime follow-up

Environment requirements:

- Linux desktop session with IBus available.
- Distribution packages or source builds for `ibus`, `ibus-rime`, librime headers
  or equivalent development files, and a RIME schema distribution.
- A running user session bus and IBus daemon.
- A disposable RIME user data directory, not a real user profile.
- A Yune `yune-rime-api` dynamic library build available to the frontend
  experiment.

Candidate commands or placeholders:

```text
ibus-daemon --xim --daemonize
ibus engine rime
# build or configure ibus-rime against the Yune RIME ABI library in a disposable prefix
# run a focused key-event reproduction under an isolated test user/session
```

Lifecycle areas to validate:

- Focus-in creates or activates an input-context session.
- Reset/focus-out clears composition before destroy or reuse.
- Key filtering maps to `RimeProcessKey` without bypassing the ABI.
- Status and context reads are paired with matching free calls.
- Commit reads are paired with `free_commit` and happen before UI text dispatch.
- Notification callbacks propagate schema and option changes.
- Stale sessions are rejected after destroy, cleanup, sync, or finalize.

Expected ABI call-sequence mapping:

```text
setup -> initialize -> set_notification_handler -> create_session
-> select_schema -> process_key* -> get_status/free_status
-> get_context/free_context -> get_commit/free_commit
-> clear_composition/reset -> destroy_session -> finalize
```

Already covered without daemon dependency:

- `native_frontends` source-model coverage validates focus-out clear before
  destroy, context/status/commit free-pairing, notification replacement, stale
  session rejection, sync/reinitialize, and finalize behavior through `RimeApi`.

Remaining manual validation:

- IBus daemon startup and engine registration.
- Actual key filtering path and desktop candidate UI interaction.
- Session-bus and per-application focus behavior.

## fcitx-rime follow-up

Environment requirements:

- Linux desktop session with Fcitx 5 available.
- Distribution packages or source builds for `fcitx5`, `fcitx5-rime`, librime
  headers or equivalent development files, and a RIME schema distribution.
- A running Fcitx daemon/session.
- A disposable RIME user data directory, not a real user profile.
- A Yune `yune-rime-api` dynamic library build available to the frontend
  experiment.

Candidate commands or placeholders:

```text
fcitx5 --disable=all --enable=rime
# build or configure fcitx5-rime against the Yune RIME ABI library in a disposable prefix
# run a focused key-event reproduction under an isolated test user/session
```

Lifecycle areas to validate:

- Focus-in creates or activates an input-context session.
- Reset/focus-out clears composition before destroy or reuse.
- Key processing maps to `RimeProcessKey` and candidate actions through the ABI.
- Status/context/commit read ordering matches candidate UI expectations.
- Surrounding-text or preedit features that are frontend-specific are documented
  as host requirements rather than hidden Yune ABI assumptions.
- Notification callbacks propagate schema and option changes.
- Stale sessions are rejected after destroy, cleanup, sync, or finalize.

Expected ABI call-sequence mapping:

```text
setup -> initialize -> set_notification_handler -> create_session
-> select_schema -> process_key* -> get_status/free_status
-> get_context/free_context -> candidate UI actions -> get_commit/free_commit
-> clear_composition/reset -> destroy_session -> finalize
```

Already covered without daemon dependency:

- `native_frontends` source-model coverage validates the common ABI lifecycle and
  includes `linux_followup.fcitx_surrounding_text_scope` as a preserved marker for
  the future daemon-backed run.

Remaining manual validation:

- Fcitx daemon startup and addon registration.
- Actual key-processing/candidate UI path under a Linux desktop session.
- Frontend-specific surrounding-text behavior and reset/focus interactions.

## Why Linux daemons are not mandatory in Cargo tests

IBus and Fcitx require a Linux desktop/session bus, daemon lifecycle, installed
frontend packages or source builds, and per-user input-method registration. Those
external dependencies are not mandatory for ordinary `cargo test` because they
would make the deterministic regression suite environment-specific. The safe
regression boundary for this plan is the RIME ABI call sequence in
`native_frontends`; daemon-backed validation remains a follow-up reproduction
with sanitized notes or fixtures before any fix.

## Fixture and documentation requirements for future fixes

Before changing Yune behavior for an ibus-rime or fcitx-rime finding, capture:

- Target frontend and version/source reference.
- Environment setup and command used.
- Exact ABI call sequence.
- Expected behavior.
- Observed behavior.
- Blocker or mismatch classification.
- reproduction status.
- Sanitization confirmation that no personal paths, environment variables, raw
  pointers, timestamps, process IDs, Cargo target paths, or real user dictionary
  contents were committed.
