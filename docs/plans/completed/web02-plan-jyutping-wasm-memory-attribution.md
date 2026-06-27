# WEB-02 Jyutping WASM Memory Attribution Plan

> **Status:** Complete - Phase 0 success / reduction follow-up required - **Track:** Web harness (browser WASM linear memory) - **Created:** 2026-06-27 - **Closed:** 2026-06-27 - **Type:** attribution-before-optimization plan
>
> Follows WEB-01 (browser measured-no-go) and M46 (`measured-no-go-owner-unclassified`). M46 closed the browser Jyutping `893.1 MiB` high-water as **unclassified**: payload-family trimming and `INITIAL_MEMORY=64 MiB` did not move it, but nobody attributed what the bytes actually are. This plan **classifies the owner before any reduction work**, because Jyutping is the product for a mobile-heavy HK audience and `893.1 MiB` risks OOM-crashing mobile tabs at launch.
>
> **Review note (2026-06-27):** an external review correctly flagged that an earlier draft's "H1 refuted" finding was based on inspecting the wrong artifact and an inverted storage-format claim. This revision demotes that to a provisional probe result and adds **Task 0 (artifact reconciliation)** before any verdict. See "Provisional probe result (superseded)" below.

## Goal

Decompose the browser Jyutping WASM linear-memory high-water (`936,509,440 B` = `893.1 MiB`) into concrete, byte-counted (or byte-bounded) owners with a definitive **byte-backed vs source-fallback** verdict, so a later reduction branch targets a *measured* owner instead of re-running spent levers. Luna settles at a fixed `167,772,160 B` (`160.0 MiB`).

Both numbers are invariant to asset payload and to `INITIAL_MEMORY` (WEB-01); JS heap is only `5–6 MB`; median == max. The owner is not understood. This plan understands it.

The fair external yardstick is **My RIME on `luna_pinyin`** (`160` vs `16 MiB`). My RIME's Cantonese-only `68 MiB` Jyutping row is **not** a like-for-like target — Yune ships a multilingual TypeDuck dictionary set. No Jyutping↔My-RIME memory-parity claim is in scope.

## Closeout verdict (2026-06-27)

Phase 0 succeeds. The owner is no longer unclassified:

- The public-demo Jyutping browser path selects `owned_heap`, not byte-backed
  storage.
- `source_fallback=true` with reason
  `source fallback after compiled reject: Invalid("prism parse failed: UnsupportedVersion")`.
- The shipped public-demo Jyutping prisms are `Rime::Prism/3.0`, while Yune's
  compact/byte-backed compiled-prism path requires current `Rime::Prism/4.0`
  payloads.
- Post-deploy `user/build` contains no generated Jyutping compiled table,
  reverse, or prism files in the measured web ABI path; selection falls back to
  the shipped `shared/` compiled inputs and then rejects the `Prism/3.0` files.
- Retained owner rows name two heap dictionaries:
  `translator.entries_by_code=510,925,748 B` for `1,139,357` items and
  `translator.entries_by_code=18,676,626 B` for `70,805` items. The total
  reported retained owner estimate is `529,602,374 B` (`505.1 MiB`).
- The remaining gap to the known `893.1 MiB` WASM high-water is still allocator
  and transient high-water territory, but the reduction branch should first fix
  the source-fallback asset/deploy contract and remeasure.

Owner classification: runtime retained bytes are engine heap dictionaries, but
the root cause is web/public-demo artifact delivery. The shipped public-demo
Jyutping compiled assets are not the current deploy artifacts M46 validated
natively (`Rime::Prism/4.0`, `source_fallback=false`, byte-backed storage).

Evidence:

- [`../../reports/evidence/web02-jyutping-wasm-memory-attribution/`](../../reports/evidence/web02-jyutping-wasm-memory-attribution/)
- [`../../reports/evidence/web02-jyutping-wasm-memory-attribution/task0-web-abi-public-demo/storage-diagnostics.json`](../../reports/evidence/web02-jyutping-wasm-memory-attribution/task0-web-abi-public-demo/storage-diagnostics.json)
- [`../../reports/evidence/web02-jyutping-wasm-memory-attribution/task0-web-abi-public-demo/compiled-asset-inventory.csv`](../../reports/evidence/web02-jyutping-wasm-memory-attribution/task0-web-abi-public-demo/compiled-asset-inventory.csv)
- [`../../reports/evidence/web02-jyutping-wasm-memory-attribution/visuals/web02-public-demo-storage-owner.svg`](../../reports/evidence/web02-jyutping-wasm-memory-attribution/visuals/web02-public-demo-storage-owner.svg)

