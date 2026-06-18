# External Integrations

**Analysis Date:** 2026-04-28

## APIs & External Services

**RIME Frontend ABI:**
- Librime-shaped C ABI surface - used by frontend-style clients to initialize runtime state, create sessions, process keys, inspect context/status/commit data, manage config, manage schemas, run deployment tasks, and access levers/userdb helpers.
  - SDK/Client: in-repo Rust crate `yune-rime-api`; public ABI types are in `crates/yune-rime-api/src/abi.rs`.
  - Function table: `rime_get_api` and `rime_levers_get_api` in `crates/yune-rime-api/src/api_table.rs`.
  - Exported functions: `#[no_mangle] extern "C"` APIs across `crates/yune-rime-api/src/session.rs`, `crates/yune-rime-api/src/context_api.rs`, `crates/yune-rime-api/src/candidate_api.rs`, `crates/yune-rime-api/src/config_api.rs`, `crates/yune-rime-api/src/deployment.rs`, `crates/yune-rime-api/src/schema_api.rs`, `crates/yune-rime-api/src/schema_selection.rs`, `crates/yune-rime-api/src/levers.rs`, `crates/yune-rime-api/src/modules.rs`, `crates/yune-rime-api/src/notifications.rs`, and `crates/yune-rime-api/src/userdb.rs`.
  - Auth: none; callers interact in-process through pointers and function tables.

**RIME Schema And Data Files:**
- RIME-compatible schema/config/dictionary files - used as the primary compatibility boundary for schema selection, deployment, processors, translators, filters, and dictionaries.
  - SDK/Client: `serde_yaml` plus local parser/config helpers.
  - Key paths: schema parsing in `crates/yune-schema/src/lib.rs`; runtime config loading in `crates/yune-rime-api/src/config_api.rs`; config include/patch handling in `crates/yune-rime-api/src/config_compiler.rs`; schema installation in `crates/yune-rime-api/src/schema_install.rs`; source dictionary parsing in `crates/yune-core/src/dictionary/source.rs`; compiled metadata parsing in `crates/yune-core/src/dictionary/compiled.rs`.
  - Auth: not applicable.

**Frontend Notification Callback:**
- In-process notification handler - frontends register a callback for deployment and schema notifications.
  - SDK/Client: `RimeNotificationHandler` in `crates/yune-rime-api/src/abi.rs`; registration in `crates/yune-rime-api/src/notifications.rs`.
  - Auth: none.

**Module Registry:**
- In-process librime-style module lookup - callers register modules and retrieve built-in/custom module pointers.
  - SDK/Client: `RimeModule` in `crates/yune-rime-api/src/abi.rs`; registry in `crates/yune-rime-api/src/modules.rs`.
  - Auth: none.

**AI Ranking Extension Point:**
- Optional local candidate reranking hook - the core engine accepts `CandidateRanker` implementations and ships a `MockAiRanker` for deterministic tests.
  - SDK/Client: `CandidateRanker`, `RerankResult`, and `MockAiRanker` in `crates/yune-core/src/lib.rs`; execution path in `crates/yune-core/src/engine.rs`.
  - Auth: none.
  - External network/model service: not detected.

## Data Storage

**Databases:**
- External database: Not detected.
- User dictionary storage: plain local files named `*.userdb` in the runtime user data directory.
  - Connection: `RimeTraits.user_data_dir` via `crates/yune-rime-api/src/abi.rs` and `crates/yune-rime-api/src/runtime.rs`.
  - Client: filesystem helpers in `crates/yune-rime-api/src/userdb.rs`.
- User dictionary sync snapshots: plain text files named `*.userdb.txt` under the per-user sync directory built by `crates/yune-rime-api/src/runtime.rs` and read/written by `crates/yune-rime-api/src/userdb.rs`.
- LevelDB or other embedded database dependency: Not detected.

**File Storage:**
- Local filesystem only.
- Shared data directory: source `default.yaml`, `*.schema.yaml`, dictionary YAML, and included preset YAML read through `crates/yune-rime-api/src/runtime.rs`, `crates/yune-rime-api/src/config_compiler.rs`, and `crates/yune-rime-api/src/deployment.rs`.
- User data directory: `installation.yaml`, `user.yaml`, custom YAML, trash, and `*.userdb` files managed by `crates/yune-rime-api/src/runtime.rs`, `crates/yune-rime-api/src/deployment.rs`, `crates/yune-rime-api/src/levers.rs`, and `crates/yune-rime-api/src/userdb.rs`.
- Staging/prebuilt directories: deployed configs and schema lists are read from or written to paths derived in `crates/yune-rime-api/src/runtime.rs` and consumed by `crates/yune-rime-api/src/schema_api.rs`.
- CLI fixtures: checked-in JSON fixtures under `fixtures/` are read by `crates/yune-cli/src/fixture.rs`.

