# Codebase Concerns

**Analysis Date:** 2026-06-17

> Orientation: Yune is a Rust reimplementation of the librime engine plus a C ABI
> (the `yune-rime-api` crate, `crate-type = ["rlib", "cdylib"]`) that re-exposes the
> RIME C API. There are **three consumers** of that boundary:
> 1. the **CLI surrogate** (`yune-cli`, the `frontend` subcommand) for scripted replay;
> 2. **TypeDuck-Web** via the `yune_typeduck_*` WASM adapter (`crates/yune-rime-api/src/typeduck_web.rs`)
>    plus the `@yune-ime/typeduck-runtime` TypeScript package (`packages/yune-typeduck-runtime/`);
> 3. **TypeDuck-Windows** via a native `rime.dll` produced by `scripts/package-typeduck-windows.ps1`.
>
> The behavior oracle is upstream `github.com/rime/librime` plus the TypeDuck fork
> `github.com/TypeDuck-HK/librime @ v1.1.2` (NOT a local checkout). **Current direction
> is web-first**: validate M9 TypeDuck-Web in a real browser (Phase 17) before resuming
> the parked M10 TypeDuck-Windows native work. AI-native input is a separate, later layer.

## Top Current Concerns (web-first)

**Browser validation is blocked on an Emscripten toolchain (the single highest-priority risk):**
- Issue: The TypeDuck-Web path cannot be validated end-to-end because no Emscripten
  toolchain is installed, so the WASM artifact has **never been built**. Phase 10 read
  NO-GO purely from absent browser evidence, not a failed seam.
- Files: `crates/yune-rime-api/src/typeduck_web.rs` (the `yune_typeduck_*` C/WASM bridge),
  `packages/yune-typeduck-runtime/` (`@yune-ime/typeduck-runtime` TS runtime),
  `scripts/typeduck-wasm-build.sh` (targets `wasm32-unknown-emscripten`, blocks with an
  install message when the Rust target or `emcc`/`emar` are missing),
  `docs/plans/typeduck-web-validation-plan.md`, `.planning/STATE.md`.
- Impact: The engine has never run in a real browser. Adapter shape assumptions are
  unverified — the runtime expects `candidate.text` / `candidate.comment` /
  `context.highlighted`, NOT non-existent context-level keys
  (see `third_party/typeduck-web/yune-integration/adapter.ts`).
- Resolution path: Install/activate the Emscripten SDK, run `scripts/typeduck-wasm-build.sh`,
  then run the real-browser TypeDuck-Web E2E and record an evidence-based GO/NO-GO.

**Comment-parity oracle coverage is partial:**
- Issue: Comment/Cantonese parity against the `TypeDuck-HK/librime @ v1.1.2` oracle is the
  core compatibility contract for **both** the web and Windows paths, and it has explicit,
  documented gaps. `DictionaryLookupFilter` (`crates/yune-core/src/filter/mod.rs`) emits the
  TypeDuck panel comment (a `\u{000c}` form-feed prefix followed by `\r1,…\r0,…` records)
  and is byte-locked against `crates/yune-core/tests/fixtures/typeduck-v1.1.2/` — but only
  for the captured source rows (e.g. `nei`/`hou`).
- Not yet golden-covered: (a) the normal reverse-lookup `"; "` joiner
  (the join exists in `filter/mod.rs` — `comments.join("; ")` — but no oracle asserts it),
  and (b) schema-name-in-prompt parity.
- Files: `crates/yune-core/src/filter/mod.rs`, `crates/yune-core/tests/cantonese_parity.rs`,
  `crates/yune-core/tests/fixtures/typeduck-v1.1.2/`.
- Impact: Regressions in the `"; "` joiner or schema-name prompt can pass CI because no
  oracle asserts them. A developer changing `dictionary_lookup_filter` or the reverse-lookup
  join must know these behaviors are NOT yet locked by goldens.

