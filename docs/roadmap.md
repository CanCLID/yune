# Roadmap

Yune is a Rust input-method engine that uses **upstream librime as a
compatibility and performance oracle** while building a cleaner Rust engine.
This file is the live dashboard: it records current state, next decisions,
scope boundaries, and readiness gates. Completed milestone detail lives in
[`ledgers/milestone-history.md`](./ledgers/milestone-history.md), completed
plans, reports, and evidence folders.

Current status: Phase 1 named-target compatibility is complete; M47's portable
TypeDuck/Jyutping keyboard memory work is complete for the Windows proxy; M51
froze the engine support contract and ABI boundaries; M53 re-verified the
engine docs and
public claims for release-readiness; M54 adds native octagram-compatible grammar
support for the named upstream `luna_pinyin` target without implementing the
librime C++ plugin ABI; WEB-04 makes that path observable in `yune-web` through
a default-off debug profile. M55 closes under the 2026-07-04 **corrective
re-baseline**: real graph-work wins landed (long rows improved ~35%, startup and
session measured faster than librime), three pre-corrective closeout mechanisms
were identified as measurement artifacts and reverted, the benchmark now reads
context after every keypress, the then-current `YUNE-POET/2` byte-backed poet
storage was an explicit opt-in (the latency ceilings bind), and the corrective
`reports/evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv` is the
standing native Track A regression gate (green twice, with a fresh M56 closeout
proof that passes with tight headroom). M56 adds productization
hardening for external frontends: compiled-artifact staleness fails loudly or
rebuilds on real paths, cold/warm conformance covers `luna_pinyin` and TypeDuck
`jyut6ping3`, user-data lifecycle gaps are dispositioned, and all discovered C
ABI exports are guarded by an abuse-suite-ratcheted panic boundary without ABI
widening. The corrected M56 closeout keeps optional poet storage out of the
default product schema payload. M59's in-progress sentence/phrase ordering
work extends the current artifact to `YUNE-POET/3`; stale `/2` artifacts are
rejected and rebuilt rather than interpreted under the expanded layout.
Independent macOS native verification then
exposed a Yune-side Track A sentence-model construction anomaly in the long and
abbreviation rows; M57 repaired that platform/comparability defect by accepting
the macOS upstream Luna MARISA checksum pair through the target-scoped compact
compiled-table path, restoring the expected sentence-model owner shape, and
proving two full macOS native verification passes. The macOS bundle is now valid
evidence for the repaired rows, not a new general performance verdict. M58 then
rebased canonical Jyutping candidate claims on upstream `rime/librime 1.17.0`
plus pinned `rime/rime-cantonese`, confirmed the reported `zijiguk` / `諮議局`
case is first under the canonical oracle, fixed current `yune-web`
TypeDuck/profile `beingo` / 畀 and `zi` / 諮 reachability by preserving
short-input profile-ranked paging, and kept the schema-id split audit-only
pending explicit sign-off. The authoritative Windows M59 Increment 4a packet is
implemented at `ca52ec42` and review-fixed at `2257fbbe`; its strict Lane A
comparator remains `2/5`. On 2026-07-11 the owner renewed the narrowly scoped
D-48 class-3 exception for all `6,086` captured equal-weight inversions, with
zero cross-weight inversions, no beyond-oracle-depth use, and the recorded
cross-weight/provenance/common-input-page-1 revisit triggers; 4b is permitted
to start.
A separate macOS follow-up, measured only at source `89875ee2`, repairs the
expanded upstream Luna 37/59 page-shape defect, preserves compiled natural-log
weight semantics and the inclusive 5% pronunciation boundary across stored
`f32`, and matches both pinned pages in five fixed-binary rounds. It is a
source-scoped diagnostic, not a combined-source measurement or a new Mac
performance gate, threshold, milestone, or change to the independently recorded
Windows disposition. It did not cause or supersede that disposition.
The current-main post-fix root-cause diagnostic at `afb7079b` now supersedes
that packet's performance interpretation while retaining its repair evidence.
Across all 17 Track A rows, Yune wins six aggregate rows and loses eleven. The
apparent 37/59 wins (`0.399x` / `0.205x`) are concentrated in intermediate
prefixes whose candidate text differs from librime; on candidate-text-matched
prefixes Yune is `1.420x` / `1.204x`, and no complete long-prefix snapshot is
oracle-exact. Short rows also execute materially more work (`n` `8.682x`, `zh`
`4.092x` librime's instructions). macOS Nano allocation contributes to the
ratio but does not explain the behavior or work-volume gaps. The durable
diagnosis is
[`reports/evidence/m59-post-fix-root-cause-20260711/`](./reports/evidence/m59-post-fix-root-cause-20260711/).
It is diagnostic only: no signed Windows ceiling, baseline, exception, or
milestone changes.

> **Compatibility oracle.** Upstream librime latest stable is the default
> behavior reference for user-visible schema semantics, standard ABI contracts,
> deployed data, and migration. The current pinned upstream target is
> `rime/librime 1.17.0`
> (`33e78140250125871856cdc5b42ddc6a5fcd3cd4`):
> <https://github.com/rime/librime>. This is a referenced upstream repository,
> not a local checkout path.

## Document Map

- This file - current engine roadmap dashboard, next sequence, scope
  boundaries, and readiness gates.
- [`conventions.md`](./conventions.md) - architecture, stack, repo structure,
  coding/testing conventions, C ABI rules, integrations, and current risks.
- [`contracts/engine-support-contract.md`](./contracts/engine-support-contract.md)
  - supported engine targets, evidence-lane rules, ABI boundaries, and profile
  accessors.
- [`requirements.md`](./requirements.md) - requirement IDs, status,
  traceability, and closeout counts.
- [`decisions.md`](./decisions.md) - standing principles and decision log.
- [`ledgers/milestone-history.md`](./ledgers/milestone-history.md) - completed
  milestone ledger and historical closeout pointers formerly carried in this
  roadmap.
- [`reports/yune-vs-librime-performance.md`](./reports/yune-vs-librime-performance.md)
  and [`reports/yune-vs-librime-root-cause-analysis.md`](./reports/yune-vs-librime-root-cause-analysis.md)
  - current performance comparison and diagnosis.
- [`reports/evidence/m59-post-fix-root-cause-20260711/`](./reports/evidence/m59-post-fix-root-cause-20260711/)
  - current-main macOS post-fix behavior/performance diagnosis and bounded
    five-round evidence packet.
- [`reports/ios-memory-budget.md`](./reports/ios-memory-budget.md) - native
  single-active-schema memory versus the iOS keyboard-extension budget; current
  values are Windows proxies, not Apple `phys_footprint`.
- [`plans/completed/m52-plan-track-a-guardrails-and-disposition.md`](./plans/completed/m52-plan-track-a-guardrails-and-disposition.md)
  - latest native Track A guardrail and blocker-disposition milestone.
- [`plans/completed/m55-plan-native-track-a-match-or-beat-program.md`](./plans/completed/m55-plan-native-track-a-match-or-beat-program.md)
  - native Track A performance milestone, closed under the 2026-07-04
    corrective re-baseline
    ([`corrective record`](./reports/evidence/m55-native-match-or-beat/corrective-2026-07-04/README.md));
    owns the standing per-key native ratchet.
- [`plans/completed/m56-plan-engine-productization-hardening.md`](./plans/completed/m56-plan-engine-productization-hardening.md)
  - engine productization hardening milestone for staleness-proofing,
    cold/warm product-path conformance, user-data lifecycle, and ABI crash
    policy.
- [`plans/completed/m57-plan-macos-track-a-sentence-model-parity.md`](./plans/completed/m57-plan-macos-track-a-sentence-model-parity.md)
  - macOS Track A sentence-model parity and verification repair milestone.
- [`plans/completed/m54-plan-native-octagram-grammar-support.md`](./plans/completed/m54-plan-native-octagram-grammar-support.md)
  - native octagram-compatible grammar support milestone.
- [`plans/completed/m51-plan-engine-support-contract-abi-freeze.md`](./plans/completed/m51-plan-engine-support-contract-abi-freeze.md)
  - engine support contract and ABI freeze milestone.
- [`plans/completed/m47-plan-ios-budget-native-memory-reduction.md`](./plans/completed/m47-plan-ios-budget-native-memory-reduction.md)
  - portable TypeDuck/Jyutping keyboard memory reduction milestone.
- [`plans/completed/web03-plan-three-schema-launch-readiness.md`](./plans/completed/web03-plan-three-schema-launch-readiness.md)
  - launch compiled-asset contract and browser remeasure milestone.
- [`plans/completed/web04-plan-octagram-debug-harness-luna-pinyin.md`](./plans/completed/web04-plan-octagram-debug-harness-luna-pinyin.md)
  - browser octagram debug harness and default-off Luna profile milestone.
- [`plans/`](./plans) - active, reference, and completed plans.

> The GSD planning system (`.planning/`) has been retired; durable planning now
> lives under `docs/`.

## Current Snapshot

| Lane | Current state | Next decision or gate |
| --- | --- | --- |
| Engine performance | M55 remains closed under the 2026-07-04 corrective Windows re-baseline and the signed M59 ceilings remain authoritative. M57 repaired the macOS compact-table sentence-model construction defect. The current-main macOS post-fix packet at `afb7079b` supersedes the earlier `89875ee2` performance read: aggregate 37/59 ratios are `0.399x` / `0.205x`, but the wins are dominated by behavior-different prefixes; the text-matched sensitivity ratios are `1.420x` / `1.204x`. Current main wins 6/17 aggregate Track A rows, while `n`/`zh` use `8.682x`/`4.092x` librime's instructions. Track B behavior/checksums remain M57-exact while its work shape moved. | Keep the signed Windows ratchet unchanged. Before optimizing, lock incremental Luna prefix/page behavior to the named oracle. Then attribute the translator residual and test behavior-preserving lazy page fill; follow with short MARISA/abbreviation work, byte-backed POET behavior/memory, and an exact-current matched Windows/macOS lane. No milestone is created by the diagnostic. |
| TypeDuck/Jyutping product memory | M47's portable scope is complete. The comments-intact `jyut6ping3_mobile` keyboard profile reached about `67 MB` working set / `22 MB` private on Windows proxy evidence, with table, prism, and rich lookup/comment payloads byte-backed from compiled storage. | Apple `phys_footprint` proof remains unnumbered far-future platform validation. Optional RED-09/10/11-style polish needs a fresh owner-ranked plan. |
| Web harness startup and memory | WEB-03 fixed the launch compiled-asset contract and the stale Jyutping source-fallback owner. WEB-04 adds a default-off `luna_pinyin_octagram` debug profile that fetches a pinned lotem `.gram` locally, delivers it only for the octagram profile, exposes delivered/fallback/checksum/schema-select high-water memory diagnostics, proves all four named ranking rows versus plain Luna in Playwright, and proves browser fail-closed behavior. Current dashboard fair `luna_pinyin` browser comparison is `64.0 MiB` peak versus My RIME `16.0 MiB`; old Jyutping `893.1 MiB` remains only as a synthetic no-launch-assets negative control. | Future browser memory work should target the fair `luna_pinyin` runtime high-water floor or another freshly measured owner, not another payload-only or stale-asset branch. Broader contextual suggestions or non-debug octagram product UX needs a new scoped plan. |
| AI-native engine layer | M11/M13 proved a default-off local AI layer can sit on top of the deterministic engine. | Keep AI outside the classic deterministic performance path unless a named engine experiment explicitly enables it. |
| Future platform work | Platform-specific native frontends remain outside this repo roadmap. | Start a separate repository or separate plan before changing platform/application contracts. |

## Current Guardrails

The current native Track A regression gate is the M55 **corrective per-key**
artifact (the benchmark reads context after every keypress; batch-shaped
pre-corrective results, including the retracted phase-5 run 5/6 numbers, are
not comparable):

- Threshold source:
  [`reports/evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv`](./reports/evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv)
- Consecutive green proof runs:
  [`corrective-2026-07-04/gate-run-d/threshold-check.csv`](./reports/evidence/m55-native-match-or-beat/corrective-2026-07-04/gate-run-d/threshold-check.csv)
  and
  [`corrective-2026-07-04/gate-run-e/threshold-check.csv`](./reports/evidence/m55-native-match-or-beat/corrective-2026-07-04/gate-run-e/threshold-check.csv)
- Latest closeout proof:
  [`reports/evidence/m56-productization-hardening/final/ratchet-run/threshold-check.csv`](./reports/evidence/m56-productization-hardening/final/ratchet-run/threshold-check.csv)
- Manual command shape:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 `
  -OutputRoot docs\reports\evidence\<new-run> `
  -Iterations 9 -SessionIterations 60 -KeyIterations 80 `
  -TrackAInputs n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru `
  -TrackBInputs neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung `
  -DeployProductBeforeBenchmark `
  -TrackAThresholds docs\reports\evidence\m55-native-match-or-beat\thresholds\m55-thresholds.csv `
  -FailOnRegression
```

The gate is local/manual because it needs same-run librime `1.17.0` artifacts.
Do not summarize "M55 guardrails pass" as "Yune is always faster than librime."
The gate proves the named native Track A and Track B regression rows against
their committed ceilings. Rows below `1.00x` may be described as faster in that
lane (startup, session, `zhongguo`, both abbreviation rows); the short keys and
both sentence rows are bounded-gap passes, not match-or-beat rows.

## Performance North Star

Broad, unqualified claims that "Yune is faster than librime" are not supported
by current evidence. Current performance is lane-specific: under the M55
corrective per-key gate, Yune is faster than librime on startup, session,
`zhongguo`, and both abbreviation rows, and slower (bounded, guarded) on the
short keys and both sentence rows; Track A memory remains far above librime's
peer process (`185.7 MB` default vs `13.5 MB`, with a `113.2 MB` byte-backed
opt-in that is latency-blocked). Browser performance remains a separate
evidence lane. The M55 pre-corrective closeout is a standing lesson: a gate is
only as honest as the shape of its measurement, and closeout claims are
verified against what the metric actually measures.

A future milestone that aims to **surpass librime** must be scoped as
performance research, not launch-readiness cleanup. It should:

- name one lane first, such as native Track A `luna_pinyin`, TypeDuck/Jyutping
  product profile, or browser fair-lane memory;
- capture fresh same-run Yune/librime evidence and a noise band before code
  changes;
- choose one structural owner before editing code;
- set a real win bar, such as `<=1.0x` median on selected latency rows or a
  measured memory target, rather than only a no-regression ceiling;
- preserve oracle behavior and the M51 ABI contract; and
- close as partial/no-go if the owner is not real or the win requires
  unacceptable parity risk.

The current native order is behavior-first: lock incremental Luna prefix/page
semantics, attribute the translator residual, then test filter-aware lazy page
fill and short MARISA/abbreviation reductions. Native memory must recover
behavior-valid `/3` byte-backed output before it can claim or optimize a memory
lane. Exact-current Windows/macOS attribution and Track B overfetch follow.
Browser `luna_pinyin` runtime-floor/startup work remains a separate plan. Every
lane needs fresh evidence and must preserve oracle behavior and the M51 ABI
contract.

## Closing The 188 MB Native Track A Memory Gap

M55 built the machinery to close this gap but the gap itself is **not closed
in the shipping default**. Phase 1 attributed the old `105.6 MB` unclassified
floor, Phase 2/2R proved the poet payload owner and redesigned the artifact as
`YUNE-POET/2` (parity-preserving, `~113.2 MB` peak), and Phase 3R landed real
graph-volume reductions. M59's current `YUNE-POET/3` format supersedes `/2`
for compatibility and rebuild purposes, without inheriting `/2`'s measured
memory result. Under the corrective per-key gate, however,
byte-backed access without the incremental sentence scratch costs
`4.6x`/`3.2x` on the long rows, so per the plan's own decided call (latency
ceilings bind) the default stays on the owned path at `185.7 MB`.

