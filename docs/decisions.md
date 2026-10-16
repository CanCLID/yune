# Yune Decision Log

Standing principles and the decisions that still bind current code, ABI, oracle
choices, scope, or test design. Decisions that only recorded how a finished
milestone was run were removed in the 2026-10 docs consolidation; the complete
log is in git history before the consolidation commit. New decisions are made
in GitHub issues and, when they become binding, summarized here.

## Standing principles

Cross-cutting decisions that govern all current and future work:

- **librime is the compatibility oracle, not an architecture template.** librime defines the externally observable contract (schema semantics, config behavior, candidate output, C ABI expectations, deployed-data compatibility, frontend integration). The implementation is free to be idiomatic, typed Rust internally as long as the boundary contract is preserved. Preserve librime-_observable behavior_, not librime's internal C++ complexity.

- **Name the protected behavior before mirroring librime.** Before adding a librime-derived mechanism, name the external behavior it preserves (user-visible input, frontend ABI, schema/config semantics, or deployed-data compatibility). If it only mirrors a librime implementation detail, prefer a smaller Yune-native design, isolate it behind an adapter, or document an explicit deferral.

- **AI-native input is a separate, local-first, non-blocking, source-labeled layer.** LLM assistance is layered as optional candidate providers, rankers, context providers, and memory stores with timeout/fallback policy. AI results must be source-labeled, optional, and safe to discard, and they must never block or slow classic input. Classic input stays predictable and low-latency without network access; remote model calls can never be required for baseline input. Context/memory collection must be opt-in, inspectable, clearable, and disabled for sensitive contexts.

- **Build compatibility fixtures and ABI tests before replacing modules.** Behavior must be measurable against librime before any difference can be classified as an improvement or a regression. Each new behavior slice names its owning implementation module, owning test module, and librime comparison target before code changes; new failing comparisons/fixtures come before dispatch or state-mutation changes.

- **Module and test ownership per slice; `lib.rs`/`main.rs` stay facades.** Each compatibility slice owns its implementation module and test module. `lib.rs` (and `main.rs`) remain thin facades plus facade-owned tests; cross-cutting behavior tests live under per-crate `tests/` modules. Mechanical refactors preserve observable behavior and assertions; they do not rewrite working slices.

- **Plugin ABI compatibility is deferred.** Full C++ librime plugin ABI compatibility (Lua, dynamic octagram plugin loading, predict, proto, etc.) is expensive and not yet required by a concrete frontend or schema-migration path. M54's native octagram-compatible `Grammar` provider is not plugin ABI compatibility. Deferred until a real distribution requirement makes it necessary.

- **Upstream-first oracle sequencing.** Upstream `rime/librime 1.17.0` is the default core compatibility oracle. TypeDuck-HK/librime `v1.1.2` is retained as a TypeDuck-Web/Windows compatibility-profile oracle only. If upstream and TypeDuck behavior disagree, upstream wins for core Yune; TypeDuck behavior must stay behind a named profile test, fixture, adapter, or ABI note.

- **Compatibility is target-driven, not feature-complete.** The oracle is a behavioral _floor_, not a feature checklist. "Done" is relative to named targets: upstream `luna_pinyin`, captured common-schema lanes, canonical Jyutping against upstream librime plus pinned `rime/rime-cantonese`, and TypeDuck multilingual/comment/profile behavior in its profile lane. Reproducing every librime gear, plugin, or code path is an explicit **non-goal**; a librime feature is implemented only when a named target needs it. The AI-native layer on top of this compatible base — not parity for its own sake — is the product goal. See D-25.

## Decision log

### Foundational decisions (from PROJECT.md Key Decisions and Context/Constraints)

**D-01 — Use librime as the compatibility oracle, not an architecture template.** Existing schemas and frontends depend on librime contracts, but Rust can model the internals more cleanly. Prefer typed, idiomatic Rust modules over cloning librime's C++ structure when the boundary contract is preserved. _Outcome: Good._

**D-02 — Build compatibility fixtures and ABI tests before replacing deeper engine modules.** Behavior must be measurable before differences can be classified as improvements or regressions. _Outcome: Good._

**D-03 — Keep AI ranking optional and local-first.** Classic input must remain predictable and low-latency without network access. _Outcome: Adopted; M11/M13 preserve the default-off local-first boundary._

**D-04 — Treat AI-native input as a separate product layer above compatibility.** librime cannot guide LLM-native behavior, so Yune needs explicit provider, context, memory, fallback, and privacy contracts. _Outcome: Adopted as a standing architecture rule._

**D-05 — Treat the module/test refactor as a structural rule for future feature work.** Large single-file accumulation slowed review, search, focused testing, and extraction; module/test ownership per slice with `lib.rs`/`main.rs` as facades is now the rule. _Outcome: Adopted as a standing ownership rule._

**D-06 — Keep plugin ABI compatibility deferred.** Plugin compatibility is expensive and not yet required by a concrete frontend or schema-migration path. Native M54 octagram-compatible grammar support does not change this because it does not load librime C++ plugins. _Outcome: Deferred by design until a named target requires it._

**D-07 — Treat runtime resource identifiers as logical IDs, not arbitrary filesystem paths.** Schema-controlled dictionary/import/pack/vocabulary IDs are validated (rejecting drive prefixes, backslashes, traversal) before any runtime data path is constructed; explicit user-supplied import/export/restore file paths remain arbitrary paths, but the derived logical names joined into runtime roots are validated. Boundaries fail closed (FALSE, -1, None, Value::Null).

**D-08 — Source `.dict.yaml` support is not sufficient for production-scale compatibility.** Compiled `.table.bin`, `.prism.bin`, and `.reverse.bin` payload consumption and rebuild execution remain a required direction.

**D-09 — The CLI frontend is a validation surrogate, not native UI.** It drives `yune-rime-api` to validate the ABI; it is not proof that Squirrel, Weasel, ibus-rime, fcitx-rime, or fcitx5-rime integration is complete, and it is not a new graphical end-user frontend.

