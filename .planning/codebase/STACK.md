# Technology Stack

**Analysis Date:** 2026-04-28

## Languages

**Primary:**
- Rust 2021 edition - all production crates under `crates/`; workspace metadata in `Cargo.toml` sets `rust-version = "1.76"`.

**Secondary:**
- Markdown - project notes in `README.md`, `docs/analysis.md`, `docs/roadmap.md`, and `docs/refactor-plan.md`.
- JSON - deterministic CLI fixtures in `fixtures/*.json`; handwritten JSON rendering lives in `crates/yune-cli/src/transcript.rs`.
- YAML - RIME schema/config/user data compatibility is parsed and emitted by `crates/yune-schema/src/lib.rs`, `crates/yune-rime-api/src/config_api.rs`, `crates/yune-rime-api/src/config_compiler.rs`, and `crates/yune-rime-api/src/deployment.rs`.
- C ABI surface - Rust exposes librime-shaped `extern "C"` APIs from `crates/yune-rime-api/src/*` using `#[repr(C)]` structs from `crates/yune-rime-api/src/abi.rs`.

## Runtime

**Environment:**
- Rust toolchain required; repo minimum is Rust 1.76 from `Cargo.toml`.
- Local scan toolchain: `rustc 1.95.0` and `cargo 1.95.0`.
- No `rust-toolchain.toml` or `.cargo/config.toml` is present; use the active developer toolchain.

**Package Manager:**
- Cargo workspace with resolver 2 in `Cargo.toml`.
- Lockfile: present at `Cargo.lock`.
- Workspace members: `crates/yune-core`, `crates/yune-schema`, `crates/yune-rime-api`, and `crates/yune-cli`.

## Frameworks

**Core:**
- Rust standard library - process state, filesystem operations, FFI, and synchronization throughout `crates/yune-core/src/*` and `crates/yune-rime-api/src/*`.
- `yune-core` 0.1.0 - input engine, session state, translators, filters, candidate ranking hooks, key handling, punctuation, spelling algebra, and dictionary parsing in `crates/yune-core/src/lib.rs` and `crates/yune-core/src/engine.rs`.
- `yune-schema` 0.1.0 - minimal RIME schema compatibility parser in `crates/yune-schema/src/lib.rs`.
- `yune-rime-api` 0.1.0 - RIME-style C ABI shim, session registry, config APIs, deployment helpers, levers module, and frontend-facing function table in `crates/yune-rime-api/src/lib.rs`, `crates/yune-rime-api/src/abi.rs`, and `crates/yune-rime-api/src/api_table.rs`.
- `yune-cli` 0.1.0 - local fixture runner and diagnostics CLI in `crates/yune-cli/src/main.rs`; the RIME API-driven frontend slot is currently reserved in `crates/yune-cli/src/rime_frontend.rs`.

**Testing:**
- Rust built-in test harness via `cargo test`.
- Inline unit tests live under `#[cfg(test)]` in files such as `crates/yune-core/src/lib.rs`, `crates/yune-cli/src/args.rs`, and `crates/yune-schema/src/lib.rs`.
- RIME ABI compatibility tests live in `crates/yune-rime-api/src/tests/*.rs`.
- Frontend-style API-table coverage lives in `crates/yune-rime-api/tests/frontend_client.rs`.
- JSON compatibility fixtures live in `fixtures/sample-nihao.json`, `fixtures/sample-composing.json`, `fixtures/sample-backspace.json`, and `fixtures/sample-punctuation.json`.

**Build/Dev:**
- Build with Cargo from the workspace root: `cargo build`, `cargo test`, `cargo run -p yune-cli`.
- No build scripts (`build.rs`) are present.
- Root workspace metadata in `Cargo.toml` declares `edition = "2021"`, `license = "BSD-3-Clause"`, `repository = "https://github.com/yune-ime/yune"`, and `rust-version = "1.76"`.
- Root workspace lint declarations in `Cargo.toml` set `unsafe_code = "forbid"` and Clippy `all`/`pedantic` to warn; member manifests do not contain per-crate `[lints]` sections.

## Key Dependencies

