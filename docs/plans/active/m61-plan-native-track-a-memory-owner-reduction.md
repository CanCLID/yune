# M61 Native Track A Memory-Owner Reduction

> **Milestone:** M61. **Status:** Draft for review — queued after M60; not
> authorized for execution. **Track:** native engine performance, Windows Track
> A `luna_pinyin` acceptance lane. **Created:** 2026-07-15. **Type:**
> attribution-first measurement and conditional reduction plan.

## Outcome

M61 proposes one narrow performance milestone: reduce the retained native
Track A `luna_pinyin` memory owner while preserving every signed M59 latency,
behavior, product-guard, and ABI boundary.

The leading hypothesis is the existing `YUNE-POET/2` byte-backed sentence-model
path. M55 measured a substantial memory reduction from that path but correctly
left it opt-in because the honest per-key long-input gate was red. Current main
now contains the later byte-backed incremental-scratch work at `759ff5d7`, plus
subsequent Luna lookup/surface changes, but no source-current Windows A/B proves
that the former latency blocker is gone. M61 therefore begins with measurement,
not a default flip.

M61 closes successfully only if all of these conditions hold:

1. one exact clean M61 measurement-tooling commit, descended from the pushed
   M60 closeout through recorded plan-only commits and containing no production
   behavior change, is reproduced in five fixed-binary Windows owned-mode
   rounds and five fixed-binary byte-backed diagnostic rounds;
2. current process counters and non-overlapping owner rows name a reducible
   owner and reconcile the observed movement rather than relying on M47/M55
   historical bytes;
3. a retained production-default implementation reduces Track A peak memory by
   the predeclared amount and is corroborated by same-process Windows private
   bytes plus non-overlapping named-owner evidence;
4. all unchanged M59/M55 signed rows, candidate snapshots, oracle behavior, and
   Track B guards remain green in five final rounds;
5. the optimized path is the native production default without a required
   environment flag, hidden cache, disabled behavior, reduced page, or browser
   payload change; and
6. compact source-bound evidence plus two independent reviews close the
   milestone.

If the current owner is absent, attribution does not reconcile, the byte-backed
control remains red, or no bounded branch clears the win bar, M61 closes
partial/no-go without a production change. That is a valid outcome.

## Authority And Boundaries

M60's plan is finalized but the milestone is not yet complete. M60 remains the
sole authorized execution milestone. M61 may be finalized only after M60 closes
and pushes; kickoff must replace `<M60_CLOSEOUT_SHA>` below with the actual
commit and record its exact tree.

The separate owner request that created this draft satisfies M60's statement
that M61 cannot be inferred from M60. Nothing in this plan changes M60's scope,
requirements, or evidence.

Binding references:

- upstream `rime/librime 1.17.0`:
  `33e78140250125871856cdc5b42ddc6a5fcd3cd4`;
- current signed native registry:
  `docs/reports/evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv`;
- final M59 Windows performance source: `443cc636862806e4f0dd1e12ab2e2e45f4189154`;
- final M59 five-round aggregate: `32/32` rows and `160/160` individual
  observations green; and
- planned M61 base: `<M60_CLOSEOUT_SHA>` after M60 is complete and pushed.

The M59 packet records a five-run Track A peak working-set median of
`153,899,008 B`, a worst of `153,956,352 B`, and about `8.9x` the same-run
librime peak. Those values are planning context only. They are not M61's
baseline because the measured M59 source predates later Luna-path commits,
including `759ff5d7`, `7f758fba`, and `51808ee6`. Historical M55 owned
`~185–188 MB`, byte-backed `~113 MB`, and Tier-M `125,000,000 B` records are
also context rather than inherited measurements.

### Evidence lanes

- **Binding acceptance:** Windows native Track A `luna_pinyin`, using the
  established same-run librime comparison and Windows working-set/private
  counters.
- **Regression guard:** Windows Track B `jyut6ping3_mobile` absolute rows. Track
  B has no peer ratio and cannot become the M61 memory-win lane.
- **Optional diagnostic:** exact-source macOS native Track A RSS. macOS RSS may
  confirm direction but cannot close or veto the Windows milestone unless a
  cross-platform claim is separately approved.
- **Separate completed product lane:** M47's comments-intact keyboard proxy.
  Its `67.4 MB` working set / `22.5 MB` private result is not a Track A baseline.
- **Out of lane:** browser WASM/encoded-resource memory and Apple
  `phys_footprint`. Neither is inferred from Windows counters.