**Five Cantonese/Jyutping parity cases are `#[ignore]`d:**
- Issue: `crates/yune-core/tests/cantonese_parity.rs` carries 5 `#[ignore]`d tests
  (lines drift; locate by the `#[ignore = "blocked: …"]` messages), each documenting a
  blocker: (1) options `combine_candidates`/`show_full_code`/`enable_sentence`;
  (2) completion/prediction + `enable_completion`; (3) correction minimal-distance +
  m-abbreviation penalties; (4) schema-menu hiding (`hide-lone-schema`/`hide-caret`);
  (5) per-entry userdb pronunciation.
- Impact: Each is blocked on capturing dedicated v1.1.2 goldens. Until captured, regressions
  in these areas are not asserted by any oracle.

**TypeDuck-Windows native artifact is unverified on an MSVC host (parked):**
- Issue: `scripts/package-typeduck-windows.ps1` builds `yune-rime-api` for
  `x86_64-pc-windows-msvc`, renames the outputs to `rime.dll`/`rime.lib`, copies the
  TypeDuck fork headers, and smoke-checks `rime_get_api` plus the `config_list_append_string`
  slot. This has **not** been run on a real MSVC host, so the artifact is unproven; the
  MSVC target/toolchain may be unavailable in the current workspace.
- Files: `scripts/package-typeduck-windows.ps1`, `crates/yune-rime-api/src/api_table.rs`.
- Note on the ABI: the fork-only `config_list_append_{string,bool,int,double}` slots are
  wired into the API table to match the fork `rime_api.h` field order — field order **is**
  the ABI, so these slots must stay positioned to match the fork header.
- Status: Parked until web validation succeeds. Resume Windows work only after the M9 GO.

## Tech Debt

**Workspace lints are declared but not enabled by member crates:**
- Issue: `Cargo.toml` declares `unsafe_code = "forbid"` and clippy policy
  (`all`/`pedantic = "warn"`) under `[workspace.lints]`, but no member manifest opts in with
  `[lints] workspace = true` (verified: `crates/{yune-core,yune-rime-api,yune-cli,yune-schema}/Cargo.toml`).
  The ABI crate contains extensive required unsafe code, so the intended policy is ambiguous
  rather than enforceable.
- Files: `Cargo.toml`, `crates/yune-rime-api/Cargo.toml`, `crates/yune-core/Cargo.toml`,
  `crates/yune-schema/Cargo.toml`, `crates/yune-cli/Cargo.toml`.
- Impact: New unsafe code and clippy regressions rely on command-line discipline instead of
  manifest-enforced policy.
- Fix approach: Add explicit crate-level lint configuration. For `crates/yune-rime-api`,
  allow unsafe intentionally and keep `unsafe_op_in_unsafe_fn`/FFI lint expectations explicit;
  for safe crates, opt into the workspace lint policy.

