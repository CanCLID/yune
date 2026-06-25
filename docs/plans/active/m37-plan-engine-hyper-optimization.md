# M37 Engine Hyper-Optimization Plan

> **Status:** Planned - **Milestone:** M37 (engine hyper-optimization) - **Created:** 2026-06-24 - **Type:** engine-performance plan
>
> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the post-M36 engine path meaningfully closer to librime by landing byte-backed or interned product storage, fixing candidate materialization/context export for the TypeDuck product path, and treating `hai` as the first residual row to explain and move. `rsmarisa` is the strongly preferred storage route, not the milestone's only acceptable implementation mechanism.

**Architecture:** M37 is not another narrow storage no-go milestone. It has four hard closeout gates: (1) the product table path must use byte-backed or interned storage with no `SourceFallback`, no M36-style owned no-marisa fallback, no parallel heap mirror, and measured memory movement; (2) the native product path must prove mmap/file-backed loading for the selected hot storage bytes, with `mapping_mode=mmap` when the selected route uses `rsmarisa`; (3) the candidate materialization/context path must become page-bounded for the measured TypeDuck product rows instead of eagerly materializing and cloning the whole candidate list; and (4) `hai` must be explained and materially moved. The implementation remains oracle-driven and ABI-stable: default `RimeApi`/`RimeCandidate` layouts do not change, TypeDuck profile behavior stays byte-identical, and full-list readers keep an explicit eager expansion path when they truly need one.

**Tech Stack:** Rust (`yune-core`, `yune-rime-api`), `rsmarisa` where it works, native mmap/file-backed loading, byte-backed/interned re-emitted storage where it is the safer outcome, `StaticTableTranslator`, `TableLookup`, `CompactTableStore`, `Engine`, `CandidateRequest`/`TranslationResult`, `RimeGetContext`, `schema_install`, `table_writer`, `compiled_table`, `compiled_prism`, `native_inprocess_benchmark`, working-set and heap attribution evidence, upstream `luna_pinyin` fixtures, TypeDuck `jyut6ping3` fixtures, and TypeDuck-Web runtime/browser gates only when runtime-visible files change.

---

## Why This Exists

M33-M36 removed real costs, but the M36 diagnosis still leaves two unacceptable gaps for a hyper-optimized engine:

- **The product storage outcome is still not librime-shaped.** M36 proved that the shipped TypeDuck product blobs use a marisa string table, then closed `rsmarisa` by no-go for that milestone and shipped a no-marisa Yune-readable re-emitted fallback. That was a useful product unblock, but it is not the final data path. M37 should make `rsmarisa` work on real TypeDuck product table data first; if the external crate or its platform story blocks safe native/WASM use, M37 must still land the same storage outcome through a reviewed byte-backed or interned re-emit path. This does not assume storage is the top `hai` latency owner.
- **Native mmap is still not enforced as a closeout result.** A byte-backed table that is immediately copied into owned heap state would repeat the M35/M36 memory pattern. M37 must make native product storage use mmap/file-backed bytes for the hot query path, or keep working on a patch, fork, owner-backed adapter, or native mapped fallback until it does.
- **Candidate materialization is still eager for the product.** The final M36 `hai` row is the clearest clue: it is the shortest product input, barely improved (`-29.2%`), and remains about `3x` slower than the other final TypeDuck rows. That points to completion/homophone explosion, full-list sort/filter/userdb merge, owned candidate construction, context snapshot clone, and ABI C-string export, not long-sentence DP.
- **Memory is still too weakly explained.** M36 moved product peak memory from about `1000.4 MB` to `885.3 MB`, but it did not produce the owner-level memory table needed to tell whether table storage, candidates/context, userdb, reverse lookup, ABI buffers, or allocator high-water is now dominant. M37 cannot close by moving latency while leaving the memory gap unattributed.

M37 therefore turns the M36 follow-up strategy into a closeable implementation milestone. It starts with attribution, but attribution alone cannot close it. The milestone remains open until byte-backed or interned product storage is active, native mmap/file-backed loading is proven, product memory moves with owner evidence, and the measured product key rows prove page-bounded materialization.

