# M56 Engine Productization Hardening Implementation Plan

> **For agentic workers:** If a plan-execution sub-skill (e.g.
> superpowers:executing-plans) is available, use it; otherwise execute the
> checkboxes directly, in order, one phase at a time. Steps use checkbox
> (`- [ ]`) syntax for tracking.

> **Status:** Ready to execute - M55 is complete under the corrective re-baseline, and its `m55-thresholds.csv` is the standing native ratchet for M56 closeout. - **Track:** Engine robustness and compatibility. - **Created:** 2026-07-03 - **Type:** hardening milestone (tests, guards, and structural staleness-proofing; no new features, no ABI widening, no behavior change on the happy path).

**Goal:** Make the engine safe for two external frontends (Windows and iOS,
developed in separate repos) to build on: **(1)** compiled-asset staleness can
no longer produce silent wrong behavior, **(2)** the user-data lifecycle is
evidence-backed end to end (learning, persistence, recovery, migration), and
**(3)** no **validly addressable** input or call pattern arriving over the C
ABI — including documented null and degenerate cases — can take down the host
process. (Arbitrary dangling non-null pointers are caller UB by every C ABI's
nature; no guard can make those safe, and the contract row says so.)

**Why now, and why these three:** the engine is oracle-parity proven, but
"correct on the happy path" and "safe as a product dependency" are different
bars. The repo's own history picks the targets:

- **Staleness is the most recurrent bug class in this project.** WEB-02's
  `893 MiB` browser regression was a stale `Rime::Prism/3.0` asset silently
  falling back to source loading; M38's Track B "regression" was a benchmark
  over stale undeployed blobs; M41's deploy-skip silently disabled
  sentence/learning/combine in the default web product. (A fourth near-miss —
  WEB-04 browser evidence almost captured against a local WASM predating the
  engine fix under test — was caught in session review and is not separately
  committed; the three cited incidents are all committed-evidence-backed.)
  One class, repeatedly. Two more frontends multiply the exposure.
- **User data is the first thing real products exercise** that the parity
  fixtures do not fully cover: commit-then-learn ordering, persistence across
  restarts, corrupted-store recovery, and profile upgrades.
- **A panic across `extern "C"` kills the host process.** As of 2026-07-03
  there are **zero `catch_unwind` boundaries in the workspace** (verified by
  grep), and the release profile is `panic = "abort"` — any panic reachable
  from an ABI entry point aborts the embedding application; for an IME, that
  is the user's text editor or browser. A frontend team's trust in the engine
  is set by the first crash they cannot explain.

---

## Decided Calls (do not re-litigate without new evidence)

- **No public C ABI change.** `RimeApi` field order is the ABI; the M51 freeze
  holds. Hardening is guards, tests, and internal structure only. TypeDuck
  fork-only slots stay behind `rime_get_typeduck_profile_api()`.
- **No behavior change on defined paths.** Where librime defines observable
  behavior (userdb formats, learning effects on ranking, deployment
  semantics), the oracle defines correct; capture fixtures, never invent
  expectations. Where behavior is Yune-defined (crash policy, corrupt-store
  recovery, staleness rejection), define it in the engine support contract and
  test the definition.
- **Panic policy — stated honestly against the release profile.** The
  workspace release profile sets `panic = "abort"` (root `Cargo.toml`), so
  `catch_unwind` guards are **inert in the shipped artifact**: the release
  `cdylib`/WASM aborts before any guard runs. Therefore the policy has two
  layers, and the contract row must say so:
  1. **Primary defense (all builds): no panic reachable from the ABI on any
     input** — enforced by the abuse suite, which is the real product
     protection.
  2. **Secondary defense (unwind builds only): standardized
     `std::panic::catch_unwind` guards** on every `#[no_mangle] extern "C"`
     entry point, so dev/test builds and unwind-profile embedders get defined
     failure returns instead of UB/abort. `catch_unwind`/`AssertUnwindSafe`
     are safe APIs; the guard lives in `yune-rime-api` because that is where
     all exports live. `yune-core` stays `unsafe_code = "forbid"` with no
     guard machinery.
  Phase 3 contains an explicit decision checkbox on whether the shipped
  `cdylib` should move to `panic = "unwind"` (with size/latency evidence);
  until that decision lands, all public wording states that release builds
  abort on panic and the guarantee is the no-reachable-panic suite.
