# Technology Stack

**Analysis Date:** 2026-04-28 (refreshed 2026-06-17)

> Yune is an engine + RIME-shaped C ABI boundary with **three consumers**:
> a CLI surrogate (`yune-cli`); **TypeDuck-Web** (the `yune_typeduck_*` WASM
> adapter consumed by the `@yune-ime/typeduck-runtime` TypeScript package); and
> **TypeDuck-Windows** (a native `rime.dll`). The stack therefore spans two
> ecosystems — Cargo/Rust and npm/Node — plus an Emscripten/WASM cross-build.
> Current direction is **web-first**: validate M9 (TypeDuck-Web) in a real
> browser before resuming the parked M10 TypeDuck-Windows native work. The
> librime oracle is upstream `github.com/rime/librime` plus the TypeDuck fork
> `github.com/TypeDuck-HK/librime @ v1.1.2`. AI-native input is a separate
> later layer above this compatibility foundation.

## Languages

**Primary:**
- Rust 2021 edition - all production crates under `crates/`; workspace metadata in `Cargo.toml` sets `rust-version = "1.76"`.
- TypeScript - the browser-facing runtime package `packages/yune-typeduck-runtime` (`type=module`, ESM, ES2022/NodeNext); see "TypeScript / Web Runtime" below.

**Secondary:**
- Markdown - project notes in `README.md`, `docs/analysis.md`, `docs/roadmap.md`. Planning docs live under `docs/plans/` (with an `archive/` subfolder for finished records); `refactor-plan.md` now lives at `docs/plans/refactor-plan.md`, alongside `typeduck-web-adapter.md`, `typeduck-web-validation-plan.md`, `typeduck-web-integration-findings.md`, `yune-windows-contract-implementation-plan.md`, and `yune-windows-native-build.md`. The parked Windows engine contract is `docs/typeduck-windows-backend-requirements.md`.
- JSON - the WASM adapter serializes the engine response (commit/context/menu/status) as JSON via `serde_json::json!` in `crates/yune-rime-api/src/typeduck_web.rs`; oracle fixtures (e.g. `crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-mobile-comments.json`) and frontend traces (e.g. `fixtures/frontend-traces/typeduck-web-basic.json`) drive parity tests; the CLI still hand-writes JSON via `to_json`/`push_str` in `crates/yune-cli/src/transcript.rs`, with deterministic fixtures under `fixtures/*.json`.
- YAML - RIME schema/config/user data compatibility is parsed and emitted by `crates/yune-schema/src/lib.rs`, `crates/yune-rime-api/src/config_api.rs`, `crates/yune-rime-api/src/config_compiler.rs`, and `crates/yune-rime-api/src/deployment.rs`.
- C ABI surface - Rust exposes librime-shaped `extern "C"` APIs from `crates/yune-rime-api/src/*` using `#[repr(C)]` structs from `crates/yune-rime-api/src/abi.rs`. The TypeDuck-Web bridge (`yune_typeduck_*` exports) lives in `crates/yune-rime-api/src/typeduck_web.rs`.

## Runtime

**Environment:**
- Rust toolchain required; repo minimum is Rust 1.76 from `Cargo.toml`.
- No `rust-toolchain.toml` or `.cargo/config.toml` is present; use the active developer toolchain.
- The WASM consumer additionally needs the `wasm32-unknown-emscripten` Rust target and the Emscripten SDK (`emcc`/`emar`) on PATH (see Build/Dev).
- The TypeScript runtime needs Node + npm; build/test via the scripts in `packages/yune-typeduck-runtime/package.json`.

**Package Managers (two ecosystems):**
- **Cargo** workspace with resolver 2 in `Cargo.toml`. Lockfile: `Cargo.lock`. Workspace members: `crates/yune-core`, `crates/yune-schema`, `crates/yune-rime-api`, and `crates/yune-cli`.
- **npm/Node** for `packages/yune-typeduck-runtime` (its own `package-lock.json`; `tsc` build + `vitest` test).

## Frameworks

