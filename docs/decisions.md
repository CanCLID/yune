# Yune Decision Log

This is the consolidated decision log for Yune. The GSD planning system that previously lived under `.planning/` has been **retired**; this file preserves the durable decisions and their rationale so that future work does not lose the reasoning behind the current architecture.

It harvests decisions from the retired `.planning/PROJECT.md` "Key Decisions" table and Context/Constraints prose, the `.planning/STATE.md` "Accumulated Context > Decisions" list, and the `decisions:` / `D-NN` entries in the phase `PLAN`/`SUMMARY`/`RESEARCH`/`CONTEXT` documents.

A note on IDs: the per-phase `D-NN` identifiers used inside `.planning/phases/*` were **phase-local** (Phase 2, Phase 3, Phase 7, and Phase 10 each restarted at `D-01`). The numbered `D-12`..`D-23` IDs in this log are the **project-wide** IDs from `STATE.md` and are preserved verbatim. Earlier, unnumbered project-wide decisions are assigned `D-01`..`D-11` here in rough chronological order. Phase-local decisions are folded into the relevant project-wide entry or captured as the `D-P<phase>-<n>` entries below; they are not renumbered against the project-wide sequence.

## Standing principles

Cross-cutting decisions that govern all current and future work:

- **librime is the compatibility oracle, not an architecture template.** librime defines the externally observable contract (schema semantics, config behavior, candidate output, C ABI expectations, deployed-data compatibility, frontend integration). The implementation is free to be idiomatic, typed Rust internally as long as the boundary contract is preserved. Preserve librime-_observable behavior_, not librime's internal C++ complexity.

- **Name the protected behavior before mirroring librime.** Before adding a librime-derived mechanism, name the external behavior it preserves (user-visible input, frontend ABI, schema/config semantics, or deployed-data compatibility). If it only mirrors a librime implementation detail, prefer a smaller Yune-native design, isolate it behind an adapter, or document an explicit deferral.

- **AI-native input is a separate, local-first, non-blocking, source-labeled layer.** LLM assistance is layered as optional candidate providers, rankers, context providers, and memory stores with timeout/fallback policy. AI results must be source-labeled, optional, and safe to discard, and they must never block or slow classic input. Classic input stays predictable and low-latency without network access; remote model calls can never be required for baseline input. Context/memory collection must be opt-in, inspectable, clearable, and disabled for sensitive contexts.

- **Build compatibility fixtures and ABI tests before replacing modules.** Behavior must be measurable against librime before any difference can be classified as an improvement or a regression. Each new behavior slice names its owning implementation module, owning test module, and librime comparison target before code changes; new failing comparisons/fixtures come before dispatch or state-mutation changes.

- **Module and test ownership per slice; `lib.rs`/`main.rs` stay facades.** Each compatibility slice owns its implementation module and test module. `lib.rs` (and `main.rs`) remain thin facades plus facade-owned tests; cross-cutting behavior tests live under per-crate `tests/` modules. Mechanical refactors preserve observable behavior and assertions; they do not rewrite working slices.

- **Plugin ABI compatibility is deferred.** Full C++ librime plugin ABI compatibility (Lua, octagram, predict, proto, etc.) is expensive and not yet required by a concrete frontend or schema-migration path. Deferred until a real distribution requirement makes it necessary.

- **Upstream-first oracle sequencing.** Upstream `rime/librime 1.17.0` is the default core compatibility oracle. TypeDuck-HK/librime `v1.1.2` is retained as a TypeDuck-Web/Windows compatibility-profile oracle only. If upstream and TypeDuck behavior disagree, upstream wins for core Yune; TypeDuck behavior must stay behind a named profile test, fixture, adapter, or ABI note.

- **Compatibility is target-driven, not feature-complete.** The oracle is a behavioral _floor_, not a feature checklist. "Done" is always relative to a _named_ set of target schemas and frontends that must behave correctly versus the oracle (near-term: the `luna_pinyin` core path and the TypeDuck `jyut6ping3` profile), broadening over time toward common RIME schemas and real frontends. Reproducing every librime gear, plugin, or code path is an explicit **non-goal**; a librime feature is implemented only when a named target needs it (see _Name the protected behavior_). The AI-native layer on top of this compatible base — not parity for its own sake — is the product goal. See D-25.

## Decision log

### Foundational decisions (from PROJECT.md Key Decisions and Context/Constraints)

**D-01 — Use librime as the compatibility oracle, not an architecture template.** Existing schemas and frontends depend on librime contracts, but Rust can model the internals more cleanly. Prefer typed, idiomatic Rust modules over cloning librime's C++ structure when the boundary contract is preserved. _Outcome: Good._

**D-02 — Build compatibility fixtures and ABI tests before replacing deeper engine modules.** Behavior must be measurable before differences can be classified as improvements or regressions. _Outcome: Good._

**D-03 — Keep AI ranking optional and local-first.** Classic input must remain predictable and low-latency without network access. _Outcome: Pending._

**D-04 — Treat AI-native input as a separate product layer above compatibility.** librime cannot guide LLM-native behavior, so Yune needs explicit provider, context, memory, fallback, and privacy contracts. _Outcome: Pending._

**D-05 — Treat the module/test refactor as a structural rule for future feature work.** Large single-file accumulation slowed review, search, focused testing, and extraction; module/test ownership per slice with `lib.rs`/`main.rs` as facades is now the rule. _Outcome: Pending._

**D-06 — Keep plugin ABI compatibility deferred.** Plugin compatibility is expensive and not yet required by a concrete frontend or schema-migration path. _Outcome: Pending._

**D-07 — Treat runtime resource identifiers as logical IDs, not arbitrary filesystem paths.** Schema-controlled dictionary/import/pack/vocabulary IDs are validated (rejecting drive prefixes, backslashes, traversal) before any runtime data path is constructed; explicit user-supplied import/export/restore file paths remain arbitrary paths, but the derived logical names joined into runtime roots are validated. Boundaries fail closed (FALSE, -1, None, Value::Null).

**D-08 — Source `.dict.yaml` support is not sufficient for production-scale compatibility.** Compiled `.table.bin`, `.prism.bin`, and `.reverse.bin` payload consumption and rebuild execution remain a required direction.

**D-09 — The CLI frontend is a validation surrogate, not native UI.** It drives `yune-rime-api` to validate the ABI; it is not proof that Squirrel, Weasel, ibus-rime, fcitx-rime, or fcitx5-rime integration is complete, and it is not a new graphical end-user frontend.

**D-10 — Cloud inference is never a required runtime dependency.** Classic input and the first AI-native layer must remain local-first and low-latency; remote LLM use is an optional enhancement only.

**D-11 — Privacy boundaries for context and memory.** Context and memory collection must be opt-in, inspectable, clearable, and disabled for sensitive contexts; remote model calls cannot be required for baseline input.

### Phase 1 — CLI frontend surrogate

**D-P1-1 — Keep existing core-backed `run`/`check` commands; add separate ABI-backed `frontend`/`frontend-check` commands.** The CLI gains an ABI-driven path without disturbing the existing core-backed commands.

**D-P1-2 — Localize unsafe RIME ABI handling to `rime_frontend.rs`.** All ABI functions are acquired through `rime_get_api()`/`RimeApi`; each populated commit/context/status read is paired with its matching free function; an RAII cleanup guard runs `destroy_session`/`cleanup_all_sessions`/`finalize` on success and error paths.

**D-P1-3 — Move deterministic transcript serialization to `transcript.rs`.** `FrontendRun::to_json` becomes a compatibility delegation; owned `FrontendEvent`/`FrontendContext` carry keycode, mask, page metadata, and select keys/labels so serializers never read raw ABI pointers. Native frontend validation is marked Phase 2 scope.

### Phase 2 — Native ABI validation and runtime safety

**D-P2-1 — Validate the Cargo-built cdylib through a dynamic loader.** A `libloading` Cargo integration test resolves `rime_get_api` and drives the returned `RimeApi` function table against the real `yune-rime-api` cdylib; `cargo build -p yune-rime-api` is required before the loader gate because `cargo test` alone does not guarantee the artifact exists. The `Library` value is kept alive for the whole symbol/table scope; the loader fails closed with distinct messages for missing artifact, load error, missing symbol, null table, null function pointer, and runtime failure.

**D-P2-2 — Keep runtime implementation unchanged where lifecycle regressions pass.** The absence of loader-exposed concurrency defects is recorded as an explicit lifecycle-safety assertion rather than broadening into multi-threaded frontend behavior. Notification ordering is asserted with full context/session/type/value tuples.

**D-P2-3 — Limit Phase 2 fixes to ABI/runtime boundaries.** Schema semantics, compiled-dictionary behavior, and userdb storage compatibility stay deferred to later phases and are recorded only as structured findings.

### Phase 3 — Schema pipeline depth

**D-P3-1 — Model non-auto previous-match splitting in `processors/speller.rs` without `lib.rs` dispatch changes.** The remaining input is preserved without emitting an unread commit; the splitting stays bounded to one appended spelling-key path.

**D-P3-2 — Lock existing editor/navigator/selector and chord/punctuation/fallback behavior with schema-loaded ABI fixtures.** Coverage is added as ABI-facing regression fixtures where existing owned behavior already matched the focused visible state.

**D-P3-3 — Make unmodeled gears explicit deferrals, not silent no-ops.** `memory` defers to Phase 5 (userdb learning); `poet`/`grammar`, `contextual_translation`, and `unity_table_encoder` defer to Phase 4 (compiled payloads). Each is recorded as a `RemainingGearDeferral` (gear, observed librime role, current Yune behavior, scope decision, target phase) during schema installation — a deterministic no-op, not an ABI-exposed compatibility claim.

**D-P3-4 — Compare against real librime schemas (`luna_pinyin`, `cangjie5`) and record out-of-phase gaps as structured findings.** Distribution comparisons assert focused categories (component order, segment tags, generated spellings, OpenCC/filter behavior, punctuation/fallback, candidate differences); converted differences stay in the comparison module with owner comments rather than changing production code.

**D-P3-5 — Use schema-loaded ABI fixtures for spelling algebra, correction/tolerance ranking, and OpenCC filter-chain behavior.** OpenCC assertions cover filter-chain tag gating and limited built-in maps only; no full OpenCC conversion-data parity is claimed, and no compiled-payload/LevelDB/plugin/AI-native code is added.

### Phase 4 — Compiled dictionary data

**D-P4-1 — Keep compiled binary layout parsing in `yune-core` byte-slice readers; `schema_install.rs` only does validated resource selection.** Valid source-only dictionaries are normal runtime behavior; stale/invalid/unsupported compiled artifacts record fallback diagnostics. Unsupported MARISA/Darts sections fail closed with structured `UnsupportedSection` errors.

