# Roadmap

Yune is a Rust input-method engine that uses **upstream librime as a
compatibility and performance oracle** while building a cleaner Rust engine.
The current priority is not application integration. It is preserving engine
behavior while carrying completed M38 engine-performance parity lessons into
future target-driven engine or product work.

> **Compatibility oracle.** Upstream librime latest stable is the default
> behavior reference for user-visible schema semantics, standard ABI contracts,
> deployed data, and migration. The current pinned upstream target is
> `rime/librime 1.17.0`
> (`33e78140250125871856cdc5b42ddc6a5fcd3cd4`):
> <https://github.com/rime/librime>. This is a referenced upstream repository,
> not a local checkout path.

## Document Map

- This file - current engine roadmap dashboard, active sequence, scope
  boundaries, and readiness gates.
- [`ledgers/milestone-history.md`](./ledgers/milestone-history.md) - historical
  milestone ledger split out of this roadmap.
- [`conventions.md`](./conventions.md) - architecture, stack, structure,
  coding/testing conventions, integrations, current risks, and planning-doc
  rules.
- [`decisions.md`](./decisions.md) - standing principles plus project-wide
  decision log.
- [`requirements.md`](./requirements.md) - requirement IDs and status,
  including the closed M37 and M38 engine gates.
- [`reports/yune-vs-librime-performance.md`](./reports/yune-vs-librime-performance.md)
  and [`reports/yune-vs-librime-root-cause-analysis.md`](./reports/yune-vs-librime-root-cause-analysis.md)
  - current performance comparison and diagnosis.
- [`plans/completed/m38-plan-engine-performance-parity.md`](./plans/completed/m38-plan-engine-performance-parity.md)
  - completed pure engine performance parity plan.
- [`plans/active/m39-plan-long-input-engine-hardening.md`](./plans/active/m39-plan-long-input-engine-hardening.md)
  - draft plan for Track A long-input latency, Cantonese `jyut6ping3_mobile`
    profile long-input attribution, memory attribution, and whole-engine
    no-regression gates.
- [`plans/completed/m37-plan-engine-hyper-optimization.md`](./plans/completed/m37-plan-engine-hyper-optimization.md)
  - completed engine hyper-optimization milestone.
- [`plans/`](./plans) - active, reference, and completed plans, findings,
  contracts, and validation artifacts; finished plans live under
  [`plans/completed/`](./plans/completed).

> The GSD planning system (`.planning/`) has been retired; durable planning now
> lives under `docs/`.

## Current Snapshot

| Lane | Current state | Next decision or gate |
| --- | --- | --- |
| Core compatibility | Phase 1 named-target upstream behavior is complete for `luna_pinyin` and common-schema basics against upstream librime `1.17.0`. | Preserve upstream-observable behavior on every engine change. |
| Engine performance | M38 closed the pure upstream `luna_pinyin` native parity gates for startup/session and the short/medium rows `hao`/`ni`/`zhongguo`. The post-M38 long-input baseline shows broader typing parity is not closed: `ceshiyixiachangjushuruxingnengzenyang` is Yune `412,192.727us` versus librime `294.151us` (`1,401.296x`), and the 59-character `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` stress row is Yune `1,202,404.588us` versus librime `702.212us` (`1,712.310x`). Memory also remains above librime: Track A median working set is `107,839,488-114,728,960 B` versus librime `11,091,968-15,884,288 B` (`7.22-9.72x`). The Cantonese `jyut6ping3_mobile` 50+ character profile row is not yet baselined and is now a blocking M39 Task 0 gate. | Draft M39 plan: keep the M38 startup/session and short-row wins, keep both Track A long rows, add the Cantonese profile long row, add finer long-composition/profile spans and length-curve counters, fix the measured latency owner, preserve mmap/`rsmarisa`, run heap-owner profiling before memory optimization claims, and close only if the full startup/typing/memory/behavior dashboard has no unaccepted regression. |
| AI-native engine layer | M11/M13 proved a default-off local AI layer can sit on top of the deterministic engine. | Keep AI outside the classic deterministic performance path unless a named engine experiment explicitly enables it. |
| Future platform work | Platform-specific frontends and application shells are outside this roadmap. | Start a separate repository or separate plan before changing platform/application contracts. |

## Authoritative Sequence

1. **M39 long-input engine hardening** - draft active plan. Treat
   uninterrupted 50+ character input as a primary engine requirement for both
   upstream `luna_pinyin` and the Cantonese `jyut6ping3_mobile` profile while
   preserving startup/session, short-input latency, memory, mmap/`rsmarisa`,
   page-bounded output, and behavior gates.
2. **Future AI-native engine experiments** - later, and only after classic
   engine performance is no longer dominated by avoidable pipeline costs.