**Core (Rust):**
- Rust standard library - process state, filesystem operations, FFI, and synchronization throughout `crates/yune-core/src/*` and `crates/yune-rime-api/src/*`.
- `yune-core` 0.1.0 - input engine, session state, translators, filters, candidate ranking hooks, key handling, punctuation, spelling algebra, and dictionary parsing in `crates/yune-core/src/lib.rs` and `crates/yune-core/src/engine.rs`.
- `yune-schema` 0.1.0 - minimal RIME schema compatibility parser in `crates/yune-schema/src/lib.rs`.
- `yune-rime-api` 0.1.0 - RIME-style C ABI shim, session registry, config APIs, deployment helpers, levers module, and frontend-facing function table in `crates/yune-rime-api/src/lib.rs`, `crates/yune-rime-api/src/abi.rs`, and `crates/yune-rime-api/src/api_table.rs`. The TypeDuck-Web C/WASM adapter (`yune_typeduck_*`) is in `crates/yune-rime-api/src/typeduck_web.rs`.
- `yune-cli` 0.1.0 - local fixture runner and diagnostics CLI in `crates/yune-cli/src/main.rs`; the RIME API-driven frontend slot is currently reserved in `crates/yune-cli/src/rime_frontend.rs`.

**TypeScript / Web Runtime:**
- Package `@yune-ime/typeduck-runtime` at `packages/yune-typeduck-runtime` (`private`, `type=module`, ESM) — the browser-facing half of the web-first direction (M9).
- Toolchain: TypeScript `^6.0.3`, built via `npm run build` (`tsc -p tsconfig.json`, targeting ES2022 / NodeNext, `strict`, emits `dist/`); tested with vitest `^4.1.5` via `npm run test` (`vitest run`).
- Source modules under `src/`: `module.ts` (Emscripten module bindings), `typeduck.ts` (the `TypeDuckRuntime` class wrapping the WASM lifecycle), `response.ts` (parses the JSON response payload), `keys.ts` (DOM `KeyboardEvent` -> RIME key mapping), `filesystem.ts`; the public API is re-exported from `src/index.ts`.

