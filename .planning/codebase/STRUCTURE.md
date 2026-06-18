# Codebase Structure

**Analysis Date:** 2026-06-17

> **Shape in one line.** A Rust workspace (the deterministic engine + a librime/RIME
> C ABI) feeds THREE frontend consumers across one shared ABI: the **CLI surrogate**
> (`crates/yune-cli`), **TypeDuck-Web** (the `yune_typeduck_*` WASM adapter +
> the `@yune-ime/typeduck-runtime` TS package under `packages/`), and
> **TypeDuck-Windows** (a native `rime.dll` built from the same `cdylib`). Current
> direction is **web-first**: validate M9 (TypeDuck-Web) in a real browser before
> resuming the parked M10 (TypeDuck-Windows native) work. AI-native input is a
> separate, later layer. Line numbers below drift — prefer the symbol/path names.

## Directory Layout

```text
yune/
|-- Cargo.toml                         # Workspace manifest and shared Rust metadata
|-- Cargo.lock                         # Locked Rust dependency graph
|-- README.md                          # Project overview, goals, and compatibility scope
|-- AGENTS.md                          # Agent/contributor guidance for this repo
|-- .editorconfig                      # utf-8, LF, final newline, trim trailing whitespace
|-- .gitattributes                     # EOL policy (LF by default; CRLF for *.bat/*.cmd)
|-- crates/
|   |-- yune-core/                     # Deterministic core engine crate
|   |   |-- Cargo.toml
|   |   |-- src/
|   |   |   |-- lib.rs                 # Public Rust facade; declares `mod tests`
|   |   |   |-- engine.rs              # Engine state machine and candidate refresh
|   |   |   |-- state.rs               # Candidate, context, status, snapshot structs
|   |   |   |-- key.rs                 # RIME-style key sequence parser and typed keys
|   |   |   |-- punctuation.rs         # Punctuation translator
|   |   |   |-- spelling_algebra.rs    # Spelling algebra formulas
|   |   |   |-- comment_format.rs      # Candidate comment formatting formulas
|   |   |   |-- userdb.rs              # Core user-dictionary model and commit scoring
|   |   |   |-- dictionary/            # RIME table source + compiled prism/reverse/table + encoder
|   |   |   |-- translator/            # Core translator implementations
|   |   |   |-- filter/                # Core candidate filter implementations
|   |   |   `-- tests/                 # Core unit tests: engine.rs, filter.rs, translator.rs
|   |   `-- tests/
|   |       |-- cantonese_parity.rs    # Oracle parity vs captured TypeDuck v1.1.2 output
|   |       `-- fixtures/typeduck-v1.1.2/  # Captured oracle fixture + provenance README
|   |-- yune-rime-api/                 # Librime/RIME-shaped C ABI compatibility crate (rlib + cdylib)
|   |   |-- Cargo.toml
|   |   |-- benches/frontend_baselines.rs  # [[bench]] frontend_baselines (harness = false)
|   |   |-- tests/                     # Integration tests driving the exported ABI
|   |   |   |-- dynamic_loader.rs      # Loads the cdylib via libloading, drives rime_get_api
|   |   |   |-- frontend_client.rs     # Function-table integration client
|   |   |   |-- frontend_hosts.rs      # Frontend-host harness entry
|   |   |   |-- frontend_hosts/        # mod.rs, native.rs, native_frontends.rs, typeduck_web.rs
|   |   |   `-- typeduck_web.rs        # TypeDuck-Web C ABI integration
|   |   `-- src/
|   |       |-- lib.rs                 # ABI facade, key routing, shared glue
|   |       |-- abi.rs                 # C ABI structs and function table types
|   |       |-- api_table.rs           # Static RimeApi/RimeLeversApi builders
|   |       |-- typeduck_web.rs        # TypeDuck-Web adapter: yune_typeduck_* C entry points
|   |       |-- session.rs             # Session registry and SessionState
|   |       |-- context_api.rs         # Context, status, commit reads
|   |       |-- candidate_api.rs       # Candidate list iterators
|   |       |-- config.rs              # YAML config state and scalar/path helpers
|   |       |-- config_api.rs          # RIME config entrypoints
|   |       |-- config_compiler.rs     # include/patch/custom patch compilation
|   |       |-- deployment.rs          # initialize, finalize, deploy, sync, maintenance
|   |       |-- runtime.rs             # Runtime path and trait handling
|   |       |-- schema_api.rs          # Schema status/option API surface
|   |       |-- schema_install.rs      # Translator/filter/segmentor installation
|   |       |-- schema_selection.rs    # Schema selection/reset workflow
|   |       |-- key_table.rs           # Key name/code lookup tables
|   |       |-- levers.rs              # Levers / custom settings / user-dict manager API
|   |       |-- notifications.rs       # Notification callback plumbing
|   |       |-- modules.rs             # RIME module registration
|   |       |-- ffi_memory.rs          # FFI allocation/free helpers
|   |       |-- resource_id.rs         # Resource id helpers
|   |       |-- userdb.rs              # Facade: #[path = "userdb/mod.rs"]
|   |       |-- userdb/                # file_store, record, recovery, snapshot, store, sync
|   |       |-- processors/            # Schema-loaded key processors
|   |       `-- tests/                 # Focused ABI/unit compatibility modules
|   |-- yune-schema/                   # Standalone typed RIME schema subset parser
|   |   |-- Cargo.toml
|   |   `-- src/lib.rs
|   `-- yune-cli/                      # Deterministic fixture CLI crate (surrogate frontend)
|       |-- Cargo.toml
|       `-- src/
|           |-- main.rs                # CLI entry point
|           |-- args.rs                # Command parsing
|           |-- sample_core.rs         # Sample core-backed runner
|           |-- fixture.rs             # Fixture comparison
|           |-- transcript.rs          # Deterministic JSON output
|           |-- render.rs              # Help rendering
|           `-- rime_frontend.rs       # RIME ABI-backed frontend surrogate
|-- packages/
|   `-- yune-typeduck-runtime/         # npm pkg @yune-ime/typeduck-runtime (TS WASM wrapper)
|       |-- package.json
|       |-- tsconfig.json
|       |-- src/                       # filesystem.ts, index.ts, keys.ts, module.ts, response.ts, typeduck.ts
|       |-- test/                      # Vitest: *.test.ts + fakes (fake-filesystem.ts, fake-module.ts)
|       `-- dist/                      # Build output (generated)
|-- scripts/
|   |-- typeduck-wasm-build.sh         # Emscripten/WASM build (wasm32-unknown-emscripten)
|   |-- typeduck-exports.txt           # Exported symbol allowlist (yune_typeduck_*) for the WASM link
|   `-- package-typeduck-windows.ps1   # TypeDuck-Windows native packaging (-> rime.dll/.lib)
|-- third_party/
|   `-- typeduck-web/                  # Vendored TypeDuck-Web upstream + Yune integration seam
|       |-- source/                    # Upstream app (includes its own librime/)
|       |-- patches/yune-typeduck-runtime.patch
|       |-- yune-integration/          # adapter.ts (app seam), assets.ts, package-alias.md, README.md
|       |-- e2e/                       # Playwright: yune-typeduck.spec.ts, playwright.config.ts, results/
|       |-- typeduck-web.lock.json     # Provenance lockfile (do not hand-edit the vendored tree)
|       `-- README.yune-source.md
|-- docs/
|   |-- analysis.md                    # Compatibility strategy and gaps
|   |-- roadmap.md                     # Milestones and next work (web-first sequencing)
|   |-- typeduck-windows-backend-requirements.md  # Parked Windows engine contract
|   `-- plans/                         # Active planning docs (status-bannered)
|       |-- refactor-plan.md
|       |-- typeduck-web-adapter.md
|       |-- typeduck-web-integration-findings.md
|       |-- typeduck-web-validation-plan.md
|       |-- yune-windows-contract-implementation-plan.md
|       |-- yune-windows-native-build.md
|       `-- archive/                   # Finished/superseded records (incl. frontend-validation/)
|-- fixtures/
|   |-- sample-backspace.json          # CLI fixture output
|   |-- sample-composing.json          # CLI fixture output
|   |-- sample-nihao.json              # CLI fixture output
|   |-- sample-punctuation.json        # CLI fixture output
|   `-- frontend-traces/               # Captured frontend lifecycle traces (native, squirrel, typeduck-web)
`-- .planning/codebase/                # Generated codebase maps (see Special Directories)
```

## Directory Purposes

**Root:**
- Purpose: Workspace-level project definition, top-level docs, and repo-wide conventions.
- Contains: `Cargo.toml`, `Cargo.lock`, `README.md`, `AGENTS.md`, `.gitignore`, `.gitattributes`, `.editorconfig`.
- Key files: `Cargo.toml`, `README.md`, `AGENTS.md`. `.gitattributes` sets the EOL policy (`* text=auto eol=lf`; `*.bat`/`*.cmd` are CRLF; `*.sh` is LF; binaries marked `binary`) so Windows checkouts do not create CRLF-only diffs. `.editorconfig` enforces utf-8, `end_of_line = lf`, final newline, and trailing-whitespace trim (relaxed for `*.md`).

**`crates/`:**
- Purpose: Houses all Rust workspace members.
- Contains: `yune-core`, `yune-schema`, `yune-rime-api`, `yune-cli`.
- Key files: each crate's `Cargo.toml`.

**`crates/yune-core/src/`:**
- Purpose: Deterministic Rust engine and reusable compatibility primitives.
- Contains: engine state machine, public traits, key parsing, candidates, dictionary parsing, translators, filters, punctuation, spelling algebra, comment formatting, and a core user-dictionary model.
- Key files: `lib.rs`, `engine.rs`, `state.rs`, `key.rs`, `userdb.rs`.

**`crates/yune-core/src/dictionary/`:**
- Purpose: RIME dictionary source and compiled-data compatibility helpers.
- Contains: source `.dict.yaml` parsing (`source.rs`), the table encoder (`encoder.rs`), checksum/compiled metadata (`compiled.rs`), and compiled prism/reverse/table data modules (`compiled_prism.rs`, `compiled_reverse.rs`, `compiled_table.rs`).
- Key files: `source.rs`, `encoder.rs`, `compiled.rs`.

**`crates/yune-core/src/translator/`:**
- Purpose: Core candidate generation components.
- Contains: echo, table, reverse lookup, history, switch, folded switch, and schema-list translators.
- Key files: `mod.rs`.

**`crates/yune-core/src/filter/`:**
- Purpose: Core candidate post-processing components.
- Contains: `UniquifierFilter`, `SingleCharFilter`, `CharsetFilter`, `DictionaryLookupFilter`, `TaggedFilter`, `SimplifierFilter`, `ReverseLookupFilter`. `DictionaryLookupFilter` (filter name `"dictionary_lookup_filter"`) attaches `DictionaryLookupRecord` data as the TypeDuck comment panel — the engine-side feature behind the Windows contract.
- Key files: `mod.rs`.

**`crates/yune-core/src/tests/`:**
- Purpose: Core unit tests, split by area (declared by `mod tests;` in `lib.rs`).
- Contains: `engine.rs`, `filter.rs`, `translator.rs` (+ `mod.rs`).

**`crates/yune-core/tests/`:**
- Purpose: Integration parity tests against a captured oracle.
- Contains: `cantonese_parity.rs`, which asserts Yune output matches captured TypeDuck-HK/librime **v1.1.2** output. Fixtures live in `fixtures/typeduck-v1.1.2/` (`jyut6ping3-mobile-comments.json` + a `README.md` documenting provenance/engine commit). Convention: oracle fixtures are captured from the real fork, never generated by Yune (non-circular).

**`crates/yune-rime-api/src/`:**
- Purpose: C ABI and runtime compatibility surface for RIME-style frontends, plus the TypeDuck-Web export adapter.
- Contains: ABI structs/tables, the `yune_typeduck_*` adapter, session lifecycle, config APIs, context/status/commit APIs, schema deployment/selection, levers, userdb (facade + submodule), runtime path APIs, notifications, modules, key tables, FFI memory helpers.
- Key files: `lib.rs`, `abi.rs`, `api_table.rs`, `typeduck_web.rs`, `session.rs`.

**`crates/yune-rime-api/src/userdb/`:**
- Purpose: User-dictionary storage internals behind the `userdb.rs` facade (`#[path = "userdb/mod.rs"]`).
- Contains: `file_store.rs`, `record.rs`, `recovery.rs`, `snapshot.rs`, `store.rs`, `sync.rs` (+ `mod.rs`).

