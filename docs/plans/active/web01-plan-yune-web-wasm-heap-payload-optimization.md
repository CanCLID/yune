# WEB-01 Yune Web WASM Heap And Payload Optimization Plan

> **Status:** Active - **Milestone:** WEB-01 (browser-harness WASM
> heap and payload optimization) - **Updated:** 2026-06-27 - **Type:**
> browser-harness execution plan
>
> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

## Goal

Reduce `apps/yune-web` browser WASM linear-memory reservation and startup
payload for `jyut6ping3_mobile` and `luna_pinyin`, using My RIME as a browser
comparator, without making or claiming native-engine changes.

## Architecture

WEB-01 is a harness-only optimization plan that starts after the M44 native
closeout and remains separate from future native residual-owner work. It starts
from browser evidence, then applies the lowest-risk owner first:

1. Make the yune-web/My RIME browser comparison benchmark reusable.
2. Reconcile the current `893.1 MiB` / `6.6 s` Jyutping browser baseline
   against M41's completed `1.25 s` startup evidence before optimizing.
3. Attribute WASM linear-memory by schema asset family early enough to stop if
   the remaining owner is engine-side heap materialization rather than harness
   packaging.
4. A/B test lower Yune browser `INITIAL_MEMORY` with bounded linear growth,
   treating it as a Luna floor lever unless Jyutping attribution proves
   otherwise.
5. Prune or defer eager browser schema assets only when real-browser evidence
   proves behavior is preserved.
6. Release copied asset buffers after MEMFS/IDBFS install where the worker only
   needs metadata or can reload by path.
7. Publish closeout evidence that separates harness wins from native-engine
   wins.

Native residual-owner plans own `ni`, native whole-process memory, and
engine/profile behavior. WEB-01 owns only browser build flags, browser asset
loading, worker memory retention, public-demo packaging, and browser evidence.

The current working hypothesis is that the bulk of the `893.1 MiB` Jyutping
WASM high-water is engine/runtime heap materialization inside the WASM instance,
not just the absence of native `mmap()` and not only transfer payload. My RIME's
same-browser Jyutping row at `68.0 MiB` proves the browser category is
reducible, but WEB-01 cannot fix that engine owner because its executable diff
must stay outside `crates/`. Its realistic Jyutping outcome is therefore either
a harness payload/defer win plus a measured no-go, or a quantified handoff to a
future WASM-memory engine milestone.

## Tech Stack

- Browser harness: `apps/yune-web/` React/Vite app and dedicated worker.
- Browser runtime glue: `apps/yune-web/src/worker.ts`,
  `apps/yune-web/src/rime.ts`, and `apps/yune-web/src/yune-integration/`.
- WASM build flags: `scripts/yune-web-wasm-build.sh`.
- Browser benchmarks: Playwright under `apps/yune-web/e2e/`.
- Evidence roots:
  - `apps/yune-web/e2e/results/yune-web-wasm-heap-optimization/`
  - `apps/yune-web/e2e/results/yune-web-vs-my-rime-baseline/`
- Analysis report:
  `docs/reports/yune-web-vs-my-rime-browser-baseline.md`.
- Comparator source:
  <https://github.com/LibreService/my_rime> at commit
  `c73ea172d28f07031ba87a1d71c4d2e1c8ba82a3`, plus the live comparator at
  <https://my-rime.vercel.app/>.

## Metric Definitions

WEB-01 uses browser-visible WASM linear-memory diagnostics:

- `WASM 佔用` / current WASM linear memory is `HEAPU8.buffer.byteLength` from
  the active worker module.
- `WASM 峰值佔用` / peak observed WASM linear memory is the maximum observed
  `HEAPU8.buffer.byteLength` across the sampled startup and input phases.
- `steady-state-resident-after-ready` is the sampled
  `HEAPU8.buffer.byteLength` after ready-to-input, first candidate, commit, and
  one short idle window. It is still a WASM linear-memory reservation metric,
  but it separates a transient deploy/startup spike from memory that stays
  allocated after the harness is usable.
- This is current/reserved WASM linear-memory size. It is not a precise
  "active bytes used by the engine" metric.
- Reducing `INITIAL_MEMORY` is therefore a real harness win when it reduces the
  browser's committed/reserved linear-memory floor, but it must be described as
  a browser linear-memory reservation reduction, not as proof that native
  engine active memory use decreased.

## Baseline

