# Post-M38 Long Input Baseline

Date: 2026-06-25

Command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\post-m38-long-input-baseline\baseline-native -Iterations 9 -SessionIterations 20 -KeyIterations 50 -TrackAInputs "ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang" -DeployProductBeforeBenchmark
```

This is a same-machine native in-process Yune versus upstream librime `1.17.0`
baseline after M38. It keeps the M38 Track A rows and adds the long continuous
pinyin row `ceshiyixiachangjushuruxingnengzenyang`.

## Track A Results

| Row | Yune median | librime median | Ratio |
| --- | ---: | ---: | ---: |
| startup/runtime-ready | `23,478.800 us` | `32,805.100 us` | `0.716x` |
| session create/select/destroy | `24,202.100 us` | `32,302.200 us` | `0.749x` |
| `hao` | `38.967 us` | `11.733 us` | `3.321x` |
| `ni` | `56.200 us` | `14.600 us` | `3.849x` |
| `zhongguo` | `62.025 us` | `172.950 us` | `0.359x` |
| `ceshiyixiachangjushuruxingnengzenyang` | `412,192.727 us` | `294.151 us` | `1,401.296x` |

The original M38 target rows remain in-family with the final M38 evidence. The
new long row exposes a separate large gap.

## Track A Memory Results

The same benchmark captured median working set and max peak working set for each
row:

| Row | Yune median working set | librime median working set | Ratio | Yune max peak | librime max peak | Ratio |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| startup/runtime-ready | `111,583,232 B` | `11,558,912 B` | `9.65x` | `163,057,664 B` | `14,045,184 B` | `11.61x` |
| session create/select/destroy | `107,839,488 B` | `11,091,968 B` | `9.72x` | `163,057,664 B` | `14,135,296 B` | `11.54x` |
| `hao` | `111,878,144 B` | `12,308,480 B` | `9.09x` | `163,057,664 B` | `14,229,504 B` | `11.46x` |
| `ni` | `111,644,672 B` | `12,189,696 B` | `9.16x` | `163,057,664 B` | `14,135,296 B` | `11.54x` |
| `zhongguo` | `112,160,768 B` | `13,332,480 B` | `8.41x` | `163,057,664 B` | `14,299,136 B` | `11.40x` |
| `ceshiyixiachangjushuruxingnengzenyang` | `114,610,176 B` | `15,638,528 B` | `7.33x` | `163,057,664 B` | `15,659,008 B` | `10.41x` |

This records a real memory baseline for M39 planning. It does not identify heap
owners. The memory result should be read as a working-set/peak gap that still
needs heap-owner profiling before a memory-specific optimization is selected.

## Long Row Owner Counters

Yune long-row median owner counters per operation:

| Counter | Value |
| --- | ---: |
| process key | `412,107.941 us` |
| translator | `412,078.328 us` |
| lookup views visited | `4.973` |
| owned candidates materialized | `3.108` |
| candidates sorted/stored | `76.054` |
| candidate sort | `0.791 us` |
| filter pipeline | `9.415 us` |
| context page clones | `0.135` |
| ABI get context | `0.149 us` |
| candidate request bounded calls | `1.000` |
| candidate request unbounded calls | `0.000` |
| bounded iterator selected | `3.081` |
| bounded iterator full count | `3.108` |
| full-list fallback count | `0.730` |
| exact lookup calls | `1.730` |
| exact lookup time | `27.254 us` |
| prefix lookup calls | `1.703` |
| prefix lookup time | `33.077 us` |
| rsmarisa exact lookup calls | `1.730` |
| rsmarisa prefix lookup calls | `1.703` |
| ABI C-string allocations | `0.189` |
| ABI C-string bytes | `7.027` |

Raw lookup microbench for the long row records:

- selected storage: `rsmarisa_byte_backed`
- table mapping: `mmap`
- prism mapping: `mmap`
- raw prism median: `0.000 us`
- raw table median: `33.400 us`
- raw table candidates: `0`
- translator median: `412,162.427 us`
- context export median: `5.500 us`

## Interpretation

This is not a marisa activation failure. The row uses the M38 Track A
`rsmarisa_byte_backed` path, table/prism bytes are mmap-backed, and rsmarisa
exact/prefix counters are positive.

The measured owner is the translator. The existing M37/M38 counters do not yet
split the expensive work inside the long-composition translator path. Code
inspection points to sentence/full-list fallback work, especially
`StaticTableTranslator::sentence_candidate`, because that path performs dynamic
substring segmentation and lookup work that is not currently timed by the raw
exact/prefix lookup counters.

The next optimization plan should start by instrumenting this long-row
translator path before choosing an implementation strategy.
