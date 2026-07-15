# Yune vs librime on macOS — Archived Detail

> Archived on 2026-07-14 when the current measurements, visualizations, and
> bottleneck disposition moved into the single
> [`performance dashboard`](../yune-vs-librime-performance.md). This snapshot
> preserves the source-bound macOS detail available immediately before
> consolidation.

Date: 2026-07-14

## Report role and freshness

This report keeps two questions separate:

1. **Latest same-Mac result:** at measured production source `0111cf47`, how
   much latency does Yune use relative to pinned upstream librime?
2. **Platform effect:** at source-matched `5879405c`, how do Yune/librime
   ratios differ between Mac and Windows?

The first question is newer. The second is the cleaner causal control because
both platforms measure the same Yune source. No report may combine the latest
Mac packet with final-M59 Windows and label the difference “macOS gain.”

All peer ratios are **Yune latency divided by librime latency**:

- below `1.000x`: Yune used less time;
- exactly `1.000x`: latency parity;
- above `1.000x`: Yune used more time.

For example, `0.250x` means Yune used 25% of librime's latency, or 75% less
time. It does not mean “0.25x faster.”

The signed Windows Increment-0 packet and consolidated threshold registry remain
authoritative. Mac-to-Windows comparisons below are diagnostic only; they do
not create a portable Mac threshold or change any signed ceiling.

The latest five-round Mac packet measures clean Yune source
`0111cf47c09bfe7a4a3d55a1832f35a55bc59435` against clean pinned librime
`33e78140250125871856cdc5b42ddc6a5fcd3cd4`, with the exact 17 inputs,
product deployment, and `9/60/80` parameters. The older immutable
source-matched aggregate remains in
[`m59-final-source-macos-20260713/`](../evidence/m59-final-source-macos-20260713/).
The new derived ratio tables and reproducible visuals are in
[`current-ratio-visuals-2026-07-14/`](../evidence/current-ratio-visuals-2026-07-14/).

## Answer

The historical macOS deficit does not reproduce on the latest measured source:

- At `0111cf47`, all 17 Track A medians are below `1.000x`, from `0.004x` to
  `0.425x`; every pooled worst is also below parity, with the largest at
  `0.576x`.
- The 37/59 medians are `0.019x` and `0.008x`; their complete-input pages are
  exact.
- Page zero is exact on 16/17 inputs. The sole `zhongdengchangdu` difference is
  deterministic and cross-platform, not a Mac-only engine-path discrepancy.
- In the older source-matched `5879405c` pair, 7 ratio differences are close, 6
  notable, and 4 material. The material rows are `n`, `ni`, and `hao` in Mac's
  favor and the 37-character ratio in Windows's favor.
- At both measured later Mac sources, the 37/59 pages exactly match librime's
  one-sentence-then-phrases page.
- In the `5879405c` control, Mac candidate evidence exactly matches the
  source-matched Windows files. The sole Track A page-zero difference from
  librime, on `zhongdengchangdu`, is identical on Windows and is therefore not
  a Mac-only path defect.

The old `afb7079b` packet—6/17 aggregate wins, behavior-confounded long-input
timing, and short-input instruction surplus—remains useful historical evidence,
but it is superseded for present direction. Increment 4e's structure-driven
surface graph coincides with the large improvement; the changed commit range
does not isolate one line as the cause. The later `0111cf47` run confirms the
same-Mac direction but, without a source-matched Windows peer, does not improve
platform attribution.

![Latest measured macOS Track A ratios relative to 1.000x librime parity](../evidence/current-ratio-visuals-2026-07-14/visuals/current-macos-track-a-parity.svg)

## Measurement identity and limitations

- MacBook Air Mac17,3; Apple M5, 10 cores, 16 GB RAM; macOS 26.5.2, APFS.
- AC power throughout, Low Power Mode disabled, no recorded thermal or
  performance warning.
- Rust/Cargo 1.96.1; Command Line Tools `26.6.0.0.1781586589`.
- Yune dylib SHA-256, identical before and after all five rounds:
  `f3365aae19d15b9d7b57dcccd30ce1c77347b8ee96a20f09ab001468074b226c`.
- librime dylib SHA-256, identical before and after all five rounds:
  `1973349f4da44c5b71765f8d064ec30428a0fd42d66c9ae95bdb6dc27cd4eecc`.
- No measured round was discarded, and there was no setup retry.
- The external-output adapter recorded the requested paths directly; the older
  move-after-run deviation does not apply to this latest packet.
- Significant UI activity was observed and is a material noise boundary,
  especially in round 4. No thermal or performance warning was recorded.
- The fixed binary identity makes the packet valid same-Mac evidence despite
  that noise. It does not make it a source-matched Windows/Mac experiment.

