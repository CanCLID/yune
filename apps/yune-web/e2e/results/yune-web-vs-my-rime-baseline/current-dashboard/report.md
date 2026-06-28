# Yune Web Comparator Benchmark

## Comparison Read

Only `luna_pinyin` rows are fair cross-engine comparisons. Jyutping rows are
guard evidence: My RIME uses the Cantonese-only `@rime-contrib/cantonese`
package, while Yune runs TypeDuck's multilingual `jyut6ping3_mobile` profile.

| Scenario | Schema | Lane | Samples | Ready ms | Input ms | Commit ms | WASM ready | WASM peak | Unique encoded resources | Commit |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| yune-tracked | luna_pinyin | fair comparison | 3 | 971 | 67 | 118 | 64.0 MiB | 64.0 MiB | 29.4 MiB | `你` |
| yune-tracked | jyutping | guard | 3 | 1279 | 89 | 107 | 160.0 MiB | 160.0 MiB | 72.2 MiB | `你` |
| yune-public-demo | luna_pinyin | fair comparison | 3 | 1000 | 74 | 107 | 64.0 MiB | 64.0 MiB | 29.5 MiB | `你` |
| yune-public-demo | jyutping | guard | 3 | 1347 | 103 | 108 | 160.0 MiB | 160.0 MiB | 72.2 MiB | `你` |
| my-rime-live | luna_pinyin | fair comparison | 3 | 634 | 95 | 119 | 16.0 MiB | 16.0 MiB | 8.5 MiB | `你` |
| my-rime-live | jyutping | guard | 3 | 998 | 99 | 114 | 56.6 MiB | 68.0 MiB | 24.9 MiB | `你` |

## Top Resources

### yune-tracked luna_pinyin

| Resource | Context | Encoded | Transfer |
| --- | --- | ---: | ---: |
| http://127.0.0.1:52033/schema/essay.txt | worker | 5.6 MiB | 5.6 MiB |
| http://127.0.0.1:52033/schema/luna_pinyin.table.bin | worker | 4.4 MiB | 4.4 MiB |
| http://127.0.0.1:52033/schema/stroke.table.bin | worker | 4.3 MiB | 4.3 MiB |
| http://127.0.0.1:52033/schema/stroke.prism.bin | worker | 3.3 MiB | 3.3 MiB |
| http://127.0.0.1:52033/schema/stroke.dict.yaml | worker | 3.2 MiB | 3.2 MiB |
| http://127.0.0.1:52033/yune-web.wasm | worker | 2.5 MiB | 2.5 MiB |
| http://127.0.0.1:52033/schema/luna_pinyin.prism.bin | worker | 1.6 MiB | 1.6 MiB |
| http://127.0.0.1:52033/schema/stroke.reverse.bin | worker | 1.4 MiB | 1.4 MiB |

### yune-tracked jyutping

| Resource | Context | Encoded | Transfer |
| --- | --- | ---: | ---: |
| http://127.0.0.1:52033/schema/jyut6ping3_mobile.prism.bin | worker | 18.4 MiB | 18.4 MiB |
| http://127.0.0.1:52033/schema/jyut6ping3.table.bin | worker | 14.5 MiB | 14.5 MiB |
| http://127.0.0.1:52033/schema/jyut6ping3_scolar.table.bin | worker | 11.9 MiB | 11.9 MiB |
| http://127.0.0.1:52033/schema/jyut6ping3_scolar.dict.yaml | worker | 6.8 MiB | 6.8 MiB |
| http://127.0.0.1:52033/schema/luna_pinyin_yune_reverse.table.bin | worker | 4.4 MiB | 4.4 MiB |
| http://127.0.0.1:52033/schema/jyut6ping3.reverse.bin | worker | 3.4 MiB | 3.4 MiB |
| http://127.0.0.1:52033/schema/jyut6ping3.dict.yaml | worker | 3.3 MiB | 3.3 MiB |
| http://127.0.0.1:52033/yune-web.wasm | worker | 2.5 MiB | 2.5 MiB |

### yune-public-demo luna_pinyin

| Resource | Context | Encoded | Transfer |
| --- | --- | ---: | ---: |
| http://127.0.0.1:52034/schema/essay.txt | worker | 5.6 MiB | 5.6 MiB |
| http://127.0.0.1:52034/schema/luna_pinyin.table.bin | worker | 4.4 MiB | 4.4 MiB |
| http://127.0.0.1:52034/schema/stroke.table.bin | worker | 4.3 MiB | 4.3 MiB |
| http://127.0.0.1:52034/schema/stroke.prism.bin | worker | 3.3 MiB | 3.3 MiB |
| http://127.0.0.1:52034/schema/stroke.dict.yaml | worker | 3.2 MiB | 3.2 MiB |
| http://127.0.0.1:52034/yune-web.wasm | worker | 2.5 MiB | 2.5 MiB |
| http://127.0.0.1:52034/schema/luna_pinyin.prism.bin | worker | 1.6 MiB | 1.6 MiB |
| http://127.0.0.1:52034/schema/stroke.reverse.bin | worker | 1.4 MiB | 1.4 MiB |

