# M19 — Breadth (Shuangpin, Cangjie, Zhuyin) + Named TypeDuck-Profile ABI Surface Implementation Plan

> **Status:** Finished · **Milestone:** M19 (Breadth toward goal B) · **Closed:** 2026-06-21 · **Type:** execution plan
> **For agentic workers:** implemented task-by-task; steps use checkbox (`- [x]`) syntax. Capture oracle goldens *before* writing or replacing any module (fixtures-before-module-replacement).

**Goal:** Widen Yune's *named* compatibility set (goal **B** in [`roadmap.md`](../../roadmap.md)) by onboarding three common upstream schemas — **Shuangpin** (double-pinyin), **Cangjie**, and **Zhuyin/bopomofo** — each measured against the upstream `rime/librime 1.17.0` oracle through the M12 parity harness; and **name the TypeDuck-profile ABI surface** (the fork-only `config_list_append_*` table slots, plus the `start_quick`/levers expectations) needed to unblock the parked M10 TypeDuck-Windows backend, *without* reopening M10 packaging.

**Architecture:** Reuse the M12 harness verbatim as the per-schema template: capture goldens from the official upstream `1.17.0` MSVC release binary via `scripts/oracle-rime-probe.cs`, compose provenance-stamped fixtures under `crates/yune-core/tests/fixtures/upstream-1.17.0/`, guard provenance in `oracle_fixture_provenance.rs`, and add one owning parity test per schema modeled on `upstream_luna_pinyin_parity.rs`. Each schema is an *own-each-slice* unit: an owning fixture, an owning parity test, an explicit non-circular oracle target. Schema behavior is built on existing primitives — the `SpellingAlgebra` (`crates/yune-core/src/spelling_algebra.rs`, supports `xform`/`xlit`/`derive`/`fuzz`/`abbrev`/`erase` at lines 120-133), the `StaticTableTranslator` with `with_spelling_algebra`/`with_show_full_code`/`with_affix` (`crates/yune-core/src/translator/mod.rs:313`, `:917`), and the schema-driven `SpellerProcessor` (`crates/yune-rime-api/src/processors/speller.rs:10` reads `alphabet`/`initials`/`finals`/`delimiter`/`max_code_length`/`auto_select`). For the ABI surface, expose the already-implemented but unwired `RimeConfigListAppend{String,Bool,Int,Double}` symbols (`crates/yune-rime-api/src/config_api.rs:425-485`) through a **named, opt-in profile table variant** — never by mutating the default upstream `rime_get_api()` table built in `crates/yune-rime-api/src/api_table.rs:63-165`.

**Tech Stack:** Rust `yune-core` / `yune-rime-api`; C# `oracle-rime-probe.cs` + PowerShell capture wrappers (`scripts/capture-upstream-luna-pinyin.ps1` template); upstream `1.17.0` Windows MSVC release binary as behavioral-capture source; `serde_json` fixture format.

**Closeout:** M19 landed the reusable `scripts/capture-upstream-schema.ps1`
recipe, provenance-stamped upstream fixtures and owning parity tests for
`double_pinyin`, `cangjie5`, and `bopomofo`, the minimal
`SpellingAlgebra` replacement-normalization fix required by captured Shuangpin
rules, and the opt-in `rime_get_typeduck_profile_api()` surface. Cangjie
phrase/table-encoder/no-match behavior, Zhuyin digit/space key-event routing,
and all sentence/lattice language-model behavior remain explicit ignored
blockers rather than M19 parity claims. `start_quick`, Windows packaging, and
real TypeDuck-Windows frontend E2E remain out of surface.

---

## Scope / Non-goals

**In scope.**
- A repeatable **per-schema onboarding recipe** (capture goldens → run through harness → fix gaps under own-each-slice), applied to Shuangpin, Cangjie, and Zhuyin against the `1.17.0` oracle.
- Provenance/manifest extensions so the three new schemas live alongside `luna_pinyin` under `upstream-1.17.0/` with non-circular source provenance enforced by `oracle_fixture_provenance.rs`.
- **Naming** the TypeDuck-profile ABI surface: a documented, opt-in profile table variant that adds the fork-only `config_list_append_*` slots (consumed at 7 sites in the Windows deployer per [`typeduck-windows-backend-requirements.md`](../../typeduck-windows-backend-requirements.md) §1), with a contract doc and a smoke test that the default table is unchanged.

