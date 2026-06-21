# Yune Decision Log

This is the consolidated decision log for Yune. The GSD planning system that
previously lived under `.planning/` has been **retired**; this file preserves the
durable decisions and their rationale so that future work does not lose the
reasoning behind the current architecture.

It harvests decisions from the retired `.planning/PROJECT.md` "Key Decisions"
table and Context/Constraints prose, the `.planning/STATE.md` "Accumulated
Context > Decisions" list, and the `decisions:` / `D-NN` entries in the phase
`PLAN`/`SUMMARY`/`RESEARCH`/`CONTEXT` documents.

A note on IDs: the per-phase `D-NN` identifiers used inside `.planning/phases/*`
were **phase-local** (Phase 2, Phase 3, Phase 7, and Phase 10 each restarted at
`D-01`). The numbered `D-12`..`D-23` IDs in this log are the **project-wide**
IDs from `STATE.md` and are preserved verbatim. Earlier, unnumbered project-wide
decisions are assigned `D-01`..`D-11` here in rough chronological order. Phase-local
decisions are folded into the relevant project-wide entry or captured as the
`D-P<phase>-<n>` entries below; they are not renumbered against the project-wide
sequence.

## Standing principles

Cross-cutting decisions that govern all current and future work:

- **librime is the compatibility oracle, not an architecture template.** librime
  defines the externally observable contract (schema semantics, config behavior,
  candidate output, C ABI expectations, deployed-data compatibility, frontend
  integration). The implementation is free to be idiomatic, typed Rust internally
  as long as the boundary contract is preserved. Preserve librime-*observable
  behavior*, not librime's internal C++ complexity.

- **Name the protected behavior before mirroring librime.** Before adding a
  librime-derived mechanism, name the external behavior it preserves
  (user-visible input, frontend ABI, schema/config semantics, or deployed-data
  compatibility). If it only mirrors a librime implementation detail, prefer a
  smaller Yune-native design, isolate it behind an adapter, or document an
  explicit deferral.

- **AI-native input is a separate, local-first, non-blocking, source-labeled
  layer.** LLM assistance is layered as optional candidate providers, rankers,
  context providers, and memory stores with timeout/fallback policy. AI results
  must be source-labeled, optional, and safe to discard, and they must never block
  or slow classic input. Classic input stays predictable and low-latency without
  network access; remote model calls can never be required for baseline input.
  Context/memory collection must be opt-in, inspectable, clearable, and disabled
  for sensitive contexts.

- **Build compatibility fixtures and ABI tests before replacing modules.**
  Behavior must be measurable against librime before any difference can be
  classified as an improvement or a regression. Each new behavior slice names its
  owning implementation module, owning test module, and librime comparison target
  before code changes; new failing comparisons/fixtures come before dispatch or
  state-mutation changes.

- **Module and test ownership per slice; `lib.rs`/`main.rs` stay facades.** Each
  compatibility slice owns its implementation module and test module. `lib.rs`
  (and `main.rs`) remain thin facades plus facade-owned tests; cross-cutting
  behavior tests live under per-crate `tests/` modules. Mechanical refactors
  preserve observable behavior and assertions; they do not rewrite working slices.

- **Plugin ABI compatibility is deferred.** Full C++ librime plugin ABI
  compatibility (Lua, octagram, predict, proto, etc.) is expensive and not yet
  required by a concrete frontend or schema-migration path. Deferred until a real
  distribution requirement makes it necessary.

- **Upstream-first oracle sequencing.** Upstream `rime/librime 1.17.0` is the
  default core compatibility oracle. TypeDuck-HK/librime `v1.1.2` is retained as
  a TypeDuck-Web/Windows compatibility-profile oracle only. If upstream and
  TypeDuck behavior disagree, upstream wins for core Yune; TypeDuck behavior must
  stay behind a named profile test, fixture, adapter, or ABI note.

- **Compatibility is target-driven, not feature-complete.** The oracle is a
  behavioral *floor*, not a feature checklist. "Done" is always relative to a
  *named* set of target schemas and frontends that must behave correctly versus
  the oracle (near-term: the `luna_pinyin` core path and the TypeDuck
  `jyut6ping3` profile), broadening over time toward common RIME schemas and real
  frontends. Reproducing every librime gear, plugin, or code path is an explicit
  **non-goal**; a librime feature is implemented only when a named target needs
  it (see *Name the protected behavior*). The AI-native layer on top of this
  compatible base — not parity for its own sake — is the product goal. See D-25.

## Decision log

### Foundational decisions (from PROJECT.md Key Decisions and Context/Constraints)

**D-01 — Use librime as the compatibility oracle, not an architecture template.**
Existing schemas and frontends depend on librime contracts, but Rust can model the
internals more cleanly. Prefer typed, idiomatic Rust modules over cloning librime's
C++ structure when the boundary contract is preserved. *Outcome: Good.*

**D-02 — Build compatibility fixtures and ABI tests before replacing deeper engine
modules.** Behavior must be measurable before differences can be classified as
improvements or regressions. *Outcome: Good.*

**D-03 — Keep AI ranking optional and local-first.** Classic input must remain
predictable and low-latency without network access. *Outcome: Pending.*

**D-04 — Treat AI-native input as a separate product layer above compatibility.**
librime cannot guide LLM-native behavior, so Yune needs explicit provider, context,
memory, fallback, and privacy contracts. *Outcome: Pending.*

**D-05 — Treat the module/test refactor as a structural rule for future feature
work.** Large single-file accumulation slowed review, search, focused testing, and
extraction; module/test ownership per slice with `lib.rs`/`main.rs` as facades is
now the rule. *Outcome: Pending.*

**D-06 — Keep plugin ABI compatibility deferred.** Plugin compatibility is
expensive and not yet required by a concrete frontend or schema-migration path.
*Outcome: Pending.*

**D-07 — Treat runtime resource identifiers as logical IDs, not arbitrary
filesystem paths.** Schema-controlled dictionary/import/pack/vocabulary IDs are
validated (rejecting drive prefixes, backslashes, traversal) before any runtime
data path is constructed; explicit user-supplied import/export/restore file paths
remain arbitrary paths, but the derived logical names joined into runtime roots are
validated. Boundaries fail closed (FALSE, -1, None, Value::Null).

**D-08 — Source `.dict.yaml` support is not sufficient for production-scale
compatibility.** Compiled `.table.bin`, `.prism.bin`, and `.reverse.bin` payload
consumption and rebuild execution remain a required direction.

