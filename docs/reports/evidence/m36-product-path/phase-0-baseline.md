# M36 Phase 0 Baseline

Phase 0 replaced the managed `.NET`/PInvoke benchmark confound with the native
Rust `native_inprocess_benchmark` harness.

Raw run:

- [`phase-0-baseline/summary.csv`](./phase-0-baseline/summary.csv)
- [`phase-0-baseline/product_path_status.csv`](./phase-0-baseline/product_path_status.csv)

Track separation:

- Track A is only `luna_pinyin`, Yune versus librime `1.17.0`.
- Track B is only Yune `jyut6ping3_mobile`, before/after product-path evidence.

Product-path status in the baseline:

| Dictionary | Checksum status | Table parse | Prism parse | Reverse parse | Compiled ready |
| --- | --- | --- | --- | --- | --- |
| `jyut6ping3` | stale | `UnsupportedSection { role: "marisa string_table" }` | `UnsupportedVersion` | `UnsupportedSection { role: "marisa reverse key/value trie" }` | false |
| `jyut6ping3_scolar` | stale | `UnsupportedSection { role: "marisa string_table" }` | `UnsupportedVersion` | `UnsupportedSection { role: "marisa reverse key/value trie" }` | false |

Track B baseline medians:

| Row | Median latency | Median working set | Max peak working set |
| --- | ---: | ---: | ---: |
| startup ready | `201,811.100us` | `818.7 MB` | `1000.4 MB` |
| session create/select/destroy | `243,946.900us` | `806.7 MB` | `1000.4 MB` |
| `hai` | `21,541.967us` | `822.9 MB` | `1000.4 MB` |
| `ngohaig` | `14,943.043us` | `823.2 MB` | `1000.4 MB` |
| `loengjathau` | `16,309.045us` | `823.7 MB` | `1000.4 MB` |
| `jigaajiusihaa` | `27,633.869us` | `824.8 MB` | `1000.4 MB` |

Conclusion: the shipped product `.bin` files were not an active compiled
runtime path for Yune before M36. Product behavior reached ready state through
source fallback and heap-backed translator storage.
