# Temporary Refactor Plan

> **Status:** Finished (refactor complete) · **Closed:** 2026-04-30 · **Type:** conventions reference (still cited by AGENTS.md)

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
- Phase 2 is complete for the current `yune-rime-api` production shape.
  `lib.rs` keeps ABI-facing exports and glue, while session, context/status,
  schema selection/install, processor behavior, deployment, levers, config,
  candidate, FFI memory, runtime, module, notification, ABI layout, and userdb
  helpers live in focused modules.
- Phase 3 is complete for `yune-rime-api` unit tests. The test parent module now
  keeps shared helpers, while compatibility cases live in named child modules.
- Phase 4 is complete as a preparatory split. `yune-cli` still behaves as the
  existing core-backed fixture runner, but its argument parsing, fixture checks,
  sample runner, transcript JSON, rendering, and reserved RIME frontend entry
  point are separated.
- `crates/yune-core/src/lib.rs` is about 5,082 lines, mostly tests.
- `crates/yune-rime-api/src/lib.rs` is about 1,851 lines after the Phase 2
  split.
- `crates/yune-rime-api/src/tests/mod.rs` is about 338 lines after the Phase 3
  split.
- `crates/yune-rime-api/tests/frontend_client.rs` is about 4,069 lines.
- `crates/yune-cli/src/main.rs` is about 41 lines after the Phase 4 preparatory
  split.

The immediate problem is not a broken design. The problem is that compatibility
increments are accumulating inside large files, which makes future librime
comparison, review, and focused development slower than necessary.

## Lessons From This Refactor

The main lesson is to treat structure as part of feature work, not as cleanup
afterward. The compatibility push moved quickly because single-file edits were
cheap at first, but the cost shifted into later review, search, focused testing,
and mechanical extraction. Future work should avoid repeating that pattern.

- Before implementing a new behavior slice, identify the owning module, the
  librime source area being mirrored, and the focused test module that will lock
  the behavior down.
- Keep `lib.rs` and `main.rs` as public facades and orchestration glue. New
  parsing, state, ABI, processor, translator, filter, dictionary, or rendering
  behavior should start in a named module unless it is a short throwaway spike.
- A temporary monolithic edit is acceptable only for discovery. If a second
  related behavior lands in the same area, extract the ownership boundary before
  adding more behavior.
- Split tests along the same ownership boundaries as implementation. A focused
  compatibility test should be easy to find from the module it protects.
- Do not wait for file size alone to force structure. Module boundaries should
  follow concepts from Yune's design and observable librime compatibility areas.
- Keep mechanical extraction behavior-free. If a module split is needed before a
  compatibility increment, do the split first, verify it, then land the behavior.

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

Completed layout:

- `abi.rs` owns exported ABI type aliases, constants, and C struct layouts.
- `config_api.rs` owns config open/load/read/write/update entrypoints plus
  state-label and simulated-key-sequence APIs.
- `config.rs` owns config state, scalar coercion, path lookup/mutation, and
  iterator storage helpers.
- `config_compiler.rs` owns librime-style config include/patch/custom patch and
  build-info freshness helpers.
- `ffi_memory.rs` owns the current FFI free entrypoints and shared C allocation
  cleanup helpers.
- `levers.rs` owns levers custom settings, switcher settings, schema-list
  helpers, and user dictionary manager API surface.
- `candidate_api.rs` owns candidate-list iterator entrypoints.
- `schema_api.rs` owns `RimeGetSchemaList` and schema-list population.
- `session.rs` owns the session registry, session lifecycle, and session
  activity helpers.
- `context_api.rs` owns context/status/commit entrypoints.
- `schema_selection.rs` owns selected-schema entrypoints and session schema
  application.
- `schema_install.rs` owns schema component prescription parsing, translator and
  filter chain installation, segment tags, recognizer patterns, and schema-switch
  resets.
- `processors/` owns `ascii_composer`, `chord_composer`, `editor`,
  `key_binder`, `navigator`, `punctuation`, `recognizer`, `selector`, `shape`,
  and `speller` behavior.
- `deployment.rs`, `runtime.rs`, `modules.rs`, `notifications.rs`,
  `api_table.rs`, `key_table.rs`, and `userdb.rs` own their corresponding
  runtime and ABI support surfaces.
- `lib.rs` intentionally remains the ABI export index and cross-module glue.

Further extraction should be driven by new compatibility work, not by file size
alone.

## Phase 3: Split Tests

Goal: make compatibility intent easier to locate.

Completed layout:

- `crates/yune-rime-api/src/tests/mod.rs`
- `crates/yune-rime-api/src/tests/abi.rs`
- `crates/yune-rime-api/src/tests/candidate_api.rs`
- `crates/yune-rime-api/src/tests/config_api.rs`
- `crates/yune-rime-api/src/tests/context_status.rs`
- `crates/yune-rime-api/src/tests/deployment.rs`
- `crates/yune-rime-api/src/tests/levers.rs`
- `crates/yune-rime-api/src/tests/runtime.rs`
- `crates/yune-rime-api/src/tests/schema_api.rs`
- `crates/yune-rime-api/src/tests/schema_processors.rs`
- `crates/yune-rime-api/src/tests/schema_selection.rs`
- `crates/yune-rime-api/src/tests/session_api.rs`
- `crates/yune-rime-api/src/tests/userdb.rs`

For `frontend_client.rs`, split only after `yune-cli` frontend-surrogate work is
underway, because the transcript/replay design may change what frontend-style
coverage should look like.

## Phase 4: Prepare `yune-cli`

Goal: avoid turning `main.rs` into the next large mixed-responsibility file.

Completed preparatory layout before implementing the interactive frontend:

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
- `sample_core.rs`
  - existing core-backed sample fixture runner retained until the frontend
    surrogate replaces it

The CLI should drive `yune-rime-api`, not `yune-core` directly, once it becomes
the frontend-surrogate input method.

Current status: the preparatory module split is complete. `rime_frontend.rs` is
reserved for the upcoming `yune-rime-api` implementation; the current `run` and
`check` commands intentionally keep their prior behavior.

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
