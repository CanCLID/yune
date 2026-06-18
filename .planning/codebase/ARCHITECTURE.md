<!-- refreshed: 2026-06-17 -->
# Architecture

**Analysis Date:** 2026-06-17

> Line numbers in this doc drift as files grow; prefer the symbol/path names when
> navigating, and treat any `file:line` as a hint to confirm, not a fixed anchor.

## System Overview

Yune is a Rust input-method engine fronted by a single librime-shaped C ABI.
The engine (`yune-core`) holds all deterministic input behavior behind traits;
the ABI crate (`yune-rime-api`) is the only compatibility surface. Everything
external consumes the engine **through that ABI**, and there are now three
concrete consumers (see *Frontend Consumers of the C ABI*).

```text
+----------------------+   +--------------------------+   +-------------------------+
| CLI surrogate        |   | TypeDuck-Web (WASM)      |   | TypeDuck-Windows native |
| yune-cli rime_frontend|  | yune_typeduck_* adapter  |   | drop-in rime.dll        |
+----------+-----------+   +------------+-------------+   +-----------+-------------+
           |                            |                             |
           +--------------+-------------+--------------+--------------+
                          v                            v
            +-------------------------------------------------------+
            | RIME compatibility / runtime adapter layer            |
            | crates/yune-rime-api/src/ (rlib + cdylib)             |
            | RimeApi/RimeLeversApi tables, sessions, key routing,  |
            | config, deployment, schema install, processors        |
            +-----------------------+-------------------------------+
                                    |
                  +-----------------+-----------------+
                  v                                   v
       +----------------------------+      +------------------------+
       | Core input engine          |      | Schema model subset    |
       | crates/yune-core/src/      |      | crates/yune-schema/    |
       | translators, filters,      |      | parse RIME YAML shape  |
       | rankers, dictionaries      |      +------------------------+
       +----------------------------+
```

`yune-rime-api` declares `crate-type = ["rlib", "cdylib"]`
(`crates/yune-rime-api/Cargo.toml`). The **rlib** is linked by in-process tests
and by `yune-cli`; the **cdylib** is the artifact loaded by native frontends
(shipped as `rime.dll`) and compiled to WebAssembly for the browser.

## Direction / Sequencing

The current direction is **web-first**. The active milestone (**M9,
TypeDuck-Web**) validates Yune in a real browser through the WASM adapter before
resuming the **parked** native milestone (**M10, TypeDuck-Windows**). M10 had a
first pass land early (the fork-only ABI slots and comment-panel filter described
below), but platform packaging work is deferred until web validation completes.
See `docs/roadmap.md` and `docs/plans/typeduck-web-validation-plan.md`.

**AI-native input is an explicitly separate, later layer** above librime
compatibility — not part of M9 or M10. The only hook reserved for it today is the
`CandidateRanker` / `RerankResult` seam in `yune-core`
(`crates/yune-core/src/lib.rs`, trait near the top); `MockAiRanker` is a
placeholder and is non-blocking (`RerankResult::Pending` preserves classic
order). No AI provider, context, memory, or privacy machinery exists yet. See
`docs/plans/archive/ai-native-frontend-readiness.md` and `docs/analysis.md`.

## Behavior Oracle

The compatibility oracle is **upstream librime**
(`https://github.com/rime/librime`) plus the **TypeDuck fork**
(`https://github.com/TypeDuck-HK/librime`, tag `v1.1.2`) — referenced upstream,
not a checkout on disk. Parity tests are **oracle-driven and non-circular**:
they compare against bytes captured from the real fork, not against Yune's own
output. The canonical example is `crates/yune-core/tests/cantonese_parity.rs`,
which checks the `DictionaryLookupFilter` comment bytes against the checked-in
fixture under `crates/yune-core/tests/fixtures/typeduck-v1.1.2/` (provenance,
including the exact engine/plugin/schema commits, is recorded in that fixture's
`README.md`).

## Frontend Consumers of the C ABI

All three consumers drive the same exported function table; none reach into
`yune-core` directly.

