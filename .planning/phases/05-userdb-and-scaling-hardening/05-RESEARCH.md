# Phase 05: UserDB And Scaling Hardening - Research

**Researched:** 2026-04-30 [VERIFIED: currentDate]
**Domain:** Rust input-method user dictionary storage, librime-observable userdb lifecycle, and scaling/test hardening [VERIFIED: .planning/ROADMAP.md]
**Confidence:** HIGH for Yune seams and librime comparison targets; MEDIUM for exact storage implementation choice because Phase 05 explicitly permits either direct LevelDB compatibility or a documented compatible abstraction. [VERIFIED: prompt locked_decisions; VERIFIED: /Users/trenton/Projects/librime source]

## User Constraints

### Locked Decisions (from `.planning/phases/05-userdb-and-scaling-hardening/05-CONTEXT.md` and research prompt)

- Use a focused Rust userdb abstraction that models librime-observable LevelDB/userdb behavior while keeping yune-core independent of C ABI pointers/storage engine details.
- Prefer behavior compatibility over cloning LevelDB internals; a documented compatible abstraction is acceptable if direct LevelDB compatibility is too large.
- Existing levers/userdb C ABI functions remain the external boundary; preserve runtime path/resource-ID validation before filesystem access.
- User dictionary names remain logical resource IDs, not arbitrary paths.
- Backup, restore, import, export, sync, recovery, rollback, and upgrade are deterministic local transaction boundaries with failure tests.
- Recovery and rollback fail closed: interrupted/malformed state preserves last valid dictionary or explicitly fails.
- Sync must be conflict-aware enough for librime-observable semantics; plain line append/dedupe is insufficient.
- Learning is commit-driven through normal runtime/session flows.
- Frequency and predictive lookup affect runtime candidate quality/order through classic deterministic behavior, not AI rankers/memory.
- HistoryTranslator and CandidateRanker are context, not substitutes for userdb learning.
- Split oversized tests only by behavior ownership; do not mix mechanical moves and semantic behavior changes.
- Plans must name owning implementation module, owning test module, and librime comparison target.

### Claude's Discretion

The exact storage backend shape, fixture byte/text formats, module split names, transaction log format, and selected librime comparison schemas are left to research and planning, provided Phase 5 context decisions remain true. [VERIFIED: .planning/phases/05-userdb-and-scaling-hardening/05-CONTEXT.md]

### Deferred Ideas (OUT OF SCOPE)

AI-native memory, source-labeled AI candidates, local/remote LLM providers, privacy policy UI, context provider design, full librime C++ plugin ABI compatibility, Lua/octagram/predict/proto plugin ecosystems, and a real graphical frontend remain outside Phase 5 unless a focused userdb ABI fixture requires frontend-style coverage. [VERIFIED: .planning/phases/05-userdb-and-scaling-hardening/05-CONTEXT.md]

## Executive Summary

Phase 05 should replace the current plain-file userdb shim with a focused Rust userdb layer that preserves existing levers/userdb ABI entrypoints while modeling librime's observable userdb records, metadata, snapshots, conflict-aware sync, recovery, transactions, and commit-driven learning. [VERIFIED: crates/yune-rime-api/src/userdb.rs; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc; VERIFIED: /Users/trenton/Projects/librime/src/rime/lever/user_dict_manager.cc]

The immediate planning risk is treating `*.userdb` text lines as the compatibility contract. Current Yune sync reads complete files, dedupes lines with a `HashSet`, and overwrites the destination, while librime snapshots encode metadata and key/value records whose values contain `commits`, `dee`, and `tick`; librime sync merges by decayed experience, maximum absolute commit count, and maximum tick. [VERIFIED: crates/yune-rime-api/src/userdb.rs:336-365; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc:17-50,182-239]

Plan Phase 05 as three separated waves: first storage/lifecycle and conflict-aware sync, then runtime learning/ranking/predictive lookup, then mechanical test splits and quality gates. Do not mix oversized-test moves with semantic userdb behavior changes. [VERIFIED: .planning/ROADMAP.md; VERIFIED: docs/refactor-plan.md]

**Primary recommendation:** Implement `yune-rime-api` userdb lifecycle/storage modules plus a storage-agnostic `yune-core` userdb translator/learning contract; preserve C ABI boundaries and logical resource-ID validation, and verify against librime source targets before optimizing storage internals. [VERIFIED: prompt locked_decisions; VERIFIED: crates/yune-rime-api/src/resource_id.rs; VERIFIED: crates/yune-core/src/engine.rs]

## Project Constraints (from AGENTS.md)

- `/Users/trenton/Projects/librime` is the external behavior oracle for user-visible behavior, schema semantics, ABI contracts, and migration support. [VERIFIED: AGENTS.md]
- Prefer typed, idiomatic Rust modules over cloning librime's internal C++ structure when the boundary contract is preserved. [VERIFIED: AGENTS.md]
- Run focused tests for each behavior slice and `cargo test --workspace` after broader phases; use `cargo clippy --workspace --all-targets -- -D warnings` as the quality gate when implementation changes warrant it. [VERIFIED: AGENTS.md]
- The CLI frontend is an intermediate validation layer and is not proof that native frontends such as Squirrel, Weasel, ibus-rime, fcitx-rime, or fcitx5-rime work. [VERIFIED: AGENTS.md]
- Source `.dict.yaml` support is not enough for production-scale compatibility; compiled `.table.bin`, `.prism.bin`, and `.reverse.bin` payloads remain a required direction. [VERIFIED: AGENTS.md]
- Runtime resource identifiers must be treated as logical IDs, not arbitrary filesystem paths. [VERIFIED: AGENTS.md]
- Use Rust 2021 syntax with workspace MSRV 1.76; avoid newer standard-library helpers unless the MSRV is raised. [VERIFIED: AGENTS.md; VERIFIED: Cargo.toml]
- Keep `lib.rs` and `main.rs` as facades/orchestration glue; add new implementation work to focused modules. [VERIFIED: AGENTS.md; VERIFIED: docs/refactor-plan.md]
- FFI boundary functions should use explicit `unsafe extern "C" fn` signatures, Rustdoc `# Safety` sections, and local `// SAFETY:` comments. [VERIFIED: AGENTS.md; VERIFIED: crates/yune-rime-api/src/userdb.rs]

<phase_requirements>

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| USERDB-01 | User dictionary storage supports librime-compatible LevelDB/userdb behavior or a documented compatible abstraction beyond the current plain text shim. [VERIFIED: .planning/REQUIREMENTS.md] | Use a `UserDbStore` abstraction with librime-shaped metadata, keys, values, prefix query, ordered iteration, snapshots, and transactions; direct LevelDB bytes are not required unless chosen later. [VERIFIED: prompt locked_decisions; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/db.h; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/level_db.cc] |
| USERDB-02 | Snapshot backup, restore, recovery, sync, and transaction rollback behavior match librime-observable semantics. [VERIFIED: .planning/REQUIREMENTS.md] | Compare against `UserDictManager::{Backup,Restore,Import,Export,Synchronize}`, `LevelDb::{Backup,Restore,Recover,BeginTransaction,AbortTransaction,CommitTransaction}`, and `UserDictionary::{NewTransaction,RevertRecentTransaction,CommitPendingTransaction}`. [VERIFIED: /Users/trenton/Projects/librime/src/rime/lever/user_dict_manager.cc; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/level_db.cc; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_dictionary.cc] |
| USERDB-03 | Learning, frequency updates, predictive lookup, and backdated scan behavior are represented in runtime candidate ranking and userdb persistence. [VERIFIED: .planning/REQUIREMENTS.md] | Hook learning into normal commit paths, model `UserDbValue { commits, dee, tick }`, implement `formula_d`/`formula_p`, add userdb candidates with deterministic quality, and cover predictive comments/resume/backdating where current core abstractions permit. [VERIFIED: crates/yune-core/src/engine.rs; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_dictionary.cc; VERIFIED: /Users/trenton/Projects/librime/src/rime/algo/dynamics.h; VERIFIED: /Users/trenton/Projects/librime/src/rime/gear/table_translator.cc] |
| QUAL-03 | Remaining oversized compatibility tests are split only along behavior ownership boundaries, without mixing mechanical moves and behavior changes. [VERIFIED: .planning/REQUIREMENTS.md] | Keep test split work in 05-03; do not move large tests during 05-01 or 05-02 semantic changes. [VERIFIED: docs/refactor-plan.md; VERIFIED: .planning/codebase/CONCERNS.md] |
| QUAL-04 | Quality gates for implementation phases include focused tests, `cargo fmt`, relevant `cargo test` targets, and workspace tests when shared behavior changes. [VERIFIED: .planning/REQUIREMENTS.md] | Use focused package/module tests per slice, then `cargo fmt`, relevant `cargo test -p ...`, `cargo test --workspace`, and clippy when implementation changes warrant it. [VERIFIED: .planning/codebase/TESTING.md; VERIFIED: AGENTS.md] |

</phase_requirements>

## Phase Scope and Non-Goals

### In Scope