Trigger-gated, not scheduled: extracting the full processor pipeline from
`yune-rime-api` into `yune-core` lands only when a real non-ABI consumer needs
the full input path. Do not milestone that extraction speculatively.

## M39 Guardrails

M39 is not a single-row speedup milestone. It is the first post-M38
whole-engine regression gate, so it must keep the complete optimization bundle
visible in every final claim:

- same-run native Yune-versus-librime benchmarking for startup, session,
  short-input, medium-input, Track A long-input, memory, and owner counters,
  plus a native Track B `jyut6ping3_mobile` 50+ character profile row;
- mmap/file-backed selected table and prism bytes with zero selected
  table/prism heap mirror bytes;
- real runtime `rsmarisa` hot-path lookup for deployed marisa table data, with
  positive exact/prefix counters and zero ordinary no-marisa fallback on the
  upstream Track A rows;
- lazy/page-bounded first-page candidate production and page-sized context
  export, with any full-list fallback counted and explained;
- long-composition/profile attribution that splits the current translator bucket
  before optimizing it and proves whether the Cantonese profile shares the Track
  A owner;
- working-set and peak-memory comparison plus heap-owner attribution before any
  memory-reduction claim;
- focused behavior tests so performance work does not alter upstream-observable
  schema behavior.

M39 cannot close by improving `ceshiyixiachangjushuruxingnengzenyang` or
`zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` alone. The
`jyut6ping3_mobile`
`neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung` row must be
measured, attributed, and either fixed against the Task 0 gate or explicitly
closed by measured no-go. Startup, session, `hao`, `ni`, `zhongguo`,
mmap/`rsmarisa` activation, bounded output, memory, and behavior all remain
closeout gates.

## M37 Closeout

M37 is complete. It improved native engine storage and candidate
materialization, but it also made the key lesson clearer: end-to-end
application-shaped rows are not the right optimization target for the next milestone.
M38 therefore treats M37 as history and returns to a clean upstream
Yune-versus-librime engine comparison.

Engine lessons carried forward:

- Full-list candidate materialization can dominate short ambiguous input.
- Page-sized context export is required before frontend-shaped rows can be
  meaningful.
- Mapped or byte-backed deployed data helps memory only if it avoids rebuilding
  parallel heap mirrors.
- A storage probe is not the same thing as a runtime lookup backend.
- `rsmarisa` is an engine-performance concern when it is the route to a real
  marisa-backed table hot path.
- A faster lookup backend does not close the milestone if candidate production
  still expands and sorts a full list for a first-page read.
- Native engine wins must not be described as application, browser, or delivery
  wins without separate evidence.

## M38 Closeout

M38 is complete. The final clean upstream comparison meets the engine latency,
storage, lookup, iteration, behavior, and quality gates while keeping memory
caveats explicit.

M38 final upstream comparison:

- Warm startup/runtime-ready: Yune `23,363.300us`, librime `24,351.000us`
  (`0.959x`).
- Session create/select/destroy: Yune `24,243.500us`, librime `27,969.500us`
  (`0.867x`).
- `hao`: Yune `38.933us`, librime `11.400us` (`3.415x`).
- `ni`: Yune `56.750us`, librime `14.300us` (`3.969x`).
- `zhongguo`: Yune `64.263us`, librime `181.375us` (`0.354x`).
- Median working set: Yune `108-112 MB`, librime `10-13 MB`; selected
  table/prism heap mirror bytes are `0`.

Post-M38 coverage update: long continuous pinyin rows have now been measured
under [`reports/evidence/post-m38-long-input-baseline/baseline-native/`](./reports/evidence/post-m38-long-input-baseline/baseline-native/)
and [`reports/evidence/post-m38-long-input-baseline/stress-59-native/`](./reports/evidence/post-m38-long-input-baseline/stress-59-native/).
They are not in parity: `ceshiyixiachangjushuruxingnengzenyang` is Yune
`412,192.727us`, librime `294.151us`, or `1,401.296x` slower; the 59-character
`zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` stress row is
Yune `1,202,404.588us`, librime `702.212us`, or `1,712.310x` slower. The next
engine-performance plan must keep both rows, add the Cantonese
`jyut6ping3_mobile`
`neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung` row, treat 50+
uninterrupted input as a primary engine requirement, instrument the
long-composition/profile translator path and length curve, and close or
explicitly no-go the measured owner before claiming broader typing parity.

The current runs record memory baselines. Track A median working set is
`107,839,488-114,728,960 B` for Yune versus `11,091,968-15,884,288 B` for
librime (`7.22-9.72x`), and Yune max peak is `163,057,664-163,119,104 B`
versus librime `14,045,184-16,154,624 B` (`10.10-11.61x`). This is measured
working-set/peak evidence, not heap-owner attribution.

