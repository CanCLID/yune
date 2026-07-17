# M61 Native Track A Memory-Owner Reduction

> **Milestone:** M61. **Status:** Complete — disposition D, measured
> partial/no-go (2026-07-16). **Track:** native engine performance, Windows Track A
> `luna_pinyin` acceptance lane. **Created:** 2026-07-15; **independently
> reviewed and corrected:** 2026-07-15; **source-bound finalization:**
> 2026-07-16; **Windows candidate-parity prerequisite amendment:** 2026-07-16;
> **Windows Track B prerequisite amendment:** 2026-07-16;
> **post-diagnostic disposition-B and quality-gate amendment:** 2026-07-16;
> **restored-tree test-contract corrections:** 2026-07-16.
> **Type:**
> attribution-first measurement and conditional reduction plan.

## Outcome

M61 is one narrow performance milestone: reduce the retained native
Track A `luna_pinyin` memory owner while preserving every signed M59 latency,
behavior, product-guard, and ABI boundary.

The leading hypothesis is the existing `YUNE-POET/3` byte-backed sentence-model
path. M55 measured a substantial memory reduction from that path but correctly
left it opt-in because the honest per-key long-input gate was red. Current main
now contains the later byte-backed incremental-scratch work at `759ff5d7`, plus
subsequent Luna lookup/surface changes, but no source-current Windows A/B proves
that the former latency blocker is gone. M61 therefore begins with measurement,
not a default flip.

M61 closes successfully only if all of these conditions hold:

1. the exact pushed Phase 0B source is reproduced in five fixed-binary Windows
   owned-mode rounds, its one exploratory byte-backed measured red is preserved,
   and the one disposition-B correction selected by that red is reproduced from
   a new exact clean source in five fresh fixed-binary owned-mode rounds and five
   fresh fixed-binary byte-backed diagnostic rounds;
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

### Final result

M61 closed through that valid disposition-D path. Correction source
`91f5969688a3d2dba96a67d1cfe813c7ba4ee861`, tree
`6626ed16d5e135fa477ca26e9786d11121c92b44`, completed five fixed-binary
owned rounds, one green exploratory byte-backed round, and five fixed-binary
byte-backed rounds. Both complete sets passed all `32/32` signed aggregate
rows and all `160/160` individual observations; every candidate receipt passed
`17/17`, and Track B remained green. The diagnostic byte-backed peak median was
`116,162,560 B` and its worst observation was `116,334,592 B`, below the
unaccepted `125,000,000 B` supplemental cap.

The binding owner reconciliation nevertheless completed red. The
`25,096,192 B` whole-process private delta had `18,724,242 B` of explained
named-owner delta, so coverage was `0.746098930`, below `0.80`; the
`6,371,950 B` residual also exceeded its `5,019,238 B` limit. The first
correction-source measured red exhausted disposition B. Commit
`01a62f2a6cd2b3d668545a110de8c7c3fc2fbb10` explicitly reverted the exact
three-path correction and restored tree
`f1c36a0079d85628f5cbef140bd94288930cc2e8`, byte-identical to the pushed
quality-repair runtime tree. No production-default candidate or final
production acceptance set was created, no supplemental ratchet was accepted,
and no runtime reduction remains.

Closeout verification preserved two deterministic test-contract reds rather
than misreporting the literal workspace command as green. The exact restored-
tree runner passed gates 1--19, then `cargo test --workspace` exited `101` in
`cantonese_parity` at `37/41`. A cfg(test)-only correction now makes four
all-pages oracle comparisons explicitly request the complete candidate list
while retaining the bounded initial product-page assertions. The disjoint
never-reached tail then exposed one older lifecycle-documentation assertion
that predated M56's narrow cross-thread contract; a second cfg(test)-only
correction now locks the current one-service boundary and valid-session cross-
thread tolerance without a parallel-progress guarantee. These closeout
corrections change no runtime, fixture, schema,
ranking, page size, ABI, measurement, or disposition-D result.

The final disjoint recovery passed source-current formatting and exact strict
workspace Clippy, the complete API library (`364 passed / 1 ignored`), every
previously unreached API bin and integration target (`114 passed / 3 ignored`),
and both remaining zero-test doc groups. Nonduplicated successful accounting
for the preserved prefix plus disjoint recovery is `1,184 passed / 12 ignored`.
Neither the `37` passing tests within the failed Cantonese target, the `363`
passing tests within the failed API library, nor the focused product-page test
that is also present in the full `yune_web` target is double-counted.

## Authority And Boundaries

M60 is complete and pushed. Its formal closeout commit is
`0eff06a088992f417602a71300c447cdfa525255`, with tree
`cbffa328e9ca7a1ea04187a67349d977bc731b62`. The current post-closeout
formalism correction and M61 kickoff base is
`bc0df36a6eee3ad63319d8c29336542082559c94`, with tree
`523ab0e5f3a8aa67f807a07586591c92f9ef1ead`. The latter changes M60
documentation, registry wording, and focused tests only; it does not change
production runtime behavior. M61 was the sole authorized execution milestone
after that boundary.

The separate owner request that authorized M61 satisfies M60's statement that
M61 cannot be inferred from M60. Nothing in this plan changes M60's scope,
requirements, or evidence.

Binding references:

- upstream `rime/librime 1.17.0`:
  `33e78140250125871856cdc5b42ddc6a5fcd3cd4`;
- current signed native registry:
  `docs/reports/evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv`;
- final M59 Windows performance source: `443cc636862806e4f0dd1e12ab2e2e45f4189154`;
- final M59 five-round aggregate: `32/32` rows and `160/160` individual
  observations green;
- formal M60 closeout:
  `0eff06a088992f417602a71300c447cdfa525255`, tree
  `cbffa328e9ca7a1ea04187a67349d977bc731b62`; and
- source-current M61 kickoff base:
  `bc0df36a6eee3ad63319d8c29336542082559c94`, tree
  `523ab0e5f3a8aa67f807a07586591c92f9ef1ead`.

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

A macOS-only execution can implement and review the portable tooling, but it
cannot close M61. The binding baseline, diagnostic, owner reconciliation, and
final acceptance require the Windows lane above. If that machine is unavailable,
stop with M61 still planned rather than substituting macOS RSS or historical
Windows evidence.

M61 makes no public C ABI/API-table/export change, schema/profile-id change,
oracle rebase, browser payload change, product/frontend change, Windows TSF/UI
change, Cloudflare change, or iOS-device claim. D-24, D-25, D-31, D-47, D-48,
and D-49 remain in force.

Every pre-existing staged, unstaged, or untracked user path remains excluded
from every M61 commit and measurement source claim. Phase 0 must inventory and
fingerprint the actual dirty state at kickoff; it must not assume
`.codex/config.toml` is staged or hard-code any current unrelated path.

## Planned Requirements

These IDs are planned in `requirements.md` and traceability. They remain open
until M61 closes with either an accepted reduction or an evidence-backed
partial/no-go disposition:

- **M61-BASELINE-01:** reproduce an exact-source five-round owned baseline and,
  after one green exploratory round, a five-round byte-backed diagnostic A/B
  with fixed binaries and full receipts; a preserved exploratory measured red
  may terminate this requirement as complete with measured no-go;
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

### Terminal requirement dispositions

M61 has two valid terminal shapes. A setup block before usable measurement does
not close the milestone and leaves the requirements planned.

| Requirement | Accepted disposition A/B/C | Disposition D — measured partial/no-go |
| --- | --- | --- |
| `M61-BASELINE-01` | Complete | Complete with measured no-go after a green five-round owned set and the plan-prescribed diagnostic stopping point |
| `M61-ATTR-01` | Complete | Complete with measured no-go: no safe owner branch clears the required attribution, reconciliation, and compatibility gates |
| `M61-BRANCH-01` | Complete | Complete: disposition D is selected and no production reduction remains after the explicit revert path |
| `M61-REDUCE-01` | Complete | Closed by no-go; no production reduction is claimed |
| `M61-COMPAT-01` | Complete | Complete: the green owned baseline plus explicit reverts to the pushed quality-repair runtime tree preserve the compatibility boundary |
| `M61-RATCHET-01` | Complete | Closed by no-go; the frozen supplemental ratchet remains unclaimed and the historical registry remains unchanged |
| `M61-EVIDENCE-01` | Complete | Complete, including the measured-red/no-accepted-branch disposition and both reviews |

## Acceptance Contract

### Fixed measurement shape

All baseline, diagnostic, and final acceptance rounds use:

- the exact Track A input string
  `n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru,zh,j,yi,che,chuang,b,ceshi,zhongdengchangdu,dazisudu`;
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

The initial owned/byte-backed A/B source is the exact pushed Phase 0B-corrected
M61 measurement commit `67d32a2bea36a391a8a11ea4e725dbfebe118252`
(tree `7e2157b5de2575728f2632fad184a05403342a13`), not either M60 boundary or
an earlier tooling/parity source. Its implementation parent boundary is the
source-current kickoff base `bc0df36a`; record the formal M60 closeout,
post-closeout correction, plan-finalization commit, immutable tooling commit,
Phase 0 tooling repairs, Phase 0A amendment/correction, source-clean helper
repair, POET-rebind tooling repair through
`f18b0df2d0149bc2a28cd9bd2c075c34030b5568` (tree
`e4ba5201eab8b8fd8cb24ae14dd49a8c9959aa10`), the Phase 0B plan-only
amendment, and its correction separately. After excluding plan/review changes,
the non-documentation implementation diff from `bc0df36a` may contain only the
original M61 measurement tooling, Phase 0A's stale-raw-edge correction and
owning proof, the source-clean/POET-rebind tooling repairs, and Phase 0B's exact
packed-syllabary representation correction plus owning tests. Phase 0B is a
Track B prerequisite and does not consume `M61-BRANCH-01`; no Track A POET
storage/default optimization is present there.

The preserved `67d32a2b` exploratory measured red activates the
post-diagnostic amendment below. Its correction-only local commit, not
`67d32a2b`, becomes the source for a wholly new complete owned/byte-backed A/B.
No `67d32a2b` round is reused in that replacement set. The final production
candidate descends from the accepted correction source and receives its own
exact-source five-round set.

All binding owned, byte-backed, and final sets must run on the same physical
Windows machine with the same OS build, power policy, and materially equivalent
Rust/Cargo/Visual Studio toolchain. The machine must be on AC power, quiet,
thermally stable, and free of concurrent compilation, indexing, backup, export,
or other CPU/memory-heavy work. Record start/end timestamps and significant
workloads. If the machine or environment changes materially, preserve the old
evidence and run a new complete owned baseline before making the relative
comparison.

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
   `process.owner_snapshot_private_bytes` worsens by more than `5%` in five-run
   median or pooled worst;
5. after-ready/steady and peak working set, private bytes, mapped-file bytes,
   and non-overlapping named-owner rows all remain visible; and
6. every final round also passes the unchanged `195,028,378 B` Track A memory
   row in `m55-thresholds.csv`. This is intentionally redundant with the
   tighter M61 cap and proves the existing signed registry remains green.

The same-run librime ratio is diagnostic, not an M61 pass/fail target. M61 does
not promise parity or use the historical "188 MB gap" as a target.

The measurement-tooling commit must emit these exact private-byte receipts:

- `track_a_private_envelope_bytes`: for each round, take the maximum
  `median_private_bytes` across the 17 Yune Track A key-sequence rows in
  `summary.csv`. Each key row contains 80 samples and uses the benchmark's lower
  median for an even sample count. Key workloads have no
  `after_finalize_private_bytes`, so every sample's `private_bytes` equals its
  `after_ready_private_bytes`. This is a private-byte envelope, not a counter
  sampled at peak working set.
- `process.owner_snapshot_private_bytes`: a new non-owner process row in
  `memory-owner-profile.csv`, sampled immediately before the owner JSON export
  in the same new zero-key service/session, after schema selection, and at the
  same lifecycle point as the named POET leaves. The benchmark process itself
  is already warmed; do not describe this as a cold/fresh process. This supplies
  the phase-aligned whole-process value for owner reconciliation; post-key
  `after_ready_private_bytes` must not be substituted.

For either receipt, the five-run median is the median of the five per-round
values and pooled worst is their maximum. Never label either receipt
"peak private bytes."

The `20%` relative bar is a deliberately material reduction, well above the
historical five-round Track A spread, rather than a claim about an inherited
baseline. The independent `125,000,000 B` cap is a conservative absolute bound
informed by the byte-backed range already demonstrated in M55, not a reuse of
M55 as current evidence. Both values are frozen by this finalized plan, before
the first accepted M61 baseline. They cannot be adjusted, rebaselined, or waived
after measurement inside M61. The relative bar is stricter when the fresh
baseline is below `156,250,000 B`; above that point the absolute cap is stricter.
Both always apply.

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

Before the first accepted M61 measurement, freeze a separate M61 memory-only
ratchet containing one Track A peak-working-set row at the predeclared
`125,000,000 B` ceiling plus source/provenance fields. Preserve its SHA-256
externally, then copy that exact file into the closeout packet at
`docs/reports/evidence/m61-native-track-a-memory-owner-reduction/m61-memory-threshold.csv`.
It must pass alongside the existing signed registry; it does not copy, replace,
or supersede that registry. Never rewrite the historical signed registry,
loosen another row, or call the new ratchet authoritative before final owner
approval.

The current aggregator cannot evaluate that file in a second ordinary
`--thresholds` invocation: it derives the expected Track A input set and run
ceiling identities from the primary registry. The measurement/tooling commit
must therefore add and test these paired, fail-closed arguments to
`scripts/aggregate-native-ratchet.py`:

- `--supplemental-thresholds <path>`; and
- `--supplemental-output <path>`.