**`crates/yune-rime-api/src/processors/`:**
- Purpose: Schema-loaded key processing behavior before falling through to `yune-core`.
- Contains: ascii composer, chord composer, editor, key binder, navigator, punctuation, recognizer, selector, shape, speller (+ `mod.rs`).
- Key files: `mod.rs`, `speller.rs`, `key_binder.rs`, `chord_composer.rs`.

**`crates/yune-rime-api/src/tests/`:**
- Purpose: Focused unit-level ABI and compatibility tests within the crate.
- Contains: `abi.rs`, `candidate_api.rs`, `config_api.rs`, `context_status.rs`, `deployment.rs`, `dictionary_data.rs`, `distribution_schema_comparison.rs`, `levers.rs`, `lifecycle_safety.rs`, `resource_id.rs`, `runtime.rs`, `schema_api.rs`, `schema_processors.rs`, `schema_selection.rs`, `session_api.rs`, `userdb.rs` (+ `mod.rs`).

**`crates/yune-rime-api/tests/`:**
- Purpose: Integration tests that drive the exported `RimeApi`/`yune_typeduck_*` surfaces as a frontend would — never internals.
- Contains: `dynamic_loader.rs` (loads the `cdylib` via `libloading` and drives `rime_get_api`), `frontend_client.rs` (function-table client), `frontend_hosts.rs` + `frontend_hosts/` (`mod.rs`, `native.rs`, `native_frontends.rs`, `typeduck_web.rs`), and `typeduck_web.rs` (TypeDuck-Web C ABI integration).

