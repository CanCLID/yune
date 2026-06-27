# M46 Jyutping Native And WASM Memory Attribution Implementation Plan

> **Status:** Complete with measured no-go - **Milestone:** M46 (Track B native and yune-web
> Jyutping WASM memory attribution) - **Updated:** 2026-06-27 - **Type:**
> engine/runtime memory attribution and gated optimization plan
>
> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Explain and, only if evidence authorizes it, reduce the TypeDuck
`jyut6ping3` / `jyut6ping3_mobile` memory owners that remain after M45 and
WEB-01: native Track B peaks around `504 MB`, steady resident rows around
`427-441 MB`, browser Jyutping WASM high-water at `893.1 MiB`, and the
schema-switch row that can grow to about `1.9 GiB` and return no Jyutping
candidates. M46 is the WEB-01 handoff; do not create another WEB-01-style
harness plan unless M46 Phase 0 reclassifies the owner as harness-only.

**Closeout verdict:** `schema-switch-correctness-fixed-memory-unchanged`.
Branch A fixed the product-affecting Cangjie -> Luna -> Jyutping no-candidate
bug. Native Track B still peaks at `504,627,200 B`, browser Jyutping still
reaches `893.1 MiB`, and named concrete native owners explain only about
`59.7 MB`, so the remaining memory blocker is
`measured-no-go-owner-unclassified`.

**Architecture:** M46 is an attribution-first bridge between the native Track B
product path and the yune-web WASM runtime. It starts by reconciling owner
bytes across native `memory-owner-profile.csv`, product path status, browser
WASM linear memory, schema asset families, and schema-switch behavior. It does
not assume that adding a Jyutping `rsmarisa` string table will save the
headline memory. The existing evidence accounts for only small named owners:
`compact_table.syllabary_codes` is about `4.2 MB`, and the `8.3 MB`
`translator.entries_by_code` row is guarded/source-YAML or small-test state,
not retained by the selected compact Track B product path. Any implementation
branch must follow a Phase 0 verdict. The most likely deeper owners to prove
or disprove are candidate text/comment payloads plus the `jyut6ping3_scolar`
second-dictionary footprint; a marisa string table interns codes, not candidate
payloads. Phase 0 must also explain the native-vs-WASM gap because a native
memory win may not automatically transfer to browser WASM linear memory.

**Tech Stack:** Rust engine and ABI benchmark code in `crates/yune-core/` and
`crates/yune-rime-api/`; native benchmark script
`scripts/benchmark-native-rime-inprocess.ps1`; browser harness and Playwright
benchmarks in `apps/yune-web/`; reports under `docs/reports/`; requirements,
roadmap, and decisions under `docs/`.

---

## Scope

M46 may change:

- Native owner instrumentation in `crates/yune-core/src/dictionary/`,
  `crates/yune-core/src/translator/`, and `crates/yune-rime-api/`.
- Native benchmark output and metric-export schema, including
  `crates/yune-rime-api/src/lib.rs` `M37_METRIC_FIELDS`.
- TypeDuck/Jyutping compiled-table experiments when Phase 0 authorizes them.
- Browser/WASM attribution harnesses under `apps/yune-web/e2e/`.
- yune-web worker/runtime cleanup only for the schema-switch `~1.9 GiB` /
  no-candidate row or for measured WASM owner release.
- Reports, visualizations, roadmap, requirements, decisions, and this plan.

WEB-01 remains closed. M46 may use browser harness evidence and may fix a
schema-switch correctness bug, but it is not a new harness-payload or
`INITIAL_MEMORY` calibration plan.

M46 must not claim:

- Track A `luna_pinyin` speed or memory wins.
- Browser-harness-only payload wins unless browser evidence shows actual
  ready, peak, and steady WASM movement.
- Public-demo, packaging, deployment, product-delivery, AI, learned
  `.gram`/octagram, plugin ABI, or broad frontend speed wins.
- A `~200 MB` Jyutping `rsmarisa` saving before owner evidence proves that
  magnitude.

## Baseline Evidence To Preserve

Start from these already-published facts:

- M45 native Track B product evidence reports `source_fallback=false`, selected
  storage `byte_backed`, table/prism mapping `mmap`, selected heap mirrors `0`,
  and `rsmarisa_status=missing_string_table`.
- Native Track B final M45 memory evidence records startup/session/long-row
  peaks around `504,639,488 B`, with steady resident rows around
  `427,003,904-440,885,248 B`. Unlike Track A's larger peak-vs-steady split,
  this is a real resident-footprint problem, not a transient to dismiss.
