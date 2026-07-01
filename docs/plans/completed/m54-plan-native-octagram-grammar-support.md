# M54 Native Octagram Grammar Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

> **Status:** Complete - **Milestone:** M54 (native octagram grammar support) -
> **Updated:** 2026-07-01 - **Type:** oracle-first engine feature. Task 0
> records the pinned data, license, checksum, and vendoring decisions; closeout
> evidence is recorded under
> `docs/reports/evidence/m54-native-octagram-grammar-support/`.

**Goal:** Promote learned `.gram` / octagram grammar from a documented deferred
surface into a named, oracle-backed Yune engine target by implementing
octagram-compatible grammar scoring as a Yune-native engine feature behind the
existing `Grammar` trait. This milestone does **not** implement the librime C++
plugin ABI.

**Architecture:** Build the behavior into Yune's deterministic `poet` /
`Grammar` path, not into a dynamic plugin loader. Upstream librime exposes
octagram through a plugin package, but the observable engine boundary is the
`Grammar::Query(context, word, is_rear)` scoring call used by `Poet`.
Yune already owns an equivalent Rust seam:
`crates/yune-core/src/poet/mod.rs` defines
`Grammar::query(context, word, is_rear)` and `NullGrammar`, with the upstream
null-grammar penalty `-13.815510557964274`. M54 should replace the current
constant null-grammar add only when a schema selects a named `.gram` model.

**Decision:** The milestone implements an `OctagramGrammar` provider in Rust and
schema-selects it for one named target. It does not load arbitrary librime C++
plugins, expose plugin lifecycle hooks, widen `rime_get_api()`, or copy GPLv3
octagram implementation code.

---

## Review Questions

Before implementation starts, review should answer:

- Does the two-lane data choice hold: `lotem/rime-octagram-data` as the
  canonical oracle lane, and `amzxyz/RIME-LMDG` as the real-world validation
  lane?
- Is `contextual_translation` required by that target's oracle path, or should
  M54 cover only the `poet` sentence/lattice grammar path and keep contextual
  translation deferred?
- Is the clean-room boundary strong enough given `librime-octagram` is GPLv3?
- Are the model-data licenses and attribution requirements verified against the
  current repository `LICENSE` files for `lotem/rime-octagram-data` and
  `amzxyz/RIME-LMDG`?
- Are the no-go gates strict enough if the oracle build, `.gram` data, or target
  selection is not reproducible?
- Should this remain M54, replacing the tentative Track A memory-gap M54
  candidate, or should octagram use a different number after review? The draft
  recommendation is: keep octagram as **M54** because it is a concrete
  compatibility target, and move the Track A memory-research candidate to M55
  if/when the roadmap is updated.

## Current Starting Point

Repo facts to preserve:

- `docs/roadmap.md` and `docs/contracts/engine-support-contract.md` currently
  list learned `.gram` / octagram grammar as unsupported/deferred until a named
  target adds an oracle, ABI contract, and evidence.
- `docs/roadmap.md` separately defers full librime C++ plugin ABI
  compatibility and says to prefer Yune-native extension points first.
- `docs/plans/reference/ai-native-context-awareness-tradeoffs.md` classifies
  octagram as compatibility-only classic-path behavior, not the AI
  differentiator.
- M17 already implemented upstream `luna_pinyin` sentence/lattice parity through
  the null-grammar path because the captured upstream `luna_pinyin` fixture had
  `grammar_model: null`.
- `crates/yune-core/src/poet/mod.rs` already has the trait shape needed for a
  grammar provider, but the active scoring paths currently call
  `null_grammar_score(entry.weight)` directly.
- Upstream librime's poet grammar context looks back two words. M54 must match
  that behavior exactly; it must not blindly pass the full accumulated sentence
  as grammar context.

External research facts to verify and pin in evidence:

- `librime-octagram` registers a `grammar` component under librime's plugin
  mechanism; the engine behavior is still grammar scoring, not frontend ABI.
- Octagram `.gram` files use the `Rime::Grammar/1.0` format backed by a Darts
  double-array trie with quantized log values (`kValueScale = 10000`).
- Octagram scoring depends on custom gram encoding, collocation min/max length,
  non/weak/strong collocation penalties, and rear-boundary `$` lookup.
- `librime-octagram` source is GPLv3. Yune must implement behavior cleanly in
  Rust from oracle fixtures/specification, not by copying GPL source.
