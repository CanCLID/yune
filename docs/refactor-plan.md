# Temporary Refactor Plan

This document is a temporary engineering guide for reducing maintenance risk
while preserving the current compatibility work. It should be treated as a
working plan, not as a permanent architecture document.

## Current Assessment

The repository is functionally healthy but structurally crowded.

- `cargo test --workspace` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- Phase 0 is complete.
- Phase 1 is complete for production `yune-core` modules. `lib.rs` now keeps the
  public API surface and the existing unit tests, while implementation lives in
  focused modules.
- Phase 2 is in progress. The low-risk RIME API moves have started to reduce
  `lib.rs`, but session, context/status/commit, schema installation, and
  processor dispatch still need to be split.
- `crates/yune-core/src/lib.rs` is about 5,082 lines, mostly tests.
- `crates/yune-rime-api/src/lib.rs` is about 5,956 lines after the current
  Phase 2 API splits.
- `crates/yune-rime-api/src/tests.rs` is about 23,556 lines.
- `crates/yune-rime-api/tests/frontend_client.rs` is about 4,069 lines.
- `crates/yune-cli/src/main.rs` is still small, but it is about to grow when it
  becomes the CLI frontend-surrogate described in `roadmap.md`.

The immediate problem is not a broken design. The problem is that compatibility
increments are accumulating inside large files, which makes future librime
comparison, review, and focused development slower than necessary.

## Refactor Rules

- Preserve externally observable behavior.
- Keep `/Users/trenton/Projects/librime` as the behavior oracle.
- Do not combine mechanical module moves with behavior changes.
- Prefer one ownership boundary per commit.
- Run focused tests after each move, then `cargo test --workspace` after each
  phase.
- Update this document and `roadmap.md` only when a phase is actually completed.
- Do not use refactoring as an excuse to rewrite working compatibility slices.

## Phase 0: Fix Tooling Debt

Goal: make the normal quality gate reliable before moving code.

Work:

- Replace Rust APIs newer than the workspace MSRV `1.76`, especially
  `Option::is_none_or`.
- Fix the current clippy cleanup items in `yune-core`:
  - manual range containment in compiled reverse metadata parsing.
  - derivable `Default` for `TableEncoder`.
  - simple `?` conversion in preset vocabulary loading.
  - nonminimal boolean expression in reverse lookup filter handling.
  - approximate constants for spelling-algebra penalties.