- Native Track B owner evidence shows selected compact storage does not retain
  a translator `BTreeMap`; the `8,327,700 B` `translator.entries_by_code` row
  is guarded/source-YAML or small-test state.
- `compact_table.syllabary_codes` retains about `4,189,674 B` for Track B.
- Product path status reports `byte_source_len=15,248,382` for `jyut6ping3`
  and `byte_source_len=27,325,622` for `jyut6ping3_scolar`. Treat these as
  selected source/status byte lengths, not retained-owner rows, unless
  `memory-owner-profile.csv` confirms the same owner class. Any claim of a
  `27.3 MB` mapped retained region must cite the exact evidence row or be
  removed.
- WEB-01 final browser evidence reports Luna at `160.0 MiB`, Jyutping at
  `893.1 MiB`, My RIME Jyutping at `68.0 MiB`, and no reduction from
  `INITIAL_MEMORY=64 MiB`.
- WEB-01 asset-family attribution leaves Jyutping at `893.1 MiB` for
  `extras`, `jyutping-core`, and `full-jyutping`.
- WEB-01 surfaced but did not classify the Cangjie -> Luna -> Jyutping row:
  about `1.9 GiB` WASM high-water and no Jyutping candidates.
- Native and browser memory are related but not interchangeable: native Track B
  is around `504 MB` peak / `427-441 MB` steady, while browser Jyutping is
  `893.1 MiB`. A native fix must prove whether and how it transfers to WASM,
  where mmap/file-backed bytes become linear-heap pressure.

## Phase 0 - Attribution Before Optimization

- [x] Create the M46 evidence roots:
  `docs/reports/evidence/m46-jyutping-native-wasm-memory-attribution/` and
  `apps/yune-web/e2e/results/yune-web-jyutping-memory-attribution/`.
- [x] Record repository provenance: current commit, branch, worktree status,
  M45 commit, WEB-01 commit, benchmark host, browser version, Node/npm
  versions, Rust version, and whether any native/browser benchmark is running.
- [x] Serialize benchmark runs. Do not run native and browser memory benchmarks
  concurrently on the same machine.
- [x] Run a fresh native Track B baseline with the existing benchmark path:

  ```powershell
  cargo build --release -p yune-rime-api
  powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot C:\Users\laubonghaudoi\Documents\GitHub\yune\docs\reports\evidence\m46-jyutping-native-wasm-memory-attribution\phase-0-native -Iterations 9 -SessionIterations 60 -KeyIterations 80 -TrackBInputs h,ha,hai,hau,nei,ngo,neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung -DeployProductBeforeBenchmark
  ```

- [x] Add every new exported native metric to `M37_METRIC_FIELDS` before
  trusting CSV output. Phase 0 added owner rows in `memory-owner-profile.csv`,
  not new `m37_metrics.csv` columns.
- [x] Add or verify native owner fields for:
  - compact code strings and code-id maps;
  - candidate text and comment payload storage;
  - lookup records and dictionary lookup side payloads;
  - reverse-lookup indexes and side dictionaries;
  - base `jyut6ping3` versus `jyut6ping3_scolar` duplicated structures;
  - table, prism, reverse, OpenCC, and schema/config retained bytes;
  - source YAML or deploy transient allocation;
  - allocator high-water or closest Windows-supported retained proxy;
  - unclassified process memory after known owners are subtracted.
- [x] Split native owner rows into `heap_owned_reducible`,
  `heap_owned_required`, `heap_owned_guarded`, `mmap_file_backed`,
  `shared_or_overlapping`, `transient`, and `unclassified` classes. A row may
  not be used as a win target unless it is non-overlapping and tied to measured
  RSS/private/working-set movement.
- [x] Write an owner lower-bound table that reconciles named native owners
  against the `504 MB` native peak and `427-441 MB` steady resident rows. If
  named owners explain only single-digit or low-double-digit megabytes, Phase 0
  must say so and avoid authorizing a narrow code-string rewrite as the main
  memory fix.
- [x] Add browser/WASM attribution rows for single-schema Luna, single-schema
  Jyutping, Jyutping without scolar where possible, Jyutping core-only assets,
  full Jyutping assets, and schema-switch sequences.
- [x] Write `schema-switch-correctness.md` independent of the memory verdict.
  It must reproduce or classify the Cangjie -> Luna -> Jyutping
  no-Jyutping-candidates row against:
  - the current runtime;
  - the pre-WEB-01 executable baseline when practical;
  - a clean fresh page with only Jyutping selected.
  It must state whether this is pre-existing or introduced after WEB-01, whether
  it affects the real single-schema product flow or only the
  Cangjie -> Luna -> Jyutping test path, the likely owning layer, and severity.
  If it is a product-affecting regression, Branch A is mandatory before M46
  may close, even if the memory verdict selects a different owner.