**Non-goals.**
- Schemas no named user/target needs (do not onboard the full RIME catalog; only the three named here).
- Bit-for-bit librime internals (oracle, not template): only match observable candidate/preedit/commit bytes for the captured cases.
- The upstream statistical **language model / lattice** (that is M17). Zhuyin and Shuangpin onboarding is scoped to **single-syllable + exact-code + speller-layout** parity; sentence-LM cases are explicit ignored blockers, identical to the existing `zhongguo` pattern.
- Reopening M10 **packaging** (`scripts/package-typeduck-windows.ps1`), the native `rime.dll` release smoke, or the real TypeDuck-Windows frontend E2E. M19 only *names and exposes* the profile ABI; resuming packaging waits for a later milestone.
- Changing the default `rime_get_api()` table or widening `RimeCandidate` (upstream-first default ABI).

---

## Tasks

### Task 1 — Generalize the capture path into a per-schema recipe

- [x] Read `scripts/capture-upstream-luna-pinyin.ps1` and `scripts/oracle-rime-probe.cs` (`UpstreamIdentity()` at `:279`, `Traits()` at `:299`, `ReadCandidates`/`ReadSchemaList` at `:352`/`:384`); confirm the probe is schema-agnostic except for the hardcoded `luna_pinyin` schema-select and fixture composer in the PS1 wrapper.
- [x] Add a parameterized capture wrapper `scripts/capture-upstream-schema.ps1 -SchemaId <id> -SchemaDataRepo <rime/...> -InputSequence <...>` that drives the existing probe with `UpstreamIdentity()` (distribution `Rime`/`rime`/`1.17.0`, `candidate_has_quality=false`) and writes a fixture stamped with the same `oracle` header `oracle_fixture_provenance.rs` already requires (`engine=rime/librime`, `engine_tag=1.17.0`, `engine_commit=33e78140…`, `release_url`, `capture_date`, `capture_command`, `schema_data`, `schema_data_commit`, `dependency_repositories`).
- [x] Document the recipe as a short "Adding a new upstream schema" section in the fixture-root `README.md` (capture → fixture → provenance test → parity test → fix-gaps), so future breadth schemas follow it mechanically.
- [x] **Acceptance:** Re-capturing the existing `luna_pinyin` basic fixture through the new generalized wrapper produces byte-identical `cases` to `luna-pinyin-basic.json` (proves the generalization did not change capture semantics), and the recipe README documents every required provenance field.

### Task 2 — Shuangpin (double-pinyin) onboarding — *capture goldens first*

- [x] Pin and record the upstream Shuangpin schema data (e.g. `rime/rime-double-pinyin`, schema id `double_pinyin`) commit in the fixture provenance; capture goldens from the `1.17.0` binary for: single-syllable maps where the two-key shuangpin code resolves to the full pinyin syllable (e.g. a `flypy`/`MSPY` mapping the schema actually ships), exact-code candidate ordering with essay weights, and `Page_Down`/select/`{space}` actions (mirror the `luna-pinyin-actions.json` snapshot scenarios).
- [x] Add `upstream-1.17.0/double-pinyin-basic.json` (+ a selection/actions fixture if the action surface differs from luna); extend `oracle_fixture_provenance.rs` with a `double_pinyin` source-row policy branch and a non-circular source-provenance assertion analogous to the `luna-pinyin-*` block (lines 33-61).
- [x] Implement/verify the speller-algebra mapping: confirm `SpellingAlgebra::parse` (`spelling_algebra.rs`) + `StaticTableTranslator::with_spelling_algebra` (`translator/mod.rs:313`) reproduce the shuangpin `xform`/`derive` rules that fold the two-key code onto the canonical pinyin code. Own the slice: if a shuangpin algebra construct (e.g. zero-initial handling, `o`/`e` final aliasing) is not yet expressed, add it to `spelling_algebra.rs` with its own unit test and cite the failing oracle case.
- [x] Wire the `SpellerProcessor` layout: ensure `speller/alphabet`/`initials`/`finals`/`delimiter`/`max_code_length` from the shuangpin `*.schema.yaml` flow through `install_schema_speller_processor` (`speller.rs:10`) and that `expecting_initial` (`speller.rs:175`) honors the shuangpin final set.
- [x] **Acceptance:** A new `upstream_double_pinyin_parity.rs` test drives Yune's real parser → `SpellingAlgebra` → `StaticTableTranslator` → Engine and matches the captured first-page texts + commit preview for every non-sentence case; any sentence-LM case is an explicit `#[ignore]` blocker with a panic message naming the missing surface (same pattern as `zhongguo_phrase_mechanics_parity_is_blocked`).