The current executable WEB-01 branch point is `main` commit `58205ad` (`Fix
public demo schema asset hashes`). The fresh browser baseline rows below were
captured from the earlier current-runtime state at `e4109a41`; `58205ad` is a
deployment-manifest repair that updates LF-normalized public schema hashes and
does not intentionally change runtime behavior. Treat the rows below as the
current same-machine measurement baseline, not as a clean WEB-01 optimization
branch claim.

The first optimization branch must be cut from `origin/main` at `58205ad` or a
newer synchronized commit, and the optimization diff must contain no `crates/`
changes. Any WEB-01 win must be measured against the committed baseline below
with browser evidence from that clean optimization branch.

Current refreshed-runtime comparator baseline from
`apps/yune-web/e2e/results/yune-web-vs-my-rime-baseline/2026-06-27-current-runtime/`:

- Command:
  `YUNE_WEB_COMPARATOR_BASELINE=1 YUNE_WEB_COMPARATOR_INCLUDE_MY_RIME=1
  YUNE_WEB_COMPARATOR_SAMPLES=3
  YUNE_WEB_COMPARATOR_PHASE=2026-06-27-current-runtime npm --prefix
  apps/yune-web/e2e run test:e2e -- --grep "YUNE WEB COMPARATOR"
  --workers=1`.
- Result: passed, `1` Playwright benchmark test.
- Current limitation: these rows were captured before the baseline was
  committed and include inherited M44/native plus browser bug-fix state. They
  are valid current measurement evidence, but not a WEB01-00 optimization
  branch claim.

| Scenario | Schema | Ready ms | Input->candidate ms | Commit ms | WASM linear ready | Observed linear peak | Unique encoded resources |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| My RIME live | Luna Pinyin | `655` | `98` | `116` | `16.0 MiB` | `16.0 MiB` | `8.5 MiB` |
| Yune public demo | Luna Pinyin | `932` | `68` | `112` | `128.0 MiB` | `128.0 MiB` | `5.5 MiB` |
| Yune tracked build | Luna Pinyin | `930` | `69` | `116` | `128.0 MiB` | `128.0 MiB` | `5.4 MiB` |
| My RIME live | Jyutping | `994` | `87` | `126` | `56.6 MiB` | `68.0 MiB` | `24.9 MiB` |
| Yune public demo | Jyutping | `6621` | `119` | `120` | `893.1 MiB` | `893.1 MiB` | `33.5 MiB` |
| Yune tracked build | Jyutping | `6574` | `105` | `116` | `893.1 MiB` | `893.1 MiB` | `33.5 MiB` |

Current known owners:

- Yune Luna Pinyin browser linear-memory size is fixed at `128 MiB` because
  `scripts/yune-web-wasm-build.sh` sets `-sINITIAL_MEMORY=134217728`.
- Current refreshed-runtime Yune Jyutping grows to `893.1 MiB` during browser
  startup/schema init and stays there through candidate and commit. Lowering
  the initial floor alone cannot be claimed as a full Jyutping fix unless the
  calibrated run proves the high-water also falls.
- Because the Jyutping row grows far above the current `128 MiB` floor, Task 1
  is a Luna-only reservation lever unless the Task 0 attribution run proves that
  a lower floor changes Jyutping steady-state high-water too.
- My RIME uses `ALLOW_MEMORY_GROWTH=1` and `MAXIMUM_MEMORY=4GB`, but does not
  set `INITIAL_MEMORY`.
- Yune Jyutping startup eagerly loads large browser assets, including
  `jyut6ping3_scolar.dict.yaml`, `jyut6ping3_scolar.table.bin`,
  `jyut6ping3.table.bin`, `jyut6ping3_scolar.reverse.bin`, and
  `jyut6ping3.dict.yaml`.

Baseline reconciliation requirement:

- M41 closed the tracked `apps/yune-web` startup milestone with a final
  `jyut6ping3_mobile` tracked cold median of `1,254 ms` and public-demo median
  of `1,291 ms`. The refreshed WEB-01 baseline is `6,574 ms` tracked and
  `6,621 ms` public-demo, with `893.1 MiB` observed linear memory.
- Task 0 must explain this before implementation: either the M41 row used a
  lighter/pre-refresh asset path, the current runtime introduced a regression,
  the benchmark phases are not comparable, or the current `893.1 MiB` growth
  changed startup behavior. Until that note exists, WEB-01 may use the
  `893.1 MiB` row for targeting but must not call it a confirmed M41
  regression.

Earlier preliminary browser baseline from
`apps/yune-web/e2e/results/yune-web-vs-my-rime-baseline/2026-06-26/`:

