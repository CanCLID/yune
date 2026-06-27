# Yune Web Startup Benchmark Dashboard

## Summary

| Scenario | Samples | Schema | Mode | Public | Median ready ms | p95 ready ms | Median first key ms | Transfer bytes | Encoded bytes | WASM heap bytes | Peak WASM heap bytes | Cache h/m/e |
| --- | ---: | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| tracked-luna-cold | 3 | luna_pinyin | real-worker-cold | no | 1216.0 | 3939.0 | 34.0 | 36594991 | 36593791 | 167772160 | 167772160 | 0/0/0 |
| tracked-luna-warm-reload | 3 | luna_pinyin | real-worker-warm-reload | no | 602.0 | 603.0 | 38.0 | 0 | 36875347 | 167772160 | 167772160 | 0/0/0 |
| tracked-luna-warm-new-page | 3 | luna_pinyin | real-worker-warm-new-page | no | 651.0 | 667.0 | 38.0 | 0 | 36593791 | 167772160 | 167772160 | 0/0/0 |
| tracked-jyut-cold | 3 | jyut6ping3_mobile | real-worker-cold | no | 5890.0 | 6218.0 | 45.0 | 33359106 | 33357906 | 936509440 | 936509440 | 0/0/0 |
| tracked-jyut-warm-reload | 3 | jyut6ping3_mobile | real-worker-warm-reload | no | 5168.0 | 5234.0 | 47.0 | 0 | 36666793 | 936509440 | 936509440 | 0/0/0 |
| tracked-jyut-warm-new-page | 3 | jyut6ping3_mobile | real-worker-warm-new-page | no | 4854.0 | 5336.0 | 42.0 | 0 | 33357906 | 936509440 | 936509440 | 0/0/0 |
| tracked-mock-cold | 3 | luna_pinyin | mock-worker-cold | no | 625.0 | 629.0 | 26.0 | 747840 | 746640 | 0 | 0 | 0/0/0 |
| tracked-mock-warm | 3 | luna_pinyin | mock-worker-warm | no | 415.0 | 422.0 | 26.0 | 747840 | 746640 | 0 | 0 | 0/0/0 |
| public-luna-cold | 3 | luna_pinyin | real-worker-cold | yes | 1207.0 | 1227.0 | 30.0 | 36594931 | 36593731 | 167772160 | 167772160 | 2/24/0 |
| public-jyut-cold | 3 | jyut6ping3_mobile | real-worker-cold | yes | 5608.0 | 5789.0 | 44.0 | 33359046 | 33357846 | 936509440 | 936509440 | 1/35/0 |

## Startup Owner Map

| Scenario | Top owner | Owner median ms | Ready median ms | Ready p95 ms |
| --- | --- | ---: | ---: | ---: |
| tracked-luna-cold | React/browser ready residual | 650.0 | 1216.0 | 3939.0 |
| tracked-luna-warm-reload | worker total to initialized | 537.0 | 602.0 | 603.0 |
| tracked-luna-warm-new-page | worker total to initialized | 547.0 | 651.0 | 667.0 |
| tracked-jyut-cold | worker total to initialized | 5282.0 | 5890.0 | 6218.0 |
| tracked-jyut-warm-reload | worker total to initialized | 5102.0 | 5168.0 | 5234.0 |
| tracked-jyut-warm-new-page | worker total to initialized | 4755.0 | 4854.0 | 5336.0 |
| tracked-mock-cold | React/browser ready residual | 625.0 | 625.0 | 629.0 |
| tracked-mock-warm | React/browser ready residual | 415.0 | 415.0 | 422.0 |
| public-luna-cold | React/browser ready residual | 609.0 | 1207.0 | 1227.0 |
| public-jyut-cold | worker total to initialized | 4989.0 | 5608.0 | 5789.0 |

## Asset Transfer By Group

| Group | Transfer bytes | Encoded bytes | Duration ms |
| --- | ---: | ---: | ---: |
| schema binary | 0 | 16647200 | 0.0 |
| schema yaml | 0 | 4339161 | 0.0 |
| wasm binary | 0 | 2594503 | 0.0 |
| other | 491496 | 491196 | 359.0 |
| app js | 221186 | 220886 | 3.0 |
| wasm glue | 0 | 72378 | 0.0 |
| opencc | 0 | 66408 | 0.0 |
| worker script | 2425 | 42850 | -55.0 |
| app css | 32733 | 32433 | 2.0 |

## Browser Memory

| Scenario | WASM heap | Peak WASM heap | JS heap used | JS heap total | DOM nodes | Windows working set |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| tracked-luna-cold | 167772160 | 167772160 | 6441124 | 33566720 | 1095 | 756838400 |
| tracked-luna-warm-reload | 167772160 | 167772160 | 9064712 | 40173568 | 2227 | 771137536 |
| tracked-luna-warm-new-page | 167772160 | 167772160 | 5736480 | 33828864 | 1095 | 750235648 |
| tracked-jyut-cold | 936509440 | 936509440 | 5646528 | 10596352 | 1004 | 1370193920 |
| tracked-jyut-warm-reload | 936509440 | 936509440 | 4954612 | 11902976 | 997 | 1568190464 |
| tracked-jyut-warm-new-page | 936509440 | 936509440 | 6108804 | 38264832 | 956 | 1376583680 |
| tracked-mock-cold | 0 | 0 | 4534224 | 8232960 | 635 | 373735424 |
| tracked-mock-warm | 0 | 0 | 7770600 | 11640832 | 1779 | 427446272 |
| public-luna-cold | 167772160 | 167772160 | 5782800 | 33828864 | 1095 | 764284928 |
| public-jyut-cold | 936509440 | 936509440 | 4845732 | 8757248 | 981 | 1373306880 |