- [x] Write `native-vs-wasm-gap.md` explaining why browser Jyutping is
  `893.1 MiB` when native Track B is about `504 MB` peak / `427-441 MB`
  steady. It must identify WASM-specific owners, browser-only retained buffers,
  copied compiled bytes, linear-heap growth behavior, and which native owners
  are expected to move or not move in WASM.
- [x] Write `phase-0-verdict.md` with a memory-action verdict. It must link to
  `schema-switch-correctness.md` as a separate correctness verdict and select
  one of these memory action families:
  - `schema-switch-regression-fix-first`;
  - `candidate-payload-owner-authorized`;
  - `rsmarisa-track-b-spike-authorized`;
  - `scolar-defer-or-lazy-load-authorized`;
  - `reverse-index-owner-authorized`;
  - `transient-deploy-peak-owner-authorized`;
  - `measured-no-go-owner-unclassified`.

No optimization branch may start until this verdict file exists.

## Branch A - Schema-Switch Regression Fix

Run this branch first if Phase 0 selects `schema-switch-regression-fix-first`
or if `schema-switch-correctness.md` classifies the no-candidates row as a
product-affecting regression. The no-candidates bug is a correctness issue even
when the `1.9 GiB` high-water has a separate memory owner.

- [x] Identify whether the failure is stale worker state, schema unload/reload,
  MEMFS/IDBFS asset lifetime, reverse-lookup side state, or engine session
  state retained across schema selection.
- [x] Fix the smallest owning layer. Candidate files include
  `apps/yune-web/src/worker.ts`, `apps/yune-web/src/rime.ts`,
  `apps/yune-web/src/yune-integration/`, and only if required,
  `crates/yune-rime-api/` session/schema lifecycle code.
- [x] Preserve normal local and public-demo typing for `ngogokdak`, `ngo`,
  `nei`, `h`, `ha`, `hai`, `hau`, and `luna_pinyin` rows.
- [x] Gate with focused browser evidence for Cangjie -> Luna -> Jyutping,
  Jyutping -> Luna -> Jyutping, reverse lookup, userdb persistence, Shift ASCII,
  and candidate commit.
- [x] Do not count this branch as a memory optimization unless WASM ready,
  peak, and steady high-water also move.

## Branch B - Candidate Payload Or Interning Reduction

Run this branch only if Phase 0 proves candidate text/comment or lookup payload
duplication is a large retained owner.

- [ ] Introduce an owned interning or borrowed-storage design at the compact
  table boundary instead of cloning strings into per-candidate structures.
- [ ] Preserve TypeDuck v1.1.2 rich comment bytes, dictionary lookup records,
  display-language ordering, reverse lookup joiners, correction/tolerance,
  partial selection, default-confirm recomposition, long composition, and
  userdb behavior.
- [ ] Add focused tests for candidate text/comment identity and lifetime safety
  through `yune-core`, `yune-rime-api`, and browser adapter export.
- [ ] Prove native RSS/private movement and browser WASM movement separately.
  Structural owner movement alone is not a success.

## Branch C - Fresh Track B `rsmarisa` Spike

Run this branch only if Phase 0 selects `rsmarisa-track-b-spike-authorized`.
This is a spike with a parity gate, not the default solution.

- [ ] Reconcile the M36 no-go before implementation. The rejected shipped
  product `rsmarisa` blobs were stale or unsupported as a table/reverse/prism
  set; M46 may only generate fresh compatible artifacts and must prove table,
  reverse, and prism semantics together. Read D-33 in `docs/decisions.md`,
  the M36 completed plan, and `docs/reports/evidence/m36-product-path/` before
  writing code.
- [ ] Add table-writer support for a fresh Jyutping marisa string table only
  when generated compiled assets preserve lookup behavior for both
  `jyut6ping3` and `jyut6ping3_scolar`.
- [ ] Ensure product path status reports `rsmarisa_status=ok` only when the
  selected deployed profile actually uses the fresh string table and remains
  `source_fallback=false`.
- [ ] Prove whether `syllabary_codes`, `syllable_ids_by_code`, and related code
  lookup structures move meaningful measured memory. Expected savings must be
  calculated from owner evidence, not copied from the hypothesis.