## Ground truth: the native byte-backed storage model (from M46 `product_path_status.csv`)

| schema | dictionary | selected_storage | table_format | byte_source_len | shipped vs deployed |
| --- | --- | --- | --- | ---: | --- |
| `luna_pinyin` | `luna_pinyin` | `rsmarisa_byte_backed` | marisa string table | `13,013,460` | deployed marisa table (`user/build`) |
| `jyut6ping3_mobile` | `jyut6ping3` | `byte_backed` | `yune_no_marisa_compact` | `15,248,382` | **deployed** (`user/build`), not shipped |
| `jyut6ping3_mobile` | `jyut6ping3_scolar` | `byte_backed` | `yune_no_marisa_compact` | `27,325,622` | **deployed** (`user/build`), not shipped |

Key consequence: the **byte-backed runtime artifact is the *deployed* table built into `user/build/` at deploy time**, not the smaller `shared/` table that ships in the bundle. On native it is mmap-backed; **in WASM there is no mmap**, so the deployed table (15.2 MB + 27.3 MB for scolar, plus prisms/reverses) must live in linear memory, and the **deploy step itself runs at runtime in the browser**.

## Provisional probe result (superseded — pending Task 0)

A no-emcc probe checked the **shipped `shared/jyut6ping3.table.bin`** (`4,306,860 B`) and reported `checksum=fresh` but `parse=fail`. That was **not** source-fallback evidence and is retracted as a verdict, because:

- It inspected the **deploy *input*** (`shared/`, 4.3 MB), not the **byte-backed deployed table** (`user/build/`, 15.2 MB) that the engine actually runs.
- `parse_rime_table_bin_dictionary` is the wrong reader for that input artifact.
- The storage format was stated backwards: **`jyut6ping3` is `yune_no_marisa_compact`** (not marisa); **`luna_pinyin` is `rsmarisa_byte_backed`**.

What the probe *did* establish (kept as Task 0 inputs):

- The shipped `shared/jyut6ping3.table.bin` is **byte-identical** (`sha256 5f686227…`) to the M46 native deploy input.
- But the shipped **`jyut6ping3.dict.yaml` differs** from the M46 native source (`b0abf4fb…` vs `f29d65de…`), so the browser may deploy from a different source than M46 validated.
- The public-demo bundle ships **no `user/build/` table** — the browser performs the deploy/expansion at runtime.

## Hypotheses (ladder)

| # | Hypothesis | Decisive test |
| --- | --- | --- |
| H1 | WASM deploy source-falls-back or rebuilds heap mirrors **despite** valid inputs (e.g. the emscripten-FS deploy can't byte-back the expanded table → an `Owned` heap table at source scale). | Task 1: surface live `storage_label`/`source_fallback`/`byte_source_len` from the browser engine after deploy. |
| H2 | No-mmap materialization: native mmaps the deployed table/prism; WASM must hold them in linear memory. ~50–60 MB of byte sources, not 893 alone. | Task 4: sum `byte_source_len` + side-maps from Task 1 owner rows. |
| H3 | Transient deploy peak that sticks: the runtime deploy/expansion spikes the heap; emscripten linear memory never returns to the OS, so the high-water = the worst deploy moment. (Fits: peak invariant to payload, JS heap tiny, median==max.) | Task 2: allocator high-water split across deploy phases. |
| H4 | Duplicate initialization: schema-switch retains stale engine/session and re-materializes (the historical ~1.9 GiB switch row); single-schema may double too. | Task 3: count deploy/init invocations; capture switch high-water. |

## Phase 0 attribution tasks

**Task 0 — Artifact reconciliation (do first; no emcc required).**
- Reconcile shipped vs deployed vs M46-validated artifacts for `jyut6ping3`/`jyut6ping3_scolar`: confirm whether the shipped `dict.yaml` + `shared/` table produce a current-format `yune_no_marisa_compact` deployed table that byte-backs, or whether the `dict.yaml` divergence yields a different/stale result.
- Method: native deploy of the **public-demo bundle's** schema dir via the real ABI path (`yune-cli frontend` or the benchmark's product-path inspection pointed at `apps/yune-web/public-demo/dist/schema`), then read `selected_storage`/`source_fallback`/`byte_source_len` for the deployed table.
- Output: a definitive native verdict on the *actually shipped* inputs, replacing the retracted probe claim.