Closed M38 gates:

- Final Track A selected storage is `rsmarisa_byte_backed`, with positive
  `rsmarisa` exact/prefix lookup counters and zero ordinary no-marisa fallback.
- Final selected Track A table/prism bytes are mmap-backed and have no selected
  heap mirror.
- Ordinary first-page reads are page-bounded for the target rows.
- Final reports make native isolated-engine claims only; no frontend, browser,
  product, packaging, deployment, or public-delivery speed claim is made.

## Track Map

| Track | Scope | Current source of truth |
| --- | --- | --- |
| Engine performance | Native engine startup, schema/session lifecycle, mmap-backed `rsmarisa` marisa-table lookup, lazy/page-bounded translation, context export, memory, and allocation | Active draft: [`plans/active/m39-plan-long-input-engine-hardening.md`](./plans/active/m39-plan-long-input-engine-hardening.md). Completed baseline: [`plans/completed/m38-plan-engine-performance-parity.md`](./plans/completed/m38-plan-engine-performance-parity.md), performance reports, and `docs/reports/evidence/m38-engine-performance-parity/`. |
| Core compatibility | Upstream behavior fixtures and standard ABI-observable behavior | [`requirements.md`](./requirements.md), [`decisions.md`](./decisions.md), and per-milestone plans. |
| AI-native engine research | Default-off AI behavior layered above the deterministic engine | Future explicit engine experiments only. |
| Historical record | Completed milestone outcomes and reference/provenance pointers | [`ledgers/milestone-history.md`](./ledgers/milestone-history.md). |

## Milestone Ledger

| Milestone or track | Status | Current roadmap meaning |
| --- | --- | --- |
| M0-M24 | Complete | Phase 1 named-target engine/basic oracle parity is complete; history lives in [`ledgers/milestone-history.md`](./ledgers/milestone-history.md). |
| M25-M30 | Complete | Early performance and runtime-hardening work is historical context only. |
| M31 | Complete | Public demo delivery is historical context and not a current engine-performance target. |
| M33-M38 | Complete | Recent engine-performance work closed fairness, shared caches, compact storage, compiled-active paths, page-bounded materialization, mapped storage, and pure upstream `luna_pinyin` native parity with `rsmarisa` hot-path lookup. |
| M39 | Draft active | Long-input engine hardening: fix 37-character and 59-character Track A uninterrupted input, baseline and gate the 50+ character `jyut6ping3_mobile` Cantonese profile row, and preserve startup/session, short rows, mmap/`rsmarisa`, memory, and behavior. |

## Scope Ledger

A living map so "parity" always names a target. Deferred rows move into scope
only when an engine target needs them; nothing here commits to a timeline.

| In scope - target-driven, measured | Deferred - implement when an engine target needs it | Non-goal |
| --- | --- | --- |
| `luna_pinyin` core versus upstream `1.17.0`, including completed M17 null-grammar sentence/lattice and M18 punctuation processor slices | Learned `.gram`/octagram grammar, contextual translation, and broader plugin-backed gears until a named engine target needs them | Bit-for-bit parity with librime internals |
| Common RIME schemas added through explicit breadth milestones | Further schema breadth only with fresh oracle fixtures and owning tests | Unbounded schema checklist work |
| Native engine performance parity for startup, session lifecycle, mmap-backed `rsmarisa` marisa-table lookup, raw lookup, lazy/page-bounded translation, context export, memory, and allocation | Frontend/application delivery evidence and platform packaging | Claiming application-visible wins from native engine evidence |
| AI-native layer on the compatible deterministic base | Richer AI experiments after the classic engine path is competitive | Replacing or altering classic input paths by default |

## Deferred / Future

- **librime C++ plugin ABI** (Lua, octagram, predict, proto): deferred until a
  concrete engine target requires it; prefer Yune-native extension points first.
- **AI-native input layer beyond M13:** future work owns richer local-first AI
  behavior, privacy/memory controls, and any explicit remote-provider decision.
  Until then, proven AI remains default-off and outside the classic performance
  path.
- **Frontend/application delivery:** platform UI, browser delivery, packaging,
  cache behavior, and public deployment are outside the engine roadmap. They
  need separate plans and separate evidence.
- **iOS keyboard developer support:** future keyboard SDK or host work needs
  its own Yune-native package/host contract.

## Principles

The standing principles that govern all current and future work - librime as
oracle not template, name-the-protected-behavior, own-each-slice, AI-native as
a separate local-first layer, fixtures before module replacement, deferred
plugin ABI, and upstream-first oracle sequencing - have one canonical home:
[`decisions.md` -> Standing principles](./decisions.md#standing-principles).
