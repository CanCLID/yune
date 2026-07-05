# macOS Native Performance Verification

This run uses the Rust `native_inprocess_benchmark` harness on macOS and reads context after every keypress.
Absolute timings and memory are platform-specific; the verdict checks the report's directional claim shape.

## Track A Latency

| Input/workload | Yune us | librime us | ratio | status |
| --- | --- | --- | --- | --- |
| ceshiyixiachangjushuruxingnengzenyang | 10400.765 | 176.904 | 58.793 | confirmed |
| cszysmsrsd | 4210.954 | 846.817 | 4.973 | contradicted |
| hao | 19.972 | 15.958 | 1.252 | confirmed |
| n | 26.083 | 24.542 | 1.063 | confirmed |
| ni | 17.166 | 17.709 | 0.969 | contradicted |
| zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong | 16967.360 | 416.960 | 40.693 | confirmed |
| zhongguo | 26.734 | 116.578 | 0.229 | confirmed |
| zybfshmsru | 4195.129 | 578.462 | 7.252 | contradicted |
| session_create_select_destroy | 18309.625 | 29722.584 | 0.616 | confirmed-on-this-run |
| startup_warm_shared_assets_runtime_ready | 18071.833 | 30759.166 | 0.588 | confirmed-on-this-run |

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