**D-10 — Cloud inference is never a required runtime dependency.** Classic input and the first AI-native layer must remain local-first and low-latency; remote LLM use is an optional enhancement only.

**D-11 — Privacy boundaries for context and memory.** Context and memory collection must be opt-in, inspectable, clearable, and disabled for sensitive contexts; remote model calls cannot be required for baseline input.

### Phase 1 — CLI frontend surrogate

**D-P1-1 — Keep existing core-backed `run`/`check` commands; add separate ABI-backed `frontend`/`frontend-check` commands.** The CLI gains an ABI-driven path without disturbing the existing core-backed commands.

**D-P1-2 — Localize unsafe RIME ABI handling to `rime_frontend.rs`.** All ABI functions are acquired through `rime_get_api()`/`RimeApi`; each populated commit/context/status read is paired with its matching free function; an RAII cleanup guard runs `destroy_session`/`cleanup_all_sessions`/`finalize` on success and error paths.

### Phase 2 — Native ABI validation and runtime safety

**D-P2-1 — Validate the Cargo-built cdylib through a dynamic loader.** A `libloading` Cargo integration test resolves `rime_get_api` and drives the returned `RimeApi` function table against the real `yune-rime-api` cdylib; `cargo build -p yune-rime-api` is required before the loader gate because `cargo test` alone does not guarantee the artifact exists. The `Library` value is kept alive for the whole symbol/table scope; the loader fails closed with distinct messages for missing artifact, load error, missing symbol, null table, null function pointer, and runtime failure.

### Phase 3 — Schema pipeline depth

**D-P3-3 — Make unmodeled gears explicit deferrals, not silent no-ops.** `memory` defers to Phase 5 (userdb learning); `poet`/`grammar`, `contextual_translation`, and `unity_table_encoder` defer to Phase 4 (compiled payloads). Each is recorded as a `RemainingGearDeferral` (gear, observed librime role, current Yune behavior, scope decision, target phase) during schema installation — a deterministic no-op, not an ABI-exposed compatibility claim.

### Phase 4 — Compiled dictionary data

**D-P4-1 — Keep compiled binary layout parsing in `yune-core` byte-slice readers; `schema_install.rs` only does validated resource selection.** Valid source-only dictionaries are normal runtime behavior; stale/invalid/unsupported compiled artifacts record fallback diagnostics. Unsupported MARISA/Darts sections fail closed with structured `UnsupportedSection` errors.

**D-P4-3 (project-wide, dated 04-03) — Represent advanced source and compiled dictionary data on `TableDictionary`, not parallel runtime-specific structs.** Use bounded local Yune fixture markers for advanced compiled-payload parity while rejecting unsupported librime sections structurally; carry compiled reverse `dict_settings` into runtime table dictionaries so `ReverseLookupTranslator` observes source and compiled settings through the same API. LevelDB/userdb learning, predictive updates, plugin translators, and AI-native ranking stay out of scope.

**D-P4-4 (project-wide, dated 04-04) — Represent correction/tolerance data as normalized `TableDictionary` metadata merged through compiled/source paths.** Lookup expansion preserves original input first, then correction canonicalization, then tolerance candidates. Correction/tolerance parser counts are capped before allocation; malformed compiled sections fall back with structured diagnostics (`YUNE-CORR`/`YUNE-TOL` fixture markers with checked offsets, lengths, UTF-8, count caps). Tests use local Rust-generated fixture bytes only; no librime compiler is invoked.

### Phase 5 — UserDB and scaling hardening

**D-P5-1 — Use file snapshots and atomic rename semantics instead of a LevelDB dependency.** Exported `RimeLevers*UserDict*` functions stay in `userdb.rs` with storage behavior behind internal userdb modules; legacy plain-text userdb import compatibility is preserved while committing typed c/d/t records.

**D-P5-2 — Classic learning stays commit-driven.** `Engine` emits typed pending userdb events (`take_pending_userdb_learning`) before clearing composition; session persistence consumes them only through normal commit paths. Core userdb is storage-neutral; `yune-rime-api` bridges core events to the store and validates logical dictionary names before storage selection. Schema-selected userdb loading is keyed by the table/script-translator dictionary, not the schema-id fallback, so unrelated schema tests do not inherit persisted learning. `HistoryTranslator`, `CandidateRanker`, `MockAiRanker`, and AI memory are not substitutes for classic userdb learning; userdb candidates enter before optional rankers.

### Phase 6 — Real frontend validation and benchmark

**D-P6-1 — Anchor native frontend validation at the cdylib `RimeApi` boundary with sanitized deterministic traces.** `FrontendHostTrace` captures ordered calls, notifications, free-pairs, stale sessions, and mismatches using logical resource IDs and deterministic event names (no paths, timestamps, PIDs, or pointer addresses); missing required function pointers are represented as blocker-capable mismatch records before any unchecked call.

**D-P6-3 (project-wide, dated 06-03) — Represent Squirrel/macOS validation as a source-modeled `RimeApi` lifecycle fixture plus a documented direct-run blocker**, rather than a mandatory app bundle or input-method registration step. Linux ibus-rime/fcitx-rime validation remains follow-up documentation with safe ABI source-model markers in `native_frontends.rs`, not a required daemon dependency for `cargo test`. Native frontend mismatch capture reuses the Phase 6 host-trace schema and sanitized fixture rules.

**D-P6-4 (project-wide, benchmark/readiness notes) — Use a dependency-free Cargo bench target instead of Criterion** to preserve MSRV safety and avoid unnecessary benchmark infrastructure. BENCH-01/BENCH-02 measurements stay at the `rime_get_api` / `RimeApi` function-table boundary rather than direct `yune-core` calls. AI-native readiness is recorded as **GO WITH CONDITIONS**, based on Phase 6 validation and benchmarks, while keeping providers, rankers, context policy, memory policy, and privacy controls out of scope.

