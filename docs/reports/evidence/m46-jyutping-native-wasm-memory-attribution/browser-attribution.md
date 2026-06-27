# M46 Phase 0 Browser Attribution

Evidence root:
[`../../../../apps/yune-web/e2e/results/yune-web-jyutping-memory-attribution/`](../../../../apps/yune-web/e2e/results/yune-web-jyutping-memory-attribution/)

## Single-Schema WASM

Evidence:
[`phase-0-post-web01-single-schema/`](../../../../apps/yune-web/e2e/results/yune-web-jyutping-memory-attribution/phase-0-post-web01-single-schema/)

| Scenario | Schema | Samples | Median ready | Median WASM heap | Max peak WASM heap |
| --- | --- | ---: | ---: | ---: | ---: |
| tracked Luna cold | `luna_pinyin` | 3 | `1216 ms` | `167,772,160 B` (`160.0 MiB`) | `167,772,160 B` (`160.0 MiB`) |
| tracked Jyutping cold | `jyut6ping3_mobile` | 3 | `5890 ms` | `936,509,440 B` (`893.1 MiB`) | `936,509,440 B` (`893.1 MiB`) |
| public Luna cold | `luna_pinyin` | 3 | `1207 ms` | `167,772,160 B` (`160.0 MiB`) | `167,772,160 B` (`160.0 MiB`) |
| public Jyutping cold | `jyut6ping3_mobile` | 3 | `5608 ms` | `936,509,440 B` (`893.1 MiB`) | `936,509,440 B` (`893.1 MiB`) |

The WEB-01 browser high-water still reproduces on post-WEB-01 main.

## Asset Families

Evidence:
[`asset-family/phase-0-post-web01-asset-family/`](../../../../apps/yune-web/e2e/results/yune-web-jyutping-memory-attribution/asset-family/phase-0-post-web01-asset-family/)

Tracked build rows:

| Family | Requested family bytes | Unique encoded resource bytes | WASM ready | WASM peak | Read |
| --- | ---: | ---: | ---: | ---: | --- |
| `jyutping-core` | `4,630,356 B` | `11,575,930 B` | `936,509,440 B` | `936,509,440 B` | Core-only reaches full high-water. |
| `jyutping-scolar` | `23,785,977 B` | `30,731,551 B` | `936,509,440 B` | `936,509,440 B` | Scolar changes payload bytes but not heap size. |
| `reverse-lookup` | `7,203,884 B` | `14,149,458 B` | `936,509,440 B` | `936,509,440 B` | Reverse assets are not the heap owner. |
| `opencc` | `4,700,673 B` | `11,646,247 B` | `936,509,440 B` | `936,509,440 B` | OpenCC is not the heap owner. |
| `extras` | `0 B` | `6,947,341 B` | `936,509,440 B` | `936,509,440 B` | Empty attribution family still reaches full high-water. |
| `full-jyutping` | `26,429,822 B` | `33,375,396 B` | `936,509,440 B` | `936,509,440 B` | Normal full payload. |

This confirms WEB-01's handoff result: browser payload-family changes alone do
not explain the `893.1 MiB` linear memory row.

## Schema Switch

Evidence:
[`phase-0-schema-switch-current/`](../../../../apps/yune-web/e2e/results/yune-web-jyutping-memory-attribution/phase-0-schema-switch-current/)

| Scenario | Verdict | Failed step | Max observed WASM |
| --- | --- | --- | ---: |
| Clean Jyutping page, `nei` | `pass` | none | `936,509,440 B` (`893.1 MiB`) |
| Cangjie `a` -> Luna `hao` -> Jyutping `nei` | `candidate-missing` | `jyutping-nei-after-switch` | `936,509,440 B` (`893.1 MiB`) |

The current post-WEB-01 runtime still loses the Jyutping candidate after this
schema-switch sequence. The older `~1.9 GiB` high-water did not reproduce in
this new structured capture; the correctness failure did.