- **Poison policy is part of the crash contract, designed up front.** The
  session/service state is process-global `Mutex` singletons and
  `docs/conventions.md` §9 notes production session locks panic on poison. A
  caught panic that poisons a lock must not brick every subsequent call: the
  contract row defines recovery (recover the `PoisonError` guard or
  clear-and-rebuild the poisoned state), and the abuse suite's
  "follow-up happy-path call succeeds" assertion is what enforces it.
- **Loud staleness, never silent fallback.** A version-mismatched, truncated,
  or corrupt compiled artifact must trigger visible rebuild-or-error, never a
  silent slower/different path. This generalizes the WEB-02 lesson to every
  artifact kind.
- **Fuzz tooling:** property-style abuse tests use `proptest` as a
  dev-dependency of `yune-rime-api` (none of
  proptest/quickcheck/arbitrary exist in the workspace today — this is a new,
  test-only dependency). `cargo-fuzz`/nightly fuzzing is a stretch goal, not a
  gate.
- **Performance is guarded, not improved, here.** M55 owns performance. M56
  closes with one full run of the M55 corrective standing regression ratchet
  to prove hardening cost nothing measurable.
- **Browser/harness surfacing is out of scope** — the companion WEB-05
  harness control-surface milestone owns exposing these diagnostics in
  `yune-web`; it may run in parallel (different track, different evidence
  lane).

## Current Starting Point

Verified repo facts (reviewed for execution readiness 2026-07-04):

- User-data machinery already exists and is non-trivial: **all modules under
  `crates/yune-rime-api/src/userdb/`** (`file_store.rs`, `record.rs`,
  `recovery.rs`, `snapshot.rs`, `store.rs`, `sync.rs`, `mod.rs`) plus the ABI
  facade `src/userdb.rs` (7 exports) and the levers surface `src/levers.rs`
  (25 exports). Existing coverage lives in the in-crate module
  `crates/yune-rime-api/src/tests/userdb.rs` (NOT under `tests/`). **This
  milestone gap-analyzes and hardens; it does not greenfield.** Phase 0 must
  inventory what these already cover before any new test is written.
- `catch_unwind` count in `crates/`: **0**, and the workspace release profile
  is `panic = "abort"` (root `Cargo.toml`). Panic reaching any `extern "C"`
  export kills the host.
- Environmental prerequisite for the gates: `cantonese_parity` (and some
  schema-selection tests/benches) read dictionary assets at runtime from the
  **gitignored machine-local** `apps/yune-web/source/public/schema` checkout.
  If it is absent, those failures are environmental, not caused by your
  change — provision the checkout or record the gap before attributing
  failures.
- Compiled-artifact kinds known to exist (Phase 0 completes this list): the
  compact table storage, compiled prism (`Rime::Prism`-shaped, stale-rejection
  precedent from the WEB-02 fix), reverse-lookup assets, octagram `.gram`
  loading (M54, logical-resource-id validated), userdb file store, and the
  M55-introduced `YUNE-POET/2` poet artifact (explicit opt-in at runtime).
- The full quality gate and parity suites
  (`upstream_luna_pinyin_parity`, `cantonese_parity`,
  `cargo test -p yune-rime-api --test yune_web`) are the standing engine
  gates; `docs/contracts/engine-support-contract.md` is where Yune-defined
  behavior gets documented.
- The CLI can drive the full ABI path from arbitrary data dirs:
  `cargo run -p yune-cli -- frontend --shared-data-dir <dir> --user-data-dir
  <dir> --schema <id> --sequence "<keys>"` — this is the product-path
  instrument for cold-start and lifecycle tests.

## Win Bars (set now, before any code)

