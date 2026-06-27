# Yune Web vs My RIME Browser Baseline

Date: 2026-06-27

## Scope

This is the WEB-01 browser-harness baseline for `apps/yune-web`, isolated from
native engine optimization. It compares freshly rebuilt local `yune-web`
production artifacts against the open-source My RIME deployment at
<https://my-rime.vercel.app/>.

This report must not be used as a native-engine, M44, packaging, deployment, or
public-demo speed claim. It is only about browser WASM linear memory, browser
startup, worker resources, and browser-harness optimization planning.

The separate optimization plan already exists:

- [`docs/plans/active/web01-plan-yune-web-wasm-heap-payload-optimization.md`](../plans/active/web01-plan-yune-web-wasm-heap-payload-optimization.md)

Current caveat: the repository worktree is mixed with unrelated native/M44 and
browser bug-fix edits. The benchmark evidence below is a current measurement
baseline, not a WEB-01 closeout and not a clean `WEB01-00` branch claim.

## Evidence

Yune samples used fresh local production outputs:

- `apps/yune-web/dist`
- `apps/yune-web/public-demo/dist`
- `apps/yune-web/public/yune-web.js`: `72,378 B`
- `apps/yune-web/public/yune-web.wasm`: `2,594,503 B`

The checked-in reusable benchmark is:

- `apps/yune-web/e2e/yune-web-comparator-benchmark.spec.ts`
- `apps/yune-web/e2e/startup-benchmark/comparator-metrics.ts`

Fresh evidence was generated under:

- `apps/yune-web/e2e/results/yune-web-vs-my-rime-baseline/2026-06-27-current-runtime/report.md`
- `apps/yune-web/e2e/results/yune-web-vs-my-rime-baseline/2026-06-27-current-runtime/summary.json`
- `apps/yune-web/e2e/results/yune-web-vs-my-rime-baseline/2026-06-27-current-runtime/samples.json`

Command:

```powershell
$env:YUNE_WEB_COMPARATOR_BASELINE='1'
$env:YUNE_WEB_COMPARATOR_INCLUDE_MY_RIME='1'
$env:YUNE_WEB_COMPARATOR_SAMPLES='3'
$env:YUNE_WEB_COMPARATOR_PHASE='2026-06-27-current-runtime'
npm.cmd --prefix apps/yune-web/e2e run test:e2e -- --grep "YUNE WEB COMPARATOR" --workers=1
```

Result: `1 passed`.

My RIME source was inspected from the local clone at
`C:\Users\laubonghaudoi\Documents\GitHub\my_rime`, commit
`c73ea172d28f07031ba87a1d71c4d2e1c8ba82a3`, package version `0.10.9`. The
local checkout has source but no built `public/rime.*` runtime assets, so the
runtime comparator uses the live deployment. My RIME's WASM build flags set
`ALLOW_MEMORY_GROWTH=1` and `MAXIMUM_MEMORY=4GB`; it does not set an explicit
`INITIAL_MEMORY`.

## Method

- Browser: Chromium through Playwright, headless, `1365x900`, `zh-HK`.
- Samples: 3 fresh browser profiles per app/schema row.
- Schemas:
  - `luna_pinyin`, input `ni`
  - Jyutping, input `nei`
- Yune readiness:
  - `documentElement.dataset.yuneInitialized === "true"`
  - no loading indicator
  - startup-complete diagnostic present
  - selected schema active
- My RIME readiness:
  - editable textarea present
  - worker `Module.HEAPU8.byteLength` visible
  - My RIME's "Copy link for current IME" control enabled, which means the
    Vue `loading` flag is false and `setIME` has finished.
- WASM memory:
  - Yune: startup and key diagnostics reporting `HEAPU8.buffer.byteLength`.
  - My RIME: direct worker evaluation of `Module.HEAPU8.byteLength`.
  - My RIME does not expose an allocator high-water counter, so its peak is the
    max observed snapshot at ready, candidate, and commit.
- Metric meaning: `WASM linear memory` is the browser-visible reserved/current
  WASM heap size. It is real browser memory pressure, but it is not the same as
  active Rust/native engine-owned bytes.
- Latency rows are directional because `SAMPLES=3` is enough for stable linear
  memory but not enough for a strong 10% latency guard.

## Results

| Scenario | Schema | Ready ms | Input->candidate ms | Commit ms | WASM ready | WASM peak | Unique encoded resources | Commit |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| My RIME live | Luna Pinyin | `655` | `98` | `116` | `16.0 MiB` | `16.0 MiB` | `8.5 MiB` | `你` |
| Yune public demo | Luna Pinyin | `932` | `68` | `112` | `128.0 MiB` | `128.0 MiB` | `5.5 MiB` | `伱` |
| Yune tracked build | Luna Pinyin | `930` | `69` | `116` | `128.0 MiB` | `128.0 MiB` | `5.4 MiB` | `伱` |
| My RIME live | Jyutping | `994` | `87` | `126` | `56.6 MiB` | `68.0 MiB` | `24.9 MiB` | `你` |
| Yune public demo | Jyutping | `6,621` | `119` | `120` | `893.1 MiB` | `893.1 MiB` | `33.5 MiB` | `你` |
| Yune tracked build | Jyutping | `6,574` | `105` | `116` | `893.1 MiB` | `893.1 MiB` | `33.5 MiB` | `你` |

## Visual Dashboard