**`crates/yune-rime-api/benches/`:**
- Purpose: Frontend performance baselines.
- Contains: `frontend_baselines.rs`, wired via `[[bench]] name = "frontend_baselines"` (`harness = false`).

**`crates/yune-schema/src/`:**
- Purpose: Standalone typed parser for a minimal RIME schema subset.
- Contains: schema metadata, engine component lists, parse errors, unit tests.
- Key files: `lib.rs`.

**`crates/yune-cli/src/`:**
- Purpose: Local deterministic runner — the **CLI surrogate frontend**. Drives both the core engine and the RIME ABI.
- Contains: command parser, sample engine setup, fixture comparison, JSON transcript formatting, help renderer, RIME ABI-backed frontend surrogate.
- Key files: `main.rs`, `sample_core.rs`, `fixture.rs`, `transcript.rs`, `rime_frontend.rs`.

**`packages/yune-typeduck-runtime/`:**
- Purpose: The browser-side TypeScript runtime (npm package `@yune-ime/typeduck-runtime`) that wraps the WASM build for the WEB-FIRST (M9) path — the most active area of the repo.
- Contains: `src/` (`filesystem.ts`, `index.ts`, `keys.ts`, `module.ts`, `response.ts`, `typeduck.ts`), `test/` (Vitest `*.test.ts` plus fakes `fake-filesystem.ts`/`fake-module.ts`), `dist/` (build output), configured by `package.json` and `tsconfig.json`.