M56 closes **complete** when all of the following hold, each with committed
evidence:

1. **Staleness:** every artifact kind in the Phase 0 inventory has (a) a
   version/validity check and (b) a stale-injection real-path test proving
   loud rejection-and-rebuild (or hard error where rebuild is impossible).
   Cold/warm conformance coverage is required for artifacts exercised by the
   two named product paths below (`luna_pinyin`, TypeDuck `jyut6ping3`);
   artifact kinds outside those paths, such as optional `.gram` or explicit
   opt-in poet storage, get artifact-specific stale-injection coverage rather
   than fictional cold-start coverage. Zero silent fallbacks.
2. **Cold-start conformance:** a standing test drives deploy → schema select →
   key sequence → candidates from **empty** shared/user dirs for both named
   profiles (`luna_pinyin`, TypeDuck `jyut6ping3`) and asserts non-empty,
   fixture-pinned first-page candidates. It runs in `cargo test --workspace`.
3. **User data:** every lifecycle behavior in the Phase 0 gap ledger is either
   (a) fixture-tested against oracle capture, (b) tested against a documented
   Yune-defined contract, or (c) recorded as a named
   `#[ignore = "blocked: ..."]` gap — zero silent gaps.
4. **ABI robustness:** the abuse suite passes with **zero panics reaching any
   ABI entry point** — including out-of-order lifecycle calls,
   invalid/degenerate inputs, and (within the documented threading promise)
   cross-thread session traffic — with post-panic recovery proven (a caught
   panic must not poison the process: follow-up happy-path calls succeed).
   Every `extern "C"` export is unwind-guarded (verified by a test-enforced
   inventory, not by convention), with the release-profile panic strategy
   explicitly decided and documented per the panic-policy Decided Call.
5. **No regressions:** full quality gate green
   (`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
   warnings`, `cargo test --workspace`), parity suites green, and one full
   standing-ratchet benchmark run green.

Close **partial** if a specific behavior turns out to require ABI or
oracle-capture work beyond scope — recorded as named blocked rows, per repo
convention.

## Scope

In scope: staleness guards + tests, cold-start conformance, user-data
lifecycle fixtures/contract tests, panic-boundary guards, proptest abuse
suite, support-contract documentation of Yune-defined behavior, closeout docs.

Out of scope: performance work (M55), new features, any sync/cloud service,
remote anything, browser/harness changes (WEB-05), frontend code, C++ plugin
ABI, `packages/yune-web-runtime`.

## Files And Responsibilities

- Create: `docs/reports/evidence/m56-productization-hardening/`
  (`phase-0-inventory/`, `phase-1-staleness/`, `phase-2-user-data/`,
  `phase-3-abi-abuse/`, `final/`).
- Create: `crates/yune-rime-api/src/ffi_guard.rs` (or similar owning module) —
  the standardized unwind-guard wrapper; applied to every
  `#[no_mangle] extern "C"` export discovered by the Phase 0 ABI ledger, not
  only the obvious `lib.rs`/`api_table.rs` bodies. Current exports span
  `config_api.rs`, `runtime.rs`, `deployment.rs`, `web_runtime.rs`,
  `levers.rs`, `userdb.rs`, `candidate_api.rs`, `context_api.rs`,
  `ffi_memory.rs`, `key_table.rs`, `modules.rs`, `notifications.rs`,
  `schema_api.rs`, `schema_selection.rs`, `session.rs`, `api_table.rs`, and
  `lib.rs`.
- Create: `crates/yune-rime-api/tests/cold_start_conformance.rs` and
  `crates/yune-rime-api/tests/abi_abuse.rs` (proptest dev-dependency).
- Modify: artifact load paths that lack validity checks (locations determined
  by the Phase 0 inventory; expected neighborhoods:
  `crates/yune-rime-api/src/schema_install.rs`, `deployment.rs`,
  `crates/yune-rime-api/src/userdb/file_store.rs` + `recovery.rs`, and the
  compiled-storage readers in `yune-core` — readers in core stay `unsafe`-free
  and return typed errors; loud handling stays in the ABI crate).
