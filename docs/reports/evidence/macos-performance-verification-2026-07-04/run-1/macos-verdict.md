# macOS Native Performance Verification

This run uses the Rust `native_inprocess_benchmark` harness on macOS and reads context after every keypress.
Absolute timings and memory are platform-specific; the verdict checks the report's directional claim shape.

## Track A Latency

| Input/workload | Yune us | librime us | ratio | status |
| --- | --- | --- | --- | --- |
| ceshiyixiachangjushuruxingnengzenyang | 10471.865 | 171.657 | 61.005 | confirmed |
| cszysmsrsd | 4339.137 | 864.446 | 5.020 | contradicted |
| hao | 20.570 | 11.222 | 1.833 | confirmed |
| n | 25.583 | 24.500 | 1.044 | confirmed |
| ni | 17.291 | 19.750 | 0.875 | contradicted |
| zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong | 17867.849 | 406.569 | 43.948 | confirmed |
| zhongguo | 26.333 | 108.708 | 0.242 | confirmed |
| zybfshmsru | 4246.887 | 603.671 | 7.035 | contradicted |
| session_create_select_destroy | 18167.584 | 34878.875 | 0.521 | confirmed-on-this-run |
| startup_warm_shared_assets_runtime_ready | 18260.458 | 36526.583 | 0.500 | confirmed-on-this-run |

## Claim Shape

| Kind | Input | Expected | Status |
| --- | --- | --- | --- |
| latency_direction | ceshiyixiachangjushuruxingnengzenyang | yune slower than librime | confirmed |
| latency_direction | cszysmsrsd | yune faster than librime | contradicted |
| latency_direction | hao | yune slower than librime | confirmed |
| latency_direction | n | yune slower than librime | confirmed |
| latency_direction | ni | yune slower than librime | contradicted |
| latency_direction | zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong | yune slower than librime | confirmed |
| latency_direction | zhongguo | yune faster than librime | confirmed |
| latency_direction | zybfshmsru | yune faster than librime | contradicted |
| latency_noise_caveat | startup_warm_shared_assets_runtime_ready | report says measured faster on Windows but run-noisy/platform-specific | confirmed-on-this-run |
| latency_noise_caveat | session_create_select_destroy | report says measured faster on Windows but run-noisy/platform-specific | confirmed-on-this-run |
| candidate_snapshot | ceshiyixiachangjushuruxingnengzenyang | first page may differ as disclosed | confirmed |
| candidate_snapshot | cszysmsrsd | first page matches | contradicted |
| candidate_snapshot | hao | first page matches | confirmed |
| candidate_snapshot | n | first page may differ as disclosed | confirmed |
| candidate_snapshot | ni | first page matches | confirmed |
| candidate_snapshot | zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong | first page may differ as disclosed | confirmed |
| candidate_snapshot | zhongguo | first page may differ as disclosed | confirmed |
| candidate_snapshot | zybfshmsru | first page matches | contradicted |
| memory_direction | track-a-peak-resident | Yune Track A peak resident memory remains above librime on native peer lane | confirmed |
| browser | browser-peer-dashboard | browser rows are carried evidence | not-rerun |

## Notes

- macOS memory sampling uses resident size from `proc_pidinfo(PROC_PIDTASKINFO)` and peak resident size from `getrusage(RUSAGE_SELF).ru_maxrss`; Windows private/pagefile counters are not available on this platform.
- Browser rows were not re-run by this native verification lane.