### yune-public-demo jyutping

| Resource | Context | Encoded | Transfer |
| --- | --- | ---: | ---: |
| http://127.0.0.1:52034/schema/jyut6ping3_mobile.prism.bin | worker | 18.4 MiB | 18.4 MiB |
| http://127.0.0.1:52034/schema/jyut6ping3.table.bin | worker | 14.5 MiB | 14.5 MiB |
| http://127.0.0.1:52034/schema/jyut6ping3_scolar.table.bin | worker | 11.9 MiB | 11.9 MiB |
| http://127.0.0.1:52034/schema/jyut6ping3_scolar.dict.yaml | worker | 6.8 MiB | 6.8 MiB |
| http://127.0.0.1:52034/schema/luna_pinyin_yune_reverse.table.bin | worker | 4.4 MiB | 4.4 MiB |
| http://127.0.0.1:52034/schema/jyut6ping3.reverse.bin | worker | 3.4 MiB | 3.4 MiB |
| http://127.0.0.1:52034/schema/jyut6ping3.dict.yaml | worker | 3.3 MiB | 3.3 MiB |
| http://127.0.0.1:52034/yune-web.wasm | worker | 2.5 MiB | 2.5 MiB |

### my-rime-live luna_pinyin

| Resource | Context | Encoded | Transfer |
| --- | --- | ---: | ---: |
| https://cdn.jsdelivr.net/npm/@rime-contrib/luna-pinyin@0.1.1/luna_pinyin.table.bin | worker | 4.0 MiB | 4.0 MiB |
| https://cdn.jsdelivr.net/npm/@rime-contrib/stroke@0.1.3/stroke.prism.bin | worker | 1.1 MiB | 1.1 MiB |
| https://cdn.jsdelivr.net/npm/@rime-contrib/stroke@0.1.3/stroke.table.bin | worker | 975.9 KiB | 976.2 KiB |
| https://cdn.jsdelivr.net/npm/@libreservice/my-rime@0.10.9/dist/rime.wasm | worker | 802.9 KiB | 803.2 KiB |
| https://cdn.jsdelivr.net/npm/@libreservice/my-rime@0.10.9/dist/rime.data | worker | 585.1 KiB | 585.4 KiB |
| https://cdn.jsdelivr.net/npm/@rime-contrib/stroke@0.1.3/stroke.reverse.bin | worker | 548.2 KiB | 548.5 KiB |
| https://my-rime.vercel.app/assets/index-Dx7zgIB7.js | page | 403.2 KiB | 403.5 KiB |
| https://cdn.jsdelivr.net/npm/@rime-contrib/luna-pinyin@0.1.1/luna_pinyin.reverse.bin | worker | 86.8 KiB | 87.1 KiB |

### my-rime-live jyutping

| Resource | Context | Encoded | Transfer |
| --- | --- | ---: | ---: |
| https://cdn.jsdelivr.net/npm/@rime-contrib/cantonese@0.1.5/jyut6ping3.table.bin | worker | 9.9 MiB | 9.9 MiB |
| https://cdn.jsdelivr.net/npm/@rime-contrib/luna-pinyin@0.1.1/luna_pinyin.table.bin | worker | 4.0 MiB | 4.0 MiB |
| https://cdn.jsdelivr.net/npm/@rime-contrib/cangjie@0.1.3/cangjie5.table.bin | worker | 3.3 MiB | 3.3 MiB |
| https://cdn.jsdelivr.net/npm/@rime-contrib/cangjie@0.1.3/cangjie5.reverse.bin | worker | 1.6 MiB | 1.6 MiB |
| https://cdn.jsdelivr.net/npm/@rime-contrib/cangjie@0.1.3/cangjie5.prism.bin | worker | 1.5 MiB | 1.5 MiB |
| https://cdn.jsdelivr.net/npm/@rime-contrib/stroke@0.1.3/stroke.prism.bin | worker | 1.1 MiB | 1.1 MiB |
| https://cdn.jsdelivr.net/npm/@rime-contrib/stroke@0.1.3/stroke.table.bin | worker | 975.9 KiB | 976.2 KiB |
| https://cdn.jsdelivr.net/npm/@libreservice/my-rime@0.10.9/dist/rime.wasm | worker | 802.9 KiB | 803.2 KiB |