**Caching:**
- Process-local in-memory state via `OnceLock` and `Mutex`.
- Runtime paths cache: `crates/yune-rime-api/src/runtime.rs`.
- Session registry: `crates/yune-rime-api/src/session.rs`.
- Notification handler state: `crates/yune-rime-api/src/notifications.rs`.
- Module registry: `crates/yune-rime-api/src/modules.rs`.
- API table singletons and state-label cache: `crates/yune-rime-api/src/api_table.rs`.
- Freshness metadata is embedded in staged YAML `__build_info` by `crates/yune-rime-api/src/config_compiler.rs` and checked by `crates/yune-rime-api/src/deployment.rs`.

## Authentication & Identity

**Auth Provider:**
- None.
  - Implementation: all current APIs are local library/CLI/FFI calls; no user login, OAuth, API key, token, or credential provider is detected.

**Identity:**
- Runtime installation identity is local metadata, not authentication.
  - Implementation: `installation_id` is read/generated in `installation.yaml` by `crates/yune-rime-api/src/runtime.rs` and `crates/yune-rime-api/src/deployment.rs`.
  - Used by: sync path construction for user dictionary snapshots in `crates/yune-rime-api/src/userdb.rs`.

## Monitoring & Observability

**Error Tracking:**
- None detected.

**Logs:**
- CLI output uses stdout/stderr in `crates/yune-cli/src/main.rs`, `crates/yune-cli/src/render.rs`, and `crates/yune-cli/src/fixture.rs`.
- RIME runtime stores `app_name` and `log_dir` from `RimeTraits` in `crates/yune-rime-api/src/runtime.rs`.
- Log maintenance deletes old app log files from `log_dir` in `crates/yune-rime-api/src/deployment.rs`.
- No `log`, `tracing`, Sentry, OpenTelemetry, or remote logging dependency is detected.

## CI/CD & Deployment

**Hosting:**
- Repository metadata points to GitHub: `https://github.com/yune-ime/yune` in `Cargo.toml`.
- Runtime deployment means local RIME workspace maintenance/staging, implemented in `crates/yune-rime-api/src/deployment.rs`.

**CI Pipeline:**
- Not detected; no `.github/` workflow files are present in the repository scan.

**Packaging:**
- Cargo workspace builds Rust libraries and the `yune-cli` binary.
- `yune-rime-api` exposes C ABI functions and builds as both `rlib` and `cdylib`; native frontend packaging uses `scripts/package-typeduck-windows.ps1` to produce the TypeDuck-Windows `rime.dll`/`rime.lib` layout.

## Environment Configuration

**Required env vars:**
- None detected.
- Build-time Cargo metadata macros are used: `env!("CARGO_PKG_VERSION")` in `crates/yune-rime-api/src/lib.rs` and `env!("CARGO_MANIFEST_DIR")` in `crates/yune-cli/src/fixture.rs`.

**Runtime config inputs:**
- `RimeTraits.shared_data_dir`, `RimeTraits.user_data_dir`, `RimeTraits.prebuilt_data_dir`, `RimeTraits.staging_dir`, and `RimeTraits.log_dir` in `crates/yune-rime-api/src/abi.rs`.
- `installation.yaml`, `default.yaml`, `user.yaml`, `*.schema.yaml`, `*.custom.yaml`, and dictionary YAML in the local RIME data directories.

**Secrets location:**
- Not applicable.
- No `.env*` files are detected in the repository scan, and no `.env` contents were read.

## Webhooks & Callbacks

**Incoming:**
- No HTTP webhooks.
- Incoming calls are in-process C ABI calls from frontend clients through the `RimeApi` function table in `crates/yune-rime-api/src/api_table.rs`.
- Frontend-style integration tests exercise this path in `crates/yune-rime-api/tests/frontend_client.rs`.

**Outgoing:**
- No HTTP callbacks or outbound webhooks.
- In-process outgoing notifications invoke the registered `RimeNotificationHandler` from `crates/yune-rime-api/src/notifications.rs`.
- Module callbacks (`initialize`, `finalize`, `get_api`) are stored in `RimeModule` from `crates/yune-rime-api/src/abi.rs` and resolved by `crates/yune-rime-api/src/modules.rs`.

---

*Integration audit: 2026-04-28*
