# M59 Increment-4e macOS Yune/librime verification

**Status:** diagnostic pass with one disclosed output-location protocol
deviation. Yune tracked source was directly recorded clean before each measured
round, detached at `5879405c7b0f76af4dca7382f00b3e0605386f2c`; after-move
cleanliness is inferred from subsequent clean preflights. Upstream librime was
clean detached at `33e78140250125871856cdc5b42ddc6a5fcd3cd4`.

> **Supersession boundary:** this packet measures Yune `5879405c` (M59
> Increment 4e), source-matched to the `5879405c` Windows packet. Final M59
> behavior source `443cc636` postdates this Mac capture, and current main
> contains later WEB-03 engine work through `7f758fba`. No Mac rerun at either
> later source is claimed. The measurements remain exact for `5879405c` and
> leave every signed Windows ceiling unchanged.

This packet answers the source-matched Increment-4e macOS question. It is
diagnostic evidence only: it does not replace or modify the
signed Windows Increment-0 baseline, change any ceiling, create a macOS
acceptance threshold, or authorize a performance implementation.

## Answer

The old macOS latency deficit does not reproduce at measured source
`5879405c`. All 17 Track A median Yune/librime ratios are below `1.0x`, ranging from
`0.006x` to `0.471x`; every pooled worst is also below `1.0x`. The earlier
`afb7079b` result—6/17 aggregate wins with behavior-confounded long rows—is
historical and is not a description of `5879405c` performance.

Against the source-matched `5879405c` Windows packet, the requested diagnostic
classification is 7 close, 6 notable, and 4 material. The material rows are
`n`, `ni`, and `hao` in macOS's favor and the 37-character ratio in Windows's
favor. The 59-character ratio is notable. These are ratio movements, not
failures: the macOS measurements remain far below `1.0x` and below the retained
signed Windows ceilings diagnostically.

The evidence does not show a Mac-only engine-path discrepancy. Parsed
candidate evidence is exactly identical to the paired Windows packet, both commit-bound
behavior gates pass, and logical model-owner shape matches Windows. Residual
ratio differences are best described as workload-dependent platform/toolchain/
allocator/scheduler scaling plus visible run noise. They are not purely noise,
because both engines' component latencies move systematically; they are not
evidence that librime is intrinsically faster on macOS.

## Identity, machine, and protocol

- Host: MacBook Air Mac17,3, Apple M5, 10 cores, 16 GB RAM; macOS 26.5.1
  (25F80), APFS.
- Power: AC throughout, Low Power Mode disabled, no recorded thermal or
  performance warning.
- Toolchain: Rust/Cargo 1.96.1; Command Line Tools
  `26.6.0.0.1781586589` at `/Library/Developer/CommandLineTools`; Apple clang
  21.0.0. Full Xcode was not selected; `xcodebuild` was unavailable through
  the active Command Line Tools directory.
- Yune: tracked source directly recorded clean before each measured round,
  detached at `5879405c7b0f76af4dca7382f00b3e0605386f2c`. After-move
  cleanliness is inferred from the next clean preflight for runs 1–4 and the
  later clean behavior-gate preflight for run 5. During each measurement Git
  reported only the transient untracked evidence directory described below.
- librime: clean detached
  `33e78140250125871856cdc5b42ddc6a5fcd3cd4`.
- Protocol: official `benchmark-native-rime-inprocess-macos.sh`, 9 startup
  iterations, 60 session iterations, 80 key iterations, product deployment,
  exact 17-input Track A and exact Track B input.
- Yune dylib SHA-256, identical before/after every round:
  `2e822d67e92794dace159b15104035954c6f2aee69e5d917793acb536e1deb56`.
- librime dylib SHA-256, identical before/after every round:
  `5a0b2b308a47141d4c6e0c23a48b3fcfdb49da2d846979cfee359660e1256dc9`.
- Candidate CSV SHA-256, identical in all five rounds:
  `bbad1dbb61f2ead3bd56a5b4888d5269737315a5a5c45fc10de5ce2da3d63408`.

The five complete run roots are now preserved externally:

1. `$HOME/yune-m59-final-macos-20260713/accepted/run-1`
2. `$HOME/yune-m59-final-macos-20260713/accepted/run-2`
3. `$HOME/yune-m59-final-macos-20260713/accepted/run-3`
4. `$HOME/yune-m59-final-macos-20260713/accepted/run-4`
5. `$HOME/yune-m59-final-macos-20260713/accepted/run-5`

