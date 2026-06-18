# External Integrations

**Analysis Date:** 2026-04-28 (refreshed 2026-06-17)

> Architecture context: Yune is an engine + a librime-shaped RIME C ABI boundary
> (`crates/yune-rime-api`) with **three consumers** of that ABI:
> 1. an in-repo **CLI surrogate** (`yune-cli`) that drives the engine directly;
> 2. **TypeDuck-Web**, via the `yune_typeduck_*` WASM adapter plus the
>    `@yune-ime/typeduck-runtime` TypeScript package (current top priority, M9);
> 3. **TypeDuck-Windows**, via a native `rime.dll` packaged from the MSVC cdylib
>    (parked, M10).
>
> Current direction is **web-first**: validate the M9 TypeDuck-Web path in a real
> browser before resuming the parked M10 TypeDuck-Windows native work. AI-native
> input is a separate, later layer (see the AI ranking extension point below) and
> is not part of either current host integration.

## APIs & External Services

**RIME Frontend ABI:**
- Librime-shaped C ABI surface - used by frontend-style clients to initialize runtime state, create sessions, process keys, inspect context/status/commit data, manage config, manage schemas, run deployment tasks, and access levers/userdb helpers.
  - SDK/Client: in-repo Rust crate `yune-rime-api`; public ABI types are in `crates/yune-rime-api/src/abi.rs`.
  - Function table: `rime_get_api` and `rime_levers_get_api` in `crates/yune-rime-api/src/api_table.rs`.
  - Exported functions: `#[no_mangle] extern "C"` APIs across `crates/yune-rime-api/src/session.rs`, `crates/yune-rime-api/src/context_api.rs`, `crates/yune-rime-api/src/candidate_api.rs`, `crates/yune-rime-api/src/config_api.rs`, `crates/yune-rime-api/src/deployment.rs`, `crates/yune-rime-api/src/schema_api.rs`, `crates/yune-rime-api/src/schema_selection.rs`, `crates/yune-rime-api/src/levers.rs`, `crates/yune-rime-api/src/modules.rs`, `crates/yune-rime-api/src/notifications.rs`, and `crates/yune-rime-api/src/userdb.rs`.
  - **Fork-only ABI extensions:** the `RimeApi` table adds four config-list mutation entry points `config_list_append_{bool,int,double,string}` for TypeDuck. They are declared in `crates/yune-rime-api/src/abi.rs` (the `config_list_append_*` fields of the `RimeApi` struct), wired to their `RimeConfigListAppend*` implementations in `crates/yune-rime-api/src/api_table.rs`, and implemented in `crates/yune-rime-api/src/config_api.rs`. These match the TypeDuck-HK fork's `rime_api.h`. **The `RimeApi` struct field order IS the ABI** and must match the fork's header — do not reorder or insert fields.
  - Auth: none; callers interact in-process through pointers and function tables.