| Scenario | Schema | Ready ms | Input->candidate ms | Commit ms | WASM linear ready | Observed linear peak | Unique encoded resources |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| My RIME live | Jyutping | `894` | `30` | `19` | `56.6 MiB` | `68.0 MiB` | `24.9 MiB` |
| Yune public demo | Jyutping | `1164` | `30` | `20` | `128.0 MiB` | `128.0 MiB` | `33.5 MiB` |
| My RIME live | Luna Pinyin | `547` | `30` | `17` | `16.0 MiB` | `16.0 MiB` | `8.5 MiB` |
| Yune public demo | Luna Pinyin | `764` | `30` | `24` | `128.0 MiB` | `128.0 MiB` | `5.4 MiB` |

The 2026-06-26 rows are retained as historical preliminary evidence. They were
captured before the local runtime was refreshed and therefore do not describe
the current Jyutping high-water.

Historical pre-refresh Yune-only check:

- Branch rebase target: `ad93ec7` (`Complete M43 native memory owner
  reduction`).
- Evidence:
  `apps/yune-web/e2e/results/yune-web-wasm-heap-optimization/post-m43-baseline/`.
- Command: `YUNE_WEB_WASM_HEAP_BENCHMARK=1 YUNE_WEB_BENCHMARK_SAMPLES=3
  YUNE_WEB_BENCHMARK_PHASE=post-m43-baseline npm --prefix apps/yune-web/e2e
  run test:e2e -- --grep "YUNE WEB WASM HEAP" --workers=1`.
- Result: passed, `1` Playwright benchmark test.
- Current limitation: this is retained only as historical Yune-only evidence
  before the refreshed local runtime exposed the `893.1 MiB` Jyutping
  high-water. It must not be used as the current WEB-01 baseline.

| Scenario | Samples | Ready ms | First key ms | WASM linear ready | Observed linear peak | Encoded resources |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| tracked `luna_pinyin` cold | `3` | `776` | `28` | `128.0 MiB` | `128.0 MiB` | `5.4 MiB` |
| tracked `jyut6ping3_mobile` cold | `3` | `1250` | `13` | `128.0 MiB` | `128.0 MiB` | `33.5 MiB` |
| public-demo `luna_pinyin` cold | `3` | `777` | `31` | `128.0 MiB` | `128.0 MiB` | `5.4 MiB` |
| public-demo `jyut6ping3_mobile` cold | `3` | `1263` | `13` | `128.0 MiB` | `128.0 MiB` | `33.5 MiB` |

## Scope Boundaries

In scope:

- `apps/yune-web/src/worker.ts`, `apps/yune-web/src/rime.ts`, and related
  browser diagnostic/UI plumbing.
- `apps/yune-web/src/yune-integration/` only for browser asset write/retention
  behavior.
- `apps/yune-web/e2e/` benchmark and regression coverage.
- `apps/yune-web/public-demo/` build/package behavior.
- `scripts/yune-web-wasm-build.sh` browser WASM build flags.
- Reports and evidence under `docs/reports/` and `apps/yune-web/e2e/results/`.

Out of scope:

- `crates/yune-core/`.
- `crates/yune-rime-api/` behavior, C ABI, native runtime, schema installer, or
  native memory owners.
- Any actual fix for the engine-side WASM memory owner that would move
  Jyutping from `893.1 MiB` toward the My RIME `68.0 MiB` browser comparator.
  That belongs in a future WASM-memory engine plan after WEB-01 produces
  attribution evidence.
- M44 native/profile behavior and future native residual-owner reductions.
- AI behavior, remote providers, or candidate ranking changes.
- Replacing the deterministic engine with TypeScript-side fake learning,
  TypeScript-side fake candidates, or fake memory accounting.

## Clean Execution Branch Gate

Before Task 1 optimization work starts:

- [ ] Create a WEB-01 implementation branch from `origin/main` at `58205ad` or
  a newer synchronized commit.
- [ ] If M45 or any other engine work is active in parallel, run WEB-01 in a
  separate Git worktree and branch. Sharing a working tree is not allowed
  because `crates/` edits would make WEB-01's WASM build use a modified engine
  and violate `WEB01-00`.
- [ ] Confirm the WEB-01 optimization diff contains no `crates/` changes.
- [ ] Keep M44 native/profile changes as inherited baseline state only; do not
  describe WEB-01 results as native-engine wins.
- [ ] Re-run the comparator with `SAMPLES=7` before accepting latency
  regression claims. The `SAMPLES=3` rows are sufficient for current memory and
  payload targeting, but not for a strong latency guard.