![Observed WASM linear memory](./evidence/yune-web-vs-my-rime-baseline/visuals/web01-wasm-heap.svg)

![Ready-to-input startup](./evidence/yune-web-vs-my-rime-baseline/visuals/web01-ready-to-input.svg)

![Unique encoded browser resources](./evidence/yune-web-vs-my-rime-baseline/visuals/web01-resource-payload.svg)

## Findings

1. WEB-01 is the right separate track.

   The current issue is browser-harness memory/startup behavior, not the M44
   native/profile engine work. The active WEB-01 plan already isolates
   `apps/yune-web`, public-demo packaging, browser WASM build flags, worker
   resource loading, and browser evidence.

2. The old 128 MiB story was stale for Jyutping.

   After rebuilding the current local WASM runtime, `jyut6ping3_mobile` reaches
   `893.1 MiB` by browser ready time and stays there through candidate and
   commit. This matches the local UI reading. The deployed `128.0 MiB` number
   is therefore an older artifact/runtime state, not the current refreshed
   local baseline.

   Luna Pinyin still behaves like the old fixed floor: it starts and stays at
   `128.0 MiB`.

3. Lowering `INITIAL_MEMORY` is still useful, but it is not enough for Jyutping.

   Yune still builds with a `128 MiB` initial browser heap floor. My RIME proves
   that Luna-style browser RIME startup can run at `16.0 MiB`, so WEB-01 should
   still calibrate a lower initial floor.

   But the Jyutping row grows far beyond the initial floor. The immediate owner
   is browser WASM linear-memory growth during Jyutping startup/schema init,
   not a display bug and not a first-key leak. If a lower initial floor leaves
   Jyutping at about `893 MiB`, the remaining owner must be treated as runtime
   allocation during schema deployment/init or deeper native/runtime retained
   state, then handed back with browser evidence.

4. Jyutping startup payload is a second browser-owned gap.

   Yune public-demo Jyutping loads `33.5 MiB` unique encoded resources versus
   My RIME's `24.9 MiB`. The largest Yune resources are:

   | Resource | Encoded |
   | --- | ---: |
   | `jyut6ping3_scolar.dict.yaml` | `6.8 MiB` |
   | `jyut6ping3_scolar.table.bin` | `5.8 MiB` |
   | `jyut6ping3.table.bin` | `4.1 MiB` |
   | `jyut6ping3_scolar.reverse.bin` | `3.4 MiB` |
   | `jyut6ping3.dict.yaml` | `3.3 MiB` |
   | `yune-web.wasm` | `2.5 MiB` |
   | `jyut6ping3_scolar.prism.bin` | `2.2 MiB` |
   | `loengfan.dict.yaml` | `1.7 MiB` |

   This payload gap is separate from the `WASM 佔用` number, but it explains
   much of the `6.6 s` ready-to-input time for Yune Jyutping.

5. Candidate latency is not the primary blocker in this browser comparison.

   After ready, Yune public-demo Jyutping records `119 ms` input-to-candidate
   externally and an internal keydown-to-paint median of `19 ms`, with worker
   process median `5 ms`. With only 3 samples, this is directional, but the
   dominant browser blockers are startup heap growth and eager payload, not
   steady-state candidate lookup.

## Recommended WEB-01 Execution

1. Keep WEB-01 separate and clean.

   Before optimization claims, start from a branch with no `crates/` changes.
   The current evidence is valid as a measurement baseline, but it does not
   satisfy the WEB01-00 clean-branch gate.

2. Calibrate `INITIAL_MEMORY`.

   Make `scripts/yune-web-wasm-build.sh` accept
   `YUNE_WEB_INITIAL_MEMORY_BYTES`, then test `64 MiB`, `48 MiB`, and `32 MiB`
   candidates with `ALLOW_MEMORY_GROWTH=1` still enabled.

   Expected outcome:

   - Luna should be able to drop below the current `128.0 MiB` floor if no
     hidden startup owner appears.
   - Jyutping may still grow to about `893 MiB`; if so, report that as the
     next measured blocker rather than calling the initial-memory work a full
     success.

3. Add richer startup memory markers.

   The comparator proves Jyutping is already at `893.1 MiB` by ready time. The
   next run should record per-phase worker markers for `wasm-glue`, runtime
   create, filesystem mount, asset load, deploy, and schema select so the
   remaining owner is visible at the browser boundary.

4. Prune or defer eager Jyutping assets.

   Audit each path in `YUNE_WEB_JYUTPING_SHARED_ASSETS`. The first candidates
   are the scolar source/compiled pairs, reverse lookup files, and `loengfan`.
   Any removal must preserve typing, commit, schema switching, reverse lookup,
   and userdb persistence.

5. Release copied asset buffers after MEMFS writes.

   This may not change `WASM 佔用`, because that metric is linear memory, but it
   can still reduce worker JS heap and overall Chromium pressure.

## Current Verdict

There is a separate plan, and the fresh baseline confirms it is needed. The
current browser-harness problem is not only "128 MiB is high"; for Jyutping,
fresh local `yune-web` now grows to `893.1 MiB` during startup while My RIME
peaks at `68.0 MiB`. WEB-01 should proceed as a browser-harness optimization
track, but it should not be marked successful until the Luna floor, Jyutping
WASM growth, Jyutping payload, and worker-retention questions are each closed
with browser evidence.