**1. CLI surrogate (`crates/yune-cli/src/rime_frontend.rs`).** The in-tree
frontend stand-in. `run_frontend` calls `rime_get_api()` and drives the full
librime lifecycle through the table:
setup → initialize → deploy → create-session → select-schema → process-key →
read context/status/commit → destroy-session → cleanup → finalize (cleanup is
RAII via `CleanupGuard`). `main.rs` dispatches this as the `Frontend` command
(human or JSON output) and `FrontendCheck` for fixture comparison, alongside the
older direct-to-core `Run`/`Check` fixture flow (`sample_core.rs`), which still
coexists.

**2. TypeDuck-Web (WASM).** `yune-rime-api` compiles to
`wasm32-unknown-emscripten` via `scripts/typeduck-wasm-build.sh`, exporting the
simplified `yune_typeduck_*` C API defined in
`crates/yune-rime-api/src/typeduck_web.rs`
(`init`, `process_key`, `select_candidate`, `delete_candidate`, `flip_page`,
`deploy`, `customize`, `cleanup`, `response_json`, `response_handled`,
`free_response`). The exported-symbol list is locked in
`scripts/typeduck-exports.txt` and passed to Emscripten as
`-sEXPORTED_FUNCTIONS`. The `@yune-ime/typeduck-runtime` TypeScript package
(`packages/yune-typeduck-runtime/src/{index,typeduck,response,keys,module,filesystem}.ts`)
wraps the module, and the upstream app integrates through the tracked seam
`third_party/typeduck-web/yune-integration/adapter.ts`.

**3. TypeDuck-Windows native.** The same cdylib ships as a drop-in `rime.dll`
(packaged by `scripts/package-typeduck-windows.ps1`). This consumer requires two
deliberate divergences from upstream librime — see *TypeDuck-Windows ABI
contract* below.

## TypeDuck-Windows ABI contract

This consumer is why the ABI carries fork-only extensions; an architecture reader
must know these are intentional.

- **Fork-only `RimeApi` slots.** The table includes
  `config_list_append_bool` / `_int` / `_double` / `_string`
  (declared in `crates/yune-rime-api/src/abi.rs`, populated in
  `crates/yune-rime-api/src/api_table.rs`). These do not exist in upstream
  librime; they match the TypeDuck fork's `rime_api.h`.
- **Field order IS the ABI.** The `RimeApi` struct field order MUST match the
  TypeDuck fork's `rime_api.h` because slot position is the binary contract.
  Positions are locked by `assert_api_slot!` tests in
  `crates/yune-rime-api/src/tests/abi.rs` (e.g. the four `config_list_append_*`
  slots at fixed indices). Never reorder or insert table fields without updating
  these locks and confirming against the fork header.
- **Comment panel.** `yune-core` ships a `DictionaryLookupFilter`
  (`crates/yune-core/src/filter/mod.rs`) that emits the TypeDuck comment-panel
  bytes (leading `\f`, record separators, multilingual dictionary columns).
  These bytes are golden — see *Behavior Oracle*.

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
| Filters | Reorder, deduplicate, convert, tag-gate, or annotate candidates after translation; includes the TypeDuck `DictionaryLookupFilter` comment panel. | `crates/yune-core/src/filter/mod.rs` |
| RIME ABI facade | Thin module-declaration + re-export facade for C ABI entrypoints, key routing, and cross-module glue. | `crates/yune-rime-api/src/lib.rs` |
| ABI layout | Defines librime-shaped structs (including fork-only slots), type aliases, and function table structs. | `crates/yune-rime-api/src/abi.rs` |
| API table | Builds static `RimeApi` and `RimeLeversApi` function tables. | `crates/yune-rime-api/src/api_table.rs` |
| TypeDuck-Web adapter | Exports the simplified `yune_typeduck_*` C API that the WASM/browser consumer drives. | `crates/yune-rime-api/src/typeduck_web.rs` |
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
| CLI harness | Runs deterministic sample sequences against core, AND drives the real exported ABI as a frontend surrogate. | `crates/yune-cli/src/` |

## Pattern Overview

**Overall:** Layered Rust workspace with a deterministic core and a single
librime-shaped compatibility ABI fronted by three concrete consumers.