Current decision:

1. **The corrective M55 artifact is the standing native Track A gate**
   (startup, session, eight key rows, peak memory, win rows `<1.00x`, Track B
   absolutes; green twice on 2026-07-04).
2. **The `/2` byte-backed result is historical.** `113.2 MB` is a valid M55
   opt-in record, but `/3` cannot inherit it. The current deployed `/3` control
   emits zero candidates on all 99 prefixes and is rejected for behavior.
3. **Behavior precedes the next memory decision.** Recover identical `/3`
   prefix/page output first, then design incremental/lazy indexing and measure
   memory and CPU separately under the standing gate. Do not assume a direct
   scratch port is sufficient: the behavior-valid fixture multiplies logical
   table/graph/DP work.

## Authoritative Sequence

1. **M53 engine release-readiness audit is complete.** The five-dimension audit
   (support-contract consistency, ABI-wording-vs-code, M52 guardrail freshness,
   public claim wording, link/evidence integrity) found the substantive
   invariants clean with no ABI/guardrail/link drift; the only real defects were
   public-facing claim drift in `README.md` (and one linked archived report)
   across performance, compatibility scope, oracle-precedence,
   frontend-validation, ABI/drop-in, and Rust safety/lint-scope wording, now
   corrected to contract-accurate, M52 lane-specific wording. Evidence:
   [`reports/evidence/m53-engine-release-readiness-audit/`](./reports/evidence/m53-engine-release-readiness-audit/).
   Plan:
   [`plans/completed/m53-plan-engine-release-readiness-audit.md`](./plans/completed/m53-plan-engine-release-readiness-audit.md).