### Phase 7 — WASM build and export contract

**D-P7-1 — Treat `wasm32-unknown-emscripten` as the browser build target, not `wasm-bindgen`.** TypeDuck-Web-style integration needs Emscripten C ABI exports and filesystem/runtime hooks. Document `EXPORTED_FUNCTIONS`/`EXPORTED_RUNTIME_METHODS` as the browser build contract without changing `lib.rs` facade wiring.

**D-P7-3 — Keep the Rust adapter inside `crates/yune-rime-api`; do not create a separate adapter crate** unless planning proves the contract cannot be expressed safely from the existing cdylib.

**D-P7-5 — Document the one-active-process-global-service constraint and host filesystem assumptions.** `yune_typeduck_cleanup` finalizes the process-global RIME service; multiple simultaneous TypeDuck states with different dirs are unsupported by this first contract. Treat missing browser schema/dictionary assets as an init-time failure before starting the service. Phase 7 is a handoff contract: one active process-global service, host-owned MEMFS/IDBFS layout and sync, and deterministic verified-or-blocked build output. Upstream clone/replace testing is Phase 10.

### Phase 8 — TypeScript bridge and runtime package

**D-P8-1 — Keep TypeScript tooling package-local under `packages/yune-typeduck-runtime`.** No root JS app scaffolding. Bind only the canonical 11 `yune_typeduck_*` exports through an injected Emscripten Module interface.

**D-P8-2 — Centralize response-pointer ownership.** `readTypeDuckResponse` frees non-null responses in a `finally` block; `TypeDuckRuntime` keeps `statePtr` private, zeros it on cleanup, and rejects operations after cleanup; keyboard mapping is DOM-free (`keyEventToRimeKey` maps `event.key` to explicit RIME constants, bans `keyCode`).

**D-P8-3 — Make native handled state authoritative and normalize malformed JSON.** `yune_typeduck_response_handled` can override the JSON envelope; malformed JSON becomes a deterministic `TypeDuckResponseError`.

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

**D-P10-10 — Treat startup setOption as a real TypeDuck-Web contract, not an optional convenience.** The upstream app calls `Actions.setOption` during startup and settings application. HR-2 adds `yune_typeduck_set_option`, exposes it through the TypeScript runtime, wires the TypeDuck-Web adapter to it, and records a browser startup smoke where option toggles succeed without adapter/runtime errors. Persistence was later proven by HR-4; paging/deletion and dictionary-comment evidence remain outside this decision.

**D-P10-11 — Browser deploy must preload the workspace-reachable schema set, not only the active schema.** HR-3 showed TypeDuck-Web `deploy()` returned false with real assets because the worker preloaded `jyut6ping3_mobile` and its dictionary but omitted the plain `jyut6ping3.schema.yaml` reached through the real `default.custom.yaml` workspace deployment path. The seam now treats that source schema as an app-owned preload asset and records browser evidence where deploy returns true.

**D-P10-14 — Post-review M9 closeout conditions are evidence rules, not a new verdict.** The delete/backspace runtime-error banner was a real delete-path adapter bug: TypeDuck-Web sends a standalone `{Control_L}` keydown before `{Control+Delete}`, and the adapter was forwarding the pure modifier into Yune. The adapter now treats pure modifier keydowns as pass-through and the HR-5 delete/backspace evidence was recaptured with no banner and zero warning/error entries. Rich dictionary-comment byte parity is clean-checkout reproducible via the committed `cantonese_parity` fixture; the browser-shaped native `typeduck_adapter_real_assets_emit_oracle_dictionary_panel_comments` test asserts the full real-assets path only when local TypeDuck v1.1.2 oracle build assets exist and otherwise emits an explicit skip reason.

### TypeDuck-Windows milestone (project-wide D-15..D-22; completed as profile by M10)

**D-17 / WIN-ORACLE-01 — Comment semantics and Cantonese/Jyutping parity must be driven by TypeDuck-HK/librime v1.1.2 goldens or documented blockers.**

**D-19 / WIN-ORACLE-01 — Pin the v1.1.2 oracle commits.** `TypeDuck-HK/librime` commit `74cb52b78fb2411137a7643f6c8bc6517acfde69`, `rime-dictionary-lookup-filter` commit `3e4605c4fae99f068df2edb85aaeab5a97752795`, and `TypeDuck-HK/schema` commit `1bed1ae6a0ab48055f073774d7dfd152a171c548`.

**D-20 / WIN-COMMENT-01 — Represent TypeDuck-Windows candidate comments as source-row dictionary lookup payloads (`\f\r1,...\r0,...`) through `dictionary_lookup_filter`.** Captured source rows assert byte output against the v1.1.2 fixture. Normal reverse lookup joins use `"; "`, and HR-6 adds dedicated v1.1.2 oracle coverage for that join plus schema-prompt preedit bytes.

**D-22 / WIN-PARITY-01 — The Cantonese/Jyutping parity suite locks captured v1.1.2 schema/menu/comment behavior** and keeps uncaptured option, completion, correction, schema-menu, and userdb-pronunciation behaviors as explicit ignored tests until dedicated oracle fixtures are captured.

### Upstream oracle refresh (project-wide D-24)

