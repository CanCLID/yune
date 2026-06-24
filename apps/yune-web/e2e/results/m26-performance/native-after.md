# M26 Native Benchmark Evidence - After Optimization

> **Status:** Complete - **Milestone:** M26 (performance hardening) - **Updated:** 2026-06-22 - **Type:** evidence

Command:

```powershell
cargo bench -p yune-rime-api --bench frontend_baselines
```

Result: passed. Cargo printed the known `yune_rime_api` output filename collision warnings and exited 0.

## Optimization Target Result

The selected M26 slice prunes TypeDuck dynamic-correction candidates by the exact edit-distance length lower bound before the expensive restricted-distance matrix runs.

| Row | Before median_us | After median_us | Before p95_us | After p95_us | Before cold_first_us | After cold_first_us |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `per_key_real_jyut6ping3_mobile_jigaajiusihaa_correction_engine_only` | 451490.692 | 121712.662 | 467909.308 | 124420.115 | 428053.900 | 71611.200 |
| `per_key_real_jyut6ping3_mobile_jigaajiusihaa_correction_full_abi` | 21474.323 | 21307.215 | 22806.554 | 21603.546 | 37745.100 | 37371.500 |

The engine-only correction row improved by about `3.7x` on median and about `3.8x` on p95. The full-ABI correction-config row remains around ordinary long TypeDuck input cost because it measures the full RIME ABI lifecycle and does not isolate the synthetic dynamic-correction stress path.

## After Rows

| benchmark | operations | schema_id | us_per_op | median_us | p95_us | p99_us | max_us | cold_first_us | notes |
| --- | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `session_create_destroy` | 200 | baseline | 20.086 | - | - | - | - | - | synthetic |
| `per_key_simple_ascii_rime_process_key` | 600 | baseline | 44.953 | - | - | - | - | - | synthetic |
| `per_key_schema_loaded_lookup_rime_process_key` | 400 | lookup | 440.604 | - | - | - | - | - | synthetic 4-entry dictionary |
| `schema_deploy_dictionary_load` | 20 | lookup | 3614.800 | - | - | - | - | - | synthetic 4-entry dictionary |
| `userdb_learning_sync` | 80 | learn | 7464.462 | - | - | - | - | - | synthetic |
| `per_key_real_jyut6ping3_mobile_hai_full_abi` | 48 | `jyut6ping3_mobile` | 16461.217 | 16596.867 | 17528.967 | 17528.967 | 17528.967 | 26981.500 | real assets |
| `per_key_real_jyut6ping3_mobile_hai_engine_only` | 48 | `jyut6ping3_mobile` | 7440.665 | 7442.667 | 8303.700 | 8303.700 | 8303.700 | 17333.900 | real assets |
| `per_key_real_jyut6ping3_mobile_ngohaig_full_abi` | 112 | `jyut6ping3_mobile` | 12575.720 | 12548.557 | 13113.543 | 13113.543 | 13113.543 | 22678.200 | real assets |
| `per_key_real_jyut6ping3_mobile_ngohaig_engine_only` | 112 | `jyut6ping3_mobile` | 8961.115 | 8941.286 | 10062.671 | 10062.671 | 10062.671 | 15218.100 | real assets |
| `per_key_real_jyut6ping3_mobile_jigaajiusihaa_full_abi` | 208 | `jyut6ping3_mobile` | 21319.714 | 20917.846 | 22866.169 | 22866.169 | 22866.169 | 37378.000 | real assets |
| `per_key_real_jyut6ping3_mobile_jigaajiusihaa_engine_only` | 208 | `jyut6ping3_mobile` | 14522.266 | 14435.454 | 15662.600 | 15662.600 | 15662.600 | 23002.900 | real assets |
| `per_key_real_jyut6ping3_mobile_loengjathau_full_abi` | 176 | `jyut6ping3_mobile` | 12128.595 | 12186.218 | 12291.336 | 12291.336 | 12291.336 | 34753.400 | real assets |
| `per_key_real_jyut6ping3_mobile_loengjathau_engine_only` | 176 | `jyut6ping3_mobile` | 7197.880 | 7178.518 | 7599.191 | 7599.191 | 7599.191 | 20102.600 | real assets |
| `per_key_real_jyut6ping3_mobile_jigaajiusihaa_correction_full_abi` | 208 | `jyut6ping3_mobile` | 21181.588 | 21307.215 | 21603.546 | 21603.546 | 21603.546 | 37371.500 | correction config enabled; full ABI lifecycle |
| `per_key_real_jyut6ping3_mobile_jigaajiusihaa_correction_engine_only` | 208 | `jyut6ping3_mobile` | 121888.287 | 121712.662 | 124420.115 | 124420.115 | 124420.115 | 71611.200 | dynamic-correction scan stress path |
| `per_key_real_luna_pinyin_ni_full_abi` | 32 | `luna_pinyin` | 1454.231 | 1431.100 | 1698.150 | 1698.150 | 1698.150 | 2255.200 | real assets |
| `per_key_real_luna_pinyin_ni_engine_only` | 32 | `luna_pinyin` | 469.650 | 466.000 | 499.550 | 499.550 | 499.550 | 1023.500 | real assets |
| `per_key_real_luna_pinyin_zhongguo_full_abi` | 128 | `luna_pinyin` | 11883.182 | 11923.300 | 12157.425 | 12157.425 | 12157.425 | 6672.100 | real assets |
| `per_key_real_luna_pinyin_zhongguo_engine_only` | 128 | `luna_pinyin` | 476.266 | 474.537 | 503.100 | 503.100 | 503.100 | 3615.000 | real assets |
| `per_key_real_cangjie5_a_full_abi` | 16 | `cangjie5` | 795.619 | 772.100 | 1196.100 | 1196.100 | 1196.100 | 1189.000 | representative breadth schema |
| `per_key_real_cangjie5_a_engine_only` | 16 | `cangjie5` | 200.319 | 200.500 | 204.300 | 204.300 | 204.300 | 280.300 | representative breadth schema |
| `startup_real_jyut6ping3_mobile_runtime_ready` | 5 | `jyut6ping3_mobile` | 15181207.060 | 15103350.100 | 15636872.300 | 15636872.300 | 15636872.300 | - | real assets |
| `deploy_real_jyut6ping3_mobile_cache_hit` | 5 | `jyut6ping3_mobile` | 8222209.140 | 2282541.900 | 32053300.600 | 32053300.600 | 32053300.600 | - | cache-hit row with one outlier |
| `startup_real_luna_pinyin_runtime_ready` | 5 | `luna_pinyin` | 792949.860 | 789428.900 | 802446.100 | 802446.100 | 802446.100 | - | real assets |

Memory/allocation note: the benchmark harness still uses `std::time` only and cannot report RSS locally. The optimization does not add a persistent index or new candidate storage; it reduces per-lookup allocation by avoiding calls that would allocate the restricted-distance matrix for impossible-length dynamic-correction candidates.