The benchmark reports small ratios to `0.001`, which amplifies relative spread
on very fast rows. Short rows also show scheduler sensitivity. These limits
discourage fine ranking but cannot erase the all-17-below-`1.000x` result.

## Latest measured 17-row same-Mac result

Every value below is a Yune/librime latency ratio. All five logical rounds are
shown and retained; “worst” is the pooled worst across them.

| Input | R1 Y/L (x) | R2 Y/L (x) | R3 Y/L (x) | R4 Y/L (x) | R5 Y/L (x) | Median (x) | Worst (x) | Spread | Parity read |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `n` | 0.140 | 0.122 | 0.127 | 0.183 | 0.157 | 0.140 | 0.183 | 50.0% | below parity |
| `ni` | 0.175 | 0.151 | 0.173 | 0.216 | 0.184 | 0.175 | 0.216 | 43.0% | below parity |
| `hao` | 0.220 | 0.169 | 0.215 | 0.287 | 0.132 | 0.215 | 0.287 | 117.4% | below parity |
| `zhongguo` | 0.029 | 0.029 | 0.030 | 0.034 | 0.031 | 0.030 | 0.034 | 17.2% | below parity |
| 37-character | 0.018 | 0.019 | 0.019 | 0.022 | 0.018 | 0.019 | 0.022 | 22.2% | below parity |
| 59-character | 0.008 | 0.008 | 0.008 | 0.009 | 0.007 | 0.008 | 0.009 | 28.6% | below parity |
| `cszysmsrsd` | 0.004 | 0.004 | 0.004 | 0.004 | 0.002 | 0.004 | 0.004 | 100.0% | below parity |
| `zybfshmsru` | 0.006 | 0.006 | 0.006 | 0.008 | 0.004 | 0.006 | 0.008 | 100.0% | below parity |
| `zh` | 0.101 | 0.099 | 0.104 | 0.133 | 0.079 | 0.101 | 0.133 | 68.4% | below parity |
| `j` | 0.423 | 0.394 | 0.421 | 0.553 | 0.435 | 0.423 | 0.553 | 40.4% | below parity |
| `yi` | 0.413 | 0.386 | 0.431 | 0.576 | 0.425 | 0.425 | 0.576 | 49.2% | below parity |
| `che` | 0.110 | 0.098 | 0.103 | 0.146 | 0.119 | 0.110 | 0.146 | 49.0% | below parity |
| `chuang` | 0.130 | 0.121 | 0.126 | 0.184 | 0.141 | 0.130 | 0.184 | 52.1% | below parity |
| `b` | 0.349 | 0.322 | 0.344 | 0.495 | 0.375 | 0.349 | 0.495 | 53.7% | below parity |
| `ceshi` | 0.103 | 0.095 | 0.100 | 0.144 | 0.107 | 0.103 | 0.144 | 51.6% | below parity |
| `zhongdengchangdu` | 0.015 | 0.015 | 0.015 | 0.018 | 0.012 | 0.015 | 0.018 | 50.0% | below parity; page differs |
| `dazisudu` | 0.113 | 0.117 | 0.118 | 0.150 | 0.112 | 0.117 | 0.150 | 33.9% | below parity |

## Historical source-matched `5879405c` comparison

“587 Win” is the median from the Increment-4e Windows packet at the same source.
Every latency cell is a Yune/librime ratio in `x`. Difference is the
ratio-of-ratios `(Mac Y/L / Windows Y/L) - 1`; negative means Yune's relative
ratio is lower on Mac. It is not a percent-faster calculation.
Absolute difference up to 10% is close, over 10% through 25% notable, and over
25% material. “Signed I0” and “ceiling” are historical Windows diagnostics.

