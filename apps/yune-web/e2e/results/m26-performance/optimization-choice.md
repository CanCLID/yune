# M26 Optimization Choice

> **Status:** Complete - **Milestone:** M26 (performance hardening) - **Updated:** 2026-06-22 - **Type:** evidence

## Measured Owners

Task 3 startup evidence identified the largest measured owner across Tasks 1-3:

- Fresh startup `startup:complete.totalMs=10324`; assets loaded at `120ms`; `schema:select`/runtime init finished at `10324ms`.
- Reload startup `startup:complete.totalMs=10435`; assets loaded at `81ms`; `schema:select`/runtime init finished at `10435ms`.
- Native after evidence also shows `startup_real_jyut6ping3_mobile_runtime_ready` remains around median `15103350.100us`.

This startup/runtime-init owner is larger than a safe M26 first slice because it spans browser worker startup, Emscripten runtime state, `TypeDuckRuntime.init`, schema install/deploy selection, dictionary parse/index build, and persistence. It is deferred to [`docs/plans/m27-plan-typeduck-web-startup-runtime-init.md`](../../../../../docs/plans/m27-plan-typeduck-web-startup-runtime-init.md) with the M26 profiling evidence attached.

Task 1 native large-real-asset benchmarks identified the next lower-risk measured owner as the TypeDuck dynamic-correction path, not exact dictionary lookup:

- `per_key_real_jyut6ping3_mobile_jigaajiusihaa_correction_engine_only`: median `451490.692us`, p95 `467909.308us`, cold-first-key `428053.900us`, `dynamic_correction_lookup=entries_by_code.keys`.
- The matching correction full-ABI row was median `21474.323us`, which indicates the synthetic engine-only correction scenario isolates the worst bounded scan owner more strongly than ordinary browser typing.
- Ordinary real TypeDuck full-ABI rows were much lower: `hai` median `15750.133us`, `ngohaig` median `12686.971us`, `jigaajiusihaa` median `21959.831us`.

Task 2 browser keydown-to-paint evidence was smaller and mostly warm-frame bound:

- `hai`: p95 keydown-to-paint `60ms`, p95 worker process `24ms`.
- `jigaajiusihaa`: p95 keydown-to-paint `58ms`, p95 worker process `44ms`.
- paging: keydown-to-paint `7ms`.
- reverse lookup: p95 keydown-to-paint `25ms`, p95 worker process `7ms`.

## Chosen M26 Slice

The first optimization targets the TypeDuck dynamic-correction scan in `crates/yune-core/src/translator/mod.rs` by applying the exact edit-distance length lower bound before candidate checks and before `typeduck_restricted_distance` allocates its distance matrix. The current distance model charges insertion/deletion as `2`, so any canonical code whose byte-length delta alone exceeds `TYPEDUCK_CORRECTION_MAX_DISTANCE` cannot become a correction match. This preserves oracle-visible text, ordering, comments, paging, commit behavior, and ABI layout because it only skips candidates that the existing distance function would have rejected.

This is intentionally the lower-risk measured M26 slice after deferring the larger startup/runtime-init owner to M27. Larger follow-ups also remain possible after M26 for avoiding double candidate storage and using a purpose-built correction/prism index.

## After Result

After `cargo bench -p yune-rime-api --bench frontend_baselines`, the isolated correction stress row moved:

- `per_key_real_jyut6ping3_mobile_jigaajiusihaa_correction_engine_only`: median `451490.692us` -> `121712.662us`; p95 `467909.308us` -> `124420.115us`; cold-first-key `428053.900us` -> `71611.200us`.
- `per_key_real_jyut6ping3_mobile_jigaajiusihaa_correction_full_abi`: median `21474.323us` -> `21307.215us`; p95 `22806.554us` -> `21603.546us`; cold-first-key `37745.100us` -> `37371.500us`. This row is a full-ABI correction-config lifecycle row, not the isolated stress path, so it is expected to move less.

The measured stress-path owner improved by roughly `3.7x` median / `3.8x` p95 without changing ABI layout or candidate materialization semantics.