- Modify: `docs/contracts/engine-support-contract.md` (Yune-defined behavior
  rows: crash policy, staleness policy, recovery semantics).
- Modify on closeout only: `docs/roadmap.md`, `docs/requirements.md`,
  `docs/ledgers/milestone-history.md`; move this plan to
  `docs/plans/completed/`.

## Execution Rules For The Implementing Agent

Read before starting: `AGENTS.md` (Verification Discipline),
`docs/conventions.md`, `docs/contracts/engine-support-contract.md`, and this
plan end to end.

1. **Inventory before code.** Phases 1-3 execute against the Phase 0 ledgers,
   not against this plan's illustrative lists. If Phase 0 finds a behavior
   already tested, record the pointer and move on — do not duplicate.
2. **Oracle vs Yune-defined, always explicit.** Every new test states in a
   comment which kind it is. Oracle-kind tests use externally captured bytes
   (the existing capture tooling and fixture-provenance conventions from
   `crates/yune-core/tests/oracle_fixture_provenance.rs`); Yune-defined tests
   cite the support-contract row they enforce.
3. **Fresh dirs, always.** Cold-start and lifecycle tests build their worlds
   in fresh temp dirs per test; never share mutable fixture state between
   tests. (This is also what makes them staleness-proof by construction.)
4. **No `unsafe` in `yune-core`; FFI/pointer work stays in `yune-rime-api`.**
   The unwind guard, null/invalid-pointer handling, and any allocator or OS
   probing live in the ABI crate or test code.
5. **Record honestly.** Blocked is blocked; gaps are named `#[ignore]` rows;
   every phase close lists the exact commands run.
6. **Keep diffs behavior-preserving.** If a hardening change alters any
   parity fixture or benchmark row, that is a defect in the change, not a new
   baseline.

---

## Phase 0: Hardening Inventory & Gap Ledgers (read-only)

**Owner:** knowing precisely what exists, so Phases 1-3 harden gaps instead of
duplicating coverage.

- [ ] **Artifact ledger** (`phase-0-inventory/artifact-ledger.md`): enumerate
  every compiled/persistent artifact kind the engine reads (grep the load
  paths in `schema_install.rs`, `deployment.rs`, `userdb/`, and the
  `yune-core` compiled-storage readers). Per kind: producer, consumer, version
  tag (or NONE), current behavior on version-mismatch / truncation /
  corruption (read the code; where unclear, prove it with a throwaway
  experiment in a temp dir), and existing test coverage.
- [ ] **User-data behavior ledger** (`user-data-ledger.md`): enumerate the
  lifecycle behaviors across **all** modules under `src/userdb/` (including
  `store.rs`) plus the ABI facade `src/userdb.rs` and the levers user-dict
  surface `src/levers.rs`; current coverage lives in
  `src/tests/userdb.rs`. Per row: behavior, oracle-defined or Yune-defined,
  current coverage, gap disposition.
- [ ] **ABI entry-point ledger** (`abi-entry-ledger.csv`): every
  `#[no_mangle] extern "C"` export in the workspace — grep the **whole crate**
  (`no_mangle` spans many files; the biggest surfaces are `levers.rs`,
  `userdb.rs`, `lib.rs`, and `web_runtime.rs`; `api_table.rs` carries the four
  API-table accessors `rime_get_api` / `rime_get_typeduck_profile_api` /
  `rime_get_yune_windows_profile_api` / `rime_levers_get_api`; `abi.rs` holds
  the table *type* and has no exports) — with: input preconditions, failure
  return value, panic-reachability guess, unwind guard present (expected:
  none today), thread-safety expectation.
- [ ] Cross-check the ledgers against `docs/contracts/engine-support-contract.md`
  and record which Yune-defined behaviors are currently **undocumented**.
- [ ] Phase gate: three committed ledgers + a prioritized gap list per phase;
  no code changes; quality gate untouched (nothing to run beyond fmt for the
  docs).

