# rsmarisa Gate

M36 investigated the real shipped product blobs before choosing the fallback
route.

Observed blockers on the actual `third_party/typeduck-web/source/public/schema`
assets:

- `jyut6ping3.table.bin`: stale checksum versus source and unsupported marisa
  string table.
- `jyut6ping3_scolar.table.bin`: stale checksum versus source and unsupported
  marisa string table.
- `jyut6ping3.reverse.bin` and `jyut6ping3_scolar.reverse.bin`: unsupported
  marisa reverse key/value trie sections.
- `jyut6ping3_mobile.prism.bin` and `jyut6ping3_scolar.prism.bin`:
  `Rime::Prism/3.0`, while Yune's compact prism lookup is tied to the parsed
  table syllabary. Mixing the shipped v3 prism with Yune-native re-emitted
  table syllabary is not byte-safe.

Decision:

- `rsmarisa` is closed as a measured no-go for M36 closeout.
- The landed route is Yune-native no-marisa re-emission through deployment,
  followed by compact table+prism storage when the compiled product artifacts
  are active.
- Native mmap and WASM byte-backed marisa loading remain future work because
  the product route first needed compiled-ready correctness and Track B
  measurement.