**D-24 / ORACLE-PRECEDENCE - Pinned upstream librime 1.17.0 is Yune's default core oracle; TypeDuck v1.1.2 is a compatibility profile.** M12 targets upstream `rime/librime 1.17.0` at commit `33e78140250125871856cdc5b42ddc6a5fcd3cd4`. TypeDuck-HK/librime `v1.1.2` at `74cb52b78fb2411137a7643f6c8bc6517acfde69` remains the oracle only for TypeDuck-Web/Windows profile behavior such as fork-only ABI slots and dictionary-panel/Cantonese fork behavior. When upstream and TypeDuck disagree, core Yune follows upstream; TypeDuck differences must be isolated behind named profile fixtures, tests, adapters, or ABI notes. M12 pinned upstream fixture provenance, added the TypeDuck coverage audit, refreshed default `RimeApi` parity to the pinned `rime/librime 1.17.0` target, and expanded `luna_pinyin` behavioral parity fixtures captured from the official upstream release binary. Active upstream-core gates now cover curated single-code mechanics, full `ni` dictionary selection with essay weights, Engine paging/selection/commit, reverse lookup, punctuation/symbols, and supported option paths. Later M17/M18 milestones closed the `luna_pinyin` sentence/lattice, `ascii_punct` processor bypass, and punctuation immediate commit blockers with fresh upstream fixtures. `start_quick` remains absent from the default table. M12 removed the `config_list_append_*` fork-only slots from the default upstream table. M19 implemented the named TypeDuck profile ABI surface, and M10 later completed the TypeDuck-Windows package/build/frontend smoke through it. The default table remains upstream-shaped.

### Compatibility scope (project-wide D-25)

**D-25 / COMPAT-SCOPE — Compatibility is target-driven, not 100% feature parity with librime.** Yune treats librime as a behavioral oracle (a floor), not a feature target to clone. Success is that a _named_ set of target schemas and frontends behaves correctly against the oracle, every difference measured and either fixed or documented: **(A)** a bounded near-term target set — the `luna_pinyin` core path vs upstream `1.17.0` and the TypeDuck `jyut6ping3` profile vs `v1.1.2` — as the working definition of "done"; and **(B)** a broader common-RIME-schema and real-frontend compatibility ambition, added incrementally through the same oracle-measured parity harness. Bit-for-bit parity with librime internals (every gear, plugin, and code path) is a non-goal; a librime feature is implemented only when a named (A)/(B) target requires it. "The engine is done" is therefore never absolute — it reads "the current target set is green against the oracle; everything else is deferred-and-documented." Extends D-01 (oracle, not template) and D-24 (oracle precedence) with an explicit scope boundary; mirrored in `roadmap.md` ("Scope Ledger"). _Outcome: Good._

**2026-07-15 target-set amendment.** The same target-driven rule now applies to
the completed broader set: upstream `luna_pinyin`, captured common-schema
lanes, canonical Jyutping against upstream librime plus pinned
`rime/rime-cantonese`, and TypeDuck multilingual/comment/profile behavior in
its profile lane. This expands the named targets under the original `(B)`
mechanism; it does not convert librime into a feature checklist.

### AI-native milestone (M11)

**D-M11-1 — Start AI-native with a CLI-only staged-result provider slice.** M11 S1 adds an `AiCandidateProvider` interface and deterministic `MockAiProvider` in `yune-core`, but provider execution is owned by the direct `yune-cli run` path, not by `Engine::refresh_candidates` and not by `yune-rime-api`/browser/Windows frontends. The engine stores only staged, input-keyed `AiResult::Ready` candidates and appends matching AI rows after classic candidates, preserving the top classic candidate. S1 kept `CandidateSource::Ai` as a unit variant with provider/confidence labels in `comment`; D-M11-2 supersedes that shape once the merge policy consumes confidence. Default Space/Return confirmation rejects AI candidates, while explicit AI selection records a commit without staging librime userdb learning. Real frontend AI exposure remains disabled by default until later M11 provider, local-model, context, memory, and privacy gates are proven.

**D-M11-2 — S2 uses a host-owned worker and fixed-point AI confidence metadata.** Provider/model work remains outside `Engine::refresh_candidates`: `AiWorker` runs the provider on a background thread, receives cloned context snapshots from the direct CLI host, and returns input-keyed `AiResult::{Pending, Ready}` so stale results cannot erase or apply to a different input. `CandidateSource::Ai` is promoted to `{ provider, confidence }`, but confidence is stored as fixed point `AiConfidence` rather than `f32` so `CandidateSource` keeps `Eq` and can continue flowing through `UserDbCommitMetadata` and tests. The merge policy still pins all classic rows ahead of AI rows, but now orders AI rows by confidence. This completes the worker/confidence part of AI-02 while leaving local-model, context, memory, privacy, and real-frontend exposure for later gates.

**D-M11-3 — S3 defaults AI context to sensitive and blocks remote providers before invocation.** `Context` now carries `AiContext` metadata (app id, field id, preceding text, and `PrivacyClass`), defaulting to `Sensitive` when the host has not supplied classification. `EngineAiContextProvider` captures a bounded `AiContextSnapshot` with the explicit app/field/text/privacy plus input cursor, schema, and candidate-count data a provider may see. Providers declare `AiProviderKind::{Mock, Local, Remote}`; `AiPrivacyPolicy` allows mock/local providers under sensitive context but blocks remote providers before `provide()` is called and returns an input-keyed `AiResult::Off { reason: Privacy }`. The policy also exposes the learning gate that S4 must apply to `MemoryStore` writes. Real frontend AI exposure remains disabled by default.

**D-M11-4 — S4 routes AI learning into a separate memory store, never librime userdb.** `crates/yune-core/src/ai/memory.rs` owns `MemoryStore`, an inspectable, clearable, disable-able ledger for explicit AI selections. `Engine::commit_candidate` continues to keep `pending_userdb_learning` empty for `CandidateSource::Ai`, but now records standard-context AI commits into `MemoryStore`; sensitive contexts and disabled memory keep the commit path working while suppressing memory writes. The store exports/imports a stable text snapshot for future hosts and exposes `memory_store_file_name` / `memory_store_snapshot_file_name`, which produce `.ai-memory` / `.ai-memory.txt` resources and reject path-like or `*.userdb` logical ids. M9/M10 frontends still do not expose AI.

### AI-native frontend exposure (M13)