**Core tests are still partly concentrated in the public facade:**
- Issue: `crates/yune-core/src/lib.rs` is ~3,357 lines and declares `mod tests;`. Most tests
  have already been extracted into `crates/yune-core/src/tests/{engine.rs,filter.rs,translator.rs}`
  (plus `mod.rs`) and the integration test `crates/yune-core/tests/cantonese_parity.rs`, but
  ~68 `#[test]` functions still remain inline in `lib.rs`. (The earlier "~5,082 lines, mostly
  tests" framing is stale.)
- Files: `crates/yune-core/src/lib.rs`, `crates/yune-core/src/tests/`,
  `crates/yune-core/tests/cantonese_parity.rs`.
- Impact: Test ownership is easier to infer than before, but focused changes still require
  scanning a large facade for the remaining inline tests.
- Fix approach: Continue moving the residual inline tests into `src/tests/` slices without
  behavior changes (see the test-ownership convention under "Conventions" below).

**RIME API facade still owns large cross-module glue:**
- Issue: `crates/yune-rime-api/src/lib.rs` is ~1,904 lines and owns ABI exports, key dispatch,
  schema switch behavior, context menu settings, config path helpers, and shared utility
  functions.
- Files: `crates/yune-rime-api/src/lib.rs`, `crates/yune-rime-api/src/processors/mod.rs`,
  `crates/yune-rime-api/src/schema_install.rs`, `crates/yune-rime-api/src/schema_selection.rs`.
- Impact: Key-processing changes can touch unrelated ABI/config/schema concerns, increasing
  review risk.
- Fix approach: Keep `lib.rs` as the export index and move key dispatch/menu/switch helper
  groups into focused modules as new behavior slices require them.

## Conventions (follow these when editing fragile areas)

**Line endings are normalized to LF:**
- `.gitattributes` sets `* text=auto eol=lf` (with `*.bat`/`*.cmd` kept CRLF and `*.sh` kept LF),
  and `.editorconfig` reinforces it. Byte-exact fixtures and assertions depend on this — the
  TypeDuck panel comment byte tests, `crates/yune-core/tests/fixtures/typeduck-v1.1.2/`, and ABI
  byte-output assertions will break if CRLF leaks in. Do not introduce CRLF into normalized files.

**Test-ownership convention:**
- Core tests live in `crates/yune-core/src/tests/{engine,filter,translator}.rs` plus integration
  tests under `crates/yune-core/tests/`. RIME-API tests live under `crates/yune-rime-api/src/tests/`
  and use **poison-tolerant** locks (`unwrap_or_else(PoisonError::into_inner)` in `tests/mod.rs`) —
  note this poison tolerance is a **test-only** helper, not a production change (production session
  locks still panic; see Scaling Limits). New tests should land beside their behavior slice, not in
  the facade.

## Known Bugs

**No production TODO/FIXME markers detected:**
- Symptoms: The production codebase has no `TODO`, `FIXME`, `HACK`, or `XXX` markers in
  `crates/*/src` outside tests.
- Files: `crates/yune-core/src`, `crates/yune-rime-api/src`, `crates/yune-cli/src`,
  `crates/yune-schema/src`.
- Trigger: Not applicable.
- Workaround: Use `docs/analysis.md`, `docs/roadmap.md`, `.planning/STATE.md`, and this document
  as the active issue inventory. (Planning docs live under `docs/plans/`, with an `archive/`
  subfolder for retired plans.)

**Switcher settings have no API-level destroy path:**
- Symptoms: `RimeSwitcherSettingsInit` (`levers.rs`) allocates a `RimeSwitcherSettings` and inserts
  pointer-keyed entries into global registries, but the levers API exposes no matching
  switcher-settings destroy function. `api_table.rs` wires only `switcher_settings_init`. Tests
  manually `drop(Box::from_raw(settings))`, leaving registry cleanup as caller/test responsibility.
- Files: `crates/yune-rime-api/src/levers.rs`, `crates/yune-rime-api/src/api_table.rs`,
  `crates/yune-rime-api/src/tests/levers.rs`.
- Trigger: Repeated calls to `RimeSwitcherSettingsInit` in a long-lived process.
- Workaround: Reuse a switcher settings object where possible, or restart the process between
  frontend sessions.

**Maintenance thread API is synchronous/no-op shaped:**
- Symptoms: `RimeStartMaintenance` runs maintenance inline, `RimeIsMaintenancing` always returns
  `FALSE` (`deployment.rs`), and `RimeJoinMaintenanceThread` is a no-op (`deployment.rs`).
- Files: `crates/yune-rime-api/src/deployment.rs`, `crates/yune-rime-api/tests/frontend_client.rs`,
  `crates/yune-rime-api/src/tests/deployment.rs`.
- Trigger: A frontend that expects librime-style asynchronous maintenance state.
- Workaround: Treat maintenance calls as synchronous and wait for
  `RimeStartMaintenance`/`RimeDeployWorkspace` to return.

## Security Considerations

**FFI ownership depends on exact API pairing:**
- Risk: `CString::into_raw`, `Box::into_raw`, `Vec::from_raw_parts`, and `std::mem::forget` are
  used for C ABI returns. Calling the wrong free function, freeing twice, or passing foreign
  pointers can cause undefined behavior.
- Files: `crates/yune-rime-api/src/context_api.rs`, `crates/yune-rime-api/src/candidate_api.rs`,
  `crates/yune-rime-api/src/schema_api.rs`, `crates/yune-rime-api/src/config_api.rs`,
  `crates/yune-rime-api/src/ffi_memory.rs`, `crates/yune-rime-api/src/levers.rs`.
- Current mitigation: Entry points check null pointers and versioned `data_size` fields where
  applicable; ownership comments and ABI tests cover many layout/lifecycle cases.
- Recommendations: Keep FFI allocation/free pairs centralized in `ffi_memory.rs`, add debug-only
  allocation provenance assertions for iterator/list/context pointers, and document caller
  ownership in generated C headers when headers exist.

**Process-wide module pointers are caller-owned:**
- Risk: `RimeRegisterModule` stores raw module pointers as `usize` and returns them later; the
  caller must keep module storage alive.
- Files: `crates/yune-rime-api/src/modules.rs`.
- Current mitigation: Safety docs state that caller-owned module storage must remain alive, and
  the built-in levers module is process-owned.
- Recommendations: Keep this API marked unsafe, avoid registering stack-allocated modules in
  tests/examples, and consider storing owned copies for Yune-native modules.

**Schema-provided regex patterns are compiled without resource limits:**
- Risk: Recognizer patterns from deployed schema config are compiled directly. Rust `regex` avoids
  catastrophic backtracking, but untrusted large pattern sets can still consume CPU and memory
  during deployment/session setup.
- Files: `crates/yune-rime-api/src/schema_install.rs`.
- Current mitigation: Invalid regex patterns are skipped.
- Recommendations: Bound pattern length/count for untrusted schemas and report skipped patterns
  through diagnostics.

> Resolved (formerly a High-priority gap): **runtime resource-ID path validation**.
> `crates/yune-rime-api/src/resource_id.rs` now validates resource IDs as logical IDs and rejects
> empty, `.`, `..`, a leading `~`, NUL, `/`, `\`, and Windows drive prefixes before joining roots.
> The `validate_{config,data,schema}_resource_id` / `validate_user_dict_name` helpers are wired into
> `config_api.rs`, `deployment.rs`, `levers.rs`, `lib.rs`, `schema_install.rs`, and `userdb/`, and a
> dedicated test lives at `crates/yune-rime-api/src/tests/resource_id.rs`. Keep new root-joining call
> sites routed through these helpers.

## Performance Bottlenecks

**Dictionary candidate lookup is linear:**
- Problem: `StaticTableTranslator` scans `entries` for every input refresh, and sentence mode scans
  entries for each input position.
- Files: `crates/yune-core/src/translator/mod.rs`, `crates/yune-core/src/engine.rs`.
- Cause: Dictionaries are stored as `Vec<(String, Candidate)>`; `refresh_candidates` collects all
  translator output and sorts the full candidate list on every key event.
- Improvement path: Add prefix indexes/trie or code-range indexes for table translators, produce
  candidates lazily by page, and cache exact/completion lookup results per dictionary generation.

**Schema dictionary loading still falls back to source YAML:**
- Problem: Schema install now **attempts** to load compiled `.table.bin`/`.prism.bin`/`.reverse.bin`
  payloads (`load_schema_compiled_dictionary` in `schema_install.rs`) and only falls back to parsing
  source `.dict.yaml`/import tables/packs/preset vocabulary when compiled data is rejected (the
  `record_dictionary_source_fallback` path). `yune-core` exports the compiled parsers
  (`parse_rime_table_bin_dictionary` / `parse_rime_prism_bin_payload` /
  `parse_rime_reverse_bin_dictionary`) and rebuild-execution types
  (`RimeDictRebuildExecutionReport`). The earlier "metadata/rebuild-plan only" framing is stale.
- Files: `crates/yune-rime-api/src/schema_install.rs`, `crates/yune-core/src/dictionary/source.rs`,
  `crates/yune-core/src/dictionary/compiled.rs`, `crates/yune-core/src/lib.rs`.
- Residual cause: source YAML remains the fallback/primary path, and full distribution-scale
  compiled consumption is incomplete.
- Improvement path: Cache parsed dictionaries by runtime path/checksum and complete compiled payload
  consumption at distribution scale.

**Session and candidate snapshots clone large structures:**
- Problem: Candidate iteration and context retrieval clone candidates or snapshots before converting
  to C-owned memory.
- Files: `crates/yune-rime-api/src/session.rs`, `crates/yune-rime-api/src/context_api.rs`,
  `crates/yune-rime-api/src/candidate_api.rs`, `crates/yune-core/src/engine.rs`.
- Cause: `session_candidates_snapshot` clones the full candidate vector; `Engine::snapshot` clones
  the full context; C APIs then allocate another representation.
- Improvement path: Snapshot only the visible page for `RimeGetContext`, expose iterator state over
  stable candidate IDs, and avoid full-context clone paths for read-only ABI calls.

## Fragile Areas

**C ABI memory and versioned structs:**
- Files: `crates/yune-rime-api/src/abi.rs`, `crates/yune-rime-api/src/context_api.rs`,
  `crates/yune-rime-api/src/ffi_memory.rs`, `crates/yune-rime-api/src/config_api.rs`,
  `crates/yune-rime-api/src/levers.rs`, `crates/yune-rime-api/src/api_table.rs`.
- Why fragile: Struct field availability depends on caller-provided `data_size`; nested pointers
  have different ownership rules per API; many functions expose borrowed process/config-owned
  pointers. The fork-only `config_list_append_*` slots in `api_table.rs` must keep the fork
  `rime_api.h` field order — field order is the ABI.
- Safe modification: Add ABI layout and lifecycle tests before changing structs, fields, allocation
  types, or free functions. Preserve data-size compatibility and pointer lifetime comments.
- Test coverage: Strong focused ABI tests exist; native frontend lifecycle is now **source-modeled**
  rather than absent (see "Native frontend validation" below), but no real OS-daemon/app-bundle run
  is covered.

**Key processing dispatch:**
- Files: `crates/yune-rime-api/src/lib.rs`, `crates/yune-rime-api/src/processors/key_binder.rs`,
  `crates/yune-rime-api/src/processors/speller.rs`, `crates/yune-rime-api/src/processors/editor.rs`,
  `crates/yune-rime-api/src/processors/chord_composer.rs`, `crates/yune-core/src/engine.rs`.
- Why fragile: `RimeProcessKey` validates masks, handles ascii composer switches, key binder
  redirects, selector/navigator overrides, processor chains, shape processing, and engine fallback
  in one flow.
- Safe modification: Add focused tests for each branch before changing dispatch order. Keep key mask
  acceptance, commit buffering, paging, and segment-tag updates observable through ABI tests.
- Test coverage: Broad focused coverage exists, but native frontend modifier timing and release-key
  sequences remain higher-risk.

**Global process state:**
- Files: `crates/yune-rime-api/src/session.rs`, `crates/yune-rime-api/src/runtime.rs`,
  `crates/yune-rime-api/src/modules.rs`, `crates/yune-rime-api/src/notifications.rs`,
  `crates/yune-rime-api/src/levers.rs`, `crates/yune-rime-api/src/api_table.rs`,
  `crates/yune-rime-api/src/typeduck_web.rs`.
- Why fragile: Runtime paths, sessions, module pointers, notifications, state-label cache, API
  tables, and switcher registries are process-wide singletons (e.g. `runtime_paths()` is a
  `OnceLock<Mutex<RuntimePaths>>` in `runtime.rs`).
- **Web-path design constraint (not just fragility):** The TypeDuck-Web handoff contract requires
  **exactly one active process-global RIME service per WASM instance**, with host-owned MEMFS/IDBFS
  layout and explicit host-driven sync. userdb persistence is an explicit host sync boundary because
  the native exports expose no userdb mutation notifications. Missing browser schema/dictionary
  assets are treated as an init-time failure **before** the process-global service starts.
  `yune_typeduck_init` (`typeduck_web.rs`) drives setup/initialize/create_session/select_schema
  against this single global service. Implication: multiple concurrent engines or schemas in one
  instance are out of scope, and the global `runtime_paths()`/session-registry singletons are
  load-bearing for this model.
- Safe modification: Reset globals explicitly in tests, avoid holding locks across callbacks or
  filesystem work, and keep session mutation inside narrow lock scopes.
- Test coverage: Tests use isolation helpers and poison-tolerant locks, but there is little
  multi-threaded concurrency coverage.

**Config path mutation semantics:**
- Files: `crates/yune-rime-api/src/config.rs`, `crates/yune-rime-api/src/config_api.rs`,
  `crates/yune-rime-api/src/config_compiler.rs`.
- Why fragile: Slash paths, list references such as `@next`, `@before`, and `@after`,
  null-to-container conversion, and lexical map iteration all emulate librime behavior.
- Safe modification: Add compatibility fixtures for each path form before editing `set_config_value`,
  `list_index`, or config iterator logic.
- Test coverage: Config API and compiler tests are substantial; resource-ID rejection is now covered
  (see `tests/resource_id.rs`).

**Large compatibility suites:**
- Files: `crates/yune-rime-api/src/tests/schema_processors.rs`,
  `crates/yune-rime-api/src/tests/schema_selection.rs`,
  `crates/yune-rime-api/tests/frontend_client.rs`, `crates/yune-core/src/lib.rs`.
- Why fragile: Several test files are large, making it easy to add near-duplicate fixtures or hide
  ownership boundaries.
- Safe modification: Split only by behavior ownership and keep fixture helpers shared; avoid mixing
  mechanical test moves with behavior changes.
- Test coverage: Coverage is broad, but file size slows review and targeted execution.

## Scaling Limits

**Single global session mutex (production poison panic):**
- Current capacity: One process-wide `Mutex<SessionRegistry>` guards all sessions.
- Limit: Concurrent frontend calls across sessions serialize through one lock, and poisoned locks
  panic — `session.rs` still uses `.expect("session registry should not be poisoned")` in production.
  (Poison tolerance was added to test locks only; see Conventions.)
- Scaling path: Shard session state or store per-session locks after registry lookup; convert poison
  handling at FFI boundaries into failure returns.

**Unbounded commit history:**
- Current capacity: `Engine` appends every commit to `context.commit_history`; `HistoryTranslator`
  reads only the configured tail.
- Limit: Long-lived sessions can accumulate unbounded history memory.
- Scaling path: Keep a bounded ring buffer sized by installed history translator needs.

**In-memory source dictionaries:**
- Current capacity: Table dictionaries are loaded into vectors and scanned for lookup.
- Limit: Distribution-scale RIME dictionaries increase startup memory, per-key CPU, candidate
  sorting, and context clone costs.
- Scaling path: Complete compiled dictionary payload loading, prefix indexes, and lazy candidate
  paging.

**File-backed (not LevelDB) userdb store:**
- Current capacity: `crates/yune-rime-api/src/userdb/` is now a structured multi-module store
  (`mod.rs`, `file_store.rs` `FileUserDbStore`, `record.rs` packing librime-shaped `c=/d=/t=` values
  with a `formula_d` decay, `snapshot.rs`, `recovery.rs`, `sync.rs`, `store.rs`) — not a flat
  append-merge text shim. (The legacy flat `crates/yune-rime-api/src/userdb.rs` still co-exists.)
- Limit: It remains **file-backed, not LevelDB-backed**, so it lacks LevelDB-compatible transactions
  and atomicity, and full-file operations still dominate large dictionaries.
- Scaling path: Add a LevelDB-compatible storage backend with atomic writes and conflict-aware merge
  semantics.

## Dependencies at Risk

**`serde_yaml`:**
- Risk: RIME compatibility is tied to yaml-cpp/libyaml behavior, while config and schema parsing use
  `serde_yaml` plus local compatibility shims.
- Impact: Subtle YAML scalar, null, duplicate, merge, or ordering behavior can differ from librime and
  affect deployed configs/dictionaries.
- Migration plan: Keep compatibility tests for yaml-cpp edge cases, isolate YAML access behind helper
  functions, and consider a parser layer with explicit librime-compatible normalization.

**`regex`:**
- Risk: Recognizer, spelling algebra, and schema pattern behavior depends on Rust regex semantics
  rather than librime's exact matching stack.
- Impact: Valid RIME schemas may compile or match differently, especially around unsupported regex
  constructs.
- Migration plan: Treat each unsupported pattern as a compatibility finding; add diagnostics and
  fixtures before introducing alternate regex engines.

**`libc`:**
- Risk: `libc` is used for Unix `ctime_r` signature formatting to match librime.
- Impact: Non-Unix builds use a different timestamp format, and signature tests must allow platform
  differences.
- Migration plan: Keep the Unix-specific path isolated and gate platform-specific tests.

## Missing Critical Features

**Native frontend validation (source-modeled, no real OS run):**
- Problem: Native frontend behavior is now represented as source-modeled lifecycle fixtures rather
  than absent. `crates/yune-rime-api/tests/frontend_hosts/native_frontends.rs` models Squirrel/macOS
  (`squirrel_macos_source_model`, schema `squirrel_luna`) and, per STATE decisions, ibus-rime /
  fcitx-rime as ABI source-model markers with documented direct-run blockers; `frontend_hosts/`
  also covers `typeduck_web`. The residual gap is the absence of a **real**
  input-method-framework / OS-daemon / app-bundle integration run.
- Files: `crates/yune-rime-api/tests/frontend_hosts/{mod.rs,native.rs,native_frontends.rs,typeduck_web.rs}`,
  `.planning/STATE.md`.
- Blocks: Confidence in struct lifetimes, notification timing, deployment behavior, focus/session
  lifecycle, and real input-method framework integration under live callback timing.

**Compiled dictionary payload consumption at distribution scale:**
- Problem: Compiled `.table.bin`/`.prism.bin`/`.reverse.bin` loading and rebuild execution exist
  (`load_schema_compiled_dictionary`, the exported parsers, and `RimeDictRebuildExecutionReport`), but
  source YAML remains the fallback path and full distribution-scale consumption is incomplete.
- Files: `crates/yune-core/src/dictionary/compiled.rs`, `crates/yune-rime-api/src/schema_install.rs`,
  `crates/yune-core/src/lib.rs`.
- Blocks: Efficient distribution-scale dictionary startup without source fallback.

**Full user dictionary behavior:**
- Problem: The userdb store is structured and file-backed (see Scaling Limits) but is not
  LevelDB-backed; it lacks librime-style transactions, recovery guarantees, predictive lookup, and
  frequency updates at LevelDB parity.
- Files: `crates/yune-rime-api/src/userdb/`.
- Blocks: Librime-compatible personalization and production userdb migration.

**Full OpenCC and plugin compatibility:**
- Problem: `SimplifierFilter` uses small built-in character maps, and plugin compatibility is
  intentionally outside the current compatibility subset (deferred in `.planning/STATE.md`).
- Blocks: Real-world schemas depending on OpenCC data chains, Lua/octagram/predict/proto plugins, or
  C++ plugin ABI.

## Test Coverage Gaps

**Real frontend integration:**
- What's not tested: Live native frontend clients and OS input-method framework behavior (the
  source-modeled fixtures stand in for them).
- Files: `crates/yune-rime-api/tests/frontend_hosts/`,
  `crates/yune-rime-api/tests/frontend_client.rs`, `docs/analysis.md`, `docs/roadmap.md`.
- Risk: ABI behavior can pass synthetic/source-modeled tests while failing under real callback
  timing, dynamic loading, focus changes, or frontend memory expectations.
- Priority: High

**Real-browser TypeDuck-Web E2E:**
- What's not tested: An end-to-end run of the WASM artifact in a real browser, including adapter
  shape (`candidate.text`/`candidate.comment`/`context.highlighted`).
- Files: `crates/yune-rime-api/src/typeduck_web.rs`, `packages/yune-typeduck-runtime/`,
  `scripts/typeduck-wasm-build.sh`, `docs/plans/typeduck-web-validation-plan.md`.
- Risk: This is the gating evidence for the M9 GO/NO-GO; blocked on the Emscripten toolchain.
- Priority: High

**Comment / Cantonese oracle coverage:**
- What's not tested: The `"; "` reverse-lookup joiner, schema-name-in-prompt parity, and the 5
  `#[ignore]`d Cantonese/Jyutping cases (all blocked on capturing dedicated `TypeDuck-HK/librime @ v1.1.2`
  goldens).