**D-P4-2 — Compute deployment freshness from source/schema/pack checksums, not mtimes.** `RimeDictRebuildExecutionReport` statuses for table/prism/reverse artifacts let deployment tests assert partial rebuild/reuse; rebuild execution stays entirely in Rust by emitting the local compiled formats the readers accept; generated schema build metadata is normalized out of prism signatures.

**D-P4-3 (project-wide, dated 04-03) — Represent advanced source and compiled dictionary data on `TableDictionary`, not parallel runtime-specific structs.** Use bounded local Yune fixture markers for advanced compiled-payload parity while rejecting unsupported librime sections structurally; carry compiled reverse `dict_settings` into runtime table dictionaries so `ReverseLookupTranslator` observes source and compiled settings through the same API. LevelDB/userdb learning, predictive updates, plugin translators, and AI-native ranking stay out of scope.

**D-P4-4 (project-wide, dated 04-04) — Represent correction/tolerance data as normalized `TableDictionary` metadata merged through compiled/source paths.** Lookup expansion preserves original input first, then correction canonicalization, then tolerance candidates. Correction/tolerance parser counts are capped before allocation; malformed compiled sections fall back with structured diagnostics (`YUNE-CORR`/`YUNE-TOL` fixture markers with checked offsets, lengths, UTF-8, count caps). Tests use local Rust-generated fixture bytes only; no librime compiler is invoked.

### Phase 5 — UserDB and scaling hardening

**D-P5-1 — Use file snapshots and atomic rename semantics instead of a LevelDB dependency.** Exported `RimeLevers*UserDict*` functions stay in `userdb.rs` with storage behavior behind internal userdb modules; legacy plain-text userdb import compatibility is preserved while committing typed c/d/t records.

**D-P5-2 — Classic learning stays commit-driven.** `Engine` emits typed pending userdb events (`take_pending_userdb_learning`) before clearing composition; session persistence consumes them only through normal commit paths. Core userdb is storage-neutral; `yune-rime-api` bridges core events to the store and validates logical dictionary names before storage selection. Schema-selected userdb loading is keyed by the table/script-translator dictionary, not the schema-id fallback, so unrelated schema tests do not inherit persisted learning. `HistoryTranslator`, `CandidateRanker`, `MockAiRanker`, and AI memory are not substitutes for classic userdb learning; userdb candidates enter before optional rankers.

**D-P5-3 — Test splits follow behavior ownership and stay separate from semantic changes.** Levers user-dictionary iterator/file-operation tests move to `yune-rime-api/src/tests/userdb.rs` (owner: userdb lifecycle); engine/translator/ filter behavior tests move out of `lib.rs`; key/dictionary/facade-specific and already-owned tests stay put until their owning modules are targeted. `yune-core/ src/lib.rs` stays a facade plus facade-owned tests.

### Phase 6 — Real frontend validation and benchmark

**D-P6-1 — Anchor native frontend validation at the cdylib `RimeApi` boundary with sanitized deterministic traces.** `FrontendHostTrace` captures ordered calls, notifications, free-pairs, stale sessions, and mismatches using logical resource IDs and deterministic event names (no paths, timestamps, PIDs, or pointer addresses); missing required function pointers are represented as blocker-capable mismatch records before any unchecked call.

**D-P6-2 — Model TypeDuck-Web as a minimized browser/WASM call sequence through Yune-owned `RimeApi` calls, without vendoring source.** Emscripten worker lifecycle, IDBFS persistence, and unavailable native dynamic loading are classified as `browser_wasm_limit` observations rather than Yune ABI failures; the observation is a sanitized fixture with mismatch classification "match" (no Yune ABI/runtime mismatch found). The 06-01 host-trace schema is reused rather than inventing target-specific formats.

**D-P6-3 (project-wide, dated 06-03) — Represent Squirrel/macOS validation as a source-modeled `RimeApi` lifecycle fixture plus a documented direct-run blocker**, rather than a mandatory app bundle or input-method registration step. Linux ibus-rime/fcitx-rime validation remains follow-up documentation with safe ABI source-model markers in `native_frontends.rs`, not a required daemon dependency for `cargo test`. Native frontend mismatch capture reuses the Phase 6 host-trace schema and sanitized fixture rules.

**D-P6-4 (project-wide, benchmark/readiness notes) — Use a dependency-free Cargo bench target instead of Criterion** to preserve MSRV safety and avoid unnecessary benchmark infrastructure. BENCH-01/BENCH-02 measurements stay at the `rime_get_api` / `RimeApi` function-table boundary rather than direct `yune-core` calls. AI-native readiness is recorded as **GO WITH CONDITIONS**, based on Phase 6 validation and benchmarks, while keeping providers, rankers, context policy, memory policy, and privacy controls out of scope.

### Phase 7 — WASM build and export contract

**D-P7-1 — Treat `wasm32-unknown-emscripten` as the browser build target, not `wasm-bindgen`.** TypeDuck-Web-style integration needs Emscripten C ABI exports and filesystem/runtime hooks. Document `EXPORTED_FUNCTIONS`/`EXPORTED_RUNTIME_METHODS` as the browser build contract without changing `lib.rs` facade wiring.

**D-P7-2 — The browser export surface is the seeded `yune_typeduck_*` adapter API.** init, process-key, select-candidate, delete-candidate, flip-page, deploy, customize, cleanup, response-json, response-handled, free-response. `scripts/typeduck-exports.txt` is the canonical non-prefixed export list (one symbol per line, no `RimeApi` symbols).

**D-P7-3 — Keep the Rust adapter inside `crates/yune-rime-api`; do not create a separate adapter crate** unless planning proves the contract cannot be expressed safely from the existing cdylib.

**D-P7-4 — Add deterministic native + Emscripten export verification that must not mutate the `RimeApi` table.** A POSIX shell script verifies native exports (`nm -g` against the export list, accepting macOS leading underscores) before browser prerequisite detection; missing `wasm32-unknown-emscripten`/`emcc`/`emar` are deterministic, actionable blockers only when `cargo test -p yune-rime-api --test typeduck_web` passes. Native adapter contract tests in `crates/yune-rime-api/tests/typeduck_web.rs` are the authoritative fallback. The browser artifact must be a loadable Emscripten main module (`yune-typeduck.js` + `.wasm`) rather than a bare side-module wasm; the build exports the `yune_typeduck_*` list plus `ccall`, `cwrap`, `UTF8ToString`, `FS`, and `IDBFS`, then smokes the module by calling one export and performing one filesystem write/read.

**D-P7-5 — Document the one-active-process-global-service constraint and host filesystem assumptions.** `yune_typeduck_cleanup` finalizes the process-global RIME service; multiple simultaneous TypeDuck states with different dirs are unsupported by this first contract. Treat missing browser schema/dictionary assets as an init-time failure before starting the service. Phase 7 is a handoff contract: one active process-global service, host-owned MEMFS/IDBFS layout and sync, and deterministic verified-or-blocked build output. Upstream clone/replace testing is Phase 10.

### Phase 8 — TypeScript bridge and runtime package

**D-P8-1 — Keep TypeScript tooling package-local under `packages/yune-typeduck-runtime`.** No root JS app scaffolding. Bind only the canonical 11 `yune_typeduck_*` exports through an injected Emscripten Module interface.

**D-P8-2 — Centralize response-pointer ownership.** `readTypeDuckResponse` frees non-null responses in a `finally` block; `TypeDuckRuntime` keeps `statePtr` private, zeros it on cleanup, and rejects operations after cleanup; keyboard mapping is DOM-free (`keyEventToRimeKey` maps `event.key` to explicit RIME constants, bans `keyCode`).

**D-P8-3 — Make native handled state authoritative and normalize malformed JSON.** `yune_typeduck_response_handled` can override the JSON envelope; malformed JSON becomes a deterministic `TypeDuckResponseError`.

**D-P8-4 — Document `@yune-ime/typeduck-runtime` as repository-owned bridge code, not a TypeDuck-Web app scaffold.** Keep the low-level C/WASM export contract alongside wrapper guidance so non-wrapper hosts can follow raw pointer ownership rules; wrapper callers receive parsed `TypeDuckResponse` objects while raw callers copy JSON before `yune_typeduck_free_response`.

### Phase 9 — Browser filesystem and persistence

**D-P9-1 — Keep filesystem behind a narrow `TypeDuckFilesystem` interface and require explicit assets.** Require explicit `dictionaryId` and asset contents rather than parsing YAML or fabricating fallback assets; mirror all five native preflight paths before `TypeDuckRuntime.init` (shared default/schema/dict plus build default/schema). Browser helper code stays DOM-free, network-free, package-local under Vitest fake-fs tests; logical IDs must match `[A-Za-z0-9_-]+` before virtual-path construction.

**D-P9-2 — Keep persistence orchestration as standalone helpers in `filesystem.ts`.** Do not modify `TypeDuckRuntime` or native exports. Use `syncFromPersistenceBeforeInit` before init and `syncToPersistenceAfterMutation` after deploy/customize/cleanup/userdb boundaries; expose sync direction as `fromPersistence`/`toPersistence` strings that lock the Emscripten `syncfs` populate boolean.

**D-P9-3 — Userdb persistence is an explicit host sync boundary (`syncAfterUserDataChange`).** Current native exports do not expose userdb mutation notifications, so the host owns the sync boundary. Stale deployed-config recovery is a deterministic test fixture over existing helpers, not a metadata heuristic; recovery documentation stays local-first and caller-owned (explicit assets, explicit sync boundaries, no browser app/network/cache policy).

### Phase 10 — TypeDuck-Web app integration and E2E

**D-P10-1 — Clone upstream TypeDuck-Web to `apps/yune-web/source` (gitignored), treated as the app under test.** Use a reproducible, auditable checkout (pinned `https://github.com/TypeDuck-HK/TypeDuck-Web` URL + full commit SHA + setup command in a lock JSON and README). Inspect and document the existing librime/WASM seam before any patching, so app-source changes stay distinguishable from Yune adapter changes.

**D-P10-2 — Keep TypeDuck-Web source changes minimal; route engine calls through the Phase 8/9 package surface.** Prefer a patch/configuration layer over UI rewrites; the seam calls `TypeDuckRuntime`, `keyEventToRimeKey`, filesystem preparation, and persistence sync helpers from `@yune-ime/typeduck-runtime` rather than raw `yune_typeduck_*` exports. Preserve one active runtime per Emscripten Module with deterministic cleanup; do not promise multi-instance browser isolation. The adapter's upstream `RimeResult` mapper must consume the runtime's actual shape: per-candidate `text`/`comment` plus `context.highlighted`.