## Phase 1: Staleness-Proofing + Cold-Start Conformance

**Owner:** the recurrent silent-staleness bug class (WEB-02 / M38 / M41 /
WEB-04-stale-WASM).

- [ ] For every artifact kind in the ledger without a validity check: add a
  version/magic/length check at load, with **loud** rejection (typed error in
  `yune-core` readers; rebuild-or-error decision at the ABI/deploy layer).
  Follow the compiled-prism stale-rejection precedent (the WEB-02 fix) as the
  reference shape.
- [ ] Stale-injection tests, one per artifact kind: corrupt / truncate /
  version-bump the artifact in a temp deployment, then drive the real path and
  assert the loud behavior (rebuild happened, or a defined error surfaced) —
  never a silent success with different behavior. Assert rebuilt-artifact
  freshness by timestamp/content, not by absence of error.
- [ ] **Cold-start conformance test**
  (`crates/yune-rime-api/tests/cold_start_conformance.rs`). Two variants,
  because `apps/yune-web/public/schema` ships **committed prebuilt `*.bin`
  artifacts and a `build/` dir** (staging it wholesale is a warm start —
  the `native_memory_probe.rs` precedent explicitly reuses prebuilt assets):
  - **Cold:** stage source YAML/text assets only (exclude `*.bin` and
    `build/`) into empty temp dirs, deploy (full compile), select, type, and
    assert fixture-pinned first-page candidates.
  - **Warm:** stage the full committed root including prebuilt artifacts and
    assert the same candidates plus artifact reuse (no rebuild).
  Schema ids, exactly: `luna_pinyin`, and `jyut6ping3` for the
  default-deploy path (that is what `default.yaml` lists); use
  `jyut6ping3_mobile` only where reusing existing TypeDuck fixtures that pin
  it. Pinned candidates are oracle-captured — upstream librime 1.17.0 for
  `luna_pinyin`, **TypeDuck v1.1.2 for the `jyut6ping3` profiles** — reusing
  existing parity fixtures where they already pin the sequence. Record who
  re-captures when a dictionary/schema update deliberately lands (the test
  couples to committed dictionary bytes by design). Budget the runtime: the
  cold TypeDuck deploy is expensive — share one deploy per test binary
  (`OnceLock`) rather than per test, and keep total added `cargo test
  --workspace` time measured and recorded in evidence.
- [ ] **Deploy-skip audit** (the M41 lesson): add a test that deploys, flips a
  config-patch option, redeploys (and skip-redeploys where the engine offers
  that path), and asserts the effective option states match the librime
  deployment semantics — a skipped deploy must be behavior-preserving. **If
  Phase 0 finds no engine-level skip path exists** (the M41 incident lived in
  the web-harness worker, not the engine ABI), close this checkbox as a
  documented N/A ledger row — do not go searching for a path that is not
  there.
- [ ] Document the staleness policy as support-contract rows.
- [ ] Phase gate: quality gate + parity suites green; new tests in
  `cargo test --workspace`; evidence README lists commands.

## Phase 2: User-Data Lifecycle

**Owner:** the user-data behaviors two frontends will exercise in week one.

Work the Phase 0 user-data ledger top-down. Expected rows (illustrative — the
ledger is authoritative):

- [ ] **Learning effect:** commit a candidate, assert the documented ranking
  effect on the next identical input — oracle-captured where librime defines
  it (capture the same interaction against librime with a fresh user dir),
  Yune-defined-and-documented otherwise.
- [ ] **Persistence:** learn → destroy session → recreate (same user dir) →
  assert the learned effect survives; then simulate process restart (new
  engine init over the same dirs) and assert again.
- [ ] **Recovery:** corrupt/truncate the user store in defined ways; assert
  the `recovery.rs` behavior matches a documented contract row (data loss
  bounded, no crash, engine still serves candidates). Extend recovery tests
  where the ledger shows gaps.
- [ ] **Migration/upgrade:** load a user store written by the oldest supported
  format (per the ledger; if only one format exists, pin it with a
  format-freeze test so future changes must add migration).
