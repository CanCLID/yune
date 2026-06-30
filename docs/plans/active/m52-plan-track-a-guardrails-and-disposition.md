# M52 Track A Performance Guardrails And Blocker Disposition Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **Status:** Active - drafted 2026-06-29, not started. - **Track:** Engine performance (native Track A `luna_pinyin`). - **Created:** 2026-06-29 - **Type:** guardrail-freeze + measured-blocker disposition.

**Goal:** Freeze the native Track A `luna_pinyin` performance guardrails and
*disposition* the three measured blockers left open by M50 - the `ni` and
37-character latency rows and the full Luna memory peak. "Disposition" means
each blocker is either really improved or honestly closed with measured
evidence; closing the strict ratio gate is **not** required when the gap is a
bounded, imperceptible absolute cost. This is a closeout-and-freeze milestone,
not a new-feature milestone.

**Architecture:** Native-engine-only, guardrails-first. The firm deliverable is
a regression guardrail set (latency + memory) so the Track A picture can never
silently regress or be reframed again. The blocker dispositions sit on top of
that frozen floor. Two structural facts shape the order of work:

1. **The two latency misses are microsecond-scale.** Final M50 same-run medians
   are `ni` `45.450 us` vs librime `14.400 us` (a `31 us` absolute gap) and the
   37-character row `890.689 us` vs `289.773 us` (a `601 us` gap). Both are far
   below human perception. The `<=3.0x` ratio gate is harsh at these absolute
   values, so a real-reduction push is justified only when owner evidence names
   a bounded, parity-safe change. Otherwise the correct disposition is a
   documented bounded-microsecond ceiling. Do **not** destabilize the
   oracle-parity-critical poet/translator scoring path to chase a ratio.

2. **Latency reduction and memory reduction are in tension on the poet path.**
   M50's latency wins came from doing *less* retained work (dropping the
   `WordGraphEntry` code clone, buffer reuse). The largest named memory owner,
   `poet.vocabulary` (`53.6 MB`), would most plausibly shrink via byte-backed /
   mmap'd access in the style of M47 - which can *add* per-lookup latency. The
   plan resolves this conflict explicitly in favor of memory and guardrails over
   sub-perceptual latency.

**Tech Stack:** Rust (`yune-core`, `yune-rime-api`), the native in-process
benchmark harness (`scripts/benchmark-native-rime-inprocess.ps1`,
`crates/yune-rime-api/benches/native_inprocess_benchmark.rs`), Windows memory
counters, upstream librime `1.17.0` same-run peer evidence, Markdown
evidence/report docs.

---

## Scope

In scope:

- Native Track A `luna_pinyin` only, same-run Yune vs upstream librime `1.17.0`.
- A committed regression guardrail for the tracked latency rows and the Luna
  memory peak. This includes **promoting the existing 59-character Luna row**
  (`zhegeyin...`) into the committed threshold: real users type 50+ characters
  uninterrupted, and that input class is already measured (`~1.5 ms` / `~2.28x`,
  passing since M39/M40) but is not yet a frozen regression guardrail. Add a
  distinct longer input only if one is intentionally named.
- Disposition of the `ni` and 37-character latency blockers: bounded
  parity-safe reduction, or a documented ceiling.
- Disposition of the full Luna Track A memory blocker: a real reduction of a
  named owner, or a precise product-profile-relevance closeout backed by
  evidence of what actually loads `poet.vocabulary` / `poet.entries_by_code`.

Out of scope:

- Web harness, frontend, WASM, public-demo, package, deployment, and
  product-delivery work. M52 reopens none of these.
- M47 TypeDuck keyboard-profile memory, Apple `phys_footprint`, and any
  platform validation. The native Luna peak must not be conflated with the
  M47 `jyut6ping3_mobile` result.
- Learned `.gram` / octagram grammar, unless a fresh upstream `luna_pinyin`
  oracle fixture proves it is required for the named target.
- ABI changes. Default `rime_get_api()` and `RimeCandidate` layouts stay
  unchanged; profile accessors stay as frozen by M51.

## Current Starting Point

M50 closed as a measured partial. Final same-run rows (authoritative closeout
run):

| Row | Yune median | librime median | Ratio | Abs gap | Gate |
| --- | ---: | ---: | ---: | ---: | --- |
| `n` | `61.000 us` | `21.200 us` | `2.877x` | `~40 us` | pass |
| `ni` | `45.450 us` | `14.400 us` | `3.156x` | `~31 us` | blocker |
| 37-char pinyin | `890.689 us` | `289.773 us` | `3.074x` | `~601 us` | blocker |
| 59-char pinyin | `1,543.071 us` | `677.731 us` | `2.277x` | `~865 us` | pass |
| Track A peak memory | `188.4 MB` | `17.1 MB` | `~11x` | `~171 MB` | blocker |