**Testing:**
- Rust built-in test harness via `cargo test`.
- Inline unit tests live under `#[cfg(test)]` in files such as `crates/yune-core/src/lib.rs`, `crates/yune-cli/src/args.rs`, and `crates/yune-schema/src/lib.rs`.
- RIME ABI compatibility tests live in `crates/yune-rime-api/src/tests/*.rs`.
- Integration tests in `crates/yune-rime-api/tests/`: `frontend_client.rs` (frontend-style API-table coverage), `typeduck_web.rs` (the WASM-adapter integration test, also the WASM build script's native fallback), and `dynamic_loader.rs` (dlopens the built cdylib via `libloading` and validates the C ABI surface).
- Cross-crate oracle parity test `crates/yune-core/tests/cantonese_parity.rs`, driven against the v1.1.2 oracle fixture.
- JSON compatibility fixtures live in `fixtures/sample-nihao.json`, `fixtures/sample-composing.json`, `fixtures/sample-backspace.json`, and `fixtures/sample-punctuation.json`.
- TypeScript runtime tests run under vitest (`npm run test` in `packages/yune-typeduck-runtime`).

**Build/Dev:**
- Core Rust build with Cargo from the workspace root: `cargo build`, `cargo test`, `cargo run -p yune-cli`. No build scripts (`build.rs`) are present.
- **WASM build:** `scripts/typeduck-wasm-build.sh` builds the native cdylib, verifies its exports with `nm`, then — if `rustup target add wasm32-unknown-emscripten` is installed and Emscripten's `emcc`/`emar` are on PATH — runs `cargo build -p yune-rime-api --target wasm32-unknown-emscripten` with `RUSTFLAGS` link-args `-sEXPORTED_FUNCTIONS` (the `_`-prefixed `yune_typeduck_*` list) and `-sEXPORTED_RUNTIME_METHODS=ccall,cwrap,UTF8ToString`. The exported-symbol contract is `scripts/typeduck-exports.txt` (`yune_typeduck_init`, `process_key`, `select_candidate`, `delete_candidate`, `flip_page`, `deploy`, `customize`, `cleanup`, `response_json`, `response_handled`, `free_response` — all `yune_typeduck_*`-prefixed). If the target or Emscripten is absent, the script degrades gracefully to the native fallback `cargo test -p yune-rime-api --test typeduck_web`.
- **Windows native packaging:** `scripts/package-typeduck-windows.ps1` (see Production).
- **Web integration seam:** the upstream TypeDuck-Web app is vendored at `third_party/typeduck-web/source` (its own `package.json`); the Yune seam is `third_party/typeduck-web/yune-integration/` (`adapter.ts`, `assets.ts`, `README.md`, `package-alias.md`), which adapts `@yune-ime/typeduck-runtime` into the upstream app. In-browser M9 validation runs through this seam.
- Root workspace metadata in `Cargo.toml` declares `edition = "2021"`, `license = "BSD-3-Clause"`, `repository = "https://github.com/yune-ime/yune"`, and `rust-version = "1.76"`.
- Root workspace lint declarations in `Cargo.toml` set `unsafe_code = "forbid"` and Clippy `all`/`pedantic` to warn; member manifests do **not** opt in with a per-crate `[lints] workspace = true`, so the forbid does not block the `unsafe extern "C"` ABI code in `yune-rime-api`.

## Key Dependencies

**Critical (runtime):**
- `regex` (1) - spelling algebra, comment formatting, table encoder exclude patterns, RIME recognizer/speller patterns, and chord output transforms; real dependency of `yune-core` and `yune-rime-api` (`crates/yune-core/src/spelling_algebra.rs`, `crates/yune-core/src/comment_format.rs`, `crates/yune-core/src/dictionary/encoder.rs`, `crates/yune-rime-api/src/schema_install.rs`, `crates/yune-rime-api/src/processors/*`).
- `serde` (1, `derive`) - derives schema structures in `crates/yune-schema/src/lib.rs` (direct dep of `yune-schema`).
- `serde_yaml` (0.9) - parses and writes RIME schema/config/deployment YAML in `crates/yune-schema/src/lib.rs`, `crates/yune-rime-api/src/config.rs`, `crates/yune-rime-api/src/config_api.rs`, `crates/yune-rime-api/src/config_compiler.rs`, `crates/yune-rime-api/src/deployment.rs`, `crates/yune-rime-api/src/levers.rs`, and `crates/yune-rime-api/src/runtime.rs`.
- `serde_json` (1) - runtime dependency of `yune-rime-api`, used by the TypeDuck-Web C adapter (`crates/yune-rime-api/src/typeduck_web.rs`) to serialize the engine response (commit/context/menu/status) into the JSON consumed by the TypeScript runtime. (Also a `yune-core` dev-dependency for oracle/fixture tests.)
- `libc` (0.2) - librime-compatible signature time formatting. **Cross-platform split:** `libc::ctime_r` is used only on `all(unix, not(target_os = "emscripten"))`; on `any(not(unix), target_os = "emscripten")` (Emscripten/WASM and Windows) a pure-Rust `format_ctime_utc` fallback is used with no libc. Both branches are in `crates/yune-rime-api/src/lib.rs` (`librime_signature_modified_time`).

**Dev-dependencies / Benchmarks:**
- `libloading` 0.8 - `yune-rime-api` dev-dep; `crates/yune-rime-api/tests/dynamic_loader.rs` loads the built cdylib and resolves `rime_get_api` / the C symbols.
- `serde_json` 1 - `yune-core` dev-dep for oracle/fixture tests.
- Benchmarks - `yune-rime-api` declares a custom-harness bench `frontend_baselines` (`crates/yune-rime-api/benches/frontend_baselines.rs`, `[[bench]] harness = false`), run via `cargo bench -p yune-rime-api`.

**Infrastructure:**
- `yune-core` path dependency - consumed by `crates/yune-rime-api/Cargo.toml` and `crates/yune-cli/Cargo.toml`; `yune-cli` also depends on `yune-rime-api` by path.
- Transitive regex stack (`aho-corasick`, `memchr`, `regex-automata`, `regex-syntax`) - pulled through `regex` in `Cargo.lock`.
- Transitive serde stack (`serde_core`, `serde_derive`, `unsafe-libyaml`, `indexmap`, `itoa`, `ryu`) - pulled through `serde`/`serde_yaml`/`serde_json` in `Cargo.lock`.

## Configuration

**Environment:**
- No required process environment variables are detected.
- Runtime paths come from `RimeTraits` fields (`shared_data_dir`, `user_data_dir`, `prebuilt_data_dir`, `staging_dir`, `log_dir`) in `crates/yune-rime-api/src/abi.rs` and are normalized by `crates/yune-rime-api/src/runtime.rs`.
- Runtime installation settings are read from `installation.yaml` in the user data directory by `crates/yune-rime-api/src/runtime.rs`.
- RIME config data is loaded from shared, prebuilt, staged, and user YAML files through `crates/yune-rime-api/src/config_api.rs`, `crates/yune-rime-api/src/config_compiler.rs`, and `crates/yune-rime-api/src/deployment.rs`.
- Build-time Cargo metadata is used through `env!("CARGO_PKG_VERSION")` in `crates/yune-rime-api/src/lib.rs`; CLI tests use `env!("CARGO_MANIFEST_DIR")` in `crates/yune-cli/src/fixture.rs`.

**Build manifests:**
- Cargo workspace manifest: `Cargo.toml`; crate manifests `crates/yune-core/Cargo.toml`, `crates/yune-schema/Cargo.toml`, `crates/yune-rime-api/Cargo.toml`, `crates/yune-cli/Cargo.toml`; lockfile `Cargo.lock`.
- Node/TypeScript manifest: `packages/yune-typeduck-runtime/package.json` (+ `tsconfig.json`, `package-lock.json`).
- WASM build config: `scripts/typeduck-wasm-build.sh` + `scripts/typeduck-exports.txt`.

## Platform Requirements

**Development:**
- Rust/Cargo compatible with Rust 1.76 or newer; run Cargo commands from repository root so workspace paths and CLI fixture lookup behave consistently.
- For the web path: the `wasm32-unknown-emscripten` target and Emscripten SDK; Node + npm for the TypeScript runtime.
- The code relies on standard filesystem access for fixtures, RIME shared/user data directories, deployment staging, sync snapshots, and log cleanup.

**Production / Distribution (web-first, then Windows):**
- **TypeDuck-Web:** ship the Emscripten WASM artifact (`.wasm` + JS glue) produced by `scripts/typeduck-wasm-build.sh`, consumed by `@yune-ime/typeduck-runtime` and wired into the upstream app through `third_party/typeduck-web/yune-integration/`.
- **TypeDuck-Windows (parked, resume after web validation):** `yune-rime-api` declares `crate-type = ["rlib", "cdylib"]`. `scripts/package-typeduck-windows.ps1` runs `cargo build -p yune-rime-api --release --target x86_64-pc-windows-msvc`, then copies `yune_rime_api.dll`/`.dll.lib`/`.pdb` into `dist/lib` as `rime.dll`/`rime.lib`/`rime.pdb` and `rime_api.h` + `rime_levers_api.h` into `dist/include` (headers sourced from the v1.1.2 oracle extract). It then runs a C# `Add-Type` smoke test that `LoadLibraryW`'s the DLL, resolves `rime_get_api`, and checks the `config_list_append_string` slot is non-null. Params: `-Target`, `-Profile`, `-OutputDir`, `-HeaderSource`, `-NoBuild`, `-SkipSmoke`.
- **CLI surrogate:** deploy as Rust libraries/binaries produced by Cargo.
- Runtime callers must provide or accept defaults for `RimeTraits` paths so `crates/yune-rime-api/src/runtime.rs` can locate shared config, user config, staging, prebuilt data, sync snapshots, and logs.
- Network access is not part of the current runtime stack (AI-native remote calls are a separate later layer).

---

*Last reviewed: 2026-06-17 — refreshed for the TypeDuck-Web (M9) and TypeDuck-Windows (M10) work; current direction is web-first.*