- [ ] Record whether the run is `baseline`, `initial-memory`, `asset-pruning`,
  `buffer-release`, `final`, or `measured-no-go` in `YUNE_WEB_COMPARATOR_PHASE`.
- [ ] Do not run WEB-01 browser benchmarks while native benchmark runs are
  active on the same machine. Coding may proceed in parallel across worktrees,
  but latency and memory measurement runs must be serialized.

## Engine/Browser Coordination After M44

- WEB-01 must not commit changes under `crates/` to claim a browser heap win.
- If a future native residual-owner plan lands before WEB-01 closes, rebase
  WEB-01 and rerun browser evidence. Any memory movement after that rebase must
  be described as "combined branch state" unless the same harness diff was
  measured before and after the native change.
- If M45 lands before WEB-01 closeout, final evidence must explicitly label the
  engine base as `pre-M45`, `post-M45`, or `combined-branch-state`. A
  post-M45 rebase does not invalidate WEB-01, but the report must distinguish
  harness-owned changes from inherited engine state.
- M45 memory attribution may be cited as supporting Phase-0 evidence when it
  identifies native `mmap` bytes that become WASM linear-memory owners, but
  WEB-01 must still produce its own browser asset-family attribution table.
- Closeout docs that are shared with engine milestones (`docs/roadmap.md`,
  `docs/requirements.md`, `docs/decisions.md`, and performance reports) must be
  sequenced after whichever engine branch lands first. The second branch rebases
  and reconciles docs instead of blindly merging both closeout narratives.
- Final WEB-01 claims must say whether the win came from:
  - browser `INITIAL_MEMORY`,
  - browser asset payload/defer behavior,
  - worker JS buffer retention,
  - or engine/runtime retained state outside WEB-01 scope.

## Acceptance Gates

- `WEB01-00`: The executable WEB-01 branch contains no `crates/` changes.
- `WEB01-01`: The yune-web/My RIME comparator benchmark is reusable and writes
  evidence under `apps/yune-web/e2e/results/yune-web-vs-my-rime-baseline/`.
- `WEB01-02`: `luna_pinyin` public-demo peak WASM linear memory drops from the
  `128.0 MiB` baseline and meets the Task 0 calibrated target, provisionally
  `<=64 MiB`.
- `WEB01-03`: `jyut6ping3_mobile` public-demo peak WASM linear memory drops
  materially from the `893.1 MiB` baseline only if Task 0 proves a harness-owned
  lever can move the high-water. Draft win gate: `<=256 MiB`; stretch gate:
  `<=128 MiB`. If the core-only or selected-schema-only attribution row already
  exceeds the win gate, WEB-01 must declare the full Jyutping memory target a
  measured no-go early and continue only for payload/defer and handoff evidence.
- `WEB01-04`: Startup median and first-key median regress by no more than
  `10%` versus the WEB-01 baseline for tracked and public-demo builds.
- `WEB01-05`: Jyutping unique encoded browser resources are lower than the
  `33.5 MiB` baseline, or the final report identifies why the remaining
  payload is required.
- `WEB01-06`: Chinese typing still produces candidates and commits `nei`.
- `WEB01-07`: Schema switching still works.
- `WEB01-08`: Reverse lookup assets still load for supported schemas and
  reverse lookup smoke still passes when not blocked by a known unrelated
  reverse-input bug.
- `WEB01-09`: Userdb learning still persists after reload.
- `WEB01-10`: Reports do not claim native-engine memory wins unless a separate
  native milestone separately proves them.
- `WEB01-11`: Latency claims use enough samples to support a `10%` guard, or
  explicitly publish the observed noise band. `SAMPLES=3` is acceptable for
  near-deterministic linear-memory checks, but not sufficient by itself for a
  strong startup or first-key latency regression claim.
- `WEB01-12`: Final evidence separates current WASM linear-memory reservation,
  observed peak WASM linear memory, steady-state-resident-after-ready, unique
  encoded browser resources, worker JS heap/storage estimates, and user-visible
  ready-to-input.
- `WEB01-13`: Task 0 writes an asset-family attribution table for `luna-core`,
  `jyutping-core`, `jyutping-scolar`, `reverse-lookup`, `opencc`, `extras`, and
  `full-jyutping`. The table must name whether each family changes payload
  bytes, transient deploy peak, steady-state linear memory, or only worker JS
  retained bytes.
