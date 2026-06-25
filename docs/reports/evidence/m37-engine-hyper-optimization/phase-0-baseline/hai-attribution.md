# M37 Phase 0 Hai Attribution

Baseline run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\m37-engine-hyper-optimization\phase-0-baseline -Iterations 5 -SessionIterations 20 -KeyIterations 20 -DeployProductBeforeBenchmark
```

Source files:

- `track-b-yune-product/summary.csv`
- `track-b-yune-product/m37_metrics.csv`

## Track B Baseline

| Row | M36 final median us | M37 phase-0 median us | M37 phase-0 median working set |
| --- | ---: | ---: | ---: |
| `hai` | 15,241.000 | 15,334.967 | 776,658,944 B |
| `ngohaig` | 3,465.057 | 3,653.843 | 776,675,328 B |
| `loengjathau` | 3,754.855 | 3,959.500 | 776,687,616 B |
| `jigaajiusihaa` | 5,065.308 | 5,369.169 | 776,679,424 B |

## `hai` Owner Split

Average per measured `hai` sample in phase 0:

| Owner | Count or time |
| --- | ---: |
| `process_key_ns` | 46,306,710 ns |
| `translator_ns` | 37,506,130 ns |
| `lookup_views_visited` | 19,918 |
| `owned_candidates_materialized` | 19,918 |
| `candidates_sorted` | 11,289 |
| `filter_pipeline_ns` | 7,273,340 ns |
| `context_full_snapshot_candidates_cloned` | 0 |
| `context_page_snapshot_candidates_cloned` | 5 |
| `abi_get_context_ns` | 364,130 ns |
| `abi_candidates_exported` | 5 |

Phase 0 attributed `hai` to full product candidate materialization and filtering, not ABI context export. `RimeGetContext` already exported only 5 visible candidates, but the engine still built 19,918 owned candidates, sorted 11,289, stored 11,289, and ran the filter pipeline over the full row set.

## Ordering Decision

Task 2/Task 3 were prioritized before broad storage work because the top `hai` owner was candidate materialization/filtering. Task 1 remained open as a non-waivable M37 gate because phase-0 memory still showed the product table path carrying M36-style owned storage.