- Replace or wrap the current plain text userdb shim in `crates/yune-rime-api/src/userdb.rs` with a userdb abstraction that stores metadata and key/value records using librime-observable semantics. [VERIFIED: crates/yune-rime-api/src/userdb.rs; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc]
- Preserve the existing levers/userdb C ABI functions: iterator init/destroy/next, backup, restore, export, and import. [VERIFIED: crates/yune-rime-api/src/userdb.rs; VERIFIED: crates/yune-rime-api/src/abi.rs]
- Preserve runtime path resolution through `RimeTraits`, `installation.yaml`, `sync_dir`, `user_id`, and `user_data_sync_dir`. [VERIFIED: crates/yune-rime-api/src/runtime.rs; VERIFIED: crates/yune-rime-api/src/deployment.rs]
- Add failure tests for malformed snapshots, interrupted writes, rollback, invalid logical IDs, and corrupt data. [VERIFIED: prompt locked_decisions; VERIFIED: crates/yune-rime-api/src/tests/resource_id.rs]
- Add runtime learning and userdb candidate ranking through classic deterministic engine flows, not through `CandidateRanker` or AI sources. [VERIFIED: prompt locked_decisions; VERIFIED: crates/yune-core/src/engine.rs; VERIFIED: crates/yune-core/src/state.rs]
- Split oversized tests only in the dedicated quality slice after behavior coverage is stable. [VERIFIED: .planning/ROADMAP.md; VERIFIED: docs/refactor-plan.md]

### Non-Goals

- Do not implement librime's C++ plugin ABI in Phase 05. [VERIFIED: .planning/PROJECT.md; VERIFIED: docs/analysis.md]
- Do not make cloud/AI ranking part of userdb learning or candidate ordering. [VERIFIED: prompt locked_decisions; VERIFIED: .planning/PROJECT.md]
- Do not require byte-for-byte LevelDB file compatibility unless the selected storage backend explicitly targets it; the locked decision permits a documented compatible abstraction. [VERIFIED: prompt locked_decisions]
- Do not redesign compiled `.table.bin`, `.prism.bin`, or `.reverse.bin` loading in Phase 05; Phase 04 owns compiled dictionary data. [VERIFIED: .planning/ROADMAP.md]
- Do not use refactoring as an excuse to rewrite working compatibility slices. [VERIFIED: docs/refactor-plan.md]

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|--------------|----------------|-----------|
| Userdb levers ABI calls | API / Backend (`yune-rime-api`) | Filesystem storage | C callers enter through `RimeLevers*UserDict*` functions and receive librime-shaped Bool/int/pointer results. [VERIFIED: crates/yune-rime-api/src/userdb.rs; VERIFIED: crates/yune-rime-api/src/abi.rs] |
| Runtime path and sync directory resolution | API / Backend (`yune-rime-api`) | Filesystem storage | Runtime paths are process-wide `RimeTraits`/`installation.yaml` state owned by `runtime.rs` and updated by deployment tasks. [VERIFIED: crates/yune-rime-api/src/runtime.rs; VERIFIED: crates/yune-rime-api/src/deployment.rs] |
| Userdb record store and transactions | Database / Storage | API / Backend (`yune-rime-api`) | Storage must own ordered key/value records, metadata, snapshots, recovery, atomic writes, and rollback semantics. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/db.h; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/level_db.cc] |
| Candidate learning event capture | API / Backend (`yune-rime-api`) | Core engine (`yune-core`) | ABI/session commit paths observe frontend-visible commits and can call storage-independent core learning hooks without exposing C pointers to core. [VERIFIED: crates/yune-rime-api/src/lib.rs; VERIFIED: crates/yune-rime-api/src/session.rs; VERIFIED: prompt locked_decisions] |
| Candidate generation/ranking from learned userdb entries | Core engine (`yune-core`) | Storage abstraction | `Engine::refresh_candidates` owns translator/filter/ranker order; userdb candidates should enter as classic translator output before optional AI rankers. [VERIFIED: crates/yune-core/src/engine.rs; VERIFIED: prompt locked_decisions] |
| Oversized test split | Test suite | Implementation modules | Test ownership follows module ownership and must not mix moves with behavior changes. [VERIFIED: docs/refactor-plan.md; VERIFIED: .planning/codebase/CONCERNS.md] |

## Current Yune UserDB/Runtime Seams

### Current UserDB ABI and Storage Surface

- `crates/yune-rime-api/src/userdb.rs` owns the current exported levers user dictionary functions and local helper functions. [VERIFIED: crates/yune-rime-api/src/userdb.rs]
- Current user dictionary files are discovered by scanning `user_data_dir` for filenames ending in `.userdb`. [VERIFIED: crates/yune-rime-api/src/userdb.rs:224-243]
- Current backup writes `dict_name.userdb` to `runtime_user_data_sync_dir()/dict_name.userdb.txt` by plain file copy. [VERIFIED: crates/yune-rime-api/src/userdb.rs:264-285]
- Current restore validates a snapshot filename suffix and copies that file directly to `user_data_dir/dict_name.userdb`. [VERIFIED: crates/yune-rime-api/src/userdb.rs:116-138]
- Current import/export copy whole text files and count non-empty, non-comment lines as entries. [VERIFIED: crates/yune-rime-api/src/userdb.rs:147-222,377-388]
- Current sync merges peer snapshots by reading the entire destination and snapshot into strings, building a `HashSet` of destination lines, appending unseen non-empty snapshot lines, and writing the merged string. [VERIFIED: crates/yune-rime-api/src/userdb.rs:336-365]
- Current `user_dict_upgrade()` is a no-op returning `true`. [VERIFIED: crates/yune-rime-api/src/userdb.rs:297-299]

### Runtime and Deployment Seams

- `RuntimePaths` stores `shared_data_dir`, `user_data_dir`, `prebuilt_data_dir`, `staging_dir`, `sync_dir`, `user_id`, and `user_data_sync_dir` as process-wide C strings. [VERIFIED: crates/yune-rime-api/src/runtime.rs:16-30]
- `RuntimePaths::new` computes `user_data_sync_dir` as `path_join(sync_dir, user_id)`. [VERIFIED: crates/yune-rime-api/src/runtime.rs:70-88]
- `RimeSetup` reads `installation.yaml` and falls back to `user_data_dir/sync` when installation metadata is loaded without an explicit sync dir. [VERIFIED: crates/yune-rime-api/src/runtime.rs:90-159]
- Deployment's installation update writes `installation_id`, sync directory, distribution metadata, and `rime_version`, then updates runtime `user_id`, `sync_dir`, and `user_data_sync_dir`. [VERIFIED: crates/yune-rime-api/src/deployment.rs:200-278]
- `RimeSyncUserData` cleans sessions, notifies deploy start, runs installation update, backs up config files, syncs all user dicts, and notifies success/failure. [VERIFIED: crates/yune-rime-api/src/deployment.rs:118-127]
- `RimeRunTask("user_dict_sync")` delegates to `sync_all_user_dicts()`. [VERIFIED: crates/yune-rime-api/src/deployment.rs:130-136]

### Resource-ID Safety Seam

