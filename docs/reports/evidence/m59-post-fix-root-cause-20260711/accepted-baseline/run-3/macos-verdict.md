# macOS Native Performance Verification

This run uses the Rust `native_inprocess_benchmark` harness on macOS and reads context after every keypress.
Absolute timings and memory are platform-specific; the verdict checks the report's directional claim shape.

## Track A Latency

| Input/workload | Yune us | librime us | ratio | status |
| --- | --- | --- | --- | --- |
| b | 82.916 | 11.750 | 7.057 | not-classified |
| ceshi | 46.342 | 31.908 | 1.452 | not-classified |
| ceshiyixiachangjushuruxingnengzenyang | 68.148 | 165.945 | 0.411 | contradicted |
| che | 95.389 | 31.778 | 3.002 | not-classified |
| chuang | 60.951 | 23.764 | 2.565 | not-classified |
| cszysmsrsd | 500.946 | 802.000 | 0.625 | confirmed |
| dazisudu | 41.854 | 28.484 | 1.469 | not-classified |
| hao | 27.722 | 10.028 | 2.764 | confirmed |
| j | 76.125 | 11.167 | 6.817 | not-classified |
| n | 89.833 | 20.292 | 4.427 | confirmed |
| ni | 50.375 | 16.916 | 2.978 | confirmed |
| yi | 41.688 | 9.146 | 4.558 | not-classified |
| zh | 140.146 | 43.666 | 3.209 | not-classified |
| zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong | 80.856 | 391.282 | 0.207 | contradicted |
| zhongdengchangdu | 81.367 | 208.841 | 0.390 | not-classified |
| zhongguo | 111.542 | 103.740 | 1.075 | contradicted |
| zybfshmsru | 477.312 | 544.992 | 0.876 | confirmed |
| session_create_select_destroy | 18116.042 | 27841.417 | 0.651 | confirmed-on-this-run |
| startup_warm_shared_assets_runtime_ready | 18103.167 | 31691.000 | 0.571 | confirmed-on-this-run |

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
| latency_direction | zhongguo | yune faster than librime | contradicted |
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