- [ ] **Sync surface:** whatever `userdb/sync.rs` + the ABI expose today gets
  a behavior-pinning test (no new sync features; freeze what exists).
- [ ] Every remaining ledger gap: fixture, contract row, or named
  `#[ignore = "blocked: ..."]`.
- [ ] Phase gate: quality gate + parity suites green; ledger fully
  dispositioned (the M21 "done = ledger fully dispositioned" pattern).

## Phase 3: ABI Abuse Suite + Panic Boundary

**Owner:** host-process safety — no crash a frontend cannot explain.

- [ ] **Threading promise first, before any concurrent test.** Nothing in the
  repo pins threading semantics today (the support contract has zero threading
  rows and D-P2-2 declined multi-threaded frontend scope). Derive a
  **Yune-defined** support-contract threading row informed by upstream librime
  source/header comments at the pinned commit
  (`rime/librime@33e78140250125871856cdc5b42ddc6a5fcd3cd4`) — state exactly
  what cross-thread traffic is tolerated (e.g. distinct sessions from distinct
  threads) vs. rejected, and land the contract row before writing tests
  against it.
- [ ] **Poison-recovery design** (per the Decided Call): decide and implement
  the `PoisonError` handling for the global session/service locks so a caught
  panic cannot brick subsequent calls; contract row + focused test (panic
  injected under the guard in a test build, next call succeeds).
- [ ] Implement the standardized unwind guard and apply it to **every** export
  in the ABI entry-point ledger. Add a meta-test that enumerates exports (via
  the ledger, kept in sync by the test) and fails if an unguarded export is
  added later.
- [ ] **Release panic-strategy decision checkbox:** evaluate flipping the
  shipped `cdylib` to `panic = "unwind"` (binary-size + one ratchet-run
  latency evidence, and note wasm32/Emscripten unwinding cost) versus keeping
  `panic = "abort"` with the no-reachable-panic suite as the sole release
  defense. Record the decision and its evidence; update the support-contract
  crash row to match. Either outcome is acceptable; an undocumented default is
  not.
- [ ] Add `proptest` as a dev-dependency of `yune-rime-api`;
  `tests/abi_abuse.rs` covers, at minimum:
  - out-of-order lifecycle (calls before init, after finalize, on destroyed
    sessions, double-destroy, wrong session ids);
  - degenerate inputs (empty/huge/invalid-UTF-8 strings, null pointers where
    the slot takes pointers, zero/negative lengths, absurd modifiers/keycodes);
  - configuration abuse (missing/unreadable data dirs, schema ids that fail
    logical-resource-id validation, deploy with partial data).
  Use proptest for **single-threaded input generation only**. Concurrency is
  tested separately with deterministic scripted interleavings or a bounded
  stress loop, restricted to what the threading promise allows — and audit
  first that no tested path mints `&'static mut` from shared handles (e.g.
  `config.rs` `config_state_mut`, `levers.rs`): two threads aliasing those is
  UB **in the test itself**, not a finding about the engine.
  Every case asserts: defined failure return, no panic escaping the entry
  point, process still usable (a follow-up happy-path call succeeds,
  including after a poisoned-lock recovery).
- [ ] Record one full-suite run with `RUST_BACKTRACE=1` in evidence proving no
  guard fired on the happy path. (Note: `cargo test` runs under
  `panic = "unwind"`, which is what makes the guards testable; the release
  posture is covered by the decision checkbox above.)
- [ ] Document the crash/threading/poison policies as support-contract rows.
- [ ] *(Stretch, non-gating)*: a `cargo-fuzz` target over `process_key` input
  bytes, recorded as future work if nightly tooling is unavailable.
- [ ] Phase gate: quality gate + parity suites green; abuse suite green with
  zero aborts.

## Phase 4: Closeout