**D-09 — The CLI frontend is a validation surrogate, not native UI.** It drives
`yune-rime-api` to validate the ABI; it is not proof that Squirrel, Weasel,
ibus-rime, fcitx-rime, or fcitx5-rime integration is complete, and it is not a new
graphical end-user frontend.

**D-10 — Cloud inference is never a required runtime dependency.** Classic input and
the first AI-native layer must remain local-first and low-latency; remote LLM use is
an optional enhancement only.

**D-11 — Privacy boundaries for context and memory.** Context and memory collection
must be opt-in, inspectable, clearable, and disabled for sensitive contexts; remote
model calls cannot be required for baseline input.

### Phase 1 — CLI frontend surrogate

**D-P1-1 — Keep existing core-backed `run`/`check` commands; add separate
ABI-backed `frontend`/`frontend-check` commands.** The CLI gains an ABI-driven path
without disturbing the existing core-backed commands.

**D-P1-2 — Localize unsafe RIME ABI handling to `rime_frontend.rs`.** All ABI
functions are acquired through `rime_get_api()`/`RimeApi`; each populated
commit/context/status read is paired with its matching free function; an RAII
cleanup guard runs `destroy_session`/`cleanup_all_sessions`/`finalize` on success
and error paths.

**D-P1-3 — Move deterministic transcript serialization to `transcript.rs`.**
`FrontendRun::to_json` becomes a compatibility delegation; owned
`FrontendEvent`/`FrontendContext` carry keycode, mask, page metadata, and select
keys/labels so serializers never read raw ABI pointers. Native frontend validation
is marked Phase 2 scope.

### Phase 2 — Native ABI validation and runtime safety

**D-P2-1 — Validate the Cargo-built cdylib through a dynamic loader.** A
`libloading` Cargo integration test resolves `rime_get_api` and drives the returned
`RimeApi` function table against the real `yune-rime-api` cdylib; `cargo build -p
yune-rime-api` is required before the loader gate because `cargo test` alone does
not guarantee the artifact exists. The `Library` value is kept alive for the whole
symbol/table scope; the loader fails closed with distinct messages for missing
artifact, load error, missing symbol, null table, null function pointer, and runtime
failure.

**D-P2-2 — Keep runtime implementation unchanged where lifecycle regressions pass.**
The absence of loader-exposed concurrency defects is recorded as an explicit
lifecycle-safety assertion rather than broadening into multi-threaded frontend
behavior. Notification ordering is asserted with full context/session/type/value
tuples.

**D-P2-3 — Limit Phase 2 fixes to ABI/runtime boundaries.** Schema semantics,
compiled-dictionary behavior, and userdb storage compatibility stay deferred to
later phases and are recorded only as structured findings.

### Phase 3 — Schema pipeline depth

**D-P3-1 — Model non-auto previous-match splitting in `processors/speller.rs`
without `lib.rs` dispatch changes.** The remaining input is preserved without
emitting an unread commit; the splitting stays bounded to one appended spelling-key
path.

**D-P3-2 — Lock existing editor/navigator/selector and chord/punctuation/fallback
behavior with schema-loaded ABI fixtures.** Coverage is added as ABI-facing
regression fixtures where existing owned behavior already matched the focused
visible state.

**D-P3-3 — Make unmodeled gears explicit deferrals, not silent no-ops.** `memory`
defers to Phase 5 (userdb learning); `poet`/`grammar`, `contextual_translation`,
and `unity_table_encoder` defer to Phase 4 (compiled payloads). Each is recorded as
a `RemainingGearDeferral` (gear, observed librime role, current Yune behavior, scope
decision, target phase) during schema installation — a deterministic no-op, not an
ABI-exposed compatibility claim.

**D-P3-4 — Compare against real librime schemas (`luna_pinyin`, `cangjie5`) and
record out-of-phase gaps as structured findings.** Distribution comparisons assert
focused categories (component order, segment tags, generated spellings, OpenCC/filter
behavior, punctuation/fallback, candidate differences); converted differences stay in
the comparison module with owner comments rather than changing production code.

**D-P3-5 — Use schema-loaded ABI fixtures for spelling algebra, correction/tolerance
ranking, and OpenCC filter-chain behavior.** OpenCC assertions cover filter-chain tag
gating and limited built-in maps only; no full OpenCC conversion-data parity is
claimed, and no compiled-payload/LevelDB/plugin/AI-native code is added.

### Phase 4 — Compiled dictionary data

**D-P4-1 — Keep compiled binary layout parsing in `yune-core` byte-slice readers;
`schema_install.rs` only does validated resource selection.** Valid source-only
dictionaries are normal runtime behavior; stale/invalid/unsupported compiled
artifacts record fallback diagnostics. Unsupported MARISA/Darts sections fail closed
with structured `UnsupportedSection` errors.

**D-P4-2 — Compute deployment freshness from source/schema/pack checksums, not
mtimes.** `RimeDictRebuildExecutionReport` statuses for table/prism/reverse artifacts
let deployment tests assert partial rebuild/reuse; rebuild execution stays entirely
in Rust by emitting the local compiled formats the readers accept; generated schema
build metadata is normalized out of prism signatures.

**D-P4-3 (project-wide, dated 04-03) — Represent advanced source and compiled
dictionary data on `TableDictionary`, not parallel runtime-specific structs.** Use
bounded local Yune fixture markers for advanced compiled-payload parity while
rejecting unsupported librime sections structurally; carry compiled reverse
`dict_settings` into runtime table dictionaries so `ReverseLookupTranslator` observes
source and compiled settings through the same API. LevelDB/userdb learning,
predictive updates, plugin translators, and AI-native ranking stay out of scope.

**D-P4-4 (project-wide, dated 04-04) — Represent correction/tolerance data as
normalized `TableDictionary` metadata merged through compiled/source paths.** Lookup
expansion preserves original input first, then correction canonicalization, then
tolerance candidates. Correction/tolerance parser counts are capped before allocation;
malformed compiled sections fall back with structured diagnostics
(`YUNE-CORR`/`YUNE-TOL` fixture markers with checked offsets, lengths, UTF-8, count
caps). Tests use local Rust-generated fixture bytes only; no librime compiler is
invoked.

### Phase 5 — UserDB and scaling hardening