**D-P10-3 — Use explicit TypeDuck-Web-owned assets; never fabricate fallback schema/dictionary data.** Missing or mismatched `default.yaml`/schema/dictionary YAML remain visible integration failures (grep-gated against fallback/dummy/placeholder wording). The app seam loads TypeDuck-Web `public/schema` assets by URL, rejects empty or missing content before init, and may preload extra shared support files such as OpenCC data without synthesizing substitutes.

**D-P10-4 — Record adapter mismatches before widening Yune.** The web mapper now uses the runtime-owned response shape (`candidate.text`, `candidate.comment`, `context.highlighted`) instead of compatibility guesses. Keep documenting the `setOption` gap as an error rather than implementing a workaround; widen the Yune adapter only for the smallest proven blocker, documented first.

**D-P10-5 — Real browser validation is required and never silently skipped.** Use a standalone Playwright spec (upstream has no browser test framework) covering composition, candidate paging, selection, deletion, commit output, deploy, customize, and persistence smoke; persistence follows the Phase 9 explicit sync contract (populate before init, flush after mutation, reload/reinitialize to prove survival). The patched app seam mounts IDBFS before init and leaves explicit before/after sync markers for the E2E evidence. Missing browser/local tooling is recorded reproducibly (command, missing dependency, fallback evidence). Asset configuration was recorded as an E2E blocker, not a build blocker.

**D-P10-6 — WI-4 browser evidence replaces the old tooling blocker with concrete behavioral failures.** The TypeDuck-Web app now loads the Yune Emscripten JS/WASM module in a real browser. Composition, candidate rendering, selection, commit output, backspace mutation, and customize pass. Candidate paging, candidate deletion, persistence sync/reload proof, and v1.1.2 dictionary-comment rendering fail and must be tracked as behavior/runtime gaps, not as missing-tooling blockers. HR-2 later resolved `setOption`; HR-3 later resolved deploy false; HR-4 later resolved persistence sync/reload proof.

**D-P10-7 — M9 remains NO-GO for AI-native frontend exposure after WI-5.** The recommendation is now evidence-based rather than tooling-based: Yune can load and type through TypeDuck-Web, but real frontends must not be treated as ready until paging/deletion and v1.1.2 dictionary-comment bytes pass in-browser. Superseded by D-P10-13 after HR-5/HR-6 closed those evidence gaps.

**D-P10-8 — Close M9 as a validation milestone, not a pass.** M9 is complete because the browser E2E ran and produced a durable GO/NO-GO recommendation. The follow-up milestone is Post-M9 TypeDuck-Web hardening; it starts from the WI-4 failure matrix rather than reopening the validation gate itself.

**D-P10-9 — Reopen the TypeDuck-Web E2E gate when evidence used the placeholder schema.** Post-review inspection found the WI-4 browser matrix exercised Yune's echo placeholder rather than TypeDuck's real `jyut6ping3_mobile` dictionary, so candidate paging, deletion, and dictionary-panel comments were not actually validated. HR-1 fixes the real-asset gate: TypeDuck-Web now loads `jyut6ping3_mobile`, exports context despite the schema's NUL `alternative_select_keys` sentinel, and renders real `nei` candidates (`你`, `呢`, `尼`) in-browser. The recommendation remains NO-GO, and TYPEDUCK-E2E-03/E2E-04 stay open until the full matrix is re-run against real assets. This supersedes D-P10-8's "do not reopen the validation gate" stance — the placeholder-evidence finding justified reopening it. Superseded by D-P10-13 after the full real-assets matrix passed.

**D-P10-10 — Treat startup setOption as a real TypeDuck-Web contract, not an optional convenience.** The upstream app calls `Actions.setOption` during startup and settings application. HR-2 adds `yune_typeduck_set_option`, exposes it through the TypeScript runtime, wires the TypeDuck-Web adapter to it, and records a browser startup smoke where option toggles succeed without adapter/runtime errors. Persistence was later proven by HR-4; paging/deletion and dictionary-comment evidence remain outside this decision.

**D-P10-11 — Browser deploy must preload the workspace-reachable schema set, not only the active schema.** HR-3 showed TypeDuck-Web `deploy()` returned false with real assets because the worker preloaded `jyut6ping3_mobile` and its dictionary but omitted the plain `jyut6ping3.schema.yaml` reached through the real `default.custom.yaml` workspace deployment path. The seam now treats that source schema as an app-owned preload asset and records browser evidence where deploy returns true.

**D-P10-12 — Browser persistence claims require live-worker diagnostics and a real reload.** HR-4 adds reason-tagged persistence diagnostics around `syncFromPersistenceBeforeInit`, runtime init, and `syncToPersistenceAfterMutation` for init, customize, deploy, commit, select, and delete paths. The TypeDuck-Web worker forwards those diagnostics to the main page under debug mode so browser evidence can prove ordering without relying on worker-console capture. The committed `persistence-sync.log` shows a fresh-origin load with no custom config, startup customize writing `page_size: '6'`, deploy syncing after mutation, and a real reload restoring `/rime/jyut6ping3_mobile.custom.yaml` before runtime init. Paging/deletion and dictionary-comment parity remain outside this decision.

**D-P10-13 — HR-7 closes M9 as GO WITH CONDITIONS.** HR-5 reran the browser matrix against real TypeDuck `jyut6ping3_mobile` assets and proved composition, candidate list, paging, selection, deletion, Space commit, phrase commit, deploy, customize, persistence sync, reload survival, and dictionary-panel comments with zero warning/error console entries. HR-6 locked the shared reverse-lookup `"; "` joiner and schema-prompt bytes against TypeDuck-HK/librime v1.1.2. AI-native behavior may proceed only behind the separate M11 gating policy: disabled by default in real frontends until provider/ranking/privacy contracts are proven and explicitly enabled.

**D-P10-14 — Post-review M9 closeout conditions are evidence rules, not a new verdict.** The delete/backspace runtime-error banner was a real delete-path adapter bug: TypeDuck-Web sends a standalone `{Control_L}` keydown before `{Control+Delete}`, and the adapter was forwarding the pure modifier into Yune. The adapter now treats pure modifier keydowns as pass-through and the HR-5 delete/backspace evidence was recaptured with no banner and zero warning/error entries. Rich dictionary-comment byte parity is clean-checkout reproducible via the committed `cantonese_parity` fixture; the browser-shaped native `typeduck_adapter_real_assets_emit_oracle_dictionary_panel_comments` test asserts the full real-assets path only when local TypeDuck v1.1.2 oracle build assets exist and otherwise emits an explicit skip reason.

**D-12 / TYPEDUCK-E2E-04 — Final findings separate three blocker classes.** TypeDuck-Web app/source blockers, Yune adapter/runtime mismatches, and environment/tooling blockers are reported separately.

**D-13 / TYPEDUCK-E2E-04 — Phase 10 ends with a NO-GO recommendation for AI-native frontend exposure** due to browser-validation blockers. Strict rubric: lack of browser evidence prevents GO / GO WITH CONDITIONS. Blockers are bounded (cargo/rustup/emcc have install paths), not a fundamental seam incompatibility; the seam patch is structurally sound and the adapter handles mismatches — environment setup is the gating requirement. Superseded by D-P10-13 after HR-5/HR-6 produced real-assets browser and oracle evidence.

**D-14 — AI-native scope remains deferred.** AI-native provider calls, candidate generation, ranking, context, memory, privacy controls, and a new first-party Yune frontend remain out of scope.

### TypeDuck-Windows milestone (project-wide D-15..D-22; completed as profile by M10)

**D-15 / WIN-TEST-01 - TypeDuck-Windows native IME was the next tracked milestone at the time; first unblock Windows test trust before feature work.** D-24 scoped this milestone behind an explicit TypeDuck profile surface while preserving the completed package and helper evidence as TypeDuck-profile work. M19 added the named profile ABI, and M10 later completed the package/build/ frontend smoke through that profile.

**D-16 / WIN-ABI-01 — Fork-only config list-append APIs are the first feature slice** after the Windows baseline, because they need no external oracle.

**D-17 / WIN-ORACLE-01 — Comment semantics and Cantonese/Jyutping parity must be driven by TypeDuck-HK/librime v1.1.2 goldens or documented blockers.**

**D-18 / WIN-ABI-01 — TypeDuck fork list-append fields are inserted after `config_list_size` and before `get_input`,** matching the fork `RimeApi` order; `config_begin_list` stays in its earlier v1.1.2 slot next to `config_begin_map`. Scalar append values follow the existing string-backed `RimeConfigSet*` representation.

**D-19 / WIN-ORACLE-01 — Pin the v1.1.2 oracle commits.** `TypeDuck-HK/librime` commit `74cb52b78fb2411137a7643f6c8bc6517acfde69`, `rime-dictionary-lookup-filter` commit `3e4605c4fae99f068df2edb85aaeab5a97752795`, and `TypeDuck-HK/schema` commit `1bed1ae6a0ab48055f073774d7dfd152a171c548`.

**D-20 / WIN-COMMENT-01 — Represent TypeDuck-Windows candidate comments as source-row dictionary lookup payloads (`\f\r1,...\r0,...`) through `dictionary_lookup_filter`.** Captured source rows assert byte output against the v1.1.2 fixture. Normal reverse lookup joins use `"; "`, and HR-6 adds dedicated v1.1.2 oracle coverage for that join plus schema-prompt preedit bytes.

**D-21 / WIN-BUILD-01 — The native Windows package is produced by `scripts/package-typeduck-windows.ps1`.** It builds `yune-rime-api` for `x86_64-pc-windows-msvc`, renames the DLL/import library to `rime.dll`/`rime.lib`, copies TypeDuck fork headers, and smoke-checks `rime_get_api` plus the `config_list_append_string` slot. The 2026-06-19 run completed without `-NoBuild` or `-SkipSmoke`, proving a pre-M12 package smoke but not the full TypeDuck-Windows frontend lifecycle. After M12, the default `RimeApi` no longer exposes TypeDuck fork-only slots, so this script is parked and fails fast until a named TypeDuck profile ABI surface exists and the slot smoke is re-derived.

**D-22 / WIN-PARITY-01 — The Cantonese/Jyutping parity suite locks captured v1.1.2 schema/menu/comment behavior** and keeps uncaptured option, completion, correction, schema-menu, and userdb-pronunciation behaviors as explicit ignored tests until dedicated oracle fixtures are captured.