### Task 3 — Cangjie onboarding (build on existing short/full-code support) — *capture goldens first*

- [x] Pin the upstream Cangjie schema data (`rime/rime-cangjie`, ids `cangjie5`/`cangjie5_express`) commit; capture goldens from the `1.17.0` binary for exact 1–5 letter code lookups. Broader phrase/table-encoder interleave, option-specific full-code comments, `max_code_length` auto-select behavior, and no-match classification are recorded as explicit blockers because the first oracle probes resolved through phrase/wildcard/punctuation surfaces rather than a narrow exact-code oracle slice.
- [x] Add `upstream-1.17.0/cangjie5-basic.json`; reuse the existing `xlit` root-letter formula shape (`xlit|abcde…|日月金木…|`, see the filter test) but capture it from the *upstream* `cangjie5.schema.yaml`, not from the TypeDuck jyut-derived cangjie sub-lookup (keep the upstream `cangjie5` slice distinct from the existing TypeDuck `jyut6ping3` cangjie reverse-lookup path — see `MEMORY.md` "don't conflate the three TypeDuck-Web things"; same conflation hazard applies here).
- [x] Extend `oracle_fixture_provenance.rs` with a `cangjie5` source-row policy branch.
- [x] Confirm `table_translator`-style behavior: `StaticTableTranslator` columns parsing handles the captured shape-table rows (`columns: [text, code, stem]`), and the active parity slice preserves upstream default empty-comment exact-code behavior with `show_full_code` off. Full-code option comments and auto-select stay out of the active claim until separately captured.
- [x] **Acceptance:** `upstream_cangjie_parity.rs` matches captured exact-code candidate texts and default comments through Yune's real dictionary/translator/Engine path; the test asserts the upstream provenance header and the `cangjie5` schema id, with the broader full-page phrase/table-encoder/no-match surface left as an ignored blocker.

### Task 4 — Zhuyin / bopomofo onboarding (speller + layout) — *capture goldens first*

