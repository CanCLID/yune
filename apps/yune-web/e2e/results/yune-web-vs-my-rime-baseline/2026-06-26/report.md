# Yune Web vs My RIME Browser Baseline

Generated: 2026-06-26T21:47:28.953Z
Samples per scenario/schema: 3

## Summary

| scenario | schema | input | ready ms | input->candidate ms | commit ms | WASM ready MiB | measured peak MiB | transfer MiB | unique encoded MiB | JS heap MiB | commit |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| my-rime-live | jyutping | nei | 894 | 30 | 19 | 56.6 | 68.0 | 24.9 | 24.9 | 11.3 | 你 |
| yune-public-demo | jyutping | nei | 1164 | 30 | 20 | 128.0 | 128.0 | 33.8 | 33.5 | 9.5 | 你 |
| yune-tracked | jyutping | nei | 1172 | 33 | 24 | 128.0 | 128.0 | 33.8 | 33.5 | 9.5 | 你 |
| my-rime-live | luna_pinyin | ni | 547 | 30 | 17 | 16.0 | 16.0 | 8.5 | 8.5 | 11.3 | 你 |
| yune-public-demo | luna_pinyin | ni | 764 | 30 | 24 | 128.0 | 128.0 | 5.6 | 5.4 | 9.5 | 伱 |
| yune-tracked | luna_pinyin | ni | 775 | 30 | 22 | 128.0 | 128.0 | 5.6 | 5.4 | 9.5 | 伱 |

## Notes

- My RIME WASM heap is read directly inside its live dedicated worker via `Module.HEAPU8.byteLength`.
- Yune WASM heap is read from the harness diagnostics/UI instrumentation added in this branch.
- Input latency rows are external Playwright timings for typing the full schema input to any visible candidate, then pressing Space until the textarea contains committed text. Yune internal keydown-to-paint metrics remain available in `samples.json`.
- Resource rows include page and dedicated-worker resource timing entries; unique encoded bytes deduplicate URLs by path.

## Top Resources By First Sample

### my-rime-live / jyutping
- 9.86 MiB encoded: https://cdn.jsdelivr.net/npm/@rime-contrib/cantonese@0.1.5/jyut6ping3.table.bin
- 4.03 MiB encoded: https://cdn.jsdelivr.net/npm/@rime-contrib/luna-pinyin@0.1.1/luna_pinyin.table.bin
- 4.03 MiB encoded: https://cdn.jsdelivr.net/npm/@rime-contrib/luna-pinyin@0.1.1/luna_pinyin.table.bin
- 3.27 MiB encoded: https://cdn.jsdelivr.net/npm/@rime-contrib/cangjie@0.1.3/cangjie5.table.bin
- 1.61 MiB encoded: https://cdn.jsdelivr.net/npm/@rime-contrib/cangjie@0.1.3/cangjie5.reverse.bin
- 1.49 MiB encoded: https://cdn.jsdelivr.net/npm/@rime-contrib/cangjie@0.1.3/cangjie5.prism.bin

### yune-public-demo / jyutping
- 6.80 MiB encoded: http://127.0.0.1:57470/schema/jyut6ping3_scolar.dict.yaml?sha256=9e394642c099303972d4823aeb161a9ad089a881699a58712192e9890d3a7b09
- 5.83 MiB encoded: http://127.0.0.1:57470/schema/jyut6ping3_scolar.table.bin?sha256=9e09a610ce5bcb59c8784d48ab5698172d72d74ad7a2b5fec34f95b6d4bdfeba
- 4.11 MiB encoded: http://127.0.0.1:57470/schema/jyut6ping3.table.bin?sha256=5f686227e2ec3461c8b2bd6d6a19903a851f3f2354260e5da4ddb5f3d1e2177a
- 3.40 MiB encoded: http://127.0.0.1:57470/schema/jyut6ping3_scolar.reverse.bin?sha256=3191705dc4615274a1ff7dde3acb87828935d60980ef4788558af189e4f75174
- 3.31 MiB encoded: http://127.0.0.1:57470/schema/jyut6ping3.dict.yaml?sha256=b0abf4fbcbf18b8cf05f4689ea05a12be6cf301f29a7316699e2b1ab9c24d172
- 2.46 MiB encoded: http://127.0.0.1:57470/yune-web.wasm?v=yune-web-wasm-heap-v1

