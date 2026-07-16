# M61 Native Track A Memory-Owner Reduction

> **Milestone:** M61. **Status:** Finalized and authorized for execution
> (2026-07-16). **Track:** native engine performance, Windows Track A
> `luna_pinyin` acceptance lane. **Created:** 2026-07-15; **independently
> reviewed and corrected:** 2026-07-15; **source-bound finalization:**
> 2026-07-16. **Type:** attribution-first measurement and conditional reduction
> plan.

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

1. one exact clean M61 measurement-tooling commit, descended from the
   source-current post-M60 kickoff base through the recorded plan-finalization
   commit and containing no production behavior change, is reproduced in five
   fixed-binary Windows owned-mode rounds and five fixed-binary byte-backed
   diagnostic rounds;
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

M60 is complete and pushed. Its formal closeout commit is
`0eff06a088992f417602a71300c447cdfa525255`, with tree
`cbffa328e9ca7a1ea04187a67349d977bc731b62`. The current post-closeout
formalism correction and M61 kickoff base is
`bc0df36a6eee3ad63319d8c29336542082559c94`, with tree
`523ab0e5f3a8aa67f807a07586591c92f9ef1ead`. The latter changes M60
documentation, registry wording, and focused tests only; it does not change
production runtime behavior. M61 is the sole authorized execution milestone
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
| `M61-ATTR-01` | Complete | Complete with measured no-go: the evidence proves that no owner/reconciliation eligible for implementation exists |
| `M61-BRANCH-01` | Complete | Complete: disposition D is selected and no production branch follows |
| `M61-REDUCE-01` | Complete | Closed by no-go; no production reduction is claimed |
| `M61-COMPAT-01` | Complete | Complete: the green owned baseline and unchanged production source preserve the compatibility boundary |
| `M61-RATCHET-01` | Complete | Closed by no-go; the frozen supplemental ratchet remains unclaimed and the historical registry remains unchanged |
| `M61-EVIDENCE-01` | Complete | Complete, including the measured-red/no-owner disposition and both reviews |

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

The owned/byte-backed A/B source is the exact pushed M61 measurement-tooling
commit created after Phase 0, not either M60 boundary. Its implementation parent
boundary is the source-current kickoff base `bc0df36a`; record the formal M60
closeout, the post-closeout correction, and the plan-finalization commit
separately. After excluding the plan/requirements/review changes, the
non-documentation implementation diff from `bc0df36a` may contain only the
diagnostic selector, missing attribution, fixed-binary aggregator provenance
support, supplemental-ratchet evaluator, and public-evidence privacy checker
required below. The final production candidate receives its own exact-source
five-round set.

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
- [ ] At execution kickoff, record the exact plan-finalization and
      measurement-tooling commit chain, final path allowlist, external evidence
      root, Windows toolchain/machine identity, power/thermal state, significant
      workloads, and pinned oracle/artifact hashes.
- [ ] Inventory and fingerprint every unrelated staged, unstaged, and untracked
      path at kickoff. Instantiate the corrected M60 isolated-index,
      pre-review-tree, exact-review-delta, path-limited-commit, commit-tree, and
      remote-equality procedure with M61 filenames; do not retain the old
      config-only staged-path assertion if the actual inventory differs.