**`scripts/`:**
- Purpose: Build and packaging entry points for the web and native paths.
- Key files: `typeduck-wasm-build.sh` (Emscripten/WASM build targeting `wasm32-unknown-emscripten`; blocks with install guidance if the target or `emcc`/`emar` are missing), `typeduck-exports.txt` (the `yune_typeduck_*` exported-symbol allowlist for the WASM link), `package-typeduck-windows.ps1` (builds the MSVC `cdylib` and packages it as `rime.dll`/`rime.lib` + headers).

**`third_party/typeduck-web/`:**
- Purpose: Vendored TypeDuck-Web upstream plus the Yune integration seam used to validate Yune in a real browser (M9). This is a tracked third-party drop, not first-party code.
- Contains: `source/` (upstream app, includes its own `librime/`), `patches/yune-typeduck-runtime.patch`, `yune-integration/` (`adapter.ts` — the upstream-app seam, `assets.ts`, `package-alias.md`, `README.md`), `e2e/` (Playwright `yune-typeduck.spec.ts`, `playwright.config.ts`, `results/`), `typeduck-web.lock.json` (provenance lockfile), `README.yune-source.md`. Do not hand-edit the vendored tree — apply changes via the patch and lockfile.

**`docs/`:**
- Purpose: Human-readable compatibility strategy, roadmap, and planning context.
- Contains: `analysis.md`, `roadmap.md`, `typeduck-windows-backend-requirements.md`, and `plans/`.
- Convention: active plan docs live under `docs/plans/` and carry a `> **Status:**` banner (status/milestone/created/type); finished or superseded plans move to `docs/plans/archive/` (which includes a `frontend-validation/` subfolder of validation records).

