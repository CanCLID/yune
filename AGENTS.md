<!-- GSD:project-start source:PROJECT.md -->
## Project

**Yune**

Yune is a Rust input-method engine project that uses librime as the external
compatibility oracle while avoiding a direct C++ architecture clone. It already
has a deterministic core, a focused RIME-style C ABI shim, schema-loaded
compatibility slices, and frontend-style ABI tests. The next milestone is to
turn that compatibility surface into a stronger frontend and data-compatibility
validation path without losing the clean module boundaries established by the
recent refactor.

**Core Value:** Existing RIME schemas and frontends should behave predictably through Yune's
Rust implementation, with every compatibility difference measurable against
librime before it is accepted.

### Constraints

- **Compatibility**: `/Users/trenton/Projects/librime` is the external behavior
  oracle for user-visible behavior, schema semantics, ABI contracts, and
  migration support.
- **Architecture**: Prefer typed, idiomatic Rust modules over cloning librime's
  internal C++ structure when the boundary contract is preserved.
- **Testing**: Run focused tests for each behavior slice and `cargo test
  --workspace` after broader phases; use `cargo clippy --workspace --all-targets
  -- -D warnings` as the quality gate when implementation changes warrant it.
- **Frontend validation**: The CLI frontend is an intermediate validation layer;
  it is not proof that Squirrel, Weasel, ibus-rime, fcitx-rime, or fcitx5-rime
  integration is complete.
- **Data compatibility**: Source `.dict.yaml` support is not enough for
  production-scale compatibility; compiled `.table.bin`, `.prism.bin`, and
  `.reverse.bin` payloads remain a required direction.
- **Security**: Runtime resource identifiers must be treated as logical IDs, not
  arbitrary filesystem paths.
<!-- GSD:project-end -->

<!-- GSD:stack-start source:codebase/STACK.md -->
## Technology Stack

