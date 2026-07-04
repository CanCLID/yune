# Roadmap

Yune is a Rust input-method engine that uses **upstream librime as a
compatibility and performance oracle** while building a cleaner Rust engine.
This file is the live dashboard: it records current state, next decisions,
scope boundaries, and readiness gates. Completed milestone detail lives in
[`ledgers/milestone-history.md`](./ledgers/milestone-history.md), completed
plans, reports, and evidence folders.

Current status: Phase 1 named-target compatibility is complete; M47's portable
TypeDuck/Jyutping keyboard memory work is complete for the Windows proxy; M51
froze the engine support contract and ABI boundaries; M52 remains the standing
green native Track A `luna_pinyin` performance guard; M53 re-verified the
engine docs and public claims for release-readiness; M54 adds native
octagram-compatible grammar support for the named upstream `luna_pinyin` target
without implementing the librime C++ plugin ABI; WEB-04 makes that path
observable in `yune-web` through a default-off debug profile. M55, the native
Track A match-or-beat performance program, is closed partial/no-go at Phase 2:
byte-backed poet storage reduces the final Track A peak to `110.5 MB`, but the
broader M55 ratchet is red on the two long Luna rows and one Track B product
latency guard, so M55 does not supersede M52.

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
  - native Track A match-or-beat research milestone, closed partial/no-go with
    M52 left as the standing green gate.
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
| Core compatibility | Phase 1 named-target upstream behavior remains complete for `luna_pinyin` and common-schema basics against upstream librime `1.17.0`. M54 adds native octagram-compatible grammar support for the named upstream `luna_pinyin` octagram target using pinned lotem oracle data, with RIME-LMDG kept as a pinned validation lane. M51 records supported targets, oracle precedence, default upstream ABI rules, profile ABI rules, `yune_web_*` export rules, storage expectations, and evidence-lane rules. Post-M51 cleanup documents and tests `rime_get_yune_windows_profile_api()` as a parallel accessor for the same current profile table. | Future engine work must preserve the contract or update it with named oracle/header evidence and focused tests. Full librime C++ plugin ABI support remains deferred. |
| Engine performance | M55 is closed partial/no-go for native Track A match-or-beat. Phase 1 attributed the old unclassified memory floor, and Phase 2 byte-backed the Luna poet payload owners, reducing final Track A peak to `110,542,848 B` (`110.5 MB`). The full M55 ratchet is red: 37-character Luna is `5.964x`, 59-character Luna is `4.030x`, and the Track B product long-row guard is `414.059us` against a `375.253us` ceiling. M52 remains the standing green native Track A regression gate. | Future native Track A work should only reopen these rows with a new owner-evidenced access-path/graph plan that preserves candidate output, avoids retained heap indexes and ABI widening, and does not loosen the M55 no-go evidence into a passing bar. |
| TypeDuck/Jyutping product memory | M47's portable scope is complete. The comments-intact `jyut6ping3_mobile` keyboard profile reached about `67 MB` working set / `22 MB` private on Windows proxy evidence, with table, prism, and rich lookup/comment payloads byte-backed from compiled storage. | Apple `phys_footprint` proof remains unnumbered far-future platform validation. Optional RED-09/10/11-style polish needs a fresh owner-ranked plan. |
| Web harness startup and memory | WEB-03 fixed the launch compiled-asset contract and the stale Jyutping source-fallback owner. WEB-04 adds a default-off `luna_pinyin_octagram` debug profile that fetches a pinned lotem `.gram` locally, delivers it only for the octagram profile, exposes delivered/fallback/checksum/schema-select high-water memory diagnostics, proves all four named ranking rows versus plain Luna in Playwright, and proves browser fail-closed behavior. Current dashboard fair `luna_pinyin` browser comparison is `64.0 MiB` peak versus My RIME `16.0 MiB`; old Jyutping `893.1 MiB` remains only as a synthetic no-launch-assets negative control. | Future browser memory work should target the fair `luna_pinyin` runtime high-water floor or another freshly measured owner, not another payload-only or stale-asset branch. Broader contextual suggestions or non-debug octagram product UX needs a new scoped plan. |
| AI-native engine layer | M11/M13 proved a default-off local AI layer can sit on top of the deterministic engine. | Keep AI outside the classic deterministic performance path unless a named engine experiment explicitly enables it. |
| Future platform work | Platform-specific native frontends remain outside this repo roadmap. | Start a separate repository or separate plan before changing platform/application contracts. |

## Current Guardrails

