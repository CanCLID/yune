# macOS Native Performance Verification

This run uses the Rust `native_inprocess_benchmark` harness on macOS and reads context after every keypress.
Absolute timings and memory are platform-specific; the verdict checks the report's directional claim shape.

## Track A Latency

| Input/workload | Yune us | librime us | ratio | status |
| --- | --- | --- | --- | --- |
| b | 74.875 | 12.375 | 6.051 | not-classified |
| ceshi | 46.233 | 32.375 | 1.428 | not-classified |
| ceshiyixiachangjushuruxingnengzenyang | 66.940 | 167.917 | 0.399 | contradicted |
| che | 96.069 | 36.195 | 2.654 | not-classified |
| chuang | 60.646 | 24.597 | 2.466 | not-classified |
| cszysmsrsd | 497.475 | 794.092 | 0.626 | confirmed |
| dazisudu | 41.448 | 26.745 | 1.550 | not-classified |
| hao | 27.208 | 15.139 | 1.797 | confirmed |
| j | 76.791 | 10.291 | 7.462 | not-classified |
| n | 90.208 | 21.209 | 4.253 | confirmed |
| ni | 49.312 | 16.458 | 2.996 | confirmed |
| yi | 42.041 | 10.291 | 4.085 | not-classified |
| zh | 137.355 | 42.125 | 3.261 | not-classified |
| zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong | 79.654 | 392.268 | 0.203 | contradicted |
| zhongdengchangdu | 79.721 | 202.471 | 0.394 | not-classified |
| zhongguo | 108.807 | 110.667 | 0.983 | confirmed |
| zybfshmsru | 474.079 | 539.467 | 0.879 | confirmed |
| session_create_select_destroy | 17470.750 | 28009.375 | 0.624 | confirmed-on-this-run |
| startup_warm_shared_assets_runtime_ready | 17636.583 | 29594.000 | 0.596 | confirmed-on-this-run |

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