**Key Characteristics:**
- Keep externally observable RIME compatibility at the boundary in `crates/yune-rime-api/src/`.
- Keep reusable engine behavior in `crates/yune-core/src/` behind Rust traits and typed state.
- Keep `lib.rs` and `main.rs` as facades and orchestration glue; put owned behavior in focused modules.
- Convert deployed YAML configuration into installed session processors, translators, filters, and segment tags.
- Preserve classic input behavior when optional ranking or schema behavior is absent.
- Keep all FFI memory work (CStrings, pointers, `RimeFree*`) at the ABI boundary — including in the `yune_typeduck_*` adapter.

## Layers

**Workspace Layer:**
- Purpose: Define crate composition and shared Rust metadata.
- Location: `Cargo.toml`
- Contains: workspace members, resolver, edition, license, MSRV, lint policy.
- Depends on: Not applicable.
- Used by: all crates under `crates/`.

**Core Engine Layer:**
- Purpose: Represent input-method state and deterministic candidate generation.
- Location: `crates/yune-core/src/`
- Contains: `Engine`, `Context`, `Status`, key parsing, translators, filters, dictionary parsing, table encoding, punctuation, spelling algebra, the AI `CandidateRanker` trait.
- Depends on: `regex` for parsing and pattern application.
- Used by: `crates/yune-cli` (sample runner), `crates/yune-rime-api`.

**RIME Compatibility Layer:**
- Purpose: Present a librime-shaped C ABI and translate frontend calls into core engine mutations; also expose the simplified TypeDuck-Web adapter.
- Location: `crates/yune-rime-api/src/`
- Contains: ABI structs (with fork-only slots), process-wide runtime state, session registry, config/deployment APIs, function table builders, schema installation, processor routing, FFI memory cleanup, `yune_typeduck_*` adapter.
- Depends on: `yune-core`, `libc`, `regex`, `serde_yaml`, `serde_json`.
- Builds: `rlib` (in-process/tests/CLI) and `cdylib` (native `rime.dll`, WASM module).
- Used by: integration tests, `yune-cli`, and any native or WASM frontend loading the exported symbols.

**Schema Model Layer:**
- Purpose: Parse a minimal standalone RIME schema subset into typed Rust values.
- Location: `crates/yune-schema/src/lib.rs`
- Contains: `Schema`, `EngineSpec`, YAML parsing, missing-field errors.
- Depends on: `serde`, `serde_yaml`.
- Used by: schema compatibility work that needs a small typed schema model.

**CLI Layer:**
- Purpose: Provide deterministic fixture generation/checking AND a frontend surrogate that exercises the real ABI.
- Location: `crates/yune-cli/src/`
- Contains: argument parsing, sample core runner (`sample_core.rs`), fixture comparison (`fixture.rs`), the ABI-driving frontend surrogate (`rime_frontend.rs`), JSON/human transcript rendering.
- Depends on: `yune-core` (sample runner) and `yune-rime-api` (frontend surrogate, via the rlib).
- Used by: fixture workflows, smoke checks, and frontend-lifecycle validation.

**TypeScript Runtime / Web Layer:**
- Purpose: Wrap the WASM module so the browser app can drive Yune.
- Location: `packages/yune-typeduck-runtime/src/` (the `@yune-ime/typeduck-runtime` package); `third_party/typeduck-web/yune-integration/adapter.ts` (upstream integration seam); `scripts/typeduck-wasm-build.sh` + `scripts/typeduck-exports.txt` (build).
- Contains: module loader, response decoding, key mapping, in-memory filesystem helpers, exported-symbol verification.
- Depends on: the `yune-rime-api` cdylib compiled to `wasm32-unknown-emscripten`.
- Used by: TypeDuck-Web (M9 validation).

**Documentation and Fixture Layer:**
- Purpose: Describe compatibility scope/direction and store checked-in expected outputs and consumer traces.
- Location: `README.md`, `docs/` (planning docs under `docs/plans/`, with an `docs/plans/archive/` subfolder), `fixtures/`, `crates/*/tests/fixtures/`.
- Contains: roadmap (`docs/roadmap.md`), analysis (`docs/analysis.md`), refactor plan (`docs/plans/refactor-plan.md`), web/Windows plans, sample JSON fixtures, the `fixtures/frontend-traces/` consumer traces, and the `crates/yune-core/tests/fixtures/typeduck-v1.1.2/` oracle fixture.
- Depends on: source behavior but is not compiled.
- Used by: planning, compatibility context, and the CLI/frontend-host/parity tests.

