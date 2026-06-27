# Post-M44 Native Bottleneck Diagnostic Summary

This is a diagnostic benchmark, not a milestone closeout. It reruns Track A with the intermediate `n` row added so the short-key path can be separated from the final `ni` result, and it records working-set bands to distinguish high-water peak from steady after-ready resident size.

## Track A Key-Row Latency Comparison

| Row | Yune median us | librime median us | Ratio | Yune p95 us | librime p95 us | Read |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| `n` | 79.400 | 21.900 | 3.626x | 163.400 | 24.100 | new worst short-prefix row; broad prefix enumeration is visible before `ni` |
| `ni` | 53.750 | 15.050 | 3.571x | 110.150 | 16.900 | still misses the short-key parity target and inherits the `n` prefix step |
| `hao` | 25.667 | 12.100 | 2.121x | 35.533 | 12.700 | now below the M44 target but still above librime by about 2x |
| `zhongguo` | 68.763 | 181.787 | 0.378x | 82.100 | 218.588 | short normal row remains faster than librime |
| `ceshiyixiachangjushuruxingnengzenyang` | 319.876 | 321.114 | 0.996x | 348.897 | 343.189 | M40 long row remains near parity |
| `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` | 611.125 | 740.808 | 0.825x | 903.215 | 876.234 | M40 59-char row remains faster than librime |
| `cszysmsrsd` | 592.040 | 1264.710 | 0.468x | 696.270 | 1372.760 | M42 abbreviation parity row remains faster than librime |
| `zybfshmsru` | 599.970 | 892.530 | 0.672x | 669.690 | 992.990 | M42 abbreviation parity row remains faster than librime |

## Startup And Session Context

| Workload | Yune median us | librime median us | Ratio | Yune max peak B | librime max peak B |
| --- | ---: | ---: | ---: | ---: | ---: |
| `startup_warm_shared_assets_runtime_ready` | 28322.300 | 27966.100 | 1.013x | 127430656 | 13897728 |
| `session_create_select_destroy` | 27338.400 | 29065.200 | 0.941x | 127430656 | 14053376 |

## Short-Key Raw Lookup Evidence

| Input | Prism code count | Lookup code count | Raw table candidates | Raw table median us | Translator median us | Context export us | Owned candidates/op | Read |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `n` | 26 | 27 | 1260 | 166.000 | 74.100 | 1.500 | 7.000 | literal `n` plus 26 prism completions produce 1260 raw table candidates before page export |
| `ni` | 1 | 1 | 182 | 19.600 | 50.200 | 1.500 | 7.000 | only one table code, so the remaining cost is not raw candidate count alone |
| `hao` | 1 | 1 | 139 | 15.500 | 22.533 | 1.500 | 7.000 | bounded row is now relatively cheap but still above librime constant factors |

## Track A Yune Memory Sample Bands

| Workload/input | Samples | Min after-ready B | Median after-ready B | Max after-ready B | Max peak B |
| --- | ---: | ---: | ---: | ---: | ---: |
| `startup_warm_shared_assets_runtime_ready` | 5 | 89669632 | 89968640 | 90742784 | 127430656 |
| `session_create_select_destroy` | 30 | 86880256 | 87240704 | 87486464 | 127430656 |
| `key_sequence_process_with_context`, `n` | 80 | 90517504 | 90714112 | 91095040 | 127430656 |
| `key_sequence_process_with_context`, `ni` | 80 | 91381760 | 91439104 | 91725824 | 127430656 |
| `key_sequence_process_with_context`, `hao` | 80 | 92241920 | 92319744 | 92778496 | 127430656 |
| `key_sequence_process_with_context`, `zhongguo` | 80 | 93532160 | 93532160 | 93761536 | 127430656 |
| `key_sequence_process_with_context`, `ceshiyixiachangjushuruxingnengzenyang` | 80 | 93941760 | 94195712 | 94396416 | 127430656 |
| `key_sequence_process_with_context`, `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` | 80 | 94879744 | 95064064 | 95862784 | 127430656 |
| `key_sequence_process_with_context`, `cszysmsrsd` | 80 | 96571392 | 96706560 | 96792576 | 127430656 |
| `key_sequence_process_with_context`, `zybfshmsru` | 80 | 97673216 | 97677312 | 97701888 | 127430656 |

## Track A Memory Owner Profile

| Owner | Byte class | Retained estimate B | Non-overlapping reducible B | Mapped file B | Notes |
| --- | --- | ---: | ---: | ---: | --- |
| `poet.entries_by_code` | `heap_owned_reducible` | 18694662 | 18694662 | 0 | sentence model entries cloned from table rows |
| `compact_table.storage` | `mmap_file_backed` | 13013460 | 0 | 13013460 | rsmarisa table bytes are excluded from heap-owned branch triggers when mapped |
| `poet.lookup_index` | `heap_owned_guarded` | 2660848 | 0 | 0 | sorted code-range index used by M40 sentence lookup |
| `compact_table.syllable_ids_by_code` | `heap_owned_reducible` | 15757 | 15757 | 0 | rsmarisa lookup side map retained on heap |
| `compact_table.syllabary_codes` | `heap_owned_reducible` | 13685 | 13685 | 0 | canonical code list retained for prism lookup |
| `runtime.session_state` | `heap_owned_guarded` | 3166 | 0 | 0 | active native session runtime shell excluding shared translator internals |
| `schema.processors` | `heap_owned_guarded` | 2599 | 0 | 0 | processor and segmentor state installed from the selected schema |
| `schema.config` | `overlap_estimate` | 1954 | 0 | 0 | deployed YAML config is parsed transiently; retained bytes are reload signatures |
| `poet.abbreviation_vocabulary` | `heap_owned_reducible` | 1433 | 1433 | 0 | abbreviation-only vocabulary used by M42 guard rows |
| `session.userdb` | `heap_owned_guarded` | 24 | 0 | 0 | per-session learned entries; empty in clean native benchmark runs |
| `translator.entries_by_code` | `shared` | 0 | 0 | 0 | compact storage path does not retain a translator BTreeMap |

## Diagnostic Read

- The short-key latency owner is sharper than the M44 closeout table showed: `n` is slower than `ni` in absolute time and touches a broad prefix span, so the next short-key fix should target bounded borrowed prefix enumeration and first-page early stop before optimizing generic candidate export.
- `ni` is not explained by raw table candidate count alone: the raw lookup is one code and 182 candidates, but translator latency remains high because the sequence includes the earlier `n` step and downstream short-key work.
- The memory peak behaves like a process high-water/transient measurement: every Yune Track A row reports the same `127,430,656 B` max peak, while steady after-ready medians sit around `87-98 MB`.
- The retained-owner profile still names only `18,694,662 B` of reducible `poet.entries_by_code` plus `13,013,460 B` of file-backed table bytes. That does not explain the whole-process gap, so the next memory milestone should begin with allocator/private/RSS/mapped-page attribution before authorizing another storage rewrite.