Named reducible memory owners (non-overlapping): `poet.vocabulary` `53.6 MB`,
`poet.entries_by_code` `18.7 MB`, with the process unclassified lower bound at
`106.2 MB`. Evidence root to compare against:
`docs/reports/evidence/m50-track-a-launch-readiness/`.

## Files And Responsibilities

- Create: `docs/reports/evidence/m52-track-a-guardrails-and-disposition/`
  - Fresh same-run baseline, guardrail threshold artifacts, and final evidence.
- Create: a committed guardrail threshold artifact (for example
  `docs/reports/evidence/m52-track-a-guardrails-and-disposition/track-a-thresholds.csv`)
  - The authoritative per-row latency ratio ceilings and the memory-peak
    ceiling that the regression gate checks against.
- Modify if the harness lacks a fail-on-regression comparison mode:
  `crates/yune-rime-api/benches/native_inprocess_benchmark.rs`,
  `scripts/benchmark-native-rime-inprocess.ps1`
  - The 59-character Luna input already exists; add only the
    fail-on-regression comparison mode and any owner counters required to
    prove M52 decisions.
- Modify only if owner evidence requires a bounded, parity-safe change:
  `crates/yune-core/src/translator/mod.rs` (short-prefix path),
  `crates/yune-core/src/poet/mod.rs` (37-character sentence graph /
  `poet.vocabulary` storage),
  `crates/yune-core/src/dictionary/compiled_table.rs` (compact-table lookup).
- Modify on closeout: `docs/reports/yune-vs-librime-performance.md`,
  `docs/reports/yune-vs-librime-root-cause-analysis.md` (dashboard values and
  verdict), `docs/roadmap.md` (M52 verdict and next sequence),
  `docs/ledgers/milestone-history.md` (completed M52 row),
  `docs/requirements.md` (M52 requirement IDs and coverage rows).
- Moved on closeout into
  `docs/plans/completed/m52-plan-track-a-guardrails-and-disposition.md`.

## Task 0: Fresh Same-Run Baseline

**Files:**

- Create: `docs/reports/evidence/m52-track-a-guardrails-and-disposition/phase-0-baseline/`

- [ ] **Step 0.1: Re-baseline Track A same-run.** Run the native in-process
  benchmark for Yune and librime `1.17.0` over the existing tracked inputs
  (`n`, `ni`, `hao`, the 37-character row, the 59-character row), capturing
  per-sample latency, peak/private memory, and the `poet.*` owner rows.
- [ ] **Step 0.2: Confirm the M50 finals reproduce** within run-to-run noise so
  later deltas are attributable to M52 work, not drift. Record the fresh `ni`,
  37-character, 59-character, and memory-peak numbers as the M52 starting
  point.

## Task 1: Freeze The Track A Regression Guardrails (firm deliverable)

This task must complete and close even if Tasks 2 and 3 end in documented
ceilings. The guardrails are the milestone's guaranteed deliverable.

**Files:**

- Create: the committed threshold artifact.
- Modify if needed: benchmark harness/script for a fail-on-regression mode.

- [ ] **Step 1.1: Promote the existing 59-character Luna row** (`zhegeyin...`)
  into the committed threshold so the long-input lattice path is permanently
  gated against regression. It is already benchmarked (`~1.5 ms` / `~2.28x`,
  passing) but is not yet a frozen guardrail. Only add a distinct longer input
  if one is intentionally named with its own owner rationale.
- [ ] **Step 1.2: Write the threshold artifact** with the authoritative ceiling
  for each row: latency ratio ceilings for `n`, `ni`, short prefixes, the
  37-character row, and the 59-character row, plus a Track A peak-memory
  ceiling. Seed ceilings from the Task 0 baseline (a small headroom over the
  current authoritative value, not an aspirational target).
- [ ] **Step 1.3: Add a regression-gate mode** to the benchmark harness that
  compares a fresh run against the threshold artifact and fails if any latency
  ratio or the memory peak exceeds its ceiling. This is the gate that prevents
  silent regression or reframing of the Track A picture.
- [ ] **Step 1.4: Document the gate** in the evidence README and in
  `docs/reports/yune-vs-librime-performance.md` so the threshold file is the
  named source of truth for "Track A has not regressed."

## Task 2: Disposition The Latency Blockers (`ni`, 37-character)

- [ ] **Step 2.1: Re-profile the `ni` and 37-character owners** from the Task 0
  run. Identify whether the remaining cost is a bounded, parity-safe target
  (for example a specific avoidable allocation or redundant scan) or a
  structural floor.
- [ ] **Step 2.2 (conditional): Apply a bounded, parity-safe reduction** only if
  Step 2.1 names one. Any change to `translator/mod.rs` or `poet/mod.rs` must
  keep `upstream_luna_pinyin_parity` and `cantonese_parity` green and must not
  reintroduce a retained heap index. Do not hand-tune the scoring path for a
  ratio gain that is not owner-evidenced.