2. **M54 native octagram-compatible grammar support is complete.** Yune now
   supports the named upstream `luna_pinyin` octagram target through a native
   Rust `Grammar` provider and logical `.gram` resource loading. Evidence:
   [`reports/evidence/m54-native-octagram-grammar-support/`](./reports/evidence/m54-native-octagram-grammar-support/).
   Plan:
   [`plans/completed/m54-plan-native-octagram-grammar-support.md`](./plans/completed/m54-plan-native-octagram-grammar-support.md).
3. **WEB-04 octagram debug harness is complete.** The tracked `apps/yune-web`
   harness now exposes `luna_pinyin_octagram` as a default-off profile, delivers
   the pinned lotem `.gram` through `extraSharedAssets`, records diagnostics,
   and proves all four octagram ranking rows against plain Luna, the plain-Luna
   negative control, and missing-model fail-closed behavior in Playwright.
   Evidence:
   [`reports/evidence/web04-octagram-debug-harness/`](./reports/evidence/web04-octagram-debug-harness/).
   Plan:
   [`plans/completed/web04-plan-octagram-debug-harness-luna-pinyin.md`](./plans/completed/web04-plan-octagram-debug-harness-luna-pinyin.md).
4. **M55 native Track A program is closed under the 2026-07-04 corrective
   re-baseline.** Real wins landed (long rows improved ~35%, startup/session
   faster than librime, Track B guard rows improved, `YUNE-POET/2` byte-backed
   storage built and parity-preserving); the pre-corrective closeout's
   headline numbers were retracted as measurement artifacts (key deferral,
   benchmark-input aliases, uninvalidated config cache — all reverted), the
   benchmark now reads context per keypress, byte-backed poet is an explicit
   opt-in, and the corrective ratchet is the standing gate, green twice. No
   ABI/browser/product/platform scope widening. Corrective record:
   [`reports/evidence/m55-native-match-or-beat/corrective-2026-07-04/`](./reports/evidence/m55-native-match-or-beat/corrective-2026-07-04/).
   Plan:
   [`plans/completed/m55-plan-native-track-a-match-or-beat-program.md`](./plans/completed/m55-plan-native-track-a-match-or-beat-program.md).