**D-P5-1 — Use file snapshots and atomic rename semantics instead of a LevelDB
dependency.** Exported `RimeLevers*UserDict*` functions stay in `userdb.rs` with
storage behavior behind internal userdb modules; legacy plain-text userdb import
compatibility is preserved while committing typed c/d/t records.

**D-P5-2 — Classic learning stays commit-driven.** `Engine` emits typed pending
userdb events (`take_pending_userdb_learning`) before clearing composition; session
persistence consumes them only through normal commit paths. Core userdb is
storage-neutral; `yune-rime-api` bridges core events to the store and validates
logical dictionary names before storage selection. Schema-selected userdb loading is
keyed by the table/script-translator dictionary, not the schema-id fallback, so
unrelated schema tests do not inherit persisted learning. `HistoryTranslator`,
`CandidateRanker`, `MockAiRanker`, and AI memory are not substitutes for classic
userdb learning; userdb candidates enter before optional rankers.

**D-P5-3 — Test splits follow behavior ownership and stay separate from semantic
changes.** Levers user-dictionary iterator/file-operation tests move to
`yune-rime-api/src/tests/userdb.rs` (owner: userdb lifecycle); engine/translator/
filter behavior tests move out of `lib.rs`; key/dictionary/facade-specific and
already-owned tests stay put until their owning modules are targeted. `yune-core/
src/lib.rs` stays a facade plus facade-owned tests.

### Phase 6 — Real frontend validation and benchmark

**D-P6-1 — Anchor native frontend validation at the cdylib `RimeApi` boundary with
sanitized deterministic traces.** `FrontendHostTrace` captures ordered calls,
notifications, free-pairs, stale sessions, and mismatches using logical resource IDs
and deterministic event names (no paths, timestamps, PIDs, or pointer addresses);
missing required function pointers are represented as blocker-capable mismatch records
before any unchecked call.

**D-P6-2 — Model TypeDuck-Web as a minimized browser/WASM call sequence through
Yune-owned `RimeApi` calls, without vendoring source.** Emscripten worker lifecycle,
IDBFS persistence, and unavailable native dynamic loading are classified as
`browser_wasm_limit` observations rather than Yune ABI failures; the observation is a
sanitized fixture with mismatch classification "match" (no Yune ABI/runtime mismatch
found). The 06-01 host-trace schema is reused rather than inventing target-specific
formats.

**D-P6-3 (project-wide, dated 06-03) — Represent Squirrel/macOS validation as a
source-modeled `RimeApi` lifecycle fixture plus a documented direct-run blocker**,
rather than a mandatory app bundle or input-method registration step. Linux
ibus-rime/fcitx-rime validation remains follow-up documentation with safe ABI
source-model markers in `native_frontends.rs`, not a required daemon dependency for
`cargo test`. Native frontend mismatch capture reuses the Phase 6 host-trace schema
and sanitized fixture rules.

**D-P6-4 (project-wide, benchmark/readiness notes) — Use a dependency-free Cargo
bench target instead of Criterion** to preserve MSRV safety and avoid unnecessary
benchmark infrastructure. BENCH-01/BENCH-02 measurements stay at the `rime_get_api` /
`RimeApi` function-table boundary rather than direct `yune-core` calls. AI-native
readiness is recorded as **GO WITH CONDITIONS**, based on Phase 6 validation and
benchmarks, while keeping providers, rankers, context policy, memory policy, and
privacy controls out of scope.

### Phase 7 — WASM build and export contract

**D-P7-1 — Treat `wasm32-unknown-emscripten` as the browser build target, not
`wasm-bindgen`.** TypeDuck-Web-style integration needs Emscripten C ABI exports and
filesystem/runtime hooks. Document `EXPORTED_FUNCTIONS`/`EXPORTED_RUNTIME_METHODS`
as the browser build contract without changing `lib.rs` facade wiring.

**D-P7-2 — The browser export surface is the seeded `yune_typeduck_*` adapter API.**
init, process-key, select-candidate, delete-candidate, flip-page, deploy, customize,
cleanup, response-json, response-handled, free-response. `scripts/typeduck-exports.txt`
is the canonical non-prefixed export list (one symbol per line, no `RimeApi` symbols).

**D-P7-3 — Keep the Rust adapter inside `crates/yune-rime-api`; do not create a
separate adapter crate** unless planning proves the contract cannot be expressed
safely from the existing cdylib.

**D-P7-4 — Add deterministic native + Emscripten export verification that must not
mutate the `RimeApi` table.** A POSIX shell script verifies native exports (`nm -g`
against the export list, accepting macOS leading underscores) before browser
prerequisite detection; missing `wasm32-unknown-emscripten`/`emcc`/`emar` are
deterministic, actionable blockers only when `cargo test -p yune-rime-api --test
typeduck_web` passes. Native adapter contract tests in
`crates/yune-rime-api/tests/typeduck_web.rs` are the authoritative fallback. The
browser artifact must be a loadable Emscripten main module (`yune-typeduck.js` +
`.wasm`) rather than a bare side-module wasm; the build exports the
`yune_typeduck_*` list plus `ccall`, `cwrap`, `UTF8ToString`, `FS`, and `IDBFS`,
then smokes the module by calling one export and performing one filesystem
write/read.

**D-P7-5 — Document the one-active-process-global-service constraint and host
filesystem assumptions.** `yune_typeduck_cleanup` finalizes the process-global RIME
service; multiple simultaneous TypeDuck states with different dirs are unsupported by
this first contract. Treat missing browser schema/dictionary assets as an init-time
failure before starting the service. Phase 7 is a handoff contract: one active
process-global service, host-owned MEMFS/IDBFS layout and sync, and deterministic
verified-or-blocked build output. Upstream clone/replace testing is Phase 10.

### Phase 8 — TypeScript bridge and runtime package

**D-P8-1 — Keep TypeScript tooling package-local under
`packages/yune-typeduck-runtime`.** No root JS app scaffolding. Bind only the
canonical 11 `yune_typeduck_*` exports through an injected Emscripten Module
interface.

**D-P8-2 — Centralize response-pointer ownership.** `readTypeDuckResponse` frees
non-null responses in a `finally` block; `TypeDuckRuntime` keeps `statePtr` private,
zeros it on cleanup, and rejects operations after cleanup; keyboard mapping is
DOM-free (`keyEventToRimeKey` maps `event.key` to explicit RIME constants, bans
`keyCode`).

**D-P8-3 — Make native handled state authoritative and normalize malformed JSON.**
`yune_typeduck_response_handled` can override the JSON envelope; malformed JSON
becomes a deterministic `TypeDuckResponseError`.

