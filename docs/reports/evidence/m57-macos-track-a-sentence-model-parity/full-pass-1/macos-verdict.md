# macOS Native Performance Verification

This run uses the Rust `native_inprocess_benchmark` harness on macOS and reads context after every keypress.
Absolute timings and memory are platform-specific; the verdict checks the report's directional claim shape.

## Track A Latency

| Input/workload | Yune us | librime us | ratio | status |
| --- | --- | --- | --- | --- |
| ceshiyixiachangjushuruxingnengzenyang | 512.650 | 174.628 | 2.936 | confirmed |
| cszysmsrsd | 419.758 | 834.000 | 0.503 | confirmed |
| hao | 26.014 | 15.333 | 1.697 | confirmed |
| n | 64.500 | 27.041 | 2.385 | confirmed |
| ni | 46.083 | 18.271 | 2.522 | confirmed |
| zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong | 943.707 | 402.268 | 2.346 | confirmed |
| zhongguo | 42.932 | 110.552 | 0.388 | confirmed |
| zybfshmsru | 461.008 | 576.663 | 0.799 | confirmed |
| session_create_select_destroy | 18817.875 | 30065.708 | 0.626 | confirmed-on-this-run |
| startup_warm_shared_assets_runtime_ready | 18110.750 | 31462.959 | 0.576 | confirmed-on-this-run |

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