Sequencing rule: the hard gates are not a ranking. Phase 0 decides the
implementation order. If `hai` is dominated by context export or candidate
materialization, execute Task 2/Task 3 before broad storage work while keeping
Task 1 open as a non-waivable storage/data-path closeout gate. If memory
attribution names storage or candidate/context ownership as the largest product
memory owner, that owner must be fixed before M37 closes.

## Scope

In scope:

- Fresh M37 baseline and per-owner attribution for Track A (`luna_pinyin` Yune vs librime) and Track B (`jyut6ping3_mobile` Yune before/after), starting with Track B `hai`.
- A byte-backed or interned product table path for TypeDuck `jyut6ping3` and `jyut6ping3_scolar`, with `rsmarisa` as the first route to try.
- Reading real marisa string-table payloads from actual TypeDuck product `.table.bin` files.
- Emitting or rebuilding fresh product table artifacts that use `rsmarisa` for the string table where safe, or a reviewed interned/byte-backed Yune product table where `rsmarisa` cannot safely satisfy native/WASM/runtime ownership constraints. Either path must still carry every Yune/TypeDuck payload needed for rich comments, lookup records, correction/tolerance, and source checksum freshness.
- Native mmap/file-backed loading for the selected hot product storage path. For the preferred `rsmarisa` route this means `Trie::mmap()` or a reviewed owner-backed mmap adapter reporting `mapping_mode=mmap`; for an interned/byte-backed fallback this means the native query path borrows from mapped deployed bytes instead of rebuilding an owned heap table. Browser/WASM uses `Trie::map()` or the safest byte-backed equivalent because OS mmap is not available there.
- Removing full-list candidate materialization from page-only product reads: translator output, engine merge/sort/filter/userdb/ranker work, context snapshot, and ABI export must be bounded to the visible page plus measured surplus where semantics allow it.
- Owner-level memory attribution and measured memory movement, not just latency movement.
- Eager fallback for behaviors that truly require full-list semantics, but not for the default measured product `hai`, `ngohaig`, `loengjathau`, and `jigaajiusihaa` rows.
- Conservative report updates that keep Track A, Track B, native, browser, latency, and memory claims separate.

Out of scope:

- Widening default `RimeApi`, `RimeCandidate`, or TypeDuck profile ABI structs.
- Changing candidate order, rich comment bytes, learning behavior, partial selection, default-confirm recomposition, or TypeDuck profile options to make performance easier.
- Replacing Yune's deterministic engine with librime's C++ component model.
- Treating `typeduck.hk/web`, `LibreService/my_rime`, or stock librime browser delivery as a behavior oracle.
- M31 public-demo Cloudflare/PWA/cache/UI work, except for running browser gates if M37 changes runtime-visible WASM behavior.
- M32 AI product UX and P2-WIN-01 TSF/frontend-shell work.

## Non-Negotiable Closeout Gates

