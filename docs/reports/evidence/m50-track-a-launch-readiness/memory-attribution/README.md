# M50 Track A Memory Attribution

Scope: native Track A `luna_pinyin` only. This run is not evidence for browser,
frontend, product package, deployment, public demo, TypeDuck keyboard-profile
memory, iOS-device `phys_footprint`, or Apple platform validation.

## Command

`powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\m50-track-a-launch-readiness\memory-attribution -Iterations 9 -SessionIterations 60 -KeyIterations 80 -TrackAInputs n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru -TrackBInputs neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung -DeployProductBeforeBenchmark`

## Memory Classes

| Class | Bytes | Evidence | Status | Notes |
| --- | ---: | --- | --- | --- |
| Track A peak working set | `188,436,480` | `summary.csv` | measured blocker | Yune high-water working set for `luna_pinyin`; same-run librime peak is `17,653,760 B`. |
| Track A private proxy | `197,017,600` | `summary.csv` | measured blocker | Maximum per-input median private proxy in the Track A Yune summary rows. |
| Process private owner row | `192,663,552` | `track-a-yune/memory-owner-profile.csv` | measured blocker | `process.private_bytes_proxy`; process counter proxy, not iOS dirty memory. |
| Named non-overlapping reducible heap owners | `72,370,289` | `track-a-yune/memory-owner-profile.csv` | reducible | Sum of non-overlapping reducible owner rows. |
| Clean/file-backed mapped payloads | `13,044,872` | `track-a-yune/memory-owner-profile.csv` | explained | `compact_table.storage` plus small prism mmap rows, all mapped as `byte_backed:mmap`. |
| Derived peak working-set gap | `103,021,319` | derived from `summary.csv` and `memory-owner-profile.csv` | measured blocker | `188,436,480 - 72,370,289 - 13,044,872`; lower-bound residual includes allocator, loader, mappings, overlap, and unclassified heap. |
| Owner-profile unclassified lower bound | `106,121,103` | `track-a-yune/memory-owner-profile.csv` | measured blocker | `process.after_ready_working_set_unclassified_lower_bound`; benchmark-provided lower-bound proxy. |

## Named Owners

| Owner | Class | Retained estimate | Non-overlapping reducible | Items | Notes |
| --- | --- | ---: | ---: | ---: | --- |
| `compact_table.storage` | `mmap_file_backed` | `13,013,460 B` | `0 B` | `498,564` | rsmarisa table bytes mapped from compact storage. |
| `prism.double_array_units` + `prism.spelling_map` | `mmap_file_backed` | `31,412 B` | `0 B` | `1,166` | small prism payload rows mapped from compact storage. |
| `poet.entries_by_code` | `heap_owned_reducible` | `18,694,662 B` | `18,694,662 B` | `513,353` | sentence model entries cloned from table rows. |
| `poet.lookup_index` | `heap_owned_guarded` | `2,660,848 B` | `0 B` | `332,604` | M40 sorted code-range index. |
| `poet.vocabulary` | `heap_owned_reducible` | `53,644,752 B` | `53,644,752 B` | `421,966` | normal preset vocabulary used by upstream sentence graph. |
| `poet.abbreviation_vocabulary` | `heap_owned_reducible` | `1,433 B` | `1,433 B` | `11` | abbreviation-only vocabulary used by M42 guard rows. |

## Verdict

Memory is attributed but not reduced in M50. The named in-scope owner is
`poet.vocabulary` at `53,644,752 B`; the remaining process-level
unclassified/private gap is the carried measured blocker. No retained heap
prefix or vocabulary index was added.

## Focused Check

- `cargo test -p yune-core memory_owner`