2026-06-21 update: M10 resume supersedes the old D-21 script-fails-fast state. `scripts/package-typeduck-windows.ps1` now builds the current Windows package with upstream-shaped default headers plus `rime_typeduck_profile_api.h`, rejects fork-shaped default headers, ships upstream-deprecated direct-call declarations for existing TypeDuck-Windows source compatibility, and smoke-checks the packaged DLL through `rime_get_typeduck_profile_api()`. M10 T1/T2/T3 are complete: Visual Studio 2022 Community plus ATL/MFC builds the pinned x64 TypeDuck-Windows solution/deployer/server projects against the Yune package; the packaged DLL lifecycle gate passes; stock `TypeDuckServer.exe` plus stock `TestTypeDuckIPC.exe /console` returns a nonzero session, sends `ngohaig` key events, and receives `status.schema_id=jyut6ping3` plus candidate/context data. Evidence: `target/typeduck-windows-e2e/evidence/m10-t3-20260621-100337-stock-real-server`.

2026-06-21 update: M14-M21 promoted the captured TypeDuck engine fixtures to active `cantonese_parity` tests, including options, completion, correction, prediction, and sentence/profile tuning. Schema-menu and userdb-pronunciation observations remain native/frontend T3 evidence limits, not default ABI changes.

### Web-first re-sequencing (project-wide D-23)

**D-23 / SEQUENCING — Re-sequence to web-first.** Validate Yune in a real web browser (reopened as Phase 17) before resuming TypeDuck-Windows platform work. Phase 10's NO-GO reflected absent browser evidence (the WASM artifact was never built), not a failed seam. HR-7 closes that browser gate as GO WITH CONDITIONS. Shared engine slices (comment shaping, Cantonese goldens, baseline fix) were reused by the web path. D-24 supersedes the post-M9 priority order by parking TypeDuck-Windows behind the upstream oracle refresh.

### Upstream oracle refresh (project-wide D-24)

**D-24 / ORACLE-PRECEDENCE - Upstream latest stable is Yune's default core oracle; TypeDuck v1.1.2 is a compatibility profile.** M12 targets upstream `rime/librime 1.17.0` at commit `33e78140250125871856cdc5b42ddc6a5fcd3cd4`. TypeDuck-HK/librime `v1.1.2` at `74cb52b78fb2411137a7643f6c8bc6517acfde69` remains the oracle only for TypeDuck-Web/Windows profile behavior such as fork-only ABI slots and dictionary-panel/Cantonese fork behavior. When upstream and TypeDuck disagree, core Yune follows upstream; TypeDuck differences must be isolated behind named profile fixtures, tests, adapters, or ABI notes. M12 pinned upstream fixture provenance, added the TypeDuck coverage audit, refreshed default `RimeApi` parity to the pinned `rime/librime 1.17.0` target, and expanded `luna_pinyin` behavioral parity fixtures captured from the official upstream release binary. Active upstream-core gates now cover curated single-code mechanics, full `ni` dictionary selection with essay weights, Engine paging/selection/commit, reverse lookup, punctuation/symbols, and supported option paths. Later M17/M18 milestones closed the `luna_pinyin` sentence/lattice, `ascii_punct` processor bypass, and punctuation immediate commit blockers with fresh upstream fixtures. `start_quick` remains absent from the default table. M12 removed the `config_list_append_*` fork-only slots from the default upstream table. M19 implemented the named TypeDuck profile ABI surface, and M10 later completed the TypeDuck-Windows package/build/frontend smoke through it. The default table remains upstream-shaped.

### Compatibility scope (project-wide D-25)

**D-25 / COMPAT-SCOPE — Compatibility is target-driven, not 100% feature parity with librime.** Yune treats librime as a behavioral oracle (a floor), not a feature target to clone. Success is that a _named_ set of target schemas and frontends behaves correctly against the oracle, every difference measured and either fixed or documented: **(A)** a bounded near-term target set — the `luna_pinyin` core path vs upstream `1.17.0` and the TypeDuck `jyut6ping3` profile vs `v1.1.2` — as the working definition of "done"; and **(B)** a broader common-RIME-schema and real-frontend compatibility ambition, added incrementally through the same oracle-measured parity harness. Bit-for-bit parity with librime internals (every gear, plugin, and code path) is a non-goal; a librime feature is implemented only when a named (A)/(B) target requires it. "The engine is done" is therefore never absolute — it reads "the current target set is green against the oracle; everything else is deferred-and-documented." Extends D-01 (oracle, not template) and D-24 (oracle precedence) with an explicit scope boundary; mirrored in `roadmap.md` ("Scope Ledger"). _Outcome: Good._

### AI-native milestone (M11)

**D-M11-1 — Start AI-native with a CLI-only staged-result provider slice.** M11 S1 adds an `AiCandidateProvider` interface and deterministic `MockAiProvider` in `yune-core`, but provider execution is owned by the direct `yune-cli run` path, not by `Engine::refresh_candidates` and not by `yune-rime-api`/browser/Windows frontends. The engine stores only staged, input-keyed `AiResult::Ready` candidates and appends matching AI rows after classic candidates, preserving the top classic candidate. S1 kept `CandidateSource::Ai` as a unit variant with provider/confidence labels in `comment`; D-M11-2 supersedes that shape once the merge policy consumes confidence. Default Space/Return confirmation rejects AI candidates, while explicit AI selection records a commit without staging librime userdb learning. Real frontend AI exposure remains disabled by default until later M11 provider, local-model, context, memory, and privacy gates are proven.

**D-M11-2 — S2 uses a host-owned worker and fixed-point AI confidence metadata.** Provider/model work remains outside `Engine::refresh_candidates`: `AiWorker` runs the provider on a background thread, receives cloned context snapshots from the direct CLI host, and returns input-keyed `AiResult::{Pending, Ready}` so stale results cannot erase or apply to a different input. `CandidateSource::Ai` is promoted to `{ provider, confidence }`, but confidence is stored as fixed point `AiConfidence` rather than `f32` so `CandidateSource` keeps `Eq` and can continue flowing through `UserDbCommitMetadata` and tests. The merge policy still pins all classic rows ahead of AI rows, but now orders AI rows by confidence. This completes the worker/confidence part of AI-02 while leaving local-model, context, memory, privacy, and real-frontend exposure for later gates.

**D-M11-3 — S3 defaults AI context to sensitive and blocks remote providers before invocation.** `Context` now carries `AiContext` metadata (app id, field id, preceding text, and `PrivacyClass`), defaulting to `Sensitive` when the host has not supplied classification. `EngineAiContextProvider` captures a bounded `AiContextSnapshot` with the explicit app/field/text/privacy plus input cursor, schema, and candidate-count data a provider may see. Providers declare `AiProviderKind::{Mock, Local, Remote}`; `AiPrivacyPolicy` allows mock/local providers under sensitive context but blocks remote providers before `provide()` is called and returns an input-keyed `AiResult::Off { reason: Privacy }`. The policy also exposes the learning gate that S4 must apply to `MemoryStore` writes. Real frontend AI exposure remains disabled by default.

**D-M11-4 — S4 routes AI learning into a separate memory store, never librime userdb.** `crates/yune-core/src/ai/memory.rs` owns `MemoryStore`, an inspectable, clearable, disable-able ledger for explicit AI selections. `Engine::commit_candidate` continues to keep `pending_userdb_learning` empty for `CandidateSource::Ai`, but now records standard-context AI commits into `MemoryStore`; sensitive contexts and disabled memory keep the commit path working while suppressing memory writes. The store exports/imports a stable text snapshot for future hosts and exposes `memory_store_file_name` / `memory_store_snapshot_file_name`, which produce `.ai-memory` / `.ai-memory.txt` resources and reject path-like or `*.userdb` logical ids. M9/M10 frontends still do not expose AI.

**D-M11-5 — S5 proves local AI through the direct CLI only.** `LocalModelProvider` is a deterministic, local `AiCandidateProvider` with rule-backed/contextual completions plus optional `MemoryStore` suggestions. It declares `AiProviderKind::Local`, runs through the existing `AiWorker`, returns input-keyed candidates with fixed-point confidence, and obeys deterministic zero-budget fallback. The direct CLI accepts `--ai-provider local` and records the same `ai_decision` field used by the mock provider; the ABI-backed `frontend` command, TypeDuck-Web, and Windows surfaces remain AI-free.

### AI-native frontend exposure (M13)

**D-26 / M13-WEB-AI — Expose AI in TypeDuck-Web through a default-off, two-pass Rust sidecar, not through the classic key path.** M13 keeps `yune_typeduck_process_key` provider-free and classic-first. The browser worker renders that classic result first, then requests the new `yune_typeduck_stage_ai` sidecar action; the sidecar runs `LocalModelProvider` synchronously in Rust/WASM, stages an input-keyed result, and returns a second response. The JS side carries no provider logic. Source labels are attached from engine snapshot data aligned to the rendered page, not by widening `RimeCandidate` or altering the upstream `RimeApi` table. AI remains default-off, TypeDuck-Web-only, local-only, and toggle-controlled; disabling AI stages an `Off` result for the current input so stale AI rows disappear immediately and classic output is restored. The browser host supplies no app/field context, so the existing sensitive default applies: local providers are allowed, remote providers remain out of scope/blocked by policy, explicit AI commits do not touch librime userdb, and AI-memory learning is suppressed in the default browser context. Real-browser M13 evidence is required alongside native `typeduck_web` and TypeScript runtime gates. _Outcome: Good._

### TypeDuck-Web fork parity (project-wide D-27)

