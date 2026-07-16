# M61 Candidate-Parity Requirement And Oracle Review

**Verdict:** APPROVE — no findings.

**Preserved pre-review candidate tree:**
`70b8d6a81795c0e4349bbc5e9c7d29d4fd6e06bc`.

**Base:** commit `1c405855427033464f8f720eae31e6d2d6c34fc3`, tree
`b879a6b0c342f58925717cc488058d7cb31ab682`.

The isolated index reconstructed the preserved tree before and after review.
Its delta is exactly the declared thirteen pre-review paths, every working blob
matched, the real index remained empty, and no source was edited during review.

The correction follows D-24 and D-25. Its acceptance fixture is copied only
from the pinned `rime/librime 1.17.0` half of the external Windows capture. The
review independently verified source CSV SHA-256
`ba3674dab7d662fdc3b184b1757da8dfc559a5163d0579457a5929c106ba0356`,
fixture SHA-256
`68143faa4a620a2d3b0c1b99a3f96b854bc0f9925ddaebf25b034dd733932196`,
the exact manifest row, and equality of all five librime rows across the nine
required fields. The preserved Yune half is red, so it is not acceptance data.
Oracle DLL/shared/build, schema source/product tree, capture source, and row
policy are hard-pinned by the deployed test.

The only production edit is the structural post-prune filter in
`upstream_script_surface_segmentation`. It prevents restoration of a stale raw
identity choice while preserving the live reverse-good graph and transformed
inverse overlaps. No weighting, ranking, selection, recomposition, schema
installation, reachability ownership, ABI/API/export, profile/schema ID,
Track B, UI/browser/WASM, memory owner, storage default, threshold, or
performance policy changes.

The comparator enforces the frozen ordered seventeen inputs, two engines, five
page-zero rows per input, canonical geometry, and exact equality of
`candidate_index`, `candidate_count`, `page_size`, `page_no`, `is_last_page`,
`highlighted_index`, `composition_preedit`, `text`, and `comment`. It emits
deterministic source/oracle-bound outputs, distinguishes behavior mismatch from
malformed/setup failure, and is independently validated and replayed by the
aggregator. The later clean pushed `17/17` native preflight remains mandatory;
this review does not preclaim it.

Independent green checks:

- candidate-parity, M61 supplemental, and M59 evidence tooling: 103 tests;
- upstream Luna parity and all deployed `m59_luna_` guards;
- schema-general default-on/explicit-false deployment matrix: 1/1;
- plain-Jyutping default-on/explicit-false deployment: 1/1;
- deployed M61 pinned-Luna fixture: 1/1;
- prefix-fallback without an owned prefix: owning test 1/1;
- PowerShell parse, macOS wrapper `bash -n`, external nine-field comparison,
  and `git diff --check`.

The first combined review harness timed out while the schema-general child
remained CPU-active and produced no behavior verdict. Only that owning slice
was rerun as `review-harness-timeout-retry`; it passed 1/1 in 573.53 seconds.
No measured red occurred.
