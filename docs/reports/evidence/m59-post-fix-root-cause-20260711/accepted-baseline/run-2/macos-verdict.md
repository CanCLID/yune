# macOS Native Performance Verification

This run uses the Rust `native_inprocess_benchmark` harness on macOS and reads context after every keypress.
Absolute timings and memory are platform-specific; the verdict checks the report's directional claim shape.

## Track A Latency

| Input/workload | Yune us | librime us | ratio | status |
| --- | --- | --- | --- | --- |
| b | 75.333 | 12.125 | 6.213 | not-classified |
| ceshi | 47.008 | 32.242 | 1.458 | not-classified |
| ceshiyixiachangjushuruxingnengzenyang | 66.524 | 168.081 | 0.396 | contradicted |
| che | 95.444 | 35.736 | 2.671 | not-classified |
| chuang | 61.702 | 24.201 | 2.550 | not-classified |
| cszysmsrsd | 509.779 | 797.500 | 0.639 | confirmed |
| dazisudu | 41.750 | 26.688 | 1.564 | not-classified |
| hao | 28.181 | 13.958 | 2.019 | confirmed |
| j | 77.125 | 10.291 | 7.494 | not-classified |
| n | 92.250 | 22.375 | 4.123 | confirmed |
| ni | 50.041 | 17.604 | 2.843 | confirmed |
| yi | 41.354 | 9.812 | 4.215 | not-classified |
| zh | 139.646 | 41.708 | 3.348 | not-classified |
| zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong | 80.233 | 391.407 | 0.205 | contradicted |
| zhongdengchangdu | 83.727 | 201.232 | 0.416 | not-classified |
| zhongguo | 111.219 | 112.151 | 0.992 | confirmed |
| zybfshmsru | 487.375 | 537.013 | 0.908 | confirmed |
| session_create_select_destroy | 17981.125 | 28892.708 | 0.622 | confirmed-on-this-run |
| startup_warm_shared_assets_runtime_ready | 18077.791 | 31367.792 | 0.576 | confirmed-on-this-run |

## Claim Shape

| Kind | Input | Expected | Status |
| --- | --- | --- | --- |
| latency_direction | b |  | not-classified |
| latency_direction | ceshi |  | not-classified |
| latency_direction | ceshiyixiachangjushuruxingnengzenyang | yune slower than librime | contradicted |
| latency_direction | che |  | not-classified |
| latency_direction | chuang |  | not-classified |
| latency_direction | cszysmsrsd | yune faster than librime | confirmed |
| latency_direction | dazisudu |  | not-classified |
| latency_direction | hao | yune slower than librime | confirmed |
| latency_direction | j |  | not-classified |
| latency_direction | n | yune slower than librime | confirmed |
| latency_direction | ni | yune slower than librime | confirmed |
| latency_direction | yi |  | not-classified |
| latency_direction | zh |  | not-classified |
| latency_direction | zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong | yune slower than librime | contradicted |
| latency_direction | zhongdengchangdu |  | not-classified |
| latency_direction | zhongguo | yune faster than librime | confirmed |
| latency_direction | zybfshmsru | yune faster than librime | confirmed |
| latency_noise_caveat | startup_warm_shared_assets_runtime_ready | report says measured faster on Windows but run-noisy/platform-specific | confirmed-on-this-run |
| latency_noise_caveat | session_create_select_destroy | report says measured faster on Windows but run-noisy/platform-specific | confirmed-on-this-run |
| candidate_snapshot | ceshiyixiachangjushuruxingnengzenyang | first page may differ as disclosed | allowed-but-unexpected-match |
| candidate_snapshot | cszysmsrsd | first page matches | confirmed |
| candidate_snapshot | hao | first page matches | confirmed |
| candidate_snapshot | n | first page may differ as disclosed | confirmed |
| candidate_snapshot | ni | first page matches | confirmed |
| candidate_snapshot | zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong | first page may differ as disclosed | allowed-but-unexpected-match |
| candidate_snapshot | zhongguo | first page may differ as disclosed | confirmed |
| candidate_snapshot | zybfshmsru | first page matches | confirmed |
| memory_direction | track-a-peak-resident | Yune Track A peak resident memory remains above librime on native peer lane | confirmed |
| browser | browser-peer-dashboard | browser rows are carried evidence | not-rerun |

## Notes

- macOS memory sampling uses resident size from `proc_pidinfo(PROC_PIDTASKINFO)` and peak resident size from `getrusage(RUSAGE_SELF).ru_maxrss`; Windows private/pagefile counters are not available on this platform.
- Browser rows were not re-run by this native verification lane.
