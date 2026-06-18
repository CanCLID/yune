# AI-Native Readiness Recommendation

> **Status:** Finished · **Milestone:** M8 / Phase 6 · **Closed:** 2026-05-01 · **Type:** validation record (archived)

Phase 06 closes with the `D-15` go/no-go gate for beginning AI-native candidate/ranking design. Per `D-16`, this recommendation is based on frontend lifecycle compatibility, documented wrapper/native blockers, captured ABI/runtime mismatch evidence, and benchmark baselines. It is not based on AI feature desirability.

## Recommendation: GO WITH CONDITIONS

Yune is ready to begin AI-native candidate/ranking design work, with conditions that keep classic RIME compatibility measurable and prevent AI implementation from entering Phase 06 scope.

The validated ABI foundation is strong enough for design of provider/ranker interfaces, timeout/fallback contracts, source labels, and CLI-observable experiments. However, native GUI/daemon integrations still have documented blockers and should remain compatibility follow-up work while AI-native design starts behind default-off, testable boundaries.

## Evidence

| Evidence area | Phase 06 artifact | Readiness signal |
|---|---|---|
| Native host lifecycle | `fixtures/frontend-traces/native-host-lifecycle.json`, Plan 06-01 summary | `rime_get_api`, setup, initialize, deploy/maintenance, schema selection, session lifecycle, key processing, context/status/commit reads, notifications, stale sessions, and teardown are validated through the Cargo-built cdylib boundary. |
| Frontend mismatch capture | Plan 06-01, 06-02, and 06-03 summaries | Frontend-observed behavior is captured as sanitized traces, blocker notes, or focused tests before future fixes, satisfying the Phase 06 mismatch-capture discipline. |
| TypeDuck-Web wrapper path | `fixtures/frontend-traces/typeduck-web-basic.json`, `docs/plans/archive/frontend-validation/typeduck-web.md` | The browser/WebAssembly wrapper-shaped path completes through Yune-owned `RimeApi` calls without a Yune ABI/runtime mismatch; Emscripten worker lifecycle, IDBFS persistence, and unavailable native dynamic loading are documented as browser/WASM limits. |
| Squirrel/macOS native path | `fixtures/frontend-traces/squirrel-lifecycle.json`, `docs/plans/archive/frontend-validation/squirrel-macos.md` | Squirrel-shaped lifecycle expectations are preserved as a source-modeled RimeApi fixture with a direct app-run blocker, rather than being treated as completed native product integration. |
| Linux frontend path | `docs/plans/archive/frontend-validation/linux-frontends.md` | ibus-rime and fcitx-rime validation are scoped with environment, daemon/session, build/runtime, lifecycle, and fixture requirements while staying out of ordinary Cargo tests. |
| Benchmark baselines | `docs/plans/archive/frontend-validation/benchmark-baselines.md` | ABI-observed baselines cover session create/destroy, simple ASCII `RimeProcessKey`, schema-loaded lookup, deploy/dictionary loading, and userdb learning/sync for `BENCH-01` and `BENCH-02`. |

## Conditions before AI-native implementation

1. Keep AI-native providers, rankers, context policy, memory policy, privacy controls, remote/cloud bridges, and local model bridges out of Phase 06. This document authorizes design readiness, not implementation.
2. Start AI-native work behind explicit disabled-by-default or mock/local-only paths so classic input remains deterministic when AI is unavailable, slow, or disabled.
3. Preserve frontend-sensitive benchmarks from `benchmark-baselines.md` as regression gates for future AI-native ranking or candidate changes.
4. Do not present TypeDuck-Web browser/WASM validation as native IME coverage; native Squirrel, ibus-rime, and fcitx-rime direct-run blockers remain follow-up validation work.
5. Keep all future AI-native transcript, context, and memory examples synthetic until privacy policy and data-classification rules are implemented.
6. Require any AI candidate/ranking design to define deterministic timeout and fallback behavior before native frontend exposure.

## Remaining blockers and non-blockers

- Direct Squirrel app integration is blocked by app bundle/input-method registration and signing or host-environment requirements; this is a native frontend validation follow-up, not a blocker to AI-native interface design.
- ibus-rime and fcitx-rime require Linux desktop daemon/session environments; they remain scoped follow-up validation and should not become mandatory for ordinary Cargo tests.
- TypeDuck-Web browser persistence and worker lifecycle limits are browser/WASM-specific and do not block AI-native design.
- No Phase 06 artifact identifies a Yune ABI/runtime mismatch that blocks starting AI-native candidate/ranking design.

## Out of scope for Phase 06

AI-native implementation remains out of scope: providers, rankers, context policy, memory policy, privacy controls, remote/cloud model calls, local model bridges, GUI exposure, and production personalization stores are not implemented or specified here. Those belong in the next milestone and should use Phase 06 validation evidence as input constraints.

## Final gate

`D-15` result: `GO WITH CONDITIONS` for beginning AI-native candidate/ranking design.

`D-16` basis: the recommendation is grounded in Phase 06 frontend lifecycle validation, TypeDuck-Web and native frontend blockers, mismatch capture status, and ABI-observed benchmark baselines rather than AI feature desirability.