M61 makes no public C ABI/API-table/export change, schema/profile-id change,
oracle rebase, browser payload change, product/frontend change, Windows TSF/UI
change, Cloudflare change, or iOS-device claim. D-24, D-25, D-31, D-47, D-48,
and D-49 remain in force.

The user-owned staged `.codex/config.toml` path remains excluded from every M61
commit and measurement source claim.

## Proposed Requirements

These IDs are provisional while this plan is a draft. Add them to
`requirements.md` and traceability only when M61 is independently reviewed and
finalized after M60:

- **M61-BASELINE-01:** reproduce an exact-source five-round owned baseline and
  five-round byte-backed diagnostic A/B with fixed binaries and full receipts;
- **M61-ATTR-01:** reconcile current whole-process and named-owner memory and
  select one structural owner before implementation;
- **M61-BRANCH-01:** preserve the diagnostic verdict and authorize at most one
  measured owner branch;
- **M61-REDUCE-01:** make the accepted reduction production-default without an
  environment opt-in or behavior omission;
- **M61-COMPAT-01:** preserve oracle candidates, reachability/order, model
  checksums, ABI shape, lifecycle, and browser-payload boundaries;
- **M61-RATCHET-01:** pass the unchanged signed native registry and a separate
  one-row M61 memory ratchet across five final rounds; and
- **M61-EVIDENCE-01:** publish compact source-bound evidence, two reviews, and a
  fail-closed packet manifest while retaining raw output externally.

## Acceptance Contract

### Fixed measurement shape

All baseline, diagnostic, and final acceptance rounds use:

- the exact same 17 Track A inputs already bound by M59;
- Track B input
  `neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung`;
- `--iterations 9`, `--session-iterations 60`, and `--key-iterations 80`
  (or the equivalent PowerShell parameters);
- product deployment enabled;
- the pinned upstream librime commit and the exact same schema/input trees;
- context read after every key, with no key deferral or input alias;
- five logical rounds per accepted set, all preserved;
- one prebuilt benchmark executable and one measured Yune DLL per source. All
  five hashes within a set must be identical. The owned/byte-backed diagnostic
  should use the same DLL so the runtime storage mode is the only variable; and
- a fresh process plus disjoint create-new output, user, shared, deployment-
  build, and work roots for every mode/round. No translator, process-global
  cache, or deployed artifact is reused across rounds. Record the table, prism,
  schema, and POET presence/hash inventory for every round; the source inputs
  behind those isolated copies remain byte-identical.

The owned/byte-backed A/B source is the exact pushed M61 measurement-tooling
commit created after Phase 0, not the untouched M60 closeout commit. Its parent
boundary is the recorded M60 closeout SHA. Record every intervening plan-only
commit; after excluding those documentation/requirements changes, the
non-documentation implementation diff from M60 may contain only the diagnostic
selector and missing attribution required below. The final production candidate
receives its own exact-source five-round set.

The Windows machine must be on AC power, quiet, thermally stable, and free of
concurrent compilation, indexing, backup, export, or other CPU/memory-heavy
work. Record start/end timestamps and significant workloads.

### Memory win

The final production-default candidate must satisfy every condition below
against the fresh five-round owned baseline:

1. both the five-run median and pooled worst Track A peak working set fall by
   at least `20%`;
2. the final pooled-worst Track A peak working set is at most
   `125,000,000 B`;
3. the selected named owner falls by at least `80%`, with no replacement
   non-overlapping heap owner above `5,000,000 B`;
4. the five-run median `track_a_private_envelope_bytes` defined below falls by
   at least `10,000,000 B`, while neither that envelope nor
   `process.after_ready_private_bytes_proxy` worsens by more than `5%` in
   five-run median or pooled worst;
5. after-ready/steady and peak working set, private bytes, mapped-file bytes,
   and non-overlapping named-owner rows all remain visible; and
6. every final round remains under the unchanged M59 memory ceiling.

The same-run librime ratio is diagnostic, not an M61 pass/fail target. M61 does
not promise parity or use the historical "188 MB gap" as a target.

The measurement-tooling commit must emit these exact private-byte receipts:

- `track_a_private_envelope_bytes`: for each round, take the maximum
  `median_private_bytes` across the 17 Yune Track A key-sequence rows in
  `summary.csv`. Each source row remains the median across its nine samples of
  `max(after_ready_private_bytes, after_finalize_private_bytes)`; this is a
  private-byte envelope, not a counter sampled at peak working set.