**D-26 / M13-WEB-AI — Expose AI in TypeDuck-Web through a default-off, two-pass Rust sidecar, not through the classic key path.** M13 keeps `yune_typeduck_process_key` provider-free and classic-first. The browser worker renders that classic result first, then requests the new `yune_typeduck_stage_ai` sidecar action; the sidecar runs `LocalModelProvider` synchronously in Rust/WASM, stages an input-keyed result, and returns a second response. The JS side carries no provider logic. Source labels are attached from engine snapshot data aligned to the rendered page, not by widening `RimeCandidate` or altering the upstream `RimeApi` table. AI remains default-off, TypeDuck-Web-only, local-only, and toggle-controlled; disabling AI stages an `Off` result for the current input so stale AI rows disappear immediately and classic output is restored. The browser host supplies no app/field context, so the existing sensitive default applies: local providers are allowed, remote providers remain out of scope/blocked by policy, explicit AI commits do not touch librime userdb, and AI-memory learning is suppressed in the default browser context. Real-browser M13 evidence is required alongside native `typeduck_web` and TypeScript runtime gates. _Outcome: Good._

_Current-surface note:_ the live browser contract is now the `yune_web_*`
family and `@yune-ime/yune-web-runtime`. The historical M13 names above record
the decision boundary; they are not current export or package names.

### Architecture hardening (project-wide D-28)

**D-28 / ARCH-HARDENING — Keep the strategic architecture, but name the current core/ABI boundary debt before product/native expansion.** The strategic decisions remain correct: librime is the oracle rather than the template, upstream 1.17.0 is the default core reference, TypeDuck v1.1.2 is a named compatibility profile, and AI stays a separate default-off layer. The structural debt is narrower: the full production key path is currently easiest to drive through the librime-shaped ABI because the RIME processor pipeline lives in `yune-rime-api/src/processors/` and falls through to `yune-core::Engine`. M23 closed the finishable debt around that boundary without doing the larger processor extraction: TypeDuck-calibrated translator tuning is profile/config gated, the workspace lint policy is real for unsafe-free crates with explicit FFI/ABI-client lint exceptions, the orphaned `yune-schema` crate was removed instead of parked, and the oversized test modules were split along behavior ownership lines.

The standing rule remains: TypeDuck-calibrated constants must not affect default upstream core behavior unless installed through an explicit profile predicate / typed translator config, or renamed with upstream-oracle evidence that the behavior is global. M18 later shipped the narrow punctuation processor behavior needed by upstream fixtures in both `yune-core` and the ABI path, but that does not change the larger rule: the full **core/ABI processor extraction stays trigger-gated**. Move processor semantics toward a core-owned Rust API only when a real non-ABI consumer needs the full input pipeline. Every such extraction must be behavior-preserving and keep existing oracle, ABI, and browser gates unchanged. _Outcome: M23 finished the bounded architecture-hardening paydown; the completed plan lives at the M23 architecture-hardening plan (see `docs/history.md`). The processor extraction remains trigger-gated future work rather than a retroactive rewrite trigger._

### Breadth schemas and TypeDuck profile ABI (project-wide D-29)

**D-29 / BREADTH-PROFILE-SURFACE - Add upstream schema breadth only through per-schema oracle fixtures, and expose TypeDuck fork-only ABI through a named non-default accessor.** M19 widens the upstream compatibility set to `double_pinyin`, `cangjie5`, and `bopomofo` by reusing the M12 capture harness against upstream `rime/librime 1.17.0`, with one provenance-stamped fixture and one owning parity test per schema family. The captured active cases cover Shuangpin algebra, Cangjie exact-code table behavior, and Zhuyin keymap/tone algebra; sentence/lattice, broader Cangjie phrase/table-encoder interleave, and schema-speller digit/space key routing remain explicit ignored blockers rather than hidden parity claims.

M19 also names the TypeDuck-Windows ABI delta through `rime_get_typeduck_profile_api()`. The default `rime_get_api()` table remains the upstream `1.17.0` shape; the opt-in profile table exposes `config_list_append_{bool,int,double,string}` for TypeDuck-profile consumers. `start_quick` remains out of surface. TypeDuck-Windows packaging and real frontend E2E were later completed by M10 through this profile surface, without widening the default ABI. Extends D-24 (oracle precedence) and D-25 (target-driven scope). _Outcome: Good._

### Upstream sentence / language model poet (project-wide D-30)

**D-30 / UPSTREAM-POET - Implement `luna_pinyin` sentence/lattice parity through the upstream null-grammar path, not through TypeDuck tuning.** M17 captures `luna-pinyin-sentence.json` and `luna-pinyin-lattice.json` from the pinned upstream `rime/librime 1.17.0` release binary before implementation. The captured schema has `essay.txt` preset vocabulary but no `.gram` model, so Yune implements the grammar-absent poet scoring path in `yune-core`: `entry_weight + kPenalty` per word, with the verbatim upstream `kPenalty = -13.815510557964274`, plus a bounded sentence/lattice candidate path installed only for the upstream `luna_pinyin` script-translator profile. The existing TypeDuck `jyut6ping3` sentence heuristic and its `21.0` word penalty remain profile-gated and separate.

This closes the two M17-owned `luna_pinyin` blockers in `upstream_luna_pinyin_parity.rs` without changing `RimeApi`, `RimeCandidate`, or the TypeDuck-profile ABI surface. M54 later added native octagram-compatible grammar support for a named upstream `luna_pinyin` target; broader `.gram`/octagram behavior, the C++ plugin ABI, and contextual translation stay deferred until a future named target proves they are required. Extends D-24 (oracle precedence), D-25 (target-driven scope), and D-28 (profile-specific tuning isolation). _Outcome: Good._

### Upstream-style Jyutping composition target (project-wide D-31)

