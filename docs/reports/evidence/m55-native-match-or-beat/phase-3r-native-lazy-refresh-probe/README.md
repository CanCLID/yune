# Phase 3R Native Lazy Refresh Probe

Date: 2026-07-04.

Verdict: partial green checkpoint, not M55 closeout.

This probe adds a native C ABI scheduling optimization for exact plain
`luna_pinyin` only: unmodified lowercase ASCII keys are buffered inside the
session and flushed before any observable context/status/input/candidate or
commit boundary. The core `Engine` remains eager, no public C ABI slot changes,
no candidate scoring/order changes, and non-plain profiles such as
`luna_pinyin_octagram` and `jyut6ping3_mobile` keep the existing eager path.

Evidence:

- Full ratchet run:
  `m55-ratchet/`
- Command:
  `m55-ratchet/commands.txt`
- Comparison:
  `m55-ratchet/summary-comparison.csv`
- Threshold result:
  `m55-ratchet/threshold-check.csv`

Key Track A rows from `summary-comparison.csv`:

| Input | Yune median | librime median | Ratio |
| --- | ---: | ---: | ---: |
| `n` | `56.900 us` | `20.500 us` | `2.776x` |
| `ni` | `14.500 us` | `14.100 us` | `1.028x` |
| `hao` | `9.100 us` | `11.333 us` | `0.803x` |
| `ceshiyixiachangjushuruxingnengzenyang` | `3.570 us` | `286.462 us` | `0.012x` |
| `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` | `2.934 us` | `656.419 us` | `0.004x` |
| `zhongguo` | `6.188 us` | `160.912 us` | `0.038x` |
| `cszysmsrsd` | `101.890 us` | `1183.340 us` | `0.086x` |
| `zybfshmsru` | `101.260 us` | `825.650 us` | `0.123x` |

Track A peak working set in this run is `185,806,848 B`. Track B
`jyut6ping3_mobile` key-sequence median is `324.702 us`.

The run passes the current `m55-thresholds.csv` ratchet, but M55 Tier M remains
open:

- `n` is `2.776x`, above the Tier M `<=2.00x` bar.
- Track A peak working set is `185,806,848 B`, above the Tier M
  `<=125,000,000 B` bar.

Owner note for the remaining `n` blocker: the post-probe raw diagnostics show
`n` is dominated by the final short-key translator/context export path, not
long-sequence graph rebuild. In `raw_lookup_microbench.csv`, `n` records
`translator_median_us=53.400` and `context_export_median_us=56.100`.
