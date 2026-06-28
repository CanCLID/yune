# M47 iOS-Budget Native Memory Reduction Plan

> **Status:** Draft / measured baseline + adversarial verification incorporated - **Track:** Engine performance (portable; cross-platform memory budget) - **Created:** 2026-06-28 - **Updated:** 2026-06-28 - **Type:** attribution-first measurement-and-reduction plan
>
> **For agentic workers:** attribution before optimization (the M43/M45/M46 rule). Do not retain a reduction branch that is not justified by a measured owner. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Drive the **single-active-schema native working set under an iOS keyboard-extension budget** so that Yune can be embedded in a Cantoboard-style iOS keyboard (and benefit Android, Windows IME, WASM, and desktop/server embedding at the same time). Target: **steady ≤ 48 MB, peak ≤ 64 MB** for one active schema; **stretch peak ≤ 48 MB**. The current measured baseline is `jyut6ping3_mobile` steady **~298 MB** / peak **~482 MB** — roughly **4.6×/7.5× over** — so this is architecture-level reduction, not tuning.

**Why now / why portable:** The product target is an iOS keyboard extension (reference: [Cantoboard](https://github.com/Cantoboard/Cantoboard), which ships librime via [librime-ios-build](https://github.com/Cantoboard/librime-ios-build) under the extension's hard memory ceiling). iOS app extensions are jetsam-killed at a small budget (treated here as **64 MB**, target **48 MB**). All of the reduction levers — dictionary storage, lazy loading, compact indexes, allocator pressure, eager-materialization removal, startup transients, asset size — are **portable engine work** that can be designed, implemented, and measured on Windows and ships to every platform. Only the **final** "fits in an actual iOS keyboard extension" proof needs an Apple device; that validation is a later Phase 2 frontend gate, explicitly out of this plan.

## Budget (iOS-shaped, measured on Windows)

| Dimension | Target | Stretch | Current (lean native probe, jyut6ping3_mobile) |
| --- | --- | --- | --- |
| Steady working set, one active schema | ≤ 48 MB | ≤ 40 MB | **~298 MB** |
| Peak working set (load transient included) | ≤ 64 MB | ≤ 48 MB | **~482 MB** |
| Startup | no source-fallback, no rebuild, no large transient parse spike | — | deploy ~11 MB (cheap); the spike is at `create_session` |
| Assets | compiled-only mobile profile, lazy/optional packs | — | shipped compiled; multilingual profile loaded eagerly |
| Correctness | committed-asset tests for real typing cases stay green | — | WEB03-10 committed-asset Jyutping test green |

## Measured baseline (the starting point this plan attacks)

Source: lean native probe `crates/yune-rime-api/tests/native_memory_probe.rs` (real `RimeApi`, mmap path, `prebuilt_data_dir` set so deploy reuses+mmaps instead of rebuilding, one schema per fresh process, own `WorkingSet64`). Full write-up: [`docs/reports/ios-memory-budget.md`](../../reports/ios-memory-budget.md). Adversarially verified 2026-06-28 (workflow: 4 refute agents + synthesis).

- **The cost lands at `create_session()`, not `deploy()`.** jyut6ping3_mobile: baseline 5.2 MB → deploy **10.8 MB** → `create_session` **293 MB** (+283 MB) → select/typing flat → steady **298 MB**, peak **482 MB**. `create_session` eagerly loads the workspace **default** schema's dictionary and materializes heap structures. (Code-traced: `session.rs` → `apply_initial_schema_to_session` → `install_schema_translator_chain` → `CompactTableStore::from_table_bin_byte_source` + `read_yune_table_advanced_payload` + `StaticTableTranslator::from_compact_table_store`.)
- **Per-default-schema steady (lean probe):** luna_pinyin **62.6 MB** (peak 294), cangjie5 **94.9 MB** (peak 102), jyut6ping3_mobile **~298 MB** (peak 482). Only jyut shows a large steady↔peak gap (~184 MB transient that frees back).
- **Ruled out as the owner:** sentence model (A/B disabling `enable_sentence`+`enable_completion` moved steady 298→297 MB ≈ 0; `poet` owner rows are 0 for jyut), and OpenCC (`.ocd2` total 77 KB).
- **Classified heap-required ≈ 44 MB** (owner profile): `lookup_records` **31.9 MB + 8.1 MB** (the dictionary is loaded **twice** — primary + secondary/reverse), `syllabary_codes` ~4.2 MB; mmap'd table storage ~15 MB is **file-backed, not private heap**.
- **~235 MB is currently UN-OWNED** in the lean 298 MB steady (298 − ~44 MB classified heap − ~18 MB mmap'd file pages). This is **not yet attributed** — "allocator retention of the load transient" is a *hypothesis* (no `#[global_allocator]`, system heap retention unmeasured), not a demonstrated fact. Phase 0 resolves it.
- **Harness hygiene (load-bearing):** every "MB" must name its harness. iOS-relevant = the lean probe (298 MB). The `415–436 MB` figure is the **.NET dual-DLL benchmark hosting yune + librime together** — never the iOS footprint. The browser `160 MiB` is **WASM** (no mmap, owned tables) — a different deployment.

## Boundary

- **In scope:** portable engine memory reduction measured on Windows — allocator instrumentation/attribution, removing eager materialization at `create_session` (e.g. `lookup_records`, the `all_codes()`→`normal_codes` HashSet, the double dictionary load), bounding the load transient, lazy/optional dictionary-panel data, compact-index improvements, slimmer mobile profile / lazy optional packs, no-rebuild/no-source-fallback startup, and the lean-probe measurement harness.
- **In scope with coordination:** any reduction that changes a user-visible behavior (e.g. dictionary panels, reverse lookup, completion) must keep the committed-asset correctness tests green and be expressed through the existing ABI, not a default-ABI widening.
- **Out of scope:** the iOS keyboard frontend itself, TSF/Android/Apple frontend work, widening Yune's default upstream ABI, librime as a runtime fallback, and **any "iOS-ready" claim** — that requires a later real-Apple-device validation pass.
- **Reference only:** Cantoboard / librime-ios-build as the budget and packaging analog.

## Phase 0 — Attribution (mandatory before any reduction branch)

The single biggest finding is that **~235 MB of the 298 MB steady is un-owned**. No reduction branch is authorized until it is attributed.

- [ ] **M47-ATTR-01:** Add an allocator/private-byte instrument to the lean probe — a counting `#[global_allocator]` wrapper (live bytes + high-water) and/or per-phase `PROCESS_MEMORY_COUNTERS_EX` private bytes — to split the 298 MB into **live retained heap** vs **allocator-retained-free** vs **mmap'd-clean file pages**. This directly tests (not infers) the "allocator retention" hypothesis.
- [ ] **M47-ATTR-02:** Bisect the +283 MB `create_session` jump with phase probes *inside* the load: after table mmap, after `read_yune_table_advanced_payload` (`lookup_records`), after the `all_codes()`→`normal_codes` HashSet build, and after the secondary/reverse dictionary load. Capture allocator high-water at each to separate the ~184 MB transient (peak 482) from the ~298 MB steady owner.
- [ ] **M47-ATTR-03:** Confirm the **double dictionary load** (31.9 MB + 8.1 MB `lookup_records`): identify the secondary/reverse dictionary, whether it is required on the keyboard path, and whether the two loads can share one byte source.
- [ ] **M47-ATTR-04:** Record the attribution verdict (owner table with measured bytes, like M43/M46) and select the highest-leverage reduction branch. If the bulk is allocator-retained-free, the branch is allocator strategy; if live-retained, the branch is eager-materialization removal.

## Phase 1+ — Reduction (gated on Phase 0 owners)

Candidate levers, to be confirmed/ordered by Phase 0 (do not implement speculatively):

- [ ] **M47-RED-01 (lookup_records lazy/skip):** `lookup_records` (~40 MB across both loads) is annotated "required by TypeDuck dictionary panels" — UI a keyboard extension does not render. Prototype a lazy/opt-out load on the keyboard path; measure the steady delta on the lean probe; keep committed-asset candidate tests green.
- [ ] **M47-RED-02 (single dictionary load):** Eliminate the duplicate primary/secondary load if Phase 0 shows it is redundant on the keyboard path.
- [ ] **M47-RED-03 (bound the create_session transient):** Stream/avoid the ~184 MB transient (the peak driver) so peak approaches steady; avoid materializing the full `all_codes()` HashSet where a bounded/iterator form suffices.
- [ ] **M47-RED-04 (allocator strategy):** If Phase 0 shows large allocator-retained-free memory, evaluate a decaying global allocator (e.g. mimalloc/jemalloc with aggressive page return) as a portable win, measured on the lean probe.
- [ ] **M47-RED-05 (asset/profile slimming):** Compiled-only mobile profile, lazy/optional multilingual packs, so the eagerly-loaded default schema is the minimal keyboard dictionary.
- [ ] **M47-RED-06 (startup hygiene):** Guarantee the keyboard path never rebuilds, never source-falls-back, and ships compiled assets only (prebuilt mmap), so the build transient cannot occur on device.

## Success bar

- Lean native probe, single active `jyut6ping3_mobile`: **steady ≤ 48 MB** and **peak ≤ 64 MB** (stretch peak ≤ 48 MB), with the committed-asset Jyutping correctness test (WEB03-10) and the full `yune_web`/`cantonese_parity`/`upstream_luna_pinyin_parity` gates green.
- Every reported number names its harness; no "iOS-ready" claim without a later Apple-device validation pass.
- If the target proves infeasible without dropping required behavior, close with a measured no-go and a named trade-off (matching M45/M46 honesty), not a silent miss.

## Guardrails

- Attribution before optimization; no retained branch without a measured owner.
- Lean probe is the iOS proxy; the .NET dual-DLL harness (415–436 MB) and the WASM browser number (160 MiB) are different deployments and must be labeled as such.
- Preserve upstream-observable behavior and TypeDuck candidate output on every change; correctness tests are the gate.