**D-27 / TYPEDUCK-PARITY — Complete the TypeDuck `jyut6ping3` target (M14–M16) as the next arc; it needs golden capture + dictionary-driven features, not the upstream language model.** Reaching "open TypeDuck-Web and get fork-like Cantonese behavior" is scheduled as **M14** (complete: captured the v1.1.2 Cantonese goldens by parameterizing the scenario-capable upstream probe's oracle identity and adding a thin v1.1.2 wrapper), **M15** (complete: implemented the dictionary-driven behaviors: `combine_candidates`, `show_full_code`, `enable_sentence`, completion, correction, checked-in `hk2s` OpenCC source-data path), and **M16** (complete: real TypeDuck-Web Playwright evidence covers the app-exposed `jyut6ping3_mobile` surface plus M13 AI, with deploy-only/UI/userdb inspection limits explicitly listed). **Gap-map finding:** `jyut6ping3` uses `script_translator` + `dictionary_lookup_filter` + `simplifier`, not `poet`/`octagram`/grammar — so the upstream statistical language model is **Track 2 / M17**, not on the TypeDuck-parity critical path. Per-entry userdb pronunciations are not exposed by the standard RIME C ABI, but M14 proved the levers user-dict export hook can capture learned per-entry `(code, freq)` rows from seeded state. Track 2 (M17 upstream LM, M18 deployment/processor depth, M19 breadth) resumes opportunistically after the TypeDuck-Web closeout. Extends D-24 (oracle precedence) and D-25 (target-driven scope). _Outcome: M14-M16 complete with explicit browser/userdb inspection limits._

### Architecture hardening (project-wide D-28)

**D-28 / ARCH-HARDENING — Keep the strategic architecture, but name the current core/ABI boundary debt before product/native expansion.** The strategic decisions remain correct: librime is the oracle rather than the template, upstream 1.17.0 is the default core reference, TypeDuck v1.1.2 is a named compatibility profile, and AI stays a separate default-off layer. The structural debt is narrower: the full production key path is currently easiest to drive through the librime-shaped ABI because the RIME processor pipeline lives in `yune-rime-api/src/processors/` and falls through to `yune-core::Engine`. M23 closed the finishable debt around that boundary without doing the larger processor extraction: TypeDuck-calibrated translator tuning is profile/config gated, the workspace lint policy is real for unsafe-free crates with explicit FFI/ABI-client lint exceptions, the orphaned `yune-schema` crate was removed instead of parked, and the oversized test modules were split along behavior ownership lines.

The standing rule remains: TypeDuck-calibrated constants must not affect default upstream core behavior unless installed through an explicit profile predicate / typed translator config, or renamed with upstream-oracle evidence that the behavior is global. M18 later shipped the narrow punctuation processor behavior needed by upstream fixtures in both `yune-core` and the ABI path, but that does not change the larger rule: the full **core/ABI processor extraction stays trigger-gated**. Move processor semantics toward a core-owned Rust API only when a real non-ABI consumer needs the full input pipeline. Every such extraction must be behavior-preserving and keep existing oracle, ABI, and browser gates unchanged. _Outcome: M23 finished the bounded architecture-hardening paydown; the completed plan lives at [`plans/completed/m23-plan-architecture-hardening.md`](./plans/completed/m23-plan-architecture-hardening.md). The processor extraction remains trigger-gated future work rather than a retroactive rewrite trigger._

### Breadth schemas and TypeDuck profile ABI (project-wide D-29)

**D-29 / BREADTH-PROFILE-SURFACE - Add upstream schema breadth only through per-schema oracle fixtures, and expose TypeDuck fork-only ABI through a named non-default accessor.** M19 widens the upstream compatibility set to `double_pinyin`, `cangjie5`, and `bopomofo` by reusing the M12 capture harness against upstream `rime/librime 1.17.0`, with one provenance-stamped fixture and one owning parity test per schema family. The captured active cases cover Shuangpin algebra, Cangjie exact-code table behavior, and Zhuyin keymap/tone algebra; sentence/lattice, broader Cangjie phrase/table-encoder interleave, and schema-speller digit/space key routing remain explicit ignored blockers rather than hidden parity claims.

M19 also names the TypeDuck-Windows ABI delta through `rime_get_typeduck_profile_api()`. The default `rime_get_api()` table remains the upstream `1.17.0` shape; the opt-in profile table exposes `config_list_append_{bool,int,double,string}` for TypeDuck-profile consumers. `start_quick` remains out of surface. TypeDuck-Windows packaging and real frontend E2E were later completed by M10 through this profile surface, without widening the default ABI. Extends D-24 (oracle precedence) and D-25 (target-driven scope). _Outcome: Good._

### Upstream sentence / language model poet (project-wide D-30)

**D-30 / UPSTREAM-POET - Implement `luna_pinyin` sentence/lattice parity through the upstream null-grammar path, not through TypeDuck tuning.** M17 captures `luna-pinyin-sentence.json` and `luna-pinyin-lattice.json` from the pinned upstream `rime/librime 1.17.0` release binary before implementation. The captured schema has `essay.txt` preset vocabulary but no `.gram` model, so Yune implements the grammar-absent poet scoring path in `yune-core`: `entry_weight + kPenalty` per word, with the verbatim upstream `kPenalty = -13.815510557964274`, plus a bounded sentence/lattice candidate path installed only for the upstream `luna_pinyin` script-translator profile. The existing TypeDuck `jyut6ping3` sentence heuristic and its `21.0` word penalty remain profile-gated and separate.

This closes the two M17-owned `luna_pinyin` blockers in `upstream_luna_pinyin_parity.rs` without changing `RimeApi`, `RimeCandidate`, or the TypeDuck-profile ABI surface. Learned `.gram`/octagram models, the C++ plugin ABI, and broader contextual translation stay deferred until a future named target proves they are required. Extends D-24 (oracle precedence), D-25 (target-driven scope), and D-28 (profile-specific tuning isolation). _Outcome: Good._

### Upstream-style Jyutping composition target (project-wide D-31)

**D-31 / JYUTPING-UPSTREAM-COMPOSITION - For long Jyutping composition in the TypeDuck-Web/Yune product surface, prefer an accepted upstream-librime-engine Jyutping fixture over TypeDuck v1.1.2 ranking when TypeDuck v1.1.2 produces worse segmentation.** Upstream `rime/librime 1.17.0` remains the engine oracle, but because upstream does not ship a built-in Jyutping schema, fixtures for this slice run the upstream engine against pinned Jyutping schema/dictionary source YAML and upstream's deployer. The intended ordering is sentence/lattice candidate first, longest valid fuzzy phrase-prefix candidates next, single-character fallback after that, and invalid fuzzy mis-segmentation ranked low. The hybrid upstream-engine fixture is the oracle of record for this named composition/ranking case once captured and accepted, even if it diverges from `typeduck.hk/web`, `my-rime.vercel.app`, or TypeDuck v1.1.2. TypeDuck v1.1.2 remains the compatibility-profile oracle for fork-only ABI/comment/profile behavior and existing M14-M28 fixtures; this hybrid fixture is not a dictionary-comment oracle because stock upstream lacks TypeDuck's `dictionary_lookup_filter` plugin. If the hybrid upstream-engine fixture cannot be captured or accepted, this milestone stops and requires explicit user sign-off for a Yune-authored ranking spec before ranking code changes.

### M33 native lookup fairness and cache boundary (project-wide D-32)

**D-32 / M33-NATIVE-PERF - Accept build-once dictionary translator sharing and lazy reverse lookup as the bounded native fairness slice; defer lazy candidate-pipeline and table+prism/mmap work until a future byte-parity design exists.** M33 fixed the unfair `luna_pinyin` startup/session comparison by deferring `stroke` reverse-lookup dictionary loading to first reverse use, matching librime for no-reverse timed rows. It also allows process-wide sharing of immutable built dictionary translators keyed by schema and resolved source/compiled asset signatures, while keeping per-session mutable state outside the cache. This does not widen `RimeApi`, change `RimeCandidate`, or alter TypeDuck-profile behavior. The lazy lookup spike is a no-go for M33 because a prism-only walk cannot produce byte-identical candidate text/comment/order; as in librime, the prism is an index into table payloads. Current byte-identical output still depends on table-backed translator state. A later milestone must split the remaining work: bounded/lazy candidate production for typing latency, plus queryable table+prism storage and possible mmap for cold-start/memory. mmap is therefore deferred as part of that broader design, not as a standalone M33 change. _Outcome: M33 complete as a fair cold/warm startup/session improvement, with no typing, browser, memory-footprint, or "faster than librime" claim._

### M36 product-path performance boundary (project-wide D-33)

**D-33 / M36-PRODUCT-PERF - Optimize the shipped TypeDuck product path separately from fair upstream `luna_pinyin` comparison rows.** M33-M35 made valid progress on the upstream comparison path, but the product path users run (`jyut6ping3` / `jyut6ping3_mobile`) still fell back to heap/source-YAML storage because the shipped TypeDuck compiled tables carry a marisa string table that Yune rejects. M36 therefore treats Track A (`luna_pinyin` Yune vs librime) as comparison-only evidence and Track B (`jyut6ping3` Yune before/after) as the product target. The `rsmarisa` path remains closed by measured no-go for this milestone because the actual product blobs require table, reverse, and prism semantics together, and the shipped artifacts are stale/unsupported as a set. The accepted M36 product strategy is schema-scoped no-marisa re-emission into Yune-readable compiled table/prism/reverse artifacts, including the configured `jyut6ping3_mobile` prism stem, then compact product storage only when compiled artifacts load. Native mmap and WASM byte-backed loading stay future gated work; mapping bytes and rebuilding owned heap maps still does not count. ABI stability, TypeDuck rich comments, lookup records, tolerance/correction, partial selection, default-confirm recomposition, long composition, and userdb learning remain hard gates. _Outcome: M36 complete. Track B `jyut6ping3_mobile` final rows run with `compiled_ready=true`, product typing rows improve up to `81.7%`, and product max peak working set drops from `1000.4 MB` to `885.3 MB`; Track A still trails librime widely and no browser/product-delivery claim is made._

### yune-web public demo readiness (project-wide D-34)

**D-34 / M31-YUNE-WEB-PUBLIC-DEMO - Publish the first public web demo as `yune-web`, with delivery claims separated from engine claims.** M31 makes the TypeDuck-Web-derived harness publishable as a Yune engine demo without treating the separate TypeDuck-Web product repo as the deployed surface. The repo-owned app path, public app name, package, Cloudflare Pages project, evidence, and docs use `yune-web`; the upstream-derived source checkout lives at `apps/yune-web/source/` and the Yune-owned source patch remains reviewable and reverse/forward-checkable. TypeDuck-Web shell and dictionary assets are owner-approved by the project owner, who is also the TypeDuck author.

M31 exposes only browser-honest output standards proven by Yune evidence: Hong Kong Traditional and `hk2s` Simplified through the existing `simplification` option and OpenCC data. Unsupported OpenCC standards remain absent. The public build is reproducible from checked-in Yune state, uses Pages Direct Upload at <https://yune-web.pages.dev>, pins schema assets by SHA-256, prunes non-public schema families, and records local plus deployed Playwright evidence. AI remains default-off/local-only with no remote calls, telemetry, or secrets, and M31 does not add richer AI UX.

M31's performance boundary is narrow: it records delivery/cache readiness, not a browser startup or typing speed win. The public `jyut6ping3_mobile` boot path still includes Luna and scholar lookup assets required by current TypeDuck schema behavior, so full reverse-lookup dependency deferral is a measured future follow-up rather than an M31 claim. No default `RimeApi`, `RimeCandidate`, or upstream ABI surface changes are part of this decision. _Outcome: M31 complete with deployed smoke evidence and a documented delivery caveat._

### M38 pure engine performance parity (project-wide D-35)

**D-35 / M38-ENGINE-PERF-PARITY - Treat M38 as a pure isolated-engine parity milestone against upstream librime, not as product or frontend optimization.** M38 returns the performance target to the clean upstream `luna_pinyin` shoulder-to-shoulder comparison against same-run upstream librime `1.17.0`. Product, frontend, browser, packaging, delivery, and external application rows may be regression guards only; they cannot close M38 and must not be used for performance claims.

M38 must not close on partial improvement. Closeout requires all hard data-path strategies and measured performance thresholds together: the selected native table/prism bytes must be mmap/file-backed or otherwise borrowed from deployed bytes; the benchmarked hot path must use a real marisa-backed deployed table through `rsmarisa` with positive lookup counters and zero ordinary no-marisa fallback; ordinary first-page candidate production must be lazy/page-bounded or equivalent, with full-list fallback explicit and counted; memory and allocation owners must be attributed without a hidden heap mirror; startup/session must be within `1.25x` of same-run librime; and `hao`, `ni`, and `zhongguo` must each be within `5x` of same-run librime. If any strategy gate or performance threshold fails, M38 stays active even if the remaining owner is understood. _Outcome: M38 complete for the original short/medium upstream Track A target set; post-M38 evidence moved broader typing parity to D-36/M39._

### M39 whole-engine performance guardrail (project-wide D-36)

**D-36 / M39-WHOLE-ENGINE-PERF - Treat post-M38 performance as a whole-engine regression problem, not a single latency row.** M38 proved that same-run native benchmarking, mmap-backed selected table/prism bytes, real `rsmarisa` marisa-table lookup, page-bounded first-page production, page-sized context export, startup/session lifecycle attribution, and owner counters can bring the short/medium upstream `luna_pinyin` path near librime. The post-M38 long-input baseline also proves that this is not enough for broader typing parity: continuous 37-character and 59-character pinyin rows remain more than `1,000x` slower than same-run librime, and the Cantonese `jyut6ping3_mobile` 50+ character profile row has not yet been baselined.

M39 therefore must keep every existing optimization strategy active while attacking the newly measured long-composition owner. It must also add and gate the Cantonese `jyut6ping3_mobile` row `neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung`, then prove whether that profile shares the Track A owner or needs a separate fix/no-go. The milestone cannot close if it fixes long input by regressing startup/session, `hao`, `ni`, `zhongguo`, mmap/file-backed selected bytes, `rsmarisa` activation, bounded output/context export, working set, peak memory, allocation shape, or upstream-observable behavior. Long-input claims must name the measured owner inside the translator bucket before implementation; memory claims require heap-owner attribution, not just working-set movement. Browser, packaging, and delivery rows remain outside the M39 performance claim unless a separate plan supplies separate evidence. _Outcome: M39 complete; M40 inherits the remaining sentence-index parity gap._

### M40 compiled sentence lookup index (project-wide D-37)

**D-37 / M40-COMPILED-SENTENCE-LOOKUP - Combine exact range indexing, reachable-vertex pruning, prefix filtering, and a librime-shaped table phrase index for native Track A sentence lookup.** M39 reduced the catastrophic long-input owner, but the remaining Track A gap still lives inside `UpstreamSentenceModel`: the 37-character row is `1.765x` and the 59-character row is `1.320x` versus same-run upstream librime, with the 59-character row still spending about `836.9us/key` in the upstream sentence model and doing about `608.6` code-prefix checks per key. M40 therefore targets the sentence lookup shape rather than raw table lookup, context export, browser startup, or frontend/product delivery.

M40 treats four tactics as one data-path change and as hard strategy gates: exact code range indexing, reachable start-vertex pruning, valid-code/prefix filtering, and a sentence phrase index shaped after librime's prism/common-prefix plus table phrase-index path. Rust-native `HashMap`/`HashSet` or range-index structures are acceptable implementation tools, but closeout must prove the runtime behavior is index-driven rather than the old all-substrings loop with repeated binary searches. M40 also requires a measured verdict on cross-keystroke graph rebuild: if repeated whole-graph rebuild becomes the top remaining long-row owner after A/B/C/D, M40 must implement a bounded incremental graph/cache path or record a measured blocker.

M40 cannot close by trading typing latency for startup or memory. Any sentence index must be lazy, shared across sessions, borrowed from compiled bytes, interned, compact, or otherwise proven cheap by first-use timing, warm-reuse timing, working-set/peak memory, and owner evidence. Startup/session, short rows, Track A memory, `rsmarisa`, mmap-backed selected bytes, bounded output/context, and upstream-observable behavior remain no-regression gates. Browser, web harness, frontend, application, packaging, deployment, and product-delivery claims remain out of scope by design; they need separate evidence and must not redirect this isolated engine-parity milestone. _Outcome: M40 complete. The accepted implementation uses a compact numeric range index over existing sorted sentence-model entries, reachable-start graph construction, prefix range narrowing with early breaks, and phrase-index walk counters rather than cloned-string `HashMap`/`HashSet` or trie mirrors. Final Track A long rows are `0.980x` and `0.712x` same-run librime, Track A peak memory is below M39, and graph rebuild is measured but not the top remaining owner._

### M41 yune-web startup optimization (project-wide D-38)

**D-38 / M41-YUNE-WEB-STARTUP - Treat `apps/yune-web` startup as a separate browser-harness pipeline, not an extension of native engine parity.** M40 closes the current native Track A long-row owner, but it does not prove browser startup, public-demo startup, worker/WASM startup, asset delivery, cache correctness, virtual filesystem persistence, schema deploy reuse, or first browser key-to-paint. M41 therefore optimizes the tracked `apps/yune-web/` harness only after a fresh post-M40 production-browser baseline names the top owner.

M41 must split real-worker and mock-worker startup for both `luna_pinyin` and `jyut6ping3_mobile`, using production builds as headline evidence. It may optimize browser shell code, asset packaging/transfer/cache, worker/WASM lifecycle, virtual filesystem persistence, deploy/schema reuse, and web-runtime integration boundaries, but it must not widen native engine claims or touch Rust engine code unless browser evidence proves a narrow runtime-boundary blocker and the relevant native gates are rerun. Public-demo claims require separate public-demo evidence. _Outcome: M41 complete under [`plans/completed/m41-plan-yune-web-startup-optimization.md`](./plans/completed/m41-plan-yune-web-startup-optimization.md). The accepted implementation fixes production runtime packaging, avoids the redundant default startup deploy path measured around `15,538 ms` on `jyut6ping3_mobile` while still customizing default runtime preferences, starts the worker with the selected schema, schema-scopes startup assets, and closes with tracked cold medians of `846 ms` for `luna_pinyin` and `1,254 ms` for `jyut6ping3_mobile`, plus public-demo cold medians of `867 ms` and `1,291 ms`. No native engine claim is made._

### M42 abbreviation sentence parity and short-key guardrails (project-wide D-39)

**D-39 / M42-ABBREVIATION-SENTENCE-PARITY - Treat `cszysmsrsd` and `zybfshmsru` as upstream behavior gaps, not speed wins; repair abbreviation-aware sentence lookup before further short-key or memory optimization.** M40 made the full-pinyin Track A long rows competitive through a compact sentence lookup index, but the incomplete-pinyin rows stayed non-comparable because Yune exports `0` candidates. Upstream librime `1.17.0` remains the oracle for these rows. My RIME can suggest the user-visible shape, but it cannot replace captured upstream oracle evidence or define final candidate ordering.

M42 therefore fixes abbreviation-aware `luna_pinyin` sentence lookup first only if Phase 0 proves there is a real upstream behavior target. Phase 0 must capture candidate count, text, comments, order, preedit, schema metadata, and provenance from upstream librime before implementation hard-codes expected output. If upstream exports no meaningful candidates for these rows, M42 stops the abbreviation implementation path and records a reporting/no-go correction instead of building a span graph for a non-bug. If the oracle does prove the gap, the accepted design must derive bounded canonical-code spans from compiled prism/schema spelling data and route the sentence model over those spans without source fallback, selected table/prism heap mirrors, or regression to the old all-substrings scan. The abbreviation path must stay behind a separate branch so full-pinyin long rows never invoke abbreviation span expansion. Once the rows are behavior-comparable, latency can be judged against same-run librime.

The remaining `ni`/`hao` gap is a separate fixed-overhead problem, not a sentence-model problem. M42 may optimize it only after counters name the owner, and only if startup/session, `zhongguo`, both full-pinyin long rows, memory, `rsmarisa`, mmap or byte-backed selected bytes, bounded output/context, and the Track B 50+ guard row remain intact. Browser, frontend, product, packaging, delivery, and public-demo claims remain outside this native-engine decision. _Outcome: M42 complete under [`plans/completed/m42-plan-abbreviation-sentence-parity-short-key-guardrails.md`](./plans/completed/m42-plan-abbreviation-sentence-parity-short-key-guardrails.md). Phase 0 proved meaningful upstream candidates, so the accepted implementation adds bounded prism-derived code-span abbreviation routing and a target-scoped phrase vocabulary behind a separate branch from the M40 full-pinyin sentence lookup path. Final native candidate output matches upstream for both rows, but latency remains a measured blocker (`3.469x` and `5.069x` same-run librime). `ni`/`hao` were profiled and not optimized. Track A storage/memory/bounded-output gates and the Track B guard remain intact, and no browser, product, or TypeDuck-profile speed claim is made._

### M43 native memory and short-key owner reduction (project-wide D-40)

**D-40 / M43-MEMORY-SHORTKEY-OWNER - Profile retained memory and short-key fixed overhead before changing representation or hot-path behavior.** After M42, the two biggest native-engine gaps are different shapes: Track A whole-process memory remains much larger than librime (`119,775,232 B` peak versus roughly `13-17 MB` observed librime rows), while `hao` and `ni` remain slower than librime (`3.424x` and `4.082x`) despite being under the existing short-key guard. M43 therefore starts with owner evidence rather than assuming the duplicated-string hypothesis, candidate materialization, or ABI export path is the top owner.

M43 must choose exactly one primary implementation branch after Phase 0: memory-owner reduction, `hao`/`ni` short-key fixed-overhead reduction, or reporting/no-go. Memory work requires structural owner attribution for compact-table syllabary state, table/translator indexes, poet `ModelEntry` storage, sentence indexes, abbreviation vocabulary, schema/config/processors, userdb, and runtime/session state. That attribution must separate non-overlapping heap-owned reducible bytes from mmap-backed, shared, guarded, or overlapping estimates, and it must reconcile estimates against measured Track A working-set/peak evidence before branch selection. A whole-process memory win requires measured Track A peak movement; estimate-backed owner movement without peak movement is only a partial structural reduction. Short-key work requires `hao`/`ni` owner buckets for lookup, translation, candidate materialization, ranking/filtering, context export, and ABI string allocation/free. A short-key win is self-relative to M42 unless final same-run ratios prove parity; reports must publish residual librime ratios and may not imply the `hao`/`ni` librime gap is closed while the rows remain multiple times slower than librime. No branch may use source fallback, selected table/prism heap mirrors, unbounded candidate materialization, unreported always-on caches, or behavior drift as a shortcut.

M43 intentionally does not target abbreviation latency. The M42 abbreviation rows must keep matching upstream candidate count, text, comments, order, preedit, commit preview, and first-page metadata, but their `3.469x`/`5.069x` latency blockers belong to a separate future abbreviation-owner milestone. Startup/session, `zhongguo`, both full-pinyin long rows, `rsmarisa`, mmap or byte-backed selected bytes, bounded output/context, and the Track B guard remain no-regression gates. Browser, frontend, product, packaging, delivery, public-demo, and TypeDuck-profile speed claims remain outside the M43 native-engine decision. _Outcome: M43 complete under [`plans/completed/m43-plan-native-memory-short-key-owner-reduction.md`](./plans/completed/m43-plan-native-memory-short-key-owner-reduction.md). Phase 0 selected memory-owner reduction because `poet.entries_by_code` retained `38,208,541 B`; final evidence reduces that owner to `18,694,662 B` while preserving behavior and storage guards. Track A peak stayed inside the Phase 0 noise band but did not meet the whole-process memory target, so the closeout is a partial structural reduction with a measured memory blocker, not a whole-process memory win or short-key speed win._

### M44 native performance owner reduction (project-wide D-41)

**D-41 / M44-NATIVE-PERFORMANCE-OWNER - Attack all four current native performance blockers with owner evidence before implementation.** Post-M43 analysis shows four remaining issues large enough to target together: Track A `hao`/`ni` short-key latency (`3.329x`/`3.954x` same-run librime), Track A abbreviation latency (`cszysmsrsd` `3.361x`, `zybfshmsru` `5.071x`), Track A peak memory still around `127.5 MB` after the M43 `poet.entries_by_code` reduction, and Track B deployed product-profile short rows such as `h` at `21.889 ms`, `ha` at `11.639 ms`, and `hai` at `7.580 ms`. M44 therefore starts by instrumenting all four owner families before changing hot-path behavior.

M44 has four workstreams: abbreviation span/search or ranking, short-key borrowed first-page materialization, memory RSS/private-owner reduction, and Track B bounded spelling/lookup reduction. A performance-success closeout requires all four target families to pass: `hao <=28.300us`, `ni <=41.813us`, `cszysmsrsd <=3,094.635us`, `zybfshmsru <=3,183.353us`, Track A peak memory `<=107,797,708 B`, and Track B selected short rows at least `50%` faster with lookup-explosion counters down at least `75%` where applicable. If any target misses, M44 may close only as a partial result with measured blockers and explicit next evidence needs. _Outcome: M44 closes as a partial native/profile performance reduction under [`plans/completed/m44-plan-native-performance-owner-reduction.md`](./plans/completed/m44-plan-native-performance-owner-reduction.md), not as a full performance success. Final evidence passes `hao` (`24.700us`, `2.123x`), both abbreviation rows (`545.020us`/`0.445x`, `540.970us`/`0.634x`), and all selected Track B short rows (`84.7-92.4%` faster, exact lookups reduced from thousands to `1-3` per key). `ni` remains a measured short-key blocker at `49.450us`/`3.434x`, and Track A peak memory remains a measured memory blocker at `127,619,072 B` against the `<=107,797,708 B` target._

All implementations must preserve closed behavior and storage invariants. The M42 abbreviation sentence path stays separate from the M40 full-pinyin sentence lookup path; `hao`/`ni` output must remain upstream-observable; Track A selected storage remains `rsmarisa_byte_backed`; table/prism heap mirrors stay `0`; `source_fallback=false`; first-page output and `RimeGetContext` stay bounded; and the Track B 50+ guard row remains stable. M44 remains native/profile-scoped: no browser, frontend, packaging, deployment, public-demo, or broad product-delivery speed claim may be made from this evidence. Old final evidence remains part of the audit trail, and disk cleanup is a separate maintenance task because `target/` is much larger than `docs/reports/evidence/`.

### M45 native short-key latency and memory attribution (project-wide D-42)

**D-42 / M45-SHORTKEY-MEMORY-ATTRIBUTION - Split short-prefix implementation from memory attribution, and keep browser/WASM work out of the native milestone.** Post-M44 diagnostic profiling shows that the residual Track A short-key blocker is sharper at the intermediate `n` row than at final `ni`: `n` is `79.400us` / `3.626x` same-run upstream librime, expands to `27` table lookup codes, and considers `1,260` raw candidates before page export. `ni` remains slow at `53.750us` / `3.571x`, while `hao` is improved but still above librime at `25.667us` / `2.121x`. M45 therefore targets the `n`/`ni`/`hao` short-prefix family directly, but only after Phase 0 captures upstream candidate-output oracle evidence for those rows.

The accepted short-key implementation shape is a bounded borrowed-prefix path: keep exact and prefix candidates borrowed until first-page order is proven, early-stop only when filters/uniquifier/simplifier/page-size behavior cannot under-fill or reorder the page, and materialize only exported candidates. The M44 under-fill fallback remains required. M45 must not invoke M40 full-pinyin sentence lookup or M42 abbreviation routing on short-key rows, and it must not silently widen TypeDuck/profile behavior. Bare `n` is a degenerate one-letter row, so if it remains above the `<=3.0x` target after `ni`/`hao` improve, the closeout must name it as a measured benchmark-parity blocker rather than a user-visible UX blocker.

M45 also reframes the memory blocker as an attribution question before code. The post-M44 diagnostic repeats the same Track A high-water peak (`127,430,656 B`) across workloads, while after-ready medians are lower (`87-98 MB`) and may already satisfy the old `<=107,797,708 B` target if the target is interpreted as steady resident memory rather than process high-water peak. Phase 0 must not lump all high-water causes together: a benchmark-cumulative artifact can be reframed away from the resident-memory target, but a real per-cold-start deploy/startup peak remains a standing peak cost and possible constrained-machine/OOM risk. Because M43 reduced `poet.entries_by_code` by `19,513,879 B` without moving peak, M45 must split steady after-ready resident size, observed high-water peak, benchmark-cumulative high-water, real cold-start peak, private heap, file-backed mapped pages, allocator high-water, deployment/startup transient memory, retained owner estimates, and unclassified process memory before authorizing any storage rewrite.

M45 memory may close as `steady-state-meets-target-benchmark-artifact`, `steady-state-meets-target-standing-peak-cost`, `transient-peak-bound`, `measured-no-go`, or a narrowly evidenced `memory-owner-reduction`. Both the high-water peak and steady resident numbers must stay visible in reports. A memory win cannot be claimed from structural owner estimates alone, and a reframe must still report the remaining librime memory gap honestly. Browser harness, WASM linear memory, yune-web/My RIME browser comparison, public-demo packaging, product delivery, AI, learned `.gram`/octagram, and plugin ABI work remain out of scope. WEB-01 owns the browser-harness sidecar, and any future WASM engine-memory reduction must be a separate milestone.

_Outcome: M45 complete with measured blockers under [`plans/completed/m45-plan-native-short-key-latency-memory-attribution.md`](./plans/completed/m45-plan-native-short-key-latency-memory-attribution.md). Phase 0 selected `short-key-measured-no-go`, so no short-key engine implementation branch was retained. Final `hao` preserves the M44 pass at `24.267us` / `2.110x`; final `n` and `ni` match upstream candidate output but miss at `68.900us` / `3.313x` and `49.450us` / `3.458x`. M45 records memory as `steady-state-meets-target-standing-peak-cost`: steady Track A resident rows are below `107,797,708 B`, while the real cold-start peak remains `127,475,712 B`. No browser, WASM, frontend, product-delivery, public-demo, packaging, deployment, AI, learned `.gram`/octagram, or plugin ABI claim is made._

### WEB-01 yune-web WASM heap and payload attribution (project-wide D-43)

**D-43 / WEB01-WASM-HEAP-PAYLOAD - Treat browser WASM linear-memory reduction as a separate harness claim, and stop when attribution proves the owner is deeper than the harness.** WEB-01 may change `apps/yune-web`, browser benchmark harnesses, public-demo packaging, worker resource loading, and WASM build flags, but it may not commit `crates/` changes or claim native-engine memory wins. Browser `HEAPU8.buffer.byteLength` is real browser pressure, but it is a reserved linear-memory metric rather than native active-byte ownership.

Lowering `INITIAL_MEMORY` is a valid browser-harness lever only if final ready, peak, and steady-state linear memory move. Payload pruning is a valid browser-harness lever only when real-browser behavior gates prove typing, commit, schema switching, reverse lookup, and userdb persistence are preserved. If asset-family attribution shows that `jyut6ping3_mobile` reaches the same high-water with core-only or empty shared-asset families, the remaining memory target must be handed to a future WASM runtime/engine plan rather than described as a WEB-01 win.

_Outcome: WEB-01 complete with measured no-go under [`plans/completed/web01-plan-yune-web-wasm-heap-payload-optimization.md`](./plans/completed/web01-plan-yune-web-wasm-heap-payload-optimization.md). Final 64 MiB `INITIAL_MEMORY` still settles at `160.0 MiB` for Luna and `893.1 MiB` for Jyutping; 48 MiB worsens Luna to `176.0 MiB`. Final attribution keeps Jyutping at `893.1 MiB` for `extras`, `jyutping-core`, and `full-jyutping`, so WEB-01 records `engine-owned-measured-no-go` at the WASM boundary. The Cangjie -> Luna -> Jyutping schema-switch row is promoted to a named follow-up because the current runtime can grow to about `1.9 GiB` WASM and return no Jyutping candidates; the follow-up must first prove whether that row reproduces on the pre-WEB-01 baseline or was introduced by WEB-01 harness changes. No native, browser heap, payload, public-demo speed, packaging, deployment, or product-delivery win is claimed._

### M46 Jyutping native and WASM memory attribution (project-wide D-44)

**D-44 / M46-JYUTPING-MEMORY-ATTRIBUTION - Attribute TypeDuck/Jyutping native
and WASM memory before selecting a storage rewrite.** M45 and WEB-01 leave a
clear memory problem, but not yet a proven owner. Native Track B product rows
peak around `504 MB` and browser Jyutping reaches `893.1 MiB`; however, the
currently named native owners do not explain the headline. Track B
`compact_table.syllabary_codes` is only about `4.2 MB`, and the `8.3 MB`
`translator.entries_by_code` row is guarded/source-YAML or small-test state,
not retained by the selected compact product path. Therefore M46 must not
accept the hypothesis that adding a Jyutping `rsmarisa` string table will save
`~200 MB` until fresh attribution proves that magnitude.

M46 must also keep byte categories honest. Product path status reports
`byte_source_len=15,248,382` for `jyut6ping3` and
`byte_source_len=27,325,622` for `jyut6ping3_scolar`, but those are selected
source/status byte lengths unless owner evidence classifies them as retained
process memory. A `27.3 MB` mapped-region or retained-owner claim must cite the
exact evidence row or be removed.

M46 spans the native TypeDuck/Jyutping product path and the yune-web WASM
runtime because WEB-01 proved the browser high-water is not a harness-only
`INITIAL_MEMORY` or payload-family problem. It starts with owner evidence for
code strings, candidate text/comment payloads, lookup records, reverse indexes,
base-vs-scolar duplicated structures, table/prism/reverse/OpenCC bytes,
source/deploy transients, allocator high-water proxies, and unclassified
process memory. Browser evidence must separately measure single-schema Luna,
single-schema Jyutping, Jyutping core/full asset families, and schema-switch
sequences. A new WEB-01-style harness plan is not scheduled; M46 is the
browser-memory handoff unless Phase 0 reclassifies the owner as harness-only.

The severe Cangjie -> Luna -> Jyutping row is a first-order gate, not an
optional smoke: M46 must reproduce or classify the `~1.9 GiB` WASM /
no-Jyutping-candidates behavior against the current runtime and, where
practical, the pre-WEB-01 executable baseline before memory optimization
continues. The no-candidates half is a correctness issue independent from the
`1.9 GiB` high-water. Phase 0 must name whether it affects real product flows
or only the Cangjie -> Luna -> Jyutping test path, name the owning layer and
severity, and fix or explicitly own it before closeout if it is
product-affecting.

M46 must also explain the native-vs-WASM gap before claiming browser memory
success. Native Track B runs around `504 MB` peak and `427-441 MB` steady,
while browser Jyutping reaches `893.1 MiB`. Native mmap/file-backed movement
does not automatically transfer to WASM linear memory, where the same compiled
bytes may be copied into heap. Phase 0 must identify WASM-specific owners and
state which native owner reductions are expected to move browser high-water.

A fresh Track B `rsmarisa` spike is allowed only after Phase 0 authorizes it
and only after reconciling the M36 product-path no-go. M36 rejected stale or
unsupported shipped product blobs as a table/reverse/prism set; M46 may only
count fresh compatible artifacts that preserve TypeDuck v1.1.2 behavior,
including rich comments, lookup records, correction/tolerance, partial
selection, default-confirm recomposition, long composition, schema switching,
reverse lookup, browser dictionary panels, and userdb behavior.
`cargo test -p yune-core --test cantonese_parity` is a hard gate for storage,
payload, candidate, lookup, correction, and TypeDuck profile changes.

M46 success requires measured movement in both the selected native memory metric
and browser WASM ready/peak/steady high-water for the owner being claimed.
Structural owner reductions without RSS/private/working-set and WASM movement
are partial at best. If the largest owners are required for TypeDuck behavior
or remain unclassified, M46 must close as a measured blocker or no-go rather
than a performance success. If code-string interning only moves single-digit or
low-double-digit megabytes, the milestone must pivot to candidate
text/comment payloads, `jyut6ping3_scolar` duplication, or another measured
owner before claiming a memory win. No Track A short-key, browser-harness
payload, public-demo, packaging, deployment, product-delivery, AI, learned
`.gram`/octagram, plugin ABI, or broad frontend speed claim may be made from
M46 evidence.

2026-06-27 Phase 0 update: M46 selected
`schema-switch-regression-fix-first`. Fresh Track B native evidence records
`504,627,200 B` peak working set and `427,356,160-442,966,016 B` steady
resident rows with `source_fallback=false`, selected `byte_backed` storage,
table/prism mmap, and selected heap mirrors `0`. The new owner profile names
`compact_table.lookup_records` at `31,920,140 B`, base
`compact_table.storage` at `15,248,382 B`, guarded
`translator.entries_by_code` at `8,327,700 B`, code strings at `4,189,674 B`,
and candidate text/comment payload at `2,575,292 B` logical overlapping bytes;
those named rows do not explain the process memory headline and do not
authorize a Track B `rsmarisa` or payload branch. Browser evidence keeps
Jyutping at `893.1 MiB` for single-schema and asset-family rows. Clean Jyutping
startup passes, but Cangjie -> Luna -> Jyutping returns zero Jyutping
candidates in the current post-WEB-01 runtime at `893.1 MiB`; the older
`~1.9 GiB` high-water remains historical WEB-01 evidence and did not reproduce
in the structured M46 capture. M46 remains active and is not a memory success.

2026-06-27 closeout update: M46 Branch A fixed the product-affecting
schema-switch correctness issue without moving memory. The app now skips the
redundant default deploy on unchanged Jyutping schema selection, Cangjie
reverse lookup uses the plain `jyut6ping3` dictionary with tone stripping, the
worker ships the missing Luna/Jyutping source dictionaries for reverse lookup,
and reverse lookup translators honor their own namespaced spelling algebra.
Post-fix browser evidence shows clean Jyutping, Cangjie -> Luna -> Jyutping,
and Jyutping -> Luna -> Jyutping all return `nei -> 你` with six candidates,
zero worker action errors, and `893.1 MiB` WASM high-water. M46 closes as
`schema-switch-correctness-fixed-memory-unchanged` with
`measured-no-go-owner-unclassified`: native Track B still peaks at
`504,627,200 B`, browser Jyutping still reaches `893.1 MiB`, and no native
Track B or browser WASM memory success is claimed.

### WEB-02 Jyutping public-demo WASM memory owner classification (project-wide D-45)

**D-45 / WEB02-JYUTPING-SOURCE-FALLBACK-OWNER - Fix the browser/public-demo
compiled-asset contract before more Jyutping memory-reduction experiments.**
WEB-02 proves the public-demo Jyutping browser path is not byte-backed. The
live web ABI path reports `source_fallback=true`, `selected_storage=owned_heap`,
`byte_source_len=0`, and fallback reason
`source fallback after compiled reject: Invalid("prism parse failed:
UnsupportedVersion")`. The shipped public-demo Jyutping prisms are
`Rime::Prism/3.0`, while M46's valid native Track B path used generated
current `Rime::Prism/4.0` deploy artifacts and selected byte-backed storage
with `source_fallback=false`.

The retained owner is engine heap at runtime:
`translator.entries_by_code=510,925,748 B` plus `18,676,626 B`
(`529,602,374 B` total). The root cause belongs to web/public-demo artifact
delivery and deploy staging, not to WEB-01 payload-family selection or
`INITIAL_MEMORY`. WEB-02 does not claim a memory reduction and does not move the
known `893.1 MiB` high-water. Any follow-up Jyutping browser memory branch must
first make the browser/public-demo path select current byte-backed compiled
storage, then rerun WASM ready/peak/steady evidence. If that still leaves a
large high-water, allocator/transient profiling can follow.

### Initialization notes (process decisions)

**D-INIT-1 - Existing `docs/plans/completed/m00-analysis-founding.md`, `docs/roadmap.md`, and `docs/plans/completed/m05-m07-record-foundation-refactor.md` are the retained source context** for the now-retired GSD project. Historical `.planning/codebase/` notes were folded into the retained docs before `.planning/` was removed. External research was skipped at setup because scope was driven by existing docs and direct librime comparison.

**D-INIT-2 — Future compatibility slices must choose module ownership, test ownership, and the librime comparison target before implementation.** (Captured as a standing principle; see "Standing principles" and D-05.)

**D-INIT-3 — Retire the GSD (get-shit-done) planning system.** The upstream GSD tool was abandoned/rug-pulled, so `.planning/` was removed and its durable content consolidated under `docs/`: `decisions.md` (this log), `requirements.md` (requirement tracker), `conventions.md` (architecture/stack/conventions/testing reference), and `roadmap.md` (current dashboard). No code change - the engine never depended on `.planning/`. Rationale: the planning content is durable and worth keeping, but the tool itself is dead and unsafe to keep installed. Outcome: Good.

**D-DOC-1 — Every doc under `docs/plans/active/`, `docs/plans/reference/`, or `docs/plans/completed/` opens with a status banner whose `Milestone` field names its owning milestone/stage**, kept separate from the `Status` field. Rationale: a plan's scope and state are visible at a glance and `grep`-auditable (`grep -rn "Status:" docs/plans`); banner milestones stay consistent with `roadmap.md` and `ledgers/milestone-history.md`. Finished/superseded plans move to `docs/plans/completed/`, never deleted. See `conventions.md` §10.

**D-DOC-2 — Browser-validation claims require committed real-browser, real-asset evidence.** An assertion without a committed artifact does not count as validation — this is why the placeholder-echo WI-4 matrix was reopened (D-P10-9) and why HR-1b committed the real-assets browser run rather than only describing it.

---

_Last updated: 2026-06-27 - D-45 records WEB-02 complete as a public-demo Jyutping source-fallback owner classification: shipped `Rime::Prism/3.0` assets force `owned_heap`, retained `translator.entries_by_code` rows total `529,602,374 B`, and the next branch must fix the compiled-asset contract before further memory experiments. D-44 M46 is complete with measured blockers: Branch A fixed the Cangjie -> Luna -> Jyutping no-candidate correctness bug, but native Track B remains `504,627,200 B` peak with mostly unclassified memory and browser Jyutping remains `893.1 MiB`, so M46 closes as `schema-switch-correctness-fixed-memory-unchanged` with `measured-no-go-owner-unclassified`. D-43 records WEB-01 complete with measured browser-harness no-go: lower `INITIAL_MEMORY` does not reduce settled linear memory, 48 MiB worsens Luna, and Jyutping remains `893.1 MiB` even for empty/core attribution rows. D-42 records M45 complete with measured native-engine blockers: `hao` passes, `n` and `ni` match upstream candidate output but miss `<=3.0x`, steady Track A resident memory meets the resident target, and the real `127,475,712 B` cold-start peak remains a standing blocker. D-41 records M44 complete as a partial native/profile performance reduction. D-40 records M43 complete as a native partial structural memory reduction. D-39 records M42 complete with a measured abbreviation-latency blocker. D-38 records M41 complete as a separate browser-harness startup milestone. D-37 records M40 complete, D-36 records M39 complete, and earlier decisions remain in force._