- `WEB01-14`: The final evidence records WEB-01 branch/worktree isolation,
  engine base label (`pre-M45`, `post-M45`, or `combined-branch-state`), and
  benchmark-run isolation. WEB-01 cannot close if browser benchmark evidence was
  captured from a working tree containing uncommitted or branch-local `crates/`
  changes.

## Task 0: Baseline And Benchmark Harness

**Files:**

- Modify: `apps/yune-web/e2e/playwright.config.ts`
- Modify or create: `apps/yune-web/e2e/yune-web-comparator-benchmark.spec.ts`
- Modify or create: `apps/yune-web/e2e/yune-web-wasm-attribution.spec.ts`
- Modify or create:
  `apps/yune-web/e2e/startup-benchmark/comparator-metrics.ts`
- Modify if needed: `apps/yune-web/src/worker.ts`
- Preserve:
  `apps/yune-web/e2e/results/yune-web-vs-my-rime-baseline/2026-06-26/`
- Evidence:
  `apps/yune-web/e2e/results/yune-web-wasm-heap-optimization/attribution/`

- [x] Add a dedicated comparator benchmark that runs these rows:
  - tracked `luna_pinyin`
  - tracked `jyut6ping3_mobile`
  - public-demo `luna_pinyin`
  - public-demo `jyut6ping3_mobile`
  - optional live My RIME `luna_pinyin`
  - optional live My RIME `jyut6ping3`
- [x] Record per sample:
  - `readyToInputMs`
  - `inputToCandidateMs`
  - `commitMs`
  - current and peak Yune WASM linear-memory size from diagnostics
  - My RIME worker `Module.HEAPU8.byteLength` when same-origin worker access is
    available
  - page and worker resource timings
  - JS heap
  - storage estimate
  - top resource list
- [x] Add environment switches:
  - `YUNE_WEB_COMPARATOR_BASELINE=1`
  - `YUNE_WEB_COMPARATOR_INCLUDE_MY_RIME=1`
  - `YUNE_WEB_COMPARATOR_SAMPLES=<n>`
  - `YUNE_WEB_COMPARATOR_PHASE=<phase-name>`
- [x] Make My RIME optional. The benchmark must still pass and write Yune-only
  evidence when external network or Vercel/CDN access is unavailable.
- [ ] Record the WEB-01 branch name, worktree path, `HEAD`, `origin/main`, and
  whether any engine branch was active in parallel. If engine work is active,
  include the separate engine worktree path or explicitly state that no WEB-01
  benchmark is running concurrently with it.
- [x] Re-run the current baseline once and compare it against the existing
  `2026-06-26` evidence. Differences larger than normal browser noise must be
  explained before optimization starts.
- [ ] Write a baseline reconciliation note under
  `apps/yune-web/e2e/results/yune-web-wasm-heap-optimization/attribution/`
  explaining why M41 reported `1,254 ms` / `1,291 ms` Jyutping startup while
  WEB-01 now sees `6,574 ms` / `6,621 ms` and `893.1 MiB`. The note must name
  whether the difference is benchmark shape, refreshed asset/runtime state,
  deploy/cache state, or an actual regression.
- [ ] Add an attribution benchmark before any optimization that records, for
  each row below, payload bytes, transient deploy/startup peak, ready-time
  linear memory, steady-state-resident-after-ready, worker JS heap/storage
  estimate, and loaded asset paths:
  - `luna-core`: Luna schema assets plus the Luna OpenCC set from
    `YUNE_WEB_LUNA_SHARED_ASSETS`;
  - `jyutping-core`: `YUNE_WEB_COMMON_SHARED_ASSETS`,
    `jyut6ping3.schema.yaml`, `jyut6ping3_mobile.schema.yaml`,
    `jyut6ping3.table.bin`, `jyut6ping3.reverse.bin`, and
    `jyut6ping3_mobile.prism.bin`;
  - `jyutping-scolar`: add `jyut6ping3_scolar.schema.yaml`,
    `jyut6ping3_scolar.dict.yaml`, `jyut6ping3_scolar.table.bin`,
    `jyut6ping3_scolar.reverse.bin`, and `jyut6ping3_scolar.prism.bin`;
  - `reverse-lookup`: add `loengfan.*`, `cangjie3.*`, `cangjie5.*`, and
    `luna_pinyin_yune_reverse.dict.yaml`;
  - `opencc`: add `YUNE_WEB_OPENCC_SHARED_ASSETS`;
  - `extras`: any path in the current Jyutping load set that is not assigned to
    the previous families, with an explicit `none` row if no paths remain;
  - `full-jyutping`: current `YUNE_WEB_JYUTPING_SHARED_ASSETS`.