- [ ] Stop this branch as `rsmarisa-measured-no-go` if it only moves
  single-digit or low-double-digit megabytes and leaves the `504 MB` native or
  `893.1 MiB` WASM headline unchanged.

## Branch D - Scolar Or Reverse Index Deferral

Run this branch only if Phase 0 proves `jyut6ping3_scolar`, reverse lookup, or
side dictionaries dominate retained or transient memory.

- [ ] Separate base Jyutping typing requirements from scolar, reverse lookup,
  OpenCC, and side dictionary requirements.
- [ ] Lazy-load or release only the owner that Phase 0 names. Do not remove
  assets needed for TypeDuck v1.1.2 candidate comments or browser dictionary
  panels.
- [ ] Gate base typing, scolar-specific candidates, reverse lookup, dictionary
  panel lookup records, schema switching, and userdb persistence.
- [ ] If safe deferral is not possible because the current UI or engine path
  needs the owner eagerly, close as `required-owner-no-go` and document it.

## Success And No-Go Gates

M46 may close as success only when all selected target families pass:

- Native Track B memory owner is reduced with measured RSS/private/working-set
  movement and no source fallback.
- Browser Jyutping WASM ready, peak, and steady high-water move in the same
  direction as the owner reduction.
- The schema-switch `~1.9 GiB` / no-candidate row is fixed or classified as a
  separate memory blocker with evidence, and the no-candidates correctness half
  has a named owner and is fixed if it affects product flows.
- The native-vs-WASM gap is explained. The closeout must state which native
  owners transfer to WASM, which owners are WASM-specific, and whether browser
  high-water movement was expected from the selected native branch.
- TypeDuck v1.1.2 behavior gates pass for rich comments, lookup records,
  correction/tolerance, partial selection, default-confirm recomposition, long
  composition, and userdb. `cargo test -p yune-core --test cantonese_parity`
  is a hard gate for any storage, payload, candidate, lookup, correction, or
  TypeDuck profile change.
- Track B short rows `h`, `ha`, `hai`, `hau`, `nei`, and `ngo` remain at least
  as good as the M45 post-M44 baseline, and the 50+ guard remains stable.
- Track A startup, session, `hao`, `n`, `ni`, `zhongguo`, M40 long rows,
  M42/M44 abbreviation rows, bounded first-page output, `RimeGetContext`,
  selected storage, mmap/byte-backed storage, heap mirrors, and
  `source_fallback=false` guards do not regress.

M46 must close as partial or measured no-go if:

- Phase 0 cannot reconcile enough owner bytes to explain the headline memory.
- The largest measured owners are required for current TypeDuck behavior.
- A structural owner moves but native RSS/private/working-set and browser WASM
  high-water do not move.
- A fresh Track B `rsmarisa` string table preserves behavior but saves only a
  small owner that does not affect the headline memory.
- Browser schema-switch correctness remains broken.
- The native owner moves but `native-vs-wasm-gap.md` proves the selected branch
  cannot move the browser number; that is a native partial result, not a
  browser memory success.

M46 may close as a useful partial result if it fixes or owns the schema-switch
correctness issue, lands a bounded `jyut6ping3_scolar` or other measured
resident-memory reduction, and names candidate payload interning or another
larger owner as a follow-up. It must not be judged a full memory success unless
the native and browser headline metrics move.

## Final Evidence And Closeout

- [x] Update
  `docs/reports/yune-vs-librime-root-cause-analysis.md` with the M46 native
  owner verdict, including before/after owner counters and any remaining
  unclassified memory.
- [x] Update `docs/reports/yune-vs-librime-performance.md` only for native
  Track B evidence actually measured by M46; do not imply Track A parity wins.
- [x] Update
  `docs/reports/yune-web-vs-my-rime-browser-baseline.md` and visuals for
  browser WASM evidence.
- [x] Update `docs/roadmap.md`, `docs/requirements.md`,
  `docs/decisions.md`, and `docs/ledgers/milestone-history.md`.
- [x] Move this plan to `docs/plans/completed/` only after the closeout verdict
  and evidence bundle exist.
- [x] Record final gates appropriate to the touched implementation. If Rust
  code changes:

  ```powershell
  cargo fmt --check
  cargo test -p yune-core --test cantonese_parity
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
  ```

- [x] If browser code changes:

  ```powershell
  npm.cmd --prefix apps/yune-web run typecheck
  npm.cmd --prefix apps/yune-web run build
  npm.cmd --prefix apps/yune-web run build:public
  ```

- [x] Always run:

  ```powershell
  git diff --check
  ```
