# M55 Native Track A Match-Or-Beat Program Implementation Plan

> **For agentic workers:** If a plan-execution sub-skill (e.g.
> superpowers:executing-plans) is available, use it; otherwise execute the
> checkboxes directly, in order, one phase at a time. Steps use checkbox
> (`- [ ]`) syntax for tracking.

> **Status:** Phase 2 design recorded; implementation next. - **Track:** Engine performance (native Track A `luna_pinyin` comparison lane). - **Created:** 2026-07-03 - **Updated:** 2026-07-03 (Phase 2 poet-storage design note is recorded; implementation has not started). - **Type:** performance research program (multi-phase; storage/algorithm work behind a full-suite regression ratchet; no ABI change, no behavior change).

> **Execution checkpoint (2026-07-03):** Phase 0 evidence is recorded under
> `docs/reports/evidence/m55-native-match-or-beat/phase-0-baseline/`. The first
> fail-on-regression gate exposed an M54 null-grammar performance regression on
> inherited M52 long-input rows. The fix restores the default-off null-grammar
> sentence path while preserving Octagram grammar-aware state. Final Phase 0
> gates `gate-run-5b-null-direct-take` and `gate-run-6-null-direct-take` pass
> against `thresholds/m55-thresholds.csv`. Phase 1 attribution evidence is
> recorded under `phase-1-attribution/`: the diagnostic Luna probe attributes
> `97.49%` of the old `106,039,183 B` lower-bound floor to named buckets, but
> the release allocator probe is blocked by the workspace's `panic=abort`
> release strategy, so the allocator bucket is classification evidence rather
> than a release ceiling. Phase 1 release ratchet `ratchet-gate-1` is green
> with `23` pass rows; the inherited 59-character row remains tight at
> `2.447x`, exactly at the committed ceiling. Phase 2 design is recorded under
> `phase-2-poet-storage/design-note.md`; implementation remains next.

**Goal:** End the whack-a-mole pattern on the native Track A `luna_pinyin` lane
and drive every tracked dimension — startup, session lifecycle, all eight
tracked latency rows, and peak memory — toward same-run librime 1.17.0, while
*keeping* the rows Yune already wins and preserving oracle parity
byte-for-byte.

**Claim discipline for this program:** the committed completion bar (**Tier M**)
is a **bounded-gap** bar (roughly halving the remaining gaps and locking every
dimension behind a ratchet). Full **match-or-beat** on latency and near-peer
memory is the stretch record (**Tier S**). Do not describe Tier M results as
"matching librime" in any public wording; per-row, lane-qualified claims only
(M53 discipline).