**Task 1 — Browser storage-status diagnostics (definitive for H1; not a thin pass-through).**
- The existing `session_inspector_snapshot`/`attach_inspector_debug` carry candidate/pipeline data only; memory-owner rows are exposed natively via `yune_m43_memory_owner_profile_json`, **not** through the web response JSON. Task 1 must **add a web-safe storage/owner diagnostics field**: extend the engine/session snapshot with `selected_storage` (`CompactTableStore::storage_label()`), `is_marisa_backed`, `source_fallback`, `byte_source_len`, `stored_entry_count`, and the `memory_owner_rows()`, then surface it through the existing `yune_web_*` JSON response (no new export; AGENTS.md).
- Wiring: `yune-core` snapshot → `web_runtime.rs` deploy/inspector response → `@yune-ime/yune-web-runtime` → `worker.ts` → page dataset diagnostic the e2e harness already reads.
- Local verification (no emcc): `cargo test -p yune-rime-api --test yune_web` (ABI contract) + `npm --prefix packages/yune-web-runtime test`.

**Task 2 — WASM allocator high-water split (H3).**
- Record `HEAPU8.buffer.byteLength` at pre-init, post-init, post-deploy, post-first-key, steady. Note that `byteLength` is *linear-memory size after growth*, not retained live heap; to attribute retained bytes per owner, pair it with emscripten allocator stats (`mallinfo`/`sbrk` watermark) where available.
- Extend `apps/yune-web/e2e/yune-web-wasm-attribution.spec.ts` with the per-phase markers.

**Task 3 — Schema-switch duplicate-init capture (H4).**
- Count engine/session deploy+init invocations across clean vs `Cangjie → Luna → Jyutping`; capture WASM high-water per step.

**Task 4 — No-mmap materialization accounting (H2).**
- From Task 1 owner rows + `byte_source_len`, sum the deployed byte sources + side-maps that are mmap-backed on native but heap-resident in WASM. Establishes the irreducible no-mmap floor.

## Measurement matrix

- **Schemas:** `luna_pinyin` (fair lane) + `jyut6ping3_mobile` (product) + `jyut6ping3_scolar`.
- **Scenarios:** single-schema cold; schema-switch; per-phase (init/deploy/first-key/steady).
- **External yardstick:** My RIME `luna_pinyin` (fair, `16 MiB`). My RIME Jyutping guard-only.
- Every WASM byte figure is paired with the Task 1 storage-status verdict for the same run.

## Decision gate (end of Phase 0)

- **H1 true (WASM source-falls-back / heap mirror):** fix the WASM byte-source/deploy path → re-measure. Highest-value branch; parallels the native `1.05 GB → 504 MB` byte-backed win.
- **Byte-backed but H3/H4 dominate:** branch = transient-peak reduction (avoid heap mirrors / pre-size linear memory / streamed deploy) and/or lazy-load lookup records; quantify the no-mmap floor.
- **Floor fundamentally above a mobile-safe ceiling (iOS tabs jetsam ~200–400 MB):** this is a **product** decision — desktop-first launch, or invest in a streamed/IndexedDB-backed dictionary. State it honestly; do not grind.

## Success / no-go

- **Phase 0 success = classification and bounding, not exact decomposition.** Success means the `893.1 MiB` is **classified and bounded** into named likely owners (byte-counted where allocator-truth is available, byte-bounded otherwise) with a byte-backed-vs-fallback verdict, and the chosen reduction branch (if any) targets a measured owner.
- Exact allocator-true per-owner decomposition requires the Task 2 watermark instrumentation; without it, owner rows are estimates/classifications, and the success bar is "bound likely owners," not "prove exact owners."
- Reduction is a **gated follow-on branch**, only after the owner is named. This plan does not promise the number moves.

## Boundaries

- **In scope:** browser WASM memory attribution instrumentation, e2e per-phase capture, native byte-backed reconciliation against the *shipped* inputs, the fair `luna_pinyin` My RIME yardstick.
- **Out of scope:** widening the default `RimeApi`; new `yune_web_*` exports when JSON transport works; native engine rewrites; any My RIME Jyutping like-for-like memory claim; AI; octagram/`.gram`.
- **Gates:** `cargo test -p yune-core --test cantonese_parity` for any storage/payload/candidate change; `cargo test -p yune-rime-api --test yune_web` for the ABI contract; real-browser (Playwright) evidence for any browser-visible claim.

## Division of labor / toolchain note (environment-neutral)

- The Rust ABI + TS-runtime instrumentation (Tasks 0/1) is verifiable without a WASM build via `cargo test -p yune-rime-api --test yune_web` and the runtime tests.
- The browser memory capture (Tasks 2/3) needs a local Emscripten WASM build + Playwright. **If Emscripten is available**, build WASM and run the extended `yune-web-wasm-attribution.spec.ts`. **If Emscripten is unavailable in the executing environment**, stop after the ABI/runtime contract tests and mark browser capture **blocked** (do not bake any one machine's toolchain state into the plan).