- Files: `crates/yune-core/src/filter/mod.rs`, `crates/yune-core/tests/cantonese_parity.rs`,
  `crates/yune-core/tests/fixtures/typeduck-v1.1.2/`.
- Risk: Regressions in these behaviors pass CI because no oracle asserts them.
- Priority: High

**Windows native artifact smoke check:**
- What's not tested: Building `rime.dll`/`rime.lib` via `scripts/package-typeduck-windows.ps1` on a
  real MSVC host and smoke-checking `rime_get_api` + the `config_list_append_string` slot.
- Files: `scripts/package-typeduck-windows.ps1`, `crates/yune-rime-api/src/api_table.rs`.
- Risk: The Windows artifact is unproven; parked until web validation succeeds.
- Priority: Medium (parked)

**Concurrency and lock behavior:**
- What's not tested: Multi-threaded session access, notification callbacks under concurrent state
  changes, poisoned mutex recovery (production session locks still panic), and switcher registry churn.
- Files: `crates/yune-rime-api/src/session.rs`, `crates/yune-rime-api/src/runtime.rs`,
  `crates/yune-rime-api/src/notifications.rs`, `crates/yune-rime-api/src/levers.rs`.
- Risk: Real frontends may call from multiple threads and expose serialization, panic, or stale
  pointer behavior.