**D-P8-4 — Document `@yune-ime/typeduck-runtime` as repository-owned bridge code,
not a TypeDuck-Web app scaffold.** Keep the low-level C/WASM export contract
alongside wrapper guidance so non-wrapper hosts can follow raw pointer ownership
rules; wrapper callers receive parsed `TypeDuckResponse` objects while raw callers
copy JSON before `yune_typeduck_free_response`.

### Phase 9 — Browser filesystem and persistence

**D-P9-1 — Keep filesystem behind a narrow `TypeDuckFilesystem` interface and require
explicit assets.** Require explicit `dictionaryId` and asset contents rather than
parsing YAML or fabricating fallback assets; mirror all five native preflight paths
before `TypeDuckRuntime.init` (shared default/schema/dict plus build default/schema).
Browser helper code stays DOM-free, network-free, package-local under Vitest fake-fs
tests; logical IDs must match `[A-Za-z0-9_-]+` before virtual-path construction.

**D-P9-2 — Keep persistence orchestration as standalone helpers in `filesystem.ts`.**
Do not modify `TypeDuckRuntime` or native exports. Use
`syncFromPersistenceBeforeInit` before init and `syncToPersistenceAfterMutation`
after deploy/customize/cleanup/userdb boundaries; expose sync direction as
`fromPersistence`/`toPersistence` strings that lock the Emscripten `syncfs` populate
boolean.

**D-P9-3 — Userdb persistence is an explicit host sync boundary
(`syncAfterUserDataChange`).** Current native exports do not expose userdb mutation
notifications, so the host owns the sync boundary. Stale deployed-config recovery is
a deterministic test fixture over existing helpers, not a metadata heuristic; recovery
documentation stays local-first and caller-owned (explicit assets, explicit sync
boundaries, no browser app/network/cache policy).

### Phase 10 — TypeDuck-Web app integration and E2E

**D-P10-1 — Clone upstream TypeDuck-Web to `third_party/typeduck-web/source`
(gitignored), treated as the app under test.** Use a reproducible, auditable checkout
(pinned `https://github.com/TypeDuck-HK/TypeDuck-Web` URL + full commit SHA + setup
command in a lock JSON and README). Inspect and document the existing librime/WASM
seam before any patching, so app-source changes stay distinguishable from Yune adapter
changes.

**D-P10-2 — Keep TypeDuck-Web source changes minimal; route engine calls through the
Phase 8/9 package surface.** Prefer a patch/configuration layer over UI rewrites; the
seam calls `TypeDuckRuntime`, `keyEventToRimeKey`, filesystem preparation, and
persistence sync helpers from `@yune-ime/typeduck-runtime` rather than raw
`yune_typeduck_*` exports. Preserve one active runtime per Emscripten Module with
deterministic cleanup; do not promise multi-instance browser isolation. The
adapter's upstream `RimeResult` mapper must consume the runtime's actual shape:
per-candidate `text`/`comment` plus `context.highlighted`.

**D-P10-3 — Use explicit TypeDuck-Web-owned assets; never fabricate fallback
schema/dictionary data.** Missing or mismatched `default.yaml`/schema/dictionary YAML
remain visible integration failures (grep-gated against fallback/dummy/placeholder
wording). The app seam loads TypeDuck-Web `public/schema` assets by URL, rejects
empty or missing content before init, and may preload extra shared support files
such as OpenCC data without synthesizing substitutes.

**D-P10-4 — Record adapter mismatches before widening Yune.** The web mapper now
uses the runtime-owned response shape (`candidate.text`, `candidate.comment`,
`context.highlighted`) instead of compatibility guesses. Keep documenting the
`setOption` gap as an error rather than implementing a workaround; widen the Yune
adapter only for the smallest proven blocker, documented first.

**D-P10-5 — Real browser validation is required and never silently skipped.** Use a
standalone Playwright spec (upstream has no browser test framework) covering
composition, candidate paging, selection, deletion, commit output, deploy, customize,
and persistence smoke; persistence follows the Phase 9 explicit sync contract
(populate before init, flush after mutation, reload/reinitialize to prove survival).
The patched app seam mounts IDBFS before init and leaves explicit before/after
sync markers for the E2E evidence. Missing browser/local tooling is recorded
reproducibly (command, missing dependency, fallback evidence). Asset configuration
was recorded as an E2E blocker, not a build blocker.

**D-P10-6 — WI-4 browser evidence replaces the old tooling blocker with concrete
behavioral failures.** The TypeDuck-Web app now loads the Yune Emscripten
JS/WASM module in a real browser. Composition, candidate rendering, selection,
commit output, backspace mutation, and customize pass. Candidate paging,
candidate deletion, persistence sync/reload proof, and v1.1.2 dictionary-comment
rendering fail and must be tracked as behavior/runtime gaps, not as
missing-tooling blockers. HR-2 later resolved `setOption`; HR-3 later resolved
deploy false; HR-4 later resolved persistence sync/reload proof.

**D-P10-7 — M9 remains NO-GO for AI-native frontend exposure after WI-5.** The
recommendation is now evidence-based rather than tooling-based: Yune can load
and type through TypeDuck-Web, but real frontends must not be treated as ready
until paging/deletion and v1.1.2 dictionary-comment bytes pass in-browser.
Superseded by D-P10-13 after HR-5/HR-6 closed those evidence gaps.

**D-P10-8 — Close M9 as a validation milestone, not a pass.** M9 is complete
because the browser E2E ran and produced a durable GO/NO-GO recommendation. The
follow-up milestone is Post-M9 TypeDuck-Web hardening; it starts from the WI-4
failure matrix rather than reopening the validation gate itself.

**D-P10-9 — Reopen the TypeDuck-Web E2E gate when evidence used the placeholder
schema.** Post-review inspection found the WI-4 browser matrix exercised Yune's
echo placeholder rather than TypeDuck's real `jyut6ping3_mobile` dictionary, so
candidate paging, deletion, and dictionary-panel comments were not actually
validated. HR-1 fixes the real-asset gate: TypeDuck-Web now loads
`jyut6ping3_mobile`, exports context despite the schema's NUL
`alternative_select_keys` sentinel, and renders real `nei` candidates (`你`,
`呢`, `尼`) in-browser. The recommendation remains NO-GO, and
TYPEDUCK-E2E-03/E2E-04 stay open until the full matrix is re-run against real
assets. This supersedes D-P10-8's "do not reopen the validation gate" stance —
the placeholder-evidence finding justified reopening it. Superseded by D-P10-13
after the full real-assets matrix passed.

