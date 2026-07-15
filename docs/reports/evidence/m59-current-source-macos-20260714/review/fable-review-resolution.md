# Fable review resolution

Fable's review found no blocker or major issue. All four minor findings are
resolved in the generated analysis, independent-review record, and portable
report.

1. **Spread quantization:** the report now distinguishes published spreads over
   the benchmark's three-decimal ratios from full-precision sensitivity. The
   full-precision spreads are `83.5%` for `cszysmsrsd`, `78.7%` for
   `zybfshmsru`, and `36.0%` for the 59-character row, versus published values
   of `100.0%`, `100.0%`, and `28.6%`. The disclosed round-4-high/run-5-low
   pattern, not quantization alone, remains the principal noise caveat.
2. **Windows evidence for `zhongdengchangdu`:**
   `aggregate/windows-zhongdengchangdu-evidence.csv` now records all 15
   increment-4c/4d/4e performance-ratchet observations, their exact repository
   source paths, and both candidate pages. Every row reproduces the same Yune
   suffix mismatch seen on macOS.
3. **Backup wording:** the unsupported statement that Time Machine was idle was
   removed. The report now states exactly what was captured: no `tmutil status`
   receipt, with boundary process snapshots showing `backupd` at `0.0%` except
   for `0.1%` at run 2 end.
4. **Percentile terminology:** the review now names the benchmark's actual
   `ceil((n-1)·p)` index rule and notes that `p=0.5` selects the upper median.

After these corrections, the aggregate generator completed, the full packet
manifest was regenerated and checked, and the portable report passed canonical
desktop/narrow viewport verification plus keyboard source interaction.