## Data Flow

### Primary RIME API Key Path

1. Frontend obtains the table via `rime_get_api` and calls `RimeApi.process_key` (`crates/yune-rime-api/src/api_table.rs`, `RimeProcessKey`).
2. `RimeProcessKey` validates the session, mask, and keycode, then looks up mutable `SessionState` (`crates/yune-rime-api/src/lib.rs`, `crates/yune-rime-api/src/session.rs`).
3. Keycodes are converted into `yune_core::KeyEvent` values (`crates/yune-rime-api/src/lib.rs`, `crates/yune-core/src/key.rs`).
4. ABI-level processors run before the core engine: ascii composer, key binder, selector, navigator, chord composer, recognizer, punctuation, alternative selection, speller, editor, and shape processing (`crates/yune-rime-api/src/processors/`, dispatched from `lib.rs`).
5. Unhandled typed keys fall through to `Engine::process_key_event` (`crates/yune-core/src/engine.rs`).
6. The core engine refreshes candidates by invoking translators, sorting by quality, applying filters, and allowing rankers to provide ready reorders (`crates/yune-core/src/engine.rs`).
7. Commits are buffered in `SessionState.unread_commit` for `RimeGetCommit`, while context/status reads copy snapshots into caller-owned C structs (`crates/yune-rime-api/src/context_api.rs`).

> Specific `lib.rs` line anchors for this flow were removed because the file has
> grown well past 1900 lines and the old numbers no longer resolve; navigate by
> the function names above (`RimeProcessKey`, processor dispatch, `RimeGetCommit`).

### TypeDuck-Web Adapter Path

1. The browser runtime calls `yune_typeduck_init(shared, user, schema)`, which internally drives the same `rime_get_api` lifecycle (setup/initialize/create-session/select-schema) and returns an opaque `YuneTypeDuckState*` (`crates/yune-rime-api/src/typeduck_web.rs`).
2. `yune_typeduck_process_key` / `select_candidate` / `delete_candidate` / `flip_page` mutate the session and return a `YuneTypeDuckResponse*` whose `json` field carries the serialized context/status/commits.
3. The TS runtime reads `yune_typeduck_response_json` / `response_handled`, then calls `yune_typeduck_free_response` and ultimately `yune_typeduck_cleanup`. All CString/pointer ownership stays inside `typeduck_web.rs`.

### Schema Selection and Installation Flow

1. Frontend selects a schema with `RimeSelectSchema` (`crates/yune-rime-api/src/schema_selection.rs`).
2. `apply_schema_to_session` resets core translators, filters, processors, paging, composition, buffered input, and unread commits (`crates/yune-rime-api/src/schema_selection.rs`).
3. Runtime config roots load deployed YAML, preferring staging over prebuilt data (`crates/yune-rime-api/src/runtime.rs` path resolution, consumed by config loading in `lib.rs`/`config.rs`).
4. Schema installers add segment tags, processors, translators, and filters in fixed order via the `install_schema_*` helpers (`crates/yune-rime-api/src/schema_install.rs`, invoked from `schema_selection.rs`).
5. Translator and filter chain installers map `engine/translators` and `engine/filters` component prescriptions to core implementations (`crates/yune-rime-api/src/schema_install.rs`).

### CLI Flows

1. **Sample/fixture flow.** `Command::parse` selects `Run`/`Check`/`Help` (`crates/yune-cli/src/args.rs`, dispatched from `main.rs::run`). `run_sequence` builds a sample `Engine`, installs punctuation and table translators, processes the key sequence, and returns a fixture output (`crates/yune-cli/src/sample_core.rs`). The transcript serializer renders deterministic JSON (`crates/yune-cli/src/transcript.rs`); `check_fixture` reruns the sample and compares (`crates/yune-cli/src/fixture.rs`).
2. **Frontend-surrogate flow.** `Command::Frontend` / `Command::FrontendCheck` run the real ABI lifecycle through `rime_frontend::run_frontend` and compare against frontend fixtures via `check_frontend_fixture` (`crates/yune-cli/src/rime_frontend.rs`, `crates/yune-cli/src/fixture.rs`).

