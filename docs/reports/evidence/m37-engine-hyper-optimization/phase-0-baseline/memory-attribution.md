# M37 Phase 0 Memory Attribution

Source files:

- `docs/reports/evidence/m36-product-path/phase-4-final/track-b-yune-product/summary.csv`
- `docs/reports/evidence/m37-engine-hyper-optimization/phase-0-baseline/track-b-yune-product/summary.csv`
- `docs/reports/evidence/m37-engine-hyper-optimization/phase-0-baseline/track-b-yune-product/product_path_status.csv`

No Windows heap-profiler trace was captured in this run. The fallback attribution is working-set evidence plus loader/storage counters from the native harness and code owner inspection.

## Product Memory Owners

| Owner | Phase-0 evidence | Owner decision |
| --- | --- | --- |
| Table bytes | Product compiled status was fresh, but the selected compact table path parsed the `.table.bin` into owned `String` rows. | Top fix owner for M37 storage. |
| Parsed table storage | `CompactTableStore` still stored `Vec<String>` codes plus `String` candidate text rows, so it repeated the M36 owned no-marisa heap mirror. | Removed in final storage path by byte-backed offsets into mapped bytes. |
| Prism and reverse storage | Product build artifacts were present and loaded: prism/reverse payloads remained required for spelling, correction, and rich comments. | Kept in scope but not the first moved owner. |
| Candidate/context storage | Phase-0 `hai` stored 11,289 candidates and cloned 5 for page export. | Fixed by page-bounded materialization. |
| Userdb and learning state | No large per-row counter in phase-0 metrics. | Not the top owner for `hai`. |
| ABI buffers | `abi_candidates_exported=5`; `abi_get_context_ns` was about 0.36 ms. | Not the top memory or latency owner. |
| Allocator high-water/fragmentation | Working set stayed near the M36 final product plateau. | Reduced only after removing owned table row mirrors. |

## Working-Set Baseline

| Row | M36 final median working set | M37 phase-0 median working set |
| --- | ---: | ---: |
| `hai` | 777,535,488 B | 776,658,944 B |
| `ngohaig` | 777,560,064 B | 776,675,328 B |
| `loengjathau` | 777,568,256 B | 776,687,616 B |
| `jigaajiusihaa` | 777,568,256 B | 776,679,424 B |
| Product peak | 928,350,208 B | 928,677,888 B |

Phase 0 memory did not move from M36. The owner was not candidate export; it was the product storage path plus retained compiled payload structures. M37 therefore needed both the page-bounded candidate fix and a mapped/byte-backed table storage fix before closeout.