**D-31 / JYUTPING-UPSTREAM-COMPOSITION - For long Jyutping composition in the TypeDuck-Web/Yune product surface, prefer an accepted upstream-librime-engine Jyutping fixture over TypeDuck v1.1.2 ranking when TypeDuck v1.1.2 produces worse segmentation.** Upstream `rime/librime 1.17.0` remains the engine oracle, but because upstream does not ship a built-in Jyutping schema, fixtures for this slice run the upstream engine against pinned Jyutping schema/dictionary source YAML and upstream's deployer. The intended ordering is sentence/lattice candidate first, longest valid fuzzy phrase-prefix candidates next, single-character fallback after that, and invalid fuzzy mis-segmentation ranked low. The hybrid upstream-engine fixture is the oracle of record for this named composition/ranking case once captured and accepted, even if it diverges from `typeduck.hk/web`, `my-rime.vercel.app`, or TypeDuck v1.1.2. TypeDuck v1.1.2 remains the compatibility-profile oracle for fork-only ABI/comment/profile behavior and existing M14-M28 fixtures; this hybrid fixture is not a dictionary-comment oracle because stock upstream lacks TypeDuck's `dictionary_lookup_filter` plugin. If the hybrid upstream-engine fixture cannot be captured or accepted, this milestone stops and requires explicit user sign-off for a Yune-authored ranking spec before ranking code changes.

**2026-07-05 amendment - Canonical Jyutping versus TypeDuck multilingual profile.** TypeDuck-HK/schema and `rime/rime-cantonese` both use `schema_id: jyut6ping3`, so schema id alone is not enough provenance. New canonical Cantonese/Jyutping candidate ordering, segmentation, fallback, and completion claims must identify the schema source repository and source commit, and must be captured from upstream `rime/librime 1.17.0` running pinned `rime/rime-cantonese` unless a later explicit decision scopes the claim as TypeDuck-profile-only. TypeDuck-HK/librime v1.1.2 remains profile-only, but the historical M14-M28 TypeDuck fixtures are grandfathered as TypeDuck-profile regression guards, including fixture-backed candidate ordering, composition, prefix-fallback, prediction-count, and partial-selection behavior. In particular, M21-GAP-01 and M21-GAP-02 remain previously authorized TypeDuck-profile candidate claims until a later migration explicitly supersedes them.

M58 completed the schema/profile blast-radius inventory. The future schema
split must make the two schema sources distinguishable by
Yune-facing id, fixture provenance, compiled artifacts, docs, and product
claims. The preferred direction is for the plain `jyut6ping3` id to resolve to
canonical `rime/rime-cantonese` and for TypeDuck multilingual/profile behavior
to use `jyut6ping3_typeduck`, but that id direction is not executable until the
owner gives explicit sign-off and an implementation milestone carries the
recorded userdb/cache-key, product-id, compiled-asset, manifest, threshold, and
profile-predicate migration evidence.

### yune-web public demo readiness (project-wide D-34)

**D-34 / M31-YUNE-WEB-PUBLIC-DEMO - Publish the first public web demo as `yune-web`, with delivery claims separated from engine claims.** M31 makes the TypeDuck-Web-derived harness publishable as a Yune engine demo without treating the separate TypeDuck-Web product repo as the deployed surface. The repo-owned app path, public app name, package, Cloudflare Pages project, evidence, and docs use `yune-web`; the upstream-derived source checkout lives at `apps/yune-web/source/` and the Yune-owned source patch remains reviewable and reverse/forward-checkable. TypeDuck-Web shell and dictionary assets are owner-approved by the project owner, who is also the TypeDuck author.

M31 exposes only browser-honest output standards proven by Yune evidence: Hong Kong Traditional and `hk2s` Simplified through the existing `simplification` option and OpenCC data. Unsupported OpenCC standards remain absent. The public build is reproducible from checked-in Yune state, uses Pages Direct Upload at <https://yune-web.pages.dev>, pins schema assets by SHA-256, prunes non-public schema families, and records local plus deployed Playwright evidence. AI remains default-off/local-only with no remote calls, telemetry, or secrets, and M31 does not add richer AI UX.

M31's performance boundary is narrow: it records delivery/cache readiness, not a browser startup or typing speed win. The public `jyut6ping3_mobile` boot path still includes Luna and scholar lookup assets required by current TypeDuck schema behavior, so full reverse-lookup dependency deferral is a measured future follow-up rather than an M31 claim. No default `RimeApi`, `RimeCandidate`, or upstream ABI surface changes are part of this decision. _Outcome: M31 complete with deployed smoke evidence and a documented delivery caveat._

### M44 native performance owner reduction (project-wide D-41)

All implementations must preserve closed behavior and storage invariants. The M42 abbreviation sentence path stays separate from the M40 full-pinyin sentence lookup path; `hao`/`ni` output must remain upstream-observable; Track A selected storage remains `rsmarisa_byte_backed`; table/prism heap mirrors stay `0`; `source_fallback=false`; first-page output and `RimeGetContext` stay bounded; and the Track B 50+ guard row remains stable. M44 remains native/profile-scoped: no browser, frontend, packaging, deployment, public-demo, or broad product-delivery speed claim may be made from this evidence. Old final evidence remains part of the audit trail, and disk cleanup is a separate maintenance task because `target/` is much larger than the former evidence tree.

### WEB-03 launch compiled-asset contract closeout (project-wide D-46)

**D-46 / WEB03-LAUNCH-COMPILED-ASSET-CONTRACT - Treat regenerated launch
compiled assets and byte-backed storage as a delivery contract, and require
browser evidence before claiming memory success.** The WEB-03 asset slice
regenerates and ships current compiled assets for the three public-demo launch
schemas: `jyut6ping3_mobile` (including `jyut6ping3_scolar` and
`luna_pinyin_yune_reverse`), `cangjie5`, and `luna_pinyin`. The public-demo
manifests, dist, cache bucket, and worker asset lists must stay in sync with
those assets, including Cangjie `.table/.prism/.reverse.bin` payloads.