They must be supplied together and may appear only once. The interface reuses
the same five loaded observations, requires a matching metric key/unit and a
strictly tighter ceiling, leaves the primary expected-input set and verdict
untouched, requires all five observations at or below the supplemental ceiling,
requires `worst_observed <= 125000000` and `individual_failures == 0`, and emits
the distinct supplemental CSV plus its own provenance sidecar. The primary
verdict, primary sidecar, supplemental verdict, and supplemental sidecar form
one atomic fail-closed output set: any validation or write failure removes or
invalidates all four. Do not edit or replace the historical threshold path.

The frozen supplemental file is UTF-8 without BOM, LF-terminated, and contains
exactly:

```csv
"kind","workload","input","metric","ceiling","unit","source_value","spread_pct","notes"
"memory_peak","","","track_a_peak_working_set_bytes","125000000","bytes","125000000","0","M61 predeclared supplemental Track A pooled-worst cap; frozen before accepted measurement; does not replace m55-thresholds.csv."
```

Its SHA-256 is
`d52d064f410df36c1c22dd5523430062563a17bb9f2f63253b607d211badefd7`.
The executing session must recreate and verify those exact bytes externally
before the first accepted baseline, then copy that unchanged file into the
closeout packet only after an accepted result.

The owned baseline is evaluated only against the historical registry. The
supplemental cap is expected to reject that baseline and therefore is not
invoked there. It is applied to the five-round byte-backed diagnostic as a
projection and to the final production-default set as a binding acceptance
gate. The complete Windows blocks below recreate and hash-check the threshold,
define every set path, invoke both aggregate forms, check the native process
exit code, and preserve a nonzero result as a measured red. No abbreviated
aggregate invocation is authoritative.

## Phase 0 — Finalization And Execution Kickoff

- [x] Bind the formal pushed M60 closeout and the source-current post-closeout
      correction separately.
- [x] Re-audit the roadmap, performance dashboard, M60 closeout, current
      `YUNE-POET/3` code, benchmark wrappers, and threshold registry.
- [x] Confirm no post-review change already moved the owner or invalidated the
      branch hypothesis.
- [x] Convert the proposed requirement IDs into planned rows after review.
- [x] Freeze the benchmark selector names, supplemental-ratchet interface,
      exact inputs, threshold bars, and literal command shape before
      measurement.
- [x] Run two finalization reviews: evidence/measurement validity, then
      scope/isolation and threshold safety.
- [x] At execution kickoff, record the exact plan-finalization and
      measurement-tooling commit chain, final path allowlist, external evidence
      root, Windows toolchain/machine identity, power/thermal state, significant
      workloads, and pinned oracle/artifact hashes.
- [x] Inventory and fingerprint every unrelated staged, unstaged, and untracked
      path at kickoff. Instantiate the corrected M60 isolated-index,
      pre-review-tree, exact-review-delta, path-limited-commit, commit-tree, and
      remote-equality procedure with M61 filenames; do not retain the old
      config-only staged-path assertion if the actual inventory differs.

If the pushed finalization source does not descend from the kickoff base, the
Phase 0 dirty-state inventory contains any unexplained or M61-overlapping path,
the current benchmark no longer reads context per key, the Windows acceptance
machine is unavailable, or the relevant owner cannot be measured, stop before
M61 measurement.

### Phase 0A — Windows Candidate-Parity Prerequisite Amendment

The original M61 measurement-tooling boundary remains immutable:

- `91b8991c5668ace690a4f6775bd8d91dfc0696f9`, tree
  `9cd2527e8cf07e78ab8ea2bb9eaaa569056c4662`.

The following pushed descendants are Phase 0 tooling-only repairs, not accepted
measurement rounds and not production-behavior sources:

- `6a1cbed7d40ad06ec08588772b2d7d18a18a5788`, tree
  `265bedf9278181acc54812210e3f143838c69f89`, fixes the PowerShell 5.1
  present-empty environment test without changing the wrapper; and
- `7805882d93428db0a3791b0631290ab319b524f0`, tree
  `c198d23fc6777ad8b119e30552980243a6acdbb6`, accepts unique supplemental
  owned POET owner rows while retaining the exact byte-backed four-row contract.

All five Windows tooling suites are green on clean pushed `7805882d` under
Windows PowerShell `5.1.26100.8875`. The fresh Windows preflight at that exact
source completed the native wrapper and strict candidate-shape validation but
failed behavior parity: `16/17` inputs were exact, with `zhongdengchangdu` the
only mismatch. Pinned librime returned
`中等長度 | 中等 | 中 | 種 | 重`; Yune returned
`中等長度 | 中等 | 中的 | 種的 | 重的`. Candidate count, page metadata,
highlight, composition preedit, comments, and the other sixteen input pages
matched. The wrapper used diagnostic `1/1/1` cadence, explicit `owned` mode,
and no Track B or threshold gate. This is a preserved correctness red, not a
setup failure, accepted round, or disposition D.

That preflight is bound to:

- librime source commit
  `33e78140250125871856cdc5b42ddc6a5fcd3cd4`, tree
  `5758d9896d7dd5c2d5316e595a91612f72a0432e`;
- official upstream `rime.dll` SHA-256
  `86b4c7357d4c6d293ce5589b234d8859ca2ac30923a03bedfa3926eeaf97fb0b`;
- upstream shared-tree SHA-256
  `3801c4c83ba919e531b80ac27e2c06d116d08b19af2034fcb86e6e17ae1eecf6`;
- freshly generated upstream build-tree SHA-256
  `446c90b2f4ffd76b4ec1f4ecca4f534c986e72e3d8803c6998926d0b1cebbf17`;
- product-schema-tree SHA-256
  `0bc042c0ab09c732419cf6ba5ce008390e87894c7d374c0d1b44efeac10a9bf0`;
- measured Yune DLL SHA-256
  `c6fc3b1594ae98771a39c784953236c3ba0e9873602c056dfcb4c9d9631e329d`;
  and
- native benchmark executable SHA-256
  `a4e6f9ccb1b090bfeb3756090e44ffb84b6fd15e235b2d0f9021b1acc1abae87`.

The fresh build tree is the exact generated M61 oracle identity. It is not
asserted byte-identical to M59's historical generated tree
`7f8ce0b50e8acb3d5e66db55fb17879073e5be05a3a7cdc582745fe1e73bf39c`:
fresh deployment regenerates schema YAML `__build_info` source timestamps while
the pinned librime source, official DLL, and shared source tree remain fixed.
The corrected preflight and every accepted round must retain the exact shared
and build identities above. A different identity is a setup failure unless a
new create-new deployment is preserved and independently reviewed before any
measurement.

Owner authorization is limited to one engine-side structural correction in
`upstream_script_surface_segmentation`: after the valid reverse-good syllable
graph is pruned, do not restore a stale raw-identity spelling edge into direct
sentence/recomposition surfaces. Genuinely transformed inverse-overlap choices
remain eligible, and every viable edge already present in the pruned graph
remains unchanged. This is a class predicate, never an input or schema allowlist.
Preserve the other sixteen pages, the existing transformed-overlap control,
37/59 Luna page guards, reachability, ranking, selection/recomposition,
lifecycle, model checksums, ABI, and Track B behavior. No rank weight, threshold,
POET storage/default, memory owner, or other runtime change is authorized. If a
narrow correction cannot satisfy that boundary, stop and request a new scope
decision.

The correction commit may contain only that structural correction, an
externally sourced pinned-librime fixture and non-circular deployed Luna test,
owned and byte-backed structural coverage, and the fail-closed candidate
comparator plus owning wrapper/aggregator integration and tests. The tracked
`scripts/check-native-candidate-parity.py` must:

- require the exact frozen seventeen Track A inputs and exactly one page-zero
  row set from each of `librime-1.17.0` and `yune`;
- reject missing, extra, duplicate, malformed, or non-contiguous rows;
- compare exactly `candidate_index`, `candidate_count`, `page_size`, `page_no`,
  `is_last_page`, `highlighted_index`, `composition_preedit`, `text`, and
  `comment`;
- emit deterministic `candidate-parity.csv`,
  `zhongdengchangdu-detail.csv`, and `candidate-parity-verdict.txt`;
- exit nonzero unless all `17/17` pages are exact; and
- run inside both native wrappers before a round can complete, with the
  five-round aggregator requiring and hashing each PASS receipt.

After this docs-only amendment and its two reviews are committed and pushed,
make the correction in a separate commit, obtain two independent correction
reviews, push it, and repeat the complete seventeen-input Windows preflight.
Only a clean pushed correction commit with a `17/17` PASS becomes
`$EXPECTED_MEASUREMENT_SHA`. The owned and byte-backed accepted sets use that
exact SHA/tree. Every later production candidate must descend from it and
retain the identical correction/comparator contract. No accepted mode may use
`91b8991c`, `6a1cbed7`, `7805882d`, or a divergent lineage.

The exact `17 + 1` inputs, `9/60/80` cadence, product deployment, M55 registry,
M61 `125,000,000 B` supplemental threshold, memory bars, fixed-binary rules,
and failure/retry policy remain unchanged. A repeated parity red blocks M61; it
cannot be waived, reclassified, or converted to disposition D.

### Phase 0B — Windows Track B Session-Private Prerequisite Amendment

Clean pushed source `f18b0df2d0149bc2a28cd9bd2c075c34030b5568`, tree
`e4ba5201eab8b8fd8cb24ae14dd49a8c9959aa10`, corrected the benchmark's restored
Luna-table/POET checksum binding and became the next diagnostic source. Its
fresh owned set stopped exactly as required:

- `owned/run-1` and `owned/run-2` completed with `32/32` signed rows green and
  `17/17` candidate parity;
- `owned/run-3` retained `17/17` candidate parity and passed the other `31/32`
  signed rows, but
  `track-b-product/session_create_select_destroy median_private_bytes` measured
  `32,727,040 B` against the unchanged `32,084,378 B` ceiling, an excess of
  `642,662 B` (about `2.0%`); and
- all 60 session observations are present and independently reproduce that
  median. The source-bound blocker receipt is retained externally at
  `C:\yune-m61\f18b0df2d0149bc2a28cd9bd2c075c34030b5568\BLOCKER.md`, SHA-256
  `bbb0a2649c3243c2680edc7d8e259a531a36e8150d508094bab6b843fd13ea4f`.

This is a measured red, not a setup failure or disposition D. Those three
rounds are immutable and unaccepted: never append runs 4–5, reuse runs 1–2,
rerun `f18b0df2` as acceptance, average the red away, or change the signed
registry.

The owner-authorized prerequisite correction is exact. The two Track B
`compact_table.syllabary_codes` rows retain `4,850,892 B` across `134,628`
codes as `Vec<String>`; the corresponding Track A row is only `11,573 B` across
424 codes. Replace that per-string representation with a class-general packed
canonical-code sequence: one contiguous valid UTF-8 buffer plus monotonic
`u32` offsets, borrowed `&str` indexed/iterated access, exact source order,
duplicates, empty-code, and Unicode-boundary preservation, and fail-closed
aggregate-length conversion. Keep prism lookups monomorphized and allocation-
free. Update the existing owner row to count the packed buffer and offset
allocation honestly with the unchanged item count. The exact owning regression
test is
`packed_syllabary_codes_preserve_order_boundaries_and_owner_accounting`.

The correction may touch only these production paths plus tests within them:

```text
crates/yune-core/src/dictionary/compiled_prism.rs
crates/yune-core/src/dictionary/compiled_table.rs
crates/yune-core/src/dictionary/prism_writer.rs
crates/yune-core/src/translator/mod.rs
crates/yune-core/src/translator/reverse_graph.rs
crates/yune-rime-api/src/schema_install.rs
```

The correction is representation-only. It preserves candidate text, comments,
order, reachability, compiled artifact formats, D-32 warm immutable-asset
semantics, session lifecycle, cache invalidation, ABI, Track A behavior, and all
Track B behavior/latency guards. No benchmark, wrapper, comparator, aggregator,
threshold, cadence, evidence tool, schema asset, POET storage/default, input or
schema allowlist, process trimming, sleep, sample timing, benchmark-only cleanup,
or process-global cache change is authorized. If packed borrowed access cannot
satisfy this boundary, stop for another scope decision rather than substituting
another owner.

Commit this plan-only amendment after two independent reviews. Then implement
the packed owner correction separately, freeze its exact pre-review tree, add
only its two review receipts after review, commit and push it, and prove remote
equality. The correction commit's parent must be exactly the Phase 0B plan-only
amendment commit. From a new clean detached clone and create-new evidence root,
repeat the complete `17 + 1` preflight and five wholly new fixed-binary owned
rounds. All five rounds must pass `32/32`; the `f18b0df2` partial set contributes
no round. Any new-source red blocks M61 and exhausts this single prerequisite
exception. In that case, immediately create and push a separate exact revert of
the Phase 0B correction, prove remote equality at the revert commit, preserve
both commits and all red evidence, and stop; an unaccepted prerequisite
correction must not remain on `main`. Only a fully green replacement set may
proceed to the existing exploratory byte-backed/A-B sequence. This prerequisite
does not consume the one later evidence-selected Track A branch authorized by
`M61-BRANCH-01`.

## Phase 1 — Fresh Baseline And Diagnostic A/B

### Benchmark mode support

The current wrappers intentionally require default-owned runtime measurement
while using byte-backed mode only during deploy preparation. M61 may add one
explicit diagnostic-only mode selector to both native wrappers:

- PowerShell parameter: `-TrackAStorageMode`;
- macOS option: `--track-a-storage-mode`;
- default value: `production-default`;
- diagnostic values: `owned` and `byte-backed`;
- recorded as `track_a_storage_mode` in `environment.txt`, commands, the owner
  profile, and the aggregator's strict provenance identity;
- rejected when combined with an ambiguous inherited environment variable;
- no effect on the existing signed invocation when omitted; and
- never accepted as proof of the final shipping default.

The selector must not skip deployment, alter inputs/iterations, loosen a gate,
or choose different candidate behavior. The parity-corrected measurement commit
keeps the omitted/default owner assertion at the current owned/no-`poet_bin`
state; only a later accepted production-candidate commit may change the normal
signed assertion to require validated `poet_bin` storage.

