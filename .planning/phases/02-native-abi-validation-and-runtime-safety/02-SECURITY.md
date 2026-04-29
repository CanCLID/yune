---
phase: 02-native-abi-validation-and-runtime-safety
reviewed: 2026-04-29
status: secured
asvs_level: 1
block_on: high
threats_reviewed: 18
threats_open: 0
findings:
  - "All declared Phase 2 STRIDE mitigations were verified in code or phase validation artifacts."
  - "Dynamic loading is constrained to Cargo-built yune-rime-api artifacts and resolves rime_get_api before ABI table use."
  - "Lifecycle, session, notification, callback, and resource-ID traversal mitigations are covered by implementation and focused regression tests."
---

# Phase 02 Security Audit: Native ABI Validation and Runtime Safety

**Reviewed:** 2026-04-29  
**ASVS Level:** 1  
**Threats reviewed:** 18  
**Threats open:** 0

## Scope

This audit verifies the declared STRIDE threat mitigations in:

- `.planning/phases/02-native-abi-validation-and-runtime-safety/02-01-PLAN.md`
- `.planning/phases/02-native-abi-validation-and-runtime-safety/02-02-PLAN.md`
- `.planning/phases/02-native-abi-validation-and-runtime-safety/02-03-PLAN.md`

Implementation files were treated as read-only. Evidence below is from implemented code, focused regression tests, or the required loader findings artifact where the threat mitigation explicitly required findings documentation.

## Threat Verification

| Threat ID | Category | Disposition | Status | Evidence |
|-----------|----------|-------------|--------|----------|
| T-02-01-01 | Spoofing | mitigate | CLOSED | `crates/yune-rime-api/Cargo.toml:8-9` declares `crate-type = ["rlib", "cdylib"]`; `crates/yune-rime-api/tests/dynamic_loader.rs:141-149` restricts platform artifact names, `:173-181` restricts candidates to active target/profile/debug/release directories, `:183-187` requires an existing file, and `:216-235` fails closed if rediscovery after Cargo build does not find the expected artifact. |
| T-02-01-02 | Tampering | mitigate | CLOSED | `crates/yune-rime-api/tests/dynamic_loader.rs:277-282` resolves only `rime_get_api\0` and rejects a null API table; `:237-239` rejects null required function pointers; `:292-313` applies that requirement before using table entries. Source export exists at `crates/yune-rime-api/src/api_table.rs:166-169`. |
| T-02-01-03 | Denial of Service | mitigate | CLOSED | `crates/yune-rime-api/tests/dynamic_loader.rs:262-435` exercises runtime calls from the test harness and asserts documented return values such as nonzero session IDs and `TRUE`/`FALSE` outcomes; any panic remains a test failure outside the dynamic library boundary. |
| T-02-01-04 | Information Disclosure | mitigate | CLOSED | Dynamic-loader diagnostic construction in `crates/yune-rime-api/tests/dynamic_loader.rs:226-234`, `:270-279`, and `:287-289` reports missing artifact/load/symbol/table failures without dumping environment variables, temp directory contents, or raw pointer values. |
| T-02-01-05 | Elevation of Privilege | mitigate | CLOSED | ABI structs initialize `data_size` in `crates/yune-rime-api/tests/dynamic_loader.rs:28-89`; loaded `Library` stays alive through API table use at `:268-285`; C strings are kept alive across setup/initialize at `:328-349`; allocated outputs are paired with frees at `:381-411`. |
| T-02-01-06 | Repudiation | mitigate | CLOSED | `.planning/phases/02-native-abi-validation-and-runtime-safety/02-native-loader-findings.md:2-16` records observed loader gap status, regression command, expected behavior, scope decision, and target phase. |
| T-02-02-01 | Denial of Service | mitigate | CLOSED | `crates/yune-rime-api/src/tests/lifecycle_safety.rs:4-44` performs the declared three-iteration setup/initialize/finalize loop; implementation sets service state in `crates/yune-rime-api/src/deployment.rs:27-37`. |
| T-02-02-02 | Tampering | mitigate | CLOSED | `crates/yune-rime-api/src/tests/lifecycle_safety.rs:46-75` asserts destroyed and cleanup-cleared sessions are rejected and handles are not reused; `crates/yune-rime-api/src/session.rs:166-186` removes destroyed sessions and clears all sessions. |
| T-02-02-03 | Repudiation | mitigate | CLOSED | `crates/yune-rime-api/src/tests/lifecycle_safety.rs:77-180` asserts the exact option/property/schema/deploy notification sequence. Deployment notifications are emitted at `crates/yune-rime-api/src/deployment.rs:40-52`. |
| T-02-02-04 | Elevation of Privilege | mitigate | CLOSED | `crates/yune-rime-api/src/notifications.rs:18-28` stores replacement/clearing state under a mutex; `:30-52` copies handler/context out of the lock, builds temporary `CString` values, and invokes the callback only while those strings are alive. Regression coverage is at `crates/yune-rime-api/src/tests/lifecycle_safety.rs:182-223`. |
| T-02-02-05 | Information Disclosure | mitigate | CLOSED | `crates/yune-rime-api/src/tests/lifecycle_safety.rs:21-43`, `:50-74`, and `:175-178` reset or overwrite lifecycle/session/notification state through public APIs. No lifecycle/notification diagnostics in the verified paths expose raw pointer values. |
| T-02-02-06 | Denial of Service | mitigate | CLOSED | Public ABI lifecycle/session regression tests in `crates/yune-rime-api/src/tests/lifecycle_safety.rs:4-75` treat panic as failure and assert deterministic false/null-style outcomes such as session creation returning `0` after finalize. Implementation fail-closed behavior appears in `crates/yune-rime-api/src/session.rs:29-37`, `:40-55`, and `:196-212`. |
| T-02-03-01 | Tampering | mitigate | CLOSED | `crates/yune-rime-api/src/resource_id.rs:0-52` rejects empty IDs, dot/dotdot, tilde, NUL, `/`, `\\`, and Windows drive prefixes. Config paths validate before joins in `crates/yune-rime-api/src/lib.rs:1521-1532`, `:1555-1564`, `:1579-1594`, and `:1611-1617`; config/schema C APIs validate at `crates/yune-rime-api/src/config_api.rs:25-69`. |
| T-02-03-02 | Information Disclosure | mitigate | CLOSED | Schema YAML-controlled dictionary/import/vocabulary/pack values are validated in `crates/yune-rime-api/src/schema_install.rs:390-428` before `selected_runtime_data_path`; runtime data paths validate before joining at `crates/yune-rime-api/src/lib.rs:1596-1609`. |
| T-02-03-03 | Tampering | mitigate | CLOSED | User dictionary names and restore-derived destination names validate before user/sync joins in `crates/yune-rime-api/src/userdb.rs:116-138`, `:147-222`, `:260-268`, and `:270-313`. |
| T-02-03-04 | Elevation of Privilege | mitigate | CLOSED | Deployment file names validate before shared/staging joins in `crates/yune-rime-api/src/deployment.rs:729-775`; schema deployment validates schema filenames and schema IDs before copying at `:983-1014`; workspace schema IDs and dependencies validate at `:587-644`. |
| T-02-03-05 | Denial of Service | mitigate | CLOSED | Invalid resource inputs fail closed by returning `FALSE`, `-1`, `None`, or `Value::Null` in `crates/yune-rime-api/src/lib.rs:1521-1532`, `:1555-1564`, `:1579-1609`; `crates/yune-rime-api/src/config_api.rs:25-69`; `crates/yune-rime-api/src/userdb.rs:103-222`; and `crates/yune-rime-api/src/levers.rs:317-342`. |
| T-02-03-06 | Repudiation | mitigate | CLOSED | `crates/yune-rime-api/src/tests/resource_id.rs:17-140` enumerates allowed and rejected config/data/userdict IDs; `:142-238` covers config API, runtime path helpers, deployment, schema dictionary loading, and levers custom settings; `:240-298` covers userdb fail-closed behavior. |

