# Roadmap

Yune is a Rust input-method engine that uses **upstream librime as a
compatibility and performance oracle** while building a cleaner Rust engine.
This file is the live dashboard: it records current state, next decisions,
scope boundaries, and readiness gates. Completed milestone detail lives in
[`ledgers/milestone-history.md`](./ledgers/milestone-history.md), completed
plans, reports, and evidence folders.

Current status: Phase 1 named-target compatibility is complete; M47's portable
TypeDuck/Jyutping keyboard memory work is complete for the Windows proxy; M51
froze the engine support contract and ABI boundaries; M52 froze native Track A
`luna_pinyin` performance guardrails and dispositioned the remaining M50
blockers; M53 re-verified the engine docs and public claims for
release-readiness and corrected stale `README.md` public-claim wording; M54
adds native octagram-compatible grammar support for the named upstream
`luna_pinyin` target without implementing the librime C++ plugin ABI; WEB-04
makes that path observable in `yune-web` through a default-off debug profile. No
active numbered engine milestone is open in this roadmap.

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
| Engine performance | M52 is complete for native Track A guardrails and blocker disposition. The committed threshold artifact and fail-on-regression benchmark gate cover `n`, `ni`, `hao`, the 37-character Luna row, the existing 59-character Luna row, and full Luna peak memory. Final same-run rows are startup `24,139.200us` / `1.113x`, session `23,404.000us` / `1.001x`, `n` `60.300us` / `2.818x`, `ni` `44.950us` / `3.143x`, `hao` `24.967us` / `2.146x`, 37-character `895.178us` / `3.053x`, and 59-character `1,545.754us` / `2.247x`. Full Luna Track A memory is `188.4 MB` peak / `194.3 MB` max-summary private versus librime `17.3 MB` max peer peak; named owners include `poet.vocabulary` `53.6 MB`, `poet.entries_by_code` `18.7 MB`, and process unclassified lower bound `105.6 MB`. | Future native Track A work should only reopen these rows with a new owner-evidenced plan. M52 guardrails are regression ceilings, not a claim that `ni` or the 37-character row meet the strict `<=3.0x` target. |
| TypeDuck/Jyutping product memory | M47's portable scope is complete. The comments-intact `jyut6ping3_mobile` keyboard profile reached about `67 MB` working set / `22 MB` private on Windows proxy evidence, with table, prism, and rich lookup/comment payloads byte-backed from compiled storage. | Apple `phys_footprint` proof remains unnumbered far-future platform validation. Optional RED-09/10/11-style polish needs a fresh owner-ranked plan. |
| Web harness startup and memory | WEB-03 fixed the launch compiled-asset contract and the stale Jyutping source-fallback owner. WEB-04 adds a default-off `luna_pinyin_octagram` debug profile that fetches a pinned lotem `.gram` locally, loads it only for the octagram profile, exposes loaded/fallback/checksum/memory diagnostics, and proves browser fail-closed behavior. Current dashboard fair `luna_pinyin` browser comparison is `64.0 MiB` peak versus My RIME `16.0 MiB`; old Jyutping `893.1 MiB` remains only as a synthetic no-launch-assets negative control. | Future browser memory work should target the fair `luna_pinyin` runtime high-water floor or another freshly measured owner, not another payload-only or stale-asset branch. Broader contextual suggestions or non-debug octagram product UX needs a new scoped plan. |
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

## Performance North Star

Broad, unqualified claims that "Yune is faster than librime" are not supported
by current evidence. Current performance is lane-specific: startup/session are
near parity, several tracked rows pass, and M52 freezes regression ceilings, but
native Track A `ni`, the 37-character Luna row, and full Luna memory still trail
same-run librime.

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

This is the one gap that is large and structural rather than
sub-perceptual-microsecond. It is not a wall: the technique that closes it is
already proven in this repo. It is deferred, not blocked, and would open as an
owner-evidenced performance-research milestone (tentatively **M55 native Track A
structural memory research**), after the release-readiness audit, only with a
fresh plan and a real win bar.

Why the peak is `188.4 MB` versus librime's `17.3 MB`: at native `luna_pinyin`
schema selection Yune loads the upstream sentence model into owned heap
structures - `poet.vocabulary` (`53.6 MB`, the full upstream Luna preset
vocabulary that M48 had to load to fix `jianli`/`biancheng` over-segmentation),
`poet.entries_by_code` (`18.7 MB`), and a process unclassified lower bound of
`105.6 MB`. librime keeps the equivalent data mmap'd/paged from compiled files,
so it never counts as dirty private memory. The gap is a storage-strategy
difference, not an algorithmic defect.

The proven precedent is **M47**: it byte-backed the TypeDuck table, prism, and
lookup/comment payloads from mmap'd compiled storage - exactly librime's
strategy - and cut the shipping `jyut6ping3_mobile` keyboard profile from about
`298 MB` to about `67 MB` working set / `22 MB` private with the full dictionary
retained. The same move applies to the poet sentence model.

Design sketch for the milestone, in order:

1. **Attribute the `105.6 MB` unclassified lower bound first.** It is currently
   a measured floor, not a named owner; you cannot reduce what you have not
   named. Profile allocator/arena residency and deploy/compile transients before
   touching poet code.
2. **Byte-back `poet.vocabulary` and `poet.entries_by_code`.** Compile them into
   an mmap-backed artifact served by offset (like the compiled table/prism), so
   the OS pages them instead of holding owned `Vec`/`BTreeMap` heap. Target the
   named `~72 MB` first.