**`fixtures/`:**
- Purpose: Checked-in deterministic CLI fixture outputs plus captured frontend traces.
- Contains: `sample-<case>.json` fixtures consumed by `crates/yune-cli/src/fixture.rs`, and `frontend-traces/` (lifecycle traces for native host, Squirrel, and TypeDuck-Web).

**`.planning/codebase/`:**
- Purpose: Generated codebase maps for planning and execution.
- Contains: `ARCHITECTURE.md`, `CONCERNS.md`, `CONVENTIONS.md`, `INTEGRATIONS.md`, `STACK.md`, `STRUCTURE.md`, `TESTING.md`.

## Key File Locations

**Entry Points:**
- `Cargo.toml`: Rust workspace entry point.
- `crates/yune-core/src/lib.rs`: public Rust API for the core engine crate.
- `crates/yune-rime-api/src/lib.rs`: exported C ABI facade and key processing path.
- `crates/yune-rime-api/src/api_table.rs`: `rime_get_api`/`rime_levers_get_api` and function-table construction.
- `crates/yune-rime-api/src/typeduck_web.rs`: `yune_typeduck_*` C entry points consumed by the WASM build / TS runtime.
- `crates/yune-cli/src/main.rs`: CLI binary entry point.
- `crates/yune-schema/src/lib.rs`: schema parser library entry point.
- `packages/yune-typeduck-runtime/src/index.ts`: TS runtime entry point.

**Configuration:**
- `Cargo.toml`: workspace members, `[workspace.package]` (edition 2021, BSD-3-Clause, repository, MSRV 1.76), and `[workspace.lints]` (`unsafe_code = "forbid"`; clippy `all`/`pedantic` = warn).
- `crates/yune-core/Cargo.toml`: dependency on `regex`; dev-dependency `serde_json` (used by `cantonese_parity.rs`).
- `crates/yune-rime-api/Cargo.toml`: `[lib] crate-type = ["rlib", "cdylib"]` (the `cdylib` produces the C dynamic library consumed by frontends, the WASM build, and the `libloading`-based `dynamic_loader` test, and is renamed to `rime.dll` for Windows). Dependencies: `libc`, `regex`, `serde_json`, `serde_yaml`, `yune-core`; dev-dependency `libloading`; `[[bench]] frontend_baselines`.
- `crates/yune-schema/Cargo.toml`: `serde` + `serde_yaml`.
- `crates/yune-cli/Cargo.toml`: depends on `yune-core` and `yune-rime-api`.
- `packages/yune-typeduck-runtime/package.json` + `tsconfig.json`: TS runtime / Vitest config.
- `.gitignore`, `.gitattributes`, `.editorconfig`: ignore rules and repo-wide EOL/encoding policy.

**Core Logic:**
- `crates/yune-core/src/engine.rs`: composition, candidate refresh, paging, selection, commits.
- `crates/yune-core/src/key.rs`: RIME key sequence parsing and key model.
- `crates/yune-core/src/state.rs`: candidate/context/status data model.
- `crates/yune-core/src/userdb.rs`: core user-dictionary model and commit scoring.
- `crates/yune-core/src/dictionary/source.rs`, `encoder.rs`, `compiled.rs`: source parsing, encoder formulas, compiled metadata/checksums.
- `crates/yune-core/src/translator/mod.rs`: candidate generation.
- `crates/yune-core/src/filter/mod.rs`: candidate filtering (incl. `DictionaryLookupFilter`).
- `crates/yune-core/src/punctuation.rs`, `spelling_algebra.rs`, `comment_format.rs`: punctuation, lookup-side spelling algebra, comment formatting.

