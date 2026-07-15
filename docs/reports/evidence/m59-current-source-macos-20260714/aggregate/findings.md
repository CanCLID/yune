# Source-current M59 macOS full benchmark

## Boundary and result

This is a current-production-source macOS diagnostic at Yune `0111cf47c09bfe7a4a3d55a1832f35a55bc59435`, not a reproduction of signed Increment-0 source `457751824b8944676dc44912b9ce31ff29d78403` and not source-matched to final-M59 Windows `443cc636862806e4f0dd1e12ab2e2e45f4189154`.

All five logical rounds passed artifact/protocol checks. Yune stayed `f3365aae19d15b9d7b57dcccd30ce1c77347b8ee96a20f09ab001468074b226c` and librime stayed `1973349f4da44c5b71765f8d064ec30428a0fd42d66c9ae95bdb6dc27cd4eecc`. No measured round was retried or discarded.

All 17/17 Track A macOS median ratios are below 1.0. Against final-M59 Windows 443, classifications are {'material': 4, 'notable': 9, 'close': 4}; against signed Increment-0 Windows, {'material': 17}. These are diagnostic labels, not thresholds.

All 17 macOS pooled-worst ratios are at or below the immutable signed Windows ceilings: `17/17`. This does not create a macOS acceptance gate.

## Complete 17-row comparison

| Input | R1 | R2 | R3 | R4 | R5 | Mac median | Worst | Spread | Win 443 | Δ | Class | Signed I0 | Ceiling | Δ vs I0 | Class |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | --- |
| `n` | 0.140 | 0.122 | 0.127 | 0.183 | 0.157 | 0.140 | 0.183 | 50.0% | 0.212 | -34.0% | material | 2.820 | 3.006 | -95.0% | material |
| `ni` | 0.175 | 0.151 | 0.173 | 0.216 | 0.184 | 0.175 | 0.216 | 43.0% | 0.254 | -31.1% | material | 2.599 | 2.666 | -93.3% | material |
| `hao` | 0.220 | 0.169 | 0.215 | 0.287 | 0.132 | 0.215 | 0.287 | 117.4% | 0.289 | -25.6% | material | 1.720 | 1.844 | -87.5% | material |
| `zhongguo` | 0.029 | 0.029 | 0.030 | 0.034 | 0.031 | 0.030 | 0.034 | 17.2% | 0.038 | -21.1% | notable | 0.293 | 0.323 | -89.8% | material |
| `ceshiyixiachangjushuruxingnengzenyang` | 0.018 | 0.019 | 0.019 | 0.022 | 0.018 | 0.019 | 0.022 | 22.2% | 0.022 | -13.6% | notable | 2.132 | 2.339 | -99.1% | material |
| `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` | 0.008 | 0.008 | 0.008 | 0.009 | 0.007 | 0.008 | 0.009 | 28.6% | 0.010 | -20.0% | notable | 1.681 | 1.748 | -99.5% | material |
| `cszysmsrsd` | 0.004 | 0.004 | 0.004 | 0.004 | 0.002 | 0.004 | 0.004 | 100.0% | 0.005 | -20.0% | notable | 0.396 | 0.474 | -99.0% | material |
| `zybfshmsru` | 0.006 | 0.006 | 0.006 | 0.008 | 0.004 | 0.006 | 0.008 | 100.0% | 0.008 | -25.0% | notable | 0.569 | 0.695 | -98.9% | material |
| `zh` | 0.101 | 0.099 | 0.104 | 0.133 | 0.079 | 0.101 | 0.133 | 68.4% | 0.097 | +4.1% | close | 0.986 | 1.047 | -89.8% | material |
| `j` | 0.423 | 0.394 | 0.421 | 0.553 | 0.435 | 0.423 | 0.553 | 40.4% | 0.406 | +4.2% | close | 4.000 | 4.372 | -89.4% | material |
| `yi` | 0.413 | 0.386 | 0.431 | 0.576 | 0.425 | 0.425 | 0.576 | 49.2% | 0.433 | -1.8% | close | 5.777 | 6.098 | -92.6% | material |
| `che` | 0.110 | 0.098 | 0.103 | 0.146 | 0.119 | 0.110 | 0.146 | 49.0% | 0.132 | -16.7% | notable | 1.081 | 1.160 | -89.8% | material |
| `chuang` | 0.130 | 0.121 | 0.126 | 0.184 | 0.141 | 0.130 | 0.184 | 52.1% | 0.172 | -24.4% | notable | 1.266 | 1.357 | -89.7% | material |
| `b` | 0.349 | 0.322 | 0.344 | 0.495 | 0.375 | 0.349 | 0.495 | 53.7% | 0.375 | -6.9% | close | 3.439 | 3.775 | -89.9% | material |
| `ceshi` | 0.103 | 0.095 | 0.100 | 0.144 | 0.107 | 0.103 | 0.144 | 51.6% | 0.142 | -27.5% | material | 0.895 | 0.966 | -88.5% | material |
| `zhongdengchangdu` | 0.015 | 0.015 | 0.015 | 0.018 | 0.012 | 0.015 | 0.018 | 50.0% | 0.018 | -16.7% | notable | 0.322 | 0.342 | -95.3% | material |
| `dazisudu` | 0.113 | 0.117 | 0.118 | 0.150 | 0.112 | 0.117 | 0.150 | 33.9% | 0.141 | -17.0% | notable | 1.034 | 1.098 | -88.7% | material |