- `.gram` model data has its own licensing and attribution obligations separate
  from the plugin code. As of 2026-07-01, the expected current license read is:
  `lotem/rime-octagram-data` = LGPL-3.0 and `amzxyz/RIME-LMDG` = CC-BY-4.0.
  Task 0 must still verify the current repository `LICENSE` files and record
  the evidence before implementation relies on either data source.
- M54 data-use policy:
  - `lotem/rime-octagram-data` is the canonical oracle lane. Use it as
    external oracle input, pin URL/commit/checksum, commit oracle output bytes,
    and prefer not to vendor full `.gram` files.
  - `amzxyz/RIME-LMDG` is the real-world validation lane. It is vendorable only
    with attribution/NOTICE if M54 explicitly needs checked-in model bytes;
    otherwise pin URL/commit/checksum and commit output evidence.
  - Synthetic tiny `.gram` data is the unit-test lane. It should be generated
    and owned by Yune for parser/scoring tests rather than copied from
    third-party model files.

## Scope

In scope:

- Use two external data lanes plus one Yune-owned unit-test data lane:
  - **Canonical oracle lane:** upstream `luna_pinyin` + `librime-octagram` +
    `lotem/rime-octagram-data`, using the appropriate `grammar:/hant`,
    `grammar:/hans`, `grammar:/hant_char`, or `grammar:/hans_char` model.
  - **Real-world validation lane:** `amzxyz/RIME-LMDG` / Wanxiang-style
    octagram data, with `translator/contextual_suggestions: false` unless
    review explicitly expands M54.
  - **Unit-test lane:** synthetic tiny `.gram` fixtures generated and owned by
    Yune for parser/scoring coverage.
- Pin the oracle and validation stacks: upstream librime version, octagram
  plugin version, schema/data commits, model checksums, and exact schema
  patches.
- Capture fresh oracle fixtures before implementation. Candidate order, text,
  comments where relevant, and schema configuration must come from the oracle,
  not from Yune.
- Capture and commit oracle output bytes, fixture manifests, source URLs,
  checksums, and license/attribution notes. Full `.gram` model files may be
  downloaded for oracle capture and integration validation; do not vendor large
  models into routine unit-test fixtures unless review accepts the size and
  license obligations.
- Implement a Yune-native `OctagramGrammar` provider behind the existing
  `Grammar` trait.
- Implement or adapt a clean `.gram` reader for the subset required by the
  named target: format validation, metadata/version checks, Darts double-array
  lookup, value scaling, and deterministic failure behavior.
- Implement octagram scoring semantics required by the target: gram encoding,
  context/word lookup, collocation penalties, rear-boundary scoring, and
  tie-order preservation.
- Thread grammar scoring through the `poet` sentence/lattice path with upstream
  two-word look-back context while preserving the current null-grammar path when
  no `.gram` model is configured.
- Load octagram grammar only for schemas that explicitly configure the named
  target; all current `luna_pinyin` null-grammar and `jyut6ping3` paths must
  keep their existing behavior.
- Update schema-install deferral records so `.gram` / `grammar` is no longer
  marked deferred for the named M54 target only.
- Add focused unit, fixture, and oracle-parity tests for grammar loading,
  scoring, and candidate ranking.
- Update roadmap, requirements, milestone ledger, support contract, and plan
  archive only on closeout.

Out of scope:

- Full librime C++ plugin ABI compatibility.
- Dynamic loading of arbitrary librime plugins.
- Lua, predict, proto, userdb learning, or broader plugin ecosystems.
- Copying GPLv3 octagram implementation code.
- Remote AI, neural LM ranking, or changing the default classic deterministic
  path.
- Frontend, browser, package, deployment, platform, or product-validation work.
- Public performance claims. Any timing or memory data is non-regression
  evidence unless M54 explicitly adds a measured performance success bar.
- Contextual translation unless Task 0 proves the named target requires it and
  review accepts that scope.
- Committing large third-party `.gram` model files into routine unit-test
  fixtures without an explicit size, license, and attribution decision.

## Files And Responsibilities

- Create: `docs/reports/evidence/m54-native-octagram-grammar-support/`
  - Oracle build notes, pinned source/data versions, fixture provenance, design
    notes, non-regression evidence, and final gates.
- Create: `crates/yune-core/tests/fixtures/upstream-octagram/`
  - Oracle fixture JSON and manifest for the named `.gram` target.
