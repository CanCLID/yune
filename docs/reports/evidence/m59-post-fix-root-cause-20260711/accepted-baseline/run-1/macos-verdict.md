# macOS Native Performance Verification

This run uses the Rust `native_inprocess_benchmark` harness on macOS and reads context after every keypress.
Absolute timings and memory are platform-specific; the verdict checks the report's directional claim shape.

## Track A Latency

| Input/workload | Yune us | librime us | ratio | status |
| --- | --- | --- | --- | --- |
| b | 76.333 | 12.000 | 6.361 | not-classified |
| ceshi | 46.092 | 33.175 | 1.389 | not-classified |
| ceshiyixiachangjushuruxingnengzenyang | 67.914 | 163.753 | 0.415 | contradicted |
| che | 96.681 | 34.000 | 2.844 | not-classified |
| chuang | 61.604 | 24.257 | 2.540 | not-classified |
| cszysmsrsd | 508.042 | 785.596 | 0.647 | confirmed |
| dazisudu | 41.755 | 26.656 | 1.566 | not-classified |
| hao | 27.986 | 12.375 | 2.261 | confirmed |
| j | 76.750 | 10.208 | 7.519 | not-classified |
| n | 91.291 | 23.750 | 3.844 | confirmed |
| ni | 49.645 | 16.625 | 2.986 | confirmed |
| yi | 42.042 | 10.250 | 4.102 | not-classified |
| zh | 139.667 | 42.188 | 3.311 | not-classified |
| zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong | 81.965 | 381.258 | 0.215 | contradicted |
| zhongdengchangdu | 81.117 | 197.875 | 0.410 | not-classified |
| zhongguo | 109.729 | 108.339 | 1.013 | contradicted |
| zybfshmsru | 491.087 | 532.571 | 0.922 | confirmed |
| session_create_select_destroy | 18000.250 | 29011.041 | 0.620 | confirmed-on-this-run |
| startup_warm_shared_assets_runtime_ready | 18039.667 | 36342.125 | 0.496 | confirmed-on-this-run |

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