5. **M56 engine productization hardening is complete.** The engine now has
   inventory-backed compiled-artifact staleness policy and tests, cold/warm
   conformance for `luna_pinyin` and TypeDuck `jyut6ping3`, user-data lifecycle
   coverage, all-discovered-export FFI guards, an ABI abuse suite, and an
   explicit release panic strategy (`panic = "abort"` retained). No ABI
   widening, no behavior change on defined happy paths. Evidence:
   [`reports/evidence/m56-productization-hardening/`](./reports/evidence/m56-productization-hardening/).
   Plan:
   [`plans/completed/m56-plan-engine-productization-hardening.md`](./plans/completed/m56-plan-engine-productization-hardening.md).
6. **WEB-05 harness control surface is complete.** The "surface all controls"
   follow-up carved out of M21 now exposes every engine control/diagnostic
   reachable through existing `apps/yune-web` seams in the playground, with
   dev-power controls gated out of the public demo by the shared
   `IS_PUBLIC_DEMO` constant. Corrective same-WASM Playwright evidence captures
   the default-behavior baseline from parent `a87c6b88`; demo-mode evidence
   proves the new debug/admin controls and WEB-05 raw/cache/asset data pulls are
   absent from the public demo. Named follow-ups remain for persisted-config
   deploy-cache freshness and the current Extended charset browser-effect N/A
   row. No engine or web-runtime package changes. Evidence:
   [`reports/evidence/web05-control-surface/`](./reports/evidence/web05-control-surface/).
   Plan:
   [`plans/completed/web05-plan-harness-control-surface.md`](./plans/completed/web05-plan-harness-control-surface.md).
