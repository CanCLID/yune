# M59 post-fix macOS performance root-cause diagnostic

**Status:** Current-main diagnostic at Yune
`afb7079b71f7f9353845114ff3e310c0a38b9b87` against pinned upstream librime
`33e78140250125871856cdc5b42ddc6a5fcd3cd4`. This packet supersedes the
performance interpretation of the earlier `89875ee2` post-page-order report;
it does not supersede that report's historical repair evidence.

This is diagnostic evidence only. It creates no milestone, changes no signed
Windows ceiling or baseline, grants no exception, and authorizes no
performance implementation. The signed Increment-0 comparison source remains
`457751824b8944676dc44912b9ce31ff29d78403`.

## Answer

Yune does not yet have a broad, behavior-normalized macOS advantage over
librime. Current main wins 6 of the 17 aggregate Track A rows and loses 11.
The result is not explained by one macOS bug:

- The 37- and 59-character aggregate ratios are `0.399x` and `0.205x`, but
  these sequence metrics include every incomplete prefix. Candidate text
  matches librime on only 19/37 and 30/59 prefixes. Text-different prefixes
  consume 82.0% and 90.1% of librime's reconstructed time.
- On candidate-text-matched prefixes, Yune is instead `1.420x` and `1.204x`
  librime. At the final key it is `1.713x` and `1.139x`. Comments and preedit
  still prevent full semantic equivalence, so these remain sensitivity results,
  not acceptance metrics.
- Short inputs execute substantially more Yune work: `n` uses `8.682x` the
  instructions and `10.730x` the cycles; `zh` uses `4.092x` and `5.061x`.
  Symbolized samples qualitatively locate that work in compact-table/MARISA
  traversal and abbreviation/sentence-model generation.
- librime's demand-driven translation, merge, uniqueness, dictionary, and page
  streams are the clearest design lesson. Yune currently selects, owns,
  filters, sorts, and stores a surplus batch before exporting five candidates.
  The likely benefit is not yet an isolated causal effect, and current `n`/`zh`
  behavior must be locked to the governing oracle before optimization.
- Disabling macOS Nano allocation moves stable medium/long Yune/librime ratios
  roughly 6-14% in Yune's favor because librime slows more. This is a real but
  partial platform effect, not an explanation for the behavior and instruction
  differences.
- Thermal/noise-only and platform-only explanations are both rejected as the
  primary cause. An exact-current Windows/macOS matched lane is still needed
  to separate CPU, compiler, linker, allocator, OS, and source effects.

## Reconciliation with the earlier report

The earlier post-review-fix packet measured Yune `89875ee2` and reported
37/59 ratios `2.428x` and `1.809x`. Current main includes the reconciled
Increment-4a mechanism and reports `0.399x` and `0.205x`. Yune sentence-graph
time falls from `358.630` to `4.605 us/key` on the 37-character input and from
`646.191` to `5.328 us/key` on the 59-character input. This proves that the
old performance read is stale, but the commit range is too broad to assign the
delta to one line.

The new aggregate wins are also not behavior-normalized: almost all Yune-faster
long prefixes emit different candidate text. The page-order repair is real;
the aggregate speed claim is not yet comparable work.

## Complete Track A ratio observations

Ratio is Yune/librime; below `1.0x` favors Yune. Every measured round is
retained. Windows values and classifications are available in
[`analysis/output/track-a-ratio-comparison.csv`](./analysis/output/track-a-ratio-comparison.csv).

| Input | Run 1 | Run 2 | Run 3 | Run 4 | Run 5 | Median | Pooled worst | Spread |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `n` | 3.844 | 4.123 | 4.427 | 3.725 | 4.253 | 4.123 | 4.427 | 18.8% |
| `ni` | 2.986 | 2.843 | 2.978 | 2.586 | 2.996 | 2.978 | 2.996 | 15.9% |
| `hao` | 2.261 | 2.019 | 2.764 | 1.902 | 1.797 | 2.019 | 2.764 | 53.8% |
| `zhongguo` | 1.013 | 0.992 | 1.075 | 0.981 | 0.983 | 0.992 | 1.075 | 9.6% |
| `ceshiyixiachangjushuruxingnengzenyang` | 0.415 | 0.396 | 0.411 | 0.395 | 0.399 | 0.399 | 0.415 | 5.1% |
| `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` | 0.215 | 0.205 | 0.207 | 0.203 | 0.203 | 0.205 | 0.215 | 5.9% |
| `cszysmsrsd` | 0.647 | 0.639 | 0.625 | 0.614 | 0.626 | 0.626 | 0.647 | 5.4% |
| `zybfshmsru` | 0.922 | 0.908 | 0.876 | 0.876 | 0.879 | 0.879 | 0.922 | 5.3% |
| `zh` | 3.311 | 3.348 | 3.209 | 3.249 | 3.261 | 3.261 | 3.348 | 4.3% |
| `j` | 7.519 | 7.494 | 6.817 | 7.367 | 7.462 | 7.462 | 7.519 | 10.3% |
| `yi` | 4.102 | 4.215 | 4.558 | 3.966 | 4.085 | 4.102 | 4.558 | 14.9% |
| `che` | 2.844 | 2.671 | 3.002 | 2.592 | 2.654 | 2.671 | 3.002 | 15.8% |
| `chuang` | 2.540 | 2.550 | 2.565 | 2.453 | 2.466 | 2.540 | 2.565 | 4.6% |
| `b` | 6.361 | 6.213 | 7.057 | 6.118 | 6.051 | 6.213 | 7.057 | 16.6% |
| `ceshi` | 1.389 | 1.458 | 1.452 | 1.459 | 1.428 | 1.452 | 1.459 | 5.0% |
| `zhongdengchangdu` | 0.410 | 0.416 | 0.390 | 0.398 | 0.394 | 0.398 | 0.416 | 6.7% |
| `dazisudu` | 1.566 | 1.564 | 1.469 | 1.519 | 1.550 | 1.550 | 1.566 | 6.6% |