- `process.after_ready_private_bytes_proxy`: a new non-owner process row in
  `memory-owner-profile.csv`, computed as the median of raw
  `after_ready_private_bytes` across that round's Yune Track A key-sequence
  samples. This row supplies the same-phase whole-process value for owner
  reconciliation.

For either receipt, the five-run median is the median of the five per-round
values and pooled worst is their maximum. Never label either receipt
"peak private bytes."

The `20%` relative bar is a deliberately material reduction, well above the
historical five-round Track A spread, rather than a claim about an inherited
baseline. The independent `125,000,000 B` cap is a conservative absolute bound
informed by the byte-backed range already demonstrated in M55, not a reuse of
M55 as current evidence. Both values are frozen when this plan is finalized,
before the first accepted M61 baseline. They cannot be adjusted, rebaselined,
or waived after measurement inside M61.

### Regression and behavior bars

- Keep `m55-thresholds.csv` byte-for-byte unchanged.
- All `32/32` aggregate rows and all `160/160` individual observations pass in
  the final five-run set, including Track B absolute rows.
- Every mode's Track A candidate snapshots and page/order/model identities
  match the same-run pinned-librime capture, with the upstream fixtures
  `crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-basic.json` and
  `crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-sentence-expanded.json`
  retaining their owning tests. Equality with the fresh owned mode is a
  source-current control, never the behavior oracle. A changed candidate,
  comment, segmentation, selection, recomposition, or commit is a correctness
  red, not a tradeoff.
- M59 Luna reachability/page-order guards, upstream Luna parity, lifecycle/
  cache invalidation, corrupt-artifact behavior, and browser manifest policy
  remain green.
- The final default does not require `YUNE_POET_BYTE_BACKED=1`. That variable
  remains diagnostic/migration compatibility only unless a later decision
  removes it.
- No `.poet.bin` is added to `apps/yune-web/public/schema`; the web manifest's
  no-POET-payload rule remains binding.

After a green result, create a separate M61 memory-only ratchet containing one
Track A peak-working-set row at the predeclared `125,000,000 B` ceiling plus
source/provenance fields. It must pass alongside the existing signed registry;
it does not copy, replace, or supersede that registry. Never rewrite the
historical signed registry, loosen another row, or call the new ratchet
authoritative before final owner approval.

## Phase 0 — Finalize After M60

- [ ] Fetch the pushed M60 closeout and fill in `<M60_CLOSEOUT_SHA>`.
- [ ] Re-audit the roadmap, performance dashboard, M60 closeout, current
      `YUNE-POET/2` code, benchmark wrappers, and threshold registry.
- [ ] Confirm no post-draft change already moved the owner or invalidated the
      branch hypothesis.
- [ ] Convert the proposed requirement IDs into planned rows only after review.
- [ ] Record the final path allowlist, external evidence root, exact inputs,
      toolchain, machine identity, pinned oracle/artifact hashes, and the exact
      M60-to-measurement-tooling commit chain.
- [ ] Run two plan reviews: evidence/measurement validity, then scope/isolation
      and threshold safety. Only then mark the plan finalized.

If M60 is not pushed, the base is dirty beyond the excluded config, the current
benchmark no longer reads context per key, or the relevant owner cannot be
measured, stop before M61 execution.

## Phase 1 — Fresh Baseline And Diagnostic A/B

### Benchmark mode support

The current wrappers intentionally require default-owned runtime measurement
while using byte-backed mode only during deploy preparation. M61 may add one
explicit diagnostic-only mode selector to both native wrappers:

- default value: `production-default`;
- diagnostic values: `owned` and `byte-backed`;
- recorded in `environment.txt`, commands, and the owner profile;
- rejected when combined with an ambiguous inherited environment variable;
- no effect on the existing signed invocation when omitted; and
- never accepted as proof of the final shipping default.

The selector must not skip deployment, alter inputs/iterations, loosen a gate,
or choose different candidate behavior. The measurement-tooling commit keeps
the omitted/default owner assertion at the current owned/no-`poet_bin` state;
only a later accepted production-candidate commit may change the normal signed
assertion to require validated `poet_bin` storage.

### Runs