### yune-tracked / jyutping
- 6.80 MiB encoded: http://127.0.0.1:57469/schema/jyut6ping3_scolar.dict.yaml?sha256=9e394642c099303972d4823aeb161a9ad089a881699a58712192e9890d3a7b09
- 5.83 MiB encoded: http://127.0.0.1:57469/schema/jyut6ping3_scolar.table.bin?sha256=9e09a610ce5bcb59c8784d48ab5698172d72d74ad7a2b5fec34f95b6d4bdfeba
- 4.11 MiB encoded: http://127.0.0.1:57469/schema/jyut6ping3.table.bin?sha256=5f686227e2ec3461c8b2bd6d6a19903a851f3f2354260e5da4ddb5f3d1e2177a
- 3.40 MiB encoded: http://127.0.0.1:57469/schema/jyut6ping3_scolar.reverse.bin?sha256=3191705dc4615274a1ff7dde3acb87828935d60980ef4788558af189e4f75174
- 3.31 MiB encoded: http://127.0.0.1:57469/schema/jyut6ping3.dict.yaml?sha256=b0abf4fbcbf18b8cf05f4689ea05a12be6cf301f29a7316699e2b1ab9c24d172
- 2.46 MiB encoded: http://127.0.0.1:57469/yune-web.wasm?v=yune-web-wasm-heap-v1

### my-rime-live / luna_pinyin
- 4.03 MiB encoded: https://cdn.jsdelivr.net/npm/@rime-contrib/luna-pinyin@0.1.1/luna_pinyin.table.bin
- 1.12 MiB encoded: https://cdn.jsdelivr.net/npm/@rime-contrib/stroke@0.1.3/stroke.prism.bin
- 0.95 MiB encoded: https://cdn.jsdelivr.net/npm/@rime-contrib/stroke@0.1.3/stroke.table.bin
- 0.78 MiB encoded: https://cdn.jsdelivr.net/npm/@libreservice/my-rime@0.10.9/dist/rime.wasm
- 0.57 MiB encoded: https://cdn.jsdelivr.net/npm/@libreservice/my-rime@0.10.9/dist/rime.data
- 0.54 MiB encoded: https://cdn.jsdelivr.net/npm/@rime-contrib/stroke@0.1.3/stroke.reverse.bin

### yune-public-demo / luna_pinyin
- 2.46 MiB encoded: http://127.0.0.1:57470/yune-web.wasm?v=yune-web-wasm-heap-v1
- 0.67 MiB encoded: http://127.0.0.1:57470/schema/luna_pinyin.reverse.bin?sha256=b41098f7b24f7f936e1e28e2fc135cfd982e4c056cc4f3ef61c617b508485f13
- 0.59 MiB encoded: http://127.0.0.1:57470/schema/luna_pinyin.table.bin?sha256=26601971bd7845c8e84eb8f8440b14633c815578fe0560c6b81fa62b69af278d
- 0.47 MiB encoded: https://fonts.googleapis.com/css2?family=Bpmf+Huninn&family=Chiron+GoRound+TC:wght@400;700&family=Chiron+Hei+HK:wght@400;700&family=Chiron+Sung+HK:wght@400;700&family=Chocolate+Classical+Sans&family=Huninn&family=Iansui&family=LXGW+WenKai+Mono+TC&family=LXGW+WenKai+TC&family=WDXL+Lubrifont+TC&display=swap
- 0.40 MiB encoded: http://127.0.0.1:57470/schema/luna_pinyin.dict.yaml?sha256=dee37dbc2cfe8c04b23f428b9afa23607be2d22a8daf8446f14f8882c801459f
- 0.39 MiB encoded: http://127.0.0.1:57470/schema/cangjie5.dict.yaml?sha256=d7edeff4e80d262e75a0f6988d94a1eed489f5d8ebdfc4c089b0da819429fbba

### yune-tracked / luna_pinyin
- 2.46 MiB encoded: http://127.0.0.1:57469/yune-web.wasm?v=yune-web-wasm-heap-v1
- 0.67 MiB encoded: http://127.0.0.1:57469/schema/luna_pinyin.reverse.bin?sha256=b41098f7b24f7f936e1e28e2fc135cfd982e4c056cc4f3ef61c617b508485f13
- 0.59 MiB encoded: http://127.0.0.1:57469/schema/luna_pinyin.table.bin?sha256=26601971bd7845c8e84eb8f8440b14633c815578fe0560c6b81fa62b69af278d
- 0.47 MiB encoded: https://fonts.googleapis.com/css2?family=Bpmf+Huninn&family=Chiron+GoRound+TC:wght@400;700&family=Chiron+Hei+HK:wght@400;700&family=Chiron+Sung+HK:wght@400;700&family=Chocolate+Classical+Sans&family=Huninn&family=Iansui&family=LXGW+WenKai+Mono+TC&family=LXGW+WenKai+TC&family=WDXL+Lubrifont+TC&display=swap
- 0.40 MiB encoded: http://127.0.0.1:57469/schema/luna_pinyin.dict.yaml?sha256=dee37dbc2cfe8c04b23f428b9afa23607be2d22a8daf8446f14f8882c801459f
- 0.39 MiB encoded: http://127.0.0.1:57469/schema/cangjie5.dict.yaml?sha256=d7edeff4e80d262e75a0f6988d94a1eed489f5d8ebdfc4c089b0da819429fbba