For `byte-backed`, the wrapper may set `YUNE_POET_BYTE_BACKED=1` only around
the Track A Yune timing subprocess. It must restore the prior environment before
the same-run librime comparison and Track B product lane. The existing
deploy-preparation scope remains separate and must continue rejecting an
ambiguous inherited value. Unit tests must prove restoration on success and
failure so the selector cannot contaminate another lane.

The aggregator currently requires run 1 to build the benchmark and runs 2–5 to
reuse it. M61 uses one executable across both diagnostic modes, so the tooling
commit must accept exactly two fixed-binary set shapes:

- run 1 builds once and runs 2–5 reuse its exact executable and receipt; or
- all five runs reuse the exact same source-bound executable and receipt built
  by the preceding accepted mode.

Mixed reuse identities, more than one build, five runs without one identical
receipt, or any DLL/executable hash drift remain hard errors. Mode provenance
must be uniform within a set and must match the requested aggregate lane.

Conditionalize, but never delete, the current post-aggregation Track A owner
assertion in both `scripts/benchmark-native-rime-inprocess.ps1` and
`scripts/benchmark-native-rime-inprocess-macos.sh`:

| Selector/source state | Required measured owner shape |
| --- | --- |
| `owned` | no `mapping_mode` beginning `poet_bin:`; owned POET owners remain visible |
| `byte-backed` | `poet.entries_by_code`, `poet.prefix_index`, `poet.vocabulary`, and `poet.abbreviation_vocabulary` all report `poet_bin:byte_backed:mmap`; no retained owned `poet.lookup_index` |
| `production-default` before the accepted flip | same as `owned` |
| `production-default` in accepted disposition A or B | same as `byte-backed` |
| `production-default` in accepted disposition C | same as `owned`; the separately selected non-POET owner must show its own reduced, explicitly reviewed shape |

For byte-backed mode, the `poet.*` owner-id set must be exactly those four rows,
once each; reject duplicates, missing rows, or any fifth POET owner.
Record the selector and asserted shape in every wrapper receipt, and record the
selected disposition in the aggregate and branch-disposition receipts.
Disposition C cannot silently force or claim a POET default flip after fresh
attribution disproves POET as the selected owner. This tooling change is not a
threshold waiver and remains separate from the Phase 3 shipping-default
transition.

### Binding Windows command shape

The executing Windows session must substitute only the source-bound absolute
roots and SHAs below. The inputs, `9/60/80` settings, product deployment,
historical threshold path, regression behavior, and selector values are frozen:

```powershell
$ErrorActionPreference = "Stop"
$REPO = (Resolve-Path .).Path
$IMMUTABLE_TOOLING_SHA = "91b8991c5668ace690a4f6775bd8d91dfc0696f9"
$PHASE0_ENV_TEST_SHA = "6a1cbed7d40ad06ec08588772b2d7d18a18a5788"
$PHASE0_OWNER_SHAPE_SHA = "7805882d93428db0a3791b0631290ab319b524f0"
$PHASE0_SOURCE_CLEAN_SHA = "a39c4d868820063dc3deaa42f7fdc9b3aee5e7a6"
$PHASE0_POET_REBIND_SHA = "f18b0df2d0149bc2a28cd9bd2c075c34030b5568"
$PHASE0_TRACK_B_AMENDMENT_SHA = "10584514d1870dc0a3e41e95e97258128ed03b60"
$PHASE0_TRACK_B_CORRECTION_SHA = "67d32a2bea36a391a8a11ea4e725dbfebe118252"
$POST_DIAGNOSTIC_AMENDMENT_SHA = "cfdbca0d86690b904a153e980506013f79245138"
$QUALITY_GATE_REPAIR_SHA = "931c7c59d6d471c69b70dc0d2f082149665a4e68"
$QUALITY_GATE_REPAIR_TREE = "f1c36a0079d85628f5cbef140bd94288930cc2e8"
$DISPOSITION_B_CORRECTION_SHA = "91f5969688a3d2dba96a67d1cfe813c7ba4ee861"
$DISPOSITION_B_CORRECTION_TREE = "6626ed16d5e135fa477ca26e9786d11121c92b44"
$EXPECTED_MEASUREMENT_SHA = $DISPOSITION_B_CORRECTION_SHA
$EXPECTED_MEASUREMENT_TREE = $DISPOSITION_B_CORRECTION_TREE
$EXPECTED_LIBRIME_SHA = "33e78140250125871856cdc5b42ddc6a5fcd3cd4"
$MEASUREMENT_SHA = (& git -C $REPO rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $MEASUREMENT_SHA -ne $EXPECTED_MEASUREMENT_SHA) {
    throw "measurement source mismatch: expected $EXPECTED_MEASUREMENT_SHA, got $MEASUREMENT_SHA"
}
$MEASUREMENT_TREE = (& git -C $REPO rev-parse "HEAD^{tree}").Trim()
if ($LASTEXITCODE -ne 0 -or $MEASUREMENT_TREE -ne $EXPECTED_MEASUREMENT_TREE) {
    throw "measurement tree mismatch: expected $EXPECTED_MEASUREMENT_TREE, got $MEASUREMENT_TREE"
}
foreach ($ANCESTOR in @(
    $IMMUTABLE_TOOLING_SHA,
    $PHASE0_ENV_TEST_SHA,
    $PHASE0_OWNER_SHAPE_SHA,
    $PHASE0_SOURCE_CLEAN_SHA,
    $PHASE0_POET_REBIND_SHA,
    $PHASE0_TRACK_B_AMENDMENT_SHA,
    $PHASE0_TRACK_B_CORRECTION_SHA,
    $POST_DIAGNOSTIC_AMENDMENT_SHA,
    $QUALITY_GATE_REPAIR_SHA,
    $DISPOSITION_B_CORRECTION_SHA
)) {
    & git -C $REPO merge-base --is-ancestor $ANCESTOR $MEASUREMENT_SHA
    if ($LASTEXITCODE -ne 0) {
        throw "measurement source does not descend from required M61 boundary: $ANCESTOR"
    }
}
$TRACK_B_CORRECTION_PARENT = (& git -C $REPO rev-parse "$PHASE0_TRACK_B_CORRECTION_SHA^").Trim()
if ($LASTEXITCODE -ne 0 -or $TRACK_B_CORRECTION_PARENT -ne $PHASE0_TRACK_B_AMENDMENT_SHA) {
    throw "Track B correction parent mismatch: expected $PHASE0_TRACK_B_AMENDMENT_SHA, got $TRACK_B_CORRECTION_PARENT"
}
$POST_DIAGNOSTIC_PARENT = (& git -C $REPO rev-parse "$POST_DIAGNOSTIC_AMENDMENT_SHA^").Trim()
if ($LASTEXITCODE -ne 0 -or $POST_DIAGNOSTIC_PARENT -ne $PHASE0_TRACK_B_CORRECTION_SHA) {
    throw "post-diagnostic amendment parent mismatch: expected $PHASE0_TRACK_B_CORRECTION_SHA, got $POST_DIAGNOSTIC_PARENT"
}
$QUALITY_GATE_REPAIR_PARENT = (& git -C $REPO rev-parse "$QUALITY_GATE_REPAIR_SHA^").Trim()
if ($LASTEXITCODE -ne 0 -or $QUALITY_GATE_REPAIR_PARENT -ne $POST_DIAGNOSTIC_AMENDMENT_SHA) {
    throw "quality-gate repair parent mismatch: expected $POST_DIAGNOSTIC_AMENDMENT_SHA, got $QUALITY_GATE_REPAIR_PARENT"
}
$ACTUAL_QUALITY_GATE_REPAIR_TREE = (& git -C $REPO rev-parse "$QUALITY_GATE_REPAIR_SHA^{tree}").Trim()
if ($LASTEXITCODE -ne 0 -or $ACTUAL_QUALITY_GATE_REPAIR_TREE -ne $QUALITY_GATE_REPAIR_TREE) {
    throw "quality-gate repair tree mismatch: expected $QUALITY_GATE_REPAIR_TREE, got $ACTUAL_QUALITY_GATE_REPAIR_TREE"
}
$DISPOSITION_B_CORRECTION_PARENT = (& git -C $REPO rev-parse "$DISPOSITION_B_CORRECTION_SHA^").Trim()
if ($LASTEXITCODE -ne 0 -or $DISPOSITION_B_CORRECTION_PARENT -ne $QUALITY_GATE_REPAIR_SHA) {
    throw "disposition-B correction parent mismatch: expected $QUALITY_GATE_REPAIR_SHA, got $DISPOSITION_B_CORRECTION_PARENT"
}
$MEASUREMENT_STATUS = @(
    & git -C $REPO status --porcelain=v1 --untracked-files=all
)
if ($LASTEXITCODE -ne 0 -or $MEASUREMENT_STATUS.Count -ne 0) {
    throw "measurement-tooling clone must be clean"
}
if (Test-Path Env:YUNE_POET_BYTE_BACKED) {
    throw "YUNE_POET_BYTE_BACKED must be absent before diagnostic selection"
}
$OUT = "C:\yune-m61\$MEASUREMENT_SHA"
if (Test-Path -LiteralPath $OUT) {
    throw "measurement output root already exists: $OUT"
}
$LIBRIME_SOURCE = "C:\absolute\path\to\pinned-librime-source"
$LIBRIME = "C:\absolute\path\to\prepared-pinned-librime-oracle"
$ACTUAL_LIBRIME_SHA = (& git -C $LIBRIME_SOURCE rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $ACTUAL_LIBRIME_SHA -ne $EXPECTED_LIBRIME_SHA) {
    throw "librime source mismatch: expected $EXPECTED_LIBRIME_SHA, got $ACTUAL_LIBRIME_SHA"
}
$LIBRIME_STATUS = @(
    & git -C $LIBRIME_SOURCE status --porcelain=v1 --untracked-files=all
)
if ($LASTEXITCODE -ne 0 -or $LIBRIME_STATUS.Count -ne 0) {
    throw "pinned librime source must be clean"
}
$PRODUCT = Join-Path $REPO "apps\yune-web\public\schema"
$YUNE_DLL = Join-Path $REPO "target\release\yune_rime_api.dll"
$THRESHOLDS = Join-Path $REPO "docs\reports\evidence\m55-native-match-or-beat\thresholds\m55-thresholds.csv"
$TRACK_A = "n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru,zh,j,yi,che,chuang,b,ceshi,zhongdengchangdu,dazisudu"
$TRACK_B = "neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung"
$M61_THRESHOLD_SHA256 = "d52d064f410df36c1c22dd5523430062563a17bb9f2f63253b607d211badefd7"

New-Item -ItemType Directory -Path $OUT | Out-Null
$OWNED = Join-Path $OUT "owned"
$BYTE_BACKED_EXPLORATORY = Join-Path $OUT "byte-backed-exploratory"
$BYTE_BACKED = Join-Path $OUT "byte-backed"
$WORK = Join-Path $OUT "work"
New-Item -ItemType Directory -Path `
    $OWNED, $BYTE_BACKED_EXPLORATORY, $BYTE_BACKED, $WORK | Out-Null
$M61_THRESHOLD = Join-Path $OUT "m61-memory-threshold.csv"
$M61_THRESHOLD_LINES = @(
    '"kind","workload","input","metric","ceiling","unit","source_value","spread_pct","notes"',
    '"memory_peak","","","track_a_peak_working_set_bytes","125000000","bytes","125000000","0","M61 predeclared supplemental Track A pooled-worst cap; frozen before accepted measurement; does not replace m55-thresholds.csv."'
)
$UTF8_NO_BOM = New-Object System.Text.UTF8Encoding -ArgumentList $false
[System.IO.File]::WriteAllText(
    $M61_THRESHOLD,
    (($M61_THRESHOLD_LINES -join "`n") + "`n"),
    $UTF8_NO_BOM
)
$ACTUAL_THRESHOLD_SHA256 = (
    Get-FileHash -Algorithm SHA256 -LiteralPath $M61_THRESHOLD
).Hash.ToLowerInvariant()
if ($ACTUAL_THRESHOLD_SHA256 -ne $M61_THRESHOLD_SHA256) {
    throw "M61 supplemental threshold hash mismatch: $ACTUAL_THRESHOLD_SHA256"
}