If the pushed finalization source does not descend from the kickoff base, the
Phase 0 dirty-state inventory contains any unexplained or M61-overlapping path,
the current benchmark no longer reads context per key, the Windows acceptance
machine is unavailable, or the relevant owner cannot be measured, stop before
M61 measurement.

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
or choose different candidate behavior. The measurement-tooling commit keeps
the omitted/default owner assertion at the current owned/no-`poet_bin` state;
only a later accepted production-candidate commit may change the normal signed
assertion to require validated `poet_bin` storage.

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
$EXPECTED_MEASUREMENT_SHA = "<pushed M61 measurement-tooling SHA>"
$EXPECTED_MEASUREMENT_TREE = "<pushed M61 measurement-tooling tree>"
$EXPECTED_LIBRIME_SHA = "33e78140250125871856cdc5b42ddc6a5fcd3cd4"
$MEASUREMENT_SHA = (& git -C $REPO rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $MEASUREMENT_SHA -ne $EXPECTED_MEASUREMENT_SHA) {
    throw "measurement source mismatch: expected $EXPECTED_MEASUREMENT_SHA, got $MEASUREMENT_SHA"
}
$MEASUREMENT_TREE = (& git -C $REPO rev-parse "HEAD^{tree}").Trim()
if ($LASTEXITCODE -ne 0 -or $MEASUREMENT_TREE -ne $EXPECTED_MEASUREMENT_TREE) {
    throw "measurement tree mismatch: expected $EXPECTED_MEASUREMENT_TREE, got $MEASUREMENT_TREE"
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

For a production candidate, start a fresh PowerShell process in a new clean
detached clone and use this complete block. It deliberately rebinds every source
and root, builds the candidate DLL and benchmark anew, rejects the diagnostic
environment variable, and never supplies `-TrackAStorageMode`:

```powershell
$ErrorActionPreference = "Stop"
$REPO = (Resolve-Path .).Path
$EXPECTED_CANDIDATE_SHA = "<local M61 production-candidate SHA>"
$EXPECTED_CANDIDATE_TREE = "<local M61 production-candidate tree>"
$EXPECTED_LIBRIME_SHA = "33e78140250125871856cdc5b42ddc6a5fcd3cd4"
$CANDIDATE_SHA = (& git -C $REPO rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $CANDIDATE_SHA -ne $EXPECTED_CANDIDATE_SHA) {
    throw "candidate source mismatch: expected $EXPECTED_CANDIDATE_SHA, got $CANDIDATE_SHA"
}
$CANDIDATE_TREE = (& git -C $REPO rev-parse "HEAD^{tree}").Trim()
if ($LASTEXITCODE -ne 0 -or $CANDIDATE_TREE -ne $EXPECTED_CANDIDATE_TREE) {
    throw "candidate tree mismatch: expected $EXPECTED_CANDIDATE_TREE, got $CANDIDATE_TREE"
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

- [ ] Create a disposable clean detached clone of the exact pushed M61
      measurement-tooling commit. Do not switch an existing tree, create a
      branch, or use the dirty main checkout as measurement evidence. Record
      the M60 parent boundary and clean status before and after every accepted
      set, record the intervening plan-only commit chain, and remove the clone
      only after the external packet is secured.
- [ ] Before building, require `git rev-parse HEAD` to equal the recorded pushed
      M61 measurement SHA, require `git status --porcelain` to be empty, and
      record `git ls-remote origin refs/heads/main` plus containment/equality at
      measurement start. A mismatch is a setup failure, not usable evidence.
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

- [ ] Name at least `10,000,000 B` of current non-overlapping reducible memory.
- [ ] Reconcile the owned-to-byte-backed whole-process private delta under the
      formula and tolerance above without double-counting process, mapped, or
      derived residual rows.
- [ ] Verify whether the likely owner still consists of the retained preset
      vocabulary recipe, `poet.vocabulary`, `poet.entries_by_code`, and the
      small `poet.abbreviation_vocabulary` reducible leaf. Report the guarded
      `translator.upstream_sentence_model_preset_vocabulary_recipe` and
      `poet.lookup_index` rows separately; neither is included in the
      `heap_owned_reducible` coverage sum unless fresh tooling explicitly and
      validly reclassifies it.
- [ ] Prove that the proposed reduction affects same-process Windows private
      bytes rather than only clean mapped residency or page-cache luck.
- [ ] Select exactly one disposition:
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

- [ ] Preserve the current versioned `YUNE-POET/3` validation and loud rejection
      of present truncated, checksum-mismatched, invalid, wrong-version, or
      legacy artifacts, including `YUNE-POET/1` and `YUNE-POET/2`. Bump the
      format only if bytes or semantics actually change.
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
- [ ] Retain and rerun the existing POET v3 format, corruption/legacy/checksum,
      deployment/reuse/copy/rebuild, cache replacement, owner shape, every
      growing 37/59 prefix, pinned-oracle byte-backed result, translator
      invalidation, and lifecycle coverage. Add only gaps introduced by M61:
      no-environment native default selection, explicit owned diagnostic
      override, absent-artifact owned fallback, invalid/stale fail-closed
      behavior without cached reuse, native-target versus WASM capability
      isolation, wrapper selector/owner-shape assertions, and lifecycle clearing
      under the new default.
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
   for M61. The default signed invocation must remain identical. Push this
   commit, prove remote `main` equals its SHA, and only then create the exact
   detached measurement clone.
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
   preserve the fingerprinted unrelated staged/unstaged/untracked state exactly.

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
the candidate tree and repeating both reviews.

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

For the pushed measurement-tooling source, create a disposable clone, detach at
`$LOCAL`, require the clone's `HEAD` to equal that expected SHA, and require
`git status --porcelain=v1 --untracked-files=all` to be empty before and after
the complete A/B set. The conditional production candidate remains local until
its exact-source acceptance set is green; its local detached clone and receipts
must label it unpushed. Push and run the remote proof only after acceptance.

## Load-Bearing Verification

These test names and owning coverage are binding. If a later source rename
makes one filter empty or invalid, stop and amend/re-review the plan before
measurement rather than silently substituting a narrower gate:

```sh
cargo test -p yune-core --test upstream_luna_pinyin_parity
cargo test -p yune-core poet
cargo test -p yune-rime-api dictionary_data
cargo test -p yune-rime-api deployment
cargo test -p yune-rime-api --test yune_web m59_luna_
cargo test -p yune-rime-api --test yune_web m59_schema_general_reachability_deployment_matrix_default_on_and_explicit_false
npm --prefix apps/yune-web run check:schema-manifest
python3 -B -m unittest scripts/tests/test_native_benchmark_script.py
python3 -B -m unittest scripts/tests/test_m59_evidence_tools.py
python3 -B -m unittest scripts/tests/test_m61_native_mode_contract.py
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

The full workspace gate is load-bearing at closeout because a default storage
change crosses deployment, cache lifecycle, compiled-artifact, and candidate
behavior boundaries. Do not run browser latency, Cloudflare, package, Windows
frontend, or iOS suites unless the implementation unexpectedly touches those
surfaces; such a touch normally stops M61 for scope review.

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
evidence-growth guard, the tested public-evidence privacy checker, and
`git diff --check`. The packet remains below the repository's `10 MiB` cap.

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
