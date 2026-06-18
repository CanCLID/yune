# Coding Conventions

**Analysis Date:** 2026-06-17

> **Repository shape.** Yune is a Rust input-method engine plus a small TypeScript
> runtime package. The core (`crates/yune-core`) speaks to frontends through the
> RIME-shaped C ABI in `crates/yune-rime-api`. That ABI boundary has **three
> consumers**: (1) the CLI surrogate (`crates/yune-cli`); (2) **TypeDuck-Web**,
> via the `yune_typeduck_*` WASM adapter (`crates/yune-rime-api/src/typeduck_web.rs`)
> plus the `@yune-ime/typeduck-runtime` TS package (`packages/yune-typeduck-runtime/`);
> and (3) **TypeDuck-Windows**, via a native `rime.dll` packaged from the MSVC
> `cdylib`. librime is the **compatibility oracle**: upstream
> <https://github.com/rime/librime> plus the TypeDuck fork
> <https://github.com/TypeDuck-HK/librime> @ v1.1.2 (NOT a local checkout path).
> Current direction is **web-first**: validate the M9 TypeDuck-Web path in a real
> browser before resuming the parked M10 TypeDuck-Windows native work. AI-native
> input is a separate later layer.

## Naming Patterns

**Files:**
- Use Rust module filenames in `snake_case`, with conceptual submodules under focused directories such as `crates/yune-core/src/dictionary/source.rs`, `crates/yune-core/src/dictionary/compiled.rs`, `crates/yune-rime-api/src/processors/key_binder.rs`, and `crates/yune-rime-api/src/schema_selection.rs`.
- Use `mod.rs` only for directory module roots such as `crates/yune-core/src/dictionary/mod.rs`, `crates/yune-core/src/filter/mod.rs`, `crates/yune-core/src/translator/mod.rs`, `crates/yune-rime-api/src/processors/mod.rs`, and `crates/yune-rime-api/src/tests/mod.rs`.
- Keep crate package names in kebab-case in manifests: `crates/yune-core/Cargo.toml`, `crates/yune-schema/Cargo.toml`, `crates/yune-rime-api/Cargo.toml`, and `crates/yune-cli/Cargo.toml`.
- Keep checked-in CLI fixtures under the top-level `fixtures/` with `sample-*.json` names such as `fixtures/sample-nihao.json` and `fixtures/sample-punctuation.json`. Oracle/parity fixtures live under the owning crate's `tests/` instead (see Module & Test Design).

**Functions:**
- Use `snake_case` for Rust functions and methods, including behavior-heavy APIs such as `Engine::process_key_event` in `crates/yune-core/src/engine.rs`, `TableDictionary::parse_rime_dict_yaml_with_imports` in `crates/yune-core/src/dictionary/source.rs`, and `Schema::parse_rime_yaml` in `crates/yune-schema/src/lib.rs`.
- Two distinct `#[no_mangle] extern "C"` export families exist; do not mix them:
  - **librime-shaped ABI → `RimePascalCase`.** Use `RimePascalCase` for functions that mirror librime's C ABI, for example `RimeConfigOpen` in `crates/yune-rime-api/src/config_api.rs` and `RimeSetup` in `crates/yune-rime-api/src/runtime.rs`.
  - **Yune-owned WASM/browser ABI → `snake_case` `yune_typeduck_*`.** The TypeDuck-Web browser surface uses snake_case symbols (`yune_typeduck_init`, `process_key`, `select_candidate`, `delete_candidate`, `flip_page`, `deploy`, `customize`, `cleanup`, `response_json`, `response_handled`, `free_response`) in `crates/yune-rime-api/src/typeduck_web.rs`. These eleven names are an explicit export contract enforced by `scripts/typeduck-exports.txt` (see Library & ABI packaging). (Symbols verified against source + the allowlist; line numbers drift, so cite by symbol.)
- Name tests as long, behavior-specific `snake_case` sentences, for example `processes_ascii_keys_and_returns_unread_commit_once` in `crates/yune-rime-api/src/tests/session_api.rs` and `checked_in_fixtures_match_cli_output` in `crates/yune-cli/src/fixture.rs`.