## Languages
- Rust 2021 edition - all production crates under `crates/`; workspace metadata in `Cargo.toml` sets `rust-version = "1.76"`.
- Markdown - project notes in `README.md`, `docs/analysis.md`, `docs/roadmap.md`, and `docs/refactor-plan.md`.
- JSON - deterministic CLI fixtures in `fixtures/*.json`; handwritten JSON rendering lives in `crates/yune-cli/src/transcript.rs`.
- YAML - RIME schema/config/user data compatibility is parsed and emitted by `crates/yune-schema/src/lib.rs`, `crates/yune-rime-api/src/config_api.rs`, `crates/yune-rime-api/src/config_compiler.rs`, and `crates/yune-rime-api/src/deployment.rs`.
- C ABI surface - Rust exposes librime-shaped `extern "C"` APIs from `crates/yune-rime-api/src/*` using `#[repr(C)]` structs from `crates/yune-rime-api/src/abi.rs`.
## Runtime
- Rust toolchain required; repo minimum is Rust 1.76 from `Cargo.toml`.
- Local scan toolchain: `rustc 1.95.0` and `cargo 1.95.0`.
- No `rust-toolchain.toml` or `.cargo/config.toml` is present; use the active developer toolchain.
- Cargo workspace with resolver 2 in `Cargo.toml`.
- Lockfile: present at `Cargo.lock`.
- Workspace members: `crates/yune-core`, `crates/yune-schema`, `crates/yune-rime-api`, and `crates/yune-cli`.
## Frameworks
- Rust standard library - process state, filesystem operations, FFI, and synchronization throughout `crates/yune-core/src/*` and `crates/yune-rime-api/src/*`.
- `yune-core` 0.1.0 - input engine, session state, translators, filters, candidate ranking hooks, key handling, punctuation, spelling algebra, and dictionary parsing in `crates/yune-core/src/lib.rs` and `crates/yune-core/src/engine.rs`.
- `yune-schema` 0.1.0 - minimal RIME schema compatibility parser in `crates/yune-schema/src/lib.rs`.
- `yune-rime-api` 0.1.0 - RIME-style C ABI shim, session registry, config APIs, deployment helpers, levers module, and frontend-facing function table in `crates/yune-rime-api/src/lib.rs`, `crates/yune-rime-api/src/abi.rs`, and `crates/yune-rime-api/src/api_table.rs`.
- `yune-cli` 0.1.0 - local fixture runner and diagnostics CLI in `crates/yune-cli/src/main.rs`; the RIME API-driven frontend slot is currently reserved in `crates/yune-cli/src/rime_frontend.rs`.
- Rust built-in test harness via `cargo test`.
- Inline unit tests live under `#[cfg(test)]` in files such as `crates/yune-core/src/lib.rs`, `crates/yune-cli/src/args.rs`, and `crates/yune-schema/src/lib.rs`.
- RIME ABI compatibility tests live in `crates/yune-rime-api/src/tests/*.rs`.
- Frontend-style API-table coverage lives in `crates/yune-rime-api/tests/frontend_client.rs`.
- JSON compatibility fixtures live in `fixtures/sample-nihao.json`, `fixtures/sample-composing.json`, `fixtures/sample-backspace.json`, and `fixtures/sample-punctuation.json`.
- Build with Cargo from the workspace root: `cargo build`, `cargo test`, `cargo run -p yune-cli`.
- No build scripts (`build.rs`) are present.
- Root workspace metadata in `Cargo.toml` declares `edition = "2021"`, `license = "BSD-3-Clause"`, `repository = "https://github.com/yune-ime/yune"`, and `rust-version = "1.76"`.
- Root workspace lint declarations in `Cargo.toml` set `unsafe_code = "forbid"` and Clippy `all`/`pedantic` to warn; member manifests do not contain per-crate `[lints]` sections.
## Key Dependencies
- `regex` 1.12.3 locked by `Cargo.lock` - used for spelling algebra, comment formatting, table encoder exclude patterns, RIME recognizer/speller patterns, and chord output transforms in `crates/yune-core/src/spelling_algebra.rs`, `crates/yune-core/src/comment_format.rs`, `crates/yune-core/src/dictionary/encoder.rs`, `crates/yune-rime-api/src/schema_install.rs`, and `crates/yune-rime-api/src/processors/*`.
- `serde` 1.0.228 locked by `Cargo.lock` - derives schema structures in `crates/yune-schema/src/lib.rs`.
- `serde_yaml` 0.9.34+deprecated locked by `Cargo.lock` - parses and writes RIME schema/config/deployment YAML in `crates/yune-schema/src/lib.rs`, `crates/yune-rime-api/src/config.rs`, `crates/yune-rime-api/src/config_api.rs`, `crates/yune-rime-api/src/config_compiler.rs`, `crates/yune-rime-api/src/deployment.rs`, `crates/yune-rime-api/src/levers.rs`, and `crates/yune-rime-api/src/runtime.rs`.
- `libc` 0.2.186 locked by `Cargo.lock` - used on Unix for librime-compatible signature time formatting in `crates/yune-rime-api/src/lib.rs`.
- `yune-core` path dependency - consumed by `crates/yune-rime-api/Cargo.toml` and `crates/yune-cli/Cargo.toml`.
- Transitive regex stack (`aho-corasick`, `memchr`, `regex-automata`, `regex-syntax`) - pulled through `regex` in `Cargo.lock`.
- Transitive serde stack (`serde_core`, `serde_derive`, `unsafe-libyaml`, `indexmap`, `itoa`, `ryu`) - pulled through `serde`/`serde_yaml` in `Cargo.lock`.
## Configuration
- No required process environment variables are detected.
- Runtime paths come from `RimeTraits` fields (`shared_data_dir`, `user_data_dir`, `prebuilt_data_dir`, `staging_dir`, `log_dir`) in `crates/yune-rime-api/src/abi.rs` and are normalized by `crates/yune-rime-api/src/runtime.rs`.
- Runtime installation settings are read from `installation.yaml` in the user data directory by `crates/yune-rime-api/src/runtime.rs`.
- RIME config data is loaded from shared, prebuilt, staged, and user YAML files through `crates/yune-rime-api/src/config_api.rs`, `crates/yune-rime-api/src/config_compiler.rs`, and `crates/yune-rime-api/src/deployment.rs`.
- Build-time Cargo metadata is used through `env!("CARGO_PKG_VERSION")` in `crates/yune-rime-api/src/lib.rs`; CLI tests use `env!("CARGO_MANIFEST_DIR")` in `crates/yune-cli/src/fixture.rs`.
- Workspace manifest: `Cargo.toml`.
- Crate manifests: `crates/yune-core/Cargo.toml`, `crates/yune-schema/Cargo.toml`, `crates/yune-rime-api/Cargo.toml`, and `crates/yune-cli/Cargo.toml`.
- Lockfile: `Cargo.lock`.
- No package.json, pyproject, go.mod, or other language package manifests are present.
## Platform Requirements
- Rust/Cargo compatible with Rust 1.76 or newer.
- Run commands from repository root so workspace paths and CLI fixture lookup behave consistently.
- The code relies on standard filesystem access for fixtures, RIME shared/user data directories, deployment staging, sync snapshots, and log cleanup.
- Deploy as Rust libraries/binaries produced by Cargo.
- `yune-rime-api` is a librime-shaped ABI shim whose manifest declares `crate-type = ["rlib", "cdylib"]`; use `scripts/package-typeduck-windows.ps1` to package the MSVC cdylib as `rime.dll` plus `rime.lib` and headers for TypeDuck-Windows.
- Runtime callers must provide or accept defaults for `RimeTraits` paths so `crates/yune-rime-api/src/runtime.rs` can locate shared config, user config, staging, prebuilt data, sync snapshots, and logs.
- Network access is not part of the current runtime stack.
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

