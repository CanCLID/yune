# WEB-01 Yune Web WASM Heap And Payload Optimization Plan

> **Status:** Active sidecar - **Milestone:** WEB-01 (browser-harness WASM
> heap and payload optimization) - **Created:** 2026-06-26 - **Type:**
> browser-harness plan
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

WEB-01 is a harness-only sidecar that can run in parallel with M43. It starts
from browser evidence, then applies the lowest-risk owner first:

1. Make the yune-web/My RIME browser comparison benchmark reusable.
2. A/B test lower Yune browser `INITIAL_MEMORY` with bounded linear growth.
3. Prune or defer eager browser schema assets only when real-browser evidence
   proves behavior is preserved.
4. Release copied asset buffers after MEMFS/IDBFS install where the worker only
   needs metadata or can reload by path.
5. Publish closeout evidence that separates harness wins from native-engine
   wins.

M43 owns native retained state and short-key engine owners. WEB-01 owns only
browser build flags, browser asset loading, worker memory retention,
public-demo packaging, and browser evidence.

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
- This is current/reserved WASM linear-memory size. It is not a precise
  "active bytes used by the engine" metric.
- Reducing `INITIAL_MEMORY` is therefore a real harness win when it reduces the
  browser's committed/reserved linear-memory floor, but it must be described as
  a browser linear-memory reservation reduction, not as proof that native
  engine active memory use decreased.

## Baseline

The current branch evidence was captured on
`codex/yune-web-wasm-heap-optimization` after commit `712a5a8`. That commit
also contains a native `crates/yune-rime-api` userdb/schema-installer change
from the earlier userdb bug fix. WEB-01 execution must first split or rebase
that native change out of the harness branch before making optimization claims.

Until that cleanup is done, the `2026-06-26` browser rows below are useful as
preliminary measurements, not a clean harness-only baseline.

Current browser baseline from
`apps/yune-web/e2e/results/yune-web-vs-my-rime-baseline/2026-06-26/`:

| Scenario | Schema | Ready ms | Input->candidate ms | Commit ms | WASM linear ready | Observed linear peak | Unique encoded resources |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| My RIME live | Jyutping | `894` | `30` | `19` | `56.6 MiB` | `68.0 MiB` | `24.9 MiB` |
| Yune public demo | Jyutping | `1164` | `30` | `20` | `128.0 MiB` | `128.0 MiB` | `33.5 MiB` |
| My RIME live | Luna Pinyin | `547` | `30` | `17` | `16.0 MiB` | `16.0 MiB` | `8.5 MiB` |
| Yune public demo | Luna Pinyin | `764` | `30` | `24` | `128.0 MiB` | `128.0 MiB` | `5.4 MiB` |

Current known owners:

- Yune browser linear-memory size is fixed at `128 MiB` because
  `scripts/yune-web-wasm-build.sh` sets `-sINITIAL_MEMORY=134217728`.
- My RIME uses `ALLOW_MEMORY_GROWTH=1` and `MAXIMUM_MEMORY=4GB`, but does not
  set `INITIAL_MEMORY`.
- Yune Jyutping startup eagerly loads large browser assets, including
  `jyut6ping3_scolar.dict.yaml`, `jyut6ping3_scolar.table.bin`,
  `jyut6ping3.table.bin`, `jyut6ping3_scolar.reverse.bin`, and
  `jyut6ping3.dict.yaml`.

Post-M43 rebase check:

- Branch rebase target: `ad93ec7` (`Complete M43 native memory owner
  reduction`).
- Evidence:
  `apps/yune-web/e2e/results/yune-web-wasm-heap-optimization/post-m43-baseline/`.
- Command: `YUNE_WEB_WASM_HEAP_BENCHMARK=1 YUNE_WEB_BENCHMARK_SAMPLES=3
  YUNE_WEB_BENCHMARK_PHASE=post-m43-baseline npm --prefix apps/yune-web/e2e
  run test:e2e -- --grep "YUNE WEB WASM HEAP" --workers=1`.
- Result: passed, `1` Playwright benchmark test.
- Current limitation: this is the existing Yune-only WASM linear-memory
  benchmark. The reusable yune-web/My RIME comparator benchmark remains Task 0
  work.
- Scope limitation: the branch still contains the known native
  `crates/yune-rime-api` userdb/schema-installer diff. Treat this as
  post-M43 branch-state evidence, not final WEB01-00-clean baseline evidence,
  until that native diff is split out.

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
- M43 native memory or short-key owner reductions.
- AI behavior, remote providers, or candidate ranking changes.
- Replacing the deterministic engine with TypeScript-side fake learning,
  TypeScript-side fake candidates, or fake memory accounting.

## Pre-Execution Cleanup Gate

Before Task 0 optimization work starts:

- [ ] Split the existing `crates/yune-rime-api/src/schema_install.rs`
  userdb/schema-installer behavior change out of this WEB-01 branch, or rebase
  WEB-01 onto a commit that does not include it.
- [ ] If the native userdb fix is preserved, give it its own native branch or
  standalone commit with an honest rationale and native/oracle verification
  separate from WEB-01.
- [ ] Confirm the WEB-01 working branch diff contains no `crates/` changes.
- [ ] Re-run or relabel browser baseline evidence after the split. Any evidence
  collected before the split must stay labeled as "preliminary branch state."

## Parallel Coordination With M43

- WEB-01 may run at the same time as M43 because the intended write sets are
  disjoint.
- WEB-01 must not commit changes under `crates/` to claim a browser heap win.
- If M43 lands before WEB-01 closes, rebase WEB-01 and rerun all browser
  evidence. Any memory movement after that rebase must be described as
  "combined branch state" unless the same harness diff was measured before and
  after M43.