## Threat Flags

No unregistered threat flags were found in the required Phase 2 summaries:

- `02-01-SUMMARY.md:116-118` reports `None`.
- `02-02-SUMMARY.md:137-139` reports no new network endpoints, auth paths, filesystem trust boundaries, or schema trust surfaces beyond the threat model.
- `02-03-SUMMARY.md:148-150` reports no new network endpoints, auth paths, file access patterns outside the plan threat model, or schema trust boundaries.

## Concise Findings

1. Dynamic artifact loading constraints are implemented. The loader discovers expected platform library names under Cargo target directories, optionally builds the package, rediscovers the artifact, and then loads that artifact with `libloading`.
2. `rime_get_api` and `RimeApi` safety checks are implemented. The dynamic harness resolves only `rime_get_api\0`, rejects null API tables and required null function pointers, and checks `RimeApi.data_size` before exercising lifecycle/session APIs through table entries.
3. Lifecycle/session/notification determinism is implemented and regression-tested. Repeated lifecycle loops, stale session rejection, exact callback sequences, handler replacement, and handler clearing have focused coverage.
4. Callback handling is implemented with the required lifetime and lock behavior. Notification state is mutex-protected, callbacks are invoked outside the mutex, and temporary C strings remain valid only for the callback duration.
5. Resource-ID traversal validation is implemented across config/schema/deployment/levers/userdb boundaries. Logical IDs are validated before filesystem joins, unsafe IDs fail closed, and focused tests cover cross-platform traversal syntax, separators, NUL, tilde paths, drive prefixes, and userdb suffix misuse.

## Open Threats

None.