7. **M57 macOS Track A sentence-model parity and verification repair is
   complete.** The independent macOS rerun found that local librime stayed
   oracle-shaped, but Yune built a different Luna sentence-model shape:
   `poet.entries_by_code` dropped from the Windows corrective `513,353` owner
   shape to `191,984`, `poet.abbreviation_vocabulary` expanded from the 11-row
   M42 target set to the full `421,966` vocabulary, the two abbreviation rows
   skipped abbreviation discovery, and the long rows exploded graph work. M57
   fixed the Yune-side construction defect by accepting the macOS upstream Luna
   MARISA checksum pair (`0xb3d4e98e` / `0x29d56c89`) behind the existing
   target gate, restored compact model counts (`332,604` codes, `513,353`
   expanded entries, 11-row abbreviation vocabulary), and produced two full
   macOS native verification passes. Evidence:
   [`reports/evidence/m57-macos-track-a-sentence-model-parity/`](./reports/evidence/m57-macos-track-a-sentence-model-parity/).
   Plan:
   [`plans/completed/m57-plan-macos-track-a-sentence-model-parity.md`](./plans/completed/m57-plan-macos-track-a-sentence-model-parity.md).
8. **M58 Jyutping oracle rebase and TypeDuck/profile bug disposition is
   complete for reachability, with a recorded performance residual.** Canonical Cantonese/Jyutping candidate ordering, segmentation,
   fallback, and completion are now evidenced from upstream `rime/librime
   1.17.0` plus pinned `rime/rime-cantonese`, not TypeDuck-HK/librime v1.1.2.
   The user-specified `zijiguk` input for `諮議局` captures `諮議局` as the first
   canonical candidate, so no canonical admission issue is reproduced. The
   shipped/current `yune-web` TypeDuck/profile lane still had page-navigation
   reachability bugs for `beingo` / 畀 and `zi` / 諮; M58 fixes them by retaining
   enough TypeDuck/profile-ranked candidates for page-size-6 browser paging
   without promoting the targets onto the first page, and the long-composition
   follow-up lets `zijiguk` page to standalone `諮`, commit `諮`, and recompose
   `jiguk`. Historical M14-M28 TypeDuck fixtures remain profile-lane regression
   guards. The standing M55/Track B ratchet was run twice after the follow-up
   and failed the recorded Track A `ni`/`hao`/`zhongguo` rows plus the Track B
   long-Jyutping latency row; that miss remains a named performance residual.
   The preferred schema split
   remains canonical `jyut6ping3` plus TypeDuck `jyut6ping3_typeduck`, but no
   id split or userdb/cache-key migration landed; Phase 3 records the blast
   radius pending explicit sign-off. Evidence:
   [`reports/evidence/m58-jyutping-exact-before-fuzzy/`](./reports/evidence/m58-jyutping-exact-before-fuzzy/).
   Plan:
   [`plans/completed/m58-plan-jyutping-exact-before-fuzzy-candidate-order.md`](./plans/completed/m58-plan-jyutping-exact-before-fuzzy-candidate-order.md).
9. **Future browser fair-lane memory slice** - the fair `luna_pinyin` browser
   high-water floor or another freshly measured owner, only with a new scoped
   plan.
10. **Future AI-native engine experiments** - later, and only after classic
   engine performance is no longer dominated by avoidable pipeline costs.
11. **Future TypeDuck/profile-storage slices** - only with a new scoped plan,
   fresh owner evidence, and no TypeDuck-profile speed claim unless the profile
   row is explicitly selected as the target.
Trigger-gated, not scheduled: extracting the full processor pipeline from
`yune-rime-api` into `yune-core` lands only when a real non-ABI consumer needs
the full input path. Do not milestone that extraction speculatively.

## Historical Closeouts

Detailed closeout narratives for completed milestones are now owned by
[`ledgers/milestone-history.md`](./ledgers/milestone-history.md), completed
plans, and report/evidence folders. This roadmap keeps only the live dashboard
and current decision rules.

## Track Map

| Track | Scope | Current source of truth |
| --- | --- | --- |
| Engine performance | Native engine startup, schema/session lifecycle, mmap-backed `rsmarisa` marisa-table lookup, lazy/page-bounded translation, context export, memory, allocation, Track A guardrails, M55 final evidence, M57 macOS Track A sentence-model repair, historical native Track A research no-go evidence, and TypeDuck/Jyutping profile storage | M57 completed plan/evidence, M55 final threshold/evidence, M52 historical guardrail evidence, M50 plan/evidence, M47 plan/evidence, and performance reports. |
| Web harness startup and memory | Tracked `apps/yune-web/` production build, public-demo dist, browser shell, asset/cache delivery, worker/WASM startup, persistence, schema selection, first key-to-paint, Chromium memory, compiled-asset contract, debug-only octagram harness diagnostics, and WEB-05 control/diagnostic surface | WEB-05 plan/evidence, WEB-04 plan/evidence, WEB-03 plan/evidence, WEB-02 owner classification, WEB-01 measured no-go, M41 startup evidence, and browser reports. |
| Core compatibility | Upstream behavior fixtures, standard ABI-observable behavior, staleness policy, user-data lifecycle, and ABI crash/threading/poison policy | Requirements, decisions, engine support contract, per-milestone plans, M53 release-readiness audit (`reports/evidence/m53-engine-release-readiness-audit/`), and M56 productization evidence. |
| AI-native engine research | Default-off AI behavior layered above the deterministic engine | Future explicit engine experiments only. |
| Historical record | Completed milestone outcomes and reference/provenance pointers | Milestone history ledger. |