- Modify after review, if implementation proceeds:
  - `crates/yune-core/src/poet/mod.rs`
  - possible new `crates/yune-core/src/poet/grammar.rs` or
    `crates/yune-core/src/poet/octagram.rs`
  - possible dictionary double-array / byte-source modules if reuse is cleaner
    than keeping `.gram` logic inside `poet`
  - `crates/yune-rime-api/src/schema_install.rs`
  - `crates/yune-rime-api/src/tests/schema_selection/deferred_oracles.rs`
  - focused `yune-core` poet/oracle tests
- Modify on closeout only:
  - `docs/roadmap.md`
  - `docs/requirements.md`
  - `docs/contracts/engine-support-contract.md`
  - `docs/ledgers/milestone-history.md`
  - this plan, moved to `docs/plans/completed/`

## Task 0: Review Gate And Target Selection

**Status:** Complete. Evidence:
`docs/reports/evidence/m54-native-octagram-grammar-support/task-0-target-selection.md`.

- [x] Confirm `lotem/rime-octagram-data` as the canonical oracle data source for
  upstream `luna_pinyin` + octagram behavior.
- [x] Confirm `amzxyz/RIME-LMDG` as the default real-world validation data
  source for Wanxiang-style grammar behavior.
- [x] Record why this target is an engine compatibility target, not an AI
  experiment or product/frontend request.
- [x] Pin both stacks: librime commit/tag, octagram plugin commit, schema source
  commit, lotem data/model commit and checksums, RIME-LMDG data/model commit
  and checksums, and exact schema patches.
- [x] Verify the current model-data licenses and attribution requirements from
  the source repositories' `LICENSE` files. The expected current read is lotem
  LGPL-3.0 and RIME-LMDG CC-BY-4.0, but the plan must use the verified current
  licenses. Decide whether any full `.gram` files are vendored, downloaded by
  evidence scripts, or referenced only by URL/checksum.
- [x] Decide whether `contextual_translation` is required for the target. If it
  is required, add it explicitly to scope before implementation; otherwise keep
  it deferred.
- [x] Confirm M54 numbering against the current roadmap. If M54 should stay
  reserved for Track A memory work, rename this plan before implementation.
  Draft recommendation: octagram remains M54 and the Track A memory research
  candidate moves to M55 during roadmap adoption.

**No-go:** Stop here if the lotem canonical oracle stack cannot be pinned and
reproduced, or if model license/attribution requirements cannot be satisfied.

## Task 1: Oracle Capture

**Status:** Complete. Evidence:
`docs/reports/evidence/m54-native-octagram-grammar-support/phase-0-oracle/`
and `crates/yune-core/tests/fixtures/upstream-octagram/`.

- [x] Build or obtain upstream librime with `librime-octagram` enabled and the
  lotem `.gram` data installed for the canonical oracle lane.
- [x] Capture the RIME-LMDG validation lane with
  `translator/contextual_suggestions: false` unless review explicitly expands
  M54 to contextual translation.
- [x] Capture same-run oracle outputs for short, medium, and long inputs where
  grammar scoring changes candidate ranking.
- [x] Include negative controls where no grammar model is configured and where
  null-grammar behavior should remain unchanged.
- [x] Record exact commands, environment, source versions, data checksums, and
  output bytes under
  `docs/reports/evidence/m54-native-octagram-grammar-support/phase-0-oracle/`.
- [x] Add fixture manifest fields that state this is an octagram-enabled oracle,
  not the default upstream `luna_pinyin` null-grammar fixture.
- [x] Record model URLs/checksums for lotem and RIME-LMDG. Keep routine
  checked-in fixtures limited to oracle outputs plus small synthetic `.gram`
  parser/scoring fixtures unless review explicitly accepts vendoring full model
  files.

**No-go:** Stop here if the lotem oracle cannot be made reproducible, if
RIME-LMDG validation cannot be pinned, or if neither lane produces observable
octagram-dependent candidate behavior.

## Task 2: Clean-Room Design

**Status:** Complete. Evidence:
`docs/reports/evidence/m54-native-octagram-grammar-support/clean-room-design.md`.

- [x] Write a short clean-room design note in the evidence folder describing the
  behavior to implement without copying GPLv3 code.
- [x] State that oracle output bytes are the primary source of truth; factual
  constants and file-format facts may be transcribed, but GPL code structure,
  helper decomposition, and implementation text must not be copied.