## Explicit findings

- `ceshiyixiachangjushuruxingnengzenyang`: macOS median `0.019x` (Yune `3.118 µs`, librime `167.895 µs`), worst `0.022x`, spread `22.2%`; final-Windows-443 delta `-13.6%` (notable).
- `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong`: macOS median `0.008x` (Yune `3.208 µs`, librime `392.011 µs`), worst `0.009x`, spread `28.6%`; final-Windows-443 delta `-20.0%` (notable).
- `n`: macOS median `0.140x` (Yune `3.000 µs`, librime `21.583 µs`), worst `0.183x`, spread `50.0%`; final-Windows-443 delta `-34.0%` (material).
- `ni`: macOS median `0.175x` (Yune `2.917 µs`, librime `16.834 µs`), worst `0.216x`, spread `43.0%`; final-Windows-443 delta `-31.1%` (material).
- `hao`: macOS median `0.215x` (Yune `2.917 µs`, librime `13.667 µs`), worst `0.287x`, spread `117.4%`; final-Windows-443 delta `-25.6%` (material).
- `zh`: macOS median `0.101x` (Yune `4.479 µs`, librime `44.021 µs`), worst `0.133x`, spread `68.4%`; final-Windows-443 delta `+4.1%` (close).
- `j`: macOS median `0.423x` (Yune `4.542 µs`, librime `10.792 µs`), worst `0.553x`, spread `40.4%`; final-Windows-443 delta `+4.2%` (close).
- `yi`: macOS median `0.425x` (Yune `4.458 µs`, librime `10.604 µs`), worst `0.576x`, spread `49.2%`; final-Windows-443 delta `-1.8%` (close).
- `che`: macOS median `0.110x` (Yune `3.889 µs`, librime `37.083 µs`), worst `0.146x`, spread `49.0%`; final-Windows-443 delta `-16.7%` (notable).
- `chuang`: macOS median `0.130x` (Yune `3.354 µs`, librime `26.660 µs`), worst `0.184x`, spread `52.1%`; final-Windows-443 delta `-24.4%` (notable).
- `b`: macOS median `0.349x` (Yune `4.542 µs`, librime `13.209 µs`), worst `0.495x`, spread `53.7%`; final-Windows-443 delta `-6.9%` (close).
- `ceshi`: macOS median `0.103x` (Yune `3.508 µs`, librime `35.200 µs`), worst `0.144x`, spread `51.6%`; final-Windows-443 delta `-27.5%` (material).
- `zhongdengchangdu`: macOS median `0.015x` (Yune `3.076 µs`, librime `204.453 µs`), worst `0.018x`, spread `50.0%`; final-Windows-443 delta `-16.7%` (notable).
- `dazisudu`: macOS median `0.117x` (Yune `3.219 µs`, librime `27.823 µs`), worst `0.150x`, spread `33.9%`; final-Windows-443 delta `-17.0%` (notable).

## Track B

Mac observations: 5.483, 5.253, 5.607, 5.663, 6.445 µs/key; median `5.607`, worst run median `6.445`, pooled worst sample `7.520`, spread `22.7%`.

