# Yune Web Comparator Benchmark

| Scenario | Schema | Samples | Ready ms | Input ms | Commit ms | WASM ready | WASM peak | Unique encoded resources | Commit |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| yune-tracked | luna_pinyin | 3 | 930 | 69 | 116 | 128.0 MiB | 128.0 MiB | 5.4 MiB | `伱` |
| yune-tracked | jyutping | 3 | 6574 | 105 | 116 | 893.1 MiB | 893.1 MiB | 33.5 MiB | `你` |
| yune-public-demo | luna_pinyin | 3 | 932 | 68 | 112 | 128.0 MiB | 128.0 MiB | 5.5 MiB | `伱` |
| yune-public-demo | jyutping | 3 | 6621 | 119 | 120 | 893.1 MiB | 893.1 MiB | 33.5 MiB | `你` |
| my-rime-live | luna_pinyin | 3 | 655 | 98 | 116 | 16.0 MiB | 16.0 MiB | 8.5 MiB | `你` |
| my-rime-live | jyutping | 3 | 994 | 87 | 126 | 56.6 MiB | 68.0 MiB | 24.9 MiB | `你` |

## Top Resources

### yune-tracked luna_pinyin

| Resource | Context | Encoded | Transfer |
| --- | --- | ---: | ---: |
| http://127.0.0.1:51686/yune-web.wasm | worker | 2.5 MiB | 2.5 MiB |
| http://127.0.0.1:51686/schema/luna_pinyin.reverse.bin | worker | 687.8 KiB | 688.1 KiB |
| http://127.0.0.1:51686/schema/luna_pinyin.table.bin | worker | 599.5 KiB | 599.8 KiB |
| https://fonts.googleapis.com/css2 | page | 479.7 KiB | 480.0 KiB |
| http://127.0.0.1:51686/schema/luna_pinyin.dict.yaml | worker | 412.4 KiB | 412.7 KiB |
| http://127.0.0.1:51686/schema/cangjie5.dict.yaml | worker | 397.1 KiB | 397.4 KiB |
| http://127.0.0.1:51686/assets/index-635zXvQx.js | page | 215.4 KiB | 215.6 KiB |
| http://127.0.0.1:51686/yune-web.js | worker | 70.7 KiB | 71.0 KiB |

### yune-tracked jyutping

| Resource | Context | Encoded | Transfer |
| --- | --- | ---: | ---: |
| http://127.0.0.1:51686/schema/jyut6ping3_scolar.dict.yaml | worker | 6.8 MiB | 6.8 MiB |
| http://127.0.0.1:51686/schema/jyut6ping3_scolar.table.bin | worker | 5.8 MiB | 5.8 MiB |
| http://127.0.0.1:51686/schema/jyut6ping3.table.bin | worker | 4.1 MiB | 4.1 MiB |
| http://127.0.0.1:51686/schema/jyut6ping3_scolar.reverse.bin | worker | 3.4 MiB | 3.4 MiB |
| http://127.0.0.1:51686/schema/jyut6ping3.dict.yaml | worker | 3.3 MiB | 3.3 MiB |
| http://127.0.0.1:51686/yune-web.wasm | worker | 2.5 MiB | 2.5 MiB |
| http://127.0.0.1:51686/schema/jyut6ping3_scolar.prism.bin | worker | 2.2 MiB | 2.2 MiB |
| http://127.0.0.1:51686/schema/loengfan.dict.yaml | worker | 1.7 MiB | 1.7 MiB |

### yune-public-demo luna_pinyin

| Resource | Context | Encoded | Transfer |
| --- | --- | ---: | ---: |
| http://127.0.0.1:51687/yune-web.wasm | worker | 2.5 MiB | 2.5 MiB |
| http://127.0.0.1:51687/schema/luna_pinyin.reverse.bin | worker | 687.8 KiB | 688.1 KiB |
| http://127.0.0.1:51687/schema/luna_pinyin.table.bin | worker | 599.5 KiB | 599.8 KiB |
| https://fonts.googleapis.com/css2 | page | 479.7 KiB | 480.0 KiB |
| http://127.0.0.1:51687/schema/luna_pinyin.dict.yaml | worker | 412.4 KiB | 412.7 KiB |
| http://127.0.0.1:51687/schema/cangjie5.dict.yaml | worker | 397.1 KiB | 397.4 KiB |
| http://127.0.0.1:51687/assets/index-635zXvQx.js | page | 215.4 KiB | 215.6 KiB |
| http://127.0.0.1:51687/yune-web.js | worker | 70.7 KiB | 71.0 KiB |

### yune-public-demo jyutping

| Resource | Context | Encoded | Transfer |
| --- | --- | ---: | ---: |
| http://127.0.0.1:51687/schema/jyut6ping3_scolar.dict.yaml | worker | 6.8 MiB | 6.8 MiB |
| http://127.0.0.1:51687/schema/jyut6ping3_scolar.table.bin | worker | 5.8 MiB | 5.8 MiB |
| http://127.0.0.1:51687/schema/jyut6ping3.table.bin | worker | 4.1 MiB | 4.1 MiB |
| http://127.0.0.1:51687/schema/jyut6ping3_scolar.reverse.bin | worker | 3.4 MiB | 3.4 MiB |
| http://127.0.0.1:51687/schema/jyut6ping3.dict.yaml | worker | 3.3 MiB | 3.3 MiB |
| http://127.0.0.1:51687/yune-web.wasm | worker | 2.5 MiB | 2.5 MiB |
| http://127.0.0.1:51687/schema/jyut6ping3_scolar.prism.bin | worker | 2.2 MiB | 2.2 MiB |
| http://127.0.0.1:51687/schema/loengfan.dict.yaml | worker | 1.7 MiB | 1.7 MiB |

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