**Critical:**
- `regex` 1.12.3 locked by `Cargo.lock` - used for spelling algebra, comment formatting, table encoder exclude patterns, RIME recognizer/speller patterns, and chord output transforms in `crates/yune-core/src/spelling_algebra.rs`, `crates/yune-core/src/comment_format.rs`, `crates/yune-core/src/dictionary/encoder.rs`, `crates/yune-rime-api/src/schema_install.rs`, and `crates/yune-rime-api/src/processors/*`.
- `serde` 1.0.228 locked by `Cargo.lock` - derives schema structures in `crates/yune-schema/src/lib.rs`.
- `serde_yaml` 0.9.34+deprecated locked by `Cargo.lock` - parses and writes RIME schema/config/deployment YAML in `crates/yune-schema/src/lib.rs`, `crates/yune-rime-api/src/config.rs`, `crates/yune-rime-api/src/config_api.rs`, `crates/yune-rime-api/src/config_compiler.rs`, `crates/yune-rime-api/src/deployment.rs`, `crates/yune-rime-api/src/levers.rs`, and `crates/yune-rime-api/src/runtime.rs`.
- `libc` 0.2.186 locked by `Cargo.lock` - used on Unix for librime-compatible signature time formatting in `crates/yune-rime-api/src/lib.rs`.

**Infrastructure:**
- `yune-core` path dependency - consumed by `crates/yune-rime-api/Cargo.toml` and `crates/yune-cli/Cargo.toml`.
- Transitive regex stack (`aho-corasick`, `memchr`, `regex-automata`, `regex-syntax`) - pulled through `regex` in `Cargo.lock`.
- Transitive serde stack (`serde_core`, `serde_derive`, `unsafe-libyaml`, `indexmap`, `itoa`, `ryu`) - pulled through `serde`/`serde_yaml` in `Cargo.lock`.

## Configuration

**Environment:**
- No required process environment variables are detected.
- Runtime paths come from `RimeTraits` fields (`shared_data_dir`, `user_data_dir`, `prebuilt_data_dir`, `staging_dir`, `log_dir`) in `crates/yune-rime-api/src/abi.rs` and are normalized by `crates/yune-rime-api/src/runtime.rs`.
- Runtime installation settings are read from `installation.yaml` in the user data directory by `crates/yune-rime-api/src/runtime.rs`.
- RIME config data is loaded from shared, prebuilt, staged, and user YAML files through `crates/yune-rime-api/src/config_api.rs`, `crates/yune-rime-api/src/config_compiler.rs`, and `crates/yune-rime-api/src/deployment.rs`.
- Build-time Cargo metadata is used through `env!("CARGO_PKG_VERSION")` in `crates/yune-rime-api/src/lib.rs`; CLI tests use `env!("CARGO_MANIFEST_DIR")` in `crates/yune-cli/src/fixture.rs`.

**Build:**
- Workspace manifest: `Cargo.toml`.
- Crate manifests: `crates/yune-core/Cargo.toml`, `crates/yune-schema/Cargo.toml`, `crates/yune-rime-api/Cargo.toml`, and `crates/yune-cli/Cargo.toml`.
- Lockfile: `Cargo.lock`.
- No package.json, pyproject, go.mod, or other language package manifests are present.

## Platform Requirements

**Development:**
- Rust/Cargo compatible with Rust 1.76 or newer.
- Run commands from repository root so workspace paths and CLI fixture lookup behave consistently.
- The code relies on standard filesystem access for fixtures, RIME shared/user data directories, deployment staging, sync snapshots, and log cleanup.

**Production:**
- Deploy as Rust libraries/binaries produced by Cargo.
- `yune-rime-api` is a librime-shaped ABI shim whose manifest declares `crate-type = ["rlib", "cdylib"]`; use `scripts/package-typeduck-windows.ps1` to package the MSVC cdylib as `rime.dll` plus `rime.lib` and headers for TypeDuck-Windows.
- Runtime callers must provide or accept defaults for `RimeTraits` paths so `crates/yune-rime-api/src/runtime.rs` can locate shared config, user config, staging, prebuilt data, sync snapshots, and logs.
- Network access is not part of the current runtime stack.

---

*Stack analysis: 2026-04-28*