**Variables:**
- Use `snake_case` locals and fields, with `is_` / `has_` prefixes for booleans such as `Status::is_ascii_mode` in `crates/yune-core/src/state.rs`, `has_selectable_candidates` in `crates/yune-core/src/engine.rs`, and `is_last_page` in `crates/yune-rime-api/src/abi.rs`.
- Use descriptive temporary names over abbreviations when crossing ABI or schema boundaries, such as `shared_data_dir`, `user_data_dir`, `prebuilt_data_dir`, and `backup_config_files` in `crates/yune-rime-api/src/runtime.rs`.
- Use `_guard` for intentionally held test mutex guards, as in `crates/yune-rime-api/src/tests/session_api.rs` and `crates/yune-rime-api/tests/typeduck_web.rs`, to serialize process-wide runtime state.

**Types:**
- Use `UpperCamelCase` for structs, enums, traits, and error types such as `Engine`, `CandidateRanker`, `TableDictionaryParseError`, `RimeConfigIterator`, and `SchemaParseError`.
- Keep C ABI mirror types prefixed with `Rime` and marked `#[repr(C)]` in `crates/yune-rime-api/src/abi.rs`.
- Derive common traits near type declarations. Current types commonly derive combinations of `Clone`, `Copy`, `Debug`, `Default`, `Eq`, `Hash`, and `PartialEq`, as in `KeyModifiers` and `KeyCode` in `crates/yune-core/src/key.rs`.

**TypeScript (`@yune-ime/typeduck-runtime`):**
- Use `UpperCamelCase` for exported interfaces and classes (e.g. `TypeDuckResponse`, `TypeDuckContext`, `TypeDuckResponseError` in `packages/yune-typeduck-runtime/src/response.ts`); `snake_case` field names mirror the JSON the WASM adapter emits (`page_size`, `page_no`, `is_last_page`).
- Named `Error` subclasses (e.g. `TypeDuckResponseError extends Error`, setting `this.name`) carry failures across the boundary.

## Code Style

**Formatting:**
- Use `rustfmt` through `cargo fmt`; no repo-specific `rustfmt.toml` or `.rustfmt.toml` is present.
- Use Rust 2021 syntax with workspace MSRV `1.76` from `Cargo.toml`; avoid newer standard-library helpers unless the MSRV is raised.
- Keep early-return `let Some(...) = ... else { return ...; };` and `let Ok(...) = ... else { return ...; };` patterns for validation-heavy code, as in `crates/yune-rime-api/src/config_api.rs` and `crates/yune-rime-api/src/runtime.rs`.
- Prefer small focused production modules. Keep `crates/yune-core/src/lib.rs` and `crates/yune-rime-api/src/lib.rs` as public facades and glue; add new implementation work to focused modules such as `crates/yune-core/src/key.rs` or `crates/yune-rime-api/src/processors/speller.rs`.
- TypeScript is built with `tsc` in strict mode (`packages/yune-typeduck-runtime/tsconfig.json` sets `"strict": true`, `target: ES2022`, `module/moduleResolution: NodeNext`), and the package is ES-module only (`package.json` `"type": "module"`).

**Line endings / EOL:**
- Line endings are normalized via `.gitattributes` (`* text=auto eol=lf`) with explicit exceptions: `*.bat`/`*.cmd` stay CRLF, `*.sh` stays LF, and listed binary extensions (including `*.wasm`, `*.dll`, `*.so`, `*.exe`) are never normalized.
- `.editorconfig` enforces UTF-8, LF, a final newline, and trailing-whitespace trim for all files, with `*.md` exempted from trailing-whitespace trim.
- The repo is developed on Windows but ships shell/WASM tooling: do not commit CRLF into normalized files, or `.sh` scripts and diffs break.

**Linting:**
- Treat the root `Cargo.toml` lint policy as the intended standard: `[workspace.lints.clippy] all = "warn"` and `pedantic = "warn"`.
- Use the documented quality gate from `docs/plans/refactor-plan.md`: `cargo clippy --workspace --all-targets -- -D warnings`.
- Public pure accessors and constructors commonly carry `#[must_use]`, for example `Engine::new` in `crates/yune-core/src/engine.rs`, `Schema::minimal` in `crates/yune-schema/src/lib.rs`, and `TableEntry::new` in `crates/yune-core/src/dictionary/source.rs`.
- FFI boundary functions use explicit `unsafe extern "C" fn` signatures plus `# Safety` docs and local `// SAFETY:` comments, as in `crates/yune-rime-api/src/config_api.rs`, `crates/yune-rime-api/src/runtime.rs`, and `crates/yune-rime-api/src/ffi_memory.rs`.