## Naming Patterns
- Use Rust module filenames in `snake_case`, with conceptual submodules under focused directories such as `crates/yune-core/src/dictionary/source.rs`, `crates/yune-core/src/dictionary/compiled.rs`, `crates/yune-rime-api/src/processors/key_binder.rs`, and `crates/yune-rime-api/src/schema_selection.rs`.
- Use `mod.rs` only for directory module roots such as `crates/yune-core/src/dictionary/mod.rs`, `crates/yune-core/src/filter/mod.rs`, `crates/yune-core/src/translator/mod.rs`, `crates/yune-rime-api/src/processors/mod.rs`, and `crates/yune-rime-api/src/tests/mod.rs`.
- Keep crate package names in kebab-case in manifests: `crates/yune-core/Cargo.toml`, `crates/yune-schema/Cargo.toml`, `crates/yune-rime-api/Cargo.toml`, and `crates/yune-cli/Cargo.toml`.
- Keep checked-in CLI fixtures under `fixtures/` with `sample-*.json` names such as `fixtures/sample-nihao.json` and `fixtures/sample-punctuation.json`.
- Use `snake_case` for Rust functions and methods, including behavior-heavy APIs such as `Engine::process_key_event` in `crates/yune-core/src/engine.rs`, `TableDictionary::parse_rime_dict_yaml_with_imports` in `crates/yune-core/src/dictionary/source.rs`, and `Schema::parse_rime_yaml` in `crates/yune-schema/src/lib.rs`.
- Use `RimePascalCase` only for exported librime-shaped C ABI functions with `#[no_mangle]`, for example `RimeConfigOpen` in `crates/yune-rime-api/src/config_api.rs` and `RimeSetup` in `crates/yune-rime-api/src/runtime.rs`.
- Name tests as long, behavior-specific `snake_case` sentences, for example `processes_ascii_keys_and_returns_unread_commit_once` in `crates/yune-rime-api/src/tests/session_api.rs` and `checked_in_fixtures_match_cli_output` in `crates/yune-cli/src/fixture.rs`.
- Use `snake_case` locals and fields, with `is_` / `has_` prefixes for booleans such as `Status::is_ascii_mode` in `crates/yune-core/src/state.rs`, `has_selectable_candidates` in `crates/yune-core/src/engine.rs`, and `is_last_page` in `crates/yune-rime-api/src/abi.rs`.
- Use descriptive temporary names over abbreviations when crossing ABI or schema boundaries, such as `shared_data_dir`, `user_data_dir`, `prebuilt_data_dir`, and `backup_config_files` in `crates/yune-rime-api/src/runtime.rs`.
- Use `_guard` for intentionally held test mutex guards, as in `crates/yune-rime-api/src/tests/session_api.rs`, to serialize process-wide runtime state.
- Use `UpperCamelCase` for structs, enums, traits, and error types such as `Engine`, `CandidateRanker`, `TableDictionaryParseError`, `RimeConfigIterator`, and `SchemaParseError`.
- Keep C ABI mirror types prefixed with `Rime` and marked `#[repr(C)]` in `crates/yune-rime-api/src/abi.rs`.
- Derive common traits near type declarations. Current types commonly derive combinations of `Clone`, `Copy`, `Debug`, `Default`, `Eq`, `Hash`, and `PartialEq`, as in `KeyModifiers` and `KeyCode` in `crates/yune-core/src/key.rs`.
## Code Style
- Use `rustfmt` through `cargo fmt`; no repo-specific `rustfmt.toml` or `.rustfmt.toml` is present.
- Use Rust 2021 syntax with workspace MSRV `1.76` from `Cargo.toml`; avoid newer standard-library helpers unless the MSRV is raised.
- Keep early-return `let Some(...) = ... else { return ...; };` and `let Ok(...) = ... else { return ...; };` patterns for validation-heavy code, as in `crates/yune-rime-api/src/config_api.rs` and `crates/yune-rime-api/src/runtime.rs`.
- Prefer small focused production modules. Keep `crates/yune-core/src/lib.rs` and `crates/yune-rime-api/src/lib.rs` as public facades and glue; add new implementation work to focused modules such as `crates/yune-core/src/key.rs` or `crates/yune-rime-api/src/processors/speller.rs`.
- Treat the root `Cargo.toml` lint policy as the intended standard: `[workspace.lints.clippy] all = "warn"` and `pedantic = "warn"`.
- Use the documented quality gate from `docs/refactor-plan.md`: `cargo clippy --workspace --all-targets -- -D warnings`.
- Public pure accessors and constructors commonly carry `#[must_use]`, for example `Engine::new` in `crates/yune-core/src/engine.rs`, `Schema::minimal` in `crates/yune-schema/src/lib.rs`, and `TableEntry::new` in `crates/yune-core/src/dictionary/source.rs`.
- FFI boundary functions use explicit `unsafe extern "C" fn` signatures plus `# Safety` docs and local `// SAFETY:` comments, as in `crates/yune-rime-api/src/config_api.rs`, `crates/yune-rime-api/src/runtime.rs`, and `crates/yune-rime-api/src/ffi_memory.rs`.
## Import Organization
- No custom path aliases are configured. Use crate names from workspace manifests, for example `yune_core` in `crates/yune-rime-api/src/schema_install.rs` and `crates/yune-cli/src/sample_core.rs`.
- Within a crate, use `crate::...` for cross-module access and `super::...` for sibling or parent module access, as in `crates/yune-core/src/punctuation.rs`, `crates/yune-core/src/filter/mod.rs`, and `crates/yune-rime-api/src/tests/session_api.rs`.
## Error Handling
- Library parsing code returns custom error types implementing `Display` and `Error`, such as `KeySequenceParseError` in `crates/yune-core/src/key.rs` and `SchemaParseError` in `crates/yune-schema/src/lib.rs`.
- CLI code returns `Result<(), String>` from `run` and maps errors to `stderr` plus `ExitCode::FAILURE` in `crates/yune-cli/src/main.rs`.
- C ABI functions return librime-shaped `Bool` values or null pointers instead of panicking. Validate null pointers and invalid C strings at the boundary, as in `crates/yune-rime-api/src/config_api.rs` and `crates/yune-rime-api/src/candidate_api.rs`.
- Use `expect` for internal invariants and test setup, not for ordinary user input. Examples include mutex poisoning checks in `crates/yune-rime-api/src/runtime.rs` and fixture/test setup checks in `crates/yune-rime-api/src/tests/mod.rs`.
- When a function is compatibility-oriented, preserve librime-shaped fallback behavior explicitly, such as missing config open behavior in `crates/yune-rime-api/src/tests/config_api.rs`.
## Logging
- No `log` or `tracing` dependency is present in workspace manifests. Use `println!` only for CLI user output in `crates/yune-cli/src/main.rs`, `crates/yune-cli/src/render.rs`, and `crates/yune-cli/src/fixture.rs`.
- Use `eprintln!` for CLI errors at the process boundary in `crates/yune-cli/src/main.rs`.
- Library crates should avoid unsolicited output. Return structured values, `Result`, `Bool`, or null pointers instead of logging from `crates/yune-core/src/*`, `crates/yune-schema/src/lib.rs`, and `crates/yune-rime-api/src/*`.
## Comments
- Document FFI safety requirements with Rustdoc `# Safety` sections on unsafe public functions and `// SAFETY:` comments next to unsafe blocks, following `crates/yune-rime-api/src/config_api.rs` and `crates/yune-rime-api/src/runtime.rs`.
- Use comments to explain compatibility behavior, struct layout, ownership, or intentionally unusual fallbacks. Avoid restating straightforward Rust control flow.
- Keep docs under `docs/` for roadmap and refactor guidance; source comments should stay local to non-obvious invariants.
- Not applicable. The repo is Rust-only.
- Use Rustdoc on public APIs when safety, ownership, or compatibility semantics are not self-evident, especially in `crates/yune-rime-api/src/abi.rs`, `crates/yune-rime-api/src/config_api.rs`, and `crates/yune-rime-api/src/runtime.rs`.
## Function Design
## Module Design
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