3. **Preserve candidate output exactly.** This path is oracle-sensitive - M48
   fixed a real scoring bug here - so byte-backed access must produce identical
   sentence ranking, gated by `upstream_luna_pinyin_parity` and `cantonese_parity`.
4. **Respect the memory/latency tension.** Byte-backed or lazy access can add
   per-lookup deserialization latency. Any change must stay within the M52
   latency ceilings; measure both memory and latency in the same run. Memory
   wins if they conflict, but the latency guardrail must still pass.

Win bar (set before coding): a measured Track A peak reduction with named-owner
movement toward the byte-backed floor - not a no-regression ceiling. Close as
partial/no-go if the owner is not real or the parity/latency risk is
unacceptable.

Priority caveat: native `luna_pinyin` Track A is the oracle-comparison lane, not
a shipping product profile (the shipping native target is TypeDuck/Jyutping,
already in budget after M47). So this work is highest value when either native
Luna becomes a shipping profile, or the byte-backed poet storage is built as a
portable technique that also lowers the Track B product, WASM, and iOS lanes.

## Authoritative Sequence

1. **No active numbered engine milestone is open.** M54 is complete for native
   octagram-compatible grammar support; M52 remains the current native
   performance guardrail source of truth.
2. **M53 engine release-readiness audit is complete.** The five-dimension audit
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
3. **M54 native octagram-compatible grammar support is complete.** Yune now
   supports the named upstream `luna_pinyin` octagram target through a native
   Rust `Grammar` provider and logical `.gram` resource loading. Evidence:
   [`reports/evidence/m54-native-octagram-grammar-support/`](./reports/evidence/m54-native-octagram-grammar-support/).
   Plan:
   [`plans/completed/m54-plan-native-octagram-grammar-support.md`](./plans/completed/m54-plan-native-octagram-grammar-support.md).
4. **WEB-04 octagram debug harness is complete.** The tracked `apps/yune-web`
   harness now exposes `luna_pinyin_octagram` as a default-off profile, loads the
   pinned lotem `.gram` through `extraSharedAssets`, records diagnostics, and
   proves loaded ranking, plain-Luna negative control, and missing-model
   fail-closed behavior in Playwright. Evidence:
   [`reports/evidence/web04-octagram-debug-harness/`](./reports/evidence/web04-octagram-debug-harness/).
   Plan:
   [`plans/completed/web04-plan-octagram-debug-harness-luna-pinyin.md`](./plans/completed/web04-plan-octagram-debug-harness-luna-pinyin.md).
5. **Future performance research is optional and must be owner-evidenced.** A
   "surpass librime" milestone should start from the Performance North Star and
   should not reuse M52's regression ceilings as a success bar. The concrete
   candidate is the native Track A memory work sketched in
   [Closing The 188 MB Native Track A Memory Gap](#closing-the-188-mb-native-track-a-memory-gap)
   (tentatively M55), opened only with a fresh owner-evidenced plan.
6. **Future browser fair-lane memory slice** - the fair `luna_pinyin` browser
   high-water floor or another freshly measured owner, only with a new scoped
   plan.
7. **Future AI-native engine experiments** - later, and only after classic
   engine performance is no longer dominated by avoidable pipeline costs.
8. **Future TypeDuck/profile-storage slices** - only with a new scoped plan,
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
| Engine performance | Native engine startup, schema/session lifecycle, mmap-backed `rsmarisa` marisa-table lookup, lazy/page-bounded translation, context export, memory, allocation, Track A guardrails, and TypeDuck/Jyutping profile storage | M52 plan/evidence, M50 plan/evidence, M47 plan/evidence, and performance reports. |
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
| WEB-04 | Complete | Default-off `luna_pinyin_octagram` browser debug harness with non-vendored pinned lotem `.gram`, loaded/fallback/checksum/memory diagnostics, Playwright ranking proof, plain-Luna negative control, and missing-model fail-closed evidence. Plan: [`plans/completed/web04-plan-octagram-debug-harness-luna-pinyin.md`](./plans/completed/web04-plan-octagram-debug-harness-luna-pinyin.md). |
| M47 | Complete for portable scope | TypeDuck/Jyutping comments-intact keyboard memory is under the Windows private/dirty proxy target; Apple `phys_footprint` proof remains parked. |
| M48-M52 | Complete | Current engine correctness, support-contract, and Track A guardrail closeouts; M52 is the current native performance source of truth. |
| M53 | Complete | Engine release-readiness audit (docs/evidence only): five-dimension consistency/ABI/guardrail/claim/link audit with adversarial verification; substantive invariants clean, no drift; corrected stale `README.md`/archived public-claim wording to the contract-accurate M52 lane-specific record. Plan: [`plans/completed/m53-plan-engine-release-readiness-audit.md`](./plans/completed/m53-plan-engine-release-readiness-audit.md). |
| M54 | Complete | Native octagram-compatible grammar support for the named upstream `luna_pinyin` target, with pinned lotem oracle data, RIME-LMDG validation evidence, external model checksums, clean-room Rust `.gram` parsing/scoring, null-grammar and TypeDuck regression gates, and no public C ABI change. Plan: [`plans/completed/m54-plan-native-octagram-grammar-support.md`](./plans/completed/m54-plan-native-octagram-grammar-support.md). |

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
