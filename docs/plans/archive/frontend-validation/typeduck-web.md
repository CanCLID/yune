# TypeDuck-Web Browser/WebAssembly Validation

> **Status:** Finished · **Milestone:** M8 / Phase 6 · **Closed:** 2026-05-01 · **Type:** validation record (archived)

## Scope

This note covers `FRONTEND-VALIDATION-02` and `FRONTEND-VALIDATION-05` for Phase 6 Plan 02. Per D-02, TypeDuck-Web is the first real application frontend validation target after the native loader harness. The checked-in reproduction artifact is `fixtures/frontend-traces/typeduck-web-basic.json`.

Per D-03, this browser/WebAssembly validation is additive. It does not replace Squirrel, ibus-rime, fcitx-rime, fcitx5-rime, native dynamic-library loading, native threading, packaging, or OS input-context validation.

No TypeDuck-Web source is vendored into Yune per D-09. The wrapper shape was source-modeled from the externally named TypeDuck-Web files `wasm/api.cpp`, `src/worker.ts`, and `src/rime.ts` as identified by Phase 06 research; only the minimized ABI call sequence is represented in this repository.

## Wrapper to Yune ABI Map

| TypeDuck-Web wrapper concern | Yune ABI validation path |
|------------------------------|--------------------------|
| Obtain global RIME API pointer | `rime_get_api` returns the process-wide `RimeApi` table. |
| Browser virtual shared/user paths | `setup` receives synthetic shared, user, prebuilt, and staging directories. |
| Runtime initialization | `deployer_initialize` and `initialize` run against the synthetic browser-modeled paths. |
| Worker notification hookup | `set_notification_handler` is registered, replaced, and cleared; events are recorded without raw pointer context. |
| Maintenance/deploy startup | `start_maintenance`, `join_maintenance_thread`, and `deploy` are invoked before session use. |
| One global frontend session | `create_session`, `find_session`, and `destroy_session` validate the wrapper's single-session shape. |
| Schema selection | `select_schema` uses logical ID `typeduck_luna`; no filesystem-looking resource ID is introduced. |
| Key input | `simulate_key_sequence` drives the TypeDuck-Web-style wrapper path, and `process_key` availability is still required by the API table. |
| Context/status/commit reads | `get_status`/`free_status`, `get_context`/`free_context`, and `get_commit`/`free_commit` are all paired and asserted by the Rust test. |
| Candidate navigation and mutation | `candidate_list_begin`/`candidate_list_next`/`candidate_list_end`, `highlight_candidate`, `highlight_candidate_on_current_page`, `change_page`, `delete_candidate_on_current_page`, and `select_candidate_on_current_page` are covered. |
| Customization/levers | The levers module is resolved with `find_module`; `custom_settings_init`, customization calls, `save_settings`, and `custom_settings_destroy` are paired. |
| Persistence | Browser persistence is modeled as synthetic user/staging paths and custom settings writes; IDBFS itself is classified as a browser/WASM limit. |
| Cleanup/teardown | `cleanup_all_sessions`, notification clearing, `finalize`, setup reset, and temporary runtime removal complete the scenario. |

## Reproduction Artifact

`fixtures/frontend-traces/typeduck-web-basic.json` is a sanitized minimized call-sequence fixture using the same host trace schema introduced by Plan 06-01. The test `typeduck_web_basic_fixture_is_sanitized_and_matches_trace_contract` checks that the artifact:

- has target `typeduck_web_browser_wasm_wrapper` and scenario `typeduck_web_basic_lifecycle`;
- contains TypeDuck-Web-specific calls such as `simulate_key_sequence`, candidate-list iteration, candidate deletion, and levers customization;
- records browser/WASM-only limit markers;
- contains no local absolute paths, `/tmp` paths, timestamps, process IDs, raw pointers, Cargo target paths, or environment variables.

## Browser/WASM Limits vs Yune ABI Mismatches

Per D-10, these are wrapper/browser limits, not Yune ABI failures:

| Classification | Finding | Artifact/Test Reference |
|----------------|---------|-------------------------|
| `browser_wasm_limit` | Emscripten worker lifecycle is source-modeled only; the Rust test does not instantiate a browser worker. | `typeduck-web-basic.json` ordered call `browser_wasm_limit.emscripten_worker_lifecycle` |
| `browser_wasm_limit` | IDBFS persistence is modeled with synthetic user/staging paths; browser persistence semantics are not native filesystem validation. | `typeduck-web-basic.json` ordered call `browser_wasm_limit.idbfs_persistence` |
| `browser_wasm_limit` | Browser/WebAssembly cannot validate native dynamic-library loading. | `typeduck-web-basic.json` ordered call `browser_wasm_limit.native_dynamic_loading` |

No TypeDuck-Web-observed Yune ABI/runtime mismatch was found in this source-modeled path. The fixture's mismatch record is classified as `match` with reproduction status `minimized_fixture`.

## FRONTEND-VALIDATION-05 Capture

`FRONTEND-VALIDATION-05` requires frontend-observed ABI/runtime mismatches to be captured before fixes. For this plan, there were no ABI/runtime fixes to apply. The observed browser/WASM gaps are captured as browser-limit entries in `fixtures/frontend-traces/typeduck-web-basic.json` and enforced by `crates/yune-rime-api/tests/frontend_hosts/typeduck_web.rs` before any future fix work is proposed.

## Boundaries Preserved

- D-02: TypeDuck-Web is treated as the first real application frontend path after the native loader harness.
- D-03: Browser/WebAssembly validation remains additive and not a native frontend substitute.
- D-09: No TypeDuck-Web source is copied or vendored into Yune.
- D-10: Browser/WASM-specific limits are separated from Yune ABI behavior.
- D-11: The plan produces a reproducible minimized call-sequence fixture.

This note intentionally does not design AI-native providers, rankers, context policy, memory, or a new GUI frontend.