## System Overview
```text
| RIME frontends / ABI  |          | Local fixture CLI            |
| `yune-rime-api` C ABI |          | `crates/yune-cli/src/main.rs`|
| Compatibility and runtime adapter layer                       |
| `crates/yune-rime-api/src/lib.rs`                             |
| sessions, key conversion, config, deployment, schema install   |
| Core input engine                |    | Schema model subset    |
| `crates/yune-core/src/engine.rs` |    | `crates/yune-schema/`  |
| translators, filters, rankers    |    | parse RIME YAML shape |
| Runtime state, candidates, dictionaries, fixture output        |
| `crates/yune-core/src/state.rs`, `crates/yune-core/src/`       |
```
## Component Responsibilities
| Component | Responsibility | File |
|-----------|----------------|------|
| Workspace | Defines the Rust workspace, crate membership, shared edition, MSRV, license, and lint level. | `Cargo.toml` |
| Core facade | Exposes the stable Rust API for engine state, translators, filters, dictionary helpers, key parsing, and the AI ranking hook. | `crates/yune-core/src/lib.rs` |
| Core engine | Owns composition state mutation, candidate refresh, selection, commits, paging, and trait invocation order. | `crates/yune-core/src/engine.rs` |
| Core state | Defines candidates, composition, context, status, snapshots, and commit history records. | `crates/yune-core/src/state.rs` |
| Key model | Converts RIME-style sequence names into typed `KeyEvent` values used by the core and ABI shim. | `crates/yune-core/src/key.rs` |
| Dictionary model | Parses RIME table dictionaries, imports, packs, preset vocabulary, compiled metadata, checksums, and table encoder rules. | `crates/yune-core/src/dictionary/` |
| Translators | Provide echo, table, reverse lookup, history, switch, and schema-list candidate generation. | `crates/yune-core/src/translator/mod.rs` |
| Filters | Reorder, deduplicate, convert, tag-gate, or annotate candidates after translation. | `crates/yune-core/src/filter/mod.rs` |
| RIME ABI facade | Exports C ABI entrypoints, key routing, public function table exports, and cross-module glue. | `crates/yune-rime-api/src/lib.rs` |
| ABI layout | Defines librime-shaped structs, type aliases, and function table structs. | `crates/yune-rime-api/src/abi.rs` |
| API table | Builds static `RimeApi` and `RimeLeversApi` function tables. | `crates/yune-rime-api/src/api_table.rs` |
| Session registry | Owns process-wide session IDs, session state, lifecycle checks, cleanup, and session lookup helpers. | `crates/yune-rime-api/src/session.rs` |
| Runtime paths | Stores process-wide RIME traits and resolves shared, user, prebuilt, staging, sync, and log paths. | `crates/yune-rime-api/src/runtime.rs` |
| Schema install | Converts deployed schema YAML into core translators, filters, segment tags, and segmentor data. | `crates/yune-rime-api/src/schema_install.rs` |
| Schema selection | Applies schema resets and installs all session processors and core chains for a selected schema. | `crates/yune-rime-api/src/schema_selection.rs` |
| Processor modules | Implement schema-loaded key handling for ascii composer, chord composer, editor, key binder, navigator, punctuation, recognizer, selector, shape, and speller. | `crates/yune-rime-api/src/processors/` |
| Config API | Exposes deployed/user config open, scalar access, mutation, iterators, and YAML-backed config state. | `crates/yune-rime-api/src/config_api.rs` |
| Config compiler | Handles librime-style include, patch, custom patch, and build-info freshness behavior. | `crates/yune-rime-api/src/config_compiler.rs` |
| Deployment | Implements initialization, maintenance, deploy, sync, staging, and notification-facing runtime operations. | `crates/yune-rime-api/src/deployment.rs` |
| Candidate/context APIs | Copy engine state into caller-owned C ABI structures and candidate iterators. | `crates/yune-rime-api/src/context_api.rs`, `crates/yune-rime-api/src/candidate_api.rs` |
| Schema crate | Provides a small typed RIME schema subset parser independent of the runtime ABI shim. | `crates/yune-schema/src/lib.rs` |
| CLI harness | Runs deterministic sample sequences and checks JSON fixtures directly against `yune-core`. | `crates/yune-cli/src/` |
## Pattern Overview
- Keep externally observable RIME compatibility at the boundary in `crates/yune-rime-api/src/`.
- Keep reusable engine behavior in `crates/yune-core/src/` behind Rust traits and typed state.
- Keep `lib.rs` and `main.rs` as facades and orchestration glue; put owned behavior in focused modules.
- Convert deployed YAML configuration into installed session processors, translators, filters, and segment tags.
- Preserve classic input behavior when optional ranking or schema behavior is absent.
## Layers
- Purpose: Define crate composition and shared Rust metadata.
- Location: `Cargo.toml`
- Contains: workspace members, resolver, edition, license, MSRV, lint policy.
- Depends on: Not applicable.
- Used by: all crates under `crates/`.
- Purpose: Represent input-method state and deterministic candidate generation.
- Location: `crates/yune-core/src/`
- Contains: `Engine`, `Context`, `Status`, key parsing, translators, filters, dictionary parsing, table encoding, punctuation, spelling algebra, AI ranker trait.
- Depends on: `regex` for parsing and pattern application.
- Used by: `crates/yune-cli`, `crates/yune-rime-api`.
- Purpose: Present a librime-shaped C ABI and translate frontend calls into core engine mutations.
- Location: `crates/yune-rime-api/src/`
- Contains: ABI structs, process-wide runtime state, session registry, config/deployment APIs, function table builders, schema installation, processor routing, FFI memory cleanup.
- Depends on: `yune-core`, `libc`, `regex`, `serde_yaml`.
- Used by: integration tests and any frontend loading the exported ABI symbols.
- Purpose: Parse a minimal standalone RIME schema subset into typed Rust values.
- Location: `crates/yune-schema/src/lib.rs`
- Contains: `Schema`, `EngineSpec`, YAML parsing, missing-field errors.
- Depends on: `serde`, `serde_yaml`.
- Used by: schema compatibility work that needs a small typed schema model.
- Purpose: Provide deterministic command-line fixture generation and checking for core behavior.
- Location: `crates/yune-cli/src/`
- Contains: argument parsing, sample core runner, fixture comparison, JSON transcript rendering.
- Depends on: `yune-core`.
- Used by: fixture workflows and smoke checks.
- Purpose: Describe current compatibility scope and store checked-in expected outputs.
- Location: `README.md`, `docs/`, `fixtures/`
- Contains: roadmap, analysis, refactor guidance, sample JSON fixtures.
- Depends on: source behavior but is not compiled.
- Used by: planning, compatibility context, CLI fixture tests.
## Data Flow
### Primary RIME API Key Path
### Schema Selection and Installation Flow
### CLI Fixture Flow
- `yune-core` keeps session-local mutable state inside each `Engine`.
- `yune-rime-api` keeps process-wide mutable runtime state in `OnceLock<Mutex<_>>` registries and an `AtomicBool` service flag.
- FFI functions copy Rust-owned state into caller-owned C structures and pair allocations with explicit `RimeFree*` functions.
- Config data is YAML-backed `serde_yaml::Value` stored behind `RimeConfig.ptr`.
## Key Abstractions
- Purpose: Own one input-method state machine.
- Examples: `crates/yune-core/src/engine.rs`, `crates/yune-core/src/state.rs`
- Pattern: mutable session object with plug-in translators, filters, and rankers.
- Purpose: Convert current composition input into candidate vectors.
- Examples: `crates/yune-core/src/lib.rs`, `crates/yune-core/src/translator/mod.rs`, `crates/yune-core/src/punctuation.rs`
- Pattern: `Send + Sync` trait object installed into `Engine.translators`.
- Purpose: Mutate candidate vectors after translation.
- Examples: `crates/yune-core/src/lib.rs`, `crates/yune-core/src/filter/mod.rs`
- Pattern: `Send + Sync` trait object with option-aware and context-aware hooks.
- Purpose: Allow optional non-blocking candidate reranking without changing fallback order.
- Examples: `crates/yune-core/src/lib.rs`
- Pattern: `RerankResult::Pending` preserves classic order; `RerankResult::Ready` replaces candidate order.
- Purpose: Bridge one RIME session to one `Engine` plus schema-loaded processor state.
- Examples: `crates/yune-rime-api/src/session.rs`, `crates/yune-rime-api/src/processors/`
- Pattern: registered by numeric `RimeSessionId` in a mutex-protected process-wide registry.
- Purpose: Resolve deployed/user YAML and expose librime-style path and config APIs.
- Examples: `crates/yune-rime-api/src/runtime.rs`, `crates/yune-rime-api/src/config.rs`, `crates/yune-rime-api/src/config_api.rs`
- Pattern: process-wide path state plus per-config heap state owned through `RimeConfig.ptr`.
- Purpose: Split `component@namespace` declarations and install matching processors, translators, filters, and segmentors.
- Examples: `crates/yune-rime-api/src/schema_install.rs`
- Pattern: string component registry implemented with match statements and config helpers.
## Entry Points
- Location: `crates/yune-core/src/lib.rs`
- Triggers: Rust crates instantiate `Engine` or use public parser/type exports.
- Responsibilities: stable API surface for core engine composition, dictionary parsing, translation/filter/ranking, and key parsing.
- Location: `crates/yune-rime-api/src/lib.rs`, `crates/yune-rime-api/src/api_table.rs`
- Triggers: C ABI symbol lookup, `rime_get_api`, direct exported `Rime*` calls.
- Responsibilities: maintain librime-compatible function table, session lifecycle, key processing, context/status/commit reads, config APIs, deployment helpers.
- Location: `crates/yune-cli/src/main.rs`
- Triggers: `cargo run -p yune-cli -- ...`
- Responsibilities: run sample sequences, check fixtures, print deterministic JSON or help.
- Location: `crates/yune-schema/src/lib.rs`
- Triggers: Rust callers parse schema YAML.
- Responsibilities: parse schema metadata and engine component lists into typed structs.
- Location: `crates/yune-core/src/lib.rs`, `crates/yune-rime-api/src/tests/`, `crates/yune-rime-api/tests/frontend_client.rs`, `crates/yune-cli/src/fixture.rs`
- Triggers: `cargo test --workspace`.
- Responsibilities: lock down core behavior, ABI compatibility, frontend-style function table use, and fixture stability.
## Architectural Constraints
- **Threading:** Runtime and sessions use mutex-protected process globals in `crates/yune-rime-api/src/session.rs` and `crates/yune-rime-api/src/runtime.rs`; the core engine itself is ordinary single-session mutable state.
- **Global state:** `sessions()`, `service_started()`, `runtime_paths()`, notification handler state, API function tables, module registries, state-label cache, and config/user dictionary process state are all module-level globals under `crates/yune-rime-api/src/`.
- **Unsafe boundary:** C ABI functions dereference caller pointers and allocate C strings. Keep unsafe pointer work in ABI/config/context/candidate/FFI memory modules, not in `yune-core`.
- **Circular imports:** Rust module cycles are not detected by the compiler and are not present. The RIME ABI facade uses `pub use` and `pub(crate) use` re-exports from `crates/yune-rime-api/src/lib.rs`; avoid introducing module dependencies that require moving owned logic back into the facade.
- **Compatibility boundary:** External behavior is shaped by RIME/librime contracts in `crates/yune-rime-api/src/`; internal Rust design can stay idiomatic when the boundary remains compatible.
- **Project skills:** `.codex/skills/` and `.agents/skills/` are not detected in this repository.
## Anti-Patterns
### Adding Owned Behavior To Facades
### Bypassing Schema Installation
### Leaking ABI Allocation Ownership Into Core
### Hand-Rolling Duplicate Config Lookup
## Error Handling
- Return typed errors for core parser failures: `TableDictionaryParseError`, `TableEncoderFormulaError`, `KeySequenceParseError`, `SchemaParseError`.
- Return `FALSE` or null from C ABI functions when session IDs, pointers, masks, config handles, or string conversions are invalid.
- Use explicit `// SAFETY:` comments around pointer operations in ABI-facing unsafe functions.
- Convert lossy or invalid C strings defensively at the ABI boundary.
- Keep commit failures as `None` in core engine methods such as `Engine::commit_composition`.
## Cross-Cutting Concerns
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->
## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, `.github/skills/`, or `.codex/skills/` with a `SKILL.md` index file.
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->



<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
