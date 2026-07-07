# M60 Schema-General Reachability — Capability-Contract Formalism (follow-up)

> **SUPERSEDED-IN-PART (2026-07-07 owner amendment): the schema-general
> single-character reachability GUARANTEE itself moved into M59.** Per the M59
> plan's "Owner amendment" section, composing an arbitrary non-lexicon phrase one
> character at a time is now **default-ON for EVERY schema (luna, jyut6ping3/
> rime-cantonese, cangjie/shape schemas, and any future schema e.g.
> `rime-teochew`), automatically on install, with ZERO per-schema adaptation
> work** — delivered in M59, not here. **This M60 draft's original premise
> (schema-general is M60-after-M59; a schema may "fail onboarding as unsupported")
> is REJECTED and must not be executed.**

> **For agentic workers:** execute this plan only after M59 has landed the
> default-ON guarantee. M60 is now strictly the *formalism* on top of an
> already-working feature: a documented capability registry and the recorded
> opt-out mechanism (a schema opts out only with an explicit reason). It does NOT
> re-implement reachability and it does NOT permit the reachability feature to
> silently fail onboarding — every schema already has it by M59 default.

> **Status:** Draft, blocked on M59. **Track:** core compatibility formalism.
> **Created:** 2026-07-06; **rescoped:** 2026-07-07. **Type:** contract/registry
> follow-up. No ABI widening, no product schema shipping, no performance claim.

## Goal (rescoped)

M59 makes single-character reachability work on every schema by engine default.
M60's residual job is only to **formalize and document** that: a capability
registry so the default and any explicit opt-outs are auditable, and shape-schema
handling documented against the oracle. The behavior is M59's; the paperwork is
M60's.

## Boundary With M59

M59 OWNS (and delivers): default-ON schema-general single-character reachability
for **all** schemas — luna, jyut6ping3/rime-cantonese, cangjie/shape, and any
future schema — with zero per-schema work; the `prefix_fallback` vs
`leading_syllable_reachability` dual-mechanism resolved; per-schema acceptance
rows; the perf pass. M60 owns only the after-the-fact contract formalism.

--- ORIGINAL DRAFT BELOW (kept for reference; its schema-general-in-M60 and
--- "fail onboarding as unsupported" framing is superseded by the above) ---

M60 starts only after M59 produces:

- green standing M55/Track B ratchet evidence;
- committed M59 oracle captures and diffs;
- focused M59 reachability/selection tests;
- typed configuration replacing the new `jyut6ping3*` schema-string gates, or a
  named residual if M59 deliberately leaves any for M60.

Do not use M60 to re-open or widen M59 while M59 is in progress.

## Design Principle

The generic behavior is not "return every character for every schema." It is:

1. A schema declares, or Yune derives and records, how to identify a **complete
   leading composition unit**.
2. Yune can enumerate every dictionary single-character row matching that
   leading unit through the schema's own code/algebra view.
3. The typed bounded request path may stay budgeted for typing latency, but the
   page-turn/full-list path must not falsely mark the list complete while
   reachable single-character rows remain hidden.
4. Selecting a reachable single-character row commits exactly its span and
   recomposes the remainder through the existing M28-style partial-selection
   semantics.
5. All acceptance behavior comes from upstream oracle captures or an explicit
   per-schema N/A classification. No expected output is derived from Yune.

## Candidate Schema Classes

M60 must classify schemas by capability, not by `schema_id` string.

| Class | Examples | M60 expectation |
| --- | --- | --- |
| Phonetic/romanized table schema with explicit syllable/code structure | `luna_pinyin`, canonical `rime-cantonese`, future Teochew PUJ/DP-style schemas if their active path is table/algebra based | Generic leading-unit single-character reachability applies after oracle capture. |
| Derived phonetic schemas already onboarded by M19 | `double_pinyin`, `bopomofo` | Same contract applies if the capture proves a complete leading unit can be mapped to dictionary codes after algebra. |
| Shape-code table schema | `cangjie5` | Do not force "syllable" semantics. Capture exact-code reachability or mark the single-character-leading-syllable contract N/A. |
| Schemas requiring Lua filters or plugin gears for candidate admission/order | possible future `OpenTeochew/rime-teochew` rows, depending on the selected schema/profile | Stop for a named prerequisite unless the target row is captured and can be matched without implementing the missing gear. |
| TypeDuck profile schema behavior | `jyut6ping3_typeduck` once signed off | Regression-guard only unless a future profile milestone explicitly makes it the acceptance target. |

## Phase 0 - Preconditions And Inventory