## Import Organization

**Order:**
1. Standard library imports first, often grouped with `use std::{...};`, as in `crates/yune-rime-api/src/runtime.rs` and `crates/yune-cli/src/main.rs`.
2. External crates next, such as `use regex::Regex;`, `use serde_yaml::Value;`, and `use yune_core::{...};` in `crates/yune-rime-api/src/schema_install.rs`.
3. Local `crate::`, `super::`, and module imports last, commonly grouped with braces in files like `crates/yune-core/src/engine.rs` and `crates/yune-rime-api/src/config_api.rs`.

**Path Aliases:**
- No custom path aliases are configured. Use crate names from workspace manifests, for example `yune_core` in `crates/yune-rime-api/src/schema_install.rs` and `crates/yune-cli/src/sample_core.rs`.
- Within a crate, use `crate::...` for cross-module access and `super::...` for sibling or parent module access, as in `crates/yune-core/src/punctuation.rs`, `crates/yune-core/src/filter/mod.rs`, and `crates/yune-rime-api/src/tests/session_api.rs`.

## Error Handling

**Patterns:**
- Library parsing code returns custom error types implementing `Display` and `Error`, such as `KeySequenceParseError` in `crates/yune-core/src/key.rs` and `SchemaParseError` in `crates/yune-schema/src/lib.rs`.
- CLI code returns `Result<(), String>` from `run` and maps errors to `stderr` plus `ExitCode::FAILURE` in `crates/yune-cli/src/main.rs`.
- C ABI functions return librime-shaped `Bool` values or null pointers instead of panicking. Validate null pointers and invalid C strings at the boundary, as in `crates/yune-rime-api/src/config_api.rs` and `crates/yune-rime-api/src/candidate_api.rs`.
- Use `expect` for internal invariants and test setup, not for ordinary user input. Examples include mutex poisoning checks in production paths (`crates/yune-rime-api/src/lib.rs`, `crates/yune-rime-api/src/runtime.rs`) and fixture/test setup checks in `crates/yune-rime-api/src/tests/mod.rs`.
- When a function is compatibility-oriented, preserve librime-shaped fallback behavior explicitly, such as missing config open behavior in `crates/yune-rime-api/src/tests/config_api.rs`.

## Logging

**Framework:** console

**Patterns:**
- No `log` or `tracing` dependency is present in workspace manifests. Use `println!` only for CLI user output in `crates/yune-cli/src/main.rs`, `crates/yune-cli/src/render.rs`, and `crates/yune-cli/src/fixture.rs`.
- Use `eprintln!` for CLI errors at the process boundary in `crates/yune-cli/src/main.rs`.
- Library crates should avoid unsolicited output. Return structured values, `Result`, `Bool`, or null pointers instead of logging from `crates/yune-core/src/*`, `crates/yune-schema/src/lib.rs`, and `crates/yune-rime-api/src/*`.

## Comments

**When to Comment:**
- Document FFI safety requirements with Rustdoc `# Safety` sections on unsafe public functions and `// SAFETY:` comments next to unsafe blocks, following `crates/yune-rime-api/src/config_api.rs` and `crates/yune-rime-api/src/runtime.rs`.
- **Name the external librime behavior.** When code exists to mirror librime, name the specific upstream construct it reproduces (class, function, or field), not just "compatibility", so a reviewer can trace the comment to the oracle. Example: `// librime's Signature::Sign stores a trimmed ctime(3) string.` in `crates/yune-rime-api/src/lib.rs` (`librime_signature_modified_time`). librime is named in comments hundreds of times across `crates/yune-rime-api/src/` (e.g. `config_api.rs`, `selector.rs`, `levers.rs`).
- Use comments to explain compatibility behavior, struct layout, ownership, or intentionally unusual fallbacks. Avoid restating straightforward Rust control flow.
- Source comments stay local to non-obvious invariants; longer-lived process and design guidance lives under `docs/` (roadmap in `docs/roadmap.md`; plans under `docs/plans/`).

**JSDoc/TSDoc:**
- The TypeScript runtime (`packages/yune-typeduck-runtime/`) does not use TSDoc. It instead expresses contracts through exported interfaces and named `Error` subclasses in strict TS (e.g. `src/response.ts`, `src/typeduck.ts`). Keep that convention rather than adding TSDoc.
- Use Rustdoc on public APIs when safety, ownership, or compatibility semantics are not self-evident, especially in `crates/yune-rime-api/src/abi.rs`, `crates/yune-rime-api/src/config_api.rs`, and `crates/yune-rime-api/src/runtime.rs`.

