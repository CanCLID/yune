# Real Frontend Validation Plan

## Goal

Exercise Yune's RIME ABI through real frontend lifecycle hosts before starting AI-native product work.

## Priority order

1. Native loader smoke test against the built `yune-rime-api` cdylib in a host-shaped process.
2. Squirrel or macOS frontend validation, because the current development environment is macOS.
3. ibus-rime or fcitx-rime validation in a Linux environment after the macOS path is understood.
4. Benchmark harnesses for hot paths discovered during frontend validation.

## Validation scenarios

- Load the dynamic library and resolve `rime_get_api` from the real frontend process shape.
- Run setup, initialize, deploy, select schema, create session, process key sequences, read context/status, commit text, and destroy session.
- Exercise repeated initialize/finalize, stale sessions, notification handler replacement, schema switching, and sync/maintenance tasks.
- Compare observed frontend calls and failure modes against the CLI surrogate and existing dynamic-loader tests.

## Benchmark scenarios

- Session create/destroy latency.
- Per-key `RimeProcessKey` latency for simple ASCII, schema-loaded table lookup, punctuation, paging, and selection paths.
- Schema deployment and dictionary load latency for representative schemas.
- Userdb learning, backup, restore, and sync latency with growing record counts.

## Outputs

- Reproducible frontend validation notes or fixtures.
- Focused regression tests for any observed ABI/runtime mismatch.
- Benchmark baselines for frontend-sensitive paths.
- A go/no-go decision for beginning AI-native candidate/ranking design.