- `validate_user_dict_name` rejects names ending in `.userdb` or `.userdb.txt`, then delegates to logical ID validation. [VERIFIED: crates/yune-rime-api/src/resource_id.rs:26-31]
- Logical ID validation rejects empty strings, `.`, `..`, `~` prefixes, NUL, `/`, `\`, and Windows drive prefixes. [VERIFIED: crates/yune-rime-api/src/resource_id.rs:33-52]
- Existing tests cover accepted logical user dict names, rejected path/suffix names, and ABI import/export/backup rejection for unsafe userdb names. [VERIFIED: crates/yune-rime-api/src/tests/resource_id.rs:103-140,241-297]

### Core Runtime Candidate Seam

- `Engine` stores translators, filters, and optional rankers, and `refresh_candidates` collects translator output, sorts by descending `quality`, applies filters, then applies optional `CandidateRanker` results. [VERIFIED: crates/yune-core/src/engine.rs:10-18,711-735]
- `CandidateSource` currently includes `Table`, `Completion`, `Sentence`, `History`, and `Ai`, but does not include a distinct `UserTable` source. [VERIFIED: crates/yune-core/src/state.rs:8-21]
- `record_commit_with_type` records `last_commit` and appends `CommitRecord { candidate_type, text }` to unbounded `commit_history`. [VERIFIED: crates/yune-core/src/engine.rs:703-709; VERIFIED: .planning/codebase/CONCERNS.md]
- `HistoryTranslator` reads recent `commit_history`, but the locked decision says it is not a substitute for persistent userdb learning. [VERIFIED: crates/yune-core/src/translator/mod.rs; VERIFIED: prompt locked_decisions]
- `StaticTableTranslator` computes candidate quality as `candidate.quality.exp() + initial_quality`, and completion candidates get a `-1.0` quality adjustment. [VERIFIED: crates/yune-core/src/translator/mod.rs:205-218]

## Librime Observable Comparison Targets

| Behavior | Librime files/functions to compare | Observable contract to model |
|----------|------------------------------------|------------------------------|
| Userdb record value | `UserDbValue::{Pack,Unpack}` in `src/rime/dict/user_db.cc` | Values pack as `c=<commits> d=<dee> t=<tick>`, parse fields independently, cap `dee` at 10000, and fail parse on invalid numeric fields. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc:17-50] |
| Userdb key format | `userdb_entry_parser` and `CreateDictEntry` | Userdb keys are `code + "\t" + phrase`, where code carries a trailing space before tab for syllable-separated code. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc:64-80; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_dictionary.cc:421-456] |
| Metadata | `Db::CreateMetadata`, `LevelDb::MetaFetch/MetaUpdate`, `UserDbHelper` | Metadata includes db name/type, tick, user id, and rime version; LevelDB stores metadata keys with `"\x01"` prefix. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/level_db.cc:16,290-300; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc:142-179] |
| Backup snapshot | `UserDbHelper::UniformBackup`, `LevelDb::Backup`, `UserDictManager::Backup` | Backup opens db read-only, ensures user id metadata matches, creates per-user sync dir, and writes uniform `*.userdb.txt` snapshots. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc:114-126; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/level_db.cc:192-203; VERIFIED: /Users/trenton/Projects/librime/src/rime/lever/user_dict_manager.cc:50-70] |
| Restore snapshot | `UserDictManager::Restore` | Restore loads snapshot into a temporary `.temp` db, verifies it is a userdb, reads db name from metadata, opens destination, and merges through `UserDbMerger`. [VERIFIED: /Users/trenton/Projects/librime/src/rime/lever/user_dict_manager.cc:72-104] |
| Conflict-aware sync | `UserDictManager::Synchronize`, `UserDbMerger::Put` | Sync scans peer sync dirs for `dict_name.userdb.txt`, restores each snapshot, and backs up current state; merge decays old `dee`, uses max absolute commits, max dee, and max tick. [VERIFIED: /Users/trenton/Projects/librime/src/rime/lever/user_dict_manager.cc:177-207; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc:182-239] |
| Import/export | `TableDb::format`, `UserDictManager::{Import,Export}`, `UserDbImporter::Put` | Import/export use table rows `phrase<Tab>code[<Tab>commits]`; exports skip deleted entries; import maps positive commits to max commits/dee and negative commits to deletion. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/table_db.cc:17-60; VERIFIED: /Users/trenton/Projects/librime/src/rime/lever/user_dict_manager.cc:106-151; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc:241-264] |
| Transactions and rollback | `LevelDb::{BeginTransaction,AbortTransaction,CommitTransaction}`, `UserDictionary::{NewTransaction,RevertRecentTransaction,CommitPendingTransaction}` | Transactions buffer writes, abort clears the batch, commit writes batch; recent rollback only aborts an active transaction less than 3 seconds old. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/level_db.cc:302-325; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_dictionary.cc:486-510] |
| Recovery | `LevelDb::Recover`, `UserDbRecoveryTask` | Recovery tries LevelDB repair; if repair fails, the task renames/removes/recreates the DB and restores from snapshot when available. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/level_db.cc:217-226; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db_recovery_task.cc] |
| Learning | `Memory::{OnCommit,OnDeleteEntry,OnUnhandledKey}`, `TableTranslator::Memorize` | Learning starts a transaction on commit, memorizes intelligible committed entries, marks deletions with negative commits, commits or discards pending transaction on unhandled keys/backspace. [VERIFIED: /Users/trenton/Projects/librime/src/rime/gear/memory.cc:98-159; VERIFIED: /Users/trenton/Projects/librime/src/rime/gear/table_translator.cc:311-349] |
| Frequency update math | `UserDictionary::UpdateEntry`, `algo::formula_d` | Positive commits revive deleted entries, increment commits and tick, and update `dee` with `formula_d`; zero and negative updates use different `dee` changes. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_dictionary.cc:421-456; VERIFIED: /Users/trenton/Projects/librime/src/rime/algo/dynamics.h:5-7] |
| Predictive lookup and ranking | `UserDictionary::LookupWords`, `CreateDictEntry`, `TableTranslation::MakePhrase`, `TableTranslation::PreferUserPhrase` | Predictive lookup can return prefix entries with comments like `~remaining_code`, exact-match ranges are sorted, deleted entries are skipped, user phrases receive a `+0.5` quality bonus in table translation. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_dictionary.cc:357-419,532-567; VERIFIED: /Users/trenton/Projects/librime/src/rime/gear/table_translator.cc:90-112] |
| Backdated scan | `DfsState::Backdate`, `UserDictionary::DfsLookup` | Librime reintroduces backdating for normal/fuzzy spellings to handle ambiguous derived spelling paths, while avoiding abbreviation backtracking in some paths. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_dictionary.cc:57-70,208-214,215-303] |

## Standard Stack

### Core

| Library / Component | Version | Purpose | Why Standard |
|---------------------|---------|---------|--------------|
| Rust workspace / Cargo | MSRV 1.76; current shell Cargo unavailable | Build/test/package all crates and enforce Rust ownership at FFI/storage boundaries. [VERIFIED: Cargo.toml; VERIFIED: command -v cargo] | Existing project stack and manifests are Cargo workspace based. [VERIFIED: Cargo.toml] |
| Rust standard library filesystem | Rust std | Local user data, sync snapshot, temp file, rename, and recovery operations. [VERIFIED: crates/yune-rime-api/src/userdb.rs; VERIFIED: crates/yune-rime-api/src/runtime.rs] | Existing runtime storage uses local filesystem only. [VERIFIED: .planning/codebase/INTEGRATIONS.md] |
| `yune-core` internal traits/modules | 0.1.0 | Engine state, translators, filters, candidates, and deterministic ranking order. [VERIFIED: crates/yune-core/src/engine.rs; VERIFIED: crates/yune-core/src/state.rs] | Keeps core independent of C ABI pointers/storage engine details. [VERIFIED: prompt locked_decisions] |
| `yune-rime-api` internal ABI/runtime modules | 0.1.0 | C ABI entrypoints, runtime paths, deployment sync, session lifecycle, and userdb levers. [VERIFIED: crates/yune-rime-api/Cargo.toml; VERIFIED: crates/yune-rime-api/src/userdb.rs; VERIFIED: crates/yune-rime-api/src/runtime.rs] | Existing external boundary must remain the levers/userdb ABI. [VERIFIED: prompt locked_decisions] |
| `serde_yaml` | 0.9 in manifest | Existing runtime config and deployment YAML parsing/writing. [VERIFIED: crates/yune-rime-api/Cargo.toml; VERIFIED: crates/yune-rime-api/src/runtime.rs] | Keep using existing YAML stack for installation metadata and config backups. [VERIFIED: crates/yune-rime-api/src/deployment.rs] |

### Supporting

| Library / Component | Version | Purpose | When to Use |
|---------------------|---------|---------|-------------|
| `regex` | 1.x in manifest | Existing schema/spelling/comment pattern support. [VERIFIED: crates/yune-rime-api/Cargo.toml; VERIFIED: crates/yune-core/src/translator/mod.rs] | Not central to userdb, but keep existing dependency for translator/config interactions. [VERIFIED: crates/yune-core/src/translator/mod.rs] |
| `libc` | 0.2 in manifest | Existing ABI/platform compatibility helpers. [VERIFIED: crates/yune-rime-api/Cargo.toml] | Do not add userdb FFI ownership to core; keep ABI details in `yune-rime-api`. [VERIFIED: AGENTS.md] |
| `libloading` | 0.8 dev-dependency | Frontend-style dynamic loading tests. [VERIFIED: crates/yune-rime-api/Cargo.toml] | Relevant only if Phase 05 touches frontend-style integration tests. [VERIFIED: .planning/codebase/TESTING.md] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Documented Rust userdb abstraction | Direct LevelDB crate / C LevelDB binding | Direct LevelDB may improve on-disk compatibility but adds native dependency/build complexity; prompt permits behavior-compatible abstraction. [VERIFIED: prompt locked_decisions; VERIFIED: crates/yune-rime-api/Cargo.toml] |
| Atomic file-backed ordered KV store | Current `HashSet` text merge | Atomic KV supports metadata, prefix query, transactions, and conflict merge; current text merge cannot represent `commits/dee/tick` semantics. [VERIFIED: crates/yune-rime-api/src/userdb.rs; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc] |
| Core `CandidateRanker` for learned ranking | Classic userdb translator/source | Ranker would violate locked decision; userdb must affect classic deterministic candidate quality/order. [VERIFIED: prompt locked_decisions; VERIFIED: crates/yune-core/src/engine.rs] |

**Installation:** No new third-party package is required for the recommended first implementation if using a file-backed Rust abstraction. [VERIFIED: crates/yune-rime-api/Cargo.toml; ASSUMED]

```bash
# No new dependency required for 05-01 if using std-backed atomic files.
# If a direct LevelDB backend is chosen later, add the selected crate only after a separate dependency review.
```

**Version verification:** There are no npm packages in the recommended stack; Cargo/Rust were probed in the current shell and `cargo`/`rustc` were not available, while `node v24.14.1` and `npm 11.11.0` were available. [VERIFIED: command -v cargo/rustc/node/npm]

## Architecture Patterns

### System Architecture Diagram

```text
RIME frontend / levers API
        |
        v
RimeLevers*UserDict* / RimeSyncUserData / RimeRunTask("user_dict_sync")
        |
        v
Resource ID validation + RuntimePaths (user_data_dir, sync_dir, user_id)
        |
        +---------------------> reject unsafe logical IDs before filesystem joins
        |
        v
UserDbManager facade in yune-rime-api
        |
        +--> Snapshot backup/export/import/restore/sync/recovery/upgrade
        |        |
        |        v
        |   UserDbStore trait (metadata, ordered records, prefix query, transactions)
        |        |
        |        v
        |   File-backed or LevelDB-backed implementation under user_data_dir
        |
        +--> Learning adapter for session commits
                 |
                 v
            yune-core userdb translator/learning contract
                 |
                 v
            Engine::refresh_candidates classic candidate ordering
```

Diagram reflects existing API/runtime/core ownership plus the recommended abstraction boundary. [VERIFIED: crates/yune-rime-api/src/userdb.rs; VERIFIED: crates/yune-rime-api/src/runtime.rs; VERIFIED: crates/yune-core/src/engine.rs; VERIFIED: prompt locked_decisions]

### Recommended Project Structure

```text
crates/yune-rime-api/src/
├── userdb.rs                 # Keep exported C ABI functions and thin manager delegation. [VERIFIED: existing file]
├── userdb/
│   ├── mod.rs                # UserDbManager facade and public internal API. [ASSUMED]
│   ├── record.rs             # UserDbValue, key parsing/formatting, formula helpers. [ASSUMED]
│   ├── store.rs              # UserDbStore trait, metadata, query/transaction contracts. [ASSUMED]
│   ├── file_store.rs         # std-backed compatible storage with atomic temp replacement. [ASSUMED]
│   ├── snapshot.rs           # Uniform userdb snapshot read/write with metadata. [ASSUMED]
│   ├── sync.rs               # conflict-aware UserDbMerger semantics. [ASSUMED]
│   └── recovery.rs           # fail-closed repair/recreate/restore flow. [ASSUMED]
└── tests/
    ├── userdb.rs             # ABI lifecycle, sync, snapshot, recovery, rollback tests. [VERIFIED: existing file]
    └── resource_id.rs        # logical ID and file path boundary tests. [VERIFIED: existing file]