- [x] Specify `.gram` format expectations, parse failures, byte ownership, and
  memory behavior for native and WASM builds.
- [x] Specify upstream-compatible grammar context handling, including the
  two-word look-back and rear-boundary behavior.
- [x] Specify schema configuration mapping for `grammar/language` and
  collocation penalty fields.
- [x] Define non-regression evidence for current null-grammar `luna_pinyin`,
  TypeDuck `jyut6ping3`, and M52 Track A guardrail rows.

**No-go:** Stop here if implementation would require copying GPLv3 source,
committing non-redistributable data, or changing the public C ABI.

## Task 3: Implement Octagram Grammar Loading And Scoring

**Status:** Complete. Evidence:
`crates/yune-core/src/poet/octagram.rs`,
`crates/yune-core/src/tests/poet.rs`, and
`docs/reports/evidence/m54-native-octagram-grammar-support/final-gates.md`.

- [x] Add a Rust `OctagramGrammar` implementation behind the existing
  `Grammar` trait.
- [x] Add `.gram` metadata/version validation and deterministic error reporting.
- [x] Add Darts double-array lookup support for `.gram` payloads, reusing
  existing byte-backed/double-array infrastructure where practical.
- [x] Implement gram encoding, scaled-value conversion, collocation penalties,
  weak/non-collocation fallback, and rear-boundary scoring.
- [x] Add unit tests for encoding, lookup, scaling, penalty selection, missing
  model fallback, malformed model rejection, and deterministic tie behavior.
- [x] Use small redistributable synthetic `.gram` data for unit tests unless
  review explicitly accepts vendoring the full lotem/RIME-LMDG model files.

## Task 4: Integrate With Poet And Schema Install

**Status:** Complete. Evidence:
`crates/yune-core/src/poet/mod.rs`,
`crates/yune-core/src/translator/mod.rs`,
`crates/yune-rime-api/src/schema_install.rs`, and focused schema tests in
`crates/yune-rime-api/src/tests/resource_id.rs`.

- [x] Replace direct `null_grammar_score(entry.weight)` use in the relevant
  sentence/lattice path with grammar-aware scoring while preserving exact
  null-grammar behavior.
- [x] Match upstream two-word look-back context. Do not use full accumulated
  sentence text unless oracle evidence proves that path is correct.
- [x] Ensure schemas without `.gram` do not load an octagram model and do not
  pay octagram memory or lookup cost.
- [x] Schema-select `OctagramGrammar` only for the named target and keep current
  deferral records for unsupported plugin/model behavior outside that target.
- [x] Keep `rime_get_api()`, `RimeCandidate`, and TypeDuck profile accessors
  unchanged.

## Task 5: Oracle Parity And Regression Tests

**Status:** Complete. Evidence:
`phase-3-yune-core-verification.md`,
`phase-3-yune-core-verification.json`,
`final-gates.md`, and the focused test commands recorded there.

- [x] Add fixture-backed evidence that compares Yune accepted top-candidate
  behavior against the captured lotem canonical oracle and RIME-LMDG validation
  fixtures. Full first-page ordering is not the M54 acceptance gate because
  known table/menu ordering differences outside the grammar provider remain
  separately scoped.
- [x] Add negative-control tests proving current null-grammar `luna_pinyin`
  sentence/lattice behavior is unchanged.
- [x] Add TypeDuck `jyut6ping3` regression tests sufficient to prove the profile
  did not accidentally start using upstream octagram behavior.
- [x] Update `schema_selection_defers_poet_grammar_contextual_translation` or
  successor tests so the named target is supported while broader plugin/model
  behavior remains deferred.

## Task 6: Non-Regression Evidence

**Status:** Complete. Evidence:
`docs/reports/evidence/m54-native-octagram-grammar-support/final-gates.md`.

- [x] Run targeted Rust tests for the changed `poet`, fixture, schema-install,
  and profile paths.
- [x] Evaluate the M52 Track A threshold command condition; it did not apply
  because tracked null-grammar rows were unchanged and the upstream
  `luna_pinyin` parity gate passed.
- [x] Record octagram model memory ownership separately from existing
  `poet.vocabulary` and `poet.entries_by_code` rows.
- [x] If the model is exposed to WASM, record that browser/package/memory claims
  are out of scope unless separately measured.

## Task 7: Closeout Docs