**D-P10-10 — Treat startup setOption as a real TypeDuck-Web contract, not an
optional convenience.** The upstream app calls `Actions.setOption` during startup
and settings application. HR-2 adds `yune_typeduck_set_option`, exposes it through
the TypeScript runtime, wires the TypeDuck-Web adapter to it, and records a
browser startup smoke where option toggles succeed without adapter/runtime
errors. Persistence was later proven by HR-4; paging/deletion and
dictionary-comment evidence remain outside this decision.

**D-P10-11 — Browser deploy must preload the workspace-reachable schema set, not
only the active schema.** HR-3 showed TypeDuck-Web `deploy()` returned false with
real assets because the worker preloaded `jyut6ping3_mobile` and its dictionary
but omitted the plain `jyut6ping3.schema.yaml` reached through the real
`default.custom.yaml` workspace deployment path. The seam now treats that source
schema as an app-owned preload asset and records browser evidence where deploy
returns true.

**D-P10-12 — Browser persistence claims require live-worker diagnostics and a
real reload.** HR-4 adds reason-tagged persistence diagnostics around
`syncFromPersistenceBeforeInit`, runtime init, and `syncToPersistenceAfterMutation`
for init, customize, deploy, commit, select, and delete paths. The TypeDuck-Web
worker forwards those diagnostics to the main page under debug mode so browser
evidence can prove ordering without relying on worker-console capture. The
committed `persistence-sync.log` shows a fresh-origin load with no custom config,
startup customize writing `page_size: '6'`, deploy syncing after mutation, and a
real reload restoring `/rime/jyut6ping3_mobile.custom.yaml` before runtime init.
Paging/deletion and dictionary-comment parity remain outside this decision.

**D-P10-13 — HR-7 closes M9 as GO WITH CONDITIONS.** HR-5 reran the browser
matrix against real TypeDuck `jyut6ping3_mobile` assets and proved composition,
candidate list, paging, selection, deletion, Space commit, phrase commit,
deploy, customize, persistence sync, reload survival, and dictionary-panel
comments with zero warning/error console entries. HR-6 locked the shared
reverse-lookup `"; "` joiner and schema-prompt bytes against TypeDuck-HK/librime
v1.1.2. AI-native behavior may proceed only behind the separate M11 gating
policy: disabled by default in real frontends until provider/ranking/privacy
contracts are proven and explicitly enabled.

**D-P10-14 — Post-review M9 closeout conditions are evidence rules, not a
new verdict.** The delete/backspace runtime-error banner was a real delete-path
adapter bug: TypeDuck-Web sends a standalone `{Control_L}` keydown before
`{Control+Delete}`, and the adapter was forwarding the pure modifier into Yune.
The adapter now treats pure modifier keydowns as pass-through and the HR-5
delete/backspace evidence was recaptured with no banner and zero warning/error
entries. Rich dictionary-comment byte parity is clean-checkout reproducible via
the committed `cantonese_parity` fixture; the browser-shaped native
`typeduck_adapter_real_assets_emit_oracle_dictionary_panel_comments` test asserts
the full real-assets path only when local TypeDuck v1.1.2 oracle build assets
exist and otherwise emits an explicit skip reason.

**D-12 / TYPEDUCK-E2E-04 — Final findings separate three blocker classes.**
TypeDuck-Web app/source blockers, Yune adapter/runtime mismatches, and
environment/tooling blockers are reported separately.

**D-13 / TYPEDUCK-E2E-04 — Phase 10 ends with a NO-GO recommendation for AI-native
frontend exposure** due to browser-validation blockers. Strict rubric: lack of
browser evidence prevents GO / GO WITH CONDITIONS. Blockers are bounded
(cargo/rustup/emcc have install paths), not a fundamental seam incompatibility; the
seam patch is structurally sound and the adapter handles mismatches — environment
setup is the gating requirement. Superseded by D-P10-13 after HR-5/HR-6 produced
real-assets browser and oracle evidence.

**D-14 — AI-native scope remains deferred.** AI-native provider calls, candidate
generation, ranking, context, memory, privacy controls, and a new first-party Yune
frontend remain out of scope.

### TypeDuck-Windows milestone (project-wide D-15..D-22; parked by D-24)

**D-15 / WIN-TEST-01 - TypeDuck-Windows native IME was the next tracked
milestone at the time; first unblock Windows test trust before feature work.**
D-24 now parks this milestone behind an explicit TypeDuck profile surface while
preserving the completed package and helper evidence as TypeDuck-profile work.

**D-16 / WIN-ABI-01 — Fork-only config list-append APIs are the first feature
slice** after the Windows baseline, because they need no external oracle.

**D-17 / WIN-ORACLE-01 — Comment semantics and Cantonese/Jyutping parity must be
driven by TypeDuck-HK/librime v1.1.2 goldens or documented blockers.**

**D-18 / WIN-ABI-01 — TypeDuck fork list-append fields are inserted after
`config_list_size` and before `get_input`,** matching the fork `RimeApi` order;
`config_begin_list` stays in its earlier v1.1.2 slot next to `config_begin_map`.
Scalar append values follow the existing string-backed `RimeConfigSet*`
representation.

**D-19 / WIN-ORACLE-01 — Pin the v1.1.2 oracle commits.** `TypeDuck-HK/librime`
commit `74cb52b78fb2411137a7643f6c8bc6517acfde69`, `rime-dictionary-lookup-filter`
commit `3e4605c4fae99f068df2edb85aaeab5a97752795`, and `TypeDuck-HK/schema` commit
`1bed1ae6a0ab48055f073774d7dfd152a171c548`.

**D-20 / WIN-COMMENT-01 — Represent TypeDuck-Windows candidate comments as source-row
dictionary lookup payloads (`\f\r1,...\r0,...`) through `dictionary_lookup_filter`.**
Captured source rows assert byte output against the v1.1.2 fixture. Normal reverse
lookup joins use `"; "`, and HR-6 adds dedicated v1.1.2 oracle coverage for that
join plus schema-prompt preedit bytes.