**External Oracle — librime (validation-only, no runtime dependency):**
- librime is the project's external **compatibility oracle**. It is never linked or called at runtime; it is the source of truth that user-visible behavior, schema semantics, and ABI contracts are validated against.
  - Upstream: `https://github.com/rime/librime`.
  - Cantonese/TypeDuck behavior is pinned to the **TypeDuck-HK fork** `https://github.com/TypeDuck-HK/librime` at tag **v1.1.2** (commit `74cb52b78fb2411137a7643f6c8bc6517acfde69`).
  - Oracle goldens are checked in under `crates/yune-core/tests/fixtures/typeduck-v1.1.2/` and asserted by `crates/yune-core/tests/cantonese_parity.rs` (which verifies the fixture's pinned engine/tag/commit). Parity tests are **non-circular**: the goldens are oracle-captured, not re-derived from Yune.
  - Legacy note: `AGENTS.md` and `crates/yune-rime-api/src/tests/distribution_schema_comparison.rs` still cite a local checkout path (`/Users/trenton/Projects/librime`). The convention has moved to the upstream GitHub references above; treat the local-path mentions as legacy.

**TypeDuck-Web browser integration:**
- A thin C/WASM adapter that lets the engine run in a real browser. This is the current top-priority host (web-first, M9).
  - Adapter: `crates/yune-rime-api/src/typeduck_web.rs` exports **11** `yune_typeduck_*` functions over the `rime_get_api()` / `rime_levers_get_api()` tables: `yune_typeduck_init`, `yune_typeduck_process_key`, `yune_typeduck_select_candidate`, `yune_typeduck_delete_candidate`, `yune_typeduck_flip_page`, `yune_typeduck_deploy`, `yune_typeduck_customize`, `yune_typeduck_cleanup`, `yune_typeduck_response_json`, `yune_typeduck_response_handled`, `yune_typeduck_free_response`. (Module is re-exported from `crates/yune-rime-api/src/lib.rs`.)
  - WASM build: `scripts/typeduck-wasm-build.sh` compiles `yune-rime-api` to `wasm32-unknown-emscripten`, linking `-sEXPORTED_FUNCTIONS` from `scripts/typeduck-exports.txt` (the same 11 names) and producing a `.wasm` artifact under `target/wasm32-unknown-emscripten/debug`.
  - TS runtime package: `@yune-ime/typeduck-runtime` (`packages/yune-typeduck-runtime/`) consumes the WASM module via Emscripten `cwrap` / `UTF8ToString`. Key files: `src/typeduck.ts` (runtime), `src/module.ts` (`TYPEDUCK_EXPORTS` + binding), `src/keys.ts` (key translation), `src/response.ts` (response decode), `src/filesystem.ts` (persistence). The package is currently `private` and resolved locally (not published) — see `third_party/typeduck-web/yune-integration/package-alias.md`.
  - Upstream app seam: `third_party/typeduck-web/yune-integration/` (`adapter.ts`, `assets.ts`, `README.md`). The adapter **replaces** the upstream TypeDuck-Web librime/WASM binding while preserving the app UI, worker queue, and Actions interface. It translates Yune's `TypeDuckResponse` (`handled`, `commits`, `context.preedit`, `context.candidates`) into the upstream `RimeResult` shape, and parses upstream key-sequence strings (e.g. `a`, `{BackSpace}`, `{Release+Enter}`) into keycode/mask via `keyEventToRimeKey`. The patch scope is intentionally minimal (touches only documented seam/config files such as `src/worker.ts`).
  - Auth: none; the adapter runs entirely in the browser worker.

**RIME Schema And Data Files:**
- RIME-compatible schema/config/dictionary files - used as the primary compatibility boundary for schema selection, deployment, processors, translators, filters, and dictionaries.
  - SDK/Client: `serde_yaml` plus local parser/config helpers.
  - Key paths: schema parsing in `crates/yune-schema/src/lib.rs`; runtime config loading in `crates/yune-rime-api/src/config_api.rs`; config include/patch handling in `crates/yune-rime-api/src/config_compiler.rs`; schema installation in `crates/yune-rime-api/src/schema_install.rs`; source dictionary parsing in `crates/yune-core/src/dictionary/source.rs`; compiled metadata parsing in `crates/yune-core/src/dictionary/compiled.rs`.
  - **Filter integrations with external-compatibility semantics** (both oracle-pinned, central to the Cantonese parity work):
    - `SimplifierFilter` (`crates/yune-core/src/filter/mod.rs`) honors a *limited subset* of librime OpenCC config names — `from_opencc_config` maps e.g. `t2s`/`hk2s` to traditional→simplified and `t2tw` to traditional→Taiwan. This is a **built-in approximation, not the real OpenCC library**. Wired during install in `crates/yune-rime-api/src/schema_install.rs` (`SimplifierFilter::new().with_opencc_config(...)`).
    - `DictionaryLookupFilter` (filter name `"dictionary_lookup_filter"`, `crates/yune-core/src/filter/mod.rs`) rewrites candidate comments into the **TypeDuck comment-panel byte format**: a leading `\u{000c}`, per-row `\r` markers, a `1`/`0` primary flag, and comma-joined fields. Validated against `TypeDuck-HK/rime-dictionary-lookup-filter` goldens via `crates/yune-core/tests/cantonese_parity.rs`.
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
- Optional local candidate reranking hook - the core engine accepts `CandidateRanker` implementations and ships a `MockAiRanker` for deterministic tests. (This is the seam for the separate, later AI-native input layer — it is not exercised by either current host integration.)
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
- Native path: local filesystem only.
- Shared data directory: source `default.yaml`, `*.schema.yaml`, dictionary YAML, and included preset YAML read through `crates/yune-rime-api/src/runtime.rs`, `crates/yune-rime-api/src/config_compiler.rs`, and `crates/yune-rime-api/src/deployment.rs`.
- User data directory: `installation.yaml`, `user.yaml`, custom YAML, trash, and `*.userdb` files managed by `crates/yune-rime-api/src/runtime.rs`, `crates/yune-rime-api/src/deployment.rs`, `crates/yune-rime-api/src/levers.rs`, and `crates/yune-rime-api/src/userdb.rs`.
- Staging/prebuilt directories: deployed configs and schema lists are read from or written to paths derived in `crates/yune-rime-api/src/runtime.rs` and consumed by `crates/yune-rime-api/src/schema_api.rs`.
- CLI fixtures: checked-in JSON fixtures under `fixtures/` are read by `crates/yune-cli/src/fixture.rs`.

**Browser persistence (TypeDuck-Web path):**
- The browser runtime persists user data to the browser via an **Emscripten IDBFS** mount over a virtual data dir, flushed with `FS.syncfs`. The native model above still applies on top of this virtual FS; IDBFS is added underneath it.
- `packages/yune-typeduck-runtime/src/filesystem.ts` defines `prepareTypeDuckFilesystem` (writes `default.yaml`, `<schema>.schema.yaml`, `<dict>.dict.yaml`, plus a `build/` dir, into the virtual FS) and the explicit sync boundaries: `syncFromPersistenceBeforeInit` (populate from IndexedDB before init), and `syncToPersistenceAfterMutation` / `deployAndSync` / `customizeAndSync` / `syncAfterUserDataChange` (flush to IndexedDB after commit/deploy/customize/delete). Mount helper: `mountTypeDuckPersistence`.
- The upstream seam (`third_party/typeduck-web/yune-integration/`) enforces these boundaries per its persistence contract (before init, after commit/deploy/customize, after candidate delete).

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
  - Implementation: all current APIs are local library/CLI/FFI calls (native or in-browser WASM); no user login, OAuth, API key, token, or credential provider is detected.

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
- Repository metadata points to GitHub: `https://github.com/yune-ime/yune` in `Cargo.toml` (`workspace.package.repository`).
- Runtime deployment means local RIME workspace maintenance/staging, implemented in `crates/yune-rime-api/src/deployment.rs`.

**CI Pipeline:**
- Not detected; no `.github/` workflow files are present in the repository scan.

**Packaging:**
- Cargo workspace builds Rust libraries and the `yune-cli` binary.
- `yune-rime-api` exposes the C ABI and declares `crate-type = ["rlib", "cdylib"]` (`crates/yune-rime-api/Cargo.toml`). There are **two distribution targets** for the cdylib, matching the two non-CLI hosts:
  1. **TypeDuck-Web (WASM, current priority, M9):** `scripts/typeduck-wasm-build.sh` builds the `wasm32-unknown-emscripten` target, exporting the 11 functions listed in `scripts/typeduck-exports.txt`; the `.wasm` artifact is consumed by `@yune-ime/typeduck-runtime`.
  2. **TypeDuck-Windows (native, parked, M10):** `scripts/package-typeduck-windows.ps1` builds the MSVC cdylib (`yune_rime_api.dll` + `.dll.lib`) for `x86_64-pc-windows-msvc` and lays it out as the TypeDuck-Windows `rime.dll`/`rime.lib` (plus headers).

## Environment Configuration

**Required env vars:**
- None detected.
- Build-time Cargo metadata macros are used: `env!("CARGO_PKG_VERSION")` in `crates/yune-rime-api/src/lib.rs` and `env!("CARGO_MANIFEST_DIR")` in `crates/yune-cli/src/fixture.rs`.

**Runtime config inputs:**
- `RimeTraits.shared_data_dir`, `RimeTraits.user_data_dir`, `RimeTraits.prebuilt_data_dir`, `RimeTraits.staging_dir`, and `RimeTraits.log_dir` in `crates/yune-rime-api/src/abi.rs`.
- `installation.yaml`, `default.yaml`, `user.yaml`, `*.schema.yaml`, `*.custom.yaml`, and dictionary YAML in the local RIME data directories.
- TypeDuck-Web path: the runtime takes shared/user data dirs as virtual FS paths and reads the required `default.yaml`, `<schema>.schema.yaml`, `<dict>.dict.yaml` assets prepared in the virtual FS (see Browser persistence above).

**Secrets location:**
- Not applicable.
- No `.env*` files are detected in the repository scan, and no `.env` contents were read.

## Webhooks & Callbacks

**Incoming:**
- No HTTP webhooks.
- Incoming calls are in-process C ABI calls from frontend clients through the `RimeApi` function table in `crates/yune-rime-api/src/api_table.rs`. Browser calls reach this same table through the `yune_typeduck_*` adapter.
- Frontend-host integration tests live under `crates/yune-rime-api/tests/frontend_hosts/` — `native.rs` and `native_frontends.rs` (native `rime.dll` contract) and `typeduck_web.rs` (the `yune_typeduck_*` browser-host contract, exercised through `rime_get_api()`) — alongside the original `crates/yune-rime-api/tests/frontend_client.rs`.

**Outgoing:**
- No HTTP callbacks or outbound webhooks.
- In-process outgoing notifications invoke the registered `RimeNotificationHandler` from `crates/yune-rime-api/src/notifications.rs`.
- Module callbacks (`initialize`, `finalize`, `get_api`) are stored in `RimeModule` from `crates/yune-rime-api/src/abi.rs` and resolved by `crates/yune-rime-api/src/modules.rs`.

---

> Note on anchors: file paths and symbol names are authoritative; any line numbers cited elsewhere drift over time — prefer the symbol/path names above and grep to locate them. Related planning docs live under `docs/plans/` (with an `archive/` subfolder), e.g. `docs/plans/refactor-plan.md`, `docs/plans/typeduck-web-validation-plan.md`, and `docs/plans/yune-windows-contract-implementation-plan.md`.

*Last reviewed: 2026-06-17 — refreshed for the TypeDuck-Web (M9) and TypeDuck-Windows (M10) work; current direction is web-first.*