crates/yune-core/src/
├── userdb.rs                 # Storage-agnostic learned candidate model and ranking math. [ASSUMED]
├── translator/mod.rs         # Install userdb translator beside table translator when schema enables user_dict. [VERIFIED: existing file]
└── engine.rs                 # Commit event capture hook or return richer commit metadata. [VERIFIED: existing file]
```

### Pattern 1: Keep ABI Functions Thin

**What:** Existing exported `RimeLevers*UserDict*` functions should validate pointers/C strings, validate logical dict names, and delegate to an internal Rust manager. [VERIFIED: crates/yune-rime-api/src/userdb.rs]

**When to use:** Use this for backup, restore, import, export, iterator, sync, recovery, and upgrade work. [VERIFIED: crates/yune-rime-api/src/userdb.rs; VERIFIED: crates/yune-rime-api/src/deployment.rs]

**Example:**

```rust
// Source: crates/yune-rime-api/src/userdb.rs
// Planning shape only: keep unsafe C parsing here, move behavior to a manager.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversBackupUserDict(dict_name: *const c_char) -> Bool {
    let Some(dict_name) = optional_c_string(dict_name) else {
        return FALSE;
    };
    bool_from(userdb_manager().backup_user_dict(&dict_name))
}
```

The example is a planning pattern, not current code. [ASSUMED]

### Pattern 2: Model Librime UserDbValue Exactly at the Boundary

**What:** Store learned entries as `(key, UserDbValue { commits, dee, tick })`, where the packed form is `c=<commits> d=<dee> t=<tick>`. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc:17-50]

**When to use:** Use this for snapshot parsing/writing, conflict merge, import/export, learning updates, and candidate ranking. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_dictionary.cc]

**Example:**

```rust
// Source: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc: UserDbValue::Pack/Unpack
#[derive(Clone, Debug, PartialEq)]
struct UserDbValue {
    commits: i32,
    dee: f64,
    tick: u64,
}