- [x] Pin upstream Zhuyin schema data (`rime/rime-bopomofo`, id `bopomofo`; note it is pinyin-derived via algebra + a bopomofo keyboard `xlit` layer); capture goldens for: single-syllable bopomofo-key input mapped to candidates, the tone-key handling (1/3/4/6 or space mapping per the schema's `speller`), exact-code ordering, and `Page_Down`/select/`{space}` actions.
- [x] Add `upstream-1.17.0/bopomofo-basic.json` (+ actions fixture if needed); extend `oracle_fixture_provenance.rs` with a `bopomofo` source-row policy branch.
- [x] Implement/verify the bopomofo speller layout: the keyboard-to-bopomofo and bopomofo-to-pinyin transforms are `xlit`/`xform` chains; `SpellingAlgebra` reproduces the captured tone-key cases. Digit tone key processing through `process_key_sequence` and first-tone literal Space remain explicit schema-speller blockers because the current core key processor treats digits/space as selection/commit once a menu exists.
- [x] **Acceptance:** `upstream_zhuyin_parity.rs` matches captured first-page texts + commit preview through Yune's real translator/Engine set-input path for exact-code tone cases; sentence-LM and tone-key event-routing cases are explicit `#[ignore]` blockers.

### Task 5 — Cross-schema breadth gate + provenance count update

- [x] Update `oracle_fixture_provenance.rs::upstream_luna_pinyin_fixtures_have_non_circular_source_provenance` (currently asserts exactly 6 `luna-pinyin-*.json` files, line ~50) — either add parallel `*_fixtures_have_non_circular_source_provenance` tests scoped per schema-prefix, or generalize the file-count guard so each onboarded schema family is independently provenance-checked. Prefer per-schema tests (own-each-slice).
- [x] Add a single breadth roll-up assertion that the four upstream schema families (`luna-pinyin`, `double-pinyin`, `cangjie5`, `bopomofo`) each have ≥1 fixture with the `1.17.0` oracle header and a non-empty source-row policy.
- [x] **Acceptance:** `cargo test -p yune-core --test oracle_fixture_provenance` passes with all four schema families enforced; no fixture contains a local absolute path (`assert_no_local_absolute_paths`).

### Task 6 — Name the TypeDuck-profile ABI surface (unblock M10, no packaging)

- [x] Document the **named profile ABI delta** in a new `docs/plans/m19-reference-typeduck-profile-abi.md`: the default upstream `rime_get_api()` table (`api_table.rs:63-165`) is the `1.17.0` shape and must stay unchanged; the TypeDuck profile adds the fork-only `config_list_append_{string,bool,int,double}` **table slots** (the deployer calls them struct-pointer style via the function table, not as flat symbols — per `typeduck-windows-backend-requirements.md` §1). Record that the implementations already exist as `#[no_mangle]` symbols `RimeConfigListAppend{String,Bool,Int,Double}` in `config_api.rs:425-485` but are **not** in any `RimeApi` field — naming the surface means deciding the table layout, not writing the function bodies.
- [x] Define the profile entry point: a feature-gated or explicitly-named `rime_get_typeduck_profile_api()` (or an extended-table accessor) that returns a superset table with the append slots populated, leaving `rime_get_api()` byte-for-byte the upstream table. Capture the levers/`start_quick` expectation: confirm whether the deployer needs `start_quick` (note: not present in the default table; document it as part of the named surface if the deployer requires it, else record it as out-of-surface).
- [x] Add an owning test `typeduck_profile_abi_surface.rs`: (a) asserts the default `rime_get_api()` table has no append slots / matches the upstream `data_size`; (b) asserts the named profile accessor exposes working `config_list_append_string` against the 7-site deployer pattern (append to a missing list creates it, extends existing list — mirror `config_list_append_creates_and_extends_lists` at `tests/config_api.rs:551`). This is the contract test, not a packaging gate.
- [x] Update [`typeduck-windows-backend-requirements.md`](../../typeduck-windows-backend-requirements.md) status checklist item (1) to point at the named profile surface, and update [`roadmap.md`](../../roadmap.md) Parked/M10 note + Concrete-next-step 5 to reference the named surface as satisfied (packaging still parked).
- [x] **Acceptance:** `cargo test -p yune-rime-api` proves the default table is unchanged AND the named profile table exposes the append slots; the reference doc names every slot the Windows deployer consumes; no default-ABI change ships.

---

## Completion criteria

- [x] Three new upstream schema families (`double_pinyin`, `cangjie5`, `bopomofo`) each have: a provenance-stamped fixture captured from the `1.17.0` binary, a non-circular provenance assertion in `oracle_fixture_provenance.rs`, and an owning parity test that drives Yune's real parser/algebra/translator/Engine and matches the captured bytes for the in-scope (non-LM) cases.
- [x] Every sentence/lattice case that depends on the upstream LM is an explicit `#[ignore]` blocker with a panic message naming the missing surface — no hidden parity claims (matches existing `zhongguo`/`ascii_punct` blocker style).
- [x] The generalized capture recipe reproduces the existing `luna_pinyin` basic fixture byte-identically, and is documented for future breadth schemas.
- [x] The TypeDuck-profile ABI surface is **named** in a reference doc, exposed via a non-default accessor, and proven by a contract test; the default `rime_get_api()` table is byte-for-byte unchanged.
- [x] `roadmap.md`, `typeduck-windows-backend-requirements.md`, and `fork-parity-ledger.md`/`requirements.md` are updated to reflect M19 status.

## Review checklist

- [x] **Oracle non-circularity:** every expected byte comes from the captured upstream `1.17.0` binary (or a pinned upstream schema-data repo commit), never regenerated from Yune. Provenance fields (`engine_commit`, `schema_data_commit`, `dependency_repositories`, `capture_command`) present and checked.
- [x] **Own-each-slice:** each schema behavior has an owning module change + owning test + named oracle case; no shared catch-all test masking a per-schema gap.
- [x] **Upstream-first default ABI:** `rime_get_api()` (`api_table.rs`) and `RimeCandidate` are unchanged; the only ABI additions are behind the explicitly named TypeDuck profile accessor.
- [x] **Name-the-behavior:** no librime feature implemented unless a captured oracle case for one of the three named schemas (or the named TypeDuck profile) requires it; the full RIME schema catalog is not onboarded.
- [x] **Fixtures-before-module-replacement:** goldens captured before any `SpellingAlgebra`/`SpellerProcessor`/translator code is added or changed.
- [x] **No conflation:** the upstream `cangjie5` slice is kept distinct from the existing TypeDuck `jyut6ping3` cangjie reverse-lookup sub-lookup; the named profile ABI work does not reopen M10 packaging or the native `rime.dll` smoke.
- [x] **Heavy/risky flags honored:** Zhuyin/Shuangpin sentence-LM cases deferred to M17 as ignored blockers, not partially implemented.
