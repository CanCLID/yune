# Yune Web Startup Benchmark Dashboard

## Summary

| Scenario | Samples | Schema | Mode | Public | Median ready ms | p95 ready ms | Median first key ms | Transfer bytes | Encoded bytes | WASM heap bytes | Peak WASM heap bytes | Cache h/m/e |
| --- | ---: | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| tracked-luna-cold | 1 | luna_pinyin | real-worker-cold | no | 928.0 | 928.0 | 49.0 | 5693804 | 5692604 | 134217728 | 134217728 | 2/20/0 |
| tracked-luna-warm-reload | 1 | luna_pinyin | real-worker-warm-reload | no | 253.0 | 253.0 | 31.0 | 0 | 9001155 | 134217728 | 134217728 | 22/0/0 |
| tracked-luna-warm-new-page | 1 | luna_pinyin | real-worker-warm-new-page | no | 311.0 | 311.0 | 36.0 | 0 | 5692604 | 134217728 | 134217728 | 22/0/0 |
| tracked-jyut-cold | 1 | jyut6ping3_mobile | real-worker-cold | no | 1360.0 | 1360.0 | 16.0 | 35104958 | 35103758 | 134217728 | 134217728 | 1/36/0 |
| tracked-jyut-warm-reload | 1 | jyut6ping3_mobile | real-worker-warm-reload | no | 662.0 | 662.0 | 13.0 | 0 | 38412315 | 134217728 | 134217728 | 37/0/0 |
| tracked-jyut-warm-new-page | 1 | jyut6ping3_mobile | real-worker-warm-new-page | no | 928.0 | 928.0 | 14.0 | 0 | 35103764 | 134217728 | 134217728 | 37/0/0 |
| tracked-mock-cold | 1 | luna_pinyin | mock-worker-cold | no | 508.0 | 508.0 | 14.0 | 747891 | 746691 | 0 | 0 | 0/0/0 |
| tracked-mock-warm | 1 | luna_pinyin | mock-worker-warm | no | 312.0 | 312.0 | 11.0 | 747906 | 746706 | 0 | 0 | 0/0/0 |
| public-luna-cold | 1 | luna_pinyin | real-worker-cold | yes | 836.0 | 836.0 | 33.0 | 5693804 | 5692604 | 134217728 | 134217728 | 2/20/0 |
| public-jyut-cold | 1 | jyut6ping3_mobile | real-worker-cold | yes | 1378.0 | 1378.0 | 14.0 | 35104964 | 35103764 | 134217728 | 134217728 | 1/36/0 |

## Startup Owner Map

| Scenario | Top owner | Owner median ms | Ready median ms | Ready p95 ms |
| --- | --- | ---: | ---: | ---: |
| tracked-luna-cold | React/browser ready residual | 595.0 | 928.0 | 928.0 |
| tracked-luna-warm-reload | worker total to initialized | 208.0 | 253.0 | 253.0 |
| tracked-luna-warm-new-page | worker total to initialized | 203.0 | 311.0 | 311.0 |
| tracked-jyut-cold | worker total to initialized | 851.0 | 1360.0 | 1360.0 |
| tracked-jyut-warm-reload | worker total to initialized | 616.0 | 662.0 | 662.0 |
| tracked-jyut-warm-new-page | worker total to initialized | 799.0 | 928.0 | 928.0 |
| tracked-mock-cold | React/browser ready residual | 508.0 | 508.0 | 508.0 |
| tracked-mock-warm | React/browser ready residual | 312.0 | 312.0 | 312.0 |
| public-luna-cold | React/browser ready residual | 501.0 | 836.0 | 836.0 |
| public-jyut-cold | worker total to initialized | 879.0 | 1378.0 | 1378.0 |

## Asset Transfer By Group

| Group | Transfer bytes | Encoded bytes | Duration ms |
| --- | ---: | ---: | ---: |
| wasm binary | 0 | 2575943 | 0.0 |
| schema binary | 0 | 1341677 | 0.0 |
| schema yaml | 0 | 846831 | 0.0 |
| other | 492065 | 491780 | 257.0 |
| app js | 220820 | 220520 | 2.0 |
| wasm glue | 0 | 72860 | 0.0 |
| opencc | 0 | 69609 | 0.0 |
| worker script | 2425 | 41103 | -78.0 |
| app css | 32581 | 32281 | 2.0 |

## Browser Memory

| Scenario | WASM heap | Peak WASM heap | JS heap used | JS heap total | DOM nodes | Windows working set |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| tracked-luna-cold | 134217728 | 134217728 | 6403224 | 11993088 | 1060 | 0 |
| tracked-luna-warm-reload | 134217728 | 134217728 | 9422612 | 18612224 | 2238 | 0 |
| tracked-luna-warm-new-page | 134217728 | 134217728 | 6010180 | 12255232 | 1060 | 0 |
| tracked-jyut-cold | 134217728 | 134217728 | 5460640 | 38813696 | 941 | 0 |
| tracked-jyut-warm-reload | 134217728 | 134217728 | 7830492 | 14024704 | 2091 | 0 |
| tracked-jyut-warm-new-page | 134217728 | 134217728 | 7488244 | 41697280 | 1059 | 0 |
| tracked-mock-cold | 0 | 0 | 4885216 | 8519680 | 635 | 0 |
| tracked-mock-warm | 0 | 0 | 6605680 | 11927552 | 1270 | 0 |
| public-luna-cold | 134217728 | 134217728 | 5718760 | 11993088 | 1106 | 0 |
| public-jyut-cold | 134217728 | 134217728 | 6180116 | 38813696 | 985 | 0 |
