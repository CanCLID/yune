# M27 Native Startup After

> **Status:** Captured after M27 optimization - **Milestone:** M27 (TypeDuck-Web startup/runtime init) - **Updated:** 2026-06-22 - **Type:** evidence

Command:

```powershell
cmd /c "cargo bench -p yune-rime-api --bench frontend_baselines > target\m27-frontend-baselines-after-final.txt 2>&1"
```

Captured output: `target/m27-frontend-baselines-after-final.txt`.

## Browser-Matching Startup Row

| Row | Before Median | After Median | Change | After P95 |
|---|---:|---:|---:|---:|
| `startup_real_jyut6ping3_mobile_runtime_ready` | `15,546,100.900us` | `6,353,395.300us` | `-59.1%` | `6,372,286.000us` |
| `startup_real_luna_pinyin_runtime_ready` | `834,006.700us` | `573,094.500us` | `-31.3%` | `601,461.400us` |

## Split Owners

| Row | Before Median | After Median | Change | After Memory Notes |
|---|---:|---:|---:|---|
| `startup_trace_jyut6ping3_mobile_select_schema_total` | `15,259,218us` | `5,521,290us` | `-63.8%` | working set delta `612,667,392` bytes; peak `1,791,848,448` bytes |
| `startup_trace_jyut6ping3_mobile_translator_install` | `15,128,769us` | `5,393,107us` | `-64.4%` | working set delta `647,008,256` bytes; peak `1,791,848,448` bytes |
| `startup_trace_jyut6ping3_mobile_spelling_algebra_expand` | `14,820,345us` | `5,089,881us` | `-65.7%` | working set delta `622,874,624` bytes; peak `1,791,848,448` bytes |
| `startup_trace_jyut6ping3_mobile_translator_index_build` | `134,103us` | `134,731us` | `+0.5%` | working set delta `581,357,568` bytes; allocator high-water noise from adjacent large allocations |
| `startup_trace_jyut6ping3_mobile_source_dictionary_parse_if_any` | `151,221us` | `150,649us` | `-0.4%` | working set delta `615,342,080` bytes; allocator high-water noise from adjacent large allocations |
| `startup_trace_jyut6ping3_mobile_filter_install` | `59,286us` | `57,444us` | `-3.1%` | working set delta `14,258,176` bytes |
| `startup_trace_jyut6ping3_mobile_compiled_table_load` | `4,751us` | `4,861us` | `+2.3%` | working set delta `4,308,992` bytes |
| `startup_trace_jyut6ping3_mobile_compiled_prism_load` | `3,840us` | `3,601us` | `-6.2%` | working set delta `12,288` bytes |
| `startup_trace_jyut6ping3_mobile_compiled_reverse_load` | `3,406us` | `3,100us` | `-9.0%` | working set delta `4,096` bytes |

## Result

The browser-matching native startup row improved materially, from about `15.55s` median to about `6.35s` median. The dominant split owner remains `spelling_algebra_expand`, but its median dropped from about `14.82s` to about `5.09s`.
