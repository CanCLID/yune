# M58 Preflight Jyutping Oracle Claim Audit

Date: 2026-07-05

## Verdict

This preflight audit corrects current-facing docs before M58 execution. It does
not rerun oracle captures, benchmarks, browser tests, or schema builds.

Current rule: canonical `jyut6ping3` candidate ordering, segmentation,
fallback, and completion use upstream `rime/librime 1.17.0` plus pinned
`rime/rime-cantonese`. TypeDuck multilingual/comment/profile behavior remains a
profile lane under current shipped ids until a signed-off schema split. The
preferred future TypeDuck profile id is `jyut6ping3_typeduck`, pending M58's
blast-radius audit and explicit user sign-off.

## Claim Inventory

| File | Claim type | Action | Rationale |
| --- | --- | --- | --- |
| `AGENTS.md` | Repo guide / future-session instructions | Corrected | Names canonical `jyut6ping3` against `rime-cantonese` and TypeDuck multilingual/comment/profile behavior as a profile lane with `jyut6ping3_typeduck` pending sign-off. |
| `CLAUDE.md` | Agent-facing workflow instructions | Corrected | Removes the old unqualified TypeDuck `jyut6ping3` oracle framing from the automatic Claude guidance and mirrors the lane split. |
| `README.md` | Current compatibility baseline | Corrected | Separates upstream Luna, canonical `jyut6ping3` candidate behavior, and TypeDuck profile behavior. |
| `docs/contracts/engine-support-contract.md` | Current support contract | Corrected | Names canonical `jyut6ping3` against `rime-cantonese`, TypeDuck profile behavior against TypeDuck-HK/librime v1.1.2, and the schema-id split as gated. |
| `docs/conventions.md` | Developer workflow/oracle rules | Corrected | Prevents `cantonese_parity` and TypeDuck v1.1.2 fixtures from being read as canonical `jyut6ping3` ordering coverage. |
| `docs/reports/yune-vs-librime-performance.md` | Current performance dashboard | Corrected | Keeps Track B numbers but labels them TypeDuck profile/product guard evidence, not canonical `rime-cantonese` behavior evidence. |
| `docs/reports/yune-vs-librime-root-cause-analysis.md` | Current root-cause dashboard | Corrected | Same Track B lane clarification as the performance dashboard. |
| `docs/ledgers/milestone-history.md` | Historical ledger | Annotated | Adds a top-level pre-M58 note and clarifies M14-M16, M28 follow-up, and M56 rows without rewriting evidence history. |
| `docs/ledgers/fork-parity-ledger.md` | Fork/profile reference | Annotated | States that the ledger governs TypeDuck/Cantoboard fork improvements, not canonical `jyut6ping3` candidate ordering. |
| `docs/plans/completed/m28-follow-up-plan-upstream-jyutping-composition.md` | Historical completed plan | Annotated | Preserves old D-31 text as history and points future work to the D-31 rime-cantonese amendment. |
| `docs/plans/completed/m53-plan-engine-release-readiness-audit.md` | Historical completed plan | Annotated | Preserves the 2026-06-30 audit while marking its old oracle framing as superseded for future canonical claims. |
| `docs/requirements.md` | Current requirement ledger | Corrected | Adds supersession language to historical M28/M53 rows and keeps M58 draft requirements authoritative. |
| `docs/requirements.md` M53 row | Historical requirement row | Annotated | Discloses that the M53 public-claim audit was correct for its 2026-06-30 TypeDuck profile framing, while future canonical `jyut6ping3` claims follow the D-31 amendment. |
| `docs/decisions.md` D-20/D-27 | Historical decision bodies | Restored + annotated | Avoids inline rewrites of historical decisions; D-27 receives a dated note pointing future canonical claims to the D-31 amendment. |
| `docs/decisions.md`, `docs/roadmap.md`, active M58 plan | Current decision/roadmap/plan | Corrected | They contain the D-31 amendment, grandfathered TypeDuck profile guards, shipped-bug disposition requirement, and schema-id sign-off gate. |

## Historical Claims Left Intact

- Raw TypeDuck v1.1.2 fixtures under `crates/yune-core/tests/fixtures/typeduck-v1.1.2/` were not edited.
- Historical milestone evidence, benchmark CSV/JSON files, and browser result
  artifacts were not rewritten.
- M9, M14-M28, M47, and related TypeDuck evidence remains valid as TypeDuck
  profile evidence. Fixture-backed M14-M28 candidate behavior remains
  grandfathered as profile-lane regression evidence.
- Existing historical asset names such as `jyut6ping3_mobile` remain in old
  evidence and completed plans. They should not be treated as the future
  canonical/schema-split naming contract, and they must be inventoried before
  any rename.
- The superseded untracked TypeDuck v1.1.2 M58 capture was removed from the new
  M58 evidence root so it cannot be mistaken for canonical rime-cantonese
  evidence.

## Stale-Claim Scan Disposition

The broad stale-claim scan is intentionally run across all `README.md` and
`docs/`, including completed plans and reference notes. After this audit, its
remaining hits are historical/profile-only rows in:

- `docs/decisions.md` D-31, where the historical hybrid body is intentionally
  restored and the 2026-07-05 amendment scopes TypeDuck candidate behavior as
  grandfathered profile evidence;
- `docs/references/typeduck-windows-backend-requirements.md`;
- `docs/plans/completed/m09-findings-typeduck-web-integration.md`;
- `docs/plans/completed/m09-plan-typeduck-web-validation.md`;
- `docs/plans/completed/m17-plan-upstream-language-model-poet.md`;
- `docs/plans/completed/m28-plan-typeduck-partial-selection.md`;
- `docs/plans/completed/m28-follow-up-plan-upstream-jyutping-composition.md`;
- `docs/plans/completed/m37-plan-engine-hyper-optimization.md`.

Those rows are not current canonical `jyut6ping3` candidate-oracle claims. The
current-facing docs, dashboards, support contract, roadmap, requirements ledger,
decision log, and M58 plan now use the D-31 split and preserve TypeDuck
candidate rows only as profile-lane grandfathered guards.

## No Rerun Required

- M55/M56/M57 Track A native performance and macOS verification evidence are
  upstream Luna/Track A scoped and remain valid.
- TypeDuck profile comment/profile/display behavior and historical
  fixture-backed candidate guards remain valid when labeled as TypeDuck profile
  evidence.
- Track B product-performance evidence remains valid as a TypeDuck profile
  guard lane with no librime peer.

## Fresh Evidence Deferred To M58

M58 must capture fresh canonical evidence before code changes for:

- canonical `jyut6ping3` candidate ordering;
- segmentation and fallback behavior;
- completion behavior;
- any reachability/admission fix;
- schema-source provenance for `rime/rime-cantonese` versus TypeDuck-HK/schema.
- the shipped TypeDuck/profile `beingo` / 畀 report if canonical captures do not
  reproduce it;
- schema/profile predicate and id-direction blast radius before any rename.

If canonical upstream captures cannot be produced or contradict the proposed
fix, M58 must stop for an explicit decision rather than substituting TypeDuck
v1.1.2 candidate output as canonical. If the reported product bug is
TypeDuck-profile-only, M58 must still disposition that shipped lane explicitly.

## Verification

The closeout verification for this audit is:

- `git diff --check`;
- a stale-claim scan for active wording that treats TypeDuck v1.1.2 as the
  canonical `jyut6ping3` behavior source;
- a positive scan proving `jyut6ping3_typeduck`, `rime-cantonese`, D-31, and
  the M58 preflight audit are visible in current docs.
