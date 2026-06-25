# M37 rsmarisa Path

Dependency check:

- Crate: `rsmarisa 0.4.2`
- License: BSD-2-Clause
- Rust version: 1.70
- Default feature: `mmap`
- APIs verified locally: `Trie::mmap`, `Trie::map`, `Trie::reverse_lookup`, `Trie::predictive_search`, `Trie::num_tries`, `Trie::num_keys`

Primary docs:

- <https://docs.rs/rsmarisa/0.4.2/rsmarisa/>
- <https://crates.io/crates/rsmarisa/0.4.2>

## Real Product Probe

Final native evidence:

- `phase-3-final-native/track-b-yune-product/product_path_status.csv`
- `phase-3-final-native/track-b-yune-product/rsmarisa-jyut6ping3-string-table.marisa`
- `phase-3-final-native/track-b-yune-product/rsmarisa-jyut6ping3_scolar-string-table.marisa`

| Dictionary | Probe payload | Status | Mapping mode | Tries | Keys | Sample key |
| --- | ---: | --- | --- | ---: | ---: | --- |
| `jyut6ping3` | 488,504 B | ok | mmap | 3 | 127,764 | `yun1` |
| `jyut6ping3_scolar` | 3,090,640 B | ok | mmap | 3 | 249,334 | `yun1,1,0,,,,,,,,,=煴,,,,` |

The real TypeDuck marisa string-table payloads can be extracted and mmaped by `rsmarisa` on native Windows. That satisfies the M37 requirement to try `rsmarisa` on real `jyut6ping3` and `jyut6ping3_scolar` data.

## Route Decision

`rsmarisa` was not selected as the final hot storage route for M37 because it only covers the marisa string table payload. The real TypeDuck `.table.bin` also uses a multi-level phrase index outside the currently implemented Yune query-table path. Selecting `rsmarisa` alone would still require a separate phrase-index adapter before it could satisfy byte-identical product lookup, prediction, correction, rich-comment, and paging behavior.

M37 therefore selected the reviewed byte-backed Yune product table route: deploy fresh Yune-readable product tables, map those deployed bytes on native, and store compact row offsets into the mapped bytes rather than rebuilding the M36 owned no-marisa heap mirror. The `rsmarisa` evidence remains checked in as the route probe and future adapter input.