- [ ] If `jyutping-core` or `jyutping-scolar` already exceeds `256 MiB`
  steady-state-resident-after-ready, mark the full Jyutping memory win as
  `engine-owned-measured-no-go` before Tasks 1-3. Continue WEB-01 only for the
  Luna floor, payload/defer partials, JS retention proof, and a quantified
  future WASM-memory engine handoff.
- [ ] Add a calibration run before accepting the provisional `64 MiB` /
  `256 MiB` / `128 MiB` targets:
  - build with a lower `INITIAL_MEMORY` floor plus `ALLOW_MEMORY_GROWTH=1`;
  - exercise startup, first candidate, commit, reload, schema switching, userdb
    persistence, and reverse lookup for `luna_pinyin` and
    `jyut6ping3_mobile`;
  - record the settled and peak `HEAPU8.buffer.byteLength` after growth;
  - derive final per-schema linear-memory gates from observed high-water plus
    explicit headroom;
  - if the provisional gates are too low or too loose, update `WEB01-02` and
    `WEB01-03` before implementation proceeds.
- [x] For latency regression rows, prefer at least `7` samples. If only `3`
  samples are available, mark the row as directional and publish the noise
  caveat.

Required command:

```sh
YUNE_WEB_COMPARATOR_BASELINE=1 \
YUNE_WEB_COMPARATOR_INCLUDE_MY_RIME=1 \
YUNE_WEB_COMPARATOR_SAMPLES=7 \
YUNE_WEB_COMPARATOR_PHASE=baseline \
npm --prefix apps/yune-web/e2e run test:e2e -- --grep "YUNE WEB COMPARATOR" --workers=1
```

Attribution command:

```sh
YUNE_WEB_WASM_ATTRIBUTION=1 \
YUNE_WEB_WASM_ATTRIBUTION_PHASE=baseline-attribution \
npm --prefix apps/yune-web/e2e run test:e2e -- --grep "YUNE WEB WASM ATTRIBUTION" --workers=1
```

## Task 1: Lower Browser Initial WASM Memory

Task 1 is expected to reduce Luna's browser linear-memory floor. It is not a
Jyutping memory fix unless Task 0 proves the Jyutping high-water falls when the
initial floor is lowered.

**Files:**

- Modify: `scripts/yune-web-wasm-build.sh`
- Modify if needed: `apps/yune-web/e2e/yune-web.spec.ts`
- Evidence:
  `apps/yune-web/e2e/results/yune-web-wasm-heap-optimization/initial-memory/`

- [ ] Change `scripts/yune-web-wasm-build.sh` so the initial memory can be
  configured by environment variable:

```sh
YUNE_WEB_INITIAL_MEMORY_BYTES=${YUNE_WEB_INITIAL_MEMORY_BYTES:-67108864}
```

- [ ] Use that variable in the Emscripten link arg:

```sh
-C link-arg=-sINITIAL_MEMORY=$YUNE_WEB_INITIAL_MEMORY_BYTES
```

- [ ] Keep these flags unchanged:

```sh
-sALLOW_MEMORY_GROWTH=1
-sMEMORY_GROWTH_GEOMETRIC_STEP=0
-sMEMORY_GROWTH_LINEAR_STEP=33554432
-sSTACK_SIZE=8388608
```

- [ ] Start from the Task 0 calibrated target. If no better target is known yet,
  rebuild with `67108864` first.
- [ ] Report Luna and Jyutping separately. A Luna reduction from `128.0 MiB`
  to the calibrated floor is a WEB-01 win even if Jyutping remains
  `engine-owned-measured-no-go`.
- [ ] If 64 MiB passes all gates, try `50331648`.
- [ ] If 48 MiB passes all gates, try `33554432`.
- [ ] Choose the lowest value that passes typing, commit, schema switching,
  userdb persistence, and reverse lookup smoke without more than `10%` startup
  or first-key median regression.
- [ ] Record the failed lower values too. A failed 32 MiB or 48 MiB attempt is
  useful evidence.

Required commands per candidate value:

```sh
YUNE_WEB_INITIAL_MEMORY_BYTES=<bytes> scripts/yune-web-wasm-build.sh
npm --prefix apps/yune-web run build
npm --prefix apps/yune-web run build:public
YUNE_WEB_WASM_HEAP_BENCHMARK=1 \
YUNE_WEB_BENCHMARK_SAMPLES=3 \
YUNE_WEB_BENCHMARK_PHASE=initial-memory-<bytes> \
npm --prefix apps/yune-web/e2e run test:e2e -- --grep "YUNE WEB WASM HEAP" --workers=1
```

