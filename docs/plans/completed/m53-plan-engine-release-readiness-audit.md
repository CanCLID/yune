# M53 Engine Release-Readiness Audit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **Status:** Complete - **Closed:** 2026-06-30 - **Track:** Core compatibility / engine release-readiness. - **Created:** 2026-06-30 - **Type:** docs/evidence consistency audit (no code, no ABI, no perf bar).

**Closeout:** M53 closed on 2026-06-30. The engine docs are release-ready for
downstream engine consumers. A five-dimension parallel audit (support-contract
consistency, ABI-wording-vs-code, M52 guardrail freshness, public claim wording,
link/evidence integrity), with each finding adversarially re-verified, found the
substantive invariants clean and no ABI/guardrail/link drift. The only real
defects were stale M45-era "faster than librime" claims and a `127 MB` memory
figure on `README.md` (plus one linked archived report), which were corrected to
the M52 (2026-06-30) lane-specific numbers. Evidence:
[`../../reports/evidence/m53-engine-release-readiness-audit/`](../../reports/evidence/m53-engine-release-readiness-audit/).

**Goal:** Confirm the engine's launch-facing docs and evidence are internally
consistent and release-ready before any platform/frontend session consumes the
engine, and correct any stale public performance claim - without reopening
performance work or touching code/ABI.

**Architecture:** Read-only audit followed by targeted doc fixes. The audit
fanned out over five dimensions in parallel; every material finding was
adversarially verified against the actual file/code/CSV before it was accepted
or downgraded. No Rust/TS/browser gates were required because the audit found no
code or export-list drift.

---

## Scope

In scope:

- Support-contract consistency across `docs/contracts/engine-support-contract.md`,
  `docs/conventions.md`, `docs/roadmap.md`, `docs/requirements.md`, and the
  milestone-history ledger.
- ABI expectation consistency vs code: default `rime_get_api()` and
  `RimeCandidate` upstream-shaped; TypeDuck/Yune Windows fork-only slots behind
  named profile accessors; `yune_web_*` as a separate 14-symbol WASM ABI.
- M52 guardrail freshness: threshold file, `threshold-check.csv`, manual command
  shape, and numeric consistency across the docs.
- Public performance-claim wording: no stale broad "faster than librime"
  framing; lane-specific claims only.
- Adopting the 2026-06-30 dashboard visuals and the reframed reports.

Out of scope:

- Any implementation or optimization; the 188 MB memory work stays the deferred
  M54 candidate.
- Frontend, browser, yune-web, platform, product, package, deployment, or
  iOS-device proof.
- ABI changes and any new performance success bar. M52 ceilings remain
  no-regression guardrails.

## Tasks

- [x] **Task 1: Audit support-contract consistency** across the doc set. Result:
  consistent; contract status line refreshed to note it is unchanged by M52 and
  re-verified by M53.
- [x] **Task 2: Verify ABI wording matches code.** Result: release-ready, no
  drift - default upstream `rime_get_api()`/`RimeCandidate`, both profile
  accessors alias the same table, exactly 14 `yune_web_*` exports.
- [x] **Task 3: Verify M52 guardrail freshness and numbers.** Result:
  release-ready - all headline numbers reconcile to the committed CSVs; the
  manual `-TrackAThresholds ... -FailOnRegression` command shape is canonical in
  the roadmap.
- [x] **Task 4: Sweep public performance-claim wording.** Result: the two live
  reports and roadmap were already lane-specific; `README.md` and one linked
  archived report carried stale M45-era overclaims and were corrected to the M52
  numbers.
- [x] **Task 5: Verify link/evidence integrity and adopt the visuals.** Result:
  all links/anchors resolve; the 2026-06-30 dashboard-visuals folder is complete
  and embedded; the reports/roadmap/visuals edits are adopted by this milestone.
- [x] **Task 6: Closeout.** Record the audit evidence, add M53 requirement IDs,
  and update roadmap, milestone-history, and requirements coverage counts.

## Definition Of Done

- A completed M53 plan/evidence record states the engine docs are release-ready
  for downstream engine consumers. (Done: this plan + the evidence README.)
- Requirements gain M53 audit rows, all complete.
- Roadmap records M53 complete and still names M52 as the native Track A
  guardrail source of truth.
- No doc implies Yune is broadly faster than librime; every "faster than
  librime" claim is scoped to `zhongguo` + the two abbreviation rows.
- Evidence lanes remain separated (native vs browser vs product vs platform).

## Requirement IDs

Added to `docs/requirements.md` on closeout:

- **M53-AUDIT-01**: Support-contract consistency verified across contract,
  conventions, roadmap, requirements, and ledger; no contradiction.
- **M53-AUDIT-02**: ABI wording matches code - default upstream
  `rime_get_api()`/`RimeCandidate`, profile-only TypeDuck/Yune Windows slots,
  and exactly 14 `yune_web_*` exports.
- **M53-AUDIT-03**: M52 guardrail evidence is fresh and numerically consistent
  across the docs; the manual regression-gate command is canonical.
- **M53-AUDIT-04**: Public performance claims are lane-specific; stale broad
  "faster than librime" wording in `README.md` and the linked archived report
  was corrected to the `zhongguo` + abbreviation-only scope.
- **M53-AUDIT-05**: All engine-doc evidence links/anchors resolve and the
  2026-06-30 dashboard visuals are adopted.