**State Management:**
- `yune-core` keeps session-local mutable state inside each `Engine`.
- `yune-rime-api` keeps process-wide mutable runtime state in `OnceLock<Mutex<_>>` registries and an `AtomicBool` service flag.
- FFI functions copy Rust-owned state into caller-owned C structures and pair allocations with explicit `RimeFree*` functions.
- Config data is YAML-backed `serde_yaml::Value` stored behind `RimeConfig.ptr`.

## Key Abstractions

**Engine:**
- Purpose: Own one input-method state machine.
- Examples: `crates/yune-core/src/engine.rs`, `crates/yune-core/src/state.rs`
- Pattern: mutable session object with plug-in translators, filters, and rankers.

**Translator:**
- Purpose: Convert current composition input into candidate vectors.
- Examples: `crates/yune-core/src/lib.rs`, `crates/yune-core/src/translator/mod.rs`, `crates/yune-core/src/punctuation.rs`
- Pattern: `Send + Sync` trait object installed into `Engine.translators`.

**CandidateFilter:**
- Purpose: Mutate candidate vectors after translation.
- Examples: `crates/yune-core/src/lib.rs`, `crates/yune-core/src/filter/mod.rs`
- Pattern: `Send + Sync` trait object with option-aware and context-aware hooks; `DictionaryLookupFilter` is the TypeDuck comment-panel implementation.

**CandidateRanker (AI seam):**
- Purpose: Allow optional non-blocking candidate reranking without changing fallback order; reserved for the separate future AI-native layer.
- Examples: `crates/yune-core/src/lib.rs` (`CandidateRanker`, `RerankResult`, `MockAiRanker`)
- Pattern: `RerankResult::Pending` preserves classic order; `RerankResult::Ready` replaces candidate order. `MockAiRanker` is a placeholder; no real provider exists.

**SessionState:**
- Purpose: Bridge one RIME session to one `Engine` plus schema-loaded processor state.
- Examples: `crates/yune-rime-api/src/session.rs`, `crates/yune-rime-api/src/processors/`
- Pattern: registered by numeric `RimeSessionId` in a mutex-protected process-wide registry.

**Runtime Config:**
- Purpose: Resolve deployed/user YAML and expose librime-style path and config APIs.
- Examples: `crates/yune-rime-api/src/runtime.rs`, `crates/yune-rime-api/src/config.rs`, `crates/yune-rime-api/src/config_api.rs`
- Pattern: process-wide path state plus per-config heap state owned through `RimeConfig.ptr`.

**Schema Component Prescription:**
- Purpose: Split `component@namespace` declarations and install matching processors, translators, filters, and segmentors.
- Examples: `crates/yune-rime-api/src/schema_install.rs`
- Pattern: string component registry implemented with match statements and config helpers.

## Entry Points

**Rust Core API:**
- Location: `crates/yune-core/src/lib.rs`
- Triggers: Rust crates instantiate `Engine` or use public parser/type exports.
- Responsibilities: stable API surface for core engine composition, dictionary parsing, translation/filter/ranking, and key parsing.

**RIME C ABI:**
- Location: `crates/yune-rime-api/src/lib.rs`, `crates/yune-rime-api/src/api_table.rs`
- Triggers: C ABI symbol lookup, `rime_get_api`, direct exported `Rime*` calls.
- Responsibilities: maintain librime-compatible function table (with fork-only `config_list_append_*` slots), session lifecycle, key processing, context/status/commit reads, config APIs, deployment helpers.

**TypeDuck-Web C API:**
- Location: `crates/yune-rime-api/src/typeduck_web.rs` (symbols listed in `scripts/typeduck-exports.txt`)
- Triggers: WASM module calls into `yune_typeduck_*` from the browser runtime.
- Responsibilities: simplified init/process/select/delete/flip/deploy/customize/cleanup lifecycle returning JSON responses.

**CLI Binary:**
- Location: `crates/yune-cli/src/main.rs`
- Triggers: `cargo run -p yune-cli -- ...`
- Responsibilities: run sample sequences, check fixtures, drive the real ABI as a frontend surrogate (`Frontend`/`FrontendCheck`), print deterministic JSON/human output or help.

