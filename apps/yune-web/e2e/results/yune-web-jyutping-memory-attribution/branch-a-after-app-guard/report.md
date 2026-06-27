# M46 Jyutping Memory Attribution Browser Evidence

| Scenario | Initialized | Verdict | Steps | Failed steps | Max observed WASM | Worker action errors |
| --- | --- | --- | ---: | --- | ---: | ---: |
| clean-jyutping | yes | pass | 1 | - | 893.1 MiB | 0 |
| schema-switch | yes | pass | 3 | - | 893.1 MiB | 0 |

## Steps

| Scenario | Step | Active schema | Input | Top candidate | Candidate count | Result | WASM current | WASM peak | Worker action errors |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: |
| clean-jyutping | jyutping-only-nei | jyut6ping3 | nei | 你 | 6 | pass | 893.1 MiB | 893.1 MiB | 0 |
| schema-switch | cangjie-a | cangjie5 | a | 日 | 5 | pass | 893.1 MiB | 893.1 MiB | 0 |
| schema-switch | luna-hao | luna_pinyin | hao | 好 | 5 | pass | 893.1 MiB | 893.1 MiB | 0 |
| schema-switch | jyutping-nei-after-switch | jyut6ping3 | nei | 你 | 6 | pass | 893.1 MiB | 893.1 MiB | 0 |