Run intervals in UTC were `19:51:48–19:54:48`, `19:56:17–19:59:13`,
`20:40:15–20:43:14`, `20:52:36–20:55:36`, and
`21:00:19–21:03:23` on 2026-07-13. The pause between rounds 2 and 3 was a user
pause, not a discarded measurement.

### Output-location protocol deviation

The unmodified benchmark script restricts its output root to
`docs/reports/evidence/` under the repository. Each round therefore first wrote
to the same untracked transient directory in the disposable worktree, then the
complete directory was moved to the external run root above. Every captured
`environment.txt` records exactly
`?? docs/reports/evidence/m59-final-macos-transient/`; each pre-run check was
clean. Runs 1–4 are inferred clean after the move from the next round's clean
preflight, and run 5 from the later clean behavior-gate preflight. There is no
per-run post-move status file. The Yune and librime hashes remained fixed.

This violates the literal requirement that all generated output stay outside
the repository throughout measurement. A strictly conforming rerun was not
performed because the exact unmodified script cannot target an external root;
it would first require a separately authorized harness change. The five runs
remain useful source-matched diagnostics, but this packet is not labeled a
fully protocol-conforming acceptance packet.

## Complete Track A comparison

Ratio is Yune/librime. “587 Win” is the median of the source-matched Increment-4e
Windows packet. “Signed I0” and “ceiling” are historical signed Windows
diagnostics only. Spread is `(max-min)/min`; every measured round is retained.

| Input | Run 1 | Run 2 | Run 3 | Run 4 | Run 5 | Mac median | Worst | Spread | 587 Win | Difference | Class | Signed I0 | Ceiling |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: |
| `n` | .161 | .110 | .107 | .111 | .153 | .111 | .161 | 50.5% | .208 | -46.6% | material | 2.820 | 3.006 |
| `ni` | .183 | .155 | .139 | .129 | .212 | .155 | .212 | 64.3% | .246 | -37.0% | material | 2.599 | 2.666 |
| `hao` | .189 | .188 | .173 | .166 | .231 | .188 | .231 | 39.2% | .284 | -33.8% | material | 1.720 | 1.844 |
| `zhongguo` | .040 | .040 | .035 | .040 | .051 | .040 | .051 | 45.7% | .038 | +5.3% | close | .293 | .323 |
| 37-character | .029 | .029 | .026 | .029 | .029 | .029 | .029 | 11.5% | .022 | +31.8% | material | 2.132 | 2.339 |
| 59-character | .013 | .012 | .012 | .012 | .015 | .012 | .015 | 25.0% | .010 | +20.0% | notable | 1.681 | 1.748 |
| `cszysmsrsd` | .006 | .006 | .005 | .006 | .007 | .006 | .007 | 40.0% | .005 | +20.0% | notable | .396 | .474 |
| `zybfshmsru` | .009 | .008 | .007 | .008 | .008 | .008 | .009 | 28.6% | .008 | 0.0% | close | .569 | .695 |
| `zh` | .107 | .100 | .077 | .100 | .108 | .100 | .108 | 40.3% | .099 | +1.0% | close | .986 | 1.047 |
| `j` | .419 | .410 | .371 | .421 | .426 | .419 | .426 | 14.8% | .399 | +5.0% | close | 4.000 | 4.372 |
| `yi` | .453 | .420 | .484 | .492 | .471 | .471 | .492 | 17.1% | .434 | +8.5% | close | 5.777 | 6.098 |
| `che` | .118 | .119 | .099 | .126 | .137 | .119 | .137 | 38.4% | .134 | -11.2% | notable | 1.081 | 1.160 |
| `chuang` | .539 | .158 | .171 | .170 | .175 | .171 | .539 | 241.1% | .174 | -1.7% | close | 1.266 | 1.357 |
| `b` | .498 | .337 | .276 | .337 | .396 | .337 | .498 | 80.4% | .380 | -11.3% | notable | 3.439 | 3.775 |
| `ceshi` | .131 | .120 | .101 | .131 | .132 | .131 | .132 | 30.7% | .143 | -8.4% | close | .895 | .966 |
| `zhongdengchangdu` | .022 | .022 | .020 | .022 | .023 | .022 | .023 | 15.0% | .019 | +15.8% | notable | .322 | .342 |
| `dazisudu` | .160 | .157 | .146 | .162 | .187 | .160 | .187 | 28.1% | .144 | +11.1% | notable | 1.034 | 1.098 |