**RIME ABI Logic:**
- `crates/yune-rime-api/src/abi.rs`: ABI type layout. **`RimeApi` struct field order IS the ABI** — it must match the TypeDuck fork's `rime_api.h`. The fork adds `config_list_append_bool/int/double/string`.
- `crates/yune-rime-api/src/api_table.rs`: builds the `RimeApi`/`RimeLeversApi` tables (the fork-only `config_list_append_*` entries are wired here).
- `crates/yune-rime-api/src/typeduck_web.rs`: TypeDuck-Web adapter. Defines `YuneTypeDuckState`/`YuneTypeDuckResponse` and the exported `unsafe extern "C"` entry points — `yune_typeduck_init`, `process_key`, `select_candidate`, `delete_candidate`, `flip_page`, `deploy`, `customize`, `cleanup`, `response_json`, `response_handled`, `free_response` — which call through `rime_get_api()` / `rime_levers_get_api()`. The exported symbol allowlist is `scripts/typeduck-exports.txt`.
- `crates/yune-rime-api/src/session.rs`: session registry and `SessionState`.
- `crates/yune-rime-api/src/context_api.rs`, `candidate_api.rs`: context/status/commit reads and candidate iterators.
- `crates/yune-rime-api/src/config.rs`, `config_api.rs`, `config_compiler.rs`: config state, entrypoints, include/patch compilation.
- `crates/yune-rime-api/src/deployment.rs`, `runtime.rs`: deploy/sync/maintenance and runtime trait/path handling.
- `crates/yune-rime-api/src/schema_install.rs`, `schema_selection.rs`, `schema_api.rs`: schema component install, selection workflow, schema status/option API.
- `crates/yune-rime-api/src/key_table.rs`, `levers.rs`, `notifications.rs`, `modules.rs`, `ffi_memory.rs`, `resource_id.rs`: key tables, levers/user-dict manager API, notifications, module registration, FFI memory helpers, resource ids.
- `crates/yune-rime-api/src/userdb.rs` (+ `userdb/`): user-dictionary operations.

**Processor Logic:**
- `crates/yune-rime-api/src/processors/{ascii_composer,chord_composer,editor,key_binder,navigator,punctuation,recognizer,selector,shape,speller}.rs`: the schema-loaded processors, aggregated by `processors/mod.rs`.

**Testing:**
- `crates/yune-core/src/tests/{engine,filter,translator}.rs`: core unit tests (declared via `mod tests;` in `lib.rs`).
- `crates/yune-core/tests/cantonese_parity.rs`: oracle parity vs captured TypeDuck v1.1.2.
- `crates/yune-schema/src/lib.rs`: schema parser unit tests.
- `crates/yune-cli/src/fixture.rs`: fixture check test that scans `fixtures/`.
- `crates/yune-rime-api/src/tests/`: focused ABI/unit modules (`mod.rs` holds shared helpers).
- `crates/yune-rime-api/tests/`: integration harnesses (`frontend_client.rs`, `dynamic_loader.rs`, `frontend_hosts/`, `typeduck_web.rs`).
- `packages/yune-typeduck-runtime/test/*.test.ts`: Vitest suites for the TS runtime.
- `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`: Playwright browser E2E.

**Documentation:**
- `README.md`: overview, goals, current compatibility surface.
- `docs/analysis.md`: strategy, compatibility layers, gaps.
- `docs/roadmap.md`: milestones and active next work (web-first sequencing).
- `docs/typeduck-windows-backend-requirements.md`: the parked Windows engine contract.
- `docs/plans/refactor-plan.md`: module ownership and split guidance.
- `docs/plans/typeduck-web-*.md`, `docs/plans/yune-windows-*.md`: active TypeDuck-Web (M9) and Windows (M10) plans.

## Naming Conventions

**Files:**
- Rust modules use snake_case: `schema_install.rs`, `config_compiler.rs`, `typeduck_web.rs`.
- Module directories with an aggregate file use `mod.rs`: `translator/mod.rs`, `processors/mod.rs`, `userdb/mod.rs`. A sibling facade may re-point at it via `#[path = "..."]` (e.g. `userdb.rs`).
- Test modules mirror feature/API areas: `config_api.rs`, `schema_selection.rs`, `schema_processors.rs`.
- TypeScript runtime files are lowercase: `keys.ts`, `response.ts`, with matching `*.test.ts` (Vitest).
- Fixture files use `sample-<case>.json`: `fixtures/sample-nihao.json`.
- Documentation files use lowercase kebab-case except generated maps: `docs/plans/refactor-plan.md`, `.planning/codebase/ARCHITECTURE.md`.
- The exported WASM C ABI uses the `yune_typeduck_*` prefix (allowlisted in `scripts/typeduck-exports.txt`).

