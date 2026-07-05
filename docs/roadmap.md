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
context after every keypress, `YUNE-POET/2` byte-backed poet storage is an
explicit opt-in (the latency ceilings bind), and the corrective
`reports/evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv` is the
standing native Track A regression gate (green twice, with a fresh M56 closeout
proof that passes with tight headroom). M56 adds productization
hardening for external frontends: compiled-artifact staleness fails loudly or
rebuilds on real paths, cold/warm conformance covers `luna_pinyin` and TypeDuck
`jyut6ping3`, user-data lifecycle gaps are dispositioned, and all discovered C
ABI exports are guarded by an abuse-suite-ratcheted panic boundary without ABI
widening. The corrected M56 closeout keeps optional poet storage out of the
default product schema payload. Independent macOS native verification then
exposed a Yune-side Track A sentence-model construction anomaly in the long and
abbreviation rows; M57 is drafted to repair that platform/comparability defect
before treating the macOS bundle as a replacement performance verdict.

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
| Engine performance | M55 is closed under the 2026-07-04 corrective re-baseline. Real wins: 37-char Luna `3.05x -> 1.913x`, 59-char `2.25x -> 1.528x`, `ni` `3.14x -> 2.433x`, `hao` `2.15x -> 1.574x`, startup `0.895x` and session `0.864x` (faster than librime, run-noisy), win rows locked `<1.00x`, Track B guard rows all green with startup/session ~3x better than their Phase 0 sources. Removed as measurement artifacts: the `luna_pinyin` key deferral, the `n`/`h` benchmark-input aliases, and the uninvalidated config cache; the benchmark now reads context per keypress. Byte-backed `YUNE-POET/2` poet storage is an explicit opt-in (`113.2 MB`, latency-blocked) while the shipping default stays owned (`185.7 MB`) — the latency ceilings bind. The 2026-07-04 macOS verification bundle found a Yune-side sentence-model/model-shape anomaly in the 37-char, 59-char, `cszysmsrsd`, and `zybfshmsru` rows; it is diagnostic evidence for M57, not a replacement for the M55 standing gate. | The corrective `m55-thresholds.csv` is the standing native Track A gate (green twice: `corrective-2026-07-04/gate-run-d`, `gate-run-e`; latest M56 closeout ratchet also green). M57 is drafted to fix the macOS Track A sentence-model construction and abbreviation-path defect, then re-run the macOS bundle. Future engine performance work: port the incremental sentence scratch to byte-backed storage (reclaims the memory win), then poet graph constants / short keys — each with fresh owner evidence and no ABI widening. |
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

Likely future structural owners are the browser `luna_pinyin` runtime floor,
browser startup phases, the optional native `n` micro-gap, and the remaining
native Track A memory peer gap. Each needs fresh evidence and must preserve
oracle behavior and the M51 ABI contract.

## Closing The 188 MB Native Track A Memory Gap

M55 built the machinery to close this gap but the gap itself is **not closed
in the shipping default**. Phase 1 attributed the old `105.6 MB` unclassified
floor, Phase 2/2R proved the poet payload owner and redesigned the artifact as
`YUNE-POET/2` (parity-preserving, `~113.2 MB` peak), and Phase 3R landed real
graph-volume reductions. Under the corrective per-key gate, however,
byte-backed access without the incremental sentence scratch costs
`4.6x`/`3.2x` on the long rows, so per the plan's own decided call (latency
ceilings bind) the default stays on the owned path at `185.7 MB`.

Current decision:

1. **The corrective M55 artifact is the standing native Track A gate**
   (startup, session, eight key rows, peak memory, win rows `<1.00x`, Track B
   absolutes; green twice on 2026-07-04).
2. **Byte-backed Luna poet consumption is an explicit opt-in**
   (`YUNE_POET_BYTE_BACKED=1`). The named path to flipping it: port the
   incremental sentence scratch to byte-backed storage, then re-run the
   default decision under the standing per-key gate.
3. **The memory result is lane-specific and conditional.** `113.2 MB` is real,
   parity-preserving, and currently latency-blocked; it is not a shipping
   default claim, browser memory parity, or a match to librime's `13.5 MB`
   peer process.

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
6. **WEB-05 harness control surface is drafted and may run in parallel.** The
   "surface all controls" follow-up carved out of M21: expose every engine
   control/diagnostic reachable through existing seams in the `yune-web`
   playground, defaults unchanged, public demo unchanged, Playwright-evidenced.
   Web-harness track; no engine changes, so it does not conflict with M56. Plan:
   [`plans/active/web05-plan-harness-control-surface.md`](./plans/active/web05-plan-harness-control-surface.md).