Regression smoke:

```sh
npm --prefix apps/yune-web run typecheck
npm --prefix apps/yune-web/e2e run test:e2e -- --grep "WASM heap metrics populate|M42 User Dictionary learns|M22 Bucket 3 schema switcher loads|Shift toggles ASCII mode" --workers=1
```

## Task 2: Classify And Prune Eager Browser Assets

Task 2 starts only after Task 0 has named the memory owner. If Task 0 already
marks the Jyutping linear-memory target `engine-owned-measured-no-go`, Task 2
is a payload/defer optimization and must not be described as the full
`893.1 MiB` memory fix.

**Files:**

- Modify: `apps/yune-web/src/worker.ts`
- Modify if needed: `apps/yune-web/e2e/yune-web.spec.ts`
- Evidence:
  `apps/yune-web/e2e/results/yune-web-wasm-heap-optimization/asset-pruning/`

- [ ] Add a temporary audit mode that records, for every loaded shared asset:
  - path
  - byte size
  - reason (`schema-init`, `reverse-lookup`, `schema-switch`, `opencc`,
    `unknown`)
  - whether the asset is fetched, written to MEMFS, and retained in JS.
- [ ] Classify every path in `YUNE_WEB_JYUTPING_SHARED_ASSETS`.
- [ ] Test removing or deferring one asset family at a time:
  - `jyut6ping3_scolar.*`
  - `loengfan.*`
  - `cangjie3.*`
  - `cangjie5.*`
  - `luna_pinyin_yune_reverse.dict.yaml`
  - Luna compiled assets when not needed by the active schema.
- [ ] For each removal/defer attempt, run:
  - Jyutping `nei` typing and commit.
  - Jyutping reverse lookup supported trigger smoke.
  - Cangjie reverse lookup smoke.
  - Luna reverse lookup smoke.
  - Schema switch Jyutping -> Cangjie -> Luna -> Jyutping.
- [ ] Keep only changes that preserve supported behavior. If lazy reverse
  lookup requires runtime reinit or deploy and causes visible input loss, do not
  ship that lazy path in WEB-01; document it as a future deeper harness/runtime
  boundary.
- [ ] Update startup diagnostics to list assets by reason and bytes, not only
  by path.
- [ ] Update the attribution table after each accepted asset-family change so
  the final report can distinguish payload movement from linear-memory
  movement.

Success target:

- Reduce Jyutping unique encoded browser resources from `33.5 MiB` to below
  `28 MiB`, or publish a path-by-path required-assets table explaining why the
  remaining payload is required.

## Task 3: Release Copied Asset Buffers

**Files:**

- Modify: `apps/yune-web/src/worker.ts`
- Modify: `apps/yune-web/src/yune-integration/adapter.ts`
- Modify if needed: `apps/yune-web/src/yune-integration/assets.ts`
- Evidence:
  `apps/yune-web/e2e/results/yune-web-wasm-heap-optimization/buffer-release/`

- [ ] Stop using long-lived `{ path, content }` arrays when metadata is enough.
- [ ] Keep diagnostics as `{ path, byteLength, sha256?, reason }`.
- [ ] If deploy-cache signatures need content hashes, compute the signature
  when assets are loaded, then release the original `ArrayBuffer`/`Uint8Array`
  after `FS.writeFile`.
- [ ] If schema switching or redeploy needs content again, reload by logical
  path through the existing manifest/cache path instead of retaining every
  buffer forever.
- [ ] Preserve the security rule that runtime resource identifiers are logical
  IDs, not arbitrary filesystem paths.
- [ ] Add browser diagnostics for:
  - retained JS asset bytes before write,
  - retained JS asset bytes after write,
  - number of reloads caused by schema switching.

Expected result:

- This task may not reduce `WASM 佔用`, because that metric is linear memory.
  It should reduce browser worker JS heap or at least prove copied buffers are
  not a major retained owner.

## Task 4: Closeout Evidence And Report Updates

**Files:**

- Modify:
  `docs/reports/yune-web-vs-my-rime-browser-baseline.md`
- Modify or create:
  `apps/yune-web/e2e/results/yune-web-wasm-heap-optimization/final/`
- Modify if needed:
  `docs/roadmap.md`

- [ ] Re-run the WASM linear-memory benchmark for tracked and public-demo
  builds.
- [ ] Re-run the yune-web/My RIME comparator with My RIME enabled when network
  access is available.
