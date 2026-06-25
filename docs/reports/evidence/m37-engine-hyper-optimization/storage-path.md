# M37 Storage Path

Final native evidence:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\m37-engine-hyper-optimization\phase-3-final-native -Iterations 5 -SessionIterations 20 -KeyIterations 20 -DeployProductBeforeBenchmark
```

Source files:

- `phase-3-final-native/track-b-yune-product/product_path_status.csv`
- `phase-3-final-native/track-b-yune-product/summary.csv`
- `phase-3-final-native/track-b-yune-product/m37_metrics.csv`

## Selected Product Storage

| Dictionary | Selected storage | Table format | Mapping mode | Source fallback | Byte source len | Stored entries |
| --- | --- | --- | --- | --- | ---: | ---: |
| `jyut6ping3` | byte_backed | yune_no_marisa_compact | mmap | false | 15,248,382 B | 127,143 |
| `jyut6ping3_scolar` | byte_backed | yune_no_marisa_compact | mmap | false | 27,325,622 B | 127,143 |

The selected native product table path now maps deployed `.table.bin` bytes and stores compact row offsets into that byte source. Candidate text is borrowed from the byte source at lookup time. This removes the M36 `CompactTableStore` owned `String` mirror for product table rows.

The product path stayed fresh and compiled:

| Dictionary | Checksum | Table | Prism | Reverse |
| --- | --- | --- | --- | --- |
| `jyut6ping3` | fresh | ok | ok | ok |
| `jyut6ping3_scolar` | fresh | ok | ok | ok |

## Product Movement

| Row | M36 final median us | M37 final median us | Delta | M36 median working set | M37 final median working set |
| --- | ---: | ---: | ---: | ---: | ---: |
| `hai` | 15,241.000 | 8,336.800 | -45.3% | 777,535,488 B | 367,271,936 B |
| `ngohaig` | 3,465.057 | 1,861.586 | -46.3% | 777,560,064 B | 367,931,392 B |
| `loengjathau` | 3,754.855 | 2,164.609 | -42.4% | 777,568,256 B | 364,859,392 B |
| `jigaajiusihaa` | 5,065.308 | 3,189.085 | -37.0% | 777,568,256 B | 369,405,952 B |
| Product peak | n/a | n/a | -45.7% | 928,350,208 B | 504,377,344 B |

The final `hai` row moved from the M36 final 15,241.000 us median to 8,336.800 us. Final product median working set moved from about 777 MB to about 367 MB on the key rows, and peak moved from 928,350,208 B to 504,377,344 B.

## Final `hai` Counters

Average per measured final `hai` sample:

| Counter | Final value |
| --- | ---: |
| `process_key_ns` | 25,240,215 ns |
| `translator_ns` | 24,340,220 ns |
| `lookup_views_visited` | 19,918 |
| `owned_candidates_materialized` | 52 |
| `candidates_sorted` | 48 |
| `filter_pipeline_ns` | 44,425 ns |
| `context_full_snapshot_candidates_cloned` | 0 |
| `context_page_snapshot_candidates_cloned` | 5 |
| `abi_get_context_ns` | 370,880 ns |
| `abi_candidates_exported` | 5 |

The remaining `hai` owner is lookup-view scanning for the visible/bounded candidate window. The M37 hard gate was candidate materialization and product storage, not eliminating all prefix scanning.
