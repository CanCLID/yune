---
phase: 02-native-abi-validation-and-runtime-safety
verified: 2026-04-29T04:38:14Z
status: passed
score: 4/4 must-haves verified
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 3/4
  gaps_closed:
    - "Resource IDs from C APIs and schema YAML are rejected when they contain path traversal, absolute paths, separators, or other filesystem syntax."
  gaps_remaining: []
  regressions: []
---

# Phase 2: Native ABI Validation And Runtime Safety Verification Report

**Phase Goal:** The ABI surface is validated against at least one real frontend or native frontend-like loader, and runtime safety gaps discovered there are converted into tests and fixes.
**Verified:** 2026-04-29T04:38:14Z
**Status:** passed
**Re-verification:** Yes — after ABI-03 gap closure

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|---|---|---|
| 1 | Developer can run a real frontend client or native frontend-like loader against the current ABI and capture failures as reproducible notes or fixtures. | VERIFIED | `crates/yune-rime-api/Cargo.toml` declares `[lib] crate-type = ["rlib", "cdylib"]`. `crates/yune-rime-api/tests/dynamic_loader.rs` discovers the platform cdylib, loads it with `libloading::Library::new`, resolves `rime_get_api`, and drives the returned `RimeApi` table. The self-build fallback runs Cargo (`cargo build -p yune-rime-api`) to create the missing Cargo-built cdylib before loading; it does not load a mock or direct Rust symbol. Full `cargo test -p yune-rime-api --manifest-path /Users/trenton/Projects/yune/Cargo.toml` passed, including `dynamic_loader_harness_loads_cargo_cdylib_and_api_table`. |
| 2 | Struct layout, lifetime, notification, deployment, and session lifecycle gaps found during validation have focused regression coverage. | VERIFIED | `crates/yune-rime-api/src/tests/abi.rs` covers `RimeApi`/frontend struct layout. `crates/yune-rime-api/tests/dynamic_loader.rs` keeps the loaded `Library` alive while using the function table and exercises setup/initialize/deploy/session/status/context/commit/finalize through dynamically resolved function pointers. `crates/yune-rime-api/src/tests/lifecycle_safety.rs` covers repeated lifecycle operations, stale session rejection, and deterministic notification behavior. Package and workspace tests passed. |
| 3 | Resource IDs from C APIs and schema YAML are rejected when they contain path traversal, absolute paths, separators, or other filesystem syntax. | VERIFIED | Prior ABI-03 gap is closed. `crates/yune-rime-api/src/levers.rs` imports `resource_id::validate_config_resource_id` and `RimeLeversCustomSettingsInit` now validates the C `config_id` before storing it, returning null on unsafe IDs. The focused regression `levers_custom_settings_reject_unsafe_config_ids` exists in `crates/yune-rime-api/src/tests/resource_id.rs` and asserts `../evil` returns null while `default` succeeds and is destroyed. Focused command `cargo test -p yune-rime-api resource_id --manifest-path /Users/trenton/Projects/yune/Cargo.toml -- --nocapture` passed with 12 resource-ID tests. |
| 4 | Repeated initialize/finalize, module, notification, switcher, and session lifecycle paths remain deterministic under the validation suite. | VERIFIED | Lifecycle tests remain present and passing in the package suite. Existing runtime/module, frontend-client levers/switcher, and lifecycle safety tests passed under both package and workspace gates. |

**Score:** 4/4 truths verified

## Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `crates/yune-rime-api/Cargo.toml` | Real dynamic library packaging while preserving Rust test linkage | VERIFIED | `[lib] crate-type = ["rlib", "cdylib"]`; `libloading = "0.8"` dev-dependency is present. |
| `crates/yune-rime-api/tests/dynamic_loader.rs` | Native frontend-like dynamic loader harness | VERIFIED | Uses `discover_dynamic_artifact`, `build_dynamic_artifact`, `Library::new(&artifact)`, `library.get(b"rime_get_api\0")`, and calls `RimeApi` lifecycle/session/context/status/commit function pointers. Self-build fallback builds the Cargo cdylib and then loads the discovered artifact under the active target directory. |
| `.planning/phases/02-native-abi-validation-and-runtime-safety/02-native-loader-findings.md` | Structured loader findings | VERIFIED | Documents successful dynamic loader validation, regression test command, and that no out-of-scope findings were surfaced. |
| `crates/yune-rime-api/src/tests/lifecycle_safety.rs` | Lifecycle/notification/session determinism tests | VERIFIED | Included in `cargo test -p yune-rime-api` and `cargo test --workspace`; package suite passed with lifecycle tests. |
| `crates/yune-rime-api/src/resource_id.rs` | Shared resource-ID validators | VERIFIED | `validate_config_resource_id`, `validate_data_resource_id`, and `validate_user_dict_name` reject empty, dot/dotdot, tilde, NUL, `/`, `\\`, and Windows drive prefixes before joins. |
| `crates/yune-rime-api/src/levers.rs` | Custom-settings config IDs validated before custom YAML path construction | VERIFIED | Lines around `RimeLeversCustomSettingsInit` call `validate_config_resource_id(&config_id)` before storing `config_id`; `custom_config_path` receives only validated settings state. |
| `crates/yune-rime-api/src/tests/resource_id.rs` | Focused resource-ID rejection and allowed-ID tests | VERIFIED | Includes focused `levers_custom_settings_reject_unsafe_config_ids` regression plus config, data, deployment, schema dictionary, runtime path helper, and userdb tests. |

## Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `tests/dynamic_loader.rs` | Cargo-built cdylib | `discover_dynamic_artifact` / `build_dynamic_artifact` / `Library::new` | WIRED | Candidate paths are active target-profile, debug, and release cdylib locations. If absent, the harness runs `cargo build -p yune-rime-api --manifest-path <crate Cargo.toml>` and then rechecks those Cargo output paths. The loaded file is the Cargo-built `libyune_rime_api.dylib`/`.so`/`.dll`, not a fixture or direct-linked in-process symbol. Verified locally by the package suite and by `cargo build -p yune-rime-api --manifest-path /Users/trenton/Projects/yune/crates/yune-rime-api/Cargo.toml`, which produced `/Users/trenton/Projects/yune/target/debug/libyune_rime_api.dylib`. |
| `tests/dynamic_loader.rs` | `rime_get_api` | `library.get(b"rime_get_api\0")` | WIRED | Symbol is resolved from the loaded library before ABI calls; null API tables fail the test. |
| `tests/dynamic_loader.rs` | `RimeApi` function table | Required function pointers | WIRED | Test requires and calls setup, initialize, notification handler, deploy, create/find/select/process/get/free/destroy/cleanup/finalize through table entries. |
| `src/levers.rs` | `src/resource_id.rs` | `validate_config_resource_id` in `RimeLeversCustomSettingsInit` | WIRED | Unsafe custom-settings config IDs fail closed before `custom_config_path` can join `<config>.custom.yaml`. |
| `src/tests/resource_id.rs` | `RimeLeversCustomSettingsInit` | `levers_custom_settings_reject_unsafe_config_ids` | WIRED | Focused regression calls the exported C API with `../evil` and asserts null, then verifies a safe ID still returns a usable pointer. |
| `config_api.rs`, `lib.rs`, `deployment.rs`, `schema_install.rs`, `userdb.rs`, `levers.rs` | `resource_id.rs` | Validator calls before filesystem path joins | WIRED | Grep/code inspection confirmed validator use at runtime config/data helpers, schema dictionary/import/pack/vocabulary paths, userdb paths/snapshots, and levers custom-settings init. |

## Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|---|---|---|---|---|
| `dynamic_loader.rs` | `artifact` | `target_dir/profile/debug/release` candidates, optionally after `cargo build -p yune-rime-api` | Yes | FLOWING — `Library::new(&artifact)` uses the discovered Cargo output path. |
| `dynamic_loader.rs` | `api` / session/context/status/commit structs | Loaded cdylib -> `rime_get_api` -> `RimeApi` calls | Yes | FLOWING — the test exercises runtime state through dynamically loaded function pointers and validates observable status/context/commit values. |
| `levers.rs` | `config_id` in `LeverCustomSettings` | C string -> `c_string_key` -> `validate_config_resource_id` -> stored settings -> `custom_config_path` | Yes | FLOWING — unsafe input returns null before storage; safe normalized IDs flow to custom YAML path construction. |
| `resource_id.rs` integrations | Normalized IDs | C API/YAML strings -> validators -> runtime path helpers | Yes | FLOWING — focused resource-ID test suite passed with 12 tests, including levers custom-settings. |

## Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| Formatting gate | `cargo fmt --all --manifest-path /Users/trenton/Projects/yune/Cargo.toml --check` | Exit 0, no output | PASS |
| Focused ABI-03 regression suite | `cargo test -p yune-rime-api resource_id --manifest-path /Users/trenton/Projects/yune/Cargo.toml -- --nocapture` | 12 passed; includes `tests::resource_id::levers_custom_settings_reject_unsafe_config_ids ... ok` | PASS |
| Package test suite including dynamic loader | `cargo test -p yune-rime-api --manifest-path /Users/trenton/Projects/yune/Cargo.toml` | 239 lib tests passed, `dynamic_loader_harness_loads_cargo_cdylib_and_api_table ... ok`, 33 frontend-client tests passed, doc-tests passed | PASS |
| Workspace gate | `cargo test --workspace --manifest-path /Users/trenton/Projects/yune/Cargo.toml` | yune-cli 27 passed, frontend_surrogate 5 passed, yune-core 141 passed, yune-rime-api 239 passed, dynamic_loader 1 passed, frontend_client 33 passed, yune-schema 3 passed, doc-tests passed | PASS |
| Dynamic loader self-build fallback command shape | `cargo build -p yune-rime-api --manifest-path /Users/trenton/Projects/yune/crates/yune-rime-api/Cargo.toml` | Exit 0; Cargo-built `/Users/trenton/Projects/yune/target/debug/libyune_rime_api.dylib` exists | PASS |

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|---|---|---|---|---|
| ABI-01 | 02-01 | Developer can run the current ABI against at least one real frontend client or native frontend-like loading path and record observed gaps. | SATISFIED | `cdylib` package target, dynamic loader via `rime_get_api`, findings file, and package/workspace tests pass. Self-build fallback still loads the real Cargo-built dynamic artifact. |
| ABI-02 | 02-01, 02-02 | Struct layout, lifetime, notification, deployment, and session gaps found by frontend validation have focused regression coverage. | SATISFIED | ABI layout tests, dynamic loader harness, lifecycle safety tests, and frontend client tests all pass. |
| ABI-03 | 02-03 | Runtime resource IDs from C APIs and schema YAML reject path traversal, absolute paths, platform separators, and other non-logical IDs before filesystem joins. | SATISFIED | `RimeLeversCustomSettingsInit` now validates config IDs before storage/path joins; focused regression exists and focused resource-ID suite passed. Other config/data/schema/userdb boundaries remain wired to validators. |
| ABI-04 | 02-02 | Process-wide session, module, notification, switcher, and runtime state behavior remains deterministic under repeated initialize/finalize and session lifecycle operations. | SATISFIED | Lifecycle safety, runtime module, switcher/levers, package, and workspace test gates pass. |

## Locked Decisions D-01 Through D-12

| Decision | Status | Evidence |
|---|---|---|
| D-01 | VERIFIED | Minimum target is a native frontend-like dynamic loader harness in `tests/dynamic_loader.rs`. |
| D-02 | VERIFIED | Loader resolves `rime_get_api` and then drives the `RimeApi` function table. |
| D-03 | VERIFIED | Loader is a Cargo integration test. |
| D-04 | VERIFIED | `cdylib` crate type exists. The loader discovers or self-builds Cargo output and loads the real dynamic artifact with `Library::new`. |
| D-05 | VERIFIED | No loader-exposed in-scope gap is documented; lifecycle/resource tests cover known safety risks. |
| D-06 | VERIFIED | Fix/test scope stays in ABI/runtime safety areas. |
| D-07 | VERIFIED | Findings file records no out-of-scope schema/compiled/userdb observations from loader validation. |
| D-08 | VERIFIED | Dynamic loading passes on this platform; there is no skip-only path. |
| D-09 | VERIFIED | Repeated setup/initialize/finalize, session cleanup, schema/deployment, and notifications are tested. |
| D-10 | VERIFIED | Lifecycle loops use count 3. |
| D-11 | VERIFIED | Multi-threaded frontend behavior is documented as not broadened absent concrete loader issue. |
| D-12 | VERIFIED | Exact callback ordering is asserted around option/property/schema/deploy paths. |

## Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---:|---|---|---|
| `crates/yune-rime-api/src/levers.rs` | 77 | `RimeSwitcherSettings { placeholder: 0 }` | INFO | Opaque ABI struct marker; not a Phase 2 blocker and unrelated to custom-settings resource-ID validation. |

## Human Verification Required

None. Phase 2 is ABI/runtime-safety test-harness work and was verified through code inspection plus Cargo gates.

## Gaps Summary

No blocking gaps remain. The prior ABI-03 blocker is closed: `RimeLeversCustomSettingsInit` validates unsafe custom-settings config IDs before they can flow to `custom_config_path`, and the focused regression test is present and passing. The dynamic loader self-build fix does not weaken Phase 2's real-cdylib requirement because the fallback invokes Cargo to build `yune-rime-api`, re-discovers the generated target artifact, and then loads that artifact with `libloading::Library::new` before resolving `rime_get_api`.

---

_Verified: 2026-04-29T04:38:14Z_
_Verifier: Claude (gsd-verifier)_
