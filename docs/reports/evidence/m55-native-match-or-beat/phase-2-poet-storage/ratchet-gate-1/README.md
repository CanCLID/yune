# M55 Phase 2 Ratchet Gate 1 - 2026-07-03

Verdict: no-go. This directory intentionally records a failing full M55
ratchet run after the Phase 2 byte-backed poet product path was wired into the
Track A benchmark flow.

## Command

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 `
  -OutputRoot docs\reports\evidence\m55-native-match-or-beat\phase-2-poet-storage\ratchet-gate-1 `
  -Iterations 9 -SessionIterations 60 -KeyIterations 80 `
  -TrackAInputs "n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru" `
  -DeployProductBeforeBenchmark `
  -TrackAThresholds docs\reports\evidence\m55-native-match-or-beat\thresholds\m55-thresholds.csv `
  -FailOnRegression
```

`commands.txt` records the build command, the untimed Track A deploy-prep
command that generates `luna_pinyin.poet.bin`, and the benchmark invocation.

## Result

`threshold-check.csv` has three failing rows:

- Track A 37-character Luna row:
  `ceshiyixiachangjushuruxingnengzenyang` observed `6.289x`, ceiling `3.267x`.
- Track A 59-character Luna row:
  `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` observed
  `4.333x`, ceiling `2.447x`.
- Track B product guard row:
  `neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung` observed
  `378.449 us`, ceiling `375.253 us`.

Track A memory did pass the current ceiling:

- `track-a-yune/memory-owner-profile.csv` reports
  `process.peak_working_set_high_water = 110198784`.
- `poet.entries_by_code`, `poet.vocabulary`, and
  `poet.abbreviation_vocabulary` are `mmap_file_backed`.
- `product_path_status.csv` reports
  `selected_storage = rsmarisa_byte_backed`,
  `checksum_status = accepted_upstream_marisa_import_checksum`, and
  `table_format = rime_marisa_string_table:1574520`.

## Artifact Policy

No third-party dictionary/model bytes are committed from this run. The
diagnostic `rsmarisa-luna_pinyin-string-table.marisa` probe generated during
the run is intentionally excluded from source control; rerunning the command
regenerates it when needed.