- `M37-ENGINE-01` (latency and memory attribution): The Phase 0 evidence must split Track B `hai` across process-key, translator lookup, completion enumeration, candidate materialization, global sort/top-K, filters, userdb merge, context snapshot, ABI allocation/free, and working-set owners. It must also produce a product memory-owner table covering table bytes, parsed storage, candidates/context, userdb, reverse lookup, ABI buffers, and allocator high-water. If `hai` or product memory is not explained, implementation continues.
- `M37-ENGINE-02` (byte-backed/interned storage hard gate): The final product storage path must be byte-backed, mapped, or interned enough to remove the M36-style owned no-marisa heap mirror and move memory. `rsmarisa` is the preferred implementation and must be tried against actual `jyut6ping3` and `jyut6ping3_scolar` table data; if a verified crate/platform blocker prevents safe use, a reviewed interned/byte-backed re-emit or owner-backed adapter can satisfy the gate only if it meets the same no-`SourceFallback`, no-heap-mirror, byte-parity, and memory-movement evidence.
- `M37-ENGINE-03` (fresh product compiled path): The final product status must show fresh table/prism/reverse artifacts, no `SourceFallback`, and a table parse/status that proves the selected byte-backed or interned product path is active rather than silently using the M36 no-marisa owned fallback.
- `M37-ENGINE-04` (candidate materialization hard gate): For the default Track B product rows, instrumentation must prove Yune materializes only the current page plus bounded surplus during ordinary `RimeProcessKey` + `RimeGetContext` reads. Full-list materialization is allowed only when an explicit full-list API, paging beyond the retained window, debug inspection, or a proven full-list-only feature asks for it.
- `M37-ENGINE-05` (context export): `RimeGetContext` must no longer require `Engine::snapshot()` to clone the full candidate list for page-only reads. It should read a page snapshot or page view and allocate C strings only for the exported page.
- `M37-ENGINE-06` (behavior): `upstream_luna_pinyin_parity`, `cantonese_parity`, `typeduck_web`, M28 long-composition/default-confirm coverage, paging, deletion, numbered selection, click selection, userdb learning, correction, prediction, and rich dictionary comments remain byte-identical for their target fixtures.
- `M37-ENGINE-07` (measured product movement): Track B `hai` must move materially from the M36 final `15,241.000 us` median and may not remain the unexplained worst row by about `3x`. If the first materialization fix does not move it, continue with the next measured owner before closing.
- `M37-ENGINE-08` (memory movement): Track B product median working set and peak working set must move materially from the M36 final `~741.5 MB` median row / `885.3 MB` peak baseline, and Track A working-set attribution must be refreshed. If memory does not move, M37 stays open to the named memory owner unless the maintainer explicitly splits a follow-up milestone before closeout.
- `M37-ENGINE-09` (native mmap hard gate): The final native product path must report mmap/file-backed loading for the selected hot storage bytes. If `rsmarisa` is selected, native evidence must report `mapping_mode=mmap` for the marisa string-table payload. If direct `Trie::mmap()` cannot safely own the required file slice or lifetime, M37 lands a reviewed local patch, fork, or owner-backed mmap adapter before closeout; an owned-buffer, no-marisa, or mmap-no-go result keeps M37 open.
- `M37-ENGINE-10` (honest public claims): Native wins are not browser wins without rebuilt release WASM and real browser evidence. Track A ratios remain comparison caveats unless they independently improve.
- `M37-ENGINE-11` (quality gates): `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, focused upstream and TypeDuck parity tests, `cargo test --workspace`, final native benchmarks, report/docs checks, and `git diff --check` pass. Runtime/browser/patch gates run when runtime-visible files change.

## File Responsibilities

- `Cargo.toml`, `Cargo.lock`, and crate manifests: own the `rsmarisa` dependency choice, version pin, feature flags, license note, MSRV check, any local patch/fork override, and any fallback storage crate choices.
- `crates/yune-core/src/dictionary/compiled_table.rs`: owns marisa string-table detection, `rsmarisa` parsing where selected, interned/byte-backed string-id resolution, compact entry construction, structured parse errors, and status labels.
- `crates/yune-core/src/dictionary/table_writer.rs`: owns writing fresh product tables, including the final rsmarisa-backed or interned/byte-backed table form.
- `crates/yune-core/src/dictionary/query_table.rs`: owns lookup candidate views and any new page-oriented or string-id-backed candidate view contracts.
- `crates/yune-core/src/translator/mod.rs`: owns `StaticTableTranslator` storage selection, bounded/eager decision logic, `TableStorage` iterator shape, rich comment formatting, correction/tolerance lookup, prefix fallback, sentence/completion interplay, and materialization.
- `crates/yune-core/src/engine.rs`: owns refresh, bounded request selection, sort/top-K, filter/ranker/userdb/AI merge behavior, candidate window state, full-list expansion, selection, commit, and learning.
- `crates/yune-core/src/state.rs`: owns public engine candidate/context/snapshot structs. Any new page snapshot type must remain internal or behavior-compatible.
- `crates/yune-rime-api/src/context_api.rs`: owns `RimeGetContext` page export and C-string allocation/free behavior.
- `crates/yune-rime-api/src/schema_install.rs`: owns deployed artifact selection, product path activation, runtime fallback prevention, and profile guardrails.
- `crates/yune-rime-api/benches/native_inprocess_benchmark.rs`: owns Track A/Track B benchmark rows, product path status, mmap/mapping-mode status, memory rows, and new per-owner instrumentation output.
- `docs/reports/evidence/m37-engine-hyper-optimization/`: owns all M37 evidence.
- `docs/reports/yune-vs-librime-performance.md`, `docs/reports/yune-vs-librime-root-cause-analysis.md`, `docs/roadmap.md`, and `docs/requirements.md`: own public claims and closeout status.

---

## Task 0 - Baseline, Owner Spans, And `hai` Attribution

**Files:**

- Read: `docs/reports/yune-vs-librime-performance.md`
- Read: `docs/reports/yune-vs-librime-root-cause-analysis.md`
- Read: `docs/reports/evidence/m36-product-path/phase-4-final/`
- Modify: `crates/yune-rime-api/benches/native_inprocess_benchmark.rs`
- Create: `docs/reports/evidence/m37-engine-hyper-optimization/phase-0-baseline/`
- Create: `docs/reports/evidence/m37-engine-hyper-optimization/phase-0-baseline/hai-attribution.md`
- Create: `docs/reports/evidence/m37-engine-hyper-optimization/phase-0-baseline/memory-attribution.md`

- [ ] **Step 0.1: Confirm repo state**

Run:

```powershell
git fetch origin --prune
git status --short --branch
git log --oneline -5 --decorate
```

Expected:

- Worktree is clean or unrelated active changes are listed before editing.
- The current branch includes the M36 closeout and the post-M36 performance diagnosis commit.

- [ ] **Step 0.2: Capture fresh M37 baseline**

Run the M36 native in-process benchmark with the same Track A/Track B split:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\m37-engine-hyper-optimization\phase-0-baseline -Iterations 5 -SessionIterations 20 -KeyIterations 20 -DeployProductBeforeBenchmark
```

