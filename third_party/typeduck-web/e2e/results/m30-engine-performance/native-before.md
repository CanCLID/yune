# M30 Native Baseline Before Lever A

Date: 2026-06-22

Command:

```powershell
cmd /c "cargo bench -p yune-rime-api --bench frontend_baselines > target\m30-frontend-baselines-before.txt 2>&1"
```

Captured output: `target/m30-frontend-baselines-before.txt`.

Result: passed. Cargo printed the known `yune_rime_api` output filename collision warnings and exited 0.

## Startup And Memory Rows

| Row | Median | P95 | Memory notes |
| --- | ---: | ---: | --- |
| `m29_single_startup_memory_jyut6ping3_mobile` | `6,406,590.100us` | `6,406,590.100us` | before `4,730,880` bytes; after-ready `1,103,331,328` bytes; after-finalize `12,951,552` bytes; peak `1,123,745,792` bytes; ready delta `1,098,600,448` bytes |
| `startup_real_jyut6ping3_mobile_runtime_ready` | `6,242,614.900us` | `6,288,639.300us` | std harness RSS unavailable |
| `startup_trace_jyut6ping3_mobile_select_schema_total` | `5,472,559us` | `5,577,741us` | working-set delta `956,739,584` bytes; peak `1,792,507,904` bytes |
| `startup_trace_jyut6ping3_mobile_translator_install` | `5,340,382us` | `5,449,418us` | working-set delta `988,053,504` bytes; peak `1,792,507,904` bytes |
| `startup_trace_jyut6ping3_mobile_spelling_algebra_expand` | `5,015,352us` | `5,127,754us` | working-set delta `923,234,304` bytes; peak `1,792,507,904` bytes |
| `startup_trace_jyut6ping3_mobile_source_dictionary_parse_if_any` | `152,665us` | `158,692us` | working-set delta `954,023,936` bytes; peak `1,792,507,904` bytes |
| `startup_trace_jyut6ping3_mobile_translator_index_build` | `144,074us` | `150,794us` | working-set delta `921,100,288` bytes; peak `1,792,507,904` bytes |
| `startup_trace_jyut6ping3_mobile_compiled_table_load` | `4,938us` | `5,087us` | working-set delta `4,308,992` bytes |
| `startup_trace_jyut6ping3_mobile_compiled_prism_load` | `3,689us` | `4,326us` | working-set delta `24,576` bytes |

## Watched Per-Key Rows

| Row | Median | P95 | Cold first | Notes |
| --- | ---: | ---: | ---: | --- |
| `per_key_real_jyut6ping3_mobile_hai_full_abi` | `15,849.267us` | `17,093.200us` | `28,382.100us` | profile-default correction |
| `per_key_real_jyut6ping3_mobile_hai_engine_only` | `8,121.467us` | `8,628.800us` | `16,760.700us` | profile-default correction |
| `per_key_real_jyut6ping3_mobile_jigaajiusihaa_full_abi` | `22,119.315us` | `23,615.408us` | `38,916.200us` | profile-default correction |
| `per_key_real_jyut6ping3_mobile_jigaajiusihaa_engine_only` | `15,326.546us` | `15,794.408us` | `23,392.400us` | profile-default correction |
| `per_key_real_jyut6ping3_mobile_jigaajiusihaa_correction_full_abi` | `22,380.546us` | `23,042.715us` | `38,394.000us` | correction enabled |
| `per_key_real_jyut6ping3_mobile_jigaajiusihaa_correction_engine_only` | `124,738.846us` | `125,883.923us` | `72,291.400us` | `entries_by_code.keys()` branch exercised |

## Baseline Reading

The M30 before-run keeps the M29 pattern: runtime-ready startup is about `6.24s`, spelling algebra remains about `5.02s`, and single-startup ready pressure is about `1.10GB`. Lever A should mainly affect retained/peak memory and translator storage duplication. Per-key rows are captured here to make sure representation changes do not silently regress `hai`, ordinary `jigaajiusihaa`, or the correction stress path.