**Schema Parser API:**
- Location: `crates/yune-schema/src/lib.rs`
- Triggers: Rust callers parse schema YAML.
- Responsibilities: parse schema metadata and engine component lists into typed structs.

**Workspace Tests:**
- Location: source-level tests under `crates/yune-core/src/` and `crates/yune-rime-api/src/tests/`; ABI slot locks in `crates/yune-rime-api/src/tests/abi.rs`; frontend-client table use in `crates/yune-rime-api/tests/frontend_client.rs`; **consumer-path validation** in `crates/yune-rime-api/tests/frontend_hosts.rs` (+ `frontend_hosts/{typeduck_web,native_frontends}.rs`) and `crates/yune-rime-api/tests/typeduck_web.rs`; the non-circular oracle parity test `crates/yune-core/tests/cantonese_parity.rs`; CLI fixtures via `crates/yune-cli/src/fixture.rs`.
- Triggers: `cargo test --workspace`.
- Responsibilities: lock core behavior, ABI compatibility and slot positions, the TypeDuck-Web wrapper and modeled native (Squirrel) frontend lifecycles against sanitized traces under `fixtures/frontend-traces/`, oracle parity, and fixture stability.

## Architectural Constraints

- **Threading:** Runtime and sessions use mutex-protected process globals in `crates/yune-rime-api/src/session.rs` and `crates/yune-rime-api/src/runtime.rs`; the core engine itself is ordinary single-session mutable state.
- **Global state:** `sessions()`, `service_started()`, `runtime_paths()`, notification handler state, API function tables, module registries, state-label cache, and config/user dictionary process state are all module-level globals under `crates/yune-rime-api/src/`.
- **ABI field order is the binary contract:** The `RimeApi` (and `RimeLeversApi`) struct field order MUST match the TypeDuck fork's `rime_api.h`. Slot positions are locked by `assert_api_slot!` tests in `crates/yune-rime-api/src/tests/abi.rs`; the fork-only `config_list_append_*` slots are part of this contract.
- **Unsafe boundary:** C ABI functions dereference caller pointers and allocate C strings. Keep unsafe pointer work in ABI/config/context/candidate/FFI-memory modules and in `typeduck_web.rs`, never in `yune-core`.
- **Circular imports:** Rust module cycles are not detected by the compiler and are not present. The RIME ABI facade uses `pub use` / `pub(crate) use` re-exports from `crates/yune-rime-api/src/lib.rs`; avoid module dependencies that force owned logic back into the facade.
- **Compatibility boundary:** External behavior is shaped by RIME/librime contracts in `crates/yune-rime-api/src/`; internal Rust design can stay idiomatic when the boundary remains compatible.
- **Build artifacts:** `yune-rime-api` is `crate-type = ["rlib", "cdylib"]`; do not break either output (native `rime.dll` and the `wasm32-unknown-emscripten` module both depend on the cdylib, and `scripts/typeduck-exports.txt` must list every WASM-exported symbol).
- **Project skills:** `.codex/skills/` and `.agents/skills/` are not detected in this repository.

## Anti-Patterns

### Adding Owned Behavior To Facades

**What happens:** New engine, processor, config, or ABI behavior is added directly to `crates/yune-core/src/lib.rs`, `crates/yune-rime-api/src/lib.rs`, or `crates/yune-cli/src/main.rs`.
**Why it's wrong:** These files already act as public surfaces and orchestration glue; growing them hides ownership boundaries and makes focused compatibility testing harder. `yune-rime-api/src/lib.rs` is a thin module-declaration + re-export facade.
**Do this instead:** Add core behavior under `crates/yune-core/src/`, ABI behavior under the matching `crates/yune-rime-api/src/*.rs` module, processor behavior under `crates/yune-rime-api/src/processors/`, and CLI behavior under focused `crates/yune-cli/src/*.rs` modules.

### Bypassing Schema Installation

**What happens:** Session-specific translator, filter, option, or processor state is mutated ad hoc from a new API path.
**Why it's wrong:** Schema reset and install order lives in `apply_schema_to_session`; bypassing it creates session state that is not reproducible from deployed config.
**Do this instead:** Extend the `install_schema_*` installers invoked from `crates/yune-rime-api/src/schema_selection.rs` and keep component-specific parsing in `crates/yune-rime-api/src/schema_install.rs` or `crates/yune-rime-api/src/processors/`.

