# Roadmap

Yune is a Rust input-method engine that uses **upstream librime as a
compatibility and performance oracle** while building a cleaner Rust engine.
The current priority is preserving the completed M40 native-engine state while
recording the completed M41 `yune-web` browser-harness startup optimization
milestone separately from native engine performance.

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
  including the closed M37-M40 engine gates and completed M41 web-harness gates.
- [`reports/yune-vs-librime-performance.md`](./reports/yune-vs-librime-performance.md)
  and [`reports/yune-vs-librime-root-cause-analysis.md`](./reports/yune-vs-librime-root-cause-analysis.md)
  - current performance comparison and diagnosis.
- [`plans/completed/m41-plan-yune-web-startup-optimization.md`](./plans/completed/m41-plan-yune-web-startup-optimization.md)
  - completed browser-harness startup optimization plan for `apps/yune-web/`.
- [`plans/completed/m40-plan-compiled-sentence-lookup-index.md`](./plans/completed/m40-plan-compiled-sentence-lookup-index.md)
  - completed compiled sentence lookup index plan.
- [`plans/completed/m39-plan-long-input-engine-hardening.md`](./plans/completed/m39-plan-long-input-engine-hardening.md)
  - completed long-input engine hardening plan.
- [`plans/completed/m38-plan-engine-performance-parity.md`](./plans/completed/m38-plan-engine-performance-parity.md)
  - completed pure engine performance parity plan.
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
| Engine performance | M40 closed the compiled sentence lookup index gates in native engine evidence. Startup/runtime-ready is Yune `23,934.200us` versus librime `26,218.400us` (`0.913x`), session is `23,994.000us` versus `25,700.000us` (`0.934x`), `hao`/`ni`/`zhongguo` remain inside gates, the 37-character Track A row is `289.914us` versus `295.800us` (`0.980x`), and the 59-character Track A row is `494.017us` versus `694.175us` (`0.712x`). The Track B Cantonese 50+ row is included as a guard at `196.387us/op` median, with two p95 outliers recorded as a caveat. | M40 is complete. Next engine work needs a new scoped plan, likely whole-process memory owner profiling, stricter short-key parity, or a separate TypeDuck-profile/native Track B owner if a named product row requires it. |
| Web harness startup | M41 is complete for the tracked `apps/yune-web/` browser harness. Final production-browser medians are tracked `luna_pinyin` cold `846 ms`, tracked `jyut6ping3_mobile` cold `1,254 ms`, public-demo `luna_pinyin` cold `867 ms`, and public-demo `jyut6ping3_mobile` cold `1,291 ms`; warm/reload rows improved by `87.4-95.9%` from the bounded phase-0 owner baseline. | Browser startup is now a completed evidence-backed lane. Future web work needs a new scoped plan, likely browser/React shell residual, Jyutping asset payload, or remote delivery/cache behavior. |
| AI-native engine layer | M11/M13 proved a default-off local AI layer can sit on top of the deterministic engine. | Keep AI outside the classic deterministic performance path unless a named engine experiment explicitly enables it. |
| Future platform work | Platform-specific native frontends remain outside this repo roadmap. | Start a separate repository or separate plan before changing platform/application contracts. |

## Authoritative Sequence

1. **Future native engine owner slices** - require a new scoped plan and fresh
   evidence. M40 leaves whole-process memory, stricter short-key parity,
   incomplete-pinyin output parity, and separate TypeDuck-profile/native Track B
   owners as possible future tracks.
2. **Future web harness slices** - require a new scoped browser plan and fresh
   evidence. M41 leaves browser/React shell residual, Jyutping asset payload,
   and remote delivery/cache behavior as possible future tracks.
3. **Future AI-native engine experiments** - later, and only after classic
   engine performance is no longer dominated by avoidable pipeline costs.
4. **Future engine memory or profile-storage slices** - only with a new scoped
   plan, fresh owner evidence, and native-engine-only claims unless browser
   evidence is explicitly collected.

Trigger-gated, not scheduled: extracting the full processor pipeline from
`yune-rime-api` into `yune-core` lands only when a real non-ABI consumer needs
the full input path. Do not milestone that extraction speculatively.

## M41 Closeout