| Input | Mac R1 Y/L (x) | Mac R2 Y/L (x) | Mac R3 Y/L (x) | Mac R4 Y/L (x) | Mac R5 Y/L (x) | Mac median Y/L (x) | Mac worst Y/L (x) | Spread | 587 Win Y/L (x) | Ratio difference | Class | Signed I0 Y/L (x) | Signed ceiling (x) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: |
| `n` | 0.161 | 0.110 | 0.107 | 0.111 | 0.153 | 0.111 | 0.161 | 50.5% | 0.208 | -46.6% | material | 2.820 | 3.006 |
| `ni` | 0.183 | 0.155 | 0.139 | 0.129 | 0.212 | 0.155 | 0.212 | 64.3% | 0.246 | -37.0% | material | 2.599 | 2.666 |
| `hao` | 0.189 | 0.188 | 0.173 | 0.166 | 0.231 | 0.188 | 0.231 | 39.2% | 0.284 | -33.8% | material | 1.720 | 1.844 |
| `zhongguo` | 0.040 | 0.040 | 0.035 | 0.040 | 0.051 | 0.040 | 0.051 | 45.7% | 0.038 | +5.3% | close | 0.293 | 0.323 |
| 37-character | 0.029 | 0.029 | 0.026 | 0.029 | 0.029 | 0.029 | 0.029 | 11.5% | 0.022 | +31.8% | material | 2.132 | 2.339 |
| 59-character | 0.013 | 0.012 | 0.012 | 0.012 | 0.015 | 0.012 | 0.015 | 25.0% | 0.010 | +20.0% | notable | 1.681 | 1.748 |
| `cszysmsrsd` | 0.006 | 0.006 | 0.005 | 0.006 | 0.007 | 0.006 | 0.007 | 40.0% | 0.005 | +20.0% | notable | 0.396 | 0.474 |
| `zybfshmsru` | 0.009 | 0.008 | 0.007 | 0.008 | 0.008 | 0.008 | 0.009 | 28.6% | 0.008 | 0.0% | close | 0.569 | 0.695 |
| `zh` | 0.107 | 0.100 | 0.077 | 0.100 | 0.108 | 0.100 | 0.108 | 40.3% | 0.099 | +1.0% | close | 0.986 | 1.047 |
| `j` | 0.419 | 0.410 | 0.371 | 0.421 | 0.426 | 0.419 | 0.426 | 14.8% | 0.399 | +5.0% | close | 4.000 | 4.372 |
| `yi` | 0.453 | 0.420 | 0.484 | 0.492 | 0.471 | 0.471 | 0.492 | 17.1% | 0.434 | +8.5% | close | 5.777 | 6.098 |
| `che` | 0.118 | 0.119 | 0.099 | 0.126 | 0.137 | 0.119 | 0.137 | 38.4% | 0.134 | -11.2% | notable | 1.081 | 1.160 |
| `chuang` | 0.539 | 0.158 | 0.171 | 0.170 | 0.175 | 0.171 | 0.539 | 241.1% | 0.174 | -1.7% | close | 1.266 | 1.357 |
| `b` | 0.498 | 0.337 | 0.276 | 0.337 | 0.396 | 0.337 | 0.498 | 80.4% | 0.380 | -11.3% | notable | 3.439 | 3.775 |
| `ceshi` | 0.131 | 0.120 | 0.101 | 0.131 | 0.132 | 0.131 | 0.132 | 30.7% | 0.143 | -8.4% | close | 0.895 | 0.966 |
| `zhongdengchangdu` | 0.022 | 0.022 | 0.020 | 0.022 | 0.023 | 0.022 | 0.023 | 15.0% | 0.019 | +15.8% | notable | 0.322 | 0.342 |
| `dazisudu` | 0.160 | 0.157 | 0.146 | 0.162 | 0.187 | 0.160 | 0.187 | 28.1% | 0.144 | +11.1% | notable | 1.034 | 1.098 |

![Source-matched 587 Mac and Windows ratios relative to 1.000x parity](../evidence/history/performance-ratio-visuals-2026-07-14/visuals/paired-587-macos-windows-parity.svg)

All nine newly signed rows are present from `zh` through `dazisudu`. All Mac
medians and pooled worsts are below the unchanged signed Windows ceilings
diagnostically.

All 17 Mac-versus-historical-signed-Increment-0 differences are material
(`-84.5%` to `-99.3%`), but that comparison combines the later engine-source
change with the platform change. It is historical context, not platform
attribution; the evidence CSV and report artifact retain every exact value.

## 37- and 59-character findings

The 37-character input
`ceshiyixiachangjushuruxingnengzenyang` has a latest Mac median of `0.019x`
(Yune `3.118 us/key`, librime `167.895 us/key`) and pooled worst `0.022x`.
The 59-character input
`zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` has a latest Mac
median of `0.008x` (Yune `3.208 us/key`, librime `392.011 us/key`) and pooled
worst `0.009x`. Both are clearly below `1.000x` parity.

Both complete-input pages are exact, page size five, not last page, with
segmented preedit. In the older source-matched `5879405c` pair, the 37 ratio is
`0.029x` on Mac versus `0.022x` on Windows (+31.8%, material), and the 59 ratio
is `0.012x` versus `0.010x` (+20.0%, notable). In that pair, Yune's absolute
latency improves about 29% on Mac for each long input while librime improves
about 43–45%, so the ratio moves toward parity even though Yune remains well
below it. The historical behavior-confounding interpretation no longer applies.

## Track B product finding