**D-21 / WIN-BUILD-01 — The native Windows package is produced by
`scripts/package-typeduck-windows.ps1`.** It builds `yune-rime-api` for
`x86_64-pc-windows-msvc`, renames the DLL/import library to `rime.dll`/`rime.lib`,
copies TypeDuck fork headers, and smoke-checks `rime_get_api` plus the
`config_list_append_string` slot. The 2026-06-19 run completed without
`-NoBuild` or `-SkipSmoke`, proving a pre-M12 package smoke but not the full
TypeDuck-Windows frontend lifecycle. After M12, the default `RimeApi` no longer
exposes TypeDuck fork-only slots, so this script is parked and fails fast until a
named TypeDuck profile ABI surface exists and the slot smoke is re-derived.

**D-22 / WIN-PARITY-01 — The Cantonese/Jyutping parity suite locks captured v1.1.2
schema/menu/comment behavior** and keeps uncaptured option, completion, correction,
schema-menu, and userdb-pronunciation behaviors as explicit ignored tests until
dedicated oracle fixtures are captured.

2026-06-21 update: M10 resume supersedes the "script is parked" part of D-21.
`scripts/package-typeduck-windows.ps1` now builds the current Windows package
with upstream-shaped default headers plus `rime_typeduck_profile_api.h`, rejects
fork-shaped default headers, and smoke-checks the packaged DLL through
`rime_get_typeduck_profile_api()`. That reaches T2. A later T1 probe found
Visual Studio 2022 Community, built local Boost, and got the x64
TypeDuck-Windows solution compiling against the Yune package, but it stops on
missing ATL/MFC headers (`atlbase.h`, `afxres.h`). T1/T3 remain blocked until
the TypeDuck-Windows checkout can build/link and run real frontend smoke against
the Yune package.

2026-06-21 update: M14-M21 promoted the captured TypeDuck engine fixtures to
active `cantonese_parity` tests, including options, completion, correction,
prediction, and sentence/profile tuning. Schema-menu and userdb-pronunciation
observations remain native/frontend T3 evidence limits, not default ABI changes.

### Web-first re-sequencing (project-wide D-23)

**D-23 / SEQUENCING — Re-sequence to web-first.** Validate Yune in a real web browser
(reopened as Phase 17) before resuming TypeDuck-Windows platform work. Phase 10's
NO-GO reflected absent browser evidence (the WASM artifact was never built), not a
failed seam. HR-7 closes that browser gate as GO WITH CONDITIONS. Shared engine
slices (comment shaping, Cantonese goldens, baseline fix) were reused by the web
path. D-24 supersedes the post-M9 priority order by parking TypeDuck-Windows
behind the upstream oracle refresh.

### Upstream oracle refresh (project-wide D-24)

**D-24 / ORACLE-PRECEDENCE - Upstream latest stable is Yune's default core
oracle; TypeDuck v1.1.2 is a compatibility profile.** M12 targets upstream
`rime/librime 1.17.0` at commit `33e78140250125871856cdc5b42ddc6a5fcd3cd4`.
TypeDuck-HK/librime `v1.1.2` at
`74cb52b78fb2411137a7643f6c8bc6517acfde69` remains the oracle only for
TypeDuck-Web/Windows profile behavior such as fork-only ABI slots and
dictionary-panel/Cantonese fork behavior. When upstream and TypeDuck disagree,
core Yune follows upstream; TypeDuck differences must be isolated behind named
profile fixtures, tests, adapters, or ABI notes. M12 pinned upstream fixture
provenance, added the TypeDuck coverage audit, refreshed default `RimeApi`
parity to the pinned `rime/librime 1.17.0` target, and expanded
`luna_pinyin` behavioral parity fixtures captured from the official upstream
release binary. Active upstream-core gates now cover curated single-code
mechanics, full `ni` dictionary selection with essay weights, Engine
paging/selection/commit, reverse lookup, punctuation/symbols, and supported
option paths. Later M17/M18 milestones closed the `luna_pinyin`
sentence/lattice, `ascii_punct` processor bypass, and punctuation immediate
commit blockers with fresh upstream fixtures. `start_quick` remains absent from
the default table. M12 removed the `config_list_append_*` fork-only slots from
the default upstream table. M10 is therefore parked until a named TypeDuck
profile ABI surface is implemented.

### Compatibility scope (project-wide D-25)

**D-25 / COMPAT-SCOPE — Compatibility is target-driven, not 100% feature parity
with librime.** Yune treats librime as a behavioral oracle (a floor), not a
feature target to clone. Success is that a *named* set of target schemas and
frontends behaves correctly against the oracle, every difference measured and
either fixed or documented: **(A)** a bounded near-term target set — the
`luna_pinyin` core path vs upstream `1.17.0` and the TypeDuck `jyut6ping3`
profile vs `v1.1.2` — as the working definition of "done"; and **(B)** a broader
common-RIME-schema and real-frontend compatibility ambition, added incrementally
through the same oracle-measured parity harness. Bit-for-bit parity with librime
internals (every gear, plugin, and code path) is a non-goal; a librime feature is
implemented only when a named (A)/(B) target requires it. "The engine is done" is
therefore never absolute — it reads "the current target set is green against the
oracle; everything else is deferred-and-documented." Extends D-01 (oracle, not
template) and D-24 (oracle precedence) with an explicit scope boundary; mirrored
in `roadmap.md` ("Compatibility goal" + scope ledger). *Outcome: Good.*

### AI-native milestone (M11)

**D-M11-1 — Start AI-native with a CLI-only staged-result provider slice.** M11 S1
adds an `AiCandidateProvider` interface and deterministic `MockAiProvider` in
`yune-core`, but provider execution is owned by the direct `yune-cli run` path,
not by `Engine::refresh_candidates` and not by `yune-rime-api`/browser/Windows
frontends. The engine stores only staged, input-keyed `AiResult::Ready`
candidates and appends matching AI rows after classic candidates, preserving the
top classic candidate. S1 kept `CandidateSource::Ai` as a unit variant with
provider/confidence labels in `comment`; D-M11-2 supersedes that shape once the
merge policy consumes confidence. Default Space/Return confirmation rejects AI
candidates, while explicit AI selection records a commit without staging librime
userdb learning. Real frontend AI exposure remains disabled by default until
later M11 provider, local-model, context, memory, and privacy gates are proven.