Final-M59 Windows 443 observations: 16.900, 16.928, 16.952, 16.784, 16.751 µs/key; median `16.900`. Signed-I0 Windows median is `315.646`. Absolute latency and memory are not portable across platforms.

## Candidate and model-owner comparison to M57

Current same-Mac Yune/librime Track A pages match exactly for `16/17` inputs. The remaining input is `zhongdengchangdu`; geometry and comments match, but Yune emits suffixed phrase candidates where librime emits single-character fallbacks. `windows-zhongdengchangdu-evidence.csv` records the identical mismatch across all `15` Windows M59 increment-4c/4d/4e performance-ratchet runs with exact repository source paths, so this is a deterministic cross-platform Yune engine-path discrepancy, not macOS noise or a macOS-specific defect. Both 37/59 pages match librime exactly.

Candidate pages shared with M57: `17`; exact text/order/geometry/comment matches: `11`. The M57 differences are historical evolution: current 37/59, `n`, and `zhongguo` now match the same-run librime page; `ni`/`hao` text and geometry are unchanged but comments evolved. Track B remains an exact M57 match. Detailed differences are in `candidate-m57-comparison.csv`.

Memory-owner shapes compared: `73`; exact normalized matches: `48`; current-only shapes: `23`; M57-only shapes: `2`. Current-only entries are the bounded lookup/surface caches and model provenance owners added after M57. Track B's byte-backed lookup index mapping changed to smaller index-byte shapes; it did not revert to heap ownership.

Product/checksum rows compared: `3`; exact normalized matches: `1`. Luna is exact. Both Track B rows retain the same checksums, compiled-ready status, byte-backed storage, and mmap table/prism modes; only `byte_source_len` differs from M57. Paths, session IDs, notes, and absolute process counters are excluded where nonportable.

The normalized logical owner and product/checksum shapes are identical across all five current macOS rounds.

## Stability and interpretation

Rows over 10% spread: `n` (50.0%), `ni` (43.0%), `hao` (117.4%), `zhongguo` (17.2%), `ceshiyixiachangjushuruxingnengzenyang` (22.2%), `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` (28.6%), `cszysmsrsd` (100.0%), `zybfshmsru` (100.0%), `zh` (68.4%), `j` (40.4%), `yi` (49.2%), `che` (49.0%), `chuang` (52.1%), `b` (53.7%), `ceshi` (51.6%), `zhongdengchangdu` (50.0%), `dazisudu` (33.9%).

Environment: MacBook Air (Apple M5, 16 GB), macOS 26.5.2 on APFS, Command Line Tools 26.6, Rust/Cargo 1.96.1, AC power with Low Power Mode disabled. No thermal or performance warning was recorded. No `tmutil status` receipt was captured; the boundary process snapshots show `backupd` at 0.0% except for 0.1% at run 2 end. Spotlight activity after checkout/build was allowed to settle before round 1. Point-in-time workload receipts nevertheless record substantial UI activity around round boundaries, especially round 4 (Codex Renderer 67.4%, WindowServer 43.4%, Chrome 16.0%, and Granola 13.3% at its start). This is a material noise confounder, not merely a cosmetic footnote.

The macOS ratio differences cannot be assigned purely to platform because the Mac source is newer than both Windows references and the measured rounds were not continuously quiet. Candidate/order and normalized owner/checksum evidence are used to detect engine-path shape drift; source evolution, platform/toolchain behavior, and workload noise cannot be separated by this run. The same-Mac Yune/librime direction remains robust because every retained pooled-worst ratio is below 1.0, but fine cross-platform attribution and row ranking are not decision-ready.

Ratios and published spreads intentionally use the benchmark's three-decimal ratio observations, matching the signed Windows method. Full-precision recomputation shows that the largest tiny-ratio spreads are primarily genuine round-to-round variance: `cszysmsrsd` is 83.5% and `zybfshmsru` is 78.7% at full precision, versus 100.0% after three-decimal rounding. Rounding can also reduce spread: the 59-character row is 36.0% at full precision versus the published 28.6%. The five published observations remain visible in the table, and the disclosed round-4-high/run-5-low shape—not quantization alone—is the main stability caveat.

The signed ceilings remain unchanged and diagnostic only.
