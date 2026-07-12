# macOS Native Performance Verification

This run uses the Rust `native_inprocess_benchmark` harness on macOS and reads context after every keypress.
Absolute timings and memory are platform-specific; the verdict checks the report's directional claim shape.

## Track A Latency

| Input/workload | Yune us | librime us | ratio | status |
| --- | --- | --- | --- | --- |
| b | 73.667 | 12.042 | 6.118 | not-classified |
| ceshi | 46.000 | 31.533 | 1.459 | not-classified |
| ceshiyixiachangjushuruxingnengzenyang | 65.698 | 166.470 | 0.395 | contradicted |
| che | 93.875 | 36.222 | 2.592 | not-classified |
| chuang | 59.403 | 24.215 | 2.453 | not-classified |
| cszysmsrsd | 490.679 | 799.083 | 0.614 | confirmed |
| dazisudu | 41.453 | 27.297 | 1.519 | not-classified |
| hao | 27.708 | 14.570 | 1.902 | confirmed |
| j | 75.208 | 10.209 | 7.367 | not-classified |
| n | 90.333 | 24.250 | 3.725 | confirmed |
| ni | 50.042 | 19.354 | 2.586 | confirmed |
| yi | 41.395 | 10.438 | 3.966 | not-classified |
| zh | 136.521 | 42.021 | 3.249 | not-classified |
| zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong | 79.463 | 391.385 | 0.203 | contradicted |
| zhongdengchangdu | 80.201 | 201.294 | 0.398 | not-classified |
| zhongguo | 109.500 | 111.604 | 0.981 | confirmed |
| zybfshmsru | 471.071 | 537.600 | 0.876 | confirmed |
| session_create_select_destroy | 17588.375 | 28889.541 | 0.609 | confirmed-on-this-run |
| startup_warm_shared_assets_runtime_ready | 17805.500 | 30610.000 | 0.582 | confirmed-on-this-run |

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