- [ ] Create a disposable clean detached clone of the exact pushed M61
      measurement-tooling commit. Do not switch an existing tree, create a
      branch, or use the dirty main checkout as measurement evidence. Record
      the M60 parent boundary and clean status before and after every accepted
      set, record the intervening plan-only commit chain, and remove the clone
      only after the external packet is secured.
- [ ] Build the benchmark and Yune library once; record SHA-256 values.
- [ ] Run five complete `owned` rounds. Require the unchanged signed registry,
      candidate/model identities, Track B guards, and fixed binary hashes to be
      green before treating memory work as eligible. Any owned-baseline red
      stops M61 for regression diagnosis.
- [ ] Run one explicitly named exploratory byte-backed round. A measured red is
      preserved and skips the five-round byte-backed set. It may select only
      disposition B or D, and only if its owner evidence is sufficient. Any
      correction receives a new source and a complete fresh diagnostic set.
- [ ] After a green exploratory round, run five complete `byte-backed` rounds.
      Record run order, set-boundary thermal state, and elapsed time so
      time/thermal bias remains visible.
- [ ] Preserve any setup failure under an explicit retry name. Preserve every
      measured red; never replace or cherry-pick it.
- [ ] Verify fixed binary hashes and source/tree identity across every completed
      round: ten binding rounds when both complete five-round sets run.
- [ ] Aggregate median, pooled worst, spread, signed verdicts, owner rows,
      private/mapped counters, and candidate/model checks.
- [ ] Confirm that byte-backed owner rows report `poet_bin` storage and that the
      default-owned rows do not accidentally reuse a prior process-global
      translator/cache instance.

The A/B is diagnostic. A green diagnostic does not authorize a default flip;
it selects the candidate branch for Phase 3.

## Phase 2 — Owner Reconciliation And Branch Decision

Build `owner-budget.csv` and `owner-delta.csv` from non-overlapping rows. They
must reconcile:

- process peak and after-ready/steady working set;
- Windows private bytes;
- clean/mapped file bytes where measurable;
- named heap and byte-backed owners; and
- phase transients versus retained state.

Only leaf rows explicitly classified `heap_owned_reducible` participate in the
private-owner sum. Process totals, phase totals, mapped-file rows, and the
derived `process.after_ready...unclassified_lower_bound` are not owners and
must never be ranked or added to that sum. For the same after-ready/steady
phase, define:

```text
whole_process_private_delta = owned_private - byte_backed_private
explained_heap_delta = sum(owned heap_owned_reducible leaves)
                     - sum(byte-backed heap_owned_reducible leaves)
coverage = explained_heap_delta / whole_process_private_delta
```

Here `owned_private` and `byte_backed_private` are the per-round
`process.after_ready_private_bytes_proxy` values defined above. Compute the
formula independently for each owned/byte-backed round pairing, report every
pair, and aggregate the five whole-process and explained deltas by median; do
not substitute `summary.csv`'s private envelope into this same-phase equation.

Both deltas must be positive. Require `0.80 <= coverage <= 1.20` and an absolute
residual no larger than the greater of `5,000,000 B` or `20%` of the
whole-process private delta. Mapped-file movement is reported separately with
its sign; it is never subtracted from private bytes or counted as a heap saving.
The peak-working-set win remains an independent acceptance row.

Allocator evidence, if useful, is a separate diagnostic only: run five fresh
processes per mode from the same exact source, with one frozen Track A input,
phase, tool binary/hash, and root-isolation protocol, then repeat it for the
final candidate. Never add or reconcile those test-process allocator bytes
against the release benchmark process. M61 does not depend on that optional
lane when same-process private bytes and named owners close the contract.

Before changing runtime behavior:

- [ ] Name at least `10,000,000 B` of current non-overlapping reducible memory.
- [ ] Reconcile the owned-to-byte-backed whole-process private delta under the
      formula and tolerance above without double-counting process, mapped, or
      derived residual rows.
- [ ] Verify whether the likely owner still consists of the retained preset
      vocabulary recipe, `poet.entries_by_code`, and `poet.lookup_index`, or
      record the current replacement owner.
- [ ] Prove that the proposed reduction affects same-process Windows private
      bytes rather than only clean mapped residency or page-cache luck.
- [ ] Select exactly one disposition:
  - **A — byte-backed POET re-land:** diagnostic clears the signed and behavior
    gates and projects the full win;
  - **B — one bounded scratch/access correction:** memory wins but one measured
    byte-backed owner remains, with an exact repair and retry contract;
  - **C — another current Track A owner:** only if fresh attribution disproves
    POET and names a higher-leverage non-overlapping owner; or
  - **D — measured partial/no-go:** no safe owner or win.

