# Final-M59 macOS aggregation findings

## Result

All five measured macOS rounds are aggregatable as a diagnostic with one disclosed protocol deviation: the Yune dylib hash stayed `2e822d67e92794dace159b15104035954c6f2aee69e5d917793acb536e1deb56`, the librime dylib hash stayed `5a0b2b308a47141d4c6e0c23a48b3fcfdb49da2d846979cfee359660e1256dc9`, the tracked source was directly recorded clean before each transient output directory was created and after-move cleanliness is inferred from the next clean preflight (runs 1–4) or later clean behavior-gate preflight (run 5), all runs remained on AC power, and the 175-row candidate snapshot was byte-identical across rounds. During measurement Git reported only the untracked transient evidence directory.

Every Track A macOS median ratio is below 1.0 (Yune faster than librime in this driver). Against the source-matched final-M59 Windows medians, 7 rows are close, 6 notable, and 4 material under the requested diagnostic labels. Against the historical signed Increment-0 Windows medians, 0 are close, 0 notable, and 17 material; that comparison combines platform and major source-state changes.

The signed Windows ceilings are used only as a diagnostic reference. All 17 macOS medians and all 17 pooled worst ratios are at or below them; this does not establish or alter any macOS acceptance threshold.

## Explicit rows

- 37-character: macOS median `0.029x`, pooled worst `0.029x`, spread `11.5%`; source-matched Windows median `0.022x` (+31.8%, material).
- 59-character: macOS median `0.012x`, pooled worst `0.015x`, spread `25.0%`; source-matched Windows median `0.010x` (+20.0%, notable).
- n: macOS median `0.111x`, pooled worst `0.161x`, spread `50.5%`; source-matched Windows median `0.208x` (-46.6%, material).
- ni: macOS median `0.155x`, pooled worst `0.212x`, spread `64.3%`; source-matched Windows median `0.246x` (-37.0%, material).
- hao: macOS median `0.188x`, pooled worst `0.231x`, spread `39.2%`; source-matched Windows median `0.284x` (-33.8%, material).

All nine newly signed rows:

- `zh`: macOS `0.100x`; source-matched 587 Windows `0.099x`; delta `+1.0%` (close).
- `j`: macOS `0.419x`; source-matched 587 Windows `0.399x`; delta `+5.0%` (close).
- `yi`: macOS `0.471x`; source-matched 587 Windows `0.434x`; delta `+8.5%` (close).
- `che`: macOS `0.119x`; source-matched 587 Windows `0.134x`; delta `-11.2%` (notable).
- `chuang`: macOS `0.171x`; source-matched 587 Windows `0.174x`; delta `-1.7%` (close).
- `b`: macOS `0.337x`; source-matched 587 Windows `0.380x`; delta `-11.3%` (notable).
- `ceshi`: macOS `0.131x`; source-matched 587 Windows `0.143x`; delta `-8.4%` (close).
- `zhongdengchangdu`: macOS `0.022x`; source-matched 587 Windows `0.019x`; delta `+15.8%` (notable).
- `dazisudu`: macOS `0.160x`; source-matched 587 Windows `0.144x`; delta `+11.1%` (notable).

## Track B product input

The five macOS key-sequence observations are 12.352, 12.491, 12.936, 12.811, 14.480 µs/key. Their median is `12.811 µs/key`, worst run median `14.480 µs/key`, pooled worst sample `16.296 µs/key`, and spread `17.2%`. The source-matched Windows median is `17.177 µs/key`; the macOS median is `-25.4%` different. Absolute latency and memory remain platform-specific diagnostics.

## Absolute-latency decomposition

Across the 17 per-input component rows, the median macOS-versus-source-matched-Windows absolute-latency change is `-30.9%` for Yune and `-31.2%` for librime. This shows that the ratio movement is not attributable to librime alone; both numerator and denominator move across platforms.
- `ceshiyixiachangjushuruxingnengzenyang`: Yune macOS/Windows absolute median delta `-29.3%`; librime `-45.1%`.
- `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong`: Yune macOS/Windows absolute median delta `-29.0%`; librime `-43.3%`.
- `n`: Yune macOS/Windows absolute median delta `-30.9%`; librime `+27.0%`.
- `ni`: Yune macOS/Windows absolute median delta `-33.7%`; librime `+7.7%`.
- `hao`: Yune macOS/Windows absolute median delta `-32.7%`; librime `-0.3%`.

## Stability and interpretation

The following macOS ratio rows exceed 10% five-round spread and must be read as noisy diagnostics: `chuang` (241.1%), `b` (80.4%), `ni` (64.3%), `n` (50.5%), `zhongguo` (45.7%), `zh` (40.3%), `cszysmsrsd` (40.0%), `hao` (39.2%), `che` (38.4%), `ceshi` (30.7%), `zybfshmsru` (28.6%), `dazisudu` (28.1%), `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` (25.0%), `yi` (17.1%), `zhongdengchangdu` (15.0%), `j` (14.8%), `ceshiyixiachangjushuruxingnengzenyang` (11.5%). All measured rounds are retained.

The five-run evidence supports a real Increment-4e source-bound macOS advantage over librime for this benchmark driver, but it does not by itself identify a single platform cause. Cross-platform absolute counters, memory, scheduler effects, thermal/noise effects, and toolchain/ABI effects are confounded. The source-matched component table should be used to choose follow-up controls rather than treating Windows ceilings as portable.

## Reproduction and sources

Run `python3 analyze.py --evidence-root <external-evidence-root> --repo-root <yune-repository-root> --output-dir <external-review-root>/analysis`. Inputs are read from the five external `accepted/run-*` directories and the committed final-M59 Windows, historical signed Increment-0, ceiling, and expanded-derivation CSVs. The script fails on missing/duplicate Track A rows, formula mismatches, source/parameter drift, unexpected output-location status, missing artifacts, or variable binaries.