function Invoke-M61Round {
    param(
        [Parameter(Mandatory = $true)][string]$Mode,
        [Parameter(Mandatory = $true)][string]$Name,
        [string]$BenchmarkExecutable = "",
        [string]$BenchmarkReceipt = ""
    )
    $RoundOutput = Join-Path $OUT $Name
    $RoundWork = Join-Path $OUT ("work\" + ($Name -replace "[\\/]", "-"))
    $Args = @(
        "-NoProfile", "-ExecutionPolicy", "Bypass",
        "-File", (Join-Path $REPO "scripts\benchmark-native-rime-inprocess.ps1"),
        "-OutputRoot", $RoundOutput,
        "-WorkRoot", $RoundWork,
        "-UpstreamOracleRoot", $LIBRIME,
        "-ProductSchemaRoot", $PRODUCT,
        "-YuneDll", $YUNE_DLL,
        "-Iterations", "9",
        "-SessionIterations", "60",
        "-KeyIterations", "80",
        "-TrackAInputs", $TRACK_A,
        "-TrackBInputs", $TRACK_B,
        "-DeployProductBeforeBenchmark",
        "-TrackAThresholds", $THRESHOLDS,
        "-FailOnRegression"
    )
    if ($Mode -ne "production-default") {
        $Args += @("-TrackAStorageMode", $Mode)
    }
    if ($BenchmarkExecutable -or $BenchmarkReceipt) {
        if (-not $BenchmarkExecutable -or -not $BenchmarkReceipt) {
            throw "prebuilt benchmark executable and receipt must be supplied together"
        }
        $Args += @(
            "-PrebuiltNativeBenchmarkExecutable", $BenchmarkExecutable,
            "-PrebuiltNativeBenchmarkReceipt", $BenchmarkReceipt
        )
    }
    & powershell @Args
    if ($LASTEXITCODE -ne 0) {
        throw "M61 round failed: $Name"
    }
}

function Read-EnvironmentValue {
    param([string]$Path, [string]$Key)
    $Prefix = "$Key="
    $Rows = @(Get-Content -LiteralPath $Path | Where-Object {
        $_.StartsWith($Prefix, [System.StringComparison]::Ordinal)
    })
    if ($Rows.Count -ne 1) {
        throw "$Path must contain exactly one $Key row"
    }
    return $Rows[0].Substring($Prefix.Length)
}

function Invoke-M61Aggregate {
    param(
        [Parameter(Mandatory = $true)][string]$SetRoot,
        [switch]$Supplemental
    )
    $AggregateArgs = @(
        "-B", (Join-Path $REPO "scripts\aggregate-native-ratchet.py"),
        "--thresholds", $THRESHOLDS,
        "--expected-runs", "5",
        "--run", (Join-Path $SetRoot "run-1"),
        "--run", (Join-Path $SetRoot "run-2"),
        "--run", (Join-Path $SetRoot "run-3"),
        "--run", (Join-Path $SetRoot "run-4"),
        "--run", (Join-Path $SetRoot "run-5"),
        "--output", (Join-Path $SetRoot "gate-verdict.csv")
    )
    if ($Supplemental) {
        $AggregateArgs += @(
            "--supplemental-thresholds", $M61_THRESHOLD,
            "--supplemental-output",
            (Join-Path $SetRoot "m61-memory-verdict.csv")
        )
    }
    & python @AggregateArgs
    if ($LASTEXITCODE -ne 0) {
        throw "M61 aggregate failed and must be preserved: $SetRoot"
    }
}

cargo build --release -p yune-rime-api
if ($LASTEXITCODE -ne 0) { throw "Yune release build failed" }
Get-FileHash -Algorithm SHA256 -LiteralPath $YUNE_DLL

# The first owned round builds the benchmark once.
Invoke-M61Round -Mode "owned" -Name "owned\run-1"
$BENCH_EXE = Read-EnvironmentValue `
    (Join-Path $OUT "owned\run-1\environment.txt") `
    "native_benchmark_executable"
$BENCH_RECEIPT = Join-Path $OUT "owned\run-1\native-benchmark-build-receipt.txt"
2..5 | ForEach-Object {
    Invoke-M61Round -Mode "owned" -Name "owned\run-$_" `
        -BenchmarkExecutable $BENCH_EXE -BenchmarkReceipt $BENCH_RECEIPT
}
Invoke-M61Aggregate -SetRoot $OWNED

# Preserve this exploratory measurement separately. Continue only if green.
Invoke-M61Round -Mode "byte-backed" -Name "byte-backed-exploratory\run-1" `
    -BenchmarkExecutable $BENCH_EXE -BenchmarkReceipt $BENCH_RECEIPT

1..5 | ForEach-Object {
    Invoke-M61Round -Mode "byte-backed" -Name "byte-backed\run-$_" `
        -BenchmarkExecutable $BENCH_EXE -BenchmarkReceipt $BENCH_RECEIPT
}
Invoke-M61Aggregate -SetRoot $BYTE_BACKED -Supplemental
```

Disposition D created no production candidate, so the candidate SHA/tree and
final-output placeholders in the following preserved command template are
intentionally uninstantiated. For an accepted production candidate, the
executing session would start a fresh PowerShell process in a new clean
detached clone and use this complete block. It deliberately rebinds every source
and root, builds the candidate DLL and benchmark anew, rejects the diagnostic
environment variable, and never supplies `-TrackAStorageMode`:

```powershell
$ErrorActionPreference = "Stop"
$REPO = (Resolve-Path .).Path
$EXPECTED_CANDIDATE_SHA = "<local M61 production-candidate SHA>"
$EXPECTED_CANDIDATE_TREE = "<local M61 production-candidate tree>"
$EXPECTED_QUALITY_GATE_REPAIR_SHA = "931c7c59d6d471c69b70dc0d2f082149665a4e68"
$EXPECTED_DISPOSITION_B_CORRECTION_SHA = "91f5969688a3d2dba96a67d1cfe813c7ba4ee861"
$EXPECTED_LIBRIME_SHA = "33e78140250125871856cdc5b42ddc6a5fcd3cd4"
$CANDIDATE_SHA = (& git -C $REPO rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $CANDIDATE_SHA -ne $EXPECTED_CANDIDATE_SHA) {
    throw "candidate source mismatch: expected $EXPECTED_CANDIDATE_SHA, got $CANDIDATE_SHA"
}
$CANDIDATE_TREE = (& git -C $REPO rev-parse "HEAD^{tree}").Trim()
if ($LASTEXITCODE -ne 0 -or $CANDIDATE_TREE -ne $EXPECTED_CANDIDATE_TREE) {
    throw "candidate tree mismatch: expected $EXPECTED_CANDIDATE_TREE, got $CANDIDATE_TREE"
}
$DISPOSITION_B_PARENT = (& git -C $REPO rev-parse "$EXPECTED_DISPOSITION_B_CORRECTION_SHA^").Trim()
if ($LASTEXITCODE -ne 0 -or $DISPOSITION_B_PARENT -ne $EXPECTED_QUALITY_GATE_REPAIR_SHA) {
    throw "disposition-B parent mismatch: expected $EXPECTED_QUALITY_GATE_REPAIR_SHA, got $DISPOSITION_B_PARENT"
}
$CANDIDATE_PARENT = (& git -C $REPO rev-parse "$EXPECTED_CANDIDATE_SHA^").Trim()
if ($LASTEXITCODE -ne 0 -or $CANDIDATE_PARENT -ne $EXPECTED_DISPOSITION_B_CORRECTION_SHA) {
    throw "production candidate parent mismatch: expected $EXPECTED_DISPOSITION_B_CORRECTION_SHA, got $CANDIDATE_PARENT"
}
foreach ($ANCESTOR in @(
    $EXPECTED_QUALITY_GATE_REPAIR_SHA,
    $EXPECTED_DISPOSITION_B_CORRECTION_SHA
)) {
    & git -C $REPO merge-base --is-ancestor $ANCESTOR $CANDIDATE_SHA
    if ($LASTEXITCODE -ne 0) {
        throw "production candidate does not descend from required M61 boundary: $ANCESTOR"
    }
}
$CANDIDATE_STATUS = @(& git -C $REPO status --porcelain=v1 --untracked-files=all)
if ($LASTEXITCODE -ne 0 -or $CANDIDATE_STATUS.Count -ne 0) {
    throw "production-candidate clone must be clean"
}
if (Test-Path Env:YUNE_POET_BYTE_BACKED) {
    throw "YUNE_POET_BYTE_BACKED must be absent for production acceptance"
}

$OUT = "C:\yune-m61\$CANDIDATE_SHA-final"
if (Test-Path -LiteralPath $OUT) {
    throw "final output root already exists: $OUT"
}
New-Item -ItemType Directory -Path $OUT | Out-Null
$FINAL = Join-Path $OUT "final"
$WORK = Join-Path $OUT "work"
New-Item -ItemType Directory -Path $FINAL, $WORK | Out-Null
$LIBRIME_SOURCE = "C:\absolute\path\to\pinned-librime-source"
$LIBRIME = "C:\absolute\path\to\prepared-pinned-librime-oracle"
$ACTUAL_LIBRIME_SHA = (& git -C $LIBRIME_SOURCE rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $ACTUAL_LIBRIME_SHA -ne $EXPECTED_LIBRIME_SHA) {
    throw "librime source mismatch: expected $EXPECTED_LIBRIME_SHA, got $ACTUAL_LIBRIME_SHA"
}
$LIBRIME_STATUS = @(
    & git -C $LIBRIME_SOURCE status --porcelain=v1 --untracked-files=all
)
if ($LASTEXITCODE -ne 0 -or $LIBRIME_STATUS.Count -ne 0) {
    throw "pinned librime source must be clean"
}

$PRODUCT = Join-Path $REPO "apps\yune-web\public\schema"
$YUNE_DLL = Join-Path $REPO "target\release\yune_rime_api.dll"
$THRESHOLDS = Join-Path $REPO "docs\reports\evidence\m55-native-match-or-beat\thresholds\m55-thresholds.csv"
$TRACK_A = "n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru,zh,j,yi,che,chuang,b,ceshi,zhongdengchangdu,dazisudu"
$TRACK_B = "neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung"
$M61_THRESHOLD = "C:\absolute\path\to\frozen-m61-memory-threshold.csv"
$M61_THRESHOLD_SHA256 = "d52d064f410df36c1c22dd5523430062563a17bb9f2f63253b607d211badefd7"
$ACTUAL_THRESHOLD_SHA256 = (
    Get-FileHash -Algorithm SHA256 -LiteralPath $M61_THRESHOLD
).Hash.ToLowerInvariant()
if ($ACTUAL_THRESHOLD_SHA256 -ne $M61_THRESHOLD_SHA256) {
    throw "M61 supplemental threshold hash mismatch: $ACTUAL_THRESHOLD_SHA256"
}

function Invoke-M61FinalRound {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [string]$BenchmarkExecutable = "",
        [string]$BenchmarkReceipt = ""
    )
    $RoundOutput = Join-Path $OUT $Name
    $RoundWork = Join-Path $OUT ("work\" + ($Name -replace "[\\/]", "-"))
    $Args = @(
        "-NoProfile", "-ExecutionPolicy", "Bypass",
        "-File", (Join-Path $REPO "scripts\benchmark-native-rime-inprocess.ps1"),
        "-OutputRoot", $RoundOutput,
        "-WorkRoot", $RoundWork,
        "-UpstreamOracleRoot", $LIBRIME,
        "-ProductSchemaRoot", $PRODUCT,
        "-YuneDll", $YUNE_DLL,
        "-Iterations", "9",
        "-SessionIterations", "60",
        "-KeyIterations", "80",
        "-TrackAInputs", $TRACK_A,
        "-TrackBInputs", $TRACK_B,
        "-DeployProductBeforeBenchmark",
        "-TrackAThresholds", $THRESHOLDS,
        "-FailOnRegression"
    )
    if ($BenchmarkExecutable -or $BenchmarkReceipt) {
        if (-not $BenchmarkExecutable -or -not $BenchmarkReceipt) {
            throw "prebuilt benchmark executable and receipt must be supplied together"
        }
        $Args += @(
            "-PrebuiltNativeBenchmarkExecutable", $BenchmarkExecutable,
            "-PrebuiltNativeBenchmarkReceipt", $BenchmarkReceipt
        )
    }
    & powershell @Args
    if ($LASTEXITCODE -ne 0) {
        throw "M61 final round failed: $Name"
    }
}

function Read-EnvironmentValue {
    param([string]$Path, [string]$Key)
    $Prefix = "$Key="
    $Rows = @(Get-Content -LiteralPath $Path | Where-Object {
        $_.StartsWith($Prefix, [System.StringComparison]::Ordinal)
    })
    if ($Rows.Count -ne 1) {
        throw "$Path must contain exactly one $Key row"
    }
    return $Rows[0].Substring($Prefix.Length)
}

cargo build --release -p yune-rime-api
if ($LASTEXITCODE -ne 0) { throw "candidate Yune release build failed" }
Get-FileHash -Algorithm SHA256 -LiteralPath $YUNE_DLL

Invoke-M61FinalRound -Name "final\run-1"
$BENCH_EXE = Read-EnvironmentValue `
    (Join-Path $OUT "final\run-1\environment.txt") `
    "native_benchmark_executable"
$BENCH_RECEIPT = Join-Path $OUT "final\run-1\native-benchmark-build-receipt.txt"
2..5 | ForEach-Object {
    Invoke-M61FinalRound -Name "final\run-$_" `
        -BenchmarkExecutable $BENCH_EXE -BenchmarkReceipt $BENCH_RECEIPT
}
$FinalAggregateArgs = @(
    "-B", (Join-Path $REPO "scripts\aggregate-native-ratchet.py"),
    "--thresholds", $THRESHOLDS,
    "--supplemental-thresholds", $M61_THRESHOLD,
    "--expected-runs", "5",
    "--run", (Join-Path $FINAL "run-1"),
    "--run", (Join-Path $FINAL "run-2"),
    "--run", (Join-Path $FINAL "run-3"),
    "--run", (Join-Path $FINAL "run-4"),
    "--run", (Join-Path $FINAL "run-5"),
    "--output", (Join-Path $FINAL "gate-verdict.csv"),
    "--supplemental-output", (Join-Path $FINAL "m61-memory-verdict.csv")
)
& python @FinalAggregateArgs
if ($LASTEXITCODE -ne 0) {
    throw "M61 final aggregate failed and must be preserved: $FINAL"
}
```

### Runs

- [x] From a disposable clean detached clone of the exact pushed Phase 0B
      Track B-corrected source `67d32a2b`, record the complete source chain,
      clean status, remote equality, fixed hashes, five wholly new owned rounds,
      and the separately named exploratory byte-backed run. The owned set is
      green; the exploratory measured owner-shape red is preserved and its
      remaining five-round byte-backed set is skipped.
- [x] After the post-diagnostic amendment and quality-gate repair are pushed and
      remote-equal, create the disposition-B correction as the exact local child
      of the repair. Create a new disposable detached clone at that local commit;
      do not switch an existing tree, create a branch, registered worktree, or
      use the main checkout as measurement evidence. Record the M60/Phase 0
      chain and clean status before and after every accepted set, and remove the
      clone only after the external packet is secured.
- [x] Before building the replacement A/B, require `git rev-parse HEAD` and
      `HEAD^{tree}` to equal the recorded local disposition-B correction
      SHA/tree, prove every frozen source named in the revised binding command
      is an ancestor with the exact direct-parent chain, and require `git status
      --porcelain` to be empty. Record `git ls-remote origin refs/heads/main` and
      require it to equal the pushed quality-gate repair SHA; the local
      correction must descend from that remote tip but is intentionally not yet
      contained by it. A mismatch is a setup failure, not usable evidence.
- [x] Build the replacement benchmark and Yune library once; record SHA-256
      values. Run five wholly new `owned` rounds and require the unchanged signed
      registry, candidate/model identities, Track B guards, and fixed binary
      hashes to be green. No `67d32a2b` or `f18b0df2` round contributes.
- [x] Run one new explicitly named exploratory byte-backed round at the
      disposition-B correction. A measured red is preserved, skips the
      five-round byte-backed set, exhausts the one B correction, and selects
      disposition D through the explicit revert path above.
- [x] After a green replacement exploratory round, run five complete
      `byte-backed` rounds.
      Record run order, set-boundary thermal state, and elapsed time so
      time/thermal bias remains visible.
- [x] Preserve any setup failure under an explicit retry name. Preserve every
      measured red; never replace or cherry-pick it.
- [x] Verify fixed binary hashes and source/tree identity across every completed
      replacement round: ten aggregate-binding rounds when both five-round sets
      run, plus the separately preserved exploratory round.
- [x] Require each wrapper round to emit a hashed
      `candidate-parity-verdict.txt` with `17/17` PASS before aggregation; the
      aggregator rejects a missing, red, changed, or source/input-mismatched
      receipt.
- [x] Aggregate median, pooled worst, spread, signed verdicts, owner rows,
      private/mapped counters, and candidate/model checks.
- [x] Confirm that byte-backed owner rows report `poet_bin` storage and that the
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

Only unique named-owner rows whose `byte_class` equals
`heap_owned_reducible` and whose `non_overlapping_reducible_bytes` is greater
than zero participate in the private-owner sum. No separate CSV
leaf-classification field is assumed. Duplicate owner IDs within a round are an
evidence error. Process totals, phase totals, mapped-file rows, and the
working-set-derived
`process.after_ready_working_set_unclassified_lower_bound` are not private
owners and must never be ranked or added to that sum. Using five-run aggregates
from the common zero-key owner-snapshot phase, define:

```text
owned_private = median(five owned process.owner_snapshot_private_bytes rows)
byte_backed_private = median(five byte-backed process.owner_snapshot_private_bytes rows)
whole_process_private_delta = owned_private - byte_backed_private

owned_leaf_total = median(five per-round sums of non_overlapping_reducible_bytes
                          where byte_class == heap_owned_reducible)
byte_backed_leaf_total = median(five per-round sums of non_overlapping_reducible_bytes
                                where byte_class == heap_owned_reducible)
explained_heap_delta = owned_leaf_total - byte_backed_leaf_total
coverage = explained_heap_delta / whole_process_private_delta
```

Report every raw per-round value, but compute the binding delta once from the
mode-level medians above. Independent fresh processes are not paired by round
index, and the plan never takes a median of arbitrary per-pair ratios. Require
the eligible owner-id set to remain stable within each accepted mode or explain
and fail closed on every missing/new identity before aggregation. Do not
substitute `summary.csv`'s post-key private envelope into this zero-key equation.

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

- [x] Name at least `10,000,000 B` of current non-overlapping reducible memory.
      The selected POET owner delta was `18,724,242 B`.
- [x] **Evaluated — FAIL:** reconcile the owned-to-byte-backed whole-process
      private delta under the formula and tolerance above without
      double-counting process, mapped, or derived residual rows. Coverage was
      `0.746098930`, and the `6,371,950 B` residual exceeded its
      `5,019,238 B` limit.
- [x] Verify whether the likely owner still consists of the retained preset
      vocabulary recipe, `poet.vocabulary`, `poet.entries_by_code`, and the
      small `poet.abbreviation_vocabulary` reducible leaf. Report the guarded
      `translator.upstream_sentence_model_preset_vocabulary_recipe` and
      `poet.lookup_index` rows separately; neither is included in the
      `heap_owned_reducible` coverage sum unless fresh tooling explicitly and
      validly reclassifies it.
- [x] Prove that the proposed reduction affects same-process Windows private
      bytes rather than only clean mapped residency or page-cache luck.
- [x] Select exactly one disposition: **D — measured partial/no-go.**
  - **A — byte-backed POET re-land:** diagnostic clears the signed and behavior
    gates and projects the full win;
  - **B — one bounded scratch/access correction:** memory wins but one measured
    byte-backed owner remains, with an exact repair and retry contract;
  - **C — another current Track A owner:** only if fresh attribution disproves
    POET and names a higher-leverage non-overlapping owner; this selects an
    owner but requires an exact owner-specific plan amendment and both reviews
    before Phase 3 implementation; or
  - **D — measured partial/no-go:** no safe owner or win.

Allocator work is not selectable from the optional cross-process diagnostic.
M47 RED-09/10/11 work is not imported by analogy; those rows belong to a
different product profile and counter.

### Post-diagnostic disposition-B amendment

The pushed Phase 0B correction source
`67d32a2bea36a391a8a11ea4e725dbfebe118252` (tree
`7e2157b5de2575728f2632fad184a05403342a13`) completed five wholly new owned
rounds at `32/32` signed rows per round with `17/17` candidate parity and green
Track B guards. The aggregate is `32/32` and all 160 individual observations
pass. Those rounds remain the eligibility baseline; they are not reusable in a
later source set.

The separately named `byte-backed-exploratory/run-1` completed timing and
candidate capture with `17/17` parity, then produced the plan-prescribed
measured owner-shape red. The four required mapped `YUNE-POET/3` owners were
present, but one fifth POET row remained:

```text
poet.normal_character_code_index
retained_estimate_bytes=11538
byte_class=heap_owned_guarded
non_overlapping_reducible_bytes=0
mapping_mode=sorted Box<[String]>
```

The run's Track A peak was `116,314,112 B`, below the frozen `125,000,000 B`
supplemental cap and below the owned five-run median `154,251,264 B`. Its
candidate result was green, so this is not a setup failure and must never be
retried, replaced, or relabeled. Skip the remaining byte-backed rounds at that
source. Preserve the complete raw root at
`C:\yune-m61\67d32a2bea36a391a8a11ea4e725dbfebe118252`.

This evidence selects **disposition B** provisionally and consumes the one
owner branch authorized by `M61-BRANCH-01`. The only correction is bounded
borrowed membership access for the mapped character-code sections: owned POET
storage retains its existing sorted normal-character-code index and behavior;
byte-backed storage must stop reconstructing that retained `Box<[String]>`,
must answer `has_normal_character_code` from already validated `YUNE-POET/3`
bytes without a retained replacement index, and must emit exactly the existing
four mapped POET owner rows. Do not change artifact bytes/version, candidate
behavior, owned fallback, default selection, deployment, wrappers, thresholds,
cadence, ABI, schema/profile IDs, browser payloads, or the exact-four owner
assertion in this correction.

The correction-only implementation allowlist is exactly:

```text
crates/yune-core/src/poet/mod.rs
crates/yune-core/src/poet/storage.rs
crates/yune-core/src/tests/poet.rs
```

Create one path-limited correction commit directly on local `main`, but do not
push it yet. From a disposable clean detached clone of that exact commit, run a
wholly new source-bound diagnostic sequence under a create-new external root:

1. five new owned rounds and aggregate;
2. one explicitly named exploratory byte-backed round;
3. only if that exploratory round is green, five new byte-backed rounds and the
   signed plus supplemental aggregates; and
4. the complete owner reconciliation in Phase 2, including stable eligible
   owner sets, positive deltas, the `0.80..=1.20` coverage interval, residual
   bound, `10,000,000 B` named-owner floor, private-byte corroboration, and
   separate mapped-byte reporting.

All fixed binary, source/tree, oracle, product, model, input, cadence, parity,
and Track B requirements remain unchanged. The first measured red at the
correction source exhausts disposition B: preserve it, create an explicit local
revert commit without rewriting history, select disposition D, and close with
no production-default change. The correction and revert commits remain in the
eventual direct-main history so the red source is auditable. If the complete
corrected A/B and reconciliation are green, keep the correction commit local as
the parent of a separate production-default candidate commit. Neither commit is
pushed until the exact production-default acceptance set is green.

A measured red in the production-default five-round set is preserved and
unconditionally selects disposition D. Create two explicit local revert commits,
first reverting the production-default candidate and then the disposition-B
correction, so the resulting runtime tree equals the pushed quality-gate repair
tree while every correction/revert SHA remains in history. Do not retry, rewrite,
or discard a red source. After a fully green final set, push the disposition-B
correction and production-default candidate chain, require `origin/main` to
equal that accepted implementation SHA, and only then build the
evidence/documentation pre-review tree for final closeout reviews.

#### Disposition-B correction result

The authorized three-path correction was committed as
`91f5969688a3d2dba96a67d1cfe813c7ba4ee861`, tree
`6626ed16d5e135fa477ca26e9786d11121c92b44`. Its detached clean-source
replacement sequence completed five owned rounds, the separately named green
exploratory byte-backed round, and five byte-backed rounds. Both complete sets
passed `32/32` aggregate rows, `160/160` individual observations, `17/17`
candidate parity in every round, fixed source/binary/model identities, Track B,
and the unchanged signed registry. Byte-backed storage emitted exactly the four
mapped POET owners and no retained fifth owner. The diagnostic supplemental row
also passed, with a `116,162,560 B` median and `116,334,592 B` worst peak.

Owner reconciliation then produced the binding measured red: coverage
`0.746098930` was below `0.80`, and residual `6,371,950 B` exceeded its
`5,019,238 B` bound. Per the no-retry rule, no production-default candidate or
final five-round production set was created. The exact correction was reverted
by `01a62f2a6cd2b3d668545a110de8c7c3fc2fbb10`; the resulting tree
`f1c36a0079d85628f5cbef140bd94288930cc2e8` exactly equals the pushed
quality-repair tree. The correction and revert remain in direct-main history so
the measured-red source remains auditable.

### Baseline quality-gate restoration amendment

The exact pushed `67d32a2b` source also exposes pre-existing source-quality reds
that are independent of the measured Track A owner branch: current rustfmt
output differs in four M60 Rust files, strict workspace clippy finds two
last-use `Option<&mut T>` reborrow patterns and one use of `Option::is_none_or`
outside the repository's Rust 1.76 MSRV. The final plan already requires
`cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings`;
leaving those reds in the production candidate is not permitted.

Restore those gates in a separate nonsemantic commit before creating the local
disposition-B correction. This repair does not consume `M61-BRANCH-01` and may
not change benchmark tooling, thresholds, cadence, artifacts, runtime behavior,
ABI, schemas, or tests beyond formatting. Its source allowlist is exactly:

```text
crates/yune-core/src/translator/mod.rs
crates/yune-rime-api/src/bin/yune-schema-reachability-audit.rs
crates/yune-rime-api/src/deployment.rs
crates/yune-rime-api/src/reachability_audit.rs
crates/yune-rime-api/tests/yune_web/m60_reachability.rs
```

The only semantic-token substitutions are:

- replace the last-use `sentence_scratch.as_deref_mut()` in
  `translator/mod.rs` with direct `sentence_scratch`;
- retain the earlier reusable `trace.as_deref_mut()` in `deployment.rs`, but
  replace its last-use reborrow with direct `trace`; and
- replace `Option::is_none_or` in `reachability_audit.rs` with the equivalent
  MSRV-safe `matches!(..., None | Some("upstream_script"))` predicate.

Apply current rustfmt output only to the four files reported by
`cargo fmt --check`; do not change formatting configuration, lint allowances,
the MSRV, or toolchain policy. Run `cargo fmt --check`, strict workspace clippy,
the focused upstream Luna, deployment, and M60 reachability tests, plus
`git diff --check`. Freeze the repair tree, obtain two independent reviews, add
only the two repair review receipts named below, commit and push the reviewed
repair separately, and prove `origin/main` equality before the local
disposition-B correction is created. The final candidate reruns the binding
workspace tests; no duplicate repair-source workspace run is required.

### Restored-tree workspace test-contract corrections

The exact 20-gate restored-tree closeout runner was bound to source
`01a62f2a6cd2b3d668545a110de8c7c3fc2fbb10`, runtime tree
`f1c36a0079d85628f5cbef140bd94288930cc2e8`. Gates 1--19 exited zero. Gate
20 executed the literal `cargo test --workspace` command and exited `101`.
Its successful prefix is retained: `yune-cli` `34/34`, `frontend_surrogate`
`5/5`, the `yune-core` library `555/555`, and canonical provenance `2/2`.
`cantonese_parity` then reported `37 passed / 4 failed` in:

- `m21_closeout_rows_match_typeduck_v112_real_dictionary_goldens`;
- `m21_nri_prefix_fallback_matches_typeduck_v112_real_dictionary_goldens`;
- `m21_prediction_count_matches_typeduck_v112_real_dictionary_goldens`; and
- `m58_current_yune_web_profile_reaches_beingo_bei_at_typeduck_rank`.

WEB03 commit `a7f61dae` intentionally made the initial TypeDuck/profile refresh
page-sized with explicit on-demand completion. Those four tests still compared
the bounded initial list to all-pages/page-50 oracle fixtures. The first
correction changes only `crates/yune-core/tests/cantonese_parity.rs`, adding
four explicit `ensure_complete_candidate_list()` calls immediately before the
complete-list comparisons. The M58 helper still asserts the bounded initial
leader and target absence before expansion, while the real deployed profile
page-size-6 guard still pages to the target. The isolated core-test correction
tree is `bf4ef0b8d7d234b248cc61e9a1c5ad6b57ee61af`; its corrected blob is
`a8bc25e21c80107caafcd19525c470f3f991378d`.

The first serial retry was deliberately interrupted before a verdict while its
scope was audited. It is preserved with `exit_code=-1`, `gate_verdict=none`,
and reuse prohibited. Retry 2 then passed `cargo fmt --check`, serial
`cantonese_parity` `41/41`, and the real deployed product-page guard `1/1`.
Following the M59 disjoint-recovery pattern, retry 3 retained the successful
workspace prefix and ran only the interrupted/never-reached suffix. Strict
workspace Clippy passed, and the eight never-reached `yune-core` integration
targets passed `69` tests with `8` declared ignores. The API package's library
target then exposed one second deterministic contract red: `363 passed / 1
failed / 1 ignored` because the old lifecycle test still required pre-M56
blanket wording that had been intentionally replaced by the current narrow
threading contract.

The second correction changes only
`crates/yune-rime-api/src/tests/lifecycle_safety.rs`, itself compiled solely
under `cfg(test)`. It renames the stale test and locks both the current
one-active-service convention and M56's valid-session cross-thread tolerance
without promising parallel progress. The combined two-test correction tree is
`6cb28424f7bcf5a535ac6173b651e9ba1b7bd160`; the corrected lifecycle-test blob
is `5a805ad7c41858fca92ef75ac4a604087e195936`. Retry 4 passed source-current
formatting and exact strict workspace Clippy, the complete owning API library
(`364 passed / 1 ignored`), every still-never-reached API bin and integration
target (`114 passed / 3 ignored`), and the remaining API/core doc groups (`0`
tests each). Its status and completion receipts have SHA-256
`e8817e5f73a8d3507d40f2351bc29b91a7e37dd3853191acf91d88aee1a2d1af`
and `815abfa7112a13263dc5dcf3ebfcc433d15c411744c663b9ffdc64be79a46f2a`.
The original broad red, the no-verdict interruption, and the second API-library
red remain preserved; the literal workspace command is not rerun or claimed as
an exit-zero receipt. These are closeout test-contract corrections, not M61
memory-measurement retries, and do not reopen disposition D.

The first isolated pre-review tree then exposed an evidence-storage red: the
working-tree packet manifest passed, but the repository-wide LF rule would have
normalized imported Windows receipts in the commit tree. That candidate was
discarded before review. The minimum `.gitattributes` packet-subtree `-text`
rule now preserves the exact curated bytes; it changes no source/runtime
behavior and is included in both final reviews and closeout path envelopes.

## Phase 3 — Conditional Production Candidate

Phase 3 exists only for disposition A, B, or C.

The production policy is native-target/capability-bound, not a global
environment default:

- WASM must never select POET storage, even if a sidecar is present. Native
  runtime consumption requires an artifact whose version plus schema/table/
  source identities validate against that deployment;
- an absent artifact preserves the owned fallback and its current behavior,
  while a present invalid, stale, or replaced artifact fails loudly;
- M61 adds no `.poet.bin` to the repository-managed web static payload. The
  existing schema-manifest check proves only the
  `apps/yune-web/public/schema` boundary, not npm/package contents or runtime
  generation/consumption;
- the shared deployer can currently generate or copy a POET sidecar while
  rebuilding dictionary artifacts. M61 must either preserve and accurately
  document that behavior or introduce an explicit internal target-aware
  generation policy with owning tests; it must not infer runtime safety from
  the manifest;
- native package scripts receive no schema-payload change. Any binding package-
  content claim requires a create-new output plus an explicit file allowlist and
  `.poet.bin` absence assertion; and
- the shipping native path requires no inherited environment variable. The
  diagnostic selector remains benchmark tooling rather than product policy.

For expected byte-backed disposition A or B:

- [x] **Not applicable under disposition D — not run and not claimed:** preserve
      the current versioned `YUNE-POET/3` validation and loud rejection
      of present truncated, checksum-mismatched, invalid, wrong-version, or
      legacy artifacts, including `YUNE-POET/1` and `YUNE-POET/2`. Bump the
      format only if bytes or semantics actually change.
- [x] **Not applicable under disposition D — not run and not claimed:** keep
      POET artifact creation in the untimed deploy/preparation phase and
      prove runtime consumption uses the validated bytes.
- [x] **Not applicable under disposition D — not run and not claimed:** port or
      retain the current incremental sentence scratch, reachability,
      lookup, and cache-invalidation behavior without reconstructing the owned
      vocabulary/entry maps.
- [x] **Not applicable under disposition D — not run and not claimed:** make the
      validated native deployed Luna path production-default in the
      same candidate that passes all gates. Do not land an intermediate default
      flip with a red ratchet.
- [x] **Not applicable under disposition D — not run and not claimed:** keep the
      environment switch out of the shipping requirement and record
      the actual storage mode in owner diagnostics.
- [x] **Not applicable under disposition D — not run and not claimed:** retain
      and rerun the existing POET v3 format, corruption/legacy/checksum,
      deployment/reuse/copy/rebuild, cache replacement, owner shape, every
      growing 37/59 prefix, pinned-oracle byte-backed result, translator
      invalidation, and lifecycle coverage. Add only gaps introduced by M61:
      no-environment native default selection, explicit owned diagnostic
      override, absent-artifact owned fallback, invalid/stale fail-closed
      behavior without cached reuse, native-target versus WASM capability
      isolation, wrapper selector/owner-shape assertions, and lifecycle clearing
      under the new default.
- [x] **Not applicable under disposition D — not run and not claimed:** report
      every new cache or hot layer as a named bounded memory owner.

Do not delete the owned fallback or change missing-artifact/source-deploy
semantics unless the fresh audit proves that change necessary and the plan is
amended before implementation. No hidden behavior tradeoff is authorized.

## Phase 4 — Final Five-Round Acceptance

- [x] **Skipped by D — not run and not claimed:** build the production-default
      candidate once and record all hashes.
- [x] **Skipped by D — not run and not claimed:** run five complete final
      Windows rounds with the normal signed invocation;
      no diagnostic mode or inherited POET environment variable may be active.
- [x] **Skipped by D — not run and not claimed:** require the memory win,
      unchanged signed registry, all individual rows,
      Track B guards, candidate/model identities, and owner-shape checks above.
- [x] Run the load-bearing restored-tree Rust, manifest, evidence, and
      documentation closeout gates. Gates 1--19 exited zero. The exact workspace
      command was preserved red at `cantonese_parity`; after the two cfg(test)
      contract corrections, the successful prefix was retained and the
      interrupted plus every never-reached target passed under the M59 disjoint-
      recovery pattern. The literal workspace invocation was not rerun and is
      not claimed as exit-zero. No production-default final-set gate is claimed.
- [x] **Skipped by D — not run and not claimed:** run a five-round exact-source
      macOS diagnostic outside the
      Windows acceptance packet and label RSS/absolute differences as
      platform-specific. Do not delay or redefine Windows acceptance around it.
- [x] Publish the compact packet, freeze the pre-review tree for two independent
      reviews, update current docs only for the measured result, and move this
      plan to `plans/completed/`. The two review receipts and regenerated packet
      manifest are the only permitted post-review delta; final commit/push
      proofs remain external.

## Execution And Commit Sequence

1. **Finalized plan boundary.** The source-bound finalization commit contains
   only this plan, roadmap/requirements traceability, and the two planning
   review receipts. It changes no runtime code, decision, threshold, M60
   evidence, or signed registry. Push it and prove remote equality before
   execution.
2. **Execution kickoff.** Fetch the pushed finalization, require local `main`
   to fast-forward to or equal `origin/main`, fingerprint the actual unrelated
   dirty state, record the Windows environment and source chain, and stop for
   manual reconciliation if branches diverge or any M61 path overlaps user
   work.
3. **Measurement/tooling commit.** Add only the explicit diagnostic selector,
   missing attribution, fixed-binary aggregator provenance support,
   supplemental-ratchet evaluator, and public-evidence privacy checker needed
   for M61. The default signed invocation must remain identical. The immutable
   original is `91b8991c`; the reviewed PowerShell-environment and owned-owner-
   shape tooling repairs are `6a1cbed7` and `7805882d`. All are pushed and
   remote-equal, and none is an accepted measurement source.
4. **Candidate-parity amendment.** Preserve the clean `7805882d` Windows
   `16/17` preflight red, review this plan-only candidate twice, add only the two
   amendment review receipts after review, commit directly to `main`, push, and
   prove remote equality before any behavior edit.
5. **Candidate-parity correction.** Add only Phase 0A's structural stale-raw-
   edge correction, external-oracle fixture/test, owned/byte-backed structural
   tests, and candidate comparator with wrapper/aggregator enforcement. Run the
   owning gates and two independent reviews. The reviews name the preserved
   pre-review tree; their two receipts are the only post-review delta. Commit
   and push separately with no memory optimization.
6. **Corrected preflight.** From a new clean detached clone of the pushed
   correction commit, repeat the complete 17-input Windows preflight with the
   frozen oracle/shared/build identity. Only `17/17` PASS selects the exact
   owned/byte-backed measurement source; another red blocks M61 without waiver.
7. **Track B prerequisite amendment.** Preserve the `f18b0df2` owned red,
   review the Phase 0B plan-only candidate twice, add only its two unique review
   receipts, commit directly to `main`, push, and prove remote equality before
   touching the retained owner.
8. **Track B packed-syllabary correction.** Change only the six frozen Phase 0B
   implementation paths and owning tests, with no benchmark or threshold edit.
   Freeze the pre-review tree, obtain two independent reviews, add only their
   two receipts, require the correction parent to equal the Phase 0B amendment
   commit, commit and push separately, and prove remote equality.
9. **Corrected-source preflight and replacement owned baseline.** From a new
   clean detached clone and create-new root, repeat the complete `17 + 1`
   preflight and five wholly new owned rounds. Reuse no `f18b0df2` round. Only
   `17/17` parity plus `32/32` in every round permits the A/B; another red blocks
   M61 with the Phase 0B exception exhausted. If red, push an exact correction
   revert, prove remote equality at the revert, preserve both commits and the
   red, and stop with no unaccepted prerequisite correction left on `main`.
10. **Initial A/B, measured red, and post-diagnostic amendment.** Preserve all
    `67d32a2b` raw results outside Git, skip its remaining byte-backed rounds,
    freeze disposition B plus the separate quality-gate repair in this plan,
    review the plan-only tree twice, add only its two receipts, commit, push, and
    prove remote equality.
11. **Baseline quality-gate repair.** Make only the five frozen nonsemantic
    source edits, run the focused gates, freeze and review the repair tree twice,
    add only its two receipts, commit directly on the amendment parent, push,
    and prove remote equality.
12. **Disposition-B correction and replacement A/B.** Commit only the three
    frozen POET paths directly to local `main`; do not push. From a disposable
    clean detached clone of that exact source, run five fresh owned rounds, the
    exploratory byte-backed round, then five byte-backed rounds only if it is
    green. Complete owner reconciliation. A measured red selects D and the
    explicit correction revert path; do not rewrite history.
    **Result:** the replacement A/B completed green through both five-round
    signed sets, then owner reconciliation completed red and selected D. The
    correction was explicitly reverted without rewriting history.
13. **Production-default candidate.** Only after the replacement A/B and owner
    reconciliation are green, create a separate direct-parent local commit that
    makes validated native POET storage the default while preserving the owned
    override/fallback and WASM boundary. Freeze the actual sorted path list and
    run focused correctness/owner tests before final measurement.
    **Result:** skipped by D; no production-default candidate was created.
14. **Five-round final acceptance and implementation push.** Use a disposable
    clean detached clone of the exact local production candidate. A measured red
    takes the explicit two-revert disposition above without retry. After a wholly
    green set, push the disposition-B correction plus production-default
    candidate chain and prove `origin/main` equals the accepted implementation
    SHA.
    **Result:** skipped by D; no final production set or accepted implementation
    push exists. The correction/revert chain was pushed for auditability, with
    `origin/main` equal to the revert before closeout.
15. **Workspace test-contract corrections and nonduplicative recovery.**
    Preserve the exact workspace red and its successful prefix. Change only the
    two cfg(test) paths named above, freeze the core-only and combined correction
    trees, preserve the interrupted no-verdict retry, and recover the interrupted
    plus never-reached targets without replacing the original broad receipt.
    Run source-current formatting and strict workspace Clippy after the final
    test-only edit.
16. **Final reviews and closeout commit.** Curate only the compact packet and
    current-document updates on top of the pushed implementation. Both reviews
    name that exact implementation commit/tree and the proposed pre-review
    evidence/documentation tree. After review, only the two receipts and
    regenerated packet manifest may change. Move the plan, verify exact
    committed tree and remote identity, push the closeout separately, and
    preserve the fingerprinted unrelated state exactly.
    **Result:** this completed-plan candidate and compact packet follow that
    closeout path; after review, the only permitted additions are the two
    receipts and regenerated packet manifest.

## Finalization Review Record

The source-current planning candidate is reviewed twice before the finalization
commit:

- [`planning-review-measurement.md`](../../reports/evidence/m61-native-track-a-memory-owner-reduction/planning-review-measurement.md)
  binds the measurement, oracle, owner-reconciliation, failure, and terminal
  disposition contract; and
- [`planning-review-isolation.md`](../../reports/evidence/m61-native-track-a-memory-owner-reduction/planning-review-isolation.md)
  binds source ancestry, threshold safety, path isolation, and documentation
  traceability.

The receipts name the exact pre-review candidate tree. They are the only
post-review additions to the finalized planning commit; any change to this
plan, `requirements.md`, or `roadmap.md` after those reviews requires rebuilding
the candidate tree and repeating both reviews unless an explicit later
owner-authorized amendment uses its own immutable pre-review tree and exact
review-delta contract below.

### Prerequisite amendment review record

This Phase 0A amendment repeats that fail-closed review shape before any
behavior edit. The plan-only candidate tree is reviewed twice for:

1. requirement/evidence correctness, production semantics, oracle provenance,
   and the non-circular parity proof; and
2. change isolation, comparator fail-closure, threshold safety, ABI/runtime
   boundaries, and accepted-source lineage.

Both receipts must name the preserved plan-only pre-review tree. The exact
post-review delta is only:

```text
docs/reports/evidence/m61-native-track-a-memory-owner-reduction/planning-amendment-review-isolation.md
docs/reports/evidence/m61-native-track-a-memory-owner-reduction/planning-amendment-review-requirements.md
```

The complete amendment commit path list is those two receipts plus:

```text
docs/plans/active/m61-plan-native-track-a-memory-owner-reduction.md
```

The lists are repository-relative and bytewise sorted. Use an isolated index,
prove the amendment candidate tree equals the commit tree, keep the real index
empty, push directly to `main`, and prove `origin/main` equality. Any other
post-review path or plan edit invalidates both reviews.

The separately scoped correction uses the same pre-review-tree discipline.
Its only post-review additions are
`candidate-parity-review-requirements.md` and
`candidate-parity-review-isolation.md` under the same evidence directory. The
correction commit's pre-review paths are frozen from the actual minimal
implementation; any other post-review delta requires both correction reviews
again. These amendment and correction reviews do not replace the two final
closeout reviews or alter their exact post-review delta.

### Track B prerequisite amendment review record

Phase 0B repeats the fail-closed review shape before the packed-syllabary edit.
The plan-only pre-review candidate is reviewed independently for:

1. requirement/evidence correctness, the preserved measured-red status, owner
   sufficiency, and the complete new-source measurement contract; and
2. change isolation, threshold/cadence safety, representation-only runtime/ABI
   boundaries, and candidate-tree/accepted-source lineage.

Both receipts name the preserved plan-only pre-review tree. The exact
post-review delta is only:

```text
docs/reports/evidence/m61-native-track-a-memory-owner-reduction/planning-track-b-amendment-review-isolation.md
docs/reports/evidence/m61-native-track-a-memory-owner-reduction/planning-track-b-amendment-review-requirements.md
```

The complete amendment commit contains exactly those two receipts plus:

```text
docs/plans/active/m61-plan-native-track-a-memory-owner-reduction.md
```

Use a bytewise-sorted path list, isolated index, intent-to-add for only the two
new receipts, a path-limited commit, preserved candidate-tree hash, exact
commit-tree equality, an empty real index, direct push to `main`, and remote
equality. Any other post-review path or plan edit invalidates both reviews.

The separately scoped packed-syllabary correction uses the same discipline. Its
pre-review tree contains only the six frozen Phase 0B implementation paths and
tests within them. Its only post-review additions are:

```text
docs/reports/evidence/m61-native-track-a-memory-owner-reduction/track-b-correction-review-isolation.md
docs/reports/evidence/m61-native-track-a-memory-owner-reduction/track-b-correction-review-requirements.md
```

Both correction receipts name the correction source parent, preserved
pre-review tree, exact implementation paths, and verification. Any other
post-review delta requires both correction reviews again. Neither Phase 0B
review pair replaces the two final closeout reviews or changes their exact
three-path post-review delta.

### Post-diagnostic amendment and quality-repair review record

Before the disposition-B correction or baseline quality repair, review this
plan-only amendment independently for:

1. measured-red classification, disposition-B owner sufficiency, the exact
   bounded correction, fresh diagnostic/reconciliation contract, and production
   acceptance lineage; and
2. change isolation, preserved owner-shape/threshold/cadence rules, the separate
   nonsemantic quality repair, candidate-tree discipline, and runtime/ABI/browser
   boundaries.

Both amendment receipts name the exact preserved plan-only pre-review tree. The
only post-review additions are:

```text
docs/reports/evidence/m61-native-track-a-memory-owner-reduction/planning-post-diagnostic-amendment-review-isolation.md
docs/reports/evidence/m61-native-track-a-memory-owner-reduction/planning-post-diagnostic-amendment-review-requirements.md
```

The complete amendment commit contains exactly those two receipts plus:

```text
docs/plans/active/m61-plan-native-track-a-memory-owner-reduction.md
```

Use bytewise-sorted path lists, an isolated index, intent-to-add for only the two
new receipts, a path-limited commit, exact candidate-tree/commit-tree equality,
an empty real index, direct push to `main`, and remote equality. Any other
post-review path or plan edit invalidates both reviews.

The separately scoped quality-gate repair repeats the same pre-review-tree
discipline. Its pre-review candidate contains exactly the five frozen source
paths in the quality-gate amendment above. Its only post-review additions are:

```text
docs/reports/evidence/m61-native-track-a-memory-owner-reduction/quality-gate-repair-review-isolation.md
docs/reports/evidence/m61-native-track-a-memory-owner-reduction/quality-gate-repair-review-requirements.md
```

Both repair receipts name the amendment parent, preserved pre-review repair
tree, exact path list, focused verification, and nonsemantic diff. The repair
commit contains exactly the five source paths plus those two receipts. Any
other post-review path requires both repair reviews again. Neither review pair
replaces the final two closeout reviews or changes their exact three-path
post-review delta.

The planning finalization itself uses this literal five-path isolated-index
procedure. The three-path pre-review list contains this plan,
`docs/requirements.md`, and `docs/roadmap.md`. The two-path receipt list contains
only the two files linked above. The complete list is their union, all lists are
repository-relative and `LC_ALL=C` sorted, and the real index must be empty:

```sh
LC_ALL=C sort -u "$OUT/m61-plan-pre-review-paths.txt" \
  > "$OUT/m61-plan-pre-review-paths.sorted"
LC_ALL=C sort -u "$OUT/m61-plan-review-receipt-paths.txt" \
  > "$OUT/m61-plan-review-receipt-paths.sorted"
LC_ALL=C sort -u "$OUT/m61-plan-paths.txt" \
  > "$OUT/m61-plan-paths.sorted"
cmp "$OUT/m61-plan-pre-review-paths.txt" \
  "$OUT/m61-plan-pre-review-paths.sorted"
cmp "$OUT/m61-plan-review-receipt-paths.txt" \
  "$OUT/m61-plan-review-receipt-paths.sorted"
cmp "$OUT/m61-plan-paths.txt" "$OUT/m61-plan-paths.sorted"
cat "$OUT/m61-plan-pre-review-paths.txt" \
  "$OUT/m61-plan-review-receipt-paths.txt" \
  | LC_ALL=C sort -u > "$OUT/m61-plan-path-union.txt"
cmp "$OUT/m61-plan-paths.txt" "$OUT/m61-plan-path-union.txt"

rm -f "$OUT/m61-plan.index"
GIT_INDEX_FILE="$OUT/m61-plan.index" git read-tree HEAD
GIT_INDEX_FILE="$OUT/m61-plan.index" git add -A \
  --pathspec-from-file="$OUT/m61-plan-pre-review-paths.txt"
GIT_INDEX_FILE="$OUT/m61-plan.index" git write-tree \
  > "$OUT/m61-plan-pre-review-tree.txt"
test -z "$(git diff --cached --name-only)"

# Run both reviews against the preserved tree above. Then create only the two
# review receipts, rebuild the isolated candidate, and prove the exact delta.
rm -f "$OUT/m61-plan.index"
GIT_INDEX_FILE="$OUT/m61-plan.index" git read-tree HEAD
GIT_INDEX_FILE="$OUT/m61-plan.index" git add -A \
  --pathspec-from-file="$OUT/m61-plan-paths.txt"
GIT_INDEX_FILE="$OUT/m61-plan.index" git write-tree \
  > "$OUT/m61-plan-final-tree.txt"
git diff-tree --no-commit-id --name-only -r \
  "$(cat "$OUT/m61-plan-pre-review-tree.txt")" \
  "$(cat "$OUT/m61-plan-final-tree.txt")" \
  | LC_ALL=C sort > "$OUT/m61-plan-post-review-actual.txt"
LC_ALL=C sort "$OUT/m61-plan-review-receipt-paths.txt" \
  > "$OUT/m61-plan-post-review-expected.txt"
cmp "$OUT/m61-plan-post-review-expected.txt" \
  "$OUT/m61-plan-post-review-actual.txt"
GIT_INDEX_FILE="$OUT/m61-plan.index" git diff --cached --check
test -z "$(git diff --cached --name-only)"

test -z "$(git diff --name-only --diff-filter=A)"
git add --intent-to-add \
  --pathspec-from-file="$OUT/m61-plan-review-receipt-paths.txt"
git diff --name-only --diff-filter=A | LC_ALL=C sort \
  > "$OUT/m61-plan-intent-paths.actual"
cmp "$OUT/m61-plan-review-receipt-paths.sorted" \
  "$OUT/m61-plan-intent-paths.actual"
git commit --only --pathspec-from-file="$OUT/m61-plan-paths.txt" \
  -m "Finalize M61 native memory plan"
test "$(git rev-parse HEAD^{tree})" = \
  "$(cat "$OUT/m61-plan-final-tree.txt")"
git diff-tree --no-commit-id --name-only -r HEAD^ HEAD \
  | LC_ALL=C sort > "$OUT/m61-plan-commit-paths.actual"
LC_ALL=C sort "$OUT/m61-plan-paths.txt" \
  > "$OUT/m61-plan-commit-paths.expected"
cmp "$OUT/m61-plan-commit-paths.expected" \
  "$OUT/m61-plan-commit-paths.actual"
test -z "$(git diff --cached --name-only)"
test -z "$(git diff --name-only --diff-filter=A)"
```

Before and after this procedure, fingerprint the complete unrelated staged,
unstaged, and untracked state using the inventory commands below and require
byte equality after excluding the five M61 planning paths. In particular, no
pre-existing image or other user path may enter the candidate or commit.

### Commit and review-tree isolation

M61 adopts the corrected isolated-index procedure in M60's
`Load-Bearing Verification` section as a binding minimum, generalized from its
then-current config-only assertion to the actual Phase 0 dirty-state inventory.
The post-M60 finalization commit must copy the literal procedure into this plan
with M61 filenames so it remains usable after the M60 plan moves to completed.

For every M61 implementation or closeout commit:

- create sorted, unique, repository-relative allowlists for all paths, new
  paths, current documents, and evidence paths; reject empty, stale, escaping,
  duplicate, or unmatched entries;
- preserve binary-diff/content fingerprints for every unrelated staged,
  unstaged, and untracked path, and compare them after the commit;
- create an isolated index from `HEAD` with `GIT_INDEX_FILE`, `git read-tree
  HEAD`, and `git add -A --pathspec-from-file`; preserve its `git write-tree`
  result before touching the real index;
- add intent-to-add entries only for the allowlisted new-path subset, commit
  only with `git commit --only --pathspec-from-file`, prove `HEAD^{tree}` equals
  the isolated candidate tree, and prove the commit's path set equals the
  allowlist;
- before review, preserve `m61-pre-review-tree.txt`. Both reviews name the
  implementation SHA/tree and that pre-review tree. Afterwards, allow only
  `review-requirements.md`, `review-isolation.md`, and the regenerated packet
  manifest; prove that exact three-path tree delta or re-review; and
- build and validate `m61-final-candidate-tree.txt` in the isolated index before
  closeout. The committed tree must equal it exactly, while every unrelated
  dirty-state fingerprint remains unchanged.

The restored-tree closeout correction allowlist adds exactly two cfg(test)
paths: `crates/yune-core/tests/cantonese_parity.rs` and
`crates/yune-rime-api/src/tests/lifecycle_safety.rs`. The first path alone
produces tree `bf4ef0b8d7d234b248cc61e9a1c5ad6b57ee61af`; both together produce
tree `6cb28424f7bcf5a535ac6173b651e9ba1b7bd160` relative to restored runtime
tree `f1c36a0079d85628f5cbef140bd94288930cc2e8`. Neither correction path is
production runtime code. Both final closeout reviews must assess their contract
accuracy and isolation.

The closeout path envelope also includes `.gitattributes` solely for the M61
packet-subtree `-text` rule discovered by the failed pre-review tree-mode
manifest check. That rule preserves imported Windows evidence bytes so the
working-tree, candidate-tree, and committed-tree manifest proofs bind the same
content; it does not alter runtime or product files.

Create and retain these newline-delimited external lists in `LC_ALL=C` sorted,
unique order; reject empty, duplicate, absolute, escaping, unmatched, or stale
entries:

- `$OUT/m61-implementation-paths.txt`;
- `$OUT/m61-implementation-new-paths.txt`;
- `$OUT/m61-paths.txt`, including the two closeout review receipts;
- `$OUT/m61-pre-review-paths.txt`, excluding only the two not-yet-created review
  receipts while retaining the pre-review packet manifest;
- `$OUT/m61-new-paths.txt`;
- `$OUT/touched-current-docs.txt`;
- `$OUT/m61-evidence-paths.txt`; and
- `$OUT/post-review-allowed-paths.txt`, containing exactly the packet manifest,
  `review-requirements.md`, and `review-isolation.md`.

At each implementation or closeout boundary, capture the unrelated state before
editing and compare it after committing:

```sh
git status --porcelain=v1 --untracked-files=all > "$OUT/unrelated-status.before"
git diff --binary > "$OUT/unrelated-unstaged.before.patch"
git diff --cached --binary > "$OUT/unrelated-staged.before.patch"
git ls-files --others --exclude-standard -z |
  while IFS= read -r -d '' file; do
    shasum -a 256 "$file"
  done | LC_ALL=C sort > "$OUT/unrelated-untracked.before.sha256"
```

Repeat with `.after` names and require byte equality for all four receipts
after excluding the explicit M61 allowlist from both snapshots. Do not use a
hard-coded filename assertion in place of this complete inventory.

The isolated implementation commit procedure is literal:

```sh
rm -f "$OUT/m61-implementation.index"
GIT_INDEX_FILE="$OUT/m61-implementation.index" git read-tree HEAD
GIT_INDEX_FILE="$OUT/m61-implementation.index" git add -A \
  --pathspec-from-file="$OUT/m61-implementation-paths.txt"
GIT_INDEX_FILE="$OUT/m61-implementation.index" git write-tree \
  > "$OUT/m61-implementation-tree.txt"
test -z "$(git diff --cached --name-only)"
LC_ALL=C sort -u "$OUT/m61-implementation-paths.txt" \
  > "$OUT/m61-implementation-paths.sorted"
LC_ALL=C sort -u "$OUT/m61-implementation-new-paths.txt" \
  > "$OUT/m61-implementation-new-paths.sorted"
cmp "$OUT/m61-implementation-paths.txt" \
  "$OUT/m61-implementation-paths.sorted"
cmp "$OUT/m61-implementation-new-paths.txt" \
  "$OUT/m61-implementation-new-paths.sorted"
comm -23 "$OUT/m61-implementation-new-paths.sorted" \
  "$OUT/m61-implementation-paths.sorted" \
  > "$OUT/m61-implementation-new-paths-not-allowed.txt"
test ! -s "$OUT/m61-implementation-new-paths-not-allowed.txt"
test -z "$(git diff --name-only --diff-filter=A)"
git add --intent-to-add \
  --pathspec-from-file="$OUT/m61-implementation-new-paths.txt"
git diff --name-only --diff-filter=A | LC_ALL=C sort \
  > "$OUT/m61-implementation-intent-actual.txt"
cmp "$OUT/m61-implementation-new-paths.sorted" \
  "$OUT/m61-implementation-intent-actual.txt"
git commit --only \
  --pathspec-from-file="$OUT/m61-implementation-paths.txt" \
  -m "Implement M61 memory measurement tooling"
test "$(git rev-parse HEAD^{tree})" = \
  "$(cat "$OUT/m61-implementation-tree.txt")"
git diff-tree --no-commit-id --name-only -r HEAD^ HEAD \
  | LC_ALL=C sort > "$OUT/m61-implementation-commit-paths.actual"
cmp "$OUT/m61-implementation-paths.sorted" \
  "$OUT/m61-implementation-commit-paths.actual"
test -z "$(git diff --cached --name-only)"
test -z "$(git diff --name-only --diff-filter=A)"
```

Before both closeout reviews, preserve the proposed evidence/documentation tree:

```sh
rm -f "$OUT/m61.index"
GIT_INDEX_FILE="$OUT/m61.index" git read-tree HEAD
GIT_INDEX_FILE="$OUT/m61.index" git add -A \
  --pathspec-from-file="$OUT/m61-pre-review-paths.txt"
GIT_INDEX_FILE="$OUT/m61.index" git write-tree \
  > "$OUT/m61-pre-review-tree.txt"
```

After the two review receipts are copied into the packet, regenerate the
manifest, rebuild the isolated index, and prove the exact three-path delta:

```sh
rm -f "$OUT/m61.index"
GIT_INDEX_FILE="$OUT/m61.index" git read-tree HEAD
GIT_INDEX_FILE="$OUT/m61.index" git add -A \
  --pathspec-from-file="$OUT/m61-paths.txt"
GIT_INDEX_FILE="$OUT/m61.index" git write-tree \
  > "$OUT/m61-final-candidate-tree.txt"
git diff-tree --no-commit-id --name-only -r \
  "$(cat "$OUT/m61-pre-review-tree.txt")" \
  "$(cat "$OUT/m61-final-candidate-tree.txt")" \
  | LC_ALL=C sort > "$OUT/post-review-actual-paths.txt"
LC_ALL=C sort "$OUT/post-review-allowed-paths.txt" \
  > "$OUT/post-review-expected-paths.txt"
cmp "$OUT/post-review-expected-paths.txt" \
  "$OUT/post-review-actual-paths.txt"
```

Run the final candidate checks against that isolated index, then perform only
the path-limited closeout commit:

```sh
python3 -B scripts/check-current-doc-links.py \
  --paths-from "$OUT/touched-current-docs.txt"
python3 -B scripts/verify-packet-manifest.py \
  docs/reports/evidence/m61-native-track-a-memory-owner-reduction/packet-manifest.csv
python3 -B scripts/check-evidence-growth.py \
  --repo-root . --paths-from "$OUT/m61-evidence-paths.txt"
GIT_INDEX_FILE="$OUT/m61.index" git diff --cached --check

test -z "$(git diff --cached --name-only)"
LC_ALL=C sort -u "$OUT/m61-paths.txt" > "$OUT/m61-paths.sorted"
LC_ALL=C sort -u "$OUT/m61-new-paths.txt" > "$OUT/m61-new-paths.sorted"
cmp "$OUT/m61-paths.txt" "$OUT/m61-paths.sorted"
cmp "$OUT/m61-new-paths.txt" "$OUT/m61-new-paths.sorted"
comm -23 "$OUT/m61-new-paths.sorted" "$OUT/m61-paths.sorted" \
  > "$OUT/m61-new-paths-not-allowed.txt"
test ! -s "$OUT/m61-new-paths-not-allowed.txt"
test -z "$(git diff --name-only --diff-filter=A)"
git add --intent-to-add --pathspec-from-file="$OUT/m61-new-paths.txt"
git diff --name-only --diff-filter=A | LC_ALL=C sort \
  > "$OUT/m61-intent-actual.txt"
cmp "$OUT/m61-new-paths.sorted" "$OUT/m61-intent-actual.txt"
git commit --only --pathspec-from-file="$OUT/m61-paths.txt" \
  -m "Close M61 memory owner reduction"
test "$(git rev-parse HEAD^{tree})" = \
  "$(cat "$OUT/m61-final-candidate-tree.txt")"
git diff-tree --no-commit-id --name-only -r HEAD^ HEAD \
  | LC_ALL=C sort > "$OUT/m61-closeout-commit-paths.actual"
cmp "$OUT/m61-paths.sorted" "$OUT/m61-closeout-commit-paths.actual"
test -z "$(git diff --cached --name-only)"
test -z "$(git diff --name-only --diff-filter=A)"
```

After every push that binds measurement or closeout evidence, retain this
literal remote-identity proof (with failure preserved):

```sh
LOCAL=$(git rev-parse HEAD)
REMOTE=$(git ls-remote --exit-code origin refs/heads/main | awk 'NR==1{print $1}')
test "$LOCAL" = "$REMOTE"
```

For the pushed Phase 0B source, create a disposable clone, detach at `$LOCAL`,
and require its clean `HEAD` to equal that expected SHA for the initial owned
set and preserved exploratory red. After the reviewed post-diagnostic amendment
and quality repair are separately pushed and remote-equal, create the local
disposition-B correction as their exact direct child. Its disposable detached
clone must be clean and must label the complete replacement A/B source unpushed.
The separate production candidate is the correction's exact direct child and
also remains local until its exact-source acceptance set is green. After green
acceptance, push both local implementation commits, rerun the remote proof at
the production candidate, and require equality before final evidence reviews.

## Load-Bearing Verification

These test names and owning coverage are binding. If a later source rename
makes one filter empty or invalid, stop and amend/re-review the plan before
measurement rather than silently substituting a narrower gate:

```sh
cargo test -p yune-core --test upstream_luna_pinyin_parity
cargo test -p yune-core upstream_script_surface_segmentation_prunes_stale_raw_identity_overlap
cargo test -p yune-core poet
cargo test -p yune-core packed_syllabary_codes_preserve_order_boundaries_and_owner_accounting
cargo test -p yune-rime-api dictionary_data
cargo test -p yune-rime-api deployment
cargo test -p yune-rime-api --test yune_web m61_luna_zhongdengchangdu_page_zero_matches_pinned_librime
cargo test -p yune-rime-api --test yune_web m59_luna_
cargo test -p yune-rime-api --test yune_web m59_schema_general_reachability_deployment_matrix_default_on_and_explicit_false
npm --prefix apps/yune-web run check:schema-manifest
python3 -B -m unittest scripts/tests/test_native_candidate_parity.py
python3 -B -m unittest scripts/tests/test_native_benchmark_script.py
python3 -B -m unittest scripts/tests/test_m59_evidence_tools.py
python3 -B -m unittest scripts/tests/test_m61_native_mode_contract.py
python3 -B -m unittest scripts/tests/test_m61_luna_poet_rebind.py
python3 -B -m unittest scripts/tests/test_m61_supplemental_ratchet.py
python3 -B -m unittest scripts/tests/test_public_evidence_privacy.py
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The measurement/tooling commit adds
`scripts/tests/test_m61_native_mode_contract.py`. Together with the existing
native-wrapper and M59 aggregator suites, it must cover PowerShell and macOS
selector parsing/defaults, inherited-variable rejection, success/failure
environment restoration before librime and Track B, disposition-specific owner
assertions, uniform `track_a_storage_mode` provenance, the one-build-plus-four-
reuse set, the five-all-prebuilt set, and every mixed/hash/receipt/mode rejection.

The Phase 0A correction adds
`scripts/tests/test_native_candidate_parity.py`. It must exercise a `17/17`
PASS, the preserved shape-valid `16/17` `zhongdengchangdu` text red, and
missing/extra/duplicate/non-contiguous/malformed negative cases. Wrapper tests
must prove that a red comparator prevents round completion; aggregator tests
must prove that every one of five PASS receipts has the expected input list,
tool hash, output hash, and uniform source/oracle identity.

The Phase 0B owner test
`packed_syllabary_codes_preserve_order_boundaries_and_owner_accounting` must
prove exact order, duplicates, empty/non-ASCII codes, borrowed boundary-safe
access, final offset, overflow rejection, unchanged item count, and honest
packed-buffer/offset owner accounting. Existing compact-table, prism,
dictionary-data, deployment, Luna, reachability, and workspace gates prove the
representation does not change observable behavior.

The full workspace gate is load-bearing at closeout because it verifies the
restored runtime tree together with the retained prerequisite and quality
repairs; for an accepted branch it would also cover a default storage change
across deployment, cache lifecycle, compiled-artifact, and candidate behavior
boundaries. Do not run browser latency, Cloudflare, package, Windows
frontend, or iOS suites unless the implementation unexpectedly touches those
surfaces; such a touch normally stops M61 for scope review.

The literal closeout workspace command remains preserved red. Its successful
prefix is retained, and the interrupted plus never-reached result groups are
covered by the source-current disjoint recovery above. This is a complete
workspace-surface proof, not a claim that the original command exited zero.
Do not duplicate the successful prefix or replace either deterministic test-
contract red with a later broad green receipt.

The final native performance command is the binding Windows command shape above
with the exact 17+1 inputs and `9/60/80` settings. The final production set
omits `-TrackAStorageMode` and requires `YUNE_POET_BYTE_BACKED` to be absent.

The measurement/tooling commit also adds
`scripts/check-public-evidence-privacy.py` with unit tests. It accepts the
curated packet allowlist plus a required external `--forbid-literal-file`
created during environment capture and never checked in. That non-empty deny
file contains the actual username, user-profile/home path, host/computer name,
hardware serial, stable machine UUIDs, and normalized variants. The checker
performs case-insensitive literal matching plus generic user-profile path,
email, and secret/token-pattern checks, while allowing model/chip/RAM/OS,
toolchain versions, and source/artifact hashes. It fails closed on missing or
empty inputs and never echoes a forbidden literal into its verdict. Unit tests
cover bare host/user strings, normalized forms, allowed hardware/OS fields, and
verdict redaction. The closeout runs the tested tool against every curated
packet path and records the exact command/verdict.

```sh
python3 -B scripts/check-public-evidence-privacy.py \
  --paths-from "$OUT/m61-evidence-paths.txt" \
  --forbid-literal-file "$OUT/private/forbidden-literals.txt"
```

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
- owned and byte-backed five-round aggregate tables, plus final production
  five-round tables only for an accepted A/B/C disposition;
- `owner-budget.csv`, `owner-delta.csv`, and branch disposition;
- unchanged signed-gate verdict and, only for an accepted A/B/C disposition,
  the separate accepted one-row M61 memory ratchet; disposition D instead
  retains the explicitly unaccepted supplemental projection;
- the preserved source-bound `16/17` prerequisite red and corrected `17/17`
  preflight receipts, including comparator tool/output hashes and the exact
  source, DLL, executable, oracle shared/build, product schema, and model
  identities;
- per-round candidate/model/storage-mode checks and hashed comparator PASS
  receipts;
- test/check summaries;
- requirement/evidence and change-isolation reviews; and
- exact packet manifest with byte size and SHA-256.

Do not check in `m37_metrics.csv`, `samples.csv`, startup traces, binaries,
compiled schemas, POET artifacts, raw benchmark trees, or screenshots. Run the
compact-evidence curator, packet-manifest verifier, current-doc link checker,
evidence-growth guard, the tested public-evidence privacy checker, and
`git diff --check`. The packet remains below the repository's `10 MiB` cap.

## Failure And Retry Policy

- A setup failure before measurement may be fixed and retried under an explicit
  `retry-N-<reason>` name. Preserve the failed setup receipt.
- A deterministic closeout test-contract red is not a memory measurement red.
  Preserve the exact failing command and successful prefix, correct only the
  stale test contract under an explicit path allowlist, then rerun its owning
  target plus every never-reached target. Do not overwrite the red with a broad
  rerun.
- An interrupted runner with no gate verdict is neither green nor red. Record
  `gate_verdict=none`, prohibit reuse, and restart only the still-unproved
  owning slice under a new retry name.
- A measured red is never discarded, renamed as setup, averaged away, or
  cherry-picked. It receives a source-bound disposition.
- A variable DLL or benchmark-executable hash rejects that complete evidence
  set. Preserve it and restart the full set after correction.
- The preserved `f18b0df2` owned Track B session-private red has exactly one
  owner-authorized exception: Phase 0B's packed `compact_table.syllabary_codes`
  correction followed by a wholly new five-round owned set at the reviewed,
  pushed correction source. No `f18b0df2` round is reusable and no second
  prerequisite correction/retry exists. A red at the corrected source blocks
  M61 with this exception exhausted. Push an exact revert of the Phase 0B
  correction, prove remote equality at that revert, retain the correction and
  revert commits plus the complete red evidence, and stop; do not leave the
  unaccepted correction on `main`.
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
