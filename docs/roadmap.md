# Roadmap

Yune is a Rust input-method engine that uses **upstream librime as a
compatibility and performance oracle** while building a cleaner Rust engine.
M45 is complete with measured native-engine blockers. It confirmed upstream
candidate-output parity for Track A `n`/`ni`/`hao`, kept `hao` inside the
short-key ratio target, and split steady resident memory from real peak memory.
It does not close as a full performance or memory success because `n` and `ni`
remain above the `<=3.0x` short-key target and Track A still records a real
`127,475,712 B` cold-start peak. WEB-01 is active next as the browser-harness
sidecar for WASM linear-memory reservation and startup payload only. It must
label inherited native changes as post-M45 baseline, must not include new
`crates/` changes, and must not claim native engine memory wins.

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
  including completed M45 native-engine gates, completed M44 native-engine gates,
  completed M43 native-engine gates, completed M42 native-engine gates, closed
  M37-M40 engine gates, and completed M41 web-harness gates.
- [`reports/yune-vs-librime-performance.md`](./reports/yune-vs-librime-performance.md)
  and [`reports/yune-vs-librime-root-cause-analysis.md`](./reports/yune-vs-librime-root-cause-analysis.md)
  - current performance comparison and diagnosis.
- [`plans/completed/m45-plan-native-short-key-latency-memory-attribution.md`](./plans/completed/m45-plan-native-short-key-latency-memory-attribution.md)
  - completed partial native-engine plan for Track A short-prefix latency and
  Track A memory attribution after M44.
- [`plans/completed/m44-plan-native-performance-owner-reduction.md`](./plans/completed/m44-plan-native-performance-owner-reduction.md)
  - completed partial native/profile plan for reducing four M44 performance
  owner families without regressing M40/M42/M43 guards.
- [`plans/completed/m43-plan-native-memory-short-key-owner-reduction.md`](./plans/completed/m43-plan-native-memory-short-key-owner-reduction.md)
  - completed native-engine plan for memory-owner attribution and the selected
  `poet.entries_by_code` owner reduction.
- [`plans/active/web01-plan-yune-web-wasm-heap-payload-optimization.md`](./plans/active/web01-plan-yune-web-wasm-heap-payload-optimization.md)
  - active browser-harness sidecar for yune-web/My RIME comparison,
  `INITIAL_MEMORY` calibration, and Jyutping startup payload reduction.
- [`plans/completed/m42-plan-abbreviation-sentence-parity-short-key-guardrails.md`](./plans/completed/m42-plan-abbreviation-sentence-parity-short-key-guardrails.md)
  - completed native-engine plan for incomplete-pinyin abbreviation sentence
  parity and short-key guardrails.
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
| Engine performance | M45 is complete as a partial native-engine follow-up to M44. Final `hao` passes at `24.267us` / `2.110x`, while `n` remains `68.900us` / `3.313x` and `ni` remains `49.450us` / `3.458x`. Track A steady resident rows meet the resident target (`87,498,752-98,684,928 B`), but the real cold-start peak remains `127,475,712 B`, so M45 records a standing peak-cost blocker. | Keep native reports scoped to M45 evidence. Any next native slice needs a new plan and a narrower owner for the `n`/`ni` constant factor or peak-memory attribution. |
| Web harness startup and memory | M41 is complete for the tracked `apps/yune-web/` browser harness. WEB-01 is active as a browser-only WASM linear-memory reservation and Jyutping payload plan. The fresh 2026-06-27 comparator baseline records Yune public-demo `luna_pinyin` at `128.0 MiB` peak WASM linear memory versus My RIME `16.0 MiB`, and Yune public-demo Jyutping at `893.1 MiB` versus My RIME `68.0 MiB`; Yune Jyutping also loads `33.5 MiB` unique encoded resources versus My RIME `24.9 MiB`. | Run WEB-01 on post-M45 main with evidence labelled post-M45. Draft win gates are Luna `<=64 MiB`, Jyutping `<=256 MiB` with `<=128 MiB` stretch, and Jyutping resources below `28 MiB` or a path-owned no-go. Claims stay limited to browser linear-memory, payload, and worker-retention owners. |
| AI-native engine layer | M11/M13 proved a default-off local AI layer can sit on top of the deterministic engine. | Keep AI outside the classic deterministic performance path unless a named engine experiment explicitly enables it. |
| Future platform work | Platform-specific native frontends remain outside this repo roadmap. | Start a separate repository or separate plan before changing platform/application contracts. |