**Directories:**
- Crates use `yune-<area>` names: `crates/yune-core`, `crates/yune-rime-api`.
- The npm package is `@yune-ime/typeduck-runtime` under `packages/`.
- Focused implementation submodules use concept names: `dictionary`, `translator`, `filter`, `processors`, `userdb`, `tests`.
- Generated planning artifacts live under `.planning/`; active plans under `docs/plans/`, finished ones under `docs/plans/archive/`.

**Line endings:** LF by default repo-wide (`.gitattributes` + `.editorconfig`); only `*.bat`/`*.cmd` are CRLF. Keep new files LF to avoid CRLF diffs on Windows checkouts.

## Where to Add New Code

**New Core Engine Behavior:**
- Primary code: `crates/yune-core/src/engine.rs` when behavior changes the generic engine state machine.
- Supporting types: `crates/yune-core/src/state.rs` for candidate/context/status shape changes.
- Tests: `crates/yune-core/src/tests/engine.rs`.

**New Core Translator:**
- Implementation: `crates/yune-core/src/translator/mod.rs`.
- Public export: `crates/yune-core/src/lib.rs`.
- Schema installation: `crates/yune-rime-api/src/schema_install.rs` when schema-driven.
- Tests: `crates/yune-core/src/tests/translator.rs` plus ABI schema tests in `crates/yune-rime-api/src/tests/schema_processors.rs` when installed from RIME config.

**New Core Filter:**
- Implementation: `crates/yune-core/src/filter/mod.rs`.
- Public export: `crates/yune-core/src/lib.rs`.
- Schema installation: `crates/yune-rime-api/src/schema_install.rs`.
- Tests: `crates/yune-core/src/tests/filter.rs` and the matching `crates/yune-rime-api/src/tests/*.rs` module.

**New Dictionary or Encoder Behavior:**
- Source parsing: `crates/yune-core/src/dictionary/source.rs`.
- Compiled metadata/checksum / compiled data: `crates/yune-core/src/dictionary/compiled.rs` (+ `compiled_prism.rs`, `compiled_reverse.rs`, `compiled_table.rs`).
- Table encoder rules: `crates/yune-core/src/dictionary/encoder.rs`.
- Public exports: `crates/yune-core/src/dictionary/mod.rs` and `crates/yune-core/src/lib.rs`.
- Tests: `crates/yune-core/src/tests/` plus ABI deployment/dictionary tests (`crates/yune-rime-api/src/tests/dictionary_data.rs`) when runtime config loads the behavior.

**New RIME ABI Function:**
- ABI struct/function-table shape: `crates/yune-rime-api/src/abi.rs` and `api_table.rs`. Field order must match the TypeDuck fork's `rime_api.h` (the fork adds `config_list_append_bool/int/double/string`).
- Implementation: add to the owning module (`context_api.rs`, `candidate_api.rs`, `config_api.rs`, `deployment.rs`, `levers.rs`, `schema_api.rs`, `schema_selection.rs`, `runtime.rs`, `userdb.rs`, `modules.rs`, …).
- Facade export: `crates/yune-rime-api/src/lib.rs`.
- Tests: matching `crates/yune-rime-api/src/tests/*.rs` plus `crates/yune-rime-api/tests/frontend_client.rs` / `dynamic_loader.rs` when the function table exposes it.

**New TypeDuck-Web Behavior:**
- Rust adapter / C ABI: `crates/yune-rime-api/src/typeduck_web.rs`. New exported functions must be added to the allowlist `scripts/typeduck-exports.txt`.
- TS runtime: `packages/yune-typeduck-runtime/src/<area>.ts` with a matching `test/<area>.test.ts`.
- Integration: `crates/yune-rime-api/tests/typeduck_web.rs` (Rust ABI) and `third_party/typeduck-web/e2e/yune-typeduck.spec.ts` (browser). Vendored-tree changes go through `third_party/typeduck-web/patches/` + the lockfile, not by hand-editing `source/`.