7. **M57 macOS Track A sentence-model parity and verification repair is
   drafted for review.** The independent macOS rerun found that local librime
   stays oracle-shaped, but Yune builds a different Luna sentence-model shape:
   `poet.entries_by_code` drops from the Windows corrective `513,353` owner
   shape to `191,984`, `poet.abbreviation_vocabulary` expands from the 11-row
   M42 target set to the full `421,966` vocabulary, the two abbreviation rows
   skip abbreviation discovery, and the long rows explode graph work. M57 fixes
   the Yune-side construction/abbreviation-path defect and reruns the macOS
   evidence bundle. Plan:
   [`plans/active/m57-plan-macos-track-a-sentence-model-parity.md`](./plans/active/m57-plan-macos-track-a-sentence-model-parity.md).
8. **Future browser fair-lane memory slice** - the fair `luna_pinyin` browser
   high-water floor or another freshly measured owner, only with a new scoped
   plan.
9. **Future AI-native engine experiments** - later, and only after classic
   engine performance is no longer dominated by avoidable pipeline costs.
10. **Future TypeDuck/profile-storage slices** - only with a new scoped plan,
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
| Engine performance | Native engine startup, schema/session lifecycle, mmap-backed `rsmarisa` marisa-table lookup, lazy/page-bounded translation, context export, memory, allocation, Track A guardrails, M55 final evidence, M57 macOS Track A sentence-model repair, historical native Track A research no-go evidence, and TypeDuck/Jyutping profile storage | M57 active plan, M55 final threshold/evidence, M52 historical guardrail evidence, M50 plan/evidence, M47 plan/evidence, and performance reports. |
| Web harness startup and memory | Tracked `apps/yune-web/` production build, public-demo dist, browser shell, asset/cache delivery, worker/WASM startup, persistence, schema selection, first key-to-paint, Chromium memory, compiled-asset contract, and debug-only octagram harness diagnostics | WEB-04 plan/evidence, WEB-03 plan/evidence, WEB-02 owner classification, WEB-01 measured no-go, M41 startup evidence, and browser reports. |
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
| M57 | Drafted / review | macOS Track A sentence-model parity and verification repair. The 2026-07-04 macOS rerun found a Yune-side model-shape defect, not an oracle/librime contradiction: long rows explode graph work, abbreviation rows skip M42 abbreviation discovery, and `poet.abbreviation_vocabulary` reports the full `421,966` vocabulary instead of the 11-row target set. Plan: fix platform-stable sentence-model construction first, add abbreviation-route protection only if correct construction still leaves candidate parity broken, then rerun the macOS native evidence bundle. Plan: [`plans/active/m57-plan-macos-track-a-sentence-model-parity.md`](./plans/active/m57-plan-macos-track-a-sentence-model-parity.md). |
| WEB-05 | Drafted / parallel-capable | Harness control surface: expose every engine control/diagnostic reachable through existing seams in the `yune-web` playground (the M21-deferred "surface all controls" slice); defaults and public demo unchanged; Playwright-evidenced. Plan: [`plans/active/web05-plan-harness-control-surface.md`](./plans/active/web05-plan-harness-control-surface.md). |

## Scope Ledger

A living map so "parity" always names a target. Deferred rows move into scope
only when an engine target needs them; nothing here commits to a timeline.

| In scope - target-driven, measured | Deferred - implement when an engine target needs it | Non-goal |
| --- | --- | --- |
| `luna_pinyin` core versus upstream `1.17.0`, including completed M17 null-grammar sentence/lattice, M18 punctuation processor slices, completed M42 abbreviation sentence parity for `cszysmsrsd`/`zybfshmsru`, completed M48 `jianli`/`biancheng` over-segmentation parity, and completed M54 native octagram-compatible grammar support for the named upstream target | Broader learned `.gram`/octagram behavior, contextual translation, and plugin-backed gears beyond the named M54 target | Bit-for-bit parity with librime internals |
| Common RIME schemas added through explicit breadth milestones | Further schema breadth only with fresh oracle fixtures and owning tests | Unbounded schema checklist work |
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
