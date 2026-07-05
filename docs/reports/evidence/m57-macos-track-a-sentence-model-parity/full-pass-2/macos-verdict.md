# macOS Native Performance Verification

This run uses the Rust `native_inprocess_benchmark` harness on macOS and reads context after every keypress.
Absolute timings and memory are platform-specific; the verdict checks the report's directional claim shape.

## Track A Latency

| Input/workload | Yune us | librime us | ratio | status |
| --- | --- | --- | --- | --- |
| ceshiyixiachangjushuruxingnengzenyang | 504.605 | 187.211 | 2.695 | confirmed |
| cszysmsrsd | 397.333 | 871.404 | 0.456 | confirmed |
| hao | 25.097 | 21.778 | 1.152 | confirmed |
| n | 62.458 | 14.459 | 4.320 | confirmed |
| ni | 44.625 | 10.875 | 4.103 | confirmed |
| zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong | 902.807 | 429.886 | 2.100 | confirmed |
| zhongguo | 42.682 | 263.958 | 0.162 | confirmed |
| zybfshmsru | 405.692 | 584.146 | 0.695 | confirmed |
| session_create_select_destroy | 17825.333 | 28991.208 | 0.615 | confirmed-on-this-run |
| startup_warm_shared_assets_runtime_ready | 17884.250 | 29398.958 | 0.608 | confirmed-on-this-run |

## Claim Shape

| Kind | Input | Expected | Status |
| --- | --- | --- | --- |
| latency_direction | ceshiyixiachangjushuruxingnengzenyang | yune slower than librime | confirmed |
| latency_direction | cszysmsrsd | yune faster than librime | confirmed |
| latency_direction | hao | yune slower than librime | confirmed |
| latency_direction | n | yune slower than librime | confirmed |
| latency_direction | ni | yune slower than librime | confirmed |
| latency_direction | zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong | yune slower than librime | confirmed |
| latency_direction | zhongguo | yune faster than librime | confirmed |
| latency_direction | zybfshmsru | yune faster than librime | confirmed |
| latency_noise_caveat | startup_warm_shared_assets_runtime_ready | report says measured faster on Windows but run-noisy/platform-specific | confirmed-on-this-run |
| latency_noise_caveat | session_create_select_destroy | report says measured faster on Windows but run-noisy/platform-specific | confirmed-on-this-run |
| candidate_snapshot | ceshiyixiachangjushuruxingnengzenyang | first page may differ as disclosed | confirmed |
| candidate_snapshot | cszysmsrsd | first page matches | confirmed |
| candidate_snapshot | hao | first page matches | confirmed |
| candidate_snapshot | n | first page may differ as disclosed | confirmed |
| candidate_snapshot | ni | first page matches | confirmed |
| candidate_snapshot | zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong | first page may differ as disclosed | confirmed |
| candidate_snapshot | zhongguo | first page may differ as disclosed | confirmed |
| candidate_snapshot | zybfshmsru | first page matches | confirmed |
| memory_direction | track-a-peak-resident | Yune Track A peak resident memory remains above librime on native peer lane | confirmed |
| browser | browser-peer-dashboard | browser rows are carried evidence | not-rerun |

## Notes

- macOS memory sampling uses resident size from `proc_pidinfo(PROC_PIDTASKINFO)` and peak resident size from `getrusage(RUSAGE_SELF).ru_maxrss`; Windows private/pagefile counters are not available on this platform.
- Browser rows were not re-run by this native verification lane.