All nine newly signed rows are explicitly present from `zh` through
`dazisudu`. All 17 Mac medians and all 17 pooled worsts are below the unchanged
signed Windows ceilings diagnostically.

Against the historical signed Increment-0 medians, all 17 differences classify
as material (`-84.5%` to `-99.3%`). That view combines a major source-state
change with the platform change and is retained only as historical context. The
exact per-row percentages are in
[`analysis/track-a-17-row-comparison.csv`](./analysis/track-a-17-row-comparison.csv)
and the validated report table.

## 37/59 behavior and latency

- 37-character exact input:
  `ceshiyixiachangjushuruxingnengzenyang`. Its Mac median is `0.029x`, versus
  `0.022x` on source-matched `5879405c` Windows: +31.8%, material.
- 59-character exact input:
  `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong`. Its Mac
  median is `0.012x`, versus `0.010x` on source-matched `5879405c` Windows: +20.0%, notable.
- The 37 page is exactly `測試一下長句輸入性能怎樣 | 測試一下 | 測試儀 | 測試 | 側室`.
- The 59 page is exactly `這個引擎其實應該支持超長句子輸入才能用 | 這個 | 這歌 | 這格 | 這`.
- Both are page zero, size five, `is_last_page=false`, with segmented preedit.
  The focused deployed-path page-order/recomposition gate passes.

The long ratios are higher on Mac than source-matched Windows because librime's
absolute latency improves more strongly: Yune changes by -29.3%/-29.0%, while
librime changes by -45.1%/-43.3%. This is component scaling, not a loss of the
large same-Mac Yune advantage.

## Track B product input

For
`neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung`, the five Mac
medians are `12.352`, `12.491`, `12.936`, `12.811`, and `14.480 us/key`.
Median is `12.811`, worst run median `14.480`, pooled maximum `16.296`, and
spread `17.2%`. The source-matched Windows median is `17.177 us/key`; the Mac
median is 25.4% lower. This absolute comparison is platform-specific and is not
a portable threshold.

Candidate page, comments, geometry, page state, `0xf6589c0c` / `0x822bccba`
checksums, fresh byte-backed/mmap status, and absent/shared-zero POET shape match
M57 and source-matched `5879405c` Windows. The exact page is `你個人經其實應該支援超場句子輸入先可以用 | 你個 | 你 | 呢 | 尼`.

## Candidate and model-owner findings

- Normalized Mac candidate evidence exactly equals all five source-matched
  `5879405c` Windows
  files.
- Track A complete-input page zero matches librime on 16/17 rows. The sole
  mismatch is `zhongdengchangdu` indexes 2–4: Yune emits
  `中的 / 種的 / 重的`; librime emits `中 / 種 / 重`. Windows has the same
  mismatch, so it is a cross-platform engine behavior gap, not a Mac defect.
- Mac Luna uses the accepted checksum pair `0xb3d4e98e / 0x29d56c89`; Windows
  uses its independently accepted `0x16ad0e3e / 0xb967cfef` pair.
- Current logical shape is exact across platforms: 513,353 sentence entries,
  332,604 lookup rows, vocabulary 193, abbreviation vocabulary 11, and
  normal-character index 423.
- Excluding process counters and the path/metadata-bearing `schema.config`
  signature, logical owner profiles are exact. Mac RSS and Windows
  working-set/private/pagefile counters are not interchangeable.
- M57 had the same entry, lookup, and abbreviation counts but a 421,966-entry
  general vocabulary and did not report the normal-character index. The `5879405c` Mac
  peak RSS is directionally lower than M57, but no threshold or Apple
  `phys_footprint` claim is made.

Candidate snapshots cover complete-input page zero. The focused Lane-B fixture
covers 7 inputs, 430 pages, and 2,135 candidates; the long-sentence gate covers
the deployed 37/59 first page and partial-selection recomposition. This does not
claim universal all-prefix/all-page parity.

## Noise, setup, and retained rounds

