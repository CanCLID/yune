# M58 Jyutping Oracle Rebase, Shipped-Lane Bug Disposition, And Schema Identity Safety Plan

> **Filename note:** the path still contains `exact-before-fuzzy` for link
> continuity with earlier drafts. The current scope is broader: oracle rebase,
> shipped TypeDuck-lane bug disposition, and schema/profile identity safety.

> **For agentic workers:** this plan supersedes the earlier TypeDuck-v1.1.2
> M58 drafts. Do not execute any old plan that used TypeDuck `jyut6ping3` as the
> canonical candidate oracle. Run one phase at a time, preserve evidence, and
> stop at any oracle/provenance contradiction.

> **Status:** Draft rewrite / not ready for implementation until Phase 0 and
> Phase 1 complete. **Track:** core compatibility plus schema/profile identity
> safety. **Created:** 2026-07-05. **Type:** oracle/provenance repair before
> any candidate-behavior fix.
>
> **Preflight claim audit:** see
> [`../../reports/evidence/m58-preflight-claim-audit/README.md`](../../reports/evidence/m58-preflight-claim-audit/README.md)
> for the docs/evidence scope correction that separates canonical
> `rime-cantonese` candidate claims from TypeDuck profile claims.

## Decision Summary

M58 has three lanes that must not be collapsed:

1. **Canonical Cantonese/Jyutping candidate behavior.** New canonical
   candidate ordering, segmentation, fallback, and completion claims must be
   captured from upstream `rime/librime 1.17.0` running pinned
   `rime/rime-cantonese` schema/data. TypeDuck-HK/librime v1.1.2 must not be
   used as the canonical candidate oracle.
2. **TypeDuck shipped/profile behavior.** Historical M14-M28 fixtures remain
   valid TypeDuck-profile regression guards, including fixture-backed candidate
   ordering/composition/prediction/prefix-fallback behavior such as
   M21-GAP-01 and M21-GAP-02. The reported shipped-product `beingo` / 畀
   reachability bug must receive an explicit TypeDuck-lane disposition; M58 may
   not close by saying the issue is "TypeDuck-only" and walking away.
3. **Schema identity safety.** `rime/rime-cantonese` and TypeDuck-HK/schema both
   use `schema_id: jyut6ping3`, so schema id alone is invalid provenance. The
   preferred future direction is for the plain `jyut6ping3` id to resolve to
   canonical `rime-cantonese` and for TypeDuck multilingual/profile behavior to
   use `jyut6ping3_typeduck`, but that id direction is not executable until
   Phase 3 inventories the blast radius and records explicit user sign-off.

Every fixture, compiled artifact, doc claim, and product row created by M58 must
name:

- schema source repository;
- schema source commit;
- oracle engine/version;
- Yune-facing schema id;
- page size and option set used for capture/diff.

## Goal

Rebuild M58 on a clean oracle boundary without abandoning the original shipped
bug report:

1. Capture canonical upstream-librime behavior for the reported Jyutping
   reachability/admission cases using pinned `rime/rime-cantonese`.
2. Diff Yune's canonical lane against that capture on the real compiled path.
3. Classify the `beingo` / 畀 report in both the canonical lane and the shipped
   TypeDuck/profile lane.
4. Implement only a narrow candidate reachability/admission fix supported by
   fresh evidence for the lane it affects.
5. Inventory schema/profile identity predicates and product blast radius before
   any schema-id split implementation; either obtain explicit sign-off for the
   id direction or spin the split into a separate milestone with evidence.

This is not a performance milestone and not a broad librime-feature parity
project.

## Non-Negotiables

- **No TypeDuck ordering source for canonical M58.** Existing TypeDuck v1.1.2
  captures may be used as historical context or TypeDuck-profile regression
  guards, not as expected ordering for canonical `rime-cantonese` behavior.
- **No code changes before canonical capture.** If upstream `rime/librime
  1.17.0` + `rime/rime-cantonese` cannot be captured, stop and ask for an
  explicit decision.
- **Hard pagination proof.** Candidate captures must record `captured_all_pages:
  true`; a partial page or "equivalent proof" is not acceptable for M58.
- **Option and page-size mirroring.** The upstream capture and Yune diff must
  run the same page size and option set, and the evidence must record both.