## Authoritative Sequence

1. **WEB-01 yune-web WASM linear memory and payload sidecar** - active under
   [`plans/active/web01-plan-yune-web-wasm-heap-payload-optimization.md`](./plans/active/web01-plan-yune-web-wasm-heap-payload-optimization.md).
   It covers browser-harness code, public-demo packaging, WASM build flags,
   browser benchmarks, reports, and evidence. It must calibrate lower browser
   `INITIAL_MEMORY` from observed WASM linear-memory high-water plus headroom
   before claiming a browser win, and it must not widen M45/M44 native-engine
   claims.
2. **Future AI-native engine experiments** - later, and only after classic
   engine performance is no longer dominated by avoidable pipeline costs.
3. **Future TypeDuck/profile-storage slices** - only with a new scoped plan,
   fresh owner evidence, and no TypeDuck-profile speed claim unless the profile
   row is explicitly selected as the target.

Trigger-gated, not scheduled: extracting the full processor pipeline from
`yune-rime-api` into `yune-core` lands only when a real non-ABI consumer needs
the full input path. Do not milestone that extraction speculatively.

## M45 Closeout

M45 is complete with measured blockers under
[`plans/completed/m45-plan-native-short-key-latency-memory-attribution.md`](./plans/completed/m45-plan-native-short-key-latency-memory-attribution.md).
It is a native-engine-only follow-up to M44 and the post-M44 diagnostic
profiling.

Final M45 result:

- `n`: `68.900us`, `3.313x`; target missed, measured benchmark-parity blocker.
- `ni`: `49.450us`, `3.458x`; target missed, measured benchmark-parity
  blocker.
- `hao`: `24.267us`, `2.110x`; target met.
- Startup/runtime-ready: `23,386.400us`, `0.875x`; guard passed.
- Session create/select/destroy: `23,444.800us`, `0.921x`; guard passed.
- `zhongguo`: `61.225us`, `0.373x`; guard passed.
- M40 long rows: `0.939x` and `0.720x`; guard passed.
- M42/M44 abbreviation rows: `0.440x` and `0.640x`; guard passed with
  candidate-output behavior preserved.
- Track A steady resident memory: `87,498,752-98,684,928 B`; resident target
  met.
- Track A peak memory: `127,475,712 B`; real standing peak-cost blocker.

M45 records the memory verdict as
`steady-state-meets-target-standing-peak-cost`: resident rows are below
`107,797,708 B`, but the first startup still records the `127 MB` peak, so
the peak cannot be reframed away as a pure benchmark artifact. No memory
storage rewrite is retained.

M45 remains native-engine scoped. It makes no browser, frontend, WASM,
product-delivery, packaging, deployment, public-demo, broad TypeDuck-profile,
AI, learned `.gram`/octagram, or plugin ABI claim. WEB-01 starts after this
point and must label inherited native evidence as post-M45 baseline.

## M44 Closeout

M44 is complete under
[`plans/completed/m44-plan-native-performance-owner-reduction.md`](./plans/completed/m44-plan-native-performance-owner-reduction.md).
It is a native-engine-only follow-up to the post-M43 bottleneck analysis.

The targets are:

- `hao <=28.300us` and `ni <=41.813us`, with same-run ratios no worse than
  `2.5x` and `3.0x`;
- `cszysmsrsd <=3,094.635us` and `zybfshmsru <=3,183.353us`, with same-run
  ratios no worse than `2.5x` and `4.0x`;
- Track A peak memory `<=107,797,708 B`;
- Track B short rows `h`, `ha`, `hai`, `hau`, `nei`, and `ngo` reduced by at
  least `50%`, with selected lookup-explosion counters down at least `75%`.

Final M44 result:

- `hao`: `24.700us`, `2.123x`; target met.
- `ni`: `49.450us`, `3.434x`; target missed, measured blocker.
- `cszysmsrsd`: `545.020us`, `0.445x`; target met.
- `zybfshmsru`: `540.970us`, `0.634x`; target met.
- Track A peak memory: `127,619,072 B`; memory target missed.
- Track B short rows: `84.7-92.4%` faster, with selected exact lookups reduced
  to `1-3` per key; target met.