- Re-run:
  - `cargo fmt`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`

This phase should not change behavior.

## Phase 1: Split `yune-core`

Goal: separate core engine concepts without changing public behavior.

Suggested module layout:

- `key.rs`
  - `KeyCode`
  - `KeyEvent`
  - `KeyModifiers`
  - key sequence parsing
  - librime key-name no-op groups
- `state.rs`
  - `Candidate`
  - `CandidateSource`
  - `Composition`
  - `Context`
  - `Status`
  - `Snapshot`
  - `CommitRecord`
- `dictionary/mod.rs`
  - `TableEntry`
  - `TableDictionary`
  - RIME `.dict.yaml` parsing
  - imports, packs, preset vocabulary, stems
- `dictionary/compiled.rs`
  - `RimeChecksumComputer`
  - dictionary source checksum
  - compiled `.table.bin`/`.prism.bin`/`.reverse.bin` metadata parsing
  - rebuild-plan primitive
- `dictionary/encoder.rs`
  - `TableEncoder`
  - `TableEncodingRule`
  - formula parsing
  - phrase encoding
- `translator/mod.rs`
  - `EchoTranslator`
  - `StaticTableTranslator`
  - `ReverseLookupTranslator`
  - `HistoryTranslator`
  - `SwitchTranslator`
  - `SchemaListTranslator`
- `filter/mod.rs`
  - `UniquifierFilter`
  - `SingleCharFilter`
  - `CharsetFilter`
  - `TaggedFilter`
  - `SimplifierFilter`
  - `ReverseLookupFilter`
- `punctuation.rs`
  - `PunctuationTranslator`
  - punctuation shape/comment helpers
- `spelling_algebra.rs`
  - `SpellingAlgebra`
  - formulas and generated-spelling penalties
- `engine.rs`
  - `Engine`
  - candidate rebuild and commit behavior

Recommended order:

1. Move key parsing first. It has clear boundaries and many tests.
2. Move state structs.
3. Move dictionary compiled metadata and checksum helpers.
4. Move dictionary source parsing and encoder together only if the first three
   moves are stable.
5. Move translators and filters after dictionary types are stable.
6. Move `Engine` last.

## Phase 2: Split `yune-rime-api`

Goal: keep ABI exports visible while moving implementation details behind
focused modules.

Current progress:

- `config_api.rs` owns config open/load/read/write/update entrypoints plus
  state-label and simulated-key-sequence APIs.
- `ffi_memory.rs` owns the current FFI free entrypoints and shared C allocation
  cleanup helpers.
- `levers.rs` owns levers custom settings, switcher settings, schema-list
  helpers, and user dictionary manager API surface.
- `candidate_api.rs` owns candidate-list iterator entrypoints.
- `schema_api.rs` owns `RimeGetSchemaList` and schema-list population.
- `lib.rs` still owns session lifecycle, key processing, context/status/commit
  entrypoints, schema selection, schema installation, processor behavior, and
  several cross-cutting session helpers.

Suggested module layout:

- `session.rs`
  - `SessionRegistry`
  - `SessionState`
  - session lifecycle helpers
  - `with_session`
- `runtime.rs`
  - `RuntimePaths`
  - `RimeSetup`
  - runtime path getters
  - installation metadata
- `ffi_memory.rs`
  - C string allocation/freeing helpers
  - context/status/schema-list clearing
  - versioned struct-member checks
- `api_table.rs`
  - `RimeGetApi`
  - function table construction
  - levers module table construction
- `modules.rs`
  - module registry
  - built-in levers module lookup
- `schema_install/mod.rs`
  - schema selection
  - translator/filter/processor/segmentor installation
  - schema component prescription parsing
- `processors/mod.rs`
  - key processing dispatch
  - `ascii_composer`
  - `chord_composer`
  - `editor`
  - `key_binder`
  - `punctuator`
  - `recognizer`
  - `speller`
  - selector/navigator key handling
- `deployment.rs`
  - maintenance
  - workspace update
  - schema deploy
  - build-info freshness
  - cleanup tasks
- `userdb.rs`
  - current plain userdb shim
  - backup/restore/import/export/sync helpers
- `levers.rs`
  - custom settings
  - schema lists
  - user dictionary manager API surface

Remaining recommended order:

1. Split session registry and lifecycle into `session.rs`, including
   `SessionRegistry`, `SessionState`, `sessions`, `with_session`, session
   activity helpers, and create/find/destroy/cleanup entrypoints.
2. Split context/status/commit entrypoints into a focused ABI module after
   `session.rs` owns the session access helpers.
3. Split schema selection and switch-state restore helpers only after the
   session boundary is stable.
4. Split schema installation helpers into `schema_install/` in small groups:
   translator chain, filter chain, segment tags, switch resets, and component
   prescription parsing.
5. Split processor implementations one component at a time: `ascii_composer`,
   `chord_composer`, `editor`, `key_binder`, `punctuator`, `recognizer`,
   `speller`, selector, and navigator.
6. Move deployment/userdb helpers only when their current module boundaries show
   real remaining mixed responsibility. Avoid churn in files that are already
   readable.
7. Keep exported `extern "C"` functions easy to find from `lib.rs` until the
   module layout proves stable.

## Phase 3: Split Tests

Goal: make compatibility intent easier to locate.

Suggested layout:

- `crates/yune-rime-api/src/tests/mod.rs`
- `crates/yune-rime-api/src/tests/abi.rs`
- `crates/yune-rime-api/src/tests/deployment.rs`
- `crates/yune-rime-api/src/tests/levers.rs`
- `crates/yune-rime-api/src/tests/runtime.rs`
- `crates/yune-rime-api/src/tests/schema_processors.rs`
- `crates/yune-rime-api/src/tests/schema_segmentors.rs`
- `crates/yune-rime-api/src/tests/schema_translators.rs`
- `crates/yune-rime-api/src/tests/schema_filters.rs`
- `crates/yune-rime-api/src/tests/userdb.rs`

For `frontend_client.rs`, split only after `yune-cli` frontend-surrogate work is
underway, because the transcript/replay design may change what frontend-style
coverage should look like.

## Phase 4: Prepare `yune-cli`

Goal: avoid turning `main.rs` into the next large mixed-responsibility file.

Suggested module layout before implementing the interactive frontend:

- `args.rs`
  - command parsing
  - shared/user data dir arguments
  - schema selection arguments
- `rime_frontend.rs`
  - `yune-rime-api` setup
  - session creation/destruction
  - key processing
  - context/status/commit reads
- `transcript.rs`
  - deterministic replay output
  - JSON serialization for regression checks
- `render.rs`
  - human-readable interactive CLI output
- `fixture.rs`
  - existing fixture check compatibility, if retained

The CLI should drive `yune-rime-api`, not `yune-core` directly, once it becomes
the frontend-surrogate input method.

## What Not To Refactor Yet

- Do not introduce a new plugin system while plugin compatibility is deferred.
- Do not replace `serde_yaml::Value` access throughout schema loading until the
  current focused schema behavior is better stabilized.
- Do not convert all global runtime state at once. The ABI compatibility layer
  currently mirrors librime's process-wide service shape; isolate it before
  changing ownership.
- Do not redesign dictionary lookup while compiled-data compatibility is still
  incomplete.

## Completion Criteria

The refactor is successful only if:

- `cargo fmt` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- `cargo test --workspace` passes.
- The public CLI and RIME ABI behavior is unchanged unless a commit explicitly
  says it is a behavior change.
- Docs still identify `yune-cli` as a frontend-surrogate, not as native frontend
  validation.