This plan implements the roadmap's
[Performance North Star](../../roadmap.md#performance-north-star) and owns the
[Closing The 188 MB Native Track A Memory Gap](../../roadmap.md#closing-the-188-mb-native-track-a-memory-gap)
design sketch. It is scoped to **one lane** (native Track A `luna_pinyin`),
with one structural owner per phase, exactly as the North Star requires.

---

## Why previous rounds felt like whack-a-mole, and what this plan does differently

Previous optimization rounds gated only the dimension being worked on (or a
subset: M52's thresholds cover `n`/`ni`/`hao`/37-char/59-char/memory but **not**
startup, session, or the three rows Yune currently wins). A slice could land a
win on its target row while silently degrading an unguarded row, and the
degradation was only discovered in a later milestone's fresh baseline.

**The core mechanism of this plan is the full-suite ratchet:**

1. **Every tracked dimension gets a committed ceiling** — including the rows
   Yune currently *wins* (`zhongguo`, `cszysmsrsd`, `zybfshmsru`) and the rows
   currently at parity (startup, session), plus Track B (TypeDuck product)
   absolute regression bounds. Nothing is unguarded.
2. **Every phase closes only with the full gate green.** A phase that improves
   its named owner but breaches any other ceiling is **not landable**. There is
   no trading one dimension for another past a committed ceiling.
3. **Ceilings only move down**, with one narrow exception. After a phase lands
   a stable win (two consecutive same-run gate passes), its ceiling is
   tightened to lock the win in. Loosening a ceiling to make a gate pass is
   forbidden and is itself a no-go signal (revert or re-scope). *Narrow
   exception:* the win-row ceilings may be re-baselined **once**, at Phase 2
   close only, as their own reviewed step with written justification — and
   even then must stay `<1.00x` (see "Win-row ceilings" below).
4. **Same-run only.** Every Yune number is compared against librime measured in
   the same benchmark run on the same machine. Cross-run comparisons are never
   evidence.

**Win-row ceilings.** The three winning rows run through the same translator/
poet machinery that Phases 2-4 modify (the abbreviation rows are
sentence-composed), so a hair-tight ceiling would make the keystone memory
phase unlandable over a still-winning few-percent latency shift. Set each
win-row ceiling to `min(0.95x, worst-of-3-baseline-runs x 1.20)`. The wins must
never flip (`<1.00x` is absolute); the one-time Phase 2 re-baseline exists so a
deliberate, measured, still-winning trade can be recorded instead of smuggled.

## Decided Calls (do not re-litigate without new evidence)

- **No retained heap indexes at runtime.** M49 measured the retained vocabulary
  prefix index (`poet.vocabulary_prefix_index`) at about `+35 MB` heap and
  rejected it. Latency indexes must be compiled into mmap-backed artifacts at
  schema-compile/deploy time (zero retained heap, OS-paged), like the existing
  `compact_table.storage` / compiled prism.
- **Memory wins conflicts, ceilings bind.** If a change trades memory against
  latency, memory wins the design argument — but the latency ceilings in the
  ratchet must still pass. If both cannot hold, the phase closes partial/no-go.
- **Parity is non-negotiable.** Candidate output must be byte-identical through
  the real production path. `upstream_luna_pinyin_parity` and
  `cantonese_parity` gate every phase. Phase 3 additionally requires the
  sentence-fixture expansion *before* any poet change lands.
- **Track A stays a comparison lane.** No product or application-visible claims
  from this plan. The shipping native lane remains TypeDuck/Jyutping (Track B),
  which is gated here only against regression (absolute ceilings), not compared
  to a librime peer.
- **Portable technique preferred.** Storage changes go through the same seams
  M47 used: the byte-source/access **trait lives in `yune-core`** (no
  `unsafe`), the **mmap construction lives in `yune-rime-api`** (which already
  depends on `memmap2` and carries the local `unsafe_code = "allow"` lint
  table), mirroring how `compact_table.storage` is wired through
  `crates/yune-rime-api/src/schema_install.rs`. WASM has no mmap; keep the seam
  loadable-from-bytes so the WASM path keeps working, but make no WASM claims
  either way.
- **No public C ABI change, no `unsafe` in `yune-core`, no new default-on
  behavior.** M51 ABI contract and the M53 claim-wording discipline hold.
- **Browser lanes are out of scope** — queued separately as the roadmap's
  browser fair-lane slice (a future WEB-numbered milestone). Do not mix browser
  numbers into this plan's evidence.
- **Gate precedence during the program:** the M55 thresholds artifact is the
  *working gate for this program's phases*. The M52 artifact remains the repo's
  standing guardrail until the Phase 5 handover formally supersedes it.

## Current Starting Point

Verified repo facts (2026-07-03). Provenance: the five gated latency rows,
startup, session, and the memory numbers are from the **M52 final same-run
benchmark of 2026-06-30**
(`docs/reports/evidence/m52-track-a-guardrails-and-disposition/final-native-benchmark/`).
The three win rows are **carried from the 2026-06-29 full-suite run**
(`docs/reports/evidence/current-performance-dashboard-2026-06-29/current-native-track-a.csv`)
— the M52 final run gated only the five threshold inputs. Phase 0 re-measures
everything same-run; until then treat the win-row ratios as indicative.

| Dimension | Yune | librime | Ratio | State |
| --- | ---: | ---: | ---: | --- |
| startup (`startup_warm_shared_assets_runtime_ready`) | `24,139.200 us` | `21,686.900 us` | `1.113x` | near parity, unguarded, **noisy** (see Phase 0) |
| session (`session_create_select_destroy`) | `23,404.000 us` | `23,390.700 us` | `1.001x` | parity, unguarded, noisy |
| `n` | `60.300 us` | `21.400 us` | `2.818x` | guarded `3.050x` |
| `ni` | `44.950 us` | `14.300 us` | `3.143x` | guarded `3.223x` |
| `hao` | `24.967 us` | `11.633 us` | `2.146x` | guarded `2.287x` |
| 37-char pinyin (`ceshiyixiachangjushuruxingnengzenyang`) | `895.178 us` | `293.211 us` | `3.053x` | guarded `3.267x` |
| 59-char pinyin (`zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong`) | `1,545.754 us` | `687.795 us` | `2.247x` | guarded `2.447x` |
| `zhongguo` | `46.150 us` | `166.600 us` | `0.277x` | **Yune wins**, carried 2026-06-29, unguarded |
| `cszysmsrsd` | `517.720 us` | `1,218.320 us` | `0.425x` | **Yune wins**, carried 2026-06-29, unguarded |
| `zybfshmsru` | `547.580 us` | `859.790 us` | `0.637x` | **Yune wins**, carried 2026-06-29, unguarded |
| Track A peak working set | `188,383,232 B` | `17,276,928 B` peer peak | `~10.9x` | guarded `198,000,000 B` |

Memory owners (M52 `memory-owner-profile.csv`): `poet.vocabulary` `53.6 MB`
(heap), `poet.entries_by_code` `18.7 MB` (heap), `poet.lookup_index` `2.7 MB`
(guarded M40 index), process unclassified lower bound `105.6 MB` (**not
attributed to a named owner — this is the single largest number in the lane**).

Known measurement caveat (from the M52 evidence): the startup ratio swung
`0.935x -> 1.113x` between the M52 phase-0 and final runs (~19%; session
`0.924x -> 1.001x`). Startup/session gating is therefore **not stable at a
`1.05x` ratio granularity today**; Phase 0 owns hardening this before any
ceiling is committed (see Phase 0).

Existing machinery this plan builds on (do not reinvent):

- Benchmark + gate: `scripts/benchmark-native-rime-inprocess.ps1` with
  `-TrackAThresholds <csv> -FailOnRegression` (see
  `Invoke-TrackAThresholdCheck`), `-DeployProductBeforeBenchmark` (Track B
  product deploy), `-SkipTrackB`, `-TrackAInputs`, and `-OutputRoot`. Read the
  script's `param()` block before the first run. **Warning: the script clears
  the directory `-OutputRoot` points at** (`Clear-DirectoryUnder`); always pass
  a fresh leaf directory, never a parent that holds committed evidence.
- Threshold artifact format:
  `docs/reports/evidence/m52-track-a-guardrails-and-disposition/track-a-thresholds.csv`
  (`kind,workload,input,metric,ceiling,unit,source_value,notes`). The matcher
  keys on `workload`+`input`; startup/session gate through the existing
  `latency_ratio` kind using the workload ids named in the table above with an
  empty `input`.
- Oracle provisioning: the script asserts
  `target/upstream-oracle/1.17.0/rime-shared` and `rime-user/build`. If the
  assert fails (fresh clone/worktree), run
  `scripts/capture-upstream-luna-pinyin.ps1` first — it rebuilds the oracle
  root from `schema-src/rime-{prelude,essay,luna-pinyin,stroke}` + `extract/`
  (see the M12 oracle-refresh plan for provenance).
- Track B prerequisite: the script hard-codes its product schema root to the
  **gitignored machine-local** `apps/yune-web/source/public/schema` checkout.
  If that tree is absent, record Track B as blocked in the phase evidence and
  proceed with `-SkipTrackB` — do not silently repoint the script.
- **Two schema roots, both intentional — do not "unify" them:** the Track B
  *benchmark* reproduces the TypeDuck product deployment from the machine-local
  `apps/yune-web/source/public/schema`; the *product-path CLI comparison*
  (Execution Rule 4) uses the **committed** `apps/yune-web/public/schema`, per
  the M48 product-verification precedent. They are different fixtures for
  different claims.
- Memory owner attribution: `crates/yune-core/src/memory_owner.rs` /
  `memory_probe.rs` (the machinery behind `memory-owner-profile.csv`), with
  `byte_class` distinguishing `heap_owned_*` from `mmap_file_backed`.
- Byte-backing precedent: M47 plan and evidence
  (`docs/plans/completed/m47-plan-ios-budget-native-memory-reduction.md`,
  `docs/reports/ios-memory-budget.md`).
- Raw-lookup diagnostics: `raw_lookup_microbench.csv` in the M52 final
  evidence (`ni`: raw table lookup `18.0 us` of `41.6 us` translator median;
  37-char: raw lookup `28.9 us` of `891.0 us` — the poet graph owns ~96% of
  the sentence row).
- Oracle fixture capture: `scripts/capture-upstream-luna-pinyin.ps1` +
  `scripts/oracle-rime-probe.cs`, driven by the scenarios JSON in the
  (gitignored) oracle root (`luna-pinyin-scenarios.json`); each fixture set
  pins an `oracle-manifest.json` checked by
  `crates/yune-core/tests/oracle_fixture_provenance.rs`, which must be updated
  alongside new fixtures.

## Win Bars (set now, before any code)

Three tiers. **Tier R is mandatory at every phase close. The program closes
"complete" at Tier M. Tier S is the stretch record, not the pass bar.**

- **Tier R (regression-proof, always):** full-suite ratchet green — every
  ceiling in `m55-thresholds.csv` passes, including win-row and Track B
  absolute ceilings; parity suites green; `cargo fmt --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace` green.
- **Tier M (bounded-gap completion bar):** same-run medians of
  - startup `<=1.05x` *or* within the Phase-0-measured startup noise band,
    whichever is looser (see Phase 0's startup hardening deliverable);
    session likewise;
  - 37-char `<=1.50x`, 59-char `<=1.50x`;
  - `n` `<=2.00x`, `ni` `<=2.00x`, `hao` `<=1.75x`;
  - `zhongguo`, `cszysmsrsd`, `zybfshmsru` still `<1.00x` (wins kept);
  - Track A peak working set `<=125,000,000 B` (revised by Phase 1 evidence).
    The earlier provisional `60,000,000 B` bar is not substantiated: Phase 1
    split the old `106,039,183 B` unclassified lower-bound proxy into
    allocator-live bytes beyond named owner rows (`71,729,000 B`, diagnostic
    test-local allocator), already byte-backed mapped rows (`13,044,872 B`),
    process/runtime resident overhead (`18,606,706 B`), and a transient peak
    delta (`15,167,488 B`). The evidence-backed post-Phase-2 projection is
    roughly `188,600,320 - 72,370,289 = 116,230,031 B` peak before mmap
    residency and remnant overhead; `125 MB` is the committed bounded-gap
    memory bar. Phase 2b remains conditional: it may tighten toward
    `110,000,000 B` only after adding subowner evidence for the allocator-live
    bucket or bounding the transient peak.
- **Tier S (match-or-beat — stretch record):** every latency ratio `<=1.00x`;
  Track A peak working set `<=26,000,000 B` (about `1.5x` the librime peer
  peak).

Phase 1's attribution evidence may revise the two memory numbers **with
committed justification in this plan file**. Latency bars may only be revised
downward.

## Scope

In scope:

- Phase 0 baseline + full-suite ratchet artifact (measurement/tooling only).
- Phase 1 attribution of the `105.6 MB` unclassified memory floor
  (diagnostics only).
- Phase 2 byte-backed poet storage (`poet.vocabulary`,
  `poet.entries_by_code`) via a versioned mmap-backed compiled artifact.
- Phase 2b (conditional) reduction of the top reducible owners named by
  Phase 1's budget table.
- Phase 3 sentence-fixture expansion, then poet-graph constant-factor latency
  work on the 37/59-char rows.
- Phase 4 compile-time short-key acceptance/exact-row index (zero retained
  heap) for `n`/`ni`/`hao`.
- Phase 5 closeout: final evidence, dashboard rewrite, roadmap/requirements/
  history updates, gate handover.

Out of scope:

- Browser/WASM lanes and any browser claims (a separate future WEB-numbered
  browser slice; WEB-05 is the harness control-surface milestone, not this).
- TypeDuck/Track B improvements (Track B is regression-gated only).
- Any C ABI change, any schema-visible behavior change, any candidate-output
  change, octagram/grammar work, AI-layer work.
- Retained runtime heap indexes (M49 rejection stands).
- iOS `phys_footprint` claims (still parked; Windows proxies only).

## Files And Responsibilities

- Create: `docs/reports/evidence/m55-native-match-or-beat/`
  - `thresholds/m55-thresholds.csv` (the ratchet artifact; superset of M52's),
    `phase-0-baseline/`, `phase-1-attribution/`, `phase-2-poet-storage/`,
    `phase-2b-owner-reduction/`, `phase-3-poet-graph/`,
    `phase-4-short-key-index/`, `final/`. Benchmark `-OutputRoot` always
    targets a fresh `run-N` leaf under the phase dir (the script clears its
    target).
- Modify: `scripts/benchmark-native-rime-inprocess.ps1`, as needed to:
  - (a) evaluate the new threshold rows. Startup/session and win-row ratios
    flow through the existing `latency_ratio` kind (workload/input keyed) and
    may need **no script change**. Track B absolutes **do** need one:
    `Invoke-TrackAThresholdCheck` currently receives only Track-A-filtered
    comparison rows and understands only `latency_ratio`/`memory_peak` — extend
    it to also take the combined summary rows and add kinds
    `latency_absolute_us` and `memory_absolute_bytes`. **Schema decision (do
    not improvise):** the existing threshold CSV has no `track` column; encode
    the track by prefixing the `workload` field (e.g.
    `track-b/key_sequence_process_with_context`), keeping the 8-column schema
    byte-compatible so the M52 artifact and existing rows parse unchanged;
  - (b) (Phase 2) produce Yune-side compiled artifacts in the Track A prep
    step: the Track A run root is rebuilt each invocation from the oracle's
    librime-built `rime-user/build` blobs, so the new poet artifact must be
    generated by an explicit untimed Yune deploy/compile step during prep —
    extending the Track A prep path for this is an authorized script change
    (keep the timed phases unchanged).
- Modify (Phase 1): instrumentation. Owner-profile extensions live in
  `crates/yune-core/src/memory_owner.rs` / `memory_probe.rs` (no `unsafe`).
  Any counting `#[global_allocator]` or allocator-stats hook **must not** go in
  `yune-core` (`unsafe_code = "forbid"`); put it in the benchmark harness
  (`crates/yune-rime-api/benches/`) or another unsafe-permitted crate.
- Modify (Phase 2): poet storage seam split across crates — byte-source/access
  trait + offset-served reads in `crates/yune-core/src/poet/`; artifact
  compile/mmap/install wiring in `yune-rime-api` (start from
  `crates/yune-rime-api/src/schema_install.rs` and the `compact_table.storage`
  seam).
- Modify (Phase 3): poet graph/scoring internals (allocation behavior only —
  identical candidate output). New/expanded fixtures under the
  `upstream_luna_pinyin_parity` test tree with oracle-captured bytes and
  updated `oracle-manifest.json`.
- Modify (Phase 4): the schema-compile step for prism/table to add the
  acceptance/exact-row index and the lookup path that consumes it.
- Modify on closeout only: `docs/reports/yune-vs-librime-performance.md`,
  `docs/reports/yune-vs-librime-root-cause-analysis.md`, `docs/roadmap.md`,
  `docs/requirements.md`, `docs/ledgers/milestone-history.md`; move this plan
  to `docs/plans/completed/`.

## Execution Rules For The Implementing Agent

Read before starting: `AGENTS.md` (especially Verification Discipline),
`docs/conventions.md`, and this plan end to end.

1. **Never loosen a ceiling.** If the full gate fails after your change, the
   change is wrong — fix it or revert it. Editing `m55-thresholds.csv` upward
   is forbidden (sole exception: the one-time reviewed win-row re-baseline at
   Phase 2 close, documented, still `<1.00x`). Tightening happens only at phase
   close, as its own reviewed step.
2. **Fresh deploy before every benchmark run.** Stale compiled blobs have
   produced false results repeatedly in this repo (M38 Track B ran on stale
   undeployed blobs — committed evidence; a stale-local-WASM near-miss was
   also caught in review during WEB-04 execution). Track B: use
   `-DeployProductBeforeBenchmark`. Track A: the script
   rebuilds the run root from the oracle each invocation; from Phase 2 onward
   also confirm the Yune-side compiled poet artifact in the run root postdates
   your code change before trusting any number.
3. **Never derive expected values from Yune.** Fixture expectations come from
   the external oracle (upstream librime 1.17.0 at the pinned commit) captured
   into checked-in fixtures via the capture tooling named above. A test that
   compares Yune to Yune-derived data is a defect, not a test.
4. **Product-path verification, not just the harness.** A green
   `upstream_luna_pinyin_parity` fixture proves the curated harness. For every
   phase that touches the poet path, additionally drive the deployed product
   path via
   `cargo run -p yune-cli -- frontend --shared-data-dir apps/yune-web/public/schema --user-data-dir <fresh tmp dir> --schema luna_pinyin --sequence "<input> "`
   for at least the 37-char and 59-char benchmark inputs and two fixture
   sentences, parse the **last** keypress event's candidates, and byte-compare
   before/after your change (identical required). Use a fresh user-data dir per
   run so compiled blobs are rebuilt.
5. **Record honestly.** Blocked is recorded as blocked. Every phase close
   lists the exact commands run (fmt, the broad clippy `-D warnings`, the
   named tests, the exact benchmark invocation with all switches) — no
   paraphrased "verification passed".
6. **Three runs for any claimed number.** Medians from `>=3` same-run
   benchmark executions; report min/median/max. A "win" needs the full gate
   green on two consecutive executions.
7. **LF line endings; no model/dictionary bytes committed; evidence CSVs are
   committed.**

---

## Phase 0: Full-Suite Baseline + Ratchet Artifact (tooling only)

**Owner:** measurement coverage (the unguarded dimensions) and startup/session
measurement stability. **No engine code changes in this phase.**

**Execution note:** the first Phase 0 gate exposed a pre-existing M54 regression
in the plain null-grammar Luna path. Restoring that default-off path was required
before the measurement ratchet could become green; no Phase 1+ optimization
owner has started.

- [x] Pre-flight: read the script's `param()` block; if the upstream oracle
  root assert fails, provision it via `scripts/capture-upstream-luna-pinyin.ps1`
  (see Current Starting Point). If `apps/yune-web/source/public/schema` is
  absent, record Track B as blocked and use `-SkipTrackB`.
- [x] Run the full benchmark 3 times. Canonical per-run invocation (adjust only
  if the `param()` block has drifted — record any drift in the evidence
  README):

  ```powershell
  powershell -ExecutionPolicy Bypass -File scripts/benchmark-native-rime-inprocess.ps1 `
    -TrackAInputs "n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru" `
    -DeployProductBeforeBenchmark `
    -OutputRoot docs\reports\evidence\m55-native-match-or-beat\phase-0-baseline\run-1
  ```

  Note the explicit `-TrackAInputs`: the script's default list **omits `n`**,
  and the M52 invocation omitted the three win rows — neither default covers
  the full 8-row suite. `-OutputRoot` must be the fresh `run-N` leaf (the
  script clears the directory it is given). Record the machine/toolchain
  fingerprint (OS build, CPU, `rustc -V`, release profile) in the README.
- [x] Compute per-dimension noise bands (min/median/max across the 3 runs) for:
  startup (`startup_warm_shared_assets_runtime_ready`), session
  (`session_create_select_destroy`), all 8 latency rows, Track A peak working
  set, Track A private-bytes proxy, and (if run) Track B absolutes (peak WS,
  median WS, private bytes key-sequence, key-sequence latency, startup,
  session). Commit as `phase-0-baseline/noise-band.csv`.
- [x] **Startup/session stability deliverable:** prior evidence shows the
  startup ratio swinging `0.935x -> 1.113x` across M52 runs. Attempt to harden
  (more in-run repetitions, warmup discipline, process isolation, high-priority
  scheduling). If the ratio band cannot be brought under `10%`, gate
  startup/session by **absolute-microsecond ceilings** (worst-of-3 x 1.10)
  instead of ratios, and record that Tier M's startup criterion is the noise
  band per the Win Bars wording. Do not let a noisy ratio become either a false
  gate failure or a fake win.
- [x] Author `thresholds/m55-thresholds.csv` as a **superset** of the M52
  artifact: keep the five M52 latency ceilings and the memory ceiling; add
  startup/session (ratio or absolute per the stability deliverable; workload
  ids as named, empty `input`), the three win rows at
  `min(0.95x, worst-of-3 x 1.20)`, and Track B absolutes (worst-of-3 x 1.10)
  if Track B ran.
- [x] Extend `Invoke-TrackAThresholdCheck` for the Track B absolute kinds
  (`latency_absolute_us`, `memory_absolute_bytes`, track encoded as a
  `track-b/` prefix on the `workload` field per the schema decision in Files
  And Responsibilities, fed from the combined summary rows). Startup/session
  and win-row ratios should need no script change (existing `latency_ratio`
  kind). Keep the M52 artifact working unchanged.
- [x] **Prove the gate can fail:** run once against a synthetic-breach copy of
  the artifact (one ceiling set below the observed value) and record the
  non-zero exit. A gate that has never failed is unverified tooling. Do this
  for one row of each `kind`.
- [x] Phase gate: full suite green against `m55-thresholds.csv` on two
  consecutive runs; evidence + README recorded.

**No-go:** if after the hardening attempt run-to-run noise still exceeds `10%`
on any *per-key latency* ratio (startup/session fall back to absolute bounds
instead), stop and fix the measurement before any engine work — optimizing
inside the noise band is how whack-a-mole starts.

## Phase 1: Attribute The `105.6 MB` Unclassified Floor (diagnostics only)

**Owner:** the process unclassified lower bound — the largest single number in
the lane and currently unnamed. **No engine behavior changes; instrumentation
only.**

- [x] Extend the memory-owner profiling (`crates/yune-core/src/memory_owner.rs`
  / `memory_probe.rs`) until at least `80%` of the unclassified floor is
  attributed to named owners. Allocator-level instrumentation (counting
  `#[global_allocator]`, allocator-reported live bytes vs OS working set) goes
  in the benchmark harness or another unsafe-permitted crate — **never in
  `yune-core`** (`unsafe_code = "forbid"`). Candidate buckets, in order:
  allocator retention/fragmentation, deploy/compile transients that never
  return to the OS, duplicated buffers across table/prism/poet loads,
  per-session state, mapped-file double counting.
- [x] Distinguish **steady-state residency** from **transient peak** (deploy /
  first-selection spikes). The ratchet gates peak; the attribution table must
  show both.
- [x] Commit `phase-1-attribution/owner-budget.csv`: every named owner, bytes,
  `byte_class`, reducibility verdict (`byte-backable` / `shrinkable` /
  `transient-boundable` / `irreducible-overhead`), and the technique that would
  reduce it.
- [x] Derive and commit the **evidence-based memory targets**: the projected
  post-Phase-2 peak, the Phase 2b candidate list (top reducible owners), and
  the reachable floor. Update the Win Bars section of this plan in the same
  commit (the `60 MB` Tier M bar is explicitly provisional until this step),
  with one paragraph of justification per number.
- [x] Phase gate: full ratchet still green (instrumentation must not move the
  numbers); attribution `>=80%`; budget table committed.

**No-go:** if attribution stalls below `80%` after the candidate buckets are
exhausted, close the phase partial with the measured coverage and re-derive
targets from what was named — do not proceed to Phase 2 with a fictional
target.

## Phase 2: Byte-Backed Poet Storage

**Owner:** `poet.vocabulary` (`53.6 MB`) + `poet.entries_by_code` (`18.7 MB`)
retained heap. **This is the keystone phase.**

- [x] Design first, as a short committed note in the phase evidence dir: the
  compiled artifact layout (offset-served, like `compact_table.storage`), its
  version tag, stale-artifact rejection behavior, the crate split (trait/reads
  in `yune-core`, compile/mmap/install in `yune-rime-api` via
  `schema_install.rs`), **and how the artifact gets produced in the Track A
  benchmark flow** (the untimed Yune deploy/compile prep step — see Files And
  Responsibilities (b); the Track A run root is otherwise librime-built and
  would never contain the artifact).
- [ ] Compile `poet.vocabulary` and `poet.entries_by_code` into the mmap-backed
  artifact at schema-compile/deploy time; serve lookups by offset with zero
  retained heap copies of entry payloads.
- [ ] Add a stale/corrupt-artifact rejection test (wrong version, truncated
  file) — the WEB-02 lesson: silent fallback to source-loading is how a memory
  win silently un-lands. Fallback must be loud (rebuild) and tested.
- [ ] Verify in `memory-owner-profile.csv` that the two owners moved to
  `mmap_file_backed` with heap remnants `<1 MB` combined.
- [ ] Parity: `upstream_luna_pinyin_parity`, `cantonese_parity`, and the
  product-path CLI comparison (Execution Rule 4) — byte-identical candidates.
- [ ] Full ratchet gate — pay particular attention to the 37/59-char rows and
  the three win rows (byte-backed access adds per-lookup cost; the ceilings
  bind; the abbreviation win rows run through the poet graph) and to Track B
  absolutes (the storage seam is shared).
- [ ] Phase close: tighten the Track A memory ceiling to the new
  `worst-of-2-green-runs x 1.05`. If a deliberate, still-winning latency trade
  was made on a win row, execute the one-time win-row re-baseline here as its
  own reviewed step (written justification; ceilings stay `<1.00x`).

**No-go:** if byte-backed access cannot hold the latency ceilings after
reasonable access-path work (offset tables, batched reads, small hot caches
`<1 MB`), close partial with measured evidence and stop — do not trade past
ceilings.

## Phase 2b: Reduce The Top Named Owners From Phase 1 (conditional)

**Owner:** whatever Phase 1's `owner-budget.csv` named as the largest
`byte-backable` / `shrinkable` / `transient-boundable` owners inside the former
unclassified floor. **This phase exists because Tier M's memory bar is not
reachable from the poet owners alone.**

- [ ] Take the top owners from the Phase 1 budget table (aim: enough named
  bytes to reach the Phase-1-projected peak) and apply the verdict-matched
  technique per owner (byte-back, shrink, bound the transient).
- [ ] One owner at a time; full ratchet gate + parity after each landed owner.
- [ ] Phase close: tighten the memory ceiling to `worst-of-2-green-runs x 1.05`.

**Skip condition:** if Phase 1 found the floor dominated by
`irreducible-overhead` (e.g. allocator/OS floor), skip this phase, revise the
Tier M memory bar per Phase 1's committed justification, and record the skip in
the evidence README.

## Phase 3: Poet Graph Constant Factors (37/59-char rows)

**Owner:** the sentence-lattice/scoring path — `~96%` of the 37-char row's
cost is graph work above the raw lookup (`891.0 us` translator vs `28.9 us`
lookup). librime does the equivalent in `293.2 us`; the gap is constant
factors (allocation, hashing, string handling, candidate materialization),
not algorithm class.

Pre-requisite — fixture expansion (must land before any poet code change):

- [ ] Capture from the oracle at least `10` new sentence fixtures beyond the
  current 2-syllable bigrams: 3-5 syllable words/phrases, mixed-length
  sentences, the 37-char and 59-char benchmark inputs themselves, and the
  known unpinned candidates (`shijian`/`beijing` completion-over-bareword
  ordering). Use the capture tooling:
  `scripts/capture-upstream-luna-pinyin.ps1` + `scripts/oracle-rime-probe.cs`,
  adding scenarios to the oracle root's `luna-pinyin-scenarios.json`, and
  update each fixture set's `oracle-manifest.json` so
  `crates/yune-core/tests/oracle_fixture_provenance.rs` stays green.
- [ ] Confirm all new fixtures pass on the current (pre-change) engine, or
  record any pre-existing mismatch as a named `#[ignore = "blocked: ..."]`
  with a `panic!()` body per repo convention — do not silently drop a row.

Optimization:

- [ ] Profile allocations on the 37-char row first (counts and bytes per
  keypress, not just time; the Phase 1 allocator hook in the bench harness can
  be reused). Commit the profile as evidence; pick the top owners.
- [ ] Apply constant-factor work in order of measured ownership — candidate
  arena/scratch reuse across keypresses, avoiding per-edge heap allocation,
  string interning/borrowing over cloning, lazy candidate materialization
  (page-bounded, as M50 did for translation) — **with identical output**.
- [ ] Parity + product-path CLI comparison after every landed step; full
  ratchet gate at phase close; tighten the 37/59-char ceilings to locked-in
  values.

**Win bar for this phase:** 37-char and 59-char `<=1.50x` (Tier M). **Stretch:**
`<=1.00x` (Tier S). **No-go:** if `<=1.50x` is unreachable without changing
candidate output or scoring order, close partial at the best green state and
record the measured wall.

## Phase 4: Compile-Time Short-Key Index (`n`/`ni`/`hao`)

**Owner:** the exact-row scan under charset filtering (`ni`: raw lookup
`18.0 us` vs librime's whole-row `14.3 us` — the lookup itself must get
cheaper, plus translator overhead above it).

- [ ] Design note first: what gets precomputed into the compiled prism/table
  artifact (charset-acceptance masks, exact-row offsets, quality-sorted row
  heads) such that runtime holds **zero new retained heap** (M49 rule) and the
  artifact version bumps with rejection-tested staleness. Production in the
  Track A flow uses the same untimed prep step as Phase 2.
- [ ] Implement the compiled index + lookup path consumption. Candidate
  enumeration order and output must be byte-identical (parity + product-path
  comparison).
- [ ] Verify via `memory-owner-profile.csv`: any new owner is
  `mmap_file_backed` or `<100 KB` heap.
- [ ] Full ratchet gate; tighten `n`/`ni`/`hao` ceilings at close.

**Win bar:** `n`/`ni` `<=2.00x`, `hao` `<=1.75x` (Tier M). **Stretch:**
`<=1.00x`. **No-go:** absolute gaps here are `13-39 us` (imperceptible); if the
compiled-index approach cannot reach Tier M without heap or parity risk, close
partial with evidence — do not burn the program's budget here.

## Phase 5: Closeout

- [ ] Final full benchmark, `3` runs, both tracks (or Track B recorded blocked),
  fresh deploy; full ratchet green; record which Tier (M / S / per-row mix)
  each dimension landed at, in one table.
- [ ] Rewrite the two dashboards
  (`yune-vs-librime-performance.md`, `yune-vs-librime-root-cause-analysis.md`)
  from the final evidence; move superseded rows to `history/` per the existing
  pattern.
- [ ] M53-style claim audit: every public wording (README and reports) matches
  the measured, lane-specific record — Tier M results are described as
  bounded-gap, never as "matching librime"; only rows measured `<=1.00x` may
  be called match-or-beat, lane-qualified.
- [ ] Update `docs/roadmap.md` (sequence, ledger, North Star state),
  `docs/requirements.md` (IDs below), `docs/ledgers/milestone-history.md`;
  move this plan to `docs/plans/completed/`.
- [ ] Gate handover: the M55 threshold artifact becomes the repo's standing
  regression gate (supersedes the M52 artifact; M52's file stays as history).

## Definition Of Done

M55 closes **complete** when:

- Tier M (as possibly revised by Phase 1's committed justification) holds on
  two consecutive final same-run executions (all rows, memory and latency,
  wins kept), with the full ratchet green.
- Parity suites and the product-path CLI comparisons are byte-identical.
- The ratchet artifact covers every tracked dimension and is handed over as
  the standing gate.
- Dashboards, roadmap, requirements, and history reflect exactly the measured
  final state (no claim drift; Tier M never described as "match").

M55 closes **partial** (still valuable, recorded honestly) when some phases hit
Tier M and others closed at documented no-go walls — the ratchet then locks in
whatever was won. M55 is a **no-go overall** only if Phase 0/1 shows the lane
cannot be measured stably or the memory floor is fictional.

## Proposed Requirement IDs (add to `docs/requirements.md` at closeout only)

(House style per `docs/requirements.md`: `<MILESTONE>-<TOPIC>-<NN>`.)

- **M55-PERF-01**: Every tracked native Track A dimension (startup, session, 8
  latency rows, peak memory) plus Track B absolutes (or a recorded Track B
  blocker) has a committed ceiling in a single ratchet artifact checked by
  `-FailOnRegression`; win rows are ceilinged `<1.00x`.
- **M55-PERF-02**: The `105.6 MB` unclassified floor is attributed `>=80%` to
  named owners with committed reducibility verdicts, and the Tier M memory bar
  is either substantiated or revised with committed justification.
- **M55-PERF-03**: `poet.vocabulary` and `poet.entries_by_code` are served
  from versioned, stale-rejecting, mmap-backed compiled storage with `<1 MB`
  combined heap remnant and byte-identical candidates.
- **M55-PERF-04**: Sentence parity fixtures cover 3+ syllable and
  full-sentence rows (including both benchmark sentences) with oracle-captured
  bytes and green provenance manifests.
- **M55-PERF-05**: The 37/59-char rows and short-key rows meet their Tier M
  ratios, or their no-go walls are documented with measurements.
- **M55-PERF-06**: Final dashboards and public claims match the final
  evidence, lane-qualified, with Tier M described as bounded-gap.

## Review Prompt

> Please review `docs/plans/active/m55-plan-native-track-a-match-or-beat-program.md`
> as the active M55 plan. Focus on: whether the full-suite ratchet actually
> prevents the historical whack-a-mole pattern (all dimensions ceilinged,
> including current wins and Track B absolutes, with the win-row re-baseline
> exception tightly bounded); whether the phase ordering (attribute ->
> byte-back -> owner reduction -> poet constants -> short-key index) respects
> the owner evidence; whether the Tier M/S bars are honest given the M52
> numbers and the startup noise history; whether parity protection (fixture
> expansion before poet changes, product-path CLI comparison over
> `apps/yune-web/public/schema`) is sufficient for the oracle-sensitive paths;
> and whether the plan remains self-contained for a weaker executor (exact
> benchmark invocation with `-TrackAInputs`/`-OutputRoot`, oracle
> provisioning, Track B machine-local prerequisite, capture tooling, crate
> boundaries for unsafe/mmap).