The M44 candidate-output artifact is scoped to the two abbreviation rows.
Short-key and Track B output preservation are guarded by benchmark/profile rows
and focused regression tests. The memory miss also remains above the historical
M42 `+5%` ceiling of `125,763,994 B`.

M44 closes as a partial result because not all four target families pass.

M44 still excludes browser/frontend speed, packaging, deployment, public-demo
behavior, learned `.gram`/octagram, plugin ABI, and broader schema breadth.

Evidence retention note: the current disk-pressure scan found
`docs/reports/evidence/` at about `120.95 MiB` and
`apps/yune-web/e2e/results/` at about `35.10 MiB`, while `target/` is about
`32,669.87 MiB`. M44 should keep new evidence compact and index final artifacts,
but old final evidence should not be deleted as part of this milestone. Any
cleanup of `target/` or historical evidence should be an explicit maintenance
task with user approval.

## M43 Closeout

M43 is complete under
[`plans/completed/m43-plan-native-memory-short-key-owner-reduction.md`](./plans/completed/m43-plan-native-memory-short-key-owner-reduction.md).
It is a native-engine-only follow-up to M42.

Phase 0 produced the required same-run native benchmark, Track A memory-owner
profile, repeated-run noise band, short-key owner profile, and native
candidate-output guard for `cszysmsrsd` and `zybfshmsru`. The decision gate
selected Branch A, `memory-owner-reduction`, because `poet.entries_by_code`
retained `38,208,541 B` of non-overlapping heap-owned reducible bytes.

Final M43 evidence:

- `poet.entries_by_code`: `38,208,541 B` -> `18,694,662 B`, a
  `19,513,879 B` structural owner reduction.
- Final Track A peak band: `127,492,096-127,627,264 B`, inside the Phase 0
  observed band of `127,574,016-127,647,744 B`.
- Whole-process memory target `<=107,797,708 B` is not met; M43 is not a
  whole-process memory win.
- `hao` and `ni` were profiled but not optimized; final ratios are `3.390x`
  and `4.096x` same-run librime.
- M42 abbreviation rows still match upstream native candidate output, but
  remain latency blockers at `3.479x` and `5.299x`.
- Track A storage remains `rsmarisa_byte_backed`, table/prism remain `mmap`,
  selected heap mirrors remain `0`, and `source_fallback=false`.
- Track B remains guard-only at `182.780us/op` median and `191.182us/op` p95;
  no TypeDuck-profile speed claim is made.

M43 therefore closes as a partial structural memory reduction with a measured
whole-process memory blocker. Future work must choose a new target: another
memory-owner pass, abbreviation graph/search latency, or a separate
`hao`/`ni` short-key branch.

## M42 Closeout

M42 is complete under
[`plans/completed/m42-plan-abbreviation-sentence-parity-short-key-guardrails.md`](./plans/completed/m42-plan-abbreviation-sentence-parity-short-key-guardrails.md).
It followed the implementation branch after Phase 0 proved that upstream
librime `1.17.0` exports meaningful `luna_pinyin` candidates for
`cszysmsrsd` and `zybfshmsru`.

Final M42 behavior evidence:

- `cszysmsrsd`: Yune now matches upstream first-page candidate text, comments,
  order, context preedit, commit preview, and first-page metadata.
- `zybfshmsru`: Yune now matches upstream first-page candidate text, comments,
  order, context preedit, commit preview, and first-page metadata.
- `RimeGetInput` remains Yune's raw keystroke buffer while context preedit
  carries the segmented display string; the candidate-output parity claim is
  native ABI candidate behavior, not a browser or product claim.

Final M42 performance evidence:

- `cszysmsrsd`: Yune `4,127.580us`, librime `1,189.890us`, ratio `3.469x`.
- `zybfshmsru`: Yune `4,257.100us`, librime `839.860us`, ratio `5.069x`.
- `ni` and `hao` were profiled before short-key optimization; the measured
  owner is translator fixed overhead, and no short-key optimization was
  attempted.
- The M40 full-pinyin path remains protected: the 37-character Track A row is
  `0.957x`, the 59-character Track A row is `0.721x`, Track A peak working set
  is `119,775,232 B`, storage remains `rsmarisa_byte_backed`, table/prism stay
  mmap, selected heap mirrors remain `0`, and `source_fallback=false`.
- The Track B 50+ row remains a guard only at `186.513us/op` median and
  `204.680us/op` p95; no TypeDuck-profile speed claim is made.