Expected:

- Baseline CSVs include Track A `ni`, `hao`, `zhongguo`, startup, session, and Track B `hai`, `ngohaig`, `loengjathau`, `jigaajiusihaa`.
- Any drift from M36 final is recorded before optimizing.

- [ ] **Step 0.3: Add owner spans for the key path**

Instrument the native harness or an internal feature-gated timing path so each product key sample can report:

- per-key processing excluding context read
- translator lookup
- completion/prefix enumeration
- candidate view to owned-candidate materialization
- sort or top-K
- filter pipeline
- userdb predictive merge
- ranker/AI merge if active
- context snapshot/page snapshot
- ABI C-string allocation and `free_context`

Expected:

- `hai-attribution.md` names the top owner for `hai`.
- If `hai` is dominated by context export rather than lookup, Task 2 is prioritized before broader materialization/storage work.

- [ ] **Step 0.4: Add materialization counters**

Record, per key row:

- dictionary candidates enumerated
- `LookupCandidate` views visited
- owned `Candidate` values constructed
- candidates sorted or considered by top-K
- candidates stored in engine context
- candidates cloned by snapshot/page snapshot
- C ABI `RimeCandidate` values exported

Expected:

- The baseline proves whether `hai` creates a large completion/homophone set even though it is only three keys.
- The final M37 run can prove materialization reduction with counts, not only latency.

- [ ] **Step 0.5: Add memory attribution**

Produce an owner table for the M37 baseline. Use the native harness working-set
and peak rows plus a heap profiler where available. If a profiler such as
`dhat`, massif, or an equivalent platform tool cannot run in the current
environment, record the blocker and use the best available owner-level fallback
instead of skipping memory attribution.

The owner table must separate:

- mapped/file bytes
- parsed table/prism/reverse storage
- candidate/context storage
- userdb and learning state
- sentence model and reverse lookup state
- ABI buffers and C-string allocations
- allocator high-water and fragmentation

Expected:

- `memory-attribution.md` names the top Track B product memory owners and the
  Track A working-set owners that keep Yune `12-14x` above librime.
- Task 1 and Task 2/3 ordering accounts for both `hai` latency and memory owner
  ranking.

## Task 1 - Land Byte-Backed Or Interned Product Storage