- **No schema-id ambiguity.** Any M58 fixture/artifact that says only
  `jyut6ping3` without source repo/commit and Yune-facing id is incomplete.
- **No blanket "exact-only" rule.** Earlier M58 drafts over-modeled the bug.
  Fresh capture defines whether same-initial, algebra, or completion rows are
  allowed.
- **No silent TypeDuck regression.** TypeDuck profile behavior remains guarded,
  including grandfathered candidate behavior where a historical fixture owns it.
- **No schema rename without sign-off.** Do not rename product ids, dictionary
  names, manifests, cache keys, or userdb keys until Phase 3 records the blast
  radius and explicit user sign-off.

## Phase 0 - Provenance, Harness, And Scope Freeze

Deliverables:

- Record the exact upstream `rime/librime 1.17.0` binary/source provenance used
  for canonical captures.
- Pin the exact `rime/rime-cantonese` source commit used for canonical captures.
- Pin the exact TypeDuck-HK/schema source commit used for shipped/profile-lane
  checks.
- Name the upstream capture mechanism before running captures:
  - either add/use an upstream `rime-cantonese` capture script/harness; or
  - document an exact one-off command sequence that builds/runs upstream
    librime `1.17.0` with pinned `rime-cantonese`.
- Explicitly reject the old fork-only path as canonical evidence:
  `scripts/capture-typeduck-jyutping.ps1` / `oracle-rime-probe.cs` may be used
  only if rebuilt and documented against upstream `rime/librime 1.17.0` plus
  pinned `rime-cantonese`, not the TypeDuck fork DLL.
- Record that canonical capture assets are compiled/deployed by upstream's
  deployer, while Yune diffs use Yune's real deploy/load path over the same
  source schema and mirrored options.
- Inventory current repo schema assets, fixture roots, profile predicates, and
  product identities that contain `jyut6ping3`, including:
  - `apps/yune-web/public/schema`;
  - `apps/yune-web/source/public/schema`;
  - `crates/yune-core/tests/fixtures/typeduck-v1.1.2`;
  - `crates/yune-core/tests/fixtures/upstream-jyutping`;
  - `crates/yune-rime-api/src/schema_install.rs` profile predicates;
  - WEB-03/public-demo manifests and schema lists;
  - Track B / M55 threshold workload names;
  - userdb dictionary-name persistence implications.
- Produce a provenance table with one row per lane:

| Lane | Current / proposed Yune-facing id | Source repo | Source commit | Oracle engine | Owns |
| --- | --- | --- | --- | --- | --- |
| Canonical | current/proposed `jyut6ping3` pending Phase 3 sign-off | `rime/rime-cantonese` | TBD | upstream `rime/librime 1.17.0` | new canonical candidate ordering, segmentation, fallback, completion |
| TypeDuck shipped/profile | current `jyut6ping3` / `jyut6ping3_mobile`; proposed `jyut6ping3_typeduck` pending Phase 3 sign-off | `TypeDuck-HK/schema` | TBD | TypeDuck-HK/librime v1.1.2 for profile fixtures; hybrid upstream-engine fixture where D-31 applies | multilingual comments, lookup payloads, profile/display behavior, grandfathered profile candidate guards, shipped bug disposition |

Stop conditions:

- The canonical `rime/rime-cantonese` source cannot be pinned.
- The executor cannot name a real upstream-librime capture mechanism.
- The executor cannot tell which current asset root came from which schema
  source.
- A proposed fixture uses bare `jyut6ping3` provenance.
- The TypeDuck shipped/profile lane cannot be identified well enough to decide
  the `beingo` / 畀 report.

## Phase 1 - Canonical Upstream Capture

Capture full ordered candidate output from upstream `rime/librime 1.17.0` with
pinned `rime/rime-cantonese`, not TypeDuck-HK/librime.

Capture rules:

- upstream capture uses upstream's deployer output;
- evidence records input, schema repo/commit, oracle version, deploy command,
  capture command, page size, options, and full ordered candidates;
- `captured_all_pages: true` is mandatory for every capture;
- Yune Phase 2 must mirror the same page size and option set.

Minimum canonical inputs:

- `beingo` and the relevant single/prefix forms (`bei`, `be`, `bein`, `being`)
  to decide whether the reported `bei2` reachability issue exists in canonical
  rime-cantonese behavior.