The current native Track A regression gate is M52:

- Threshold source:
  [`reports/evidence/m52-track-a-guardrails-and-disposition/track-a-thresholds.csv`](./reports/evidence/m52-track-a-guardrails-and-disposition/track-a-thresholds.csv)
- Final proof:
  [`reports/evidence/m52-track-a-guardrails-and-disposition/final-native-benchmark/threshold-check.csv`](./reports/evidence/m52-track-a-guardrails-and-disposition/final-native-benchmark/threshold-check.csv)
- Manual command shape:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 `
  -OutputRoot docs\reports\evidence\<new-run> `
  -Iterations 9 -SessionIterations 60 -KeyIterations 80 `
  -TrackAInputs n,ni,hao,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong `
  -SkipTrackB `
  -TrackAThresholds docs\reports\evidence\m52-track-a-guardrails-and-disposition\track-a-thresholds.csv `
  -FailOnRegression
```

The gate is local/manual because it needs same-run librime `1.17.0` artifacts.
Do not summarize "M52 guardrails pass" as "Yune is faster than librime" or as
"Track A meets every strict ratio target." The guardrail freezes the current
measured state and fails on regression.

M55's broader threshold artifact remains research/no-go evidence only:
[`reports/evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv`](./reports/evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv).
The final M55 run fails the 37-character Luna row, the 59-character Luna row,
and one Track B product latency guard, so it does not supersede M52. The exact
handoff decision is recorded in
[`reports/evidence/m55-native-match-or-beat/final/partial-closeout-2026-07-04.md`](./reports/evidence/m55-native-match-or-beat/final/partial-closeout-2026-07-04.md).

## Performance North Star

Broad, unqualified claims that "Yune is faster than librime" are not supported
by current evidence. Current performance is lane-specific: M55 reduced the
native Track A memory peak but failed the broader full-suite ratchet, while M52
remains the last green regression gate. The short-key rows remain above librime,
the 37/59-character Luna rows are the current native latency no-go, and browser
performance remains a separate evidence lane.

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

Likely native Track A structural owners are the `ni` exact-row/filtering
constant factor, the 37/59-character poet graph and scoring path, and full Luna
`poet.vocabulary` / `poet.entries_by_code` residency. Those are not small
cleanup tasks. They require a new owner-evidenced design, likely around compact
byte-backed poet storage, top-k or incremental scoring, or another algorithmic
change that changes the cost model without changing candidate output.

## Closing The 188 MB Native Track A Memory Gap

M55 tested this path and closed partial/no-go at Phase 2. The memory owner was
real: Phase 1 attributed the old `105.6 MB` unclassified floor, and Phase 2
compiled the upstream Luna poet payloads into `YUNE-POET/1` byte-backed storage
served from mapped bytes. The final closeout run reports Track A peak
`110,542,848 B`, down from the M52-era `188,383,232 B`.

The tradeoff is also real. The byte-backed access path fails the broader M55
latency ratchet: the 37-character Luna row is `5.964x`, the 59-character Luna
row is `4.030x`, and the Track B product long-row guard is `414.059us` against
a `375.253us` ceiling. Candidate output remains the non-negotiable constraint,
and M55 did not add retained heap indexes, widen the public C ABI, or make a
browser/product/platform performance claim.

Current decision:

1. **M52 remains the standing green native Track A gate.** M55's threshold CSV is
   retained as research/no-go evidence and does not supersede the M52 guard.
2. **M55 does not continue into Phase 3 or Phase 4.** The next native Track A
   attempt needs fresh owner evidence for the byte-backed poet access path and
   sentence graph constant factors, not another storage-only change.
3. **Memory progress is recorded as structural evidence, not a launch claim.**
   The `110.5 MB` final peak is useful, but no public match-or-beat or product
   performance claim follows from a red full ratchet.

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
4. **M55 native Track A match-or-beat program is closed partial/no-go.** It
   proved the poet-storage memory owner and reduced final Track A peak to
   `110.5 MB`, but the full ratchet fails the two long Luna rows and one Track B
   product latency guard. M52 remains the standing green native Track A gate;
   M55's threshold artifact is research/no-go evidence only. Evidence:
   [`reports/evidence/m55-native-match-or-beat/`](./reports/evidence/m55-native-match-or-beat/).
   Plan:
   [`plans/completed/m55-plan-native-track-a-match-or-beat-program.md`](./plans/completed/m55-plan-native-track-a-match-or-beat-program.md).
5. **M56 engine productization hardening is drafted and queued next for engine
   work.** Three tracks driven by the repo's own incident history: structural
   staleness-proofing + cold-start conformance (the WEB-02/M38/M41 bug class),
   user-data lifecycle evidence, and an ABI abuse suite with unwind-guarded
   exports (the workspace currently has zero `catch_unwind` boundaries). No ABI
   widening, no behavior change on defined paths. Plan:
   [`plans/active/m56-plan-engine-productization-hardening.md`](./plans/active/m56-plan-engine-productization-hardening.md).
6. **WEB-05 harness control surface is drafted and may run in parallel.** The
   "surface all controls" follow-up carved out of M21: expose every engine
   control/diagnostic reachable through existing seams in the `yune-web`
   playground, defaults unchanged, public demo unchanged, Playwright-evidenced.
   Web-harness track; no engine changes, so it does not conflict with M56. Plan:
   [`plans/active/web05-plan-harness-control-surface.md`](./plans/active/web05-plan-harness-control-surface.md).
7. **Future browser fair-lane memory slice** - the fair `luna_pinyin` browser
   high-water floor or another freshly measured owner, only with a new scoped
   plan.
8. **Future AI-native engine experiments** - later, and only after classic
   engine performance is no longer dominated by avoidable pipeline costs.
9. **Future TypeDuck/profile-storage slices** - only with a new scoped plan,
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
| Engine performance | Native engine startup, schema/session lifecycle, mmap-backed `rsmarisa` marisa-table lookup, lazy/page-bounded translation, context export, memory, allocation, Track A guardrails, native Track A research no-go evidence, and TypeDuck/Jyutping profile storage | M55 partial/no-go evidence, M52 standing guardrail evidence, M50 plan/evidence, M47 plan/evidence, and performance reports. |
| Web harness startup and memory | Tracked `apps/yune-web/` production build, public-demo dist, browser shell, asset/cache delivery, worker/WASM startup, persistence, schema selection, first key-to-paint, Chromium memory, compiled-asset contract, and debug-only octagram harness diagnostics | WEB-04 plan/evidence, WEB-03 plan/evidence, WEB-02 owner classification, WEB-01 measured no-go, M41 startup evidence, and browser reports. |
| Core compatibility | Upstream behavior fixtures and standard ABI-observable behavior | Requirements, decisions, engine support contract, per-milestone plans, and the M53 release-readiness audit (`reports/evidence/m53-engine-release-readiness-audit/`). |
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
| M48-M52 | Complete | Current engine correctness, support-contract, and Track A guardrail closeouts; M52 is the current native performance source of truth. |
| M53 | Complete | Engine release-readiness audit (docs/evidence only): five-dimension consistency/ABI/guardrail/claim/link audit with adversarial verification; substantive invariants clean, no drift; corrected stale `README.md`/archived public-claim wording to the contract-accurate M52 lane-specific record. Plan: [`plans/completed/m53-plan-engine-release-readiness-audit.md`](./plans/completed/m53-plan-engine-release-readiness-audit.md). |
| M54 | Complete | Native octagram-compatible grammar support for the named upstream `luna_pinyin` target, with pinned lotem oracle data, RIME-LMDG validation evidence, external model checksums, clean-room Rust `.gram` parsing/scoring, null-grammar and TypeDuck regression gates, and no public C ABI change. Plan: [`plans/completed/m54-plan-native-octagram-grammar-support.md`](./plans/completed/m54-plan-native-octagram-grammar-support.md). |
| M55 | Complete with measured no-go | Native Track A match-or-beat program closed at Phase 2: Phase 1 attributed the memory floor, Phase 2 byte-backed Luna poet storage and reduced final Track A peak to `110.5 MB`, but the full M55 ratchet fails the 37-character row, 59-character row, and one Track B product latency guard. M52 remains the standing green native Track A gate; M55 thresholds are research/no-go evidence only. Plan: [`plans/completed/m55-plan-native-track-a-match-or-beat-program.md`](./plans/completed/m55-plan-native-track-a-match-or-beat-program.md). |
| M56 | Drafted / queued | Engine productization hardening for the external Windows/iOS frontends: staleness-proofing + cold-start conformance, user-data lifecycle evidence, ABI abuse suite + panic-boundary guards; no ABI change, behavior-preserving. Queued next after the M55 threshold-handoff decision. Plan: [`plans/active/m56-plan-engine-productization-hardening.md`](./plans/active/m56-plan-engine-productization-hardening.md). |
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