**New TypeDuck-Windows / Native Behavior (parked, resume after M9):**
- The native `rime.dll` is the same `cdylib` (`yune-rime-api`); packaging is `scripts/package-typeduck-windows.ps1`. Engine contract reference: `docs/typeduck-windows-backend-requirements.md` and `docs/plans/yune-windows-*.md`.

**New Schema Processor:**
- Implementation: `crates/yune-rime-api/src/processors/<processor>.rs`; export via `processors/mod.rs`.
- Per-session state: `crates/yune-rime-api/src/session.rs`.
- Installer call: `crates/yune-rime-api/src/schema_selection.rs` and/or `schema_install.rs`.
- Tests: `crates/yune-rime-api/src/tests/schema_processors.rs`.

**New Config Behavior:**
- State/helpers: `crates/yune-rime-api/src/config.rs`; entrypoints: `config_api.rs`; include/patch/build freshness: `config_compiler.rs`.
- Tests: `crates/yune-rime-api/src/tests/config_api.rs` or `deployment.rs`.

**New Deployment or Runtime Behavior:**
- Runtime trait/path fields: `crates/yune-rime-api/src/runtime.rs`; maintenance/deploy/sync: `deployment.rs`; notifications: `notifications.rs`.
- Tests: `crates/yune-rime-api/src/tests/deployment.rs` and `runtime.rs`.

**New CLI Behavior:**
- Command parsing: `crates/yune-cli/src/args.rs`; dispatch: `main.rs`; core-backed sample: `sample_core.rs`; RIME ABI-backed surrogate: `rime_frontend.rs`; output: `transcript.rs`/`render.rs`.
- Tests: same module or `crates/yune-cli/src/fixture.rs` when fixture output changes.

**New Schema Parser Behavior:**
- Implementation and tests: `crates/yune-schema/src/lib.rs`.

**New Fixtures:**
- CLI JSON fixture: `fixtures/sample-<case>.json`; generation/check logic: `crates/yune-cli/src/{sample_core,fixture,transcript}.rs`.
- Oracle parity fixtures: `crates/yune-core/tests/fixtures/typeduck-v1.1.2/` — captured from the real fork, not generated by Yune.

**New Documentation:**
- Overview: `README.md`; strategy/gaps: `docs/analysis.md`; roadmap: `docs/roadmap.md`.
- Plans: `docs/plans/<name>.md` with a `> **Status:**` banner; move finished plans to `docs/plans/archive/`.
- Generated maps: `.planning/codebase/`.

**Utilities:**
- Core helpers belong in the focused core module that owns their domain.
- ABI helpers belong in `config.rs`, `runtime.rs`, `ffi_memory.rs`, or `lib.rs` only when shared across multiple ABI modules.
- Avoid a generic utility module unless two or more existing ownership areas need the same helper.

## Special Directories

**`target/`:**
- Purpose: Cargo build output (incl. `wasm32-unknown-emscripten/` and the MSVC `cdylib`).
- Generated: Yes. Committed: No.

**`packages/yune-typeduck-runtime/{dist,node_modules}/`:**
- Purpose: TS build output / installed deps.
- Generated: Yes. Committed: `dist/` may be; `node_modules/` is not.

**`.planning/`:**
- Purpose: Generated codebase-map artifacts.
- Generated: Yes. Committed: Repository-dependent.

**`fixtures/`:**
- Purpose: Deterministic sample outputs and captured frontend traces.
- Generated: Produced by CLI runs / captured, then checked in as compatibility data.
- Committed: Yes.

**`third_party/typeduck-web/`:**
- Purpose: Vendored upstream + integration seam.
- Generated: Vendored (tracked); `source/node_modules/` is installed, not hand-authored.
- Committed: Tracked drop governed by `typeduck-web.lock.json` and `patches/`.

**`docs/`:**
- Purpose: Human-authored project context, roadmap, and plans.
- Generated: No. Committed: Yes.

**Oracle reference (external):** the librime oracle is upstream `github.com/rime/librime` plus the TypeDuck fork `github.com/TypeDuck-HK/librime` @ **v1.1.2** — a versioned upstream/fork, not a local checkout path.

---

*Last reviewed: 2026-06-17 — refreshed for the TypeDuck-Web (M9) and TypeDuck-Windows (M10) work; current direction is web-first.*