### Leaking ABI Allocation Ownership Into Core

**What happens:** Core types or algorithms start depending on C pointers, `CString`, or caller-owned ABI structures.
**Why it's wrong:** `yune-core` is the deterministic Rust engine layer and is used directly by the CLI sample runner; ABI memory ownership belongs to the RIME compatibility layer.
**Do this instead:** Keep C allocation, pointer validation, and `RimeFree*` pairing in `crates/yune-rime-api/src/context_api.rs`, `crates/yune-rime-api/src/candidate_api.rs`, `crates/yune-rime-api/src/config_api.rs`, `crates/yune-rime-api/src/ffi_memory.rs`, and `crates/yune-rime-api/src/typeduck_web.rs`.

### Hand-Rolling Duplicate Config Lookup

**What happens:** New code parses slash-separated config paths or scalar coercions independently.
**Why it's wrong:** Existing helpers encode librime-style scalar and path behavior; duplicate parsing drifts from compatibility tests.
**Do this instead:** Use `find_config_value`, `config_scalar_string`, `config_scalar_bool`, `config_scalar_int`, `config_scalar_double`, and `set_config_value` from `crates/yune-rime-api/src/config.rs` and `crates/yune-rime-api/src/lib.rs`.

### Reordering or Inserting ABI Table Fields Casually

**What happens:** A new function pointer is inserted in the middle of `RimeApi`, or fields are reordered for tidiness.
**Why it's wrong:** Field order is the binary ABI; native frontends and the TypeDuck fork header depend on exact slot positions.
**Do this instead:** Append where the fork header appends, update the `assert_api_slot!` locks in `crates/yune-rime-api/src/tests/abi.rs`, and confirm against the TypeDuck fork's `rime_api.h`.

## Error Handling

**Strategy:** Public Rust APIs use `Result` for parse failures; C ABI entrypoints return librime-style booleans/nulls and leave detailed behavior to tests.

**Patterns:**
- Return typed errors for core parser failures: `TableDictionaryParseError`, `TableEncoderFormulaError`, `KeySequenceParseError`, `SchemaParseError`.
- Return `FALSE` or null from C ABI functions (and `null`/`FALSE` from `yune_typeduck_*`) when session IDs, pointers, masks, config handles, or string conversions are invalid.
- Use explicit `// SAFETY:` comments around pointer operations in ABI-facing unsafe functions.
- Convert lossy or invalid C strings defensively at the ABI boundary.
- Keep commit failures as `None` in core engine methods such as `Engine::commit_composition`.

## Cross-Cutting Concerns

**Logging:** No structured logging framework is present. Runtime setup stores `log_dir` and `app_name` in `crates/yune-rime-api/src/runtime.rs`; operational messages use notification callbacks in `crates/yune-rime-api/src/notifications.rs`.

**Validation:** Core parsers validate input through typed parse errors; ABI functions validate null pointers, data sizes, session IDs, key masks, and string conversions before mutating state.

**Authentication:** Not applicable. This repository has no user authentication layer.

**Configuration:** RIME configuration is YAML-backed and resolved through runtime paths. Deployed config lookup prefers staging over prebuilt data; user config lookup uses the user data directory.

**Compatibility Testing:** Source-level unit tests live beside core code and ABI test modules. Frontend-style ABI testing lives in `crates/yune-rime-api/tests/frontend_client.rs`; consumer-path lifecycles (TypeDuck-Web wrapper, modeled Squirrel native frontend) are validated in `crates/yune-rime-api/tests/frontend_hosts.rs` and `tests/typeduck_web.rs` against sanitized trace fixtures under `fixtures/frontend-traces/`; non-circular oracle parity is checked in `crates/yune-core/tests/cantonese_parity.rs` against the `crates/yune-core/tests/fixtures/typeduck-v1.1.2/` fixture.

---

*Last reviewed: 2026-06-17 — refreshed for the TypeDuck-Web (M9) and TypeDuck-Windows (M10) work; current direction is web-first.*