M41 is complete under
[`plans/completed/m41-plan-yune-web-startup-optimization.md`](./plans/completed/m41-plan-yune-web-startup-optimization.md).
It resumed the `yune-web` startup work after M40 by measuring the tracked
browser harness directly, not by extrapolating from native-engine results.

M41's measured owner was not native lookup. The old browser startup pain came
from incomplete production runtime packaging plus a redundant startup
customize/deploy path that cost about `15,538 ms` on the
`jyut6ping3_mobile` startup path. The accepted implementation packages the
WASM runtime, initializes the worker with the selected schema, skips no-op
default deploy preferences, and loads schema-scoped startup assets.

Final M41 browser evidence:

- tracked `luna_pinyin` cold ready-to-input: `846 ms` median, `932 ms` p95;
- tracked `jyut6ping3_mobile` cold ready-to-input: `1,254 ms` median,
  `1,330 ms` p95;
- public-demo `luna_pinyin` cold ready-to-input: `867 ms` median, `883 ms`
  p95;
- public-demo `jyut6ping3_mobile` cold ready-to-input: `1,291 ms` median,
  `1,349 ms` p95;
- warm/reload tracked rows improved by `87.4-95.9%` versus the phase-0 owner
  baseline;
- final first-key after ready remains interactive, with tracked cold p95 no
  worse than `235 ms` across required typed inputs;
- final evidence records Chromium heap/DOM metrics and Windows working set for
  all scenarios.

M41 kept these boundaries explicit:

- Use production builds as headline evidence; dev-server numbers are secondary.
- Measure both `luna_pinyin` and `jyut6ping3_mobile` rows, including short,
  long, incomplete, cold, warm, and mock-worker cases.
- Split startup into browser shell, asset transfer/cache, worker/WASM startup,
  virtual filesystem/persistence, deploy/schema reuse, engine schema selection,
  first key-to-paint, and browser memory.
- Optimize the measured top owner first, then prove no regression in typing,
  memory, cache correctness, public-demo honesty, or native-engine behavior.
- Do not touch Rust engine code unless M41 evidence proves a narrow runtime
  boundary blocker and the native M40 gates are rerun.

## M40 Closeout

M40 is complete under
[`plans/completed/m40-plan-compiled-sentence-lookup-index.md`](./plans/completed/m40-plan-compiled-sentence-lookup-index.md).
It followed directly from the post-M39 bottleneck: Track A long rows were no
longer broken by raw table lookup, context export, or `rsmarisa` activation;
the remaining gap was the sentence lookup shape around `word_graph_for_input`.

M40 closed with all four required strategies active:

- exact range indexing over sorted sentence entries;
- reachable-vertex pruning so unreachable start positions are skipped;
- prefix filtering with early breaks for impossible substrings;
- a compact phrase-index walk over sorted ranges rather than the old
  all-substrings loop.

Final M40 native comparison:

- Warm startup/runtime-ready: Yune `23,934.200us`, librime `26,218.400us`
  (`0.913x`).
- Session create/select/destroy: Yune `23,994.000us`, librime `25,700.000us`
  (`0.934x`).
- `hao`: Yune `38.200us`, librime `11.800us` (`3.237x`).
- `ni`: Yune `56.850us`, librime `14.700us` (`3.867x`).
- `zhongguo`: Yune `60.275us`, librime `186.400us` (`0.323x`).
- `ceshiyixiachangjushuruxingnengzenyang`: Yune `289.914us`, librime
  `295.800us` (`0.980x`).
- `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong`: Yune
  `494.017us`, librime `694.175us` (`0.712x`).
- `cszysmsrsd`: Yune `24.820us`, librime `1,237.820us`; not a comparable
  speed ratio because Yune exports `0` candidates.
- `zybfshmsru`: Yune `26.350us`, librime `866.720us`; not a comparable speed
  ratio because Yune exports `0` candidates.
- `jyut6ping3_mobile`
  `neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung`: final Yune
  median `196.387us/op`; p95 `605.125us/op` is retained as a measured outlier
  caveat.

The M40-ENGINE-12 graph-rebuild verdict is also closed: graph rebuild is
measured at `17.303us/key` and `31.014us/key` on the two long rows, below the
remaining sentence-model/translator owner, so bounded incrementality was not
implemented in M40.

M40 remains a native-engine-only milestone. It does not claim web-harness,
frontend, product-delivery, packaging, browser startup, browser typing, or
public-demo speed wins.