- A tone-scoped input that would catch letter-to-tone leakage if cap lifting or
  fallback gating is later changed.
- `ngohaig` / `ngohaigo` only as canonical captures, not imported from old
  TypeDuck fixtures.
- `n` / `nri` only to classify canonical single-letter and correction behavior;
  do not reuse the old TypeDuck first-6 truncation as oracle truth.
- `mgoi` or another common valid multi-syllable control, if Phase 0 identifies
  it as load-bearing for segmentation.
- The reported "諮議局" case, but Phase 0 must first record the exact ASCII
  keystrokes; do not guess or derive them from Yune output.

Stop conditions:

- Capture harness cannot produce `captured_all_pages: true`.
- Canonical upstream output contradicts the proposed fix model.
- The only available expected output is TypeDuck v1.1.2.
- The capture omits page size or option state.

## Phase 2 - Yune Canonical Diff

Run Yune against the canonical lane using the same schema source, page size,
options, and compiled-path shape intended for the product/harness. Diff ordered
output against the Phase 1 capture.

Classify each difference:

- **Reachability:** an oracle candidate exists but cannot appear in Yune before
  paging/cap exhaustion.
- **Admission overage:** Yune admits a candidate group absent from the canonical
  oracle.
- **Admission underage:** Yune drops a candidate group present in the canonical
  oracle.
- **Order-only:** same set, different rank/order.
- **Comment/display-only:** TypeDuck profile lane, not canonical candidate
  behavior.

The old hypotheses remain hypotheses only:

- cap lifting for prefix fallback is valid only if canonical capture proves a
  candidate is hidden by a cap;
- raw-code versus derived-form admission is valid only if canonical capture
  proves that specific over/under-admission pattern;
- a "leading parse" or "exact-only" rule is invalid unless canonical capture
  proves it.

## Phase 2b - Shipped TypeDuck/Profile Bug Disposition

M58 started from a user-visible shipped-product report: typing `beingo` on the
multilingual TypeDuck/Jyutping product path fails to surface 畀. That report must
be dispositioned even if canonical rime-cantonese does not reproduce it.

Required work:

- Run the shipped/profile lane for `beingo` and the relevant prefix forms using
  the current product path (`jyut6ping3` / `jyut6ping3_mobile`) before any fix.
- If the canonical lane reproduces the bug, classify whether one canonical fix
  also fixes the shipped/profile lane without weakening grandfathered TypeDuck
  fixtures.
- If the canonical lane does not reproduce the bug, capture or reuse an
  explicitly scoped TypeDuck-profile expected behavior source:
  - preferred: the original D-31 hybrid shape, upstream `rime/librime 1.17.0`
    engine plus pinned TypeDuck-derived schema/data, for composition/ranking
    only; or
  - TypeDuck-HK/librime v1.1.2 only when the claim is explicitly profile-only
    and does not become a canonical `rime-cantonese` expectation.
- Decide one of the following with evidence:
  - fix 畀 reachability in the shipped/profile lane against the scoped profile
    oracle/hybrid capture;
  - prove the shipped product default has moved to canonical rime-cantonese and
    record the TypeDuck lane as a separate profile with its own follow-up; or
  - stop for explicit user decision.

M58 cannot close as complete by treating the original report as a harmless
"TypeDuck-only" no-go.

## Phase 3 - Schema/Profile Identity Safety And Sign-Off Gate

M58 must make schema identity risk explicit before any rename/split lands. The
schema split implementation may happen in M58 only after explicit user sign-off;
otherwise it becomes a separate milestone with this phase as its evidence base.

Required audit:

- Inspect and test the current profile predicates in
  `crates/yune-rime-api/src/schema_install.rs`, especially:
  - `is_typeduck_jyut6ping3_profile`;
  - `is_yune_web_launch_byte_backed_profile`.
- Prove the intended split will not make canonical `rime-cantonese` inherit
  TypeDuck-profile shims such as prefix fallback, the `21.0` sentence word
  penalty, prediction-limit-1, or dynamic correction.
- Prove the intended split will not make the TypeDuck multilingual lane lose
  its existing profile behavior if schema id or dictionary name changes.