Allocator work is not selectable from the optional cross-process diagnostic.
M47 RED-09/10/11 work is not imported by analogy; those rows belong to a
different product profile and counter.

## Phase 3 — Conditional Production Candidate

Phase 3 exists only for disposition A, B, or C.

The production policy is native-only and capability-bound, not a global
environment default:

- a repository-owned internal native deployment policy enables POET generation
  for the supported deployed `luna_pinyin` source; it is not a public ABI flag;
- runtime consumption requires an artifact whose version plus schema/table/
  source identities validate against that deployment;
- an absent artifact preserves the owned fallback and its current behavior,
  while a present invalid, stale, or replaced artifact fails loudly;
- public browser/package deployment policies neither generate, copy, nor
  consume `.poet.bin`, and their manifest/packaging checks prove that boundary;
  and
- the shipping native path requires no inherited environment variable. The
  diagnostic selector remains benchmark tooling rather than product policy.

For the expected byte-backed branch:

- [ ] Preserve the current versioned `YUNE-POET/2` validation and loud rejection
      of present invalid/legacy artifacts. Bump the format only if bytes or
      semantics actually change.
- [ ] Keep POET artifact creation in the untimed deploy/preparation phase and
      prove runtime consumption uses the validated bytes.
- [ ] Port or retain the current incremental sentence scratch, reachability,
      lookup, and cache-invalidation behavior without reconstructing the owned
      vocabulary/entry maps.
- [ ] Make the validated native deployed Luna path production-default in the
      same candidate that passes all gates. Do not land an intermediate default
      flip with a red ratchet.
- [ ] Keep the environment switch out of the shipping requirement and record
      the actual storage mode in owner diagnostics.
- [ ] Add owning tests for cold/warm initialization, source/cache changes,
      corrupt or replaced POET artifacts, all growing prefixes of the 37/59
      rows, and process-global lifecycle clearing.
- [ ] Report every new cache or hot layer as a named bounded memory owner.

Do not delete the owned fallback or change missing-artifact/source-deploy
semantics unless the fresh audit proves that change necessary and the plan is
amended before implementation. No hidden behavior tradeoff is authorized.

## Phase 4 — Final Five-Round Acceptance

- [ ] Build the production-default candidate once and record all hashes.
- [ ] Run five complete final Windows rounds with the normal signed invocation;
      no diagnostic mode or inherited POET environment variable may be active.
- [ ] Require the memory win, unchanged signed registry, all individual rows,
      Track B guards, candidate/model identities, and owner-shape checks above.
- [ ] Run the load-bearing Rust, manifest, evidence, and documentation gates.
- [ ] If available, run a five-round exact-source macOS diagnostic outside the
      Windows acceptance packet and label RSS/absolute differences as
      platform-specific. Do not delay or redefine Windows acceptance around it.
- [ ] Publish the compact packet, obtain two independent reviews, update the
      dashboard/roadmap/requirements/decisions only for the measured result,
      and move this plan to `plans/completed/`.

## Execution And Commit Sequence

1. **Planning commit now.** Land only this draft and the roadmap update. Do not
   touch M60, requirements, decisions, thresholds, or runtime code.
2. **Finalize after M60.** Fetch the pushed M60 closeout, require local `main`
   to fast-forward to or equal `origin/main`, fill the base SHA, add planned
   requirements, record both plan reviews, and push the finalized plan before
   measurement. Stop for manual reconciliation if the branches diverge; do not
   rebase around the excluded staged config.
3. **Measurement/tooling commit.** Add only the explicit diagnostic selector or
   missing attribution needed for Phase 1. The default signed invocation must
   remain identical.
4. **External A/B and owner decision.** Preserve all raw results outside Git.
   No production code change follows a no-go.
5. **Conditional implementation commit.** Commit directly to local `main` only
   after focused correctness/owner tests pass, but do not push it before the
   exact-source acceptance set is green. Use a disposable clean detached clone
   for measurement rather than a branch or registered worktree. A measured-red
   default is preserved externally and resolved by an explicit local revert or
   fix-forward disposition before any push; do not rewrite history to hide it.
6. **Five-round final acceptance and reviews.** A completed red stops closeout.
   Both reviews name the exact implementation commit/tree and the proposed
   final evidence/documentation tree. After review, only the review receipts
   and their manifest entries may change; any other delta requires re-review.
