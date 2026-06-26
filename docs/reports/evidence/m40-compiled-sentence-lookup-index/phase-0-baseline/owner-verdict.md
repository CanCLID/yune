# M40 Phase 0 Owner Verdict

Date: 2026-06-26

Scope: native engine evidence only. This baseline does not make browser,
frontend, product delivery, packaging, or public-demo speed claims.

## Command

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\m40-compiled-sentence-lookup-index\phase-0-baseline -Iterations 9 -SessionIterations 20 -KeyIterations 20 -TrackAInputs "ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru" -TrackBInputs "neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung" -DeployProductBeforeBenchmark
```

## Fresh Baseline Latency

| Row | Yune median | Same-run librime median | Ratio |
| --- | ---: | ---: | ---: |
| startup/runtime-ready | `25,456.100 us` | `28,912.300 us` | `0.880x` |
| session create/select/destroy | `25,421.500 us` | `31,227.700 us` | `0.814x` |
| `ni` | `56.300 us` | `14.750 us` | `3.817x` |
| `hao` | `38.033 us` | `11.833 us` | `3.214x` |
| `zhongguo` | `59.525 us` | `182.562 us` | `0.326x` |
| `ceshiyixiachangjushuruxingnengzenyang` | `500.249 us` | `294.041 us` | `1.701x` |
| `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` | `898.641 us` | `684.581 us` | `1.313x` |
| `cszysmsrsd` | `29.270 us` | `1,247.160 us` | `0.023x` |
| `zybfshmsru` | `32.370 us` | `878.300 us` | `0.037x` |
| Track B `neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung` | `187.541 us` | N/A | guard row |

## Owner Counters

Per-key medians from `m37_metrics.csv`.

| Row | `upstream_sentence_model_ns` | Code-prefix checks | Table entries considered | Vocabulary entries considered | Graph edges | Bounded page candidates | Full-list fallback |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `ceshiyixiachangjushuruxingnengzenyang` | `432.354 us/key` | `241.054/key` | `3,564.216/key` | `0/key` | `3,564.216/key` | `0.135/key` | `0/key` |
| `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` | `806.671 us/key` | `608.576/key` | `6,344.559/key` | `0/key` | `6,344.559/key` | `0.085/key` | `0/key` |
| `cszysmsrsd` | `5.930 us/key` | `43.800/key` | `0/key` | `0/key` | `0/key` | `0/key` | `0.900/key` |
| `zybfshmsru` | `7.240 us/key` | `43.800/key` | `22.800/key` | `0/key` | `22.800/key` | `0/key` | `0.900/key` |
| Track B guard row | `0 us/key` | `0/key` | `0/key` | `0/key` | `0/key` | `0/key` | `0.934/key` |

## Storage Baseline

Track A `luna_pinyin` remains on the M38/M39 storage path:

- `selected_storage=rsmarisa_byte_backed`
- `table_mapping_mode=mmap`
- `prism_mapping_mode=mmap`
- `source_fallback=false`
- `table_heap_mirror_bytes=0`
- `prism_heap_mirror_bytes=0`
- `rsmarisa_status=ok`
- `rsmarisa_mapping_mode=mmap`
- `rsmarisa_num_keys=463586`

## Verdict

Implementation may proceed. The fresh baseline still places both Track A long
rows inside `upstream_sentence_model_ns`, with the 59-character row spending
`806.671 us/key` in the owner and visiting `608.576` code prefixes plus
`6,344.559` table entries per key. This matches the M40 plan's expected owner:
sentence graph lookup/indexing remains the top optimization target after M39.

The short rows, startup/session, incomplete pinyin rows, selected storage,
bounded output/context, and Track B guard row are present in the baseline and
must be preserved by the final evidence.