## Function Design

**Size:** Keep production functions focused around one compatibility or state transition. Extract repeated behavior into private helpers such as `read_installation_settings` in `crates/yune-rime-api/src/runtime.rs`, `required_field` in `crates/yune-schema/src/lib.rs`, and `parse_key_event_repr` in `crates/yune-core/src/key.rs`.

**Parameters:** Prefer `impl Into<String>` and `impl IntoIterator` for ergonomic Rust-facing APIs, as in `Engine::set_schema`, `Engine::set_property`, and `TableDictionary::new`. Use exact raw pointer types only at ABI boundaries in `crates/yune-rime-api/src/abi.rs` and `crates/yune-rime-api/src/*_api.rs`.

**Return Values:** Use `Option<String>` for optional commits in `crates/yune-core/src/engine.rs`, `Result<T, Error>` for parsers in `crates/yune-core/src/key.rs` and `crates/yune-schema/src/lib.rs`, and librime-compatible `Bool`/pointer returns in `crates/yune-rime-api/src/*_api.rs`.

## C ABI Layout

- **`RimeApi` field order is the ABI.** The `#[repr(C)]` function table in `crates/yune-rime-api/src/abi.rs` is accessed by struct-pointer offset, so the order of its fields is the actual C contract. New function-table entries must be **appended at the exact position they occupy in the TypeDuck fork's `rime_api.h`**, never inserted mid-struct — a misplaced field silently breaks every native frontend.
- Verify placement against the fork header before merging. Example: `config_list_append_{bool,int,double,string}` were appended right after `config_list_size` in `crates/yune-rime-api/src/abi.rs`, with matching wiring in `crates/yune-rime-api/src/api_table.rs` (`RimeConfigListAppend*`). Rationale is recorded in `docs/plans/yune-windows-native-build.md` and `docs/plans/yune-windows-contract-implementation-plan.md`. (Anchors verified by symbol; line numbers drift.)

## Library & ABI Packaging

- `crates/yune-rime-api` declares `crate-type = ["rlib", "cdylib"]` (`Cargo.toml`), so it serves both as a Rust dependency and as a C shared library.
- The **browser build** targets `wasm32-unknown-emscripten` and exports **only** the symbols listed in `scripts/typeduck-exports.txt`, passed via `-sEXPORTED_FUNCTIONS` by `scripts/typeduck-wasm-build.sh`. When you add or rename an exported C function, **update `scripts/typeduck-exports.txt` to match** or the WASM build silently drops it.
- The **native Windows build** packages the MSVC `cdylib` as `rime.dll` (plus `.lib` and headers) via `scripts/package-typeduck-windows.ps1`.
- The crate facade re-exports the export families publicly: `crates/yune-rime-api/src/lib.rs` carries `pub use typeduck_web::*;` and `pub use api_table::{rime_get_api, ...};` alongside the `Rime*` ABI surface.

## Module & Test Design

**Exports / barrels:** Keep public re-exports centralized in crate facades. `crates/yune-core/src/lib.rs` re-exports engine, state, dictionary, filter, key, punctuation, and translator types; `crates/yune-rime-api/src/lib.rs` re-exports the ABI and API surface modules (including `typeduck_web`). Use Rust module roots as barrels where they define ownership boundaries, e.g. `crates/yune-core/src/dictionary/mod.rs`, `crates/yune-core/src/translator/mod.rs`, `crates/yune-rime-api/src/processors/mod.rs`, and `crates/yune-rime-api/src/tests/mod.rs`.

**Own each slice.** Each behavior slice owns its production module *and* its tests; `lib.rs`/`main.rs` stay thin facades (re-exports + orchestration only). Unit tests live under `<crate>/src/tests/<slice>.rs` behind `#[cfg(test)] mod tests`; integration/parity tests and their oracle fixtures live under `<crate>/tests/` (e.g. `crates/yune-core/tests/cantonese_parity.rs` with goldens in `crates/yune-core/tests/fixtures/typeduck-v1.1.2/`). CLI sample fixtures stay in the top-level `fixtures/` as `sample-*.json`.

## Testing Conventions