impl UserDbValue {
    fn pack(&self) -> String {
        format!("c={} d={} t={}", self.commits, self.dee, self.tick)
    }
}
```

The Rust shape is a planning recommendation; exact formatting of floating values must be covered by tests against expected observable strings. [ASSUMED]

### Pattern 3: Use Atomic File Transactions for the Compatible Abstraction

**What:** If direct LevelDB compatibility is deferred, implement a file-backed ordered key/value store that writes snapshots or store files to temporary paths and atomically renames them into place only after validation succeeds. [ASSUMED]

**When to use:** Use this for 05-01 to get deterministic recovery/rollback/failure tests without introducing a native LevelDB dependency. [ASSUMED; VERIFIED: prompt locked_decisions]

**Why:** Librime LevelDB transactions buffer writes in a batch and only persist on commit; fail-closed Yune behavior should preserve the last valid dictionary if a write is interrupted. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/level_db.cc:302-325; VERIFIED: prompt locked_decisions]

### Anti-Patterns to Avoid

- **Continuing line append/dedupe sync:** It cannot represent metadata, ticks, deletion markers, decayed `dee`, or conflict-aware max commit/dee semantics. [VERIFIED: crates/yune-rime-api/src/userdb.rs:336-365; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc:203-239]
- **Putting filesystem or C ABI pointers in `yune-core`:** The locked decision requires `yune-core` to remain independent of C ABI pointers/storage engine details. [VERIFIED: prompt locked_decisions]
- **Using `CandidateRanker` as userdb learning:** Optional rankers run after filters and can replace candidate order, but userdb behavior must be classic deterministic translator/ranking behavior. [VERIFIED: crates/yune-core/src/engine.rs:711-735; VERIFIED: prompt locked_decisions]
- **Mixing test moves with behavior changes:** Refactor guidance requires behavior-free mechanical extraction and focused behavior slices. [VERIFIED: docs/refactor-plan.md]
- **Accepting snapshot filenames as dict names:** User dictionary names are logical resource IDs; snapshot file paths are file paths and must be handled separately. [VERIFIED: crates/yune-rime-api/src/resource_id.rs; VERIFIED: crates/yune-rime-api/src/tests/resource_id.rs]

## Recommended Storage/Lifecycle Design

### `UserDbStore` Contract

Use a storage trait with these operations: `open`, `open_read_only`, `exists`, `remove`, `metadata_get/update`, `fetch/update/erase`, `query_prefix`, `query_all`, `begin_transaction`, `abort_transaction`, `commit_transaction`, `backup_snapshot`, `restore_snapshot`, and `recover`. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/db.h; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/level_db.cc]

The trait should expose ordered iteration because librime `DbAccessor` supports `Jump`, `Reset`, and sequential `GetNextRecord`, and predictive/backdated lookup depends on prefix-ordered scans. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/db.h; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_dictionary.cc]

Keep the trait and concrete storage in `yune-rime-api` for Phase 05 unless core needs a test-only in-memory read interface; `yune-core` should see only storage-agnostic learned candidate inputs or a small trait without filesystem concerns. [VERIFIED: prompt locked_decisions; ASSUMED]

### File-Backed Compatible Abstraction

Prefer a std-backed abstraction for 05-01 unless direct LevelDB dependency review is explicitly added to the plan. [ASSUMED]

Recommended on-disk shape: a deterministic text or binary store containing metadata and sorted records, with all writes staged to a temp file and atomically renamed; snapshots remain uniform `*.userdb.txt` for sync/import/export compatibility. [ASSUMED; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc]

Use a temporary restore DB abstraction just like librime `UserDictManager::Restore`: parse snapshot into temp state, validate `db_type == userdb` and non-empty db name, then merge into destination. [VERIFIED: /Users/trenton/Projects/librime/src/rime/lever/user_dict_manager.cc:72-104]

### Metadata

Store and snapshot at least `/db_name`, `/db_type`, `/tick`, `/user_id`, and `/rime_version` because librime helper code reads these fields. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc:142-179; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/level_db.cc:290-300]

Use `RuntimePaths.user_id` and deployment's installation metadata as the Yune source for `/user_id`. [VERIFIED: crates/yune-rime-api/src/runtime.rs; VERIFIED: crates/yune-rime-api/src/deployment.rs]

### Conflict-Aware Merge

Implement `UserDbMerger` semantics exactly enough for observable tests: when restoring a snapshot, read local tick and remote tick, decay local/remote `dee` to their tick contexts with `formula_d(0, tick, dee, old_tick)`, keep the value with maximum absolute commits, keep maximum decayed `dee`, set merged tick to `max(local_tick, remote_tick)`, and update `/tick` and `/user_id` after merging any entries. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc:182-239; VERIFIED: /Users/trenton/Projects/librime/src/rime/algo/dynamics.h]

### Recovery and Rollback

Implement rollback as an active transaction abort that preserves prior durable state, and model librime's recent-transaction rule with a test-controlled clock if the public API exposes rollback timing. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_dictionary.cc:486-510; ASSUMED]

Implement recovery fail-closed: if store parse/repair fails, do not overwrite the last valid db; if a recreate path is used, restore only from a validated current snapshot. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db_recovery_task.cc; VERIFIED: prompt locked_decisions]

## Recommended Learning/Ranking Design

### Learning Entry Point

Add a commit-learning adapter in `yune-rime-api` around session commit paths that already append unread commits: `RimeProcessKey`, `RimeCommitComposition`, key-binder commits, shape commits, and processor-generated commits. [VERIFIED: crates/yune-rime-api/src/lib.rs:322-519,1132-1138]

The adapter must capture candidate metadata before `Engine::clear_composition()` removes candidates, because `Engine::commit_candidate` currently records only `candidate_type` and `text`. [VERIFIED: crates/yune-core/src/engine.rs:692-709]

Recommended core change: return or expose a `CommittedCandidate` record containing text, candidate source/type, lookup code/custom code, and enough learned-entry metadata to update userdb. [ASSUMED; VERIFIED: crates/yune-core/src/engine.rs]

### Userdb Candidate Generation

Add a classic userdb translator/source that emits learned candidates before optional AI rankers and after table candidates are assembled, preserving `Engine::refresh_candidates` translator/filter/ranker order. [ASSUMED; VERIFIED: crates/yune-core/src/engine.rs:711-735]

Add a distinct `CandidateSource::UserTable` or equivalent classic source label if tests need librime-like `user_table` commit type and learning history distinctions. [ASSUMED; VERIFIED: /Users/trenton/Projects/librime/src/rime/gear/table_translator.cc]

### Ranking Math

Implement `formula_d(d, t, da, ta) = d + da * exp((ta - t) / 200)` and `formula_p` with librime constants before using userdb values in candidate quality. [VERIFIED: /Users/trenton/Projects/librime/src/rime/algo/dynamics.h]

Create userdb candidate weight as `log(max(formula_p(...), DBL_EPSILON)) + credibility` and then map to Yune's `quality` model so `Engine::refresh_candidates` sorts deterministically. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_dictionary.cc:532-567; VERIFIED: crates/yune-core/src/engine.rs:720-725]

In table translation, model user phrase priority/bonus: librime's `MakePhrase` applies `exp(e->weight) + initial_quality + incomplete penalty + 0.5` for user phrases. [VERIFIED: /Users/trenton/Projects/librime/src/rime/gear/table_translator.cc:90-112]

### Predictive Lookup and Backdating

Implement exact lookup first, then predictive prefix lookup with `~remaining_code` comments and sorted exact-match ranges. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_dictionary.cc:357-419]

Treat backdated DFS lookup as a later subtask inside 05-02 unless Phase 04 has already delivered sufficient prism/syllable graph data; current Yune source dictionary/prism support may not expose all librime graph information needed for full `DfsLookup`. [ASSUMED; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_dictionary.cc:215-303]

## Test and Quality Strategy

### Test Owners

| Behavior | Owning implementation module | Owning test module | Librime comparison target |
|----------|------------------------------|--------------------|---------------------------|
| UserDbValue parse/pack and formulas | `crates/yune-rime-api/src/userdb/record.rs` or `crates/yune-core/src/userdb.rs` | New focused unit tests near owning module plus `crates/yune-rime-api/src/tests/userdb.rs` | `user_db.cc`, `dynamics.h` [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc; VERIFIED: /Users/trenton/Projects/librime/src/rime/algo/dynamics.h] |
| Snapshot backup/restore/import/export | `crates/yune-rime-api/src/userdb/snapshot.rs` and `userdb.rs` facade | `crates/yune-rime-api/src/tests/userdb.rs` | `user_db.cc`, `table_db.cc`, `user_dict_manager.cc` [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/table_db.cc; VERIFIED: /Users/trenton/Projects/librime/src/rime/lever/user_dict_manager.cc] |
| Conflict-aware sync | `crates/yune-rime-api/src/userdb/sync.rs` | `crates/yune-rime-api/src/tests/userdb.rs` | `UserDbMerger`, `UserDictManager::Synchronize` [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc; VERIFIED: /Users/trenton/Projects/librime/src/rime/lever/user_dict_manager.cc] |
| Transactions/rollback/recovery | `crates/yune-rime-api/src/userdb/store.rs` and `recovery.rs` | `crates/yune-rime-api/src/tests/userdb.rs` | `LevelDb` transaction/recovery and `UserDbRecoveryTask` [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/level_db.cc; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db_recovery_task.cc] |
| Runtime learning | `crates/yune-rime-api/src/session.rs`/commit adapter plus `crates/yune-core/src/engine.rs` commit metadata | `crates/yune-rime-api/src/tests/userdb.rs` and core tests near engine/userdb module | `Memory`, `TableTranslator::Memorize`, `UserDictionary::UpdateEntry` [VERIFIED: /Users/trenton/Projects/librime/src/rime/gear/memory.cc; VERIFIED: /Users/trenton/Projects/librime/src/rime/gear/table_translator.cc; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_dictionary.cc] |
| Predictive lookup/ranking | `crates/yune-core/src/userdb.rs` and `translator/mod.rs` | Core translator tests plus ABI schema-loaded integration tests | `UserDictionary::LookupWords`, `TableTranslation` [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_dictionary.cc; VERIFIED: /Users/trenton/Projects/librime/src/rime/gear/table_translator.cc] |
| Resource-ID safety | `crates/yune-rime-api/src/resource_id.rs` and `userdb.rs` | `crates/yune-rime-api/src/tests/resource_id.rs` | Project security constraint, not librime internals [VERIFIED: crates/yune-rime-api/src/resource_id.rs; VERIFIED: AGENTS.md] |
| Test splitting | Test files only | New split files under `crates/yune-core/src/*` tests or `crates/yune-rime-api/tests/` | `docs/refactor-plan.md` [VERIFIED: docs/refactor-plan.md] |

### Required Focused Tests

- `UserDbValue` parse/pack handles valid fields, unknown fields, invalid numeric fields, and `dee` capping at 10000. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc:27-50]
- Snapshot backup writes metadata and data rows; restore rejects missing `/db_type: userdb`, missing db name, malformed rows, and unsafe db names. [VERIFIED: /Users/trenton/Projects/librime/src/rime/lever/user_dict_manager.cc:72-104; VERIFIED: crates/yune-rime-api/src/resource_id.rs]
- Sync conflict test with local and remote records verifies max absolute commits, max decayed `dee`, merged max tick, and snapshot backup after merge. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc:203-239]
- Import/export tests verify table row order `phrase<Tab>code<Tab>commits`, deleted entries omitted on export, positive commits max behavior, and negative commits deletion behavior. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/table_db.cc:17-60; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc:248-263]
- Transaction tests verify commit persists, abort discards, rollback fails closed when no transaction exists, and interrupted temp writes preserve last valid state. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/level_db.cc:302-325; VERIFIED: prompt locked_decisions]
- Learning tests verify selecting/committing a table candidate increments userdb commits/tick/dee through normal session flow and updates subsequent candidate ordering. [VERIFIED: /Users/trenton/Projects/librime/src/rime/gear/memory.cc; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_dictionary.cc]
- Backspace/unhandled-key tests verify recent transaction discard/commit behavior if this behavior is exposed in Yune's key flow. [VERIFIED: /Users/trenton/Projects/librime/src/rime/gear/memory.cc:150-159]
- Predictive lookup tests verify prefix lookup returns comments starting with `~` and exact match priority is stable. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_dictionary.cc:357-419]

### Quality Gates

- Per behavior task: run the focused test owner, such as `cargo test -p yune-rime-api userdb` or the core userdb/translator module tests. [VERIFIED: .planning/codebase/TESTING.md]
- After formatting-affecting code: run `cargo fmt`. [VERIFIED: .planning/codebase/TESTING.md; VERIFIED: AGENTS.md]
- After shared core/runtime behavior changes: run relevant package tests and `cargo test --workspace`. [VERIFIED: .planning/codebase/TESTING.md; VERIFIED: AGENTS.md]
- Before closing implementation slices with broad changes: run `cargo clippy --workspace --all-targets -- -D warnings` when implementation changes warrant it. [VERIFIED: AGENTS.md]
- Current shell does not expose `cargo` or `rustc`, so executor plans must either ensure Rust toolchain availability or document the environment blocker before running gates. [VERIFIED: command -v cargo/rustc]

## Security and Scaling Considerations

### Security Controls

- Keep logical user dictionary names separate from file paths; dictionary names must pass `validate_user_dict_name` before joining with `user_data_dir`. [VERIFIED: crates/yune-rime-api/src/resource_id.rs; VERIFIED: crates/yune-rime-api/src/userdb.rs]
- Treat snapshot paths supplied to restore as file paths, but validate the embedded `/db_name` metadata or derived dict name before selecting a destination. [VERIFIED: /Users/trenton/Projects/librime/src/rime/lever/user_dict_manager.cc:72-104; VERIFIED: crates/yune-rime-api/src/resource_id.rs]
- Reject malformed snapshots before mutating destination storage; restore should stage in temp state and merge only after validation. [VERIFIED: /Users/trenton/Projects/librime/src/rime/lever/user_dict_manager.cc:72-104]
- Do not follow symlinked userdb files or snapshots unless a deliberate policy is documented; this is a recommended hardening measure for local filesystem safety. [ASSUMED]
- Keep FFI pointer validation and `# Safety` docs on exported unsafe functions. [VERIFIED: crates/yune-rime-api/src/userdb.rs; VERIFIED: AGENTS.md]

### Scaling Controls

- Replace full-file `HashSet` sync with ordered record streaming or an indexed store; current full-file merge is a documented scaling limit. [VERIFIED: crates/yune-rime-api/src/userdb.rs:336-365; VERIFIED: .planning/codebase/CONCERNS.md]
- Bound or page predictive lookup results; librime lazy lookup starts with a limit and expands by a factor of 10. [VERIFIED: /Users/trenton/Projects/librime/src/rime/gear/table_translator.cc:117-187]
- Avoid cloning full candidate lists as part of userdb lookup; existing candidate/context snapshots already clone large structures and are documented scaling concerns. [VERIFIED: .planning/codebase/CONCERNS.md]
- Keep long-lived session memory bounded because `commit_history` is currently unbounded. [VERIFIED: crates/yune-core/src/state.rs; VERIFIED: .planning/codebase/CONCERNS.md]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Conflict resolution | Line append/dedupe | Librime-shaped `UserDbMerger` semantics | Merge depends on tick, decayed `dee`, commit signs, and metadata. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc] |
| Ranking math | Ad hoc frequency score | `formula_d` and `formula_p` equivalents | Librime userdb candidate quality is based on these formulas. [VERIFIED: /Users/trenton/Projects/librime/src/rime/algo/dynamics.h; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_dictionary.cc] |
| Transaction semantics | Direct overwrite per update | Store transaction trait with begin/abort/commit | Librime LevelDB batches writes and abort clears pending changes. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/level_db.cc] |
| Restore | Copy snapshot to destination | Temp restore + metadata validation + merge | Librime restores to `.temp`, validates db type/name, then merges. [VERIFIED: /Users/trenton/Projects/librime/src/rime/lever/user_dict_manager.cc] |
| Logical ID safety | Path string filtering at call sites | `validate_user_dict_name` before filesystem joins | Central helper already rejects traversal and suffix tricks. [VERIFIED: crates/yune-rime-api/src/resource_id.rs] |
| Learning substitution | `HistoryTranslator` or `CandidateRanker` | Commit-driven userdb update and classic translator output | Locked decision explicitly rejects those substitutes. [VERIFIED: prompt locked_decisions] |

**Key insight:** The hard part is not the storage engine name; it is preserving the observable userdb record, metadata, merge, transaction, and ranking semantics at the RIME boundary. [VERIFIED: prompt locked_decisions; VERIFIED: /Users/trenton/Projects/librime source]

## Runtime State Inventory

> Phase 05 is not a rename/refactor/migration phase, but it changes persisted runtime user data, so persisted state still needs explicit inventory. [VERIFIED: .planning/ROADMAP.md]

| Category | Items Found | Action Required |
|----------|-------------|-----------------|
| Stored data | `*.userdb` files under runtime `user_data_dir` and `*.userdb.txt` snapshots under `sync_dir/<user_id>/`. [VERIFIED: crates/yune-rime-api/src/userdb.rs; VERIFIED: crates/yune-rime-api/src/runtime.rs] | Add read/upgrade path for existing plain shim files or document one-time conversion; preserve last valid db on malformed data. [ASSUMED; VERIFIED: prompt locked_decisions] |
| Live service config | No external live service config is part of current userdb flow; runtime config is local `installation.yaml`. [VERIFIED: .planning/codebase/INTEGRATIONS.md; VERIFIED: crates/yune-rime-api/src/runtime.rs] | None for external services; include local `installation.yaml` fixtures in tests. [VERIFIED: crates/yune-rime-api/src/tests/userdb.rs] |
| OS-registered state | No OS-registered userdb state detected in project docs or codebase integration audit. [VERIFIED: .planning/codebase/INTEGRATIONS.md] | None. [VERIFIED: .planning/codebase/INTEGRATIONS.md] |
| Secrets/env vars | No required env vars or secrets are detected; runtime paths come from `RimeTraits` and local metadata. [VERIFIED: .planning/codebase/INTEGRATIONS.md; VERIFIED: crates/yune-rime-api/src/runtime.rs] | None. [VERIFIED: .planning/codebase/INTEGRATIONS.md] |
| Build artifacts | No userdb build artifacts are currently produced; storage is local runtime files. [VERIFIED: .planning/codebase/INTEGRATIONS.md; VERIFIED: crates/yune-rime-api/src/userdb.rs] | None for existing artifacts; adding a native LevelDB dependency would require separate build toolchain planning. [ASSUMED; VERIFIED: current environment cargo/rustc missing] |

## Common Pitfalls

### Pitfall 1: Treating `.userdb.txt` snapshots as simple import/export files

**What goes wrong:** Sync and restore pass tests that only append rows but fail conflict, metadata, deletion, and tick semantics. [VERIFIED: crates/yune-rime-api/src/userdb.rs; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc]

**Why it happens:** Librime uses uniform userdb snapshots for backup/restore and separate table format for import/export. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/table_db.cc]

**How to avoid:** Build separate snapshot and table import/export parsers/writers. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/table_db.cc]

**Warning signs:** Tests only assert copied text lines and do not inspect `/tick`, `/db_type`, deleted entries, or merged `dee`. [VERIFIED: crates/yune-rime-api/src/tests/userdb.rs]

### Pitfall 2: Losing commit metadata before learning

**What goes wrong:** Learning records only committed text and cannot reconstruct custom code, candidate type, or table/user phrase source. [VERIFIED: crates/yune-core/src/engine.rs:692-709]

**Why it happens:** Current `commit_candidate` records only `candidate_type` and `text`, then clears composition. [VERIFIED: crates/yune-core/src/engine.rs:692-709]

**How to avoid:** Capture a richer committed-candidate record before clearing composition. [ASSUMED]

**Warning signs:** Learning tests can increment a phrase but cannot distinguish table, sentence, completion, user_table, or raw commits. [ASSUMED; VERIFIED: crates/yune-core/src/state.rs]

### Pitfall 3: Pulling storage or FFI into `yune-core`

**What goes wrong:** Core becomes coupled to runtime paths, C strings, sync directories, or a concrete storage engine. [VERIFIED: prompt locked_decisions]

**Why it happens:** Learning/ranking needs both engine events and persistent storage, tempting direct imports across crate boundaries. [ASSUMED; VERIFIED: crates/yune-core/src/engine.rs; VERIFIED: crates/yune-rime-api/src/userdb.rs]

**How to avoid:** Define storage-neutral core models and keep concrete filesystem/ABI work in `yune-rime-api`. [VERIFIED: prompt locked_decisions]

**Warning signs:** `yune-core` starts importing `PathBuf`, `CString`, `RimeTraits`, or `runtime_paths` for userdb behavior. [ASSUMED]

### Pitfall 4: Testing rollback with real wall-clock sleeps

**What goes wrong:** Tests become slow or flaky around librime's 3-second recent transaction rollback window. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_dictionary.cc:486-510]

**Why it happens:** Librime uses `time(NULL)` to decide whether rollback is recent. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_dictionary.cc:486-510]

**How to avoid:** Inject a test clock into Yune's transaction manager or test immediate rollback only until a timing abstraction exists. [ASSUMED]

**Warning signs:** Tests use `sleep` to cross transaction age thresholds. [ASSUMED]

### Pitfall 5: Splitting tests during semantic changes

**What goes wrong:** Review cannot distinguish behavior changes from mechanical moves. [VERIFIED: docs/refactor-plan.md]

**Why it happens:** Phase 05 includes both userdb behavior and QUAL-03 test split goals. [VERIFIED: .planning/ROADMAP.md]

**How to avoid:** Make 05-03 a separate mechanical/quality slice after 05-01 and 05-02 behavior tests are stable. [VERIFIED: .planning/ROADMAP.md; VERIFIED: docs/refactor-plan.md]

**Warning signs:** A commit both changes userdb code and moves thousands of test lines. [VERIFIED: docs/refactor-plan.md]

## Code Examples

### Librime UserDbValue Packing Target

```cpp
// Source: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc
string UserDbValue::Pack() const {
  std::ostringstream packed;
  packed << "c=" << commits << " d=" << dee << " t=" << tick;
  return packed.str();
}
```

Use this as the compatibility target for Yune value serialization tests. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc]

### Current Yune Plain Merge to Replace

```rust
// Source: crates/yune-rime-api/src/userdb.rs
let destination_text = fs::read_to_string(&destination)?;
let snapshot_text = fs::read_to_string(snapshot)?;
let mut seen = destination_text
    .lines()
    .map(ToOwned::to_owned)
    .collect::<HashSet<_>>();
let mut merged = destination_text;
for line in snapshot_text.lines() {
    if line.trim().is_empty() || !seen.insert(line.to_owned()) {
        continue;
    }
    if !merged.is_empty() && !merged.ends_with('\n') {
        merged.push('\n');
    }
    merged.push_str(line);
    merged.push('\n');
}
fs::write(destination, merged)
```

This code is the concrete anti-pattern for Phase 05 because it is not conflict-aware and is whole-file based. [VERIFIED: crates/yune-rime-api/src/userdb.rs]

### Librime Merge Semantics Target

```cpp
// Source: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc
if (std::abs(o.commits) < std::abs(v.commits))
  o.commits = v.commits;
o.dee = (std::max)(o.dee, v.dee);
o.tick = max_tick_;
return db_->Update(key, o.Pack()) && ++merged_entries_;
```

Yune sync tests should assert these conflict semantics directly. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc]

### Current Candidate Refresh Order

```rust
// Source: crates/yune-core/src/engine.rs
candidates.sort_by(|left, right| {
    right
        .quality
        .partial_cmp(&left.quality)
        .unwrap_or(Ordering::Equal)
});
for filter in &self.filters {
    filter.apply_with_context(&mut candidates, &self.options, &self.context);
}
for ranker in &self.rankers {
    if let RerankResult::Ready(ranked) = ranker.try_rerank(&self.context, &candidates) {
        candidates = ranked;
    }
}
```

Userdb candidates must enter before optional rankers if they are part of classic deterministic behavior. [VERIFIED: crates/yune-core/src/engine.rs; VERIFIED: prompt locked_decisions]

## State of the Art

| Old Approach | Current Approach for Phase 05 | When Changed / Source | Impact |
|--------------|-------------------------------|-----------------------|--------|
| Plain `*.userdb` file copy and line dedupe | Userdb abstraction with metadata, ordered records, transactions, snapshots, and conflict-aware merge | Phase 05 target. [VERIFIED: .planning/ROADMAP.md] | Required for USERDB-01 and USERDB-02. [VERIFIED: .planning/REQUIREMENTS.md] |
| Learning via transient history only | Commit-driven persistent userdb updates | Phase 05 target. [VERIFIED: .planning/ROADMAP.md; VERIFIED: prompt locked_decisions] | Required for USERDB-03. [VERIFIED: .planning/REQUIREMENTS.md] |
| Optional AI ranker for ordering experiments | Classic deterministic userdb quality/order before AI rankers | Locked Phase 05 decision. [VERIFIED: prompt locked_decisions] | Prevents AI from substituting for librime-compatible learning. [VERIFIED: prompt locked_decisions] |
| Mechanical refactor mixed with feature work | Dedicated test split slice after behavior work | Existing refactor rule. [VERIFIED: docs/refactor-plan.md] | Required for QUAL-03. [VERIFIED: .planning/REQUIREMENTS.md] |

**Deprecated/outdated:**

- The current plain text userdb shim is insufficient as a compatibility claim for sync, recovery, transactions, learning, predictive lookup, and frequency updates. [VERIFIED: docs/analysis.md; VERIFIED: .planning/codebase/CONCERNS.md]
- Full-file `HashSet` sync is a scaling and correctness bottleneck. [VERIFIED: crates/yune-rime-api/src/userdb.rs; VERIFIED: .planning/codebase/CONCERNS.md]

## Plan Slice Guidance (05-01, 05-02, 05-03)

### 05-01: Storage, Snapshot, Recovery, Sync, Rollback

**Goal:** Implement userdb storage/lifecycle compatibility beyond the plain text shim. [VERIFIED: .planning/ROADMAP.md]

**Recommended tasks:**

1. Add `UserDbValue`, key formatting/parsing, metadata constants, and `formula_d` tests. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc; VERIFIED: /Users/trenton/Projects/librime/src/rime/algo/dynamics.h]
2. Introduce `UserDbStore` plus a deterministic file-backed implementation or selected LevelDB backend. [ASSUMED; VERIFIED: prompt locked_decisions]
3. Replace `backup_plain_user_dict`, `RimeLeversRestoreUserDict`, import/export, and `sync_all_user_dicts` internals with manager methods while preserving C ABI signatures. [VERIFIED: crates/yune-rime-api/src/userdb.rs]
4. Implement snapshot format with metadata and data rows, separate from table import/export format. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/table_db.cc]
5. Implement restore through temp validation and merge; reject malformed snapshots without mutating destination. [VERIFIED: /Users/trenton/Projects/librime/src/rime/lever/user_dict_manager.cc]
6. Implement conflict-aware sync with decayed `dee`, max absolute commits, max tick, and backup-after-merge. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc; VERIFIED: /Users/trenton/Projects/librime/src/rime/lever/user_dict_manager.cc]
7. Implement transaction begin/abort/commit and recovery fail-closed behavior. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/level_db.cc; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db_recovery_task.cc]

**Quality gates:** Focused `cargo test -p yune-rime-api userdb`, `cargo test -p yune-rime-api resource_id`, `cargo fmt`, and `cargo test --workspace` after shared behavior changes. [VERIFIED: .planning/codebase/TESTING.md]

**Dependency note:** Current shell lacks `cargo`/`rustc`; execution must provision or expose the Rust toolchain before running these gates. [VERIFIED: command -v cargo/rustc]

### 05-02: Learning, Frequency, Predictive Lookup, Backdated Scan

**Goal:** Represent persistent userdb learning in runtime candidate ranking and persistence. [VERIFIED: .planning/ROADMAP.md]

**Recommended tasks:**

1. Extend core commit metadata so selected table/user/sentence candidates retain text, source, and custom code before composition is cleared. [ASSUMED; VERIFIED: crates/yune-core/src/engine.rs]
2. Add a userdb learning adapter in `yune-rime-api` session/commit paths that updates the store on normal commits and handles delete/backspace rollback semantics where supported. [ASSUMED; VERIFIED: crates/yune-rime-api/src/lib.rs; VERIFIED: /Users/trenton/Projects/librime/src/rime/gear/memory.cc]
3. Add storage-neutral userdb candidate lookup in `yune-core` or a core trait supplied by `yune-rime-api`, preserving core independence from filesystem and C ABI details. [ASSUMED; VERIFIED: prompt locked_decisions]
4. Implement exact and predictive lookup with `~remaining_code` comments and deterministic candidate quality based on librime formulas. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_dictionary.cc; VERIFIED: /Users/trenton/Projects/librime/src/rime/gear/table_translator.cc]
5. Add tests proving `HistoryTranslator` and `CandidateRanker` are not required for learned candidate ordering. [VERIFIED: prompt locked_decisions; VERIFIED: crates/yune-core/src/engine.rs]
6. Defer full DFS backdating only if prerequisite prism/syllable graph data is absent, but document the observable gap and add at least one focused test for the currently supported path. [ASSUMED; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_dictionary.cc]

**Quality gates:** Focused core userdb/translator/engine tests, focused ABI userdb learning tests, `cargo fmt`, relevant package tests, and workspace tests. [VERIFIED: .planning/codebase/TESTING.md]

### 05-03: Test Split and Quality Gates

**Goal:** Split remaining oversized tests where useful and codify future phase quality gates. [VERIFIED: .planning/ROADMAP.md]

**Recommended tasks:**

1. Split only tests whose behavior ownership is clear after 05-01 and 05-02, such as userdb lifecycle tests versus learning/ranking tests. [VERIFIED: docs/refactor-plan.md; ASSUMED]
2. Do not split `frontend_client.rs` unless Phase 05 work creates a clear frontend-surrogate ownership boundary; refactor guidance says that split should wait until transcript/replay design clarifies ownership. [VERIFIED: docs/refactor-plan.md]
3. Move core tests out of `crates/yune-core/src/lib.rs` only by module ownership and with behavior-free commits. [VERIFIED: .planning/codebase/CONCERNS.md; VERIFIED: docs/refactor-plan.md]
4. Codify Phase 05 gates in plan documentation or task checklists: focused tests, `cargo fmt`, relevant package tests, workspace tests, and clippy when warranted. [VERIFIED: .planning/codebase/TESTING.md; VERIFIED: AGENTS.md]

**Quality gates:** Test split commits should run before/after focused tests for the moved module and `cargo test --workspace`; no behavior diffs should be included. [VERIFIED: docs/refactor-plan.md; VERIFIED: .planning/codebase/TESTING.md]

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Rust `cargo` | Build/test/quality gates | No in current shell | — | Provision Rust toolchain or run gates in developer environment with Cargo. [VERIFIED: command -v cargo] |
| Rust `rustc` | Build/test/quality gates | No in current shell | — | Provision Rust toolchain or run gates in developer environment with Rust 1.76+. [VERIFIED: command -v rustc; VERIFIED: Cargo.toml] |
| CMake | Potential direct LevelDB native backend | No in current shell | — | Prefer std-backed abstraction for 05-01; only require CMake if direct native dependency is selected. [VERIFIED: command -v cmake; ASSUMED] |
| `leveldbutil` | Inspecting direct LevelDB files | No in current shell | — | Use librime source-level behavior tests or a Rust abstraction; direct LevelDB inspection would need installation. [VERIFIED: command -v leveldbutil; VERIFIED: prompt locked_decisions] |
| Node/npm | Graph/init tooling fallback only | Yes | node v24.14.1, npm 11.11.0 | Not required for implementation. [VERIFIED: command -v node/npm] |

**Missing dependencies with no fallback:**

- Cargo/Rust are blocking for actually running Phase 05 quality gates in this shell. [VERIFIED: command -v cargo/rustc]

**Missing dependencies with fallback:**

- CMake/LevelDB utilities are only blocking if a direct LevelDB backend is chosen; a documented compatible abstraction avoids that dependency for 05-01. [VERIFIED: prompt locked_decisions; VERIFIED: command -v cmake/leveldbutil]

## Validation Architecture

Validation Architecture is omitted because `.planning/config.json` explicitly sets `workflow.nyquist_validation` to `false`. [VERIFIED: .planning/config.json]

Use the test and quality strategy above instead of Nyquist-specific validation mapping. [VERIFIED: .planning/config.json; VERIFIED: .planning/codebase/TESTING.md]

## Security Domain

Security enforcement is enabled because `.planning/config.json` sets `workflow.security_enforcement` to `true`. [VERIFIED: .planning/config.json]

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | No | No user authentication exists in current local ABI/storage flow. [VERIFIED: .planning/codebase/INTEGRATIONS.md] |
| V3 Session Management | Yes | Keep RIME session lifecycle deterministic; `RimeSyncUserData` currently cleans sessions before sync. [VERIFIED: crates/yune-rime-api/src/deployment.rs; VERIFIED: crates/yune-rime-api/src/session.rs] |
| V4 Access Control | Yes | Treat runtime resource IDs as logical IDs and reject traversal before filesystem access. [VERIFIED: AGENTS.md; VERIFIED: crates/yune-rime-api/src/resource_id.rs] |
| V5 Input Validation | Yes | Validate C strings, dict names, snapshot metadata, numeric fields, TSV rows, and paths before mutation. [VERIFIED: crates/yune-rime-api/src/userdb.rs; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc] |
| V6 Cryptography | No | No cryptographic operation is part of current userdb scope. [VERIFIED: .planning/codebase/INTEGRATIONS.md] |
| V8 Data Protection | Yes | Preserve local user dictionary integrity with atomic writes, fail-closed recovery, and rollback tests. [VERIFIED: prompt locked_decisions; ASSUMED] |
| V12 File and Resources | Yes | Avoid path traversal, unsafe suffix handling, symlink surprises, and non-atomic overwrites in local files. [VERIFIED: crates/yune-rime-api/src/resource_id.rs; ASSUMED] |

### Known Threat Patterns for This Stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Path traversal through dict names | Tampering / Information Disclosure | `validate_user_dict_name` before every destination path join. [VERIFIED: crates/yune-rime-api/src/resource_id.rs] |
| Malformed snapshot overwrites valid DB | Tampering / Denial of Service | Restore into temp state, validate metadata/rows, then merge transactionally. [VERIFIED: /Users/trenton/Projects/librime/src/rime/lever/user_dict_manager.cc; VERIFIED: prompt locked_decisions] |
| Interrupted write corrupts userdb | Tampering / Denial of Service | Temp file + fsync/rename or transaction batch before commit. [ASSUMED; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/level_db.cc] |
| Large userdb exhausts memory during sync | Denial of Service | Avoid full-file `HashSet` merge; stream or use indexed ordered store. [VERIFIED: crates/yune-rime-api/src/userdb.rs; VERIFIED: .planning/codebase/CONCERNS.md] |
| FFI invalid pointer misuse | Elevation of Privilege / Tampering | Keep null checks, safety docs, and narrow unsafe boundary in exported functions. [VERIFIED: crates/yune-rime-api/src/userdb.rs; VERIFIED: AGENTS.md] |

## Risks, Deferrals, and Open Findings

1. **Direct LevelDB compatibility remains a decision point.**
   - What we know: Phase 05 permits a compatible Rust abstraction instead of cloning LevelDB internals. [VERIFIED: prompt locked_decisions]
   - What's unclear: Whether future migration requires reading existing librime LevelDB directories byte-for-byte. [ASSUMED]
   - Recommendation: Plan a file-backed compatible abstraction first and leave a documented backend trait seam for direct LevelDB later. [ASSUMED]

2. **Full backdated DFS lookup may require prism/syllable graph data not yet exposed in Yune.**
   - What we know: Librime backdating is tied to `SyllableGraph`, `Prism`, spelling properties, and DFS lookup. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_dictionary.cc]
   - What's unclear: Whether Phase 04 completed enough compiled dictionary/prism support before Phase 05 executes. [VERIFIED: .planning/ROADMAP.md]
   - Recommendation: Implement exact/predictive `LookupWords` first, then scope DFS backdating to available graph data or document the remaining gap. [ASSUMED]

3. **Current shell cannot run Rust quality gates.**
   - What we know: `cargo` and `rustc` were not found in this shell. [VERIFIED: command -v cargo/rustc]
   - What's unclear: Whether the executor environment will differ from the research shell. [ASSUMED]
   - Recommendation: Add Wave 0 environment gate to expose/provision Rust before implementation. [ASSUMED]

4. **Snapshot text exactness needs tests.**
   - What we know: Librime uses `TsvWriter` and metadata rows for uniform userdb snapshots. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc; VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/db_utils.cc]
   - What's unclear: Exact ordering/comment formatting requirements visible to real users across librime versions. [ASSUMED]
   - Recommendation: Lock Yune tests to behaviorally meaningful metadata/record parsing and stable export strings discovered from local librime fixtures. [ASSUMED]

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | No new third-party package is required for the recommended first implementation if using a file-backed Rust abstraction. | Standard Stack | Planner may omit a dependency needed for robust atomic file or locking behavior. |
| A2 | Recommended module paths under `crates/yune-rime-api/src/userdb/` and `crates/yune-core/src/userdb.rs` are future planning shapes. | Architecture Patterns | Actual code organization may prefer different names after implementation discovery. |
| A3 | A std-backed atomic file abstraction is preferable for 05-01 unless direct LevelDB dependency review is added. | Recommended Storage/Lifecycle Design | Direct LevelDB compatibility could become required by migration scope. |
| A4 | Backdated DFS lookup may need deferral depending on Phase 04 prism/syllable graph availability. | Recommended Learning/Ranking Design | USERDB-03 could be under-scoped if Phase 05 expects full DFS behavior immediately. |
| A5 | Symlink hardening should reject or explicitly handle symlinked userdb files/snapshots. | Security and Scaling Considerations | Could differ from librime behavior or platform expectations. |
| A6 | Test-clock injection is preferred for rollback timing tests. | Common Pitfalls | Implementation might choose immediate-only rollback tests and defer age thresholds. |
| A7 | Snapshot exact formatting may vary, so tests should emphasize behaviorally meaningful parsing unless exact output is confirmed. | Risks, Deferrals, and Open Findings | Compatibility tests may miss user-visible snapshot formatting differences. |

## Open Questions

1. **Should Phase 05 read existing librime LevelDB directories byte-for-byte, or only support librime-observable behavior through snapshots/import/export?**
   - What we know: Locked decision permits a documented compatible abstraction. [VERIFIED: prompt locked_decisions]
   - What's unclear: Migration expectations for existing users with `.userdb` LevelDB directories. [ASSUMED]
   - Recommendation: Treat direct LevelDB file compatibility as optional unless user confirms migration from existing librime userdb directories is required. [ASSUMED]

2. **What exact userdb source labels should Yune expose?**
   - What we know: Yune lacks `CandidateSource::UserTable`; librime uses user phrase distinctions and `user_table` appears in commit history logic. [VERIFIED: crates/yune-core/src/state.rs; VERIFIED: /Users/trenton/Projects/librime/src/rime/gear/table_translator.cc]
   - What's unclear: Whether external Yune JSON/ABI consumers require a distinct label or only ranking behavior. [ASSUMED]
   - Recommendation: Add a distinct classic userdb source if learning tests need to distinguish user phrases from table phrases. [ASSUMED]

3. **How far should Phase 05 go on full DFS/backdated lookup?**
   - What we know: Librime's deeper lookup uses `SyllableGraph`, `Prism`, spelling credibility, and backdating. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_dictionary.cc]
   - What's unclear: Whether prerequisite compiled dictionary/prism work exists when Phase 05 starts. [VERIFIED: .planning/ROADMAP.md]
   - Recommendation: Scope 05-02 into exact/predictive lookup first, then add backdated tests only for available graph support. [ASSUMED]

## Sources

### Primary (HIGH confidence)

- `AGENTS.md` - project constraints, stack, conventions, and workflow guidance. [VERIFIED: AGENTS.md]
- `.planning/REQUIREMENTS.md` - USERDB-01, USERDB-02, USERDB-03, QUAL-03, QUAL-04. [VERIFIED: .planning/REQUIREMENTS.md]
- `.planning/ROADMAP.md` - Phase 05 goal, success criteria, and slices. [VERIFIED: .planning/ROADMAP.md]
- `.planning/PROJECT.md` - compatibility oracle, constraints, active/out-of-scope work. [VERIFIED: .planning/PROJECT.md]
- `.planning/codebase/INTEGRATIONS.md` - current storage/integration audit. [VERIFIED: .planning/codebase/INTEGRATIONS.md]
- `.planning/codebase/CONCERNS.md` - userdb scaling gaps and oversized tests. [VERIFIED: .planning/codebase/CONCERNS.md]
- `.planning/codebase/TESTING.md` - test patterns and quality commands. [VERIFIED: .planning/codebase/TESTING.md]
- `docs/analysis.md`, `docs/roadmap.md`, `docs/refactor-plan.md` - compatibility direction and refactor rules. [VERIFIED: docs/analysis.md; VERIFIED: docs/roadmap.md; VERIFIED: docs/refactor-plan.md]
- `crates/yune-rime-api/src/userdb.rs` - current userdb ABI/storage shim. [VERIFIED: crates/yune-rime-api/src/userdb.rs]
- `crates/yune-rime-api/src/runtime.rs` and `deployment.rs` - runtime paths and sync tasks. [VERIFIED: crates/yune-rime-api/src/runtime.rs; VERIFIED: crates/yune-rime-api/src/deployment.rs]
- `crates/yune-rime-api/src/resource_id.rs` and tests - logical ID validation. [VERIFIED: crates/yune-rime-api/src/resource_id.rs; VERIFIED: crates/yune-rime-api/src/tests/resource_id.rs]
- `crates/yune-core/src/engine.rs`, `state.rs`, `translator/mod.rs` - current candidate/commit/ranking seams. [VERIFIED: crates/yune-core/src/engine.rs; VERIFIED: crates/yune-core/src/state.rs; VERIFIED: crates/yune-core/src/translator/mod.rs]
- `/Users/trenton/Projects/librime/src/rime/dict/user_db.cc` - UserDbValue, snapshots, merge/importer. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_db.cc]
- `/Users/trenton/Projects/librime/src/rime/dict/level_db.cc` - LevelDB metadata, transactions, recovery hooks. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/level_db.cc]
- `/Users/trenton/Projects/librime/src/rime/lever/user_dict_manager.cc` - backup/restore/import/export/sync/upgrade. [VERIFIED: /Users/trenton/Projects/librime/src/rime/lever/user_dict_manager.cc]
- `/Users/trenton/Projects/librime/src/rime/dict/user_dictionary.cc` - lookup, learning update, transactions, predictive/backdated behavior. [VERIFIED: /Users/trenton/Projects/librime/src/rime/dict/user_dictionary.cc]
- `/Users/trenton/Projects/librime/src/rime/gear/memory.cc` and `table_translator.cc` - commit-driven learning and user phrase ranking. [VERIFIED: /Users/trenton/Projects/librime/src/rime/gear/memory.cc; VERIFIED: /Users/trenton/Projects/librime/src/rime/gear/table_translator.cc]
- `/Users/trenton/Projects/librime/src/rime/algo/dynamics.h` - formula_d and formula_p. [VERIFIED: /Users/trenton/Projects/librime/src/rime/algo/dynamics.h]

### Secondary (MEDIUM confidence)

- None used; research relied on local project files and local librime source. [VERIFIED: tool usage]

### Tertiary (LOW confidence)

- Assumptions listed in the Assumptions Log. [ASSUMED]

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH for existing stack; MEDIUM for no-new-dependency recommendation because it is an implementation recommendation. [VERIFIED: Cargo.toml; ASSUMED]
- Architecture: HIGH for current seams; MEDIUM for proposed module paths. [VERIFIED: project source; ASSUMED]
- Librime behavior targets: HIGH for source-verified functions read from local `/Users/trenton/Projects/librime`. [VERIFIED: /Users/trenton/Projects/librime source]
- Pitfalls: HIGH for current plain sync and merge/ranking mismatch; MEDIUM for symlink/test-clock guidance. [VERIFIED: crates/yune-rime-api/src/userdb.rs; VERIFIED: /Users/trenton/Projects/librime source; ASSUMED]
- Environment: HIGH for this shell's missing Cargo/Rust; LOW for executor environment availability. [VERIFIED: command -v cargo/rustc; ASSUMED]

**Research date:** 2026-04-30 [VERIFIED: currentDate]
**Valid until:** 2026-05-30 for source-seam findings; re-check environment and librime source before implementation if more than 30 days elapse. [ASSUMED]