Two runtime/deploy details are now part of that contract: workspace rebuilds
must load vocabulary `.txt` packs separately from imported `.dict.yaml` packs,
and the launch schemas' compiled dictionaries may select compact byte-backed
storage only when the compiled path loaded successfully. Native diagnostics
must show `source_fallback=false`, no fallback rows,
`selected_storage=byte_backed`, and positive `byte_source_len` for all three
schemas.

Follow-up correctness note: the `d4d84203` memory closeout was valid for the
`160.0 MiB` browser measurement, but it missed a byte-backed Jyutping phrase
composition regression. The compact path must resolve prism aliases for
sentence substrings and prefix fallback probes, and the WEB-03 guard now covers
`ngogokdak -> 我覺得` plus visible `zouhapci` dictionary rows before treating the
launch path as complete.

Latency follow-up note: the phrase-composition repair must also stay bounded.
The compact byte-backed path now caps sentence alias expansion, sentence-span
candidate collection, and prefix-fallback pending rows. Rebuilt local
public-demo evidence records the affected long Jyutping rows at `130 ms` and
`74 ms` while keeping ready/peak WASM memory at `160.0 MiB`.

Deployment-maintenance design (2026-07-15): D-46 remains fail-closed, while a
source-pinned GitHub workflow owns classification, the unchanged local
WEB03-11 certification, run-attempt-named receipts, one deployed preview canary, and promotion
of the identical hashed artifact. Deployment credentials exist only in fresh
upload jobs; they are never present while repository build or test code runs.
Cloudflare Git auto-builds are disabled before the workflow reaches `main`, and
each credentialed upload reasserts that setting through the Pages API; the first
green source-bound run then closes the migration. This changes pipeline placement, not engine behavior,
latency/cadence policy, or the historical closeout evidence.

2026-10-16 amendment: production promotion is automatic again. On 2026-07-19 the
WEB-06 plan's per-milestone rule ("after a green preview, stop for explicit user
approval") was copied into the shared workflow, which made every production
release a manual `production-promotion` dispatch; no decision recorded that, and
the rule now binds only WEB-06-measured bytes, which use the local promotion
script. `auto-promote-yune-web.yml` now dispatches that same promotion once a
push run's preview canary and the Rust checks both pass and the commit is still
current `main`, so every exact-byte, receipt, and current-main check still runs.
The added Rust checks gate stands in for the removed human approval.

**D-47 / M59-DEFAULT-ON-REACHABILITY — Leading-single reachability is an
engine-level default for every schema, and its measured per-keystroke cost gets a
4-row owner-signed injection-on ceiling.** Per the owner amendment (2026-07-07,
recorded in the M59 plan, see `docs/history.md`)
composing an arbitrary non-lexicon phrase one character at a time
(`moboyi`→莫伯洢) must work on **every** schema by default with no per-schema
adaptation. The flip makes `leading_syllable_reachability` default **on** in
`schema_install` (opt out only with an explicit `translator/leading_syllable_reachability: false`);
luna drops its schema flag and inherits the default. It ships with the corrective
mechanism series: the leading-single walk gets an O(1) boundary skip + a range cap
(bounded to the longest syllabary code) so the long-input CPU cost stays under the
gate; a **schema-level precedence fix** stops the injection from firing on jyutping
keystrokes where `prefix_fallback` did not apply per-request (that was a +24 MB
Track B regression and a latent perturbation of the byte-pinned jyutping order —
the fix restores the M58 contract and makes the documented dual-mechanism
precedence real in code); and a bare-syllable call-site gate keeps complete
syllables (`hao`, `mai`) out of the composition machinery.