- [ ] Confirm M59 is complete, pushed, and archived with all required evidence.
- [ ] Read the M59 closeout evidence and list exactly which reachability
      mechanisms landed: complete-list materialization, leading single-family
      enumeration, selection/recomposition, typed schema configuration, and
      performance guards.
- [ ] Inventory every remaining schema-specific reachability branch, cap, or
      completion flag in:
      - `crates/yune-core/src/engine.rs`;
      - `crates/yune-core/src/translator/mod.rs`;
      - `crates/yune-rime-api/src/schema_install.rs`;
      - `crates/yune-rime-api/src/processors/selector.rs`;
      - `apps/yune-web/src/worker.ts` and schema-selection code only if browser
        schema exposure is touched.
- [ ] Record a `docs/reports/evidence/m60-schema-general-reachability/phase-0/`
      inventory with each branch classified as:
      `generic`, `profile-specific`, `schema-onboarding`, `legacy-to-remove`,
      or `not-reachability-related`.
- [ ] Stop if M59 left a red ratchet, uncommitted oracle capture, or unresolved
      canonical/profile predicate leak.

## Phase 1 - Define The Schema Capability Contract

- [ ] Add a written contract, preferably under `docs/contracts/`, that defines
      a schema reachability capability with these states:
      - `leading_complete_unit_single_characters`;
      - `exact_code_single_characters`;
      - `not_applicable_shape_code`;
      - `blocked_missing_gear`;
      - `disabled_explicitly`.
- [ ] Define required evidence for enabling the capability:
      - upstream oracle provenance;
      - schema source repository and commit;
      - dependency repositories and commits;
      - page size and options;
      - bare leading-unit capture;
      - mid-composition capture;
      - selection/recomposition capture;
      - all-pages proof or a bounded proof that names why all pages are not
        applicable.
- [ ] Define the non-circular comparator shape:
      - ordered single-character family comes from oracle bytes;
      - Yune output is compared as page/action snapshots;
      - source dictionary inspection may prove feasibility, but never expected
        order.
- [ ] Add the contract to the schema-onboarding recipe so future schema work
      cannot skip the reachability decision.

## Phase 2 - Generalize Capture And Diff Harnesses

- [ ] Extend `scripts/capture-upstream-schema.ps1` or add a sibling harness for
      reachability flows. It must support:
      - setting page size;
      - selecting schema id;
      - paginating until `is_last_page`;
      - selecting a candidate by page/index;
      - recording remainder/preedit/commit state after selection;
      - serializing schema/source/dependency provenance.
- [ ] Add fixture schema metadata for reachability captures under
      `crates/yune-core/tests/fixtures/upstream-1.17.0/` or a new
      `upstream-schema-reachability/` root if the fixture shape diverges from
      existing M12/M19 fixtures.
- [ ] Extend `crates/yune-core/tests/oracle_fixture_provenance.rs` so any M60
      reachability fixture:
      - has oracle engine/version/commit;
      - has schema repo/commit;
      - has no local absolute paths;
      - declares the capability state;
      - declares whether full pagination was captured.
- [ ] Add a diff tool or focused test helper that compares:
      - ordered page text/comment snapshots;
      - `is_last_page` / page count;
      - selected candidate consumed span;
      - post-selection remainder;
      - final commit text.

## Phase 3 - Implement The Generic Engine Capability

- [ ] Replace any residual schema-id checks controlling reachability with typed
      translator/schema configuration installed from schema data and M60's
      capability contract.
- [ ] Introduce or refine a single generic provider for "single-character rows
      matching the leading complete unit." It must operate over the same
      dictionary/algebra view used by normal translation, not a separate
      hand-coded romanization table.
- [ ] Keep bounded typing requests finite. The typing path may materialize only
      a page-sized or guard-sized prefix, as long as it reports incompleteness
      truthfully when deeper reachable rows exist.
- [ ] Make the page-turn/full-list path complete for enabled schemas. It must
      not use fixed input-length gates, per-tone top-N gates, or schema-specific
      constants that hide deeper single-character rows.
- [ ] Ensure candidate ordering follows the schema's normal oracle order:
      phrase/sentence rows that the oracle places before singles stay before
      singles; single rows are not promoted just to satisfy reachability.
- [ ] Ensure partial selection uses the existing consumed-span/recomposition
      machinery. Do not add a second partial-selection path.
- [ ] Preserve `not_applicable_shape_code` and `blocked_missing_gear` as loud
      onboarding outcomes, not as green compatibility claims.