## M39 Closeout

M39 is complete. It was not a single-row speedup milestone; it closed a
whole-engine regression gate across startup/session, short rows, Track A long
rows, the Track B Cantonese profile long row, storage, memory, and behavior.

Final M39 native comparison:

- Warm startup/runtime-ready: Yune `25,048.200us`, librime `27,314.000us`
  (`0.917x`).
- Session create/select/destroy: Yune `25,255.500us`, librime `26,938.500us`
  (`0.938x`).
- `hao`: Yune `38.933us`, librime `11.867us` (`3.281x`).
- `ni`: Yune `56.200us`, librime `14.550us` (`3.863x`).
- `zhongguo`: Yune `60.588us`, librime `183.887us` (`0.329x`).
- `ceshiyixiachangjushuruxingnengzenyang`: Yune `514.903us`, librime
  `291.786us` (`1.765x`).
- `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong`: Yune
  `917.961us`, librime `695.653us` (`1.320x`).
- `jyut6ping3_mobile`
  `neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung`: final Yune
  median `188.857us/op`, p95 `194.910us/op`, below Phase 0.

M39 proved the owner split before implementation: Track A was dominated by
upstream sentence-model scanning, while Track B was a profile-specific
no-marisa prefix/fallback path. The final Track A path keeps
`selected_storage=rsmarisa_byte_backed`, table/prism `mmap`, selected heap
mirrors `0`, positive runtime `rsmarisa` exact/prefix counters, and bounded
first-page output. Track B keeps byte-backed mmap storage and profile fallback
semantics without regressing the 50+ row. Final reports remain native-engine
claims only.

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
| Engine performance | Native engine startup, schema/session lifecycle, mmap-backed `rsmarisa` marisa-table lookup, lazy/page-bounded translation, context export, memory, allocation, and completed M40 sentence lookup indexing | Completed M40 plan: [`plans/completed/m40-plan-compiled-sentence-lookup-index.md`](./plans/completed/m40-plan-compiled-sentence-lookup-index.md). Completed M39 plan: [`plans/completed/m39-plan-long-input-engine-hardening.md`](./plans/completed/m39-plan-long-input-engine-hardening.md). |
| Web harness startup | Tracked `apps/yune-web/` production build, public-demo dist, browser shell, asset/cache delivery, worker/WASM startup, persistence, schema selection, first key-to-paint, and Chromium memory | Completed M41 plan: [`plans/completed/m41-plan-yune-web-startup-optimization.md`](./plans/completed/m41-plan-yune-web-startup-optimization.md); final evidence under [`apps/yune-web/e2e/results/m41-yune-web-startup-optimization/`](../apps/yune-web/e2e/results/m41-yune-web-startup-optimization/). |
| Core compatibility | Upstream behavior fixtures and standard ABI-observable behavior | [`requirements.md`](./requirements.md), [`decisions.md`](./decisions.md), and per-milestone plans. |
| AI-native engine research | Default-off AI behavior layered above the deterministic engine | Future explicit engine experiments only. |
| Historical record | Completed milestone outcomes and reference/provenance pointers | [`ledgers/milestone-history.md`](./ledgers/milestone-history.md). |

## Milestone Ledger

| Milestone or track | Status | Current roadmap meaning |
| --- | --- | --- |
| M0-M24 | Complete | Phase 1 named-target engine/basic oracle parity is complete; history lives in [`ledgers/milestone-history.md`](./ledgers/milestone-history.md). |
| M25-M30 | Complete | Early performance and runtime-hardening work is historical context only. |
| M31 | Complete | Public demo delivery is historical context and not a current engine-performance target. |
| M33-M40 | Complete | Recent engine-performance work closed fairness, shared caches, compact storage, compiled-active paths, page-bounded materialization, mapped storage, pure upstream `luna_pinyin` native parity with `rsmarisa` hot-path lookup, M39 long-input hardening, and M40 compiled sentence lookup indexing for both Track A long rows while preserving the Track B Cantonese profile guard row. |
| M41 | Complete | Browser-harness startup optimization for tracked `apps/yune-web/`, with production-browser evidence, runtime packaging fixed, redundant startup deploy avoided, schema-scoped worker startup, and separate claims from native engine performance. |

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