## Milestone Ledger

| Milestone or track | Status | Current roadmap meaning |
| --- | --- | --- |
| M0-M36 | Complete | Historical compatibility, frontend-validation, browser, product, and early performance build-out; see the milestone history ledger. |
| M37-M45 | Complete / measured blockers | Native and browser performance history leading to the M45/WEB-01/M46 handoff; see the history ledger and completed plans. |
| WEB-01/02/03 | Complete | Browser memory attribution, stale-asset owner classification, and launch compiled-asset contract. |
| WEB-04 | Complete | Default-off `luna_pinyin_octagram` browser debug harness with non-vendored pinned lotem `.gram`, delivered/fallback/checksum/schema-select high-water diagnostics, Playwright all-row ranking proof versus plain Luna, plain-Luna negative control, and missing-model fail-closed evidence. Plan: [`plans/completed/web04-plan-octagram-debug-harness-luna-pinyin.md`](./plans/completed/web04-plan-octagram-debug-harness-luna-pinyin.md). |
| M47 | Complete for portable scope | TypeDuck/Jyutping comments-intact keyboard memory is under the Windows private/dirty proxy target; Apple `phys_footprint` proof remains parked. |
| M48-M52 | Complete | Engine correctness, support-contract, and historical Track A guardrail closeouts; M52 is superseded by the corrective M55 per-key gate (the metric changed: context is read per keypress). |
| M53 | Complete | Engine release-readiness audit (docs/evidence only): five-dimension consistency/ABI/guardrail/claim/link audit with adversarial verification; substantive invariants clean, no drift; corrected stale `README.md`/archived public-claim wording to the contract-accurate M52 lane-specific record. Plan: [`plans/completed/m53-plan-engine-release-readiness-audit.md`](./plans/completed/m53-plan-engine-release-readiness-audit.md). |
| M54 | Complete | Native octagram-compatible grammar support for the named upstream `luna_pinyin` target, with pinned lotem oracle data, RIME-LMDG validation evidence, external model checksums, clean-room Rust `.gram` parsing/scoring, null-grammar and TypeDuck regression gates, and no public C ABI change. Plan: [`plans/completed/m54-plan-native-octagram-grammar-support.md`](./plans/completed/m54-plan-native-octagram-grammar-support.md). |
| M55 | Complete (corrective re-baseline) | Native Track A performance program, closed 2026-07-04 after a corrective review. Real: 37-char `3.05x -> 1.913x`, 59-char `2.25x -> 1.528x`, `ni` `2.433x`, `hao` `1.574x`, startup `0.895x`, session `0.864x`, win rows locked `<1.00x`, Track B guards green and tightened (~3x better startup/session absolutes), `YUNE-POET/2` byte-backed poet built (opt-in `113.2 MB`, latency-blocked; default owned `185.7 MB`). Retracted as measurement artifacts: the pre-corrective `0.237x`/`0.086x`/`0.286x` rows (key deferral, benchmark-input aliases, uninvalidated config cache - all reverted; benchmark now reads context per keypress). Corrective gate green twice. Corrective record: [`reports/evidence/m55-native-match-or-beat/corrective-2026-07-04/`](./reports/evidence/m55-native-match-or-beat/corrective-2026-07-04/). Plan: [`plans/completed/m55-plan-native-track-a-match-or-beat-program.md`](./plans/completed/m55-plan-native-track-a-match-or-beat-program.md). |
| M56 | Complete | Engine productization hardening for external Windows/iOS frontend consumers: staleness-proofing + isolated cold/warm conformance, user-data lifecycle evidence, ABI abuse suite + panic-boundary guards, session-registry poison recovery, and explicit release `panic = "abort"` policy; no ABI change, behavior-preserving on defined happy paths, and no new default product `*.poet.bin` payloads. Evidence: [`reports/evidence/m56-productization-hardening/`](./reports/evidence/m56-productization-hardening/). Plan: [`plans/completed/m56-plan-engine-productization-hardening.md`](./plans/completed/m56-plan-engine-productization-hardening.md). |
| M57 | Complete | macOS Track A sentence-model parity and verification repair. The 2026-07-04 macOS rerun found a Yune-side model-shape defect, not an oracle/librime contradiction: long rows exploded graph work, abbreviation rows skipped M42 abbreviation discovery, and `poet.abbreviation_vocabulary` reported the full `421,966` vocabulary instead of the 11-row target set. M57 accepts the macOS upstream Luna MARISA checksum pair under the existing target gate, restores compact model construction (`332,604` codes, `513,353` expanded entries, 11-row abbreviation vocabulary), and records two full macOS native passes. Evidence: [`reports/evidence/m57-macos-track-a-sentence-model-parity/`](./reports/evidence/m57-macos-track-a-sentence-model-parity/). Plan: [`plans/completed/m57-plan-macos-track-a-sentence-model-parity.md`](./plans/completed/m57-plan-macos-track-a-sentence-model-parity.md). |
| WEB-05 | Complete | Harness control surface: 108-row control/diagnostic ledger, 13 retained Phase 1 surface rows implemented through existing `apps/yune-web` seams, unsupported key-binder shortcut reference classified `no-surface`, parent-baseline same-WASM default behavior unchanged, public demo debug/admin controls plus WEB-05 raw/cache/asset data pulls gated hidden, and `debug.storage` plus `get_option` read-back deferred to their proper runtime/engine lanes. Named follow-ups: persisted-config deploy-cache freshness and current Extended charset browser-effect N/A. Evidence: [`reports/evidence/web05-control-surface/`](./reports/evidence/web05-control-surface/). Plan: [`plans/completed/web05-plan-harness-control-surface.md`](./plans/completed/web05-plan-harness-control-surface.md). |
| M58 | Complete for reachability; perf residual recorded | Canonical Jyutping candidate behavior was recaptured from upstream `rime/librime 1.17.0` plus pinned `rime/rime-cantonese`; TypeDuck-HK/librime v1.1.2 remains profile-only for multilingual/comment/profile and grandfathered candidate guards. The reported `zijiguk` / `諮議局` case is canonical-first and does not reproduce a canonical issue. The shipped/current `yune-web` TypeDuck/profile lane now reaches `beingo` / 畀 at TypeDuck/profile index 6 and `zi` / 諮 at index 27 through page-size-6 browser paging without first-page promotion; the post-closeout long-composition corrective also lets `zijiguk` page to standalone `諮`, select it, commit only `諮`, and recompose `jiguk`. The public dictionary row is restored to the TypeDuck source `畀	bei2	200000` value/order and compiled schema assets were regenerated. The standing M55/Track B ratchet failed twice after the long-composition follow-up (`ni`, `hao`, `zhongguo`, and Track B long-Jyutping latency), so that performance residual is recorded separately from the candidate-behavior closeout. No schema id split landed; the `jyut6ping3_typeduck` direction remains sign-off gated after the blast-radius audit. Evidence: [`reports/evidence/m58-jyutping-exact-before-fuzzy/`](./reports/evidence/m58-jyutping-exact-before-fuzzy/). Plan: [`plans/completed/m58-plan-jyutping-exact-before-fuzzy-candidate-order.md`](./plans/completed/m58-plan-jyutping-exact-before-fuzzy-candidate-order.md). |
| M59 | In progress — full Path A locked, NOT closeable | General single-character reachability so the owner can compose an arbitrary non-lexicon phrase one character at a time. GPT's first execution (`77a9540a`) was gamed (per-input `match` arms replaying oracle candidates baked into engine `.tsv` + circular tests) and was reverted (`c70774ce`), keeping the real `ni`/`hao` perf fix. **Lane B reachability was then reimplemented as a genuine general mechanism and landed (`c89a8ea9`)** with structure-driven anti-gaming controls; its D-48 exact-order lane remains open. The binding owner amendments make this a default-on, zero-per-schema-adaptation guarantee for every current and future schema and require D-48 page/prefix-exact ordering for the exact 13-input canonical rime-cantonese capture, upstream Luna, and Cangjie, with only owner-signed exceptions and no promotion/input/oracle hacks. **Full Path A is locked:** nothing remaining is moved to M60 or another milestone. Workspace deployment fidelity completed at `2ee0805f`; unified TypeDuck/profile navigation completed at `e37ee011`; compiled `sort: original` completed through `d55b203e`; schema-general transformed-algebra reachability completed at `2cb7e411`; Increment 4a's sentence/phrase mechanism landed at `ca52ec42`; and its owner-provided Opus blocking review is fixed forward at `2257fbbe`. The exact explicit-false deployment matrix, full workspace clippy, focused parity, and a fresh 32/32 signed post-fix ratchet guard are green. Its strict five-row Lane A comparator remains visibly red (`2/5` exact): the remaining surface is deterministically classified as predeclared 4c OpenCC plus equal-weight residue. On 2026-07-11 the owner renewed the narrowly scoped D-48 class-3 exception for the complete captured equal-weight residual (`6,086` inversions, zero cross-weight inversions, no beyond-oracle-depth use) with mandatory cross-weight/provenance/common-input-page-1 revisit triggers; 4b is permitted to start. M59-REACH-02 is complete. Remaining close blockers are canonical manifest-to-acceptance reconciliation (REACH-03); 4b abbreviation/segmentation, 4c OpenCC variants, 4d Cangjie CJ-1, and 4e Lane B exact order; final executable-evidence reconciliation; five final expanded Track A/Track B rounds from the final behavior commit; and the exact native release, source-current WASM, runtime/app, manifest, Playwright, browser, and packaging gates. Abbreviation-graph 4b is likewise review-blocking, and a red 4b short-key ratchet requires an owner decision rather than a quiet re-baseline or revert. Evidence remains under [`reports/evidence/m59-canonical-jyutping-reachability-parity/`](./reports/evidence/m59-canonical-jyutping-reachability-parity/). Plan: [`plans/active/m59-plan-canonical-jyutping-reachability-parity.md`](./plans/active/m59-plan-canonical-jyutping-reachability-parity.md). |