- [ ] Add final charts to the report:
  - baseline vs final peak observed WASM linear memory,
  - baseline vs final steady-state-resident-after-ready,
  - baseline vs final ready-to-input,
  - baseline vs final unique encoded resources,
  - baseline vs final worker JS heap/storage estimate,
  - owner attribution waterfall or path table for asset pruning.
- [ ] Include the M41/current-runtime reconciliation note in the final report so
  readers can see why the plan used `893.1 MiB` / `6.6 s` as the current
  WEB-01 target while M41 previously closed at `1.25 s`.
- [ ] State the final attribution:
  - `browser-initial-memory-win`,
  - `browser-asset-payload-win`,
  - `browser-js-retention-win`,
  - `engine-owned-measured-no-go`,
  - `engine-owned-remaining`,
  - or `measured-no-go`.
- [ ] If the remaining `893.1 MiB` owner is engine/runtime heap materialization,
  write a compact future-plan handoff section with the exact Task 0 rows and the
  first files a later WASM-memory engine plan must inspect. Do not implement
  that future engine plan in WEB-01.
- [ ] If the branch has been rebased after later native work, explicitly say
  whether final evidence is pure WEB-01 or combined branch state.
- [ ] State whether final measurements were run with any native benchmark or
  browser benchmark process active in another worktree. If so, discard those
  rows and rerun after the machine is idle enough for the stated latency guard.
- [ ] Update `docs/roadmap.md` so WEB-01 appears as the active browser-harness
  sidecar while native residual-owner work remains a separate future plan.
- [ ] Move this plan to `docs/plans/completed/` only after all acceptance gates
  are satisfied or a measured no-go is documented.

Final required commands:

```sh
npm --prefix apps/yune-web run typecheck
npm --prefix apps/yune-web run build
npm --prefix apps/yune-web run build:public
YUNE_WEB_WASM_HEAP_BENCHMARK=1 \
YUNE_WEB_BENCHMARK_SAMPLES=3 \
YUNE_WEB_BENCHMARK_PHASE=final \
npm --prefix apps/yune-web/e2e run test:e2e -- --grep "YUNE WEB WASM HEAP" --workers=1
YUNE_WEB_WASM_ATTRIBUTION=1 \
YUNE_WEB_WASM_ATTRIBUTION_PHASE=final-attribution \
npm --prefix apps/yune-web/e2e run test:e2e -- --grep "YUNE WEB WASM ATTRIBUTION" --workers=1
YUNE_WEB_COMPARATOR_BASELINE=1 \
YUNE_WEB_COMPARATOR_INCLUDE_MY_RIME=1 \
YUNE_WEB_COMPARATOR_SAMPLES=7 \
YUNE_WEB_COMPARATOR_PHASE=final \
npm --prefix apps/yune-web/e2e run test:e2e -- --grep "YUNE WEB COMPARATOR" --workers=1
```

## Closeout Rules

WEB-01 may close as a win if:

- the branch contains no `crates/` changes; and
- any parallel engine work used a separate branch/worktree and no WEB-01
  benchmark row was captured while native benchmarks were running; and
- Yune public-demo `jyut6ping3_mobile` peak WASM linear memory drops from
  `893.1 MiB` to the Task 0 calibrated target, provisionally `<=256 MiB` with
  `<=128 MiB` as the stretch gate; and
- `luna_pinyin` peak WASM linear memory drops from `128.0 MiB` to the Task 0
  calibrated target, provisionally `<=64 MiB`; and
- startup/first-key medians stay within the `10%` regression guard; and
- typing, commit, schema switching, reverse lookup, and userdb persistence
  smoke pass.

WEB-01 may close as a partial harness win if:

- `luna_pinyin` meets the calibrated lower-floor target; and
- Jyutping unique encoded browser resources are lower or a required-assets
  table proves why they cannot move; and
- the `893.1 MiB` Jyutping high-water is classified as
  `engine-owned-measured-no-go` by the Task 0 attribution rows; and
- the final report clearly separates the harness wins from the future
  WASM-memory engine blocker.

WEB-01 may close as a measured no-go if:

- Lower `INITIAL_MEMORY` fails for behavior or stability reasons; and
- eager assets are proven required for current supported browser behavior; and
- retained copied buffers are proven not to dominate browser JS heap; and
- the report names the remaining owner as engine/runtime retained state or a
  future deeper runtime boundary.

WEB-01 must not close by claiming that M44 or later native work reduced browser
memory unless the same browser benchmark proves the harness diff independently.