**D-M11-2 — S2 uses a host-owned worker and fixed-point AI confidence metadata.**
Provider/model work remains outside `Engine::refresh_candidates`: `AiWorker`
runs the provider on a background thread, receives cloned context snapshots from
the direct CLI host, and returns input-keyed `AiResult::{Pending, Ready}` so
stale results cannot erase or apply to a different input. `CandidateSource::Ai`
is promoted to `{ provider, confidence }`, but confidence is stored as fixed
point `AiConfidence` rather than `f32` so `CandidateSource` keeps `Eq` and can
continue flowing through `UserDbCommitMetadata` and tests. The merge policy still
pins all classic rows ahead of AI rows, but now orders AI rows by confidence.
This completes the worker/confidence part of AI-02 while leaving local-model,
context, memory, privacy, and real-frontend exposure for later gates.

**D-M11-3 — S3 defaults AI context to sensitive and blocks remote providers before
invocation.** `Context` now carries `AiContext` metadata (app id, field id,
preceding text, and `PrivacyClass`), defaulting to `Sensitive` when the host has
not supplied classification. `EngineAiContextProvider` captures a bounded
`AiContextSnapshot` with the explicit app/field/text/privacy plus input cursor,
schema, and candidate-count data a provider may see. Providers declare
`AiProviderKind::{Mock, Local, Remote}`; `AiPrivacyPolicy` allows mock/local
providers under sensitive context but blocks remote providers before
`provide()` is called and returns an input-keyed `AiResult::Off { reason:
Privacy }`. The policy also exposes the learning gate that S4 must apply to
`MemoryStore` writes. Real frontend AI exposure remains disabled by default.

**D-M11-4 — S4 routes AI learning into a separate memory store, never librime
userdb.** `crates/yune-core/src/ai/memory.rs` owns `MemoryStore`, an inspectable,
clearable, disable-able ledger for explicit AI selections. `Engine::commit_candidate`
continues to keep `pending_userdb_learning` empty for `CandidateSource::Ai`, but
now records standard-context AI commits into `MemoryStore`; sensitive contexts
and disabled memory keep the commit path working while suppressing memory writes.
The store exports/imports a stable text snapshot for future hosts and exposes
`memory_store_file_name` / `memory_store_snapshot_file_name`, which produce
`.ai-memory` / `.ai-memory.txt` resources and reject path-like or `*.userdb`
logical ids. M9/M10 frontends still do not expose AI.

**D-M11-5 — S5 proves local AI through the direct CLI only.**
`LocalModelProvider` is a deterministic, local `AiCandidateProvider` with
rule-backed/contextual completions plus optional `MemoryStore` suggestions. It
declares `AiProviderKind::Local`, runs through the existing `AiWorker`, returns
input-keyed candidates with fixed-point confidence, and obeys deterministic
zero-budget fallback. The direct CLI accepts `--ai-provider local` and records
the same `ai_decision` field used by the mock provider; the ABI-backed
`frontend` command, TypeDuck-Web, and Windows surfaces remain AI-free.

### AI-native frontend exposure (M13)

**D-26 / M13-WEB-AI — Expose AI in TypeDuck-Web through a default-off, two-pass
Rust sidecar, not through the classic key path.** M13 keeps
`yune_typeduck_process_key` provider-free and classic-first. The browser worker
renders that classic result first, then requests the new
`yune_typeduck_stage_ai` sidecar action; the sidecar runs `LocalModelProvider`
synchronously in Rust/WASM, stages an input-keyed result, and returns a second
response. The JS side carries no provider logic. Source labels are attached from
engine snapshot data aligned to the rendered page, not by widening
`RimeCandidate` or altering the upstream `RimeApi` table. AI remains default-off,
TypeDuck-Web-only, local-only, and toggle-controlled; disabling AI stages an
`Off` result for the current input so stale AI rows disappear immediately and
classic output is restored. The browser host supplies no app/field context, so
the existing sensitive default applies: local providers are allowed, remote
providers remain out of scope/blocked by policy, explicit AI commits do not
touch librime userdb, and AI-memory learning is suppressed in the default browser
context. Real-browser M13 evidence is required alongside native `typeduck_web`
and TypeScript runtime gates. *Outcome: Good.*

### TypeDuck-Web fork parity (project-wide D-27)

**D-27 / TYPEDUCK-PARITY — Complete the TypeDuck `jyut6ping3` target (M14–M16) as
the next arc; it needs golden capture + dictionary-driven features, not the
upstream language model.** Reaching "open TypeDuck-Web and get fork-like Cantonese
behavior" is scheduled as **M14** (complete: captured the v1.1.2 Cantonese
goldens by parameterizing the scenario-capable upstream probe's oracle identity
and adding a thin v1.1.2 wrapper), **M15** (complete: implemented the dictionary-driven
behaviors: `combine_candidates`, `show_full_code`, `enable_sentence`, completion,
correction, checked-in `hk2s` OpenCC source-data path), and **M16** (complete: real
TypeDuck-Web Playwright evidence covers the app-exposed `jyut6ping3_mobile` surface
plus M13 AI, with deploy-only/UI/userdb inspection limits explicitly listed).
**Gap-map finding:** `jyut6ping3`
uses `script_translator` + `dictionary_lookup_filter` + `simplifier`, not
`poet`/`octagram`/grammar — so the upstream statistical language model is **Track 2
/ M17**, not on the TypeDuck-parity critical path. Per-entry userdb pronunciations
are not exposed by the standard RIME C ABI, but M14 proved the levers user-dict
export hook can capture learned per-entry `(code, freq)` rows from seeded state.
Track 2 (M17 upstream LM, M18 deployment/processor depth, M19
breadth) resumes opportunistically after the TypeDuck-Web closeout. Extends D-24 (oracle
precedence) and D-25 (target-driven scope). *Outcome: M14-M16 complete with explicit
browser/userdb inspection limits.*

### Architecture hardening (project-wide D-28)

**D-28 / ARCH-HARDENING — Keep the strategic architecture, but name the current
core/ABI boundary debt before product/native expansion.** The strategic decisions
remain correct: librime is the oracle rather than the template, upstream 1.17.0
is the default core reference, TypeDuck v1.1.2 is a named compatibility profile,
and AI stays a separate default-off layer. The structural debt is narrower: the
full production key path is currently easiest to drive through the
librime-shaped ABI because the RIME processor pipeline lives in
`yune-rime-api/src/processors/` and falls through to `yune-core::Engine`. M23
closed the finishable debt around that boundary without doing the larger
processor extraction: TypeDuck-calibrated translator tuning is profile/config
gated, the workspace lint policy is real for unsafe-free crates with explicit
FFI/ABI-client lint exceptions, the orphaned `yune-schema` crate was removed
instead of parked, and the oversized test modules were split along behavior
ownership lines.