**M59 supplemental macOS diagnostic and resolution note (2026-07-10/11; no
milestone or active-scope change):** At exact signed Increment-0 source
`457751824b8944676dc44912b9ce31ff29d78403`, a read-only macOS diagnostic
reproduced the [already disclosed M55 expanded-Luna first-page debt](./reports/evidence/m55-native-match-or-beat/phase-3r-fixture-expansion/README.md)
on the 37- and 59-character rows: pinned librime emits its one-best full
sentence followed by shorter phrase candidates, while Yune exposes up to five
full-span sentence paths. The supplemental follow-up repairs that cross-platform
Yune defect and a compiled-log weight-domain error at independently measured
source `89875ee2`; five fixed-binary macOS rounds match both pinned-librime long
pages exactly. The Mac-versus-Windows latency comparison remains diagnostic
only, with signed ceilings unchanged. This evidence neither measures the
combined/reconciled source nor changes the authoritative Windows Increment 4a
packet, strict Lane A `2/5` result, renewed class-3 owner disposition and 4b
permission, or broader Lane B requirement; it did not cause or supersede the
owner disposition. [Original diagnostic](./reports/evidence/m59-macos-librime-analysis-20260710/README.md);
[supplemental repair evidence](./reports/evidence/m59-canonical-jyutping-reachability-parity/increment-4a-luna-script-translation-order/README.md).

