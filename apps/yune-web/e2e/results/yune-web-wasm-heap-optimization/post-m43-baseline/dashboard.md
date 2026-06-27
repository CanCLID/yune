# Yune Web Startup Benchmark Dashboard

## Summary

| Scenario | Samples | Schema | Mode | Public | Median ready ms | p95 ready ms | Median first key ms | Transfer bytes | Encoded bytes | WASM heap bytes | Peak WASM heap bytes | Cache h/m/e |
| --- | ---: | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| tracked-luna-cold | 3 | luna_pinyin | real-worker-cold | no | 776.0 | 932.0 | 28.0 | 5705544 | 5704344 | 134217728 | 134217728 | 0/0/0 |
| tracked-luna-warm-reload | 3 | luna_pinyin | real-worker-warm-reload | no | 246.0 | 252.0 | 23.0 | 0 | 5985570 | 134217728 | 134217728 | 0/0/0 |
| tracked-luna-warm-new-page | 3 | luna_pinyin | real-worker-warm-new-page | no | 331.0 | 331.0 | 23.0 | 0 | 5704344 | 134217728 | 134217728 | 0/0/0 |
| tracked-jyut-cold | 3 | jyut6ping3_mobile | real-worker-cold | no | 1250.0 | 1576.0 | 13.0 | 35116704 | 35115504 | 134217728 | 134217728 | 0/0/0 |
| tracked-jyut-warm-reload | 3 | jyut6ping3_mobile | real-worker-warm-reload | no | 672.0 | 763.0 | 14.0 | 0 | 38424055 | 134217728 | 134217728 | 0/0/0 |
| tracked-jyut-warm-new-page | 3 | jyut6ping3_mobile | real-worker-warm-new-page | no | 768.0 | 800.0 | 14.0 | 0 | 35115504 | 134217728 | 134217728 | 0/0/0 |
| tracked-mock-cold | 3 | luna_pinyin | mock-worker-cold | no | 482.0 | 510.0 | 13.0 | 747906 | 746706 | 0 | 0 | 0/0/0 |
| tracked-mock-warm | 3 | luna_pinyin | mock-worker-warm | no | 314.0 | 317.0 | 14.0 | 747906 | 746706 | 0 | 0 | 0/0/0 |
| public-luna-cold | 3 | luna_pinyin | real-worker-cold | yes | 777.0 | 799.0 | 31.0 | 5705484 | 5704284 | 134217728 | 134217728 | 2/20/0 |
| public-jyut-cold | 3 | jyut6ping3_mobile | real-worker-cold | yes | 1263.0 | 1290.0 | 13.0 | 35116644 | 35115444 | 134217728 | 134217728 | 1/36/0 |

## Startup Owner Map

| Scenario | Top owner | Owner median ms | Ready median ms | Ready p95 ms |
| --- | --- | ---: | ---: | ---: |
| tracked-luna-cold | React/browser ready residual | 495.0 | 776.0 | 932.0 |
| tracked-luna-warm-reload | worker total to initialized | 203.0 | 246.0 | 252.0 |
| tracked-luna-warm-new-page | worker total to initialized | 220.0 | 331.0 | 331.0 |
| tracked-jyut-cold | worker total to initialized | 770.0 | 1250.0 | 1576.0 |
| tracked-jyut-warm-reload | worker total to initialized | 625.0 | 672.0 | 763.0 |
| tracked-jyut-warm-new-page | worker total to initialized | 661.0 | 768.0 | 800.0 |
| tracked-mock-cold | React/browser ready residual | 482.0 | 482.0 | 510.0 |
| tracked-mock-warm | React/browser ready residual | 314.0 | 314.0 | 317.0 |
| public-luna-cold | React/browser ready residual | 475.0 | 777.0 | 799.0 |
| public-jyut-cold | worker total to initialized | 778.0 | 1263.0 | 1290.0 |

## Asset Transfer By Group

| Group | Transfer bytes | Encoded bytes | Duration ms |
| --- | ---: | ---: | ---: |
| wasm binary | 0 | 2587623 | 0.0 |
| schema binary | 0 | 1341677 | 0.0 |
| schema yaml | 0 | 846831 | 0.0 |
| other | 492074 | 491780 | 248.0 |
| app js | 220820 | 220520 | 3.0 |
| wasm glue | 0 | 72860 | 0.0 |
| opencc | 0 | 69609 | 0.0 |
| worker script | 2425 | 41163 | -80.0 |
| app css | 32581 | 32281 | 2.0 |

## Browser Memory

| Scenario | WASM heap | Peak WASM heap | JS heap used | JS heap total | DOM nodes | Windows working set |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| tracked-luna-cold | 134217728 | 134217728 | 6349996 | 11730944 | 1096 | 0 |
| tracked-luna-warm-reload | 134217728 | 134217728 | 9652624 | 18612224 | 2238 | 0 |
| tracked-luna-warm-new-page | 134217728 | 134217728 | 6458420 | 11993088 | 1106 | 0 |
| tracked-jyut-cold | 134217728 | 134217728 | 6788844 | 20054016 | 987 | 0 |
| tracked-jyut-warm-reload | 134217728 | 134217728 | 6619000 | 15859712 | 954 | 0 |
| tracked-jyut-warm-new-page | 134217728 | 134217728 | 6761508 | 11927552 | 1107 | 0 |
| tracked-mock-cold | 0 | 0 | 4874500 | 8519680 | 635 | 0 |
| tracked-mock-warm | 0 | 0 | 7154512 | 11141120 | 1270 | 0 |
| public-luna-cold | 134217728 | 134217728 | 5757624 | 11730944 | 1060 | 0 |
| public-jyut-cold | 134217728 | 134217728 | 6348384 | 38813696 | 987 | 0 |