For
`neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung`, the five Mac
observations at `0111cf47` are `5.483`, `5.253`, `5.607`, `5.663`, and
`6.445 us/key`. Median is `5.607`, worst run median `6.445`, pooled maximum
`7.520`, and spread `22.7%`. Track B is a Yune-only product guard: there is no
librime denominator and therefore no `1.000x` peer-parity interpretation.

The Track B candidate page, comments, geometry, and page state exactly match
M57. Checksums and compiled-ready byte-backed/mmap storage states also match;
`byte_source_len` and normalized lookup-index shapes have evolved. For
historical source-matched context only, the `5879405c` Mac median was
`12.811 us/key` versus `17.177 us/key` on Windows. Absolute latency and memory
are platform-specific, so this is not a portable acceptance comparison.

## Candidate, checksum, and model-owner comparison

The normalized current Mac candidate rows are stable across all five rounds.
Track A page zero equals same-run librime on 16/17 complete inputs. The sole difference
is `zhongdengchangdu` indexes 2–4: Yune has `中的 / 種的 / 重的`; librime has
`中 / 種 / 重`. The same difference exists on Windows, so it is a
cross-platform engine behavior gap.

Across the 17 page keys shared with M57—eight librime Track A pages, eight Yune
Track A pages, and one Yune Track B page—11 match exact text, order, geometry,
and comments. The differences describe historical evolution: current 37/59,
`n`, and `zhongguo` now match same-run librime; `ni`/`hao` retain text and
geometry but have evolved comments. Track B remains an exact M57 candidate
match.

Of 73 normalized memory-owner shapes, 48 match exactly, 23 are current-only,
and 2 are M57-only. Twenty-one current-only rows are bounded lookup/surface
caches and model-provenance owners added after M57. The other two current-only
rows replace the two M57-only Track B lookup-record shapes with smaller index
shapes while retaining byte-backed/mmap ownership. Of three product/checksum rows, Luna
matches exactly; both Track B rows retain the same checksums, compiled-ready
state, byte-backed storage, and mmap table/prism modes, with only
`byte_source_len` differing. Normalized owner and product/checksum shapes are
identical across all five current Mac rounds. Mac RSS remains nonportable to
Windows private/pagefile memory, and no Apple `phys_footprint` claim is made.

## Cross-platform interpretation

Across the 17 inputs, the median absolute Mac-versus-`5879405c`-Windows latency
change is -30.9% for Yune and -31.2% for librime. Row shapes differ:

- On `n`/`ni`/`hao`, Yune's absolute latency changes by `-30.9%`, `-33.7%`,
  and `-32.7%`, while librime changes by `+27.0%`, `+7.7%`, and `-0.3%`;
  the ratio materially favors Mac.
- On 37/59, both engines are faster on Mac, but librime scales more strongly;
  the ratio favors Windows while the same-Mac Yune advantage remains large.

This exact-source pair does not support a librime-only explanation. The latest
`0111cf47` packet reinforces the same-Mac direction but combines source,
platform/toolchain, and UI-noise differences when compared with Windows. No
evidence indicates a real Mac-only engine-path discrepancy, and no performance
fix is recommended from these packets. If finer attribution becomes
decision-bearing, first capture a quiet exact-source Windows/Mac pair, then
profile one short and one long row with fixed binaries and low-perturbation
cycle/instruction counters.

## Related evidence

- [`evidence/m59-final-source-macos-20260713/`](../evidence/m59-final-source-macos-20260713/):
  source-bound Increment-4e diagnostic, disclosed output-location deviation, and
  validated report artifact.
- [`evidence/current-ratio-visuals-2026-07-14/`](../evidence/current-ratio-visuals-2026-07-14/):
  derived current-Mac ratios plus reproducible parity-centered visual sources.
- [`evidence/history/performance-ratio-visuals-2026-07-14/`](../evidence/history/performance-ratio-visuals-2026-07-14/):
  archived source-matched platform and behavior-control charts.
- [`evidence/m59-post-fix-root-cause-20260711/`](../evidence/m59-post-fix-root-cause-20260711/):
  historical `afb7079b` work-volume and allocator diagnosis.
- [`evidence/m57-macos-track-a-sentence-model-parity/`](../evidence/m57-macos-track-a-sentence-model-parity/):
  repaired Mac checksum/model-construction defect.
- [`yune-vs-librime-performance.md`](../yune-vs-librime-performance.md): overall
  platform/lane scorecard.
- [`2026-07-14-yune-vs-librime-root-cause-analysis-pre-consolidation.md`](./2026-07-14-yune-vs-librime-root-cause-analysis-pre-consolidation.md):
  causal history and present disposition.

No number in this report modifies the signed M59 Windows baseline or threshold
registry.