7. **Closeout commit and push.** Curate only compact receipts, update current
   docs, move the plan, verify exact committed tree and remote identity, and
   leave `.codex/config.toml` staged and uncommitted.

## Load-Bearing Verification

The exact test names may be corrected during post-M60 finalization, but the
owning coverage cannot be reduced:

```sh
cargo test -p yune-core --test upstream_luna_pinyin_parity
cargo test -p yune-core poet
cargo test -p yune-rime-api dictionary_data
cargo test -p yune-rime-api deployment
cargo test -p yune-rime-api --test yune_web m59_luna_
cargo test -p yune-rime-api --test yune_web m59_schema_general_reachability_deployment_matrix_default_on_and_explicit_false
npm --prefix apps/yune-web run check:schema-manifest
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The full workspace gate is load-bearing at closeout because a default storage
change crosses deployment, cache lifecycle, compiled-artifact, and candidate
behavior boundaries. Do not run browser latency, Cloudflare, package, Windows
frontend, or iOS suites unless the implementation unexpectedly touches those
surfaces; such a touch normally stops M61 for scope review.

The final native performance command remains the current Windows benchmark with
the exact 17+1 inputs and `9/60/80` settings. The finalized plan must paste the
literal command after M60 so it cannot silently drift from the then-current
wrapper interface.

## Evidence Contract

Use a create-new external root such as:

```text
$HOME/yune-m61-native-track-a-memory/<source-sha>/
```

Retain full raw baseline, diagnostic, candidate, setup-failure, measured-red,
and final outputs there. The tracked compact packet belongs under:

```text
docs/reports/evidence/m61-native-track-a-memory-owner-reduction/
```

Curate only:

- `README.md` with the exact verdict and source boundary;
- provenance, commands, environment, timestamps, and binary hashes;
- owned/byte-backed/final five-round aggregate tables;
- `owner-budget.csv`, `owner-delta.csv`, and branch disposition;
- unchanged signed-gate verdict and the separate accepted one-row M61 memory
  ratchet;
- candidate/model/storage-mode checks;
- test/check summaries;
- requirement/evidence and change-isolation reviews; and
- exact packet manifest with byte size and SHA-256.

Do not check in `m37_metrics.csv`, `samples.csv`, startup traces, binaries,
compiled schemas, POET artifacts, raw benchmark trees, or screenshots. Run the
compact-evidence curator, packet-manifest verifier, current-doc link checker,
privacy scan, evidence-growth guard, and `git diff --check`. The packet remains
below the repository's `10 MiB` cap.

## Failure And Retry Policy

- A setup failure before measurement may be fixed and retried under an explicit
  `retry-N-<reason>` name. Preserve the failed setup receipt.
- A measured red is never discarded, renamed as setup, averaged away, or
  cherry-picked. It receives a source-bound disposition.
- A variable DLL or benchmark-executable hash rejects that complete evidence
  set. Preserve it and restart the full set after correction.
- An exploratory diagnostic red may select one bounded correction only when
  the owned baseline is green and the red's owner evidence names it. Skip the
  remaining byte-backed rounds. After correction, run a new complete set under
  a new source/name; the red remains in the packet.
- Any candidate/output, reachability, ordering, ABI, Track B, startup/session,
  or signed latency red prevents a default flip. No threshold rebaseline or
  waiver exists inside M61.
- If the memory win appears only in working set while same-process private bytes
  and named owners do not corroborate it, classify it as paging/mapping noise
  and close no-go.
- If success requires a browser POET payload, feature disablement, reduced
  candidates, source fallback, hidden cache, or diagnostic environment flag,
  close no-go or request a new scope decision.

## Non-Goals

- Generic cross-platform, browser, product, or iOS memory optimization.
- Same-run librime memory parity as a required M61 outcome.
- Reusing M47 or historical M55 numbers as current baselines.
- M47 RED-09 asset slimming, RED-10 allocator selection, or RED-11 keyboard
  startup hygiene without fresh Track A ownership.
- New schema assets, `.poet.bin` browser payloads, page-size reductions, or
  disabled sentence/grammar/reachability behavior.
- C ABI/API-table/export, TypeDuck-profile, schema-id, userdb, UI, package,
  Cloudflare, Windows product, or Apple-device work.
- Loosening, rewriting, or silently superseding the M59/M55 signed registry.
- Opening M62 or promising a later performance milestone.