## Track B and M57 comparison

Track B's first page, comments, geometry, and page state are byte-identical to
both retained M57 passes and all five current rounds. Its candidate CSV hash is
`5fe17ccb53dd8ee40d9ceeb00dc9c7aab0cc30e3dfdf8216914cf675b7e597e9`.
The current run medians are `265.696`, `264.941`, `263.880`, `268.620`, and
`263.914 us/key`: median `264.941`, worst run median `268.620`, spread 1.8%.
Runs 4 and 5 retain `559.840` and `642.907 us/key` tail observations, so no
tail-latency improvement is claimed.

Both dictionary checksums, compiled readiness, byte-backed/mmap storage,
entry counts, and absent/shared-zero POET owners remain stable. Work shape did
move: Track-B-specific candidate materialization is +95.2% and bounded
selection +110.9%, while global owned candidates and sorting fall 48-51%.
Those counters have different definitions and are not additive.

## Future milestone inputs

No milestone is created here. The evidence ranks future work as:

1. Lock incremental-prefix candidate text/order, comments, preedit, pagination,
   and selection identity to the governing oracle.
2. Add lower-perturbation translator-residual and producer attribution.
3. Prototype behavior-locked, filter-aware, resumable lazy page filling.
4. Reduce duplicate MARISA/abbreviation work on short keys after behavior is
   resolved.
5. Recover correct deployed byte-backed POET behavior, then measure memory and
   CPU separately.
6. Run the same current commit and payload on Windows and macOS.
7. Follow up Track B overfetch while preserving the exact M57 product guard.

## Identities and validation

- Yune commit: `afb7079b71f7f9353845114ff3e310c0a38b9b87` (clean detached source).
- librime commit: `33e78140250125871856cdc5b42ddc6a5fcd3cd4` (clean detached source).
- Yune dylib SHA-256:
  `3dd5a414c68f7884884c5dc172b3f0b088d1f5ae19cb983eb0eeb2f95bc6c710`.
- librime dylib SHA-256:
  `743acf3e3a0b64f94680a2f822b00ae42d35ce1e2ab3c8994441bc305adaf8f6`.
- All five post-run hashes remained identical.
- Local report validation: 294/294 checks passed.
- MCP artifact validation and the single report render passed.
- Two independent reviews found no remaining completeness or analytical/chart
  blocker.

The five accepted run records are preserved at [`run-1`](./accepted-baseline/run-1/),
[`run-2`](./accepted-baseline/run-2/), [`run-3`](./accepted-baseline/run-3/),
[`run-4`](./accepted-baseline/run-4/), and [`run-5`](./accepted-baseline/run-5/).

## Preserved setup failures and limitations

- An unmeasured Spotlight-heavy setup warmup was excluded and preserved before
  accepted measurement.
- Timing-control attempt 0 failed before measurement on runner permissions and
  was retried explicitly.
- The first allocator continuation rejected macOS `env -u` syntax and was
  rerun under an explicit retry root; completed earlier families were kept.
- The deployed byte-backed POET timing lane emitted zero candidates on all 99
  prefixes. Its timing rows are preserved but rejected as behavior-invalid.
- Full Instruments, DTrace, and unattended `powermetrics` were unavailable.
  `/usr/bin/sample` profiles are qualitative because of warmup synchronization,
  fixed order, active UI/Spotlight load, and low librime-`n` sample count.

## Packet map

- [`report/artifact.json`](./report/artifact.json): canonical bounded report.
- [`report/snapshot.sqlite`](./report/snapshot.sqlite): queryable report rows.
- [`report/report-source-notes.md`](./report/report-source-notes.md): chart and
  claim boundaries.
- [`report/report-validation.json`](./report/report-validation.json) and
  [`report/validation-checks.csv`](./report/validation-checks.csv): final local
  validation.
- [`report/mcp-validation.json`](./report/mcp-validation.json) and
  [`report/render-receipt.json`](./report/render-receipt.json): reader validation
  and render receipts.
- [`accepted-baseline/`](./accepted-baseline/): curated five-round environment,
  commands, summaries, candidates, M37 counters, owner profiles, product status,
  verdicts, commit identities, and binary hashes. Raw samples and binaries stay
  outside Git.
- [`analysis/`](./analysis/): bounded source tables and validation receipts used
  by the report.
- [`packet-manifest.csv`](./packet-manifest.csv): SHA-256 and byte size for
  every curated packet file except the manifest itself.

The full raw control workspace is about 43 GB and remains external. This
tracked packet intentionally preserves the decision-bearing evidence without
checking in build trees, dylibs, raw high-iteration timing roots, or profiler
captures. The copied builder and validator preserve their original absolute
workspace paths as provenance; the bounded `artifact.json`/`snapshot.sqlite`
source contract uses portable logical paths. Re-running the full transformation
requires the external raw root or an explicit path remap.
