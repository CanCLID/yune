# Compatibility Foundation Summary

> **Status:** Finished · **Milestone:** M5–M7 (Compatibility foundation) · **Closed:** 2026-04-30 · **Type:** milestone summary (archived)

Yune's first compatibility foundation milestone is complete as of `v0.1-compat-foundation`.

## Completed scope

- CLI frontend surrogate: `yune-cli` can drive setup, deployment, schema selection, session lifecycle, key processing, rendering, and transcript replay through the RIME ABI path.
- Native ABI validation: a dynamic-loader integration path exercises the built cdylib and locks struct layout, API table, runtime lifecycle, notification, module, and session behavior.
- Schema pipeline depth: processor, segmentor, translator, filter, spelling algebra, correction/tolerance, OpenCC, and larger schema-chain behaviors have focused compatibility coverage or explicit deferrals.
- Compiled dictionary data: runtime dictionary loading and rebuild behavior consume compiled table, prism, reverse, stem, dict settings, preset vocabulary, encoder, correction, and tolerance data where the current compatibility slice requires it.
- User dictionary behavior: userdb storage, typed records, backup/restore, sync, recovery, transaction rollback, runtime learning, frequency updates, predictive lookup, and frontend-style persistence are implemented and covered by focused tests.

## Explicit boundaries

- Yune is not a full librime clone yet; the implemented surface is the measured compatibility foundation needed for the current Rust ABI and frontend-surrogate workflows.
- The userdb implementation is a typed file-backed compatibility abstraction, not full LevelDB binary compatibility.
- The C++ plugin ABI, Lua, octagram, predict, proto, and broader librime plugin ecosystem remain out of scope.
- A real native frontend integration is not complete; current validation uses CLI and native frontend-like loader paths.
- AI-native candidates, ranking, context policy, memory policy, and privacy controls are intentionally deferred to a separate product layer.

## Next-stage priorities

1. Real frontend validation against Squirrel, ibus-rime, fcitx-rime, or equivalent lifecycle hosts.
2. Benchmarking for session lifecycle, key-processing latency, schema deployment, dictionary loading, and userdb sync/learning paths.
3. Compatibility gap triage from real frontend behavior before expanding product-facing features.
4. AI-native design after the ABI/runtime foundation has been exercised by real frontend lifecycles.
