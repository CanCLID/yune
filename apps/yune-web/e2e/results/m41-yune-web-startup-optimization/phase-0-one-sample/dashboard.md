# M41 Startup Dashboard

## Summary

| Scenario | Samples | Schema | Mode | Public | Median ready ms | p95 ready ms | Median first key ms | Transfer bytes | Encoded bytes | Cache h/m/e |
| --- | ---: | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| tracked-luna-cold | 1 | luna_pinyin | real-worker-cold | no | 3115.0 | 3115.0 | 32.0 | 780501 | 779301 | 1/34/0 |
| tracked-luna-warm-reload | 1 | luna_pinyin | real-worker-warm-reload | no | 2399.0 | 2399.0 | 88.0 | 0 | 1060857 | 35/0/0 |
| tracked-luna-warm-new-page | 1 | luna_pinyin | real-worker-warm-new-page | no | 2438.0 | 2438.0 | 89.0 | 0 | 779301 | 35/0/0 |
| tracked-jyut-cold | 1 | jyut6ping3_mobile | real-worker-cold | no | 17041.0 | 17041.0 | 60.0 | 780501 | 779301 | 1/34/0 |
| tracked-jyut-warm-reload | 1 | jyut6ping3_mobile | real-worker-warm-reload | no | 15783.0 | 15783.0 | 59.0 | 0 | 1060857 | 35/0/0 |
| tracked-jyut-warm-new-page | 1 | jyut6ping3_mobile | real-worker-warm-new-page | no | 16081.0 | 16081.0 | 58.0 | 0 | 779301 | 35/0/0 |
| tracked-mock-cold | 1 | luna_pinyin | mock-worker-cold | no | 652.0 | 652.0 | 11.0 | 742520 | 741320 | 0/0/0 |
| tracked-mock-warm | 1 | luna_pinyin | mock-worker-warm | no | 413.0 | 413.0 | 28.0 | 742520 | 741320 | 0/0/0 |
| public-luna-cold | 1 | luna_pinyin | real-worker-cold | yes | 3119.0 | 3119.0 | 103.0 | 780501 | 779301 | 1/34/0 |
| public-jyut-cold | 1 | jyut6ping3_mobile | real-worker-cold | yes | 16872.0 | 16872.0 | 62.0 | 780501 | 779301 | 1/34/0 |

## Startup Owner Map

| Scenario | Top owner | Owner median ms | Ready median ms | Ready p95 ms |
| --- | --- | ---: | ---: | ---: |
| tracked-luna-cold | React/browser ready gap | 2361.0 | 3115.0 | 3115.0 |
| tracked-luna-warm-reload | React/browser ready gap | 1810.0 | 2399.0 | 2399.0 |
| tracked-luna-warm-new-page | React/browser ready gap | 1834.0 | 2438.0 | 2438.0 |
| tracked-jyut-cold | React/browser ready gap | 16346.0 | 17041.0 | 17041.0 |
| tracked-jyut-warm-reload | React/browser ready gap | 15259.0 | 15783.0 | 15783.0 |
| tracked-jyut-warm-new-page | React/browser ready gap | 15546.0 | 16081.0 | 16081.0 |
| tracked-mock-cold | React/browser ready gap | 652.0 | 652.0 | 652.0 |
| tracked-mock-warm | React/browser ready gap | 413.0 | 413.0 | 413.0 |
| public-luna-cold | React/browser ready gap | 2351.0 | 3119.0 | 3119.0 |
| public-jyut-cold | React/browser ready gap | 16170.0 | 16872.0 | 16872.0 |

## Asset Transfer By Group

| Group | Transfer bytes | Encoded bytes | Duration ms |
| --- | ---: | ---: | ---: |
| other | 491496 | 491196 | 345.0 |
| app js | 217229 | 216929 | 4.0 |
| worker script | 2314 | 39995 | -59.0 |
| app css | 31481 | 31181 | 2.0 |

## Browser Memory

| Scenario | JS heap used | JS heap total | DOM nodes | Windows working set |
| --- | ---: | ---: | ---: | ---: |
| tracked-luna-cold | 4996936 | 10330112 | 692 | 0 |
| tracked-luna-warm-reload | 7324948 | 15048704 | 1103 | 0 |
| tracked-luna-warm-new-page | 5259920 | 9805824 | 840 | 0 |
| tracked-jyut-cold | 5192200 | 9019392 | 839 | 0 |
| tracked-jyut-warm-reload | 4960484 | 9281536 | 973 | 0 |
| tracked-jyut-warm-new-page | 5594864 | 9281536 | 997 | 0 |
| tracked-mock-cold | 4982916 | 8757248 | 609 | 0 |
| tracked-mock-warm | 6956796 | 12427264 | 1713 | 0 |
| public-luna-cold | 4528356 | 10592256 | 840 | 0 |
| public-jyut-cold | 5484392 | 10854400 | 977 | 0 |
