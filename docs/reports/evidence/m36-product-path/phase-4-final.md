# M36 Phase 4 Final

Raw run:

- [`phase-4-final/summary.csv`](./phase-4-final/summary.csv)
- [`phase-4-final/product_path_status.csv`](./phase-4-final/product_path_status.csv)

Track B before/after medians:

| Row | Baseline latency | Final latency | Change | Baseline working set | Final working set | Working-set change |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| startup ready | `201,811.100us` | `175,424.800us` | `-13.1%` | `818.7 MB` | `738.8 MB` | `-79.9 MB` |
| session create/select/destroy | `243,946.900us` | `219,919.200us` | `-9.8%` | `806.7 MB` | `726.6 MB` | `-80.2 MB` |
| `hai` | `21,541.967us` | `15,241.000us` | `-29.2%` | `822.9 MB` | `741.5 MB` | `-81.4 MB` |
| `ngohaig` | `14,943.043us` | `3,465.057us` | `-76.8%` | `823.2 MB` | `741.5 MB` | `-81.7 MB` |
| `loengjathau` | `16,309.045us` | `3,754.855us` | `-77.0%` | `823.7 MB` | `741.5 MB` | `-82.2 MB` |
| `jigaajiusihaa` | `27,633.869us` | `5,065.308us` | `-81.7%` | `824.8 MB` | `741.5 MB` | `-83.2 MB` |

Track B max peak working set moved from `1000.4 MB` to `885.3 MB`
(`-115.0 MB`) across the measured product rows.

Track A final fair comparison remains separate:

| Row | Yune median | librime 1.17.0 median | Ratio |
| --- | ---: | ---: | ---: |
| startup ready | `48,144.900us` | `22,105.300us` | `2.18x` |
| session create/select/destroy | `47,112.900us` | `22,852.100us` | `2.06x` |
| `hao` | `4,072.000us` | `11.700us` | `348.03x` |
| `ni` | `2,977.300us` | `14.450us` | `206.04x` |
| `zhongguo` | `4,403.738us` | `178.600us` | `24.66x` |

The Track A cross-engine rows are retained as comparison evidence only. They
are not used as the TypeDuck product typing headline.
