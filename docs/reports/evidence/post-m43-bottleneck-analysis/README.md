# Post-M43 Native Bottleneck Analysis

Date: 2026-06-26

This evidence root is a post-M43 diagnostic pass. It is native-engine evidence
only. It does not claim browser, frontend, product-delivery, packaging,
public-demo, or TypeDuck-profile speed wins.

## Runs

### `fresh-native/`

Fresh Track A `luna_pinyin` Yune-vs-librime run on current `main`
`ad93ec787d2b6e4f952b05836e8f0ed46b5a79d2`.

This run is valid for Track A rows and prefix probes:

- `n`, `ni`
- `h`, `ha`, `hao`
- `z`, `zh`, `zho`, `zhon`, `zhong`, `zhongg`, `zhongguo`
- `cszysmsrsd`, `zybfshmsru`
- the two M40 long full-pinyin rows

The Track B rows in this run are diagnostic only because the product dictionary
was not redeployed before measurement and `source_fallback=true`.

### `fresh-native-trackb-deployed/`

Corrected deployed Track B slice with `-DeployProductBeforeBenchmark`.

This run is valid for Track B deployed product-path rows:

- `h`, `ha`, `hai`, `hau`, `nei`, `ngo`
- `neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung`

The corrected run preserves the deployed product storage state:

- `compiled_ready=true`
- `selected_storage=byte_backed`
- table/prism `mmap`
- selected heap mirrors `0`
- `source_fallback=false`

## Findings

Track A short keys remain slower than librime because Yune still spends most of
the row in translator production:

| Row | Yune median | librime median | Ratio | Main Yune owner |
| --- | ---: | ---: | ---: | --- |
| `hao` | `37.733 us` | `11.333 us` | `3.329x` | `33.817 us/key` translator |
| `ni` | `55.750 us` | `14.100 us` | `3.954x` | `51.500 us/key` translator |

The short-key raw lookup path is not the only owner. `hao` spends about
`12.100 us/key` in exact+prefix lookup, while `ni` spends about `20.050 us/key`;
the remaining translator time is bounded candidate selection, filtering,
quality/comment handling, and first-page materialization around the lookup
results.

Track A abbreviation latency is still dominated by the upstream sentence-model
abbreviation path:

| Row | Yune median | librime median | Ratio | Main Yune owner |
| --- | ---: | ---: | ---: | --- |
| `cszysmsrsd` | `4,126.180 us` | `1,227.830 us` | `3.361x` | `4,101.990 us/key` upstream sentence-model path |
| `zybfshmsru` | `4,244.470 us` | `836.990 us` | `5.071x` | `4,212.075 us/key` upstream sentence-model path |

The M40/M43 graph rebuild counter is not the top abbreviation owner anymore:
graph rebuild is only about `118.340 us/key` for `cszysmsrsd` and
`98.025 us/key` for `zybfshmsru`. The remaining abbreviation cost is likely in
abbreviation span discovery, repeated `model.has_code` checks, graph-to-sentence
ranking, or an uninstrumented part of `abbreviation_sentence_candidates`.

Corrected deployed Track B short product rows are much slower than the existing
long-row guard:

| Row | Median | Main Yune owner |
| --- | ---: | --- |
| `h` | `21,888.600 us` | `21,873.450 us/key` translator; `7,627` exact lookups/key |
| `ha` | `11,639.250 us` | `11,605.425 us/key` translator; `3,814` exact lookups/key |
| `hai` | `7,580.300 us` | `7,556.967 us/key` translator; `2,544.667` exact lookups/key |
| long Track B guard | `185.531 us` | `183.066 us/key` translator; guard still stable |

Track B short-row work is therefore a separate product-profile storage and
spelling-expansion owner. It should not be mixed into Track A `luna_pinyin`
parity work unless the milestone explicitly opens TypeDuck-profile scope.
