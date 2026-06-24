# M27 Native Startup Before

> **Status:** Captured before M27 optimization - **Milestone:** M27 (TypeDuck-Web startup/runtime init) - **Updated:** 2026-06-22 - **Type:** evidence

Command:

```powershell
cargo bench -p yune-rime-api --bench frontend_baselines
```

Captured output: `target/m27-frontend-baselines-before.txt`.

## Browser-Matching Startup Row

| Row | Median | P95 | Notes |
|---|---:|---:|---|
| `startup_real_jyut6ping3_mobile_runtime_ready` | `15,546,100.900us` | `16,103,129.100us` | Browser-matching native path after schema assets are present. |
| `startup_real_luna_pinyin_runtime_ready` | `834,006.700us` | `865,127.400us` | Upstream/common-schema control. |

## Split Owners

| Row | Median | P95 | Memory Notes |
|---|---:|---:|---|
| `startup_trace_jyut6ping3_mobile_select_schema_total` | `15,259,218us` | `15,285,489us` | working set delta `576,438,272` bytes; peak `1,861,349,376` bytes |
| `startup_trace_jyut6ping3_mobile_translator_install` | `15,128,769us` | `15,154,248us` | working set delta `602,775,552` bytes; peak `1,861,349,376` bytes |
| `startup_trace_jyut6ping3_mobile_spelling_algebra_expand` | `14,820,345us` | `14,844,195us` | working set delta `578,441,216` bytes; peak `1,861,349,376` bytes |
| `startup_trace_jyut6ping3_mobile_translator_index_build` | `134,103us` | `140,973us` | working set delta `536,444,928` bytes; allocator high-water noise from adjacent large allocations |
| `startup_trace_jyut6ping3_mobile_source_dictionary_parse_if_any` | `151,221us` | `156,737us` | working set delta `571,219,968` bytes; allocator high-water noise from adjacent large allocations |
| `startup_trace_jyut6ping3_mobile_filter_install` | `59,286us` | `60,661us` | working set delta `19,550,208` bytes |
| `startup_trace_jyut6ping3_mobile_compiled_table_load` | `4,751us` | `5,622us` | working set delta `4,308,992` bytes |
| `startup_trace_jyut6ping3_mobile_compiled_prism_load` | `3,840us` | `3,908us` | working set delta `36,864` bytes |
| `startup_trace_jyut6ping3_mobile_compiled_reverse_load` | `3,406us` | `3,551us` | working set delta `4,096` bytes |

## Result

The measured startup owner is `spelling_algebra_expand`, nested inside `translator_install` during `select_schema_total`. `deploy_real_jyut6ping3_mobile_cache_hit` is not the browser-matching row for this optimization because the browser evidence shows the long wait after deploy, during `rime:init` / schema selection.