- [ ] One full standing-ratchet benchmark run — green, committed under
  `final/`; hardening must cost nothing measurable. M55 has handed over the
  standing native Track A gate, so use its corrective per-key thresholds and
  include Track B product handling:

  ```powershell
  powershell -ExecutionPolicy Bypass -File scripts/benchmark-native-rime-inprocess.ps1 `
    -OutputRoot docs\reports\evidence\m56-productization-hardening\final\ratchet-run `
    -Iterations 9 -SessionIterations 60 -KeyIterations 80 `
    -TrackAInputs "n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru" `
    -TrackBInputs "neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung" `
    -DeployProductBeforeBenchmark `
    -TrackAThresholds docs\reports\evidence\m55-native-match-or-beat\thresholds\m55-thresholds.csv `
    -FailOnRegression
  ```

  The explicit `-TrackAInputs` is load-bearing: the script's default input
  list omits `n`, which the threshold artifact gates — omitting the switch
  makes the closeout run fail on a missing row. (`-OutputRoot` must be a
  fresh leaf dir — the script clears its target.) The M55 threshold artifact
  includes `track-b/` rows, so `-DeployProductBeforeBenchmark` and the
  machine-local `apps/yune-web/source/public/schema` checkout are required; if
  that checkout is unavailable, record it as an environmental blocker rather
  than closing the ratchet.
- [ ] Full quality gate + parity suites, commands listed verbatim.
- [ ] Support-contract, `docs/roadmap.md`, `docs/requirements.md`,
  `docs/ledgers/milestone-history.md` updated; plan moved to
  `docs/plans/completed/`.
- [ ] Handoff note for the frontend repos: one page listing the new guarantees
  (staleness policy, crash policy, user-data contract) with pointers to the
  enforcing tests.

## Definition Of Done

All five Win Bars hold with committed evidence; every ledger row is
dispositioned (tested / contracted / named-blocked); no ABI diff
(`assert_api_slot!` untouched); no parity or ratchet regression. Partial
closure requires each unmet bar to have a named blocked row, not silence.

## Proposed Requirement IDs (add to `docs/requirements.md` at closeout only)

(House style per `docs/requirements.md`: `<MILESTONE>-<TOPIC>-<NN>`.)

- **M56-HARDEN-01**: Every compiled/persistent artifact kind has
  version/validity checking with loud rejection, a stale-injection test, and
  cold/warm conformance coverage.
- **M56-HARDEN-02**: Standing cold-start (source-assets-only) and warm-start
  (prebuilt-artifact) conformance tests drive deploy → select → type →
  candidates from temp dirs for the named profiles inside
  `cargo test --workspace`, with measured runtime recorded.
- **M56-HARDEN-03**: The user-data lifecycle ledger is fully dispositioned:
  oracle-fixtured, contract-tested, or named-blocked — zero silent gaps.
- **M56-HARDEN-04**: Every `extern "C"` export is unwind-guarded (meta-test
  enforced); the abuse suite passes with zero panic escapes and proven
  post-panic recovery (poison policy included); the release panic strategy is
  an explicit, evidenced decision.
- **M56-HARDEN-05**: Yune-defined staleness, recovery, crash/poison, and
  threading policies are documented in the engine support contract with
  enforcing tests cited.
- **M56-HARDEN-06**: Closeout proves no performance or parity regression via
  the standing ratchet (verbatim command recorded) and parity suites.

## Review Prompt

> Please review `docs/plans/active/m56-plan-engine-productization-hardening.md`
> as the queued M56 plan. Focus on: whether the three tracks (staleness,
> user-data, ABI abuse) are the right hardening priorities given the repo's
> incident history; whether the inventory-first structure (Phase 0 ledgers
> driving Phases 1-3) prevents duplicate or fictional work given the existing
> `userdb/` machinery; whether the panic-boundary policy is correctly split
> across the crate lint boundary (guards in `yune-rime-api`, no `unsafe` and
> no guard machinery in `yune-core`); whether oracle-defined vs Yune-defined
> behavior is kept explicit everywhere; and whether the plan stays inside "no
> ABI change, no behavior change on defined paths" while still being
> executable by a weaker model.
