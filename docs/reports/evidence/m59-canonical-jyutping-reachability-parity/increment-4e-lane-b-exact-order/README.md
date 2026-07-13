# M59 Increment 4e Lane B exact-order packet

Status: Increment 4e is implemented at clean production source
`5879405c7b0f76af4dca7382f00b3e0605386f2c`. All seven captured Lane B
inputs match pinned librime page-for-page and position-for-position on tracked
byte-backed Luna assets. The exact deployed 37/59-character page-shape gate is
green, and native WEB-04 plain/null plus Octagram top-candidate parity is `8/8`.

## Mechanism

Un-toned Luna now uses the same structure-driven `UpstreamScript` surface graph
as other script translators. The graph performs one common-prefix prism walk per
live vertex, rejects phrase-only flattened codes as syllable edges, follows the
pinned three-syllable table trunk and packed tail, and reuses bounded sentence
scratch without changing the complete-list prefix.

Translator-local order and cross-translator order are separate. Script rows keep
their page-stable positional quality while an aligned internal merge-quality
channel retains librime-normalized dictionary weight for producer-head election.
That channel is workspace-internal: no C ABI field or export changes. Equal-weight,
weighted/initial-quality, owned/byte-backed, and userdb remerge controls guard it.

The tracked Octagram schema restores the pinned `grammar:/hant` limits
`max_homophones: 7` and `max_homographs: 7`. The native runner fails closed on a
dirty or unexpected source commit, wrong reused CLI hash, in-repository scratch
roots, wrong selected schema, undeployed page size, and the pinned grammar-model
size or SHA-256.

## Behavior verdict

- Lane B: all captured pages for `moboyi`, `boyi`, `yi`, `zhonggao`,
  `zhongguo`, `gao`, and `guo` match candidate text, page number, page size,
  page-local index, global index, and final-page state. The gate asserts tracked
  byte-backed `luna_pinyin` storage before enumeration.
- Long inputs: the deployed 37- and 59-character product rows preserve the
  pinned one-best-sentence-then-phrase page shape and selection remainder.
- WEB-04: [`behavior/web04-native/web04-native-verdict.json`](./behavior/web04-native/web04-native-verdict.json)
  records `8/8`, source-built release mode, clean source binding, CLI SHA-256
  `d738aae187addcaeb53857082d9f3cabc9afacafe5cb180a62bd4775eec8f7d8`,
  and pinned model SHA-256
  `574c99d100f422766c433c601ed6efd642e881d69a30df9fffb6f1695be550e3`.

## Performance verdict

[`performance-ratchet/gate-verdict.csv`](./performance-ratchet/gate-verdict.csv)
contains five fresh complete rounds from the same clean source over all 17 Track
A inputs and the Track B product row. Product deployment is enabled and the
iterations remain `9/60/80`. Run 1 builds one source-bound benchmark executable;
runs 2-5 reuse its exact bytes and receipt.

All 32 aggregate median rows pass their unchanged signed ceilings across 160
observations with zero individual failures. Guarded medians include `n 0.208x <=
3.006x`, `ni 0.246x <= 2.666x`, `hao 0.284x <= 1.844x`, the 37-character row
`0.022x <= 2.339x`, and the 59-character row `0.010x <= 1.748x`.

## Packet boundary

- `behavior/web04-native/` contains the final verdict, result table, and eight
  raw CLI stdout/stderr pairs.
- `performance-ratchet/` contains all five untouched text-only benchmark runs,
  aggregate verdict, and aggregate provenance.
- `verification.md`, `commands-provenance.md`, and `reviews/` record the focused
  gates and both independent review passes.
- `packet-manifest.json` inventories every packet-local file except itself.

No DLL, CLI, benchmark executable, grammar model, compiled table, deployed tree,
or other binary payload is copied into this packet. Increment 4e closes the
remaining Lane B and WEB-04 behavior work; M59 still requires the final native,
WASM, browser, packaging, documentation, and archive closeout gates.