M42 is therefore a behavior-parity closeout with a measured abbreviation
latency blocker, not a performance win.

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
| Engine performance | Native engine startup, schema/session lifecycle, mmap-backed `rsmarisa` marisa-table lookup, lazy/page-bounded translation, context export, memory, allocation, completed M40 sentence lookup indexing, completed M42 abbreviation sentence parity/short-key guardrails, completed M43 memory-owner reduction, and completed partial M44 native/profile performance owner reduction | Completed M44 plan: [`plans/completed/m44-plan-native-performance-owner-reduction.md`](./plans/completed/m44-plan-native-performance-owner-reduction.md). Completed M43 plan: [`plans/completed/m43-plan-native-memory-short-key-owner-reduction.md`](./plans/completed/m43-plan-native-memory-short-key-owner-reduction.md). Completed M42 plan: [`plans/completed/m42-plan-abbreviation-sentence-parity-short-key-guardrails.md`](./plans/completed/m42-plan-abbreviation-sentence-parity-short-key-guardrails.md). Completed M40 plan: [`plans/completed/m40-plan-compiled-sentence-lookup-index.md`](./plans/completed/m40-plan-compiled-sentence-lookup-index.md). |
| Web harness startup and memory | Tracked `apps/yune-web/` production build, public-demo dist, browser shell, asset/cache delivery, worker/WASM startup, persistence, schema selection, first key-to-paint, and Chromium memory | Active WEB-01 plan: [`plans/active/web01-plan-yune-web-wasm-heap-payload-optimization.md`](./plans/active/web01-plan-yune-web-wasm-heap-payload-optimization.md). Completed M41 plan: [`plans/completed/m41-plan-yune-web-startup-optimization.md`](./plans/completed/m41-plan-yune-web-startup-optimization.md); final evidence under [`apps/yune-web/e2e/results/m41-yune-web-startup-optimization/`](../apps/yune-web/e2e/results/m41-yune-web-startup-optimization/). |
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
| M42 | Complete with measured blocker | Native-engine abbreviation sentence parity and short-key guardrails: Phase 0 proved the upstream target, Yune now matches candidate output for `cszysmsrsd`/`zybfshmsru`, `ni`/`hao` were profiled, and M40 wins were preserved. The abbreviation rows remain `3.469x`/`5.069x` same-run librime, so future latency work needs a new scoped plan. |
| M43 | Complete with measured blocker | Native-engine memory-owner reduction: Phase 0 selected `poet.entries_by_code`, final evidence reduced that owner by `19,513,879 B`, and whole-process memory remains a measured blocker because Track A peak stayed inside Phase 0 noise rather than reaching the memory-win target. |
| M44 | Complete with measured blockers | Native-engine partial owner reduction closed Track A abbreviation latency, Track B native product-profile short-row lookup explosion, and the `hao` short-key target. `ni` latency and Track A whole-process memory remain measured blockers; no browser, frontend, packaging, deployment, or public-demo speed claim is made from M44 evidence. |
| WEB-01 | Active | Browser-harness WASM linear-memory and payload sidecar for `apps/yune-web/`, with claims limited to browser linear-memory reservation, startup payload, worker retention, and browser evidence. |

## Scope Ledger

A living map so "parity" always names a target. Deferred rows move into scope
only when an engine target needs them; nothing here commits to a timeline.

| In scope - target-driven, measured | Deferred - implement when an engine target needs it | Non-goal |
| --- | --- | --- |
| `luna_pinyin` core versus upstream `1.17.0`, including completed M17 null-grammar sentence/lattice, M18 punctuation processor slices, and completed M42 abbreviation sentence parity for `cszysmsrsd`/`zybfshmsru` | Learned `.gram`/octagram grammar, contextual translation, and broader plugin-backed gears until a named engine target needs them | Bit-for-bit parity with librime internals |
| Common RIME schemas added through explicit breadth milestones | Further schema breadth only with fresh oracle fixtures and owning tests | Unbounded schema checklist work |
| Native engine performance parity for startup, session lifecycle, mmap-backed `rsmarisa` marisa-table lookup, raw lookup, lazy/page-bounded translation, context export, memory, allocation, completed M43 owner-backed memory reduction, and completed partial M44 native/profile owner reduction | Frontend/application delivery evidence and platform packaging | Claiming application-visible wins from native engine evidence |
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