The standing rule remains: TypeDuck-calibrated constants must not affect default
upstream core behavior unless installed through an explicit profile predicate /
typed translator config, or renamed with upstream-oracle evidence that the
behavior is global. M18 later shipped the narrow punctuation processor behavior
needed by upstream fixtures in both `yune-core` and the ABI path, but that does
not change the larger rule: the full **core/ABI processor extraction stays
trigger-gated**. Move processor semantics toward a core-owned Rust API only when
a real non-ABI consumer needs the full input pipeline. Every such extraction must
be behavior-preserving and keep existing oracle, ABI, and browser gates unchanged.
*Outcome: M23 finished the bounded architecture-hardening paydown; the completed
plan is archived at
[`plans/archive/m23-plan-architecture-hardening.md`](./plans/archive/m23-plan-architecture-hardening.md).
The processor extraction remains trigger-gated future work rather than a
retroactive rewrite trigger.*

### Breadth schemas and TypeDuck profile ABI (project-wide D-29)

**D-29 / BREADTH-PROFILE-SURFACE - Add upstream schema breadth only through
per-schema oracle fixtures, and expose TypeDuck fork-only ABI through a named
non-default accessor.** M19 widens the upstream compatibility set to
`double_pinyin`, `cangjie5`, and `bopomofo` by reusing the M12 capture harness
against upstream `rime/librime 1.17.0`, with one provenance-stamped fixture and
one owning parity test per schema family. The captured active cases cover
Shuangpin algebra, Cangjie exact-code table behavior, and Zhuyin keymap/tone
algebra; sentence/lattice, broader Cangjie phrase/table-encoder interleave, and
schema-speller digit/space key routing remain explicit ignored blockers rather
than hidden parity claims.

M19 also names the parked TypeDuck-Windows ABI delta through
`rime_get_typeduck_profile_api()`. The default `rime_get_api()` table remains the
upstream `1.17.0` shape; the opt-in profile table exposes
`config_list_append_{bool,int,double,string}` for TypeDuck-profile consumers.
`start_quick`, TypeDuck-Windows packaging, and real TypeDuck-Windows frontend E2E
remain out of surface until separately resumed. Extends D-24 (oracle precedence)
and D-25 (target-driven scope). *Outcome: Good.*

### Upstream sentence / language model poet (project-wide D-30)

**D-30 / UPSTREAM-POET - Implement `luna_pinyin` sentence/lattice parity through
the upstream null-grammar path, not through TypeDuck tuning.** M17 captures
`luna-pinyin-sentence.json` and `luna-pinyin-lattice.json` from the pinned
upstream `rime/librime 1.17.0` release binary before implementation. The
captured schema has `essay.txt` preset vocabulary but no `.gram` model, so Yune
implements the grammar-absent poet scoring path in `yune-core`: `entry_weight +
kPenalty` per word, with the verbatim upstream `kPenalty =
-13.815510557964274`, plus a bounded sentence/lattice candidate path installed
only for the upstream `luna_pinyin` script-translator profile. The existing
TypeDuck `jyut6ping3` sentence heuristic and its `21.0` word penalty remain
profile-gated and separate.

This closes the two M17-owned `luna_pinyin` blockers in
`upstream_luna_pinyin_parity.rs` without changing `RimeApi`, `RimeCandidate`, or
the TypeDuck-profile ABI surface. Learned `.gram`/octagram models, the C++
plugin ABI, and broader contextual translation stay deferred until a future
named target proves they are required. Extends D-24 (oracle precedence), D-25
(target-driven scope), and D-28 (profile-specific tuning isolation). *Outcome:
Good.*

### Initialization notes (process decisions)

**D-INIT-1 - Existing `docs/plans/archive/m00-analysis-founding.md`, `docs/roadmap.md`, and
`docs/plans/archive/m05-m07-record-foundation-refactor.md` are the retained source context** for the
now-retired GSD project. Historical `.planning/codebase/` notes were folded into
the retained docs before `.planning/` was removed. External research was skipped
at setup because scope was driven by existing docs and direct librime comparison.

**D-INIT-2 — Future compatibility slices must choose module ownership, test
ownership, and the librime comparison target before implementation.** (Captured as a
standing principle; see "Standing principles" and D-05.)

**D-INIT-3 — Retire the GSD (get-shit-done) planning system.** The upstream GSD tool
was abandoned/rug-pulled, so `.planning/` was removed and its durable content
consolidated under `docs/`: `decisions.md` (this log), `requirements.md` (requirement
tracker), `CONVENTIONS.md` (architecture/stack/conventions/testing reference), and
`roadmap.md` (milestone map through M12). No code change - the engine never depended on
`.planning/`. Rationale: the planning content is durable and worth keeping, but the
tool itself is dead and unsafe to keep installed. Outcome: Good.

**D-DOC-1 — Every doc under `docs/plans/` (and `archive/`) opens with a status banner
whose `Milestone` field names its owning milestone/stage**, kept separate from the
`Status` field. Rationale: a plan's scope and state are visible at a glance and
`grep`-auditable (`grep -rn "Status:" docs/plans`); banner milestones stay consistent
with `roadmap.md`. Finished/superseded plans move to `docs/plans/archive/`, never
deleted. See CONVENTIONS §10.

**D-DOC-2 — Browser-validation claims require committed real-browser, real-asset
evidence.** An assertion without a committed artifact does not count as validation —
this is why the placeholder-echo WI-4 matrix was reopened (D-P10-9) and why HR-1b
committed the real-assets browser run rather than only describing it.

---
*Last updated: 2026-06-21 - D-30 records M17 upstream `luna_pinyin` null-grammar poet completion and keeps learned `.gram`/octagram deferred; D-29 records M19 schema breadth and the named TypeDuck-profile ABI accessor; D-28 records M23 architecture hardening as finished and M18's narrow punctuation processor slice as landed while keeping full processor extraction trigger-gated; D-26 records M13 default-off two-pass TypeDuck-Web AI exposure; D-27 records M14-M16 TypeDuck-Web fork-parity closeout with explicit browser/userdb limits; M10 TypeDuck-Windows now has current profile package/header smoke and packaged-DLL host-loader lifecycle coverage, but remains blocked on the real TypeDuck-Windows build/link and frontend smoke tiers because the local Windows build tools are unavailable.*
