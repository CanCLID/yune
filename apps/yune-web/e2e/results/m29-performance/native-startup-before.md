# M29 Native Startup Before

Date: 2026-06-22

Command:

```powershell
cmd /c "cargo bench -p yune-rime-api --bench frontend_baselines > target\m29-frontend-baselines-before.txt 2>&1"
```

Captured output: `target/m29-frontend-baselines-before.txt`.

## Startup Rows

| Row | Median | P95 | Notes |
| --- | ---: | ---: | --- |
| `startup_real_jyut6ping3_mobile_runtime_ready` | `6,441,132.800us` | `6,487,698.400us` | Full TypeDuck real-assets startup to runtime ready. |
| `startup_trace_jyut6ping3_mobile_select_schema_total` | `5,705,357us` | `5,779,699us` | Working-set delta `692,527,104` bytes; repeated-trace peak `1,795,403,776` bytes. |
| `startup_trace_jyut6ping3_mobile_translator_install` | `5,576,695us` | `5,644,827us` | Largest owner group under schema selection. |
| `startup_trace_jyut6ping3_mobile_spelling_algebra_expand` | `5,253,577us` | `5,320,971us` | Fresh M29 top startup owner. |
| `startup_trace_jyut6ping3_mobile_source_dictionary_parse_if_any` | `151,949us` | `162,382us` | Not the top startup target. |
| `startup_trace_jyut6ping3_mobile_translator_index_build` | `141,716us` | `152,295us` | Not the top startup target. |

## Single-Startup Memory Row

`m29_single_startup_memory_jyut6ping3_mobile`:

- Total: `6,523.796ms`.
- Before: `4,739,072` bytes.
- After ready: `1,105,899,520` bytes.
- After finalize: `14,897,152` bytes.
- Peak: `1,126,006,784` bytes.
- Ready delta: `1,101,160,448` bytes.
- Finalize delta: `-1,091,002,368` bytes.

## Typing Context Rows

| Row | Median | P95 | Notes |
| --- | ---: | ---: | --- |
| `per_key_real_jyut6ping3_mobile_hai_full_abi` | `16,260.567us` | `17,674.467us` | Short input through full ABI. |
| `per_key_real_jyut6ping3_mobile_hai_engine_only` | `8,790.933us` | `9,723.100us` | Short input engine-only. |
| `per_key_real_jyut6ping3_mobile_jigaajiusihaa_full_abi` | `21,845.162us` | `23,221.831us` | Long phrase through full ABI. |
| `per_key_real_jyut6ping3_mobile_jigaajiusihaa_engine_only` | `15,156.508us` | `16,629.831us` | Long phrase engine-only. |
| `per_key_real_jyut6ping3_mobile_jigaajiusihaa_correction_engine_only` | `126,049.462us` | `130,484.600us` | Dynamic correction stress row. |

Finding: fresh M29 attribution kept `spelling_algebra_expand` as the top startup owner. Native typing rows were already far below startup cost and were handled as attribution evidence rather than the primary optimization target.