## Phase 4 - Apply The Contract To Regression Targets

- [ ] Convert M59's canonical Jyutping reachability tests into contract users
      rather than bespoke schema behavior.
- [ ] Convert M59's upstream `luna_pinyin` `ziyiju` / `moboyi` tests into
      contract users rather than bespoke schema behavior.
- [ ] Add one M19-derived phonetic breadth control if the oracle supports it
      cleanly, such as `double_pinyin` or `bopomofo`, to prove the contract is
      not hardcoded to Luna/Jyutping.
- [ ] Add one non-phonetic negative control, likely `cangjie5`, that proves M60
      does not invent syllable semantics for shape-code schemas.
- [ ] Optionally use `OpenTeochew/rime-teochew` as an external stress probe:
      pin a specific repository commit and selected schema id (`teochew_puj`,
      `teochew_dp`, or another row discovered during capture), then classify it
      as enabled, blocked by Lua/filter/plugin dependency, or out of M60 scope.
      Do not ship Teochew product support unless the owner explicitly widens
      the milestone.

## Phase 5 - Browser And Product Surface Audit

- [ ] If no browser code changed, record that M60 is engine/ABI-only and skip
      browser gates with reason.
- [ ] If schema onboarding or `apps/yune-web` schema exposure changes, run the
      relevant yune-web and runtime gates and record browser-visible evidence.
- [ ] Confirm public-demo schema filtering remains explicit: M60 must not make
      every schema publicly selectable merely because the engine contract exists.
- [ ] Confirm userdb dictionary keys and schema ids are not silently migrated.
      Any schema-id/product rename remains separately sign-off-gated.

## Phase 6 - Documentation, Requirements, And Closeout

- [ ] Update `docs/conventions.md` or the new contract doc to make the
      reachability decision part of future schema onboarding.
- [ ] Update `docs/roadmap.md` and `docs/requirements.md` with the measured M60
      result.
- [ ] Add an evidence README under
      `docs/reports/evidence/m60-schema-general-reachability/` with:
      - captured schemas;
      - enabled capability states;
      - blocked/N/A states;
      - exact commands;
      - test gates;
      - residual gaps.
- [ ] Move this plan to `docs/plans/completed/` only after code, evidence,
      docs, requirements, roadmap, and closeout state agree.

## Required Verification

Run the exact gates required by touched paths. Minimum expected gates:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo test -p yune-core --test oracle_fixture_provenance`
- `cargo test -p yune-core --test upstream_luna_pinyin_parity`
- `cargo test -p yune-core --test cantonese_parity`
- focused M60 schema reachability tests
- M55 ratchet / Track B guard if product-path performance, candidate
  materialization, or translator bounds change
- `git diff --check`

Add schema-specific gates when the selected controls require them, for example
`upstream_double_pinyin_parity`, `upstream_zhuyin_parity`,
`upstream_cangjie_parity`, yune-rime-api frontend tests, or yune-web browser
gates.

## Decided Calls

- M60 depends on M59; it does not run in parallel with M59 behavior fixes.
- The contract is capability-driven, not schema-id-driven.
- Future schemas inherit the feature only when they satisfy the contract and
  have oracle evidence.
- Shape-code schemas and plugin-dependent schemas must be classified honestly;
  they do not get fake green reachability.
- No candidate promotion. Reachability means paging to oracle-ordered rows.
- No product schema exposure, schema-id rename, or TypeDuck profile migration
  without separate owner sign-off.

## Non-Goals

- No blanket "all RIME schemas now supported" claim.
- No full librime C++ plugin ABI, Lua plugin runtime, predict/proto, or dynamic
  gear compatibility unless a selected M60 fixture proves it is strictly
  required and the owner widens scope.
- No shipping OpenTeochew, Teochew UI, Teochew public-demo schema, or Teochew
  product docs as part of M60 unless separately authorized.
- No ranking invention, promotion, or Yune-authored expected output where
  upstream oracle behavior is capturable.
- No performance win claim.

## Review Checklist

- Did M60 wait for a clean M59 closeout?
- Does every enabled schema have upstream-oracle reachability captures and
  provenance?
- Does every future schema go through a capability decision rather than a
  hardcoded schema-id branch?
- Are shape-code and plugin-dependent schemas classified without overclaiming?
- Does paging remain truthful (`is_last_page` is false while reachable rows
  remain hidden)?
- Does selection commit only the selected span and recompose the remainder?
- Are M59 canonical Jyutping and Luna tests preserved as contract users?
- Are all TypeDuck profile guards still profile-scoped?