The flip is the **first honest ratchet measurement with the injection actually on**
— every prior run measured the feature off because the benchmark deployed a luna
without the flag (the finding-#8 hole). A provenance guard now fails the benchmark
if the deployed luna and the shipped web-product luna disagree on the reachability
state. The injection-off M55 ceilings were built as `baseline × 1.05`, a noise
band the feature's real per-keystroke cost partly eats on the rows with the least
headroom. **Initial owner decision (2026-07-08):** re-derive an injection-on ceiling for exactly
`n`, `hao`, and the 37-char row = **pooled-worst of the committed injection-on runs
× 1.05**; the 59-char fourth-row addendum follows below. The other standing rows
are untouched; the injection-off numbers +
per-row `spread_pct` are preserved in `m55-thresholds.csv` as the feature-cost
record. Protocol upgrades landed with it: a `spread_pct` column on every gate row,
and a written **median-of-committed-runs** gate-verdict rule (a row passes iff its
median ≤ its ceiling — any-row-single-run gating gets flakier as rows are added).
The owner-expanded Track A set (`zh, j, yi, che, chuang, b`; words `ceshi,
zhongdengchangdu, dazisudu`) is baselined this round (recorded-not-enforced) and
gates from the next change; `yi`/`che`/`chuang` are bare-syllable-guard sentinels
whose expected feature cost is ≈ 0. Evidence: the `m59-flip-final` packet
(gate-verdict, expanded baselines, README with the m37 decomposition written as
method) and `m59-flipoff-isolation` (the feature-off comparison), both in git
history before the 2026-10 docs consolidation. Extends D-24
(oracle precedence), D-28 (profile-specific tuning isolation), and the M55
corrective ratchet discipline. _Outcome: Good — median gate green 23/23, memory
solved, feature-off record preserved._

_Addendum 2026-07-08 (owner decision): the 59-char row becomes the **4th**
adjusted row, same formula — pooled-worst of the 6 committed feature-on runs
(1.665; the 5 `m59-flip-final` runs + fable's `claude-verify`) × 1.05 = **1.748**;
feature-off record (ceiling 1.625 / source 1.548) preserved in its
`m55-thresholds.csv` notes. It had escaped the first adjustment because its median
(1.634) squeaked 0.25% under the old 1.625 ceiling, but its ~3.8% same-dll spread
made single-run gating a coin-flip (fable's independent 22/23 at `06cd1800`
straddled at 1.647). Only that row moved; the other 19 standing rows remain
untouched._

_Addendum 2026-07-13 (M59-REACH-03):_ commit `5fa986d8` makes the shipped-schema
acceptance surface fail closed. Both manifests cover the exact 60-asset schema
tree; the checked-in registry dispositions all 10 schema assets and three
configuration carriers and links nine executable validation rows. The fixed
eight-row deployment matrix remains distinct from the direct selectable
tracked-`jyut6ping3` real-deploy row. A newly discovered schema asset is added
to the manifest and registry as `status: open`, so it cannot silently inherit an
acceptance claim. The dual `prefix_fallback` / `leading_syllable_reachability`
mechanism and TypeDuck profile precedence remain unchanged.

_Correction 2026-07-15 (M60 runtime formalism):_ that precedence is per input,
not schema-wide. `prefix_fallback` owns only requests with a deployed proper
prefix (`prefix_fallback_owned`); when it does not own the request,
`leading_syllable_reachability` remains independently available. The canonical
[schema-general reachability contract](contracts/schema-general-reachability.md)
records this production-path rule without changing either mechanism or any
candidate behavior.

_Correction 2026-07-16 (post-M60 review):_ "does not own" above has two
distinct causes. A correction-bearing expanded lookup bypasses the ownership
probe; an eligible request reaches the probe and can return `NoPrefix`. The
former may retain a separate normal leading edge on the common
complete/compact-fallback collector, subject to existing path-local correction
guards. The latter cannot itself yield a newly injected leading row because
both mechanisms share deployed normal-prefix discovery. The canonical contract
now records this eligibility boundary; no runtime behavior changed.

**D-48 / PER-LANE-RANKING-PARITY-POLICY — Ranking-order parity is required for
three lanes; the TypeDuck profile lane keeps reachability + comment parity only.**
Owner FINAL decision 2026-07-08, resolving what "parity" means per lane:

1. **Order-parity REQUIRED lanes: canonical rime-cantonese (Lane A), upstream
   `luna_pinyin` (Lane B), and `cangjie5`.** Form = **page/prefix-exact candidate
   text AND order through the captured range** with options mirrored (NOT an
   ordered-subsequence — Yune may not inject extra candidates before/between the
   oracle's). Every `order-only` divergence is classified and returned as a
   **disposition table for owner sign-off — no self-disposition.** Allowed
   dispositions: **fix**, or a **named owner-signed exception** citing the owning
   feature (e.g. an injected single-char family beyond oracle depth).
2. **`cangjie5` onboarding:** first capture librime `cangjie5` over the same dict
   for a named input set including the three owner composition rows
   (`hwmvsqtt`→粵拼, `ebcnyripm`→測試, `takohaeosk`→莫伯洢; product bytes use the
   HK-variant 粵). If librime composes them → oracle-backed order rows; if not →
   record as **owner-provenance beyond-oracle rows** (first-candidate pins) and
   scope cangjie order parity to the librime-comparable range. Wire them into the
   standing test suite either way — this guards M59-REACH-02 / D-47 (the
   schema-general composition guarantee) for cangjie.
3. **TypeDuck profile lane: reachability + comment/profile parity ONLY — no new
   ranking-parity requirements.** Reaffirms product decision #1 and D-31. The
   grandfathered fixture-backed candidate guards (e.g. `beingo` page vs TypeDuck
   v1.1.2) remain as regression pins — **do not extend, do not delete.**
   Multilingual comment behavior stays oracle-backed against TypeDuck `v1.1.2`.

**Owner-signed M59 Increment 4a class-3 exception (renewed 2026-07-11):** the
exception is limited to the complete captured equal-weight residual after the
declared OpenCC normalization: **6,086 inversions, zero cross-weight inversions,
and no beyond-oracle-depth use**. The cause is librime's equal-weight
import/traversal tie-break. Revisit is mandatory if any cross-weight inversion
appears, capture/provenance is incomplete, or a tie residual moves onto page 1
for a common input. This exception did not waive 4b, 4c, 4d, or 4e and is not
consumed by the final strict Lane A capture.

**D-49 / WINDOWS-PRODUCT-REPOSITORY-OWNERSHIP — Transfer the Windows product/frontend track out of this engine repository.** [`CanCLID/yune-windows`](https://github.com/CanCLID/yune-windows) owns P2-WIN-01 execution: TSF input delivery, candidate UI, settings, installer/update, diagnostics, accessibility, Windows product evidence, and its delivery schedule. This repository preserves the superseded P2-WIN-01 handoff record and continues to own the Yune engine/package boundary, upstream-shaped default ABI, named TypeDuck profile API, oracle fixtures, and separately reviewed engine changes requested by the product. Cross-repository work must arrive as a named proposal with pinned package/source provenance and owning tests; Windows product needs do not implicitly widen `rime_get_api()` or reopen the completed P2-WIN-02 boundary fix.

### Initialization notes (process decisions)

**D-DOC-1 — Planning lives in GitHub issues; the docs tree describes only what is true about the system.** Planned and in-progress work is an issue under the pinned `roadmap` issue with parent and blocked-by relationships; completed work is a row in `docs/history.md`; binding rules live in `conventions.md`, `contracts/`, and this file. No plan, roadmap, requirement, or ledger files are added under `docs/` (2026-10 docs consolidation; the former plan-banner rule is in git history).

**D-DOC-2 — Browser-validation claims require committed real-browser, real-asset evidence.** An assertion without a committed artifact does not count as validation — this is why the placeholder-echo WI-4 matrix was reopened (D-P10-9) and why HR-1b committed the real-assets browser run rather than only describing it.

**2026-07-15 evidence-retention amendment.** The durable validation rule is
unchanged, while its storage form is refined: compact manifests, hashes,
summaries, and required fixtures are committed; bulky raw receipts stay in a
recoverable external evidence packet under the retention policy. A committed
compact receipt must identify that source-bound packet. This amendment does not
permit an assertion without recoverable real-browser, real-asset evidence.