**Status:** Complete. Evidence:
`docs/contracts/engine-support-contract.md`, `docs/roadmap.md`,
`docs/requirements.md`, `docs/ledgers/milestone-history.md`, and this archived
plan.

- [x] Update the support contract so learned `.gram` / octagram grammar is
  supported only for the named M54 target, while the full C++ plugin ABI remains
  deferred.
- [x] Update roadmap Scope Ledger and Authoritative Sequence with the M54
  verdict.
- [x] Add M54 requirement IDs and coverage mappings.
- [x] Add a milestone-history row.
- [x] Archive this plan to `docs/plans/completed/` and leave evidence under
  `docs/reports/evidence/m54-native-octagram-grammar-support/`.

## Definition Of Done

M54 closes as complete only when:

- The canonical lotem oracle lane and RIME-LMDG validation lane are pinned and
  documented.
- Fresh oracle fixtures from librime + octagram + data are checked in with
  reproducible provenance; real `.gram` model dependencies are represented by
  permitted checked-in data or by URL/checksum plus committed oracle outputs and
  attribution/license notes.
- Yune matches oracle candidate behavior for the accepted octagram fixture set.
- Current null-grammar `luna_pinyin` behavior is unchanged.
- Current TypeDuck `jyut6ping3` profile behavior is unchanged.
- Schemas without `.gram` do not load octagram data.
- The public C ABI is unchanged.
- Docs clearly distinguish "native octagram-compatible grammar support" from
  "librime C++ plugin ABI support."
- Evidence records memory/timing impact without making unsupported performance,
  browser, platform, or product claims.

M54 closes as partial/no-go if:

- The lotem canonical oracle stack cannot be reproduced.
- The RIME-LMDG validation stack cannot be pinned.
- Behavior parity would require copying GPLv3 source.
- Reproducible evidence depends on `.gram` data that cannot be referenced,
  attributed, regenerated, or redistributed safely.
- Parity requires changing the frozen public ABI.
- Octagram support regresses existing null-grammar or TypeDuck behavior and the
  regression cannot be isolated.

## Proposed Requirement IDs

Add these to `docs/requirements.md` only on closeout:

- **M54-OCTAGRAM-01**: The lotem canonical oracle lane and RIME-LMDG validation
  lane have pinned source/data versions, model checksums, data-license status,
  attribution notes, and fixture manifests defining the supported octagram
  scope.
- **M54-OCTAGRAM-02**: Yune implements octagram-compatible grammar loading and
  scoring through a native Rust `Grammar` provider, not a librime C++ plugin ABI.
- **M54-OCTAGRAM-03**: The `poet` sentence/lattice path uses upstream-compatible
  grammar context and preserves null-grammar behavior when no model is
  configured.
- **M54-OCTAGRAM-04**: Oracle fixture tests prove candidate behavior for the
  named octagram target, including the executable synthetic empty-context
  rear-boundary oracle, and negative controls prove existing `luna_pinyin` and
  `jyut6ping3` behavior is unchanged.
- **M54-OCTAGRAM-05**: Documentation and support-contract wording distinguish
  named octagram support from broader plugin ABI compatibility, which remains
  deferred.
- **M54-OCTAGRAM-06**: Evidence records octagram model memory/timing impact
  without making unsupported frontend, browser, platform, product, or broad
  performance claims.
- **M54-OCTAGRAM-07**: Checked-in fixture data respects model-data licensing:
  full lotem/RIME-LMDG `.gram` files are vendored only with an explicit size,
  license, and attribution decision; otherwise the repository stores oracle
  outputs, checksums/URLs, license notes, and small synthetic parser/scoring
  fixtures.

## Review Prompt

Suggested prompt for review:

> Please review
> `docs/plans/completed/m54-plan-native-octagram-grammar-support.md` as a draft
> M54 plan. Focus on whether the milestone should build octagram as a native
> Yune `Grammar` provider rather than a librime C++ plugin ABI, whether the
> target/oracle/no-go gates are strict enough, whether `contextual_translation`
> should be in or out of scope, whether the lotem canonical oracle lane and
> RIME-LMDG validation lane handle `.gram` data licensing/attribution safely,
> whether octagram should take M54 and push Track A memory research to M55, and
> whether the plan preserves M51 ABI, M52 guardrails, M53 support-contract
> wording, and existing null-grammar/TypeDuck behavior.
