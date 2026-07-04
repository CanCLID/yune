# M55 Phase 3R-1 Access-Volume Owner Note

Date: 2026-07-04

Input rows:

- `m55_37`: `ceshiyixiachangjushuruxingnengzenyang`
- `m55_59`: `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong`

Command:

```powershell
$env:YUNE_M55_PHASE3R_VOLUME_CSV='..\..\docs\reports\evidence\m55-native-match-or-beat\phase-3r-access-volume\access-volume.csv'
cargo test -p yune-core --test upstream_luna_pinyin_parity capture_phase3r_access_volume_csv -- --ignored --nocapture
Remove-Item Env:YUNE_M55_PHASE3R_VOLUME_CSV
```

Run from the workspace root. The test process runs from the `yune-core` package
directory, so the output path intentionally points two directories back to the
workspace `docs\` tree.

Note: Rust test builds deny unsafe code, so this evidence does not install a
test-local global allocator. The `allocation_count_lower_bound` and
`allocation_bytes_lower_bound` columns count `WordGraphEntry` text
materialization events and text bytes, not process-wide allocator events.

## Summary

| input | storage | span minimum entries | graph entries inserted | vocabulary rows examined | DP states created | beam evictions | graph rebuild ns |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 37-char | owned | 91 | 96 | 10 | 21,418 | 18,058 | 128,500 |
| 37-char | byte-backed | 91 | 96 | 10 | 21,418 | 18,058 | 207,900 |
| 59-char | owned | 161 | 164 | 8 | 40,645 | 34,286 | 131,200 |
| 59-char | byte-backed | 161 | 164 | 8 | 40,645 | 34,286 | 305,300 |

`theoretical_minimum_entries` is the bounded exact dictionary entry count for
the valid emitted spans. It is the practical span minimum available from the
current table/sentence index path.

## Finding

The owned and byte-backed builders perform the same graph work volume for these
rows:

- same index probes;
- same exact entry ranges emitted;
- same bounded table entries considered;
- same graph entries inserted;
- same DP states and beam evictions.

The byte-backed path is still slower per rebuild, so per-touch cost remains real.
But the default owned path is also above Tier M, and the dominant owner exposed
here is DP state volume after a near-minimal graph is built: tens of thousands
of states are created from fewer than 200 materialized graph entries.

Decision: prefix scans and storage residency are not the next owner. Continue to
Phase 3R-2, but target sentence graph/DP work volume rather than another
byte-storage redesign. Phase 3R-3 byte-backing should remain last, after the
owned path reduces the DP/state explosion and proves the full ratchet green.

## Metrics-Off Ratchet

The diagnostic-only volume counters are compiled out of release hot-path
collection sites with `cfg!(debug_assertions)`. The metrics-off standing gate
was rerun after that change:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -Iterations 9 -SessionIterations 60 -KeyIterations 80 -TrackAInputs "n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru" -DeployProductBeforeBenchmark -TrackAThresholds docs\reports\evidence\m52-track-a-guardrails-and-disposition\track-a-thresholds.csv -FailOnRegression -OutputRoot docs\reports\evidence\m55-native-match-or-beat\phase-3r-access-volume\metrics-off-m52-ratchet
```

Result: green. `threshold-check.csv` reports 37-character `3.006x` vs `3.267x`,
59-character `2.288x` vs `2.447x`, and Track A peak `185,921,536 B` vs
`198,000,000 B`.