- [ ] **Step 2.3: Disposition each row.** For each of `ni` and the 37-character
  row: either record it now inside the `<=3.0x` gate with fresh evidence, or
  close it as a **bounded-microsecond ceiling** - a documented statement that
  the absolute gap (`~31 us` for `ni`, `~601 us` for the 37-character row) is
  below user perception and that no parity-safe reduction was found. A
  ceiling closeout is a valid M52 outcome, not a partial.

## Task 3: Disposition The Full Luna Memory Blocker

- [ ] **Step 3.1: Establish product-profile relevance first.** Determine and
  document whether native Track A `luna_pinyin` is a shipping profile or an
  oracle-comparison-only lane, and exactly which runtime path loads
  `poet.vocabulary` (`53.6 MB`) and `poet.entries_by_code` (`18.7 MB`). The
  shipping native target is `jyut6ping3` / TypeDuck (M46/M47); the shipped Luna
  surface is the browser public demo (a separate WASM lane). If nothing ships
  the native Luna full-vocabulary path, the `188 MB` peak is a comparison-lane
  number, and the correct disposition is a precise not-launch-blocking closeout
  rather than a reduction project.
- [ ] **Step 3.2 (conditional): Reduce a named owner** only if Step 3.1 shows a
  shipping path that loads it. The plausible reduction is byte-backing
  `poet.vocabulary` from mmap'd compiled storage in the M47 style. If pursued,
  measure the per-lookup latency cost against the Task 1 ceilings - this is the
  latency/memory tension; memory wins if they conflict, but the latency
  guardrail must still pass.
- [ ] **Step 3.3: Disposition the peak.** Either record a real measured
  reduction of the Track A peak with named owner movement, or close the peak as
  a documented blocker with a precise product-profile-relevance statement.
  Do not reframe a real peak away; name the lane it belongs to.

## Task 4: Closeout

- [ ] **Step 4.1: Run the final same-run benchmark** into
  `docs/reports/evidence/m52-track-a-guardrails-and-disposition/final-native-benchmark/`
  and run the new regression gate against the committed thresholds.
- [ ] **Step 4.2: Run final gates.** Required:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p yune-core short_key
cargo test -p yune-core poet
cargo test -p yune-core --test upstream_luna_pinyin_parity
cargo test -p yune-core --test cantonese_parity
```

Run broader `cargo test --workspace` only if shared ABI/runtime files changed.

- [ ] **Step 4.3: Close with the correct verdict** (see Definition of Done).
- [ ] **Step 4.4: Register requirement IDs** in `docs/requirements.md`
  (M52-ENGINE-01..06 below), update the coverage summary and mapped-count, and
  update the roadmap Snapshot/Sequence/Ledger and milestone-history.
- [ ] **Step 4.5: Commit and push**, then move this plan to
  `docs/plans/completed/`.

## Definition Of Done

M52 closes as **complete** when all three of the following hold. Each blocker
has an explicit honest-closeout path so the milestone is not forced into another
"measured partial":

1. **Guardrails frozen (firm):** a committed threshold artifact and a working
   regression gate cover `n`, `ni`, short prefixes, the 37-character row, the
   existing 59-character Luna row, and the Track A memory peak; the gate passes
   on the final run.
2. **Latency dispositioned:** `ni` and the 37-character row are each either
   inside `<=3.0x` with fresh evidence, or closed with a documented
   bounded-microsecond ceiling rationale. No tracked passing row (`n`, `hao`,
   59-character) regresses above its ceiling.
3. **Memory dispositioned:** the full Luna Track A peak is either reduced with
   named owner movement, or closed with a precise product-profile-relevance
   statement naming what loads `poet.vocabulary`.

Broad clippy is green; reports and roadmap show current values only in dashboard
sections; native Track A scope only - no web, browser, product, package,
deployment, iOS-device, or ABI claim.

## Proposed Requirement IDs

To be added to `docs/requirements.md` on closeout (mirroring the M50/M51 ID
style):

- **M52-ENGINE-01**: A committed Track A latency regression guardrail covers
  `n`, `ni`, short prefixes, the 37-character row, and the existing 59-character
  Luna row, with a fail-on-regression gate.
- **M52-ENGINE-02**: A committed Track A memory-peak attribution/ceiling gate
  prevents silent regression or reframing of the Luna peak.
- **M52-ENGINE-03**: The `ni` latency row is improved under `<=3.0x` or closed
  with a bounded-microsecond ceiling rationale.
- **M52-ENGINE-04**: The 37-character Luna latency row is improved under
  `<=3.0x` or closed with a bounded-microsecond ceiling rationale.
- **M52-ENGINE-05**: The full Luna Track A memory peak is reduced with named
  owner movement or closed with a precise product-profile-relevance statement.
- **M52-ENGINE-06**: M52 stays native Track A scoped and makes no web, browser,
  product, package, deployment, iOS-device, or ABI claim.