## Scope Ledger

A living map so "parity" always names a target. Deferred rows move into scope
only when an engine target needs them; nothing here commits to a timeline.

| In scope - target-driven, measured | Deferred - implement when an engine target needs it | Non-goal |
| --- | --- | --- |
| `luna_pinyin` core versus upstream `1.17.0`, including M17's null-grammar sentence/lattice slice; the source-scoped M59 supplemental repair at `89875ee2` for the one-best-sentence-then-phrase first page on the 37/59 rows and compiled-log weight handling; M18 punctuation processor slices; completed M42 abbreviation sentence parity for `cszysmsrsd`/`zybfshmsru`; completed M48 `jianli`/`biancheng` over-segmentation parity; and completed M54 native octagram-compatible grammar support for the named upstream target. The broader M59 seven-row complete-list lane remains open; the supplemental measurement does not assert its state on later Windows or reconciled commits. | Broader learned `.gram`/octagram behavior, contextual translation, and plugin-backed gears beyond the named M54 target | Bit-for-bit parity with librime internals |
| Common RIME schemas added through explicit breadth milestones | Further schema breadth only with fresh oracle fixtures and owning tests | Unbounded schema checklist work |
| Canonical Cantonese/Jyutping candidate behavior: upstream `rime/librime 1.17.0` + pinned `rime/rime-cantonese`; exact Yune-facing id direction gated by M58 Phase 3 sign-off | TypeDuck multilingual comments/profile/display behavior and grandfathered profile candidate guards under current shipped ids until a signed-off split; preferred future id `jyut6ping3_typeduck` | Using bare `schema_id: jyut6ping3` as provenance when schema sources differ; renaming product ids without blast-radius/userdb evidence |
| **Compose an arbitrary non-lexicon phrase one character at a time — DEFAULT-ON for EVERY schema** (M59 owner amendment 2026-07-07, binding): `luna_pinyin`, `jyut6ping3`/rime-cantonese, **cangjie/shape schemas**, and **any FUTURE schema (e.g. `rime-teochew`) inherit it automatically on install with ZERO per-schema adaptation work.** Engine/translator-level default (opt-out only, recorded reason). Delivered IN M59. | The M60 draft's capability-contract formalism / opt-out registry may follow later, but the default-on guarantee itself is **not** deferrable out of M59 | Per-input hardcoding or baked oracle data; **any per-schema adaptation work** required to enable the feature; a schema silently failing onboarding as "unsupported" |
| Native engine performance guardrails for startup, session lifecycle, lookup, lazy/page-bounded translation, context export, memory, and allocation | Frontend/application delivery evidence and platform packaging | Claiming application-visible wins from native engine evidence |
| AI-native layer on the compatible deterministic base | Richer AI experiments after the classic engine path is competitive | Replacing or altering classic input paths by default |

## Deferred / Future

- **Far-future Apple-device memory validation:** confirm M47's ~22 MB Windows
  private/dirty proxy on real Apple hardware when a Mac/Xcode environment exists.
  Build a minimal iOS keyboard extension or macOS host loading the comments-intact
  `jyut6ping3_mobile` profile and measure `phys_footprint` in Instruments. This
  is intentionally not a numbered milestone while the current focus remains
  portable engine optimization.
- **Future M47-derived engine memory polish:** RED-09 compiled-asset/profile
  slimming, RED-10 allocator strategy, and RED-11 startup hygiene remain optional
  engine candidates. They are useful for download size, cold start, WASM, and
  conservative resident footprint, but are not required for the current
  iOS-dirty proxy result. Open them only with a fresh owner-ranked plan.
- **librime C++ plugin ABI** (Lua, dynamic octagram plugin loading, predict,
  proto): deferred until a concrete engine target requires it; M54's native
  octagram-compatible grammar support is not C++ plugin ABI support. Prefer
  Yune-native extension points first.
- **AI-native input layer beyond M13:** future work owns richer local-first AI
  behavior, privacy/memory controls, and any explicit remote-provider decision.
  Until then, proven AI remains default-off and outside the classic performance
  path.

## Principles

The standing principles that govern all current and future work - librime as
oracle, target-driven scope, support-contract/ABI boundaries, evidence-lane
separation, and upstream-first oracle sequencing - have one canonical home:
[`conventions.md`](./conventions.md), [`decisions.md`](./decisions.md), and
[`contracts/engine-support-contract.md`](./contracts/engine-support-contract.md).