- Final WEB-01 claims must say whether the win came from:
  - browser `INITIAL_MEMORY`,
  - browser asset payload/defer behavior,
  - worker JS buffer retention,
  - or native retained state outside WEB-01 scope.

## Acceptance Gates

- `WEB01-00`: The executable WEB-01 branch contains no `crates/` changes.
- `WEB01-01`: The yune-web/My RIME comparator benchmark is reusable and writes
  evidence under `apps/yune-web/e2e/results/yune-web-vs-my-rime-baseline/`.
- `WEB01-02`: `luna_pinyin` public-demo peak WASM linear memory meets the
  Task 0 calibrated target, provisionally `<=64 MiB`.
- `WEB01-03`: `jyut6ping3_mobile` public-demo peak WASM linear memory meets
  the Task 0 calibrated target, provisionally `<=96 MiB`.
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
- `WEB01-10`: Reports do not claim native-engine memory wins unless M43
  evidence separately proves them.
- `WEB01-11`: Latency claims use enough samples to support a `10%` guard, or
  explicitly publish the observed noise band. `SAMPLES=3` is acceptable for
  near-deterministic linear-memory checks, but not sufficient by itself for a
  strong startup or first-key latency regression claim.

## Task 0: Baseline And Benchmark Harness

**Files:**

- Modify: `apps/yune-web/e2e/playwright.config.ts`
- Modify or create: `apps/yune-web/e2e/yune-web-comparator-benchmark.spec.ts`
- Modify or create:
  `apps/yune-web/e2e/startup-benchmark/comparator-metrics.ts`
- Preserve:
  `apps/yune-web/e2e/results/yune-web-vs-my-rime-baseline/2026-06-26/`

- [ ] Add a dedicated comparator benchmark that runs these rows:
  - tracked `luna_pinyin`
  - tracked `jyut6ping3_mobile`
  - public-demo `luna_pinyin`
  - public-demo `jyut6ping3_mobile`
  - optional live My RIME `luna_pinyin`
  - optional live My RIME `jyut6ping3`
- [ ] Record per sample:
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
- [ ] Add environment switches:
  - `YUNE_WEB_COMPARATOR_BASELINE=1`
  - `YUNE_WEB_COMPARATOR_INCLUDE_MY_RIME=1`
  - `YUNE_WEB_COMPARATOR_SAMPLES=<n>`
  - `YUNE_WEB_COMPARATOR_PHASE=<phase-name>`
- [ ] Make My RIME optional. The benchmark must still pass and write Yune-only
  evidence when external network or Vercel/CDN access is unavailable.
- [ ] Re-run the current baseline once and compare it against the existing
  `2026-06-26` evidence. Differences larger than normal browser noise must be
  explained before optimization starts.
- [ ] Add a calibration run before accepting the provisional `64 MiB` /
  `96 MiB` targets:
  - build with a lower `INITIAL_MEMORY` floor plus `ALLOW_MEMORY_GROWTH=1`;
  - exercise startup, first candidate, commit, reload, schema switching, userdb
    persistence, and reverse lookup for `luna_pinyin` and
    `jyut6ping3_mobile`;
  - record the settled and peak `HEAPU8.buffer.byteLength` after growth;
  - derive final per-schema linear-memory gates from observed high-water plus
    explicit headroom;
  - if the provisional gates are too low or too loose, update `WEB01-02` and
    `WEB01-03` before implementation proceeds.
- [ ] For latency regression rows, prefer at least `7` samples. If only `3`
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

## Task 1: Lower Browser Initial WASM Memory

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
  - baseline vs final ready-to-input,
  - baseline vs final unique encoded resources,
  - owner attribution waterfall or path table for asset pruning.
- [ ] State the final attribution:
  - `browser-initial-memory-win`,
  - `browser-asset-payload-win`,
  - `browser-js-retention-win`,
  - `native-owned-remaining`,
  - or `measured-no-go`.
- [ ] If the branch has been rebased after M43, explicitly say whether final
  evidence is pre-M43, post-M43, or combined.
- [ ] Update `docs/roadmap.md` so WEB-01 appears as the active browser-harness
  sidecar while M43 remains the active native-engine milestone.
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
YUNE_WEB_COMPARATOR_BASELINE=1 \
YUNE_WEB_COMPARATOR_INCLUDE_MY_RIME=1 \
YUNE_WEB_COMPARATOR_SAMPLES=7 \
YUNE_WEB_COMPARATOR_PHASE=final \
npm --prefix apps/yune-web/e2e run test:e2e -- --grep "YUNE WEB COMPARATOR" --workers=1
```

## Closeout Rules

WEB-01 may close as a win if:

- the branch contains no `crates/` changes; and
- Yune public-demo `jyut6ping3_mobile` peak WASM linear memory drops from
  `128.0 MiB` to the Task 0 calibrated target, provisionally `<=96 MiB`; and
- `luna_pinyin` peak WASM linear memory drops from `128.0 MiB` to the Task 0
  calibrated target, provisionally `<=64 MiB`; and
- startup/first-key medians stay within the `10%` regression guard; and
- typing, commit, schema switching, reverse lookup, and userdb persistence
  smoke pass.

WEB-01 may close as a measured no-go if:

- Lower `INITIAL_MEMORY` fails for behavior or stability reasons; and
- eager assets are proven required for current supported browser behavior; and
- retained copied buffers are proven not to dominate browser JS heap; and
- the report names the remaining owner as native/runtime retained state or a
  future deeper runtime boundary.

WEB-01 must not close by claiming that M43 reduced browser memory unless the
same browser benchmark proves the harness diff independently.