**Oracle-driven, NON-circular parity.** Compatibility tests must capture expected bytes/behavior from the **external oracle** (upstream <https://github.com/rime/librime> or the TypeDuck fork <https://github.com/TypeDuck-HK/librime>, e.g. v1.1.2 commit `74cb52b78fb2411137a7643f6c8bc6517acfde69`) into a checked-in fixture, then run Yune's **real production path** and assert it reproduces the oracle output. Never derive the expected value from Yune itself.
- Example: `crates/yune-core/tests/cantonese_parity.rs` locks the fixture's `oracle.engine = "TypeDuck-HK/librime"` and `engine_tag = "v1.1.2"`, then `assert_source_rows_emit_oracle_comment` feeds authored TypeDuck TSV source rows through the real `DictionaryLookupFilter` and compares the emitted comment against the oracle comment. The golden is `crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-mobile-comments.json`.

**`#[ignore]` must carry a documented blocker.** A blocked or not-yet-implementable behavior gets a *named* test marked `#[ignore = "blocked: <what is missing before it can be enabled>"]` whose body `panic!()`s. Never silently drop a behavior slice — the ignore reason documents the precise blocker (usually a missing oracle fixture). See the five ignored parity tests in `crates/yune-core/tests/cantonese_parity.rs` (e.g. `options_combine_candidates_show_full_code_enable_sentence_parity`, `correction_minimal_distance_and_m_abbreviation_parity`).

**Tests exercise the public surface, not crate internals.** ABI/frontend tests obtain the function table via `rime_get_api()` and call its members, or call the exported `yune_typeduck_*` functions — the same surface a real frontend uses — rather than reaching into private modules. See `crates/yune-rime-api/tests/frontend_client.rs` (`let api = &*rime_get_api(); api.setup / find_module / get_schema_list`) and `crates/yune-rime-api/tests/typeduck_web.rs` (imports only the exported `yune_typeduck_*` + `rime_get_api` symbols).

**Cross-platform hygiene.** Because Yune builds for Unix, Windows (MSVC `cdylib`), and `wasm32-unknown-emscripten`, shared behavior must be identical across targets and tests must not assert platform-specific values.
- *No platform-divergent assertions.* The librime signature timestamp is computed two ways — `libc::ctime_r` on Unix and a pure-Rust civil-from-days formatter (`format_ctime_utc`) on non-Unix/emscripten — but both yield the same `ctime(3)` string shape. Tests assert the *shape*, never a platform value: `assert_librime_ctime_shape` / `librime_signature_modified_time_uses_ctime_shape` in `crates/yune-rime-api/src/tests/mod.rs` check field count, weekday/month tokens, the `HH:MM:SS` layout, and a numeric year. (See `librime_signature_modified_time` in `crates/yune-rime-api/src/lib.rs`.)
- *Lock poisoning.* Test-only lock helpers are poison-tolerant (`.unwrap_or_else(PoisonError::into_inner)`), so one failing test cannot cascade — see `test_guard` / `notification_events_lock` in `crates/yune-rime-api/src/tests/mod.rs` and the matching `test_guard` in `crates/yune-rime-api/tests/typeduck_web.rs`. Production locks intentionally stay panic-on-poison via `.expect("...should not be poisoned")` (e.g. `crates/yune-rime-api/src/lib.rs`). Know which side you are on.
- Hold a serializing guard (`let _guard = test_guard();`) in tests that touch process-wide runtime state.

## Planning Docs

- Every doc under `docs/plans/` opens with a status-banner line, for example:
  `> **Status:** <Active|Parked|Finished> · **Milestone:** Mx · **Updated:** YYYY-MM-DD · **Type:** ...`.
  Current examples: `docs/plans/refactor-plan.md` (Finished, Closed 2026-04-30), `docs/plans/typeduck-web-validation-plan.md` (Active, M9), `docs/plans/yune-windows-native-build.md` (Parked, M10 — deferred behind M9).
- When a plan is finished it is **moved to `docs/plans/archive/`**, not deleted (e.g. `archive/ai-native-frontend-readiness.md`, `archive/real-frontend-validation-plan.md`, `archive/frontend-validation/`).
- The roadmap stays at `docs/roadmap.md`; refactor and other plan guidance live under `docs/plans/` (note: the older `docs/refactor-plan.md` path no longer exists — it is now `docs/plans/refactor-plan.md`).

---

*Last reviewed: 2026-06-17 — refreshed for the TypeDuck-Web (M9) and TypeDuck-Windows (M10) work; current direction is web-first.*