- Inventory `jyut6ping3_mobile` explicitly. It is a live shipped/product id, not
  an incidental old name.
- Inventory affected manifests, selectors, compiled artifact stems, cache keys,
  WEB-03/public-demo assets, Track B/M55 thresholds, and userdb dictionary-name
  persistence.
- Present the id-direction choice for explicit sign-off:
  - preferred but costly: canonical keeps/uses `jyut6ping3`, TypeDuck moves to
    `jyut6ping3_typeduck`;
  - lower-churn alternative: shipped TypeDuck ids remain, canonical receives a
    new Yune-facing id.

Stop conditions:

- No explicit user sign-off for a schema-id direction.
- The predicate audit cannot prove both lanes keep the intended profile shims.
- The rename would orphan user data or break Track B/WEB-03 without a migration
  plan.

## Phase 4 - Candidate Fix, Only If Proved

After Phase 1/2/2b prove a real lane-specific divergence, implement the
narrowest fix for that lane.

Possible fix areas, subject to proof:

- Prefix-fallback cap sizing or exemption on the compiled path.
- Admission gate scoping for single-letter fallback.
- Derived-form matching for algebra-generated forms.
- Ordering adjustment when the set is correct but rank differs.

Guardrails:

- Preserve ABI shape and TypeDuck profile ABI boundaries.
- Keep fixes schema/profile scoped where behavior differs by schema source.
- If a cap is raised, re-measure the relevant native/product latency guard
  because previous evidence showed the long product row can saturate fallback
  caps.
- Do not weaken existing TypeDuck comment/profile/candidate tests to make
  canonical behavior pass.
- Do not change profile predicates without tests that prove both canonical and
  TypeDuck/profile lanes select the intended behavior.

## Phase 5 - Verification And Closeout

Minimum gates, adjusted by touched files:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- focused canonical Jyutping parity tests created from Phase 1 captures
- focused TypeDuck profile regression tests for multilingual comments/profile
  behavior and grandfathered candidate behavior touched by the fix
- focused shipped/profile `beingo` / 畀 regression evidence or explicit no-fix
  decision evidence
- if `apps/yune-web` schema lists, assets, or UI are touched:
  - `npm.cmd --prefix apps/yune-web run typecheck`
  - focused Playwright schema-selection or candidate-output evidence
  - public-demo gating check if any public/demo surface changes
- if translator fallback/caps are touched:
  - the standing M55 ratchet with the current `m55-thresholds.csv`
  - any WEB-03/Track-B tripwire named by the implementation diff
- `git diff --check`

Closeout docs must update:

- this plan, moved to completed only if all closeout gates pass;
- `docs/roadmap.md`;
- `docs/requirements.md`;
- `docs/ledgers/milestone-history.md`;
- `docs/ledgers/fork-parity-ledger.md` if TypeDuck profile behavior changes;
- any evidence README created under `docs/reports/evidence/m58-*`.

## Win Bars

M58 can close only when all of these are true:

- New canonical `rime-cantonese` candidate behavior is evidenced against
  upstream `rime/librime 1.17.0` plus pinned `rime/rime-cantonese`.
- TypeDuck v1.1.2 is absent from canonical candidate-order expected outputs.
- Historical TypeDuck profile fixtures remain valid or are explicitly
  superseded by a new profile-lane decision.
- The shipped `beingo` / 畀 report has an explicit disposition in both the
  canonical and TypeDuck/profile lanes.
- Any candidate reachability/admission code change is justified by a lane-owned
  capture diff and guarded by an oracle fixture.
- Schema/profile predicate risks are inventoried, and schema-id direction is
  either signed off or explicitly deferred to a separate milestone.
- Docs and evidence name schema source repo, source commit, oracle version,
  Yune-facing id, page size, and option set.

## No-Go Outcomes

Close as blocked or no-go, not complete, if:

- canonical upstream captures cannot be produced;
- canonical upstream captures show no M58-owned divergence and the shipped
  TypeDuck/profile bug disposition is unresolved;
- the only passing canonical fix depends on TypeDuck v1.1.2 candidate ordering;
- schema/profile predicate behavior cannot be proven safe for the intended lane;
- latency or public-demo guardrails regress beyond the standing thresholds;
- the executor cannot get explicit sign-off for any schema-id rename required by
  the implementation.
