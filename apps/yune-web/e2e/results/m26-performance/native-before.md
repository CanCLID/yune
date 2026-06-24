# M26 Native Before Evidence

> **Status:** Captured before optimization - **Milestone:** M26 (engine/runtime performance hardening) - **Captured:** 2026-06-22 - **Type:** evidence

Command:

```powershell
cargo bench -p yune-rime-api --bench frontend_baselines
```

Result: passed. Cargo printed the known `yune_rime_api` output filename collision warnings and exited 0.

## Notes

- The full-ABI rows drive `RimeApi` session creation/schema selection/key processing plus status/context/commit/free cycles.
- The engine-only rows use public `yune-core` APIs inside the benchmark harness and do not widen `RimeApi`, `RimeCandidate`, or TypeDuck profile ABI slots.
- The engine-only rows fall back to source dictionary parsing when the checked-in compiled `.table.bin` / `.prism.bin` payload contains unsupported compiled sections. This is a benchmark-harness limitation and is recorded separately from the full-ABI production path.
- RSS/allocation data is not available from the dependency-free `std::time` harness; each real row records copied/source asset file count and byte size instead.

## Key Rows

| benchmark | schema | operations | median_us | p95_us | p99_us | max_us | cold_first_us | us_per_op | notes |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `per_key_real_jyut6ping3_mobile_hai_full_abi` | `jyut6ping3_mobile` | 48 | 15750.133 | 16697.867 | 16697.867 | 16697.867 | 27172.900 | 15532.004 | full ABI, status/context/commit/free |
| `per_key_real_jyut6ping3_mobile_hai_engine_only` | `jyut6ping3_mobile` | 48 | 8338.333 | 9453.133 | 9453.133 | 9453.133 | 17671.700 | 8277.075 | engine only, source fallback |
| `per_key_real_jyut6ping3_mobile_ngohaig_full_abi` | `jyut6ping3_mobile` | 112 | 12686.971 | 13165.400 | 13165.400 | 13165.400 | 22288.500 | 12456.135 | full ABI |
| `per_key_real_jyut6ping3_mobile_ngohaig_engine_only` | `jyut6ping3_mobile` | 112 | 9104.729 | 9440.043 | 9440.043 | 9440.043 | 13701.900 | 9013.846 | engine only, source fallback |
| `per_key_real_jyut6ping3_mobile_jigaajiusihaa_full_abi` | `jyut6ping3_mobile` | 208 | 21959.831 | 23061.838 | 23061.838 | 23061.838 | 36345.800 | 21818.316 | full ABI |
| `per_key_real_jyut6ping3_mobile_jigaajiusihaa_engine_only` | `jyut6ping3_mobile` | 208 | 14898.569 | 15949.100 | 15949.100 | 15949.100 | 22696.700 | 14927.892 | engine only, source fallback |
| `per_key_real_jyut6ping3_mobile_loengjathau_full_abi` | `jyut6ping3_mobile` | 176 | 12712.209 | 13299.200 | 13299.200 | 13299.200 | 33183.700 | 12681.394 | long TypeDuck sentence/composition fixture input |
| `per_key_real_jyut6ping3_mobile_loengjathau_engine_only` | `jyut6ping3_mobile` | 176 | 7940.627 | 8565.473 | 8565.473 | 8565.473 | 19860.300 | 7771.363 | engine only, source fallback |
| `per_key_real_jyut6ping3_mobile_jigaajiusihaa_correction_full_abi` | `jyut6ping3_mobile` | 208 | 21474.323 | 22806.554 | 22806.554 | 22806.554 | 37745.100 | 21698.609 | temp `common:/enable_correction` |
| `per_key_real_jyut6ping3_mobile_jigaajiusihaa_correction_engine_only` | `jyut6ping3_mobile` | 208 | 451490.692 | 467909.308 | 467909.308 | 467909.308 | 428053.900 | 453477.876 | `entries_by_code.keys()` dynamic-correction scan exercised |
| `per_key_real_luna_pinyin_ni_full_abi` | `luna_pinyin` | 32 | 1425.500 | 1714.300 | 1714.300 | 1714.300 | 2474.400 | 1450.762 | full ABI |
| `per_key_real_luna_pinyin_ni_engine_only` | `luna_pinyin` | 32 | 471.500 | 709.250 | 709.250 | 709.250 | 1091.400 | 487.494 | engine only, source fallback |
| `per_key_real_luna_pinyin_zhongguo_full_abi` | `luna_pinyin` | 128 | 11958.700 | 12244.275 | 12244.275 | 12244.275 | 6682.200 | 11890.655 | full ABI |
| `per_key_real_luna_pinyin_zhongguo_engine_only` | `luna_pinyin` | 128 | 475.062 | 548.800 | 548.800 | 548.800 | 3335.800 | 485.452 | engine only, source fallback |
| `per_key_real_cangjie5_a_full_abi` | `cangjie5` | 16 | 769.800 | 1199.200 | 1199.200 | 1199.200 | 1170.500 | 796.550 | representative breadth schema |
| `per_key_real_cangjie5_a_engine_only` | `cangjie5` | 16 | 195.500 | 201.300 | 201.300 | 201.300 | 274.000 | 195.888 | representative breadth schema, source fallback |
| `startup_real_jyut6ping3_mobile_runtime_ready` | `jyut6ping3_mobile` | 5 | 15614407.100 | 15738535.700 | 15738535.700 | 15738535.700 | - | 15472283.240 | startup/runtime-ready |
| `deploy_real_jyut6ping3_mobile_cache_hit` | `jyut6ping3_mobile` | 5 | 2285194.900 | 31061239.400 | 31061239.400 | 31061239.400 | - | 8040903.080 | deploy/cache-hit row; max shows an outlier |
| `startup_real_luna_pinyin_runtime_ready` | `luna_pinyin` | 5 | 789906.000 | 806107.500 | 806107.500 | 806107.500 | - | 791988.220 | startup/runtime-ready |

All real rows used `apps/yune-web/source/public/schema` with `asset_files=48` and `asset_bytes=35836386`.