Task numbering is not a command to optimize storage before the measured top
owner. After Task 0, run this task before or after Task 2/Task 3 according to
the `hai` attribution and memory attribution. It remains a hard M37 closeout
gate either way. `rsmarisa` is the first route to try; the storage closeout
condition is the storage outcome, not the external crate name. Native mmap or
file-backed loading is a separate hard gate for the selected native route.

**Files:**

- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/yune-core/Cargo.toml`
- Modify: `crates/yune-core/src/dictionary/mod.rs`
- Modify: `crates/yune-core/src/dictionary/compiled_table.rs`
- Modify: `crates/yune-core/src/dictionary/table_writer.rs`
- Modify: `crates/yune-rime-api/src/schema_install.rs`
- Modify: `crates/yune-rime-api/benches/native_inprocess_benchmark.rs`
- Test: `crates/yune-core/src/tests/facade_tests/compiled_payloads.rs`
- Test: `crates/yune-core/src/tests/dictionary.rs`
- Create: `docs/reports/evidence/m37-engine-hyper-optimization/rsmarisa-path.md`
- Create: `docs/reports/evidence/m37-engine-hyper-optimization/storage-path.md`

- [ ] **Step 1.1: Pin and verify `rsmarisa`**

Check the current `rsmarisa` crate, then pin a version that satisfies the repo's MSRV, license, Windows, Linux, and WASM requirements. The currently documented API includes `Trie::mmap`, `Trie::map`, `Trie::reverse_lookup`, `Trie::predictive_search`, `Trie::num_tries`, and `Trie::num_keys`; verify those names at execution time before coding. Treat native mmap support as a required capability, not a nice-to-have benchmark variant.

Expected:

- `rsmarisa-path.md` records crate version, license, MSRV result, feature flags, Windows/native result, WASM result, real product blob result, and whether `rsmarisa` can safely own or borrow runtime-loaded bytes.
- If upstream `rsmarisa` has a crate, lifetime, nested/multi-trie, or WASM blocker, M37 records the exact blocker and continues with either a reviewed local patch/fork, a small owner-backed adapter, or the interned/byte-backed re-emit path below. Do not close M37 by declaring `rsmarisa` no-go.

- [ ] **Step 1.2: Add a marisa string-table adapter**

Implement a narrow adapter that can:

- locate the existing `string_table_offset` and `string_table_size` fields in a real `.table.bin`
- mmap only that marisa payload on native, without reading the whole table into an owned buffer
- map or load the same payload on WASM/browser through the safest supported byte-backed API
- reverse-lookup a string id into UTF-8 text through `rsmarisa`
- report `num_tries`, `num_keys`, tail mode, node order, and mapping mode for evidence; native final evidence must say `mapping_mode=mmap`
- keep byte ownership safe on native and WASM

Expected:

- Real `jyut6ping3.table.bin` and `jyut6ping3_scolar.table.bin` tests can recover sampled strings by id through native mmap mode.
- If `Trie::mmap()` cannot map an interior payload slice directly, the implementation adds a safe owner-backed mmap adapter, local patch, or fork rather than copying the payload into a normal owned buffer or falling back to no-marisa.
- If `Trie::map(&'static [u8])` is too restrictive for runtime-loaded WASM bytes, the implementation adds a safe owner-backed solution rather than leaking per-load buffers or falling back to no-marisa.

- [ ] **Step 1.3: Teach `compiled_table.rs` to parse marisa-backed table entries**

Replace the hard rejection of `marisa string_table` with a dual resolver:

- plain current C-string/self-relative offsets for no-marisa Yune tables
- `rsmarisa` reverse lookup for marisa-backed table string ids

Expected:

- Existing no-marisa compact table tests still pass.
- New tests prove the real product table can parse candidate text through `rsmarisa`.
- Unsupported multi-level index, advanced payload, correction/tolerance, and lookup-record gaps remain structured errors unless the selected fresh product build supplies those payloads separately.

- [ ] **Step 1.4: Select the storage route**

After Step 1.1-Step 1.3, make an explicit storage-route decision in
`storage-path.md`.

Preferred route:

- real product marisa string tables parse safely through `rsmarisa`
- runtime product loads can use `rsmarisa` without leaking buffers or relying on
  invalid lifetimes
- native and WASM ownership/mapping are both described

Fallback route, only after a verified preferred-route blocker:

- emit a Yune-owned product table format that stores text/code/comment payloads
  as offsets, ids, or interned arena entries
- avoid rebuilding the M36 owned `String` heap mirror
- keep source checksum freshness, TypeDuck rich comments, lookup records,
  correction/tolerance, and prism/reverse compatibility intact

Expected:

- `storage-path.md` records the selected route and why it satisfies the same
  storage outcome.
- A crate-level or WASM-level `rsmarisa` blocker does not strand M37 if the
  interned/byte-backed route satisfies the storage outcome gate and the native
  mmap/file-backed gate.

- [ ] **Step 1.5: Emit fresh storage-backed product tables**

Update the table writer or product deploy path so `workspace_update:<schema>` can create fresh product tables whose text storage follows the selected route and whose checksum matches source. Preserve the M36 lesson: the final passing path must be a coherent table/prism/reverse set, not an isolated table shortcut.

Expected:

- Fresh `jyut6ping3.table.bin` and `jyut6ping3_scolar.table.bin` use the selected byte-backed/interned storage route. For the preferred route, they have nonzero marisa string-table fields.
- `product_path_status.csv` reports fresh checksum, selected storage route, native mapping mode, table parse ok, prism parse ok, reverse parse ok, and `compiled_ready=true`.
- Final runtime path does not use source-YAML fallback or the M36 owned no-marisa final fallback.

- [ ] **Step 1.6: Prove behavior byte parity**

Run focused gates:

```powershell
cargo test -p yune-core --test cantonese_parity -- --nocapture
cargo test -p yune-rime-api --test typeduck_web -- --nocapture
cargo test -p yune-core compiled_payloads -- --nocapture
```

Expected:

- TypeDuck rich comments, lookup records, correction/tolerance, long composition, partial selection, default-confirm recomposition, and userdb learning stay byte-identical.
- `storage-path.md` records any crate patch/fork/fallback and why it remains safe.

- [ ] **Step 1.7: Prove storage memory movement**

Rerun the native memory rows and the memory-attribution table after storage
changes, even if latency work has not started.

Expected:

- Track B product median and peak working-set rows move in the expected
  direction beyond benchmark noise, or the evidence names the next memory owner
  and M37 continues.
- The selected product table path does not retain a parallel owned heap mirror
  of table strings/candidates.

## Task 2 - Remove Full Snapshot Cloning From Page-Only Context Reads

**Files:**

- Modify: `crates/yune-core/src/engine.rs`
- Modify: `crates/yune-core/src/state.rs`
- Modify: `crates/yune-rime-api/src/context_api.rs`
- Test: `crates/yune-rime-api/tests/frontend_client/`
- Test: `crates/yune-rime-api/tests/frontend_hosts/`
- Create: `docs/reports/evidence/m37-engine-hyper-optimization/context-page-export.md`

- [ ] **Step 2.1: Add a page snapshot API**

Add an internal engine API for page-only reads, shaped around the current menu page size and highlighted index. It should clone only:

- composition/status/preedit data needed by `RimeGetContext`
- the visible page candidates
- candidate-list completeness and page metadata

Expected:

- `Engine::snapshot()` remains available for full snapshot callers.
- `RimeGetContext` can use the page snapshot without cloning the entire `Vec<Candidate>`.

- [ ] **Step 2.2: Route `RimeGetContext` through page export**

Update `context_api.rs` so ordinary context reads export only the page view. Preserve hidden-candidate mode, select labels, select keys, commit text preview, chord prompt, affix prompt, `is_last_page`, and highlighted index behavior.

Expected:

- Existing frontend-client and frontend-host tests pass.
- The materialization counters show full candidate-list clone count drops to zero for ordinary page reads.

- [ ] **Step 2.3: Keep full-list readers explicit**

Audit callers that require a full candidate list and route them through `ensure_complete_candidate_list()` or `Engine::snapshot()` intentionally.

Expected:

- Debug inspector, candidate iterators, out-of-window paging, and selection beyond retained window remain correct.
- `context-page-export.md` lists each full-list caller and why it is still eager.

## Task 3 - Page-Bound Product Candidate Materialization

**Files:**

- Modify: `crates/yune-core/src/dictionary/query_table.rs`
- Modify: `crates/yune-core/src/translator/mod.rs`
- Modify: `crates/yune-core/src/engine.rs`
- Test: `crates/yune-core/src/tests/translator.rs`
- Test: `crates/yune-core/src/tests/engine.rs`
- Test: `crates/yune-core/tests/cantonese_parity.rs`
- Create: `docs/reports/evidence/m37-engine-hyper-optimization/materialization-gate.md`

- [ ] **Step 3.1: Replace boxed table iterators on the hot path**

If Phase 0 shows iterator dispatch is measurable, replace `TableStorage`'s boxed iterator returns with concrete enum iterators for heap, compact, and rsmarisa-backed stores.

Expected:

- No behavior change.
- This step is skipped only if Phase 0 proves it is not measurable; skipping it does not waive the materialization hard gate.

- [ ] **Step 3.2: Make TypeDuck product rows eligible for bounded requests**

Generalize the M34 bounded refresh gate past `luna_pinyin` only after proving the default TypeDuck product rows are safe. The first target set is:

- `hai`
- `ngohaig`
- `loengjathau`
- `jigaajiusihaa`

Expected:

- These rows use bounded materialization under the default product options measured by the native harness.
- Full-list-sensitive settings keep eager fallback with explicit evidence.

- [ ] **Step 3.3: Bound sort and merge work**

Replace full-list `sort_by` with a stable page-sized top-K or k-way merge where the request is bounded. Keep tie behavior deterministic and byte-identical for the first page.

Expected:

- First page, paging, selection, and default confirm are unchanged.
- Materialization counters prove only page plus surplus is sorted/materialized for default product rows.

- [ ] **Step 3.4: Classify filters, userdb, rankers, and prediction**

For each product feature, classify it as:

- page-safe
- surplus-safe
- full-list-only

At minimum classify `charset_filter`, rich dictionary lookup comments, userdb predictive merge, correction/tolerance, prefix fallback, `prediction_never_first`, `prediction_candidate_limit`, sentence-over-completion, and AI staged merge.

Expected:

- Page-safe and surplus-safe features use bounded work.
- Full-list-only features force explicit eager fallback and are named in `materialization-gate.md`.

- [ ] **Step 3.5: Prove the `hai` hard gate**

Run the native product key row repeatedly after the materialization change.

Expected:

- `hai` has a clear before/after reduction from the M37 baseline.
- `hai` no longer remains the unexplained `3x` outlier.
- If `hai` remains dominated by a new owner, continue with the next measured owner before closing M37.

## Task 4 - Index Remaining Full-List Product Owners

**Files:**

- Modify: `crates/yune-core/src/translator/mod.rs`
- Modify: `crates/yune-core/src/dictionary/compiled_prism.rs`
- Modify: `crates/yune-core/src/engine.rs`
- Test: `crates/yune-core/tests/cantonese_parity.rs`
- Test: `crates/yune-rime-api/tests/typeduck_web.rs`
- Create: `docs/reports/evidence/m37-engine-hyper-optimization/full-list-owner-indexes.md`

- [ ] **Step 4.1: Remove dynamic-correction all-code scans where measured**

If Phase 0 or Task 3 identifies correction scans as a top owner, add length/syllable buckets and reusable restricted-distance scratch space so `dynamic_correction_lookup` no longer requires scanning every code for ordinary product rows.

Expected:

- Correction-on TypeDuck rows improve or the evidence proves correction is not the current top owner.
- Default non-correction rows do not regress.

- [ ] **Step 4.2: Index prefix fallback and prediction-limit metadata**

If prefix fallback or prediction limits block bounding for product rows, add enough metadata to decide the first page without materializing the whole candidate list.

Expected:

- `prediction_never_first`, `assign_ordered_candidate_qualities`, and sentence-over-completion remain explicit stop gates. If one requires a full list, that ring stays eager and documented.

- [ ] **Step 4.3: Replace sentence path cloning only if measured**

If sentence/path DP appears as a top owner after materialization is bounded, replace `Vec<String>` path cloning with backpointers or piece ids.

Expected:

- This is not a default assumption. It lands only if measurement shows it owns product latency.

## Task 5 - Final Benchmarks, Reports, And Roadmap Closeout

**Files:**

- Modify: `docs/reports/yune-vs-librime-performance.md`
- Modify: `docs/reports/yune-vs-librime-root-cause-analysis.md`
- Modify: `docs/roadmap.md`
- Modify: `docs/requirements.md`
- Move on closeout: `docs/plans/active/m37-plan-engine-hyper-optimization.md` to `docs/plans/completed/m37-plan-engine-hyper-optimization.md`
- Create: `docs/reports/evidence/m37-engine-hyper-optimization/final/`
- Create: `docs/reports/evidence/m37-engine-hyper-optimization/final-gates.md`

- [ ] **Step 5.1: Run final native evidence**

Run:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p yune-core --test upstream_luna_pinyin_parity
cargo test -p yune-core --test cantonese_parity
cargo test -p yune-rime-api --test typeduck_web
cargo test --workspace
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\m37-engine-hyper-optimization\final -Iterations 5 -SessionIterations 20 -KeyIterations 20 -DeployProductBeforeBenchmark
git diff --check
```

Expected:

- All gates pass or any missing external/browser gate is explicitly justified.
- Final evidence includes selected storage route, `rsmarisa` outcome if tried, native mmap/mapping-mode status, materialization counters, Track A comparison rows, Track B before/after rows, memory-owner tables, memory movement rows, and `hai` attribution before/after.

- [ ] **Step 5.2: Run runtime/browser gates if runtime-visible files changed**

If M37 changes the WASM/runtime-visible engine path, rebuild the TypeDuck-Web WASM assets and run focused real-browser evidence before any browser claim.

Expected:

- Native engine wins remain native-only unless this evidence exists.
- M31 public-demo delivery/cache claims remain separate.

- [ ] **Step 5.3: Update public reports**

Update the performance and root-cause reports with:

- exact storage-route outcome, including `rsmarisa` result or verified fallback reason
- exact native mmap/file-backed loading outcome, including `mapping_mode=mmap`
  when `rsmarisa` is selected
- exact candidate materialization before/after counters
- Track B `hai` before/after movement
- Track B product row before/after medians and working set
- memory-owner attribution and memory movement versus the M36 final baseline
- Track A final ratios versus librime
- native versus browser caveats
- any full-list-only eager fallback that remains by design

Expected:

- No "faster than librime" claim unless Track A fair evidence proves it.
- No browser startup/typing claim unless browser evidence exists.

- [ ] **Step 5.4: Close the milestone honestly**

M37 may close only when:

- the selected byte-backed or interned product storage path is active on real product table data, with `rsmarisa` active or a documented verified fallback that satisfies the same no-heap-mirror outcome
- native product rows prove mmap/file-backed loading for the selected hot
  storage path, with `mapping_mode=mmap` when `rsmarisa` is selected
- product key rows no longer take the old full-list materialization/context clone path for page reads
- `hai` is explained and materially moved
- memory owners are attributed and Track B product memory moves materially from the M36 final baseline, or a maintainer-approved split to M38 is recorded before closeout
- final evidence and reports are checked in
- roadmap and requirements statuses match reality

If any hard gate is still open, keep the plan active and continue with the next measured owner. Do not archive M37 as "closed by no-go" for `rsmarisa`, storage outcome, native mmap/file-backed loading, candidate materialization, context export, or memory attribution/movement.

## Execution Handoff

Start from current `origin/main`. Read `AGENTS.md`, `docs/conventions.md`, `docs/roadmap.md`, `docs/requirements.md`, this plan, `docs/reports/yune-vs-librime-performance.md`, `docs/reports/yune-vs-librime-root-cause-analysis.md`, and the M36 evidence directory before editing. Keep M31 delivery/UI/Cloudflare work in its own lane. M37 owns engine storage and candidate materialization; if an M31 session touches engine/schema-install/runtime storage paths, serialize or move one track to a separate worktree. Stage only M37 files and preserve unrelated worktree changes.