The setup warmup at
`$HOME/yune-m59-final-macos-20260713/setup/setup-warmup-unmeasured`
was explicitly unmeasured and excluded. No setup failure invalidated a measured
round. A long user pause occurred between rounds 2 and 3. Transient Spotlight,
Chrome, Claude, CloudKit, `duetexpertd`, and UI activity delayed some starts;
round 5 is visibly noisier and remains included. No measured red or noisy round
was discarded. Quiet-machine state was not continuously observed; the available
point-in-time workload snapshots show the listed bursts, so thermal/noise
attribution remains bounded.

The per-round `cargo build --release` check was a no-op for the prebuilt
measured Yune dylib (`0.08–0.09 s`). However, each script-owned `cargo bench`
invocation compiled the Yune crate and benchmark harness for roughly `24–30 s`
before the deploy, Track A Yune, Track A librime, and Track B lanes. Compilation
was sequential and had ended before each lane process ran; the lane loaded the
pre-copied, hash-fixed dylibs. There was no separately recorded cooldown, so
compilation heat and fixed lane order remain thermal/noise boundaries even
though no compilation ran concurrently with a measured lane.

Very small ratios are reported to `0.001`, so one reporting unit can inflate
relative spread; short rows also show scheduler sensitivity. Across all 17
inputs, the median absolute Mac-versus-`5879405c`-Windows change is -30.9% for Yune
and -31.2% for librime. This is why no single engine, thermal state, or operating
system label explains every row.

The efficient sequence intentionally did not add a new universal 17-input
all-prefix/all-page capture, repeat historical Nano allocator toggles, or run
hardware-counter profiling. The `5879405c` Mac candidates equal their Windows peer and both
focused behavior gates pass, so those higher-cost controls are follow-ups only
if finer attribution becomes decision-bearing. Full Xcode/Instruments was not
available through the active developer directory. No setup failure invalidated
a measured round.

## Focused verification

At the exact measured Yune commit and with external build output:

- complete seven-input Lane-B pinned order: 1 passed, 0 failed; 430 pages and
  2,135 candidates;
- deployed 37/59 page order and partial-selection recomposition: 1 passed,
  0 failed.

The detached worktree was clean before and after both checks. Full logs and
commands are under [`analysis/behavior-gates/`](./analysis/behavior-gates/).

## Packet map

- [`analysis/track-a-17-row-comparison.csv`](./analysis/track-a-17-row-comparison.csv):
  complete five-round/source-matched/signed-ceiling table.
- [`analysis/track-a-component-absolute-latency.csv`](./analysis/track-a-component-absolute-latency.csv):
  numerator/denominator platform scaling.
- [`analysis/track-b-five-observations.csv`](./analysis/track-b-five-observations.csv):
  product row and platform boundary.
- [`analysis/artifact-hash-audit.csv`](./analysis/artifact-hash-audit.csv): run
  identities, times, source cleanliness, required artifacts, and hashes.
- [`analysis/validation-checks.csv`](./analysis/validation-checks.csv):
  deterministic aggregation checks.
- [`analysis/toolchain-packaging-check.txt`](./analysis/toolchain-packaging-check.txt):
  read-only Command Line Tools/full-Xcode identity check captured during packet
  packaging.
- [`analysis/findings.md`](./analysis/findings.md): concise generated findings.
- [`analysis/analyze.py`](./analysis/analyze.py): fail-closed aggregation and
  validation. The normalized packet copy requires explicit `--evidence-root`
  and `--repo-root`; the external capture-time original retains its absolute
  defaults as provenance.
- [`report/artifact.json`](./report/artifact.json) and
  [`report/snapshot.sqlite`](./report/snapshot.sqlite): validated technical
  report artifact and bounded query snapshot.
- [`report/mcp-validation.json`](./report/mcp-validation.json) and
  [`report/render-receipt.json`](./report/render-receipt.json): successful
  validation of the canonical artifact and the receipt for the earlier rendered
  revision. The compliance-corrected canonical artifact was not rendered a
  second time; no HTML fallback was generated.
- [`packet-manifest.csv`](./packet-manifest.csv): SHA-256 and byte size for each
  curated packet file except the manifest itself.

The full five-run outputs, binaries, and build trees remain external. This
tracked packet preserves only the decision-bearing aggregate, focused logs,
validated report, and reproducibility metadata. No threshold or baseline file
is copied or modified here.