- Priority: Medium

**Distribution-scale performance:**
- What's not tested: Large dictionaries, large schema chains, many candidates, long user dictionaries,
  and repeated schema switching under realistic data sizes.
- Files: `crates/yune-core/src/translator/mod.rs`, `crates/yune-core/src/dictionary/source.rs`,
  `crates/yune-rime-api/src/schema_install.rs`, `crates/yune-rime-api/src/userdb/`.
- Risk: Focused fixtures can hide per-key linear scans, parse-time costs, and full-file sync
  bottlenecks.
- Priority: Medium

> Resolved (formerly listed here as open):
> - **RIME API-backed CLI frontend** — `crates/yune-cli/src/rime_frontend.rs` is now a fully
>   implemented (~821-line) RIME-API-backed driver: it imports `rime_get_api` and the `Rime*` ABI
>   structs, drives the full setup/initialize/deploy/select/create-session/process-key/read-state/
>   destroy/finalize lifecycle, maps X keysyms, and produces a `FrontendTranscript`. `main.rs` wires
>   `Command::Frontend` through `rime_frontend::run_frontend` (Json/Human modes), with a
>   `check_frontend_fixture` path.
> - **Resource-ID path validation** and its test-coverage gap — see Security Considerations.

---

*Last reviewed: 2026-06-17 — refreshed for the TypeDuck-Web (M9) and TypeDuck-Windows (M10) work; current direction is web-first.*
