# M61 Provenance

## Binding identities

- correction source: `91f5969688a3d2dba96a67d1cfe813c7ba4ee861`;
- correction tree: `6626ed16d5e135fa477ca26e9786d11121c92b44`;
- correction parent / pushed quality repair:
  `931c7c59d6d471c69b70dc0d2f082149665a4e68`;
- quality-repair tree: `f1c36a0079d85628f5cbef140bd94288930cc2e8`;
- explicit revert: `01a62f2a6cd2b3d668545a110de8c7c3fc2fbb10`;
- revert tree: `f1c36a0079d85628f5cbef140bd94288930cc2e8`;
- core-test-only closeout correction tree:
  `bf4ef0b8d7d234b248cc61e9a1c5ad6b57ee61af`;
- combined two-test closeout correction tree:
  `6cb28424f7bcf5a535ac6173b651e9ba1b7bd160`;
- corrected Cantonese test blob:
  `a8bc25e21c80107caafcd19525c470f3f991378d`;
- corrected lifecycle test blob:
  `5a805ad7c41858fca92ef75ac4a604087e195936`;
- pinned librime source: `33e78140250125871856cdc5b42ddc6a5fcd3cd4`;
- pinned oracle DLL SHA-256:
  `86b4c7357d4c6d293ce5589b234d8859ca2ac30923a03bedfa3926eeaf97fb0b`;
- oracle shared-tree SHA-256:
  `3801c4c83ba919e531b80ac27e2c06d116d08b19af2034fcb86e6e17ae1eecf6`;
- oracle build-tree SHA-256:
  `446c90b2f4ffd76b4ec1f4ecca4f534c986e72e3d8803c6998926d0b1cebbf17`;
- product-schema-tree SHA-256:
  `0bc042c0ab09c732419cf6ba5ce008390e87894c7d374c0d1b44efeac10a9bf0`;
- M55 signed registry SHA-256:
  `e74e77b4dd5b253e0c2b5f4b12cc1e0279784d3c3fbf02006b5f8f18fccacdba`;
- externally frozen, unaccepted M61 supplemental threshold SHA-256:
  `d52d064f410df36c1c22dd5523430062563a17bb9f2f63253b607d211badefd7`.

All eleven correction-source rounds used Yune DLL SHA-256
`787e3017e0e68e79296894b73c33ad399f297a3195d1251a11bc0fbf8af88152`,
benchmark executable SHA-256
`351ebd30c88634b1b646c4c6e73f0d14cbed8c38ebc75a15fb29c124b3661782`,
and benchmark build-receipt SHA-256
`70036afa4a5516efe7d391fcdae8241b594b55900469ee1776d3dce9986d479e`.
The Luna table source/table checksums were `0xb3d4e98e` / `0x29d56c89`;
the rebound POET artifact was `28,616,073 B` with SHA-256
`45f5c91d0f3a0ddd953a954a33687377fb8f5565fe12ac306fb41952eca39144`.

## Raw retention

Full raw evidence remains external at source-keyed roots:

- `C:\m61s\7805882d\candidate-parity-preflight`;
- `C:\m61s\a39c4d86\candidate-parity-preflight`;
- `C:\yune-m61\a39c4d868820063dc3deaa42f7fdc9b3aee5e7a6`;
- `C:\yune-m61\f18b0df2d0149bc2a28cd9bd2c075c34030b5568`;
- `C:\yune-m61\67d32a2bea36a391a8a11ea4e725dbfebe118252`;
- `C:\yune-m61\91f5969688a3d2dba96a67d1cfe813c7ba4ee861`;
- `C:\m61s\91f59696`;
- `C:\m61s\01a62f2a`;
- `C:\m61s\01a62f2a\final-gates-20260716-retry-1-runner-capture`;
- `C:\m61s\01a62f2a\workspace-gate20-retry-1-bounded-refresh-test-repair`;
- `C:\m61s\01a62f2a\workspace-gate20-retry-2-test-contract-and-product-page`;
- `C:\m61s\01a62f2a\workspace-gate20-retry-3-never-reached-tail`;
- `C:\m61s\01a62f2a\workspace-gate20-retry-4-api-lib-contract-and-unreached-tail`.

The correction-source raw non-work inventory contains 660 files totaling
`59,804,398 B`; its manifest SHA-256 is
`94ee905ba4a42c3664262847c32ffc8588e97686674236d55a5f4d38d8187c96`.
The tracked `evidence-integrity.txt` is a UTF-8 normalization of the preserved
external UTF-16 setup receipt whose SHA-256 is
`4b597c7d767527c1abedfaeb828357023dee2032960ffc5faac2ddea92e72716`;
its text is unchanged, and the normalization makes the curated packet readable
by the mandatory public-evidence privacy checker.
Raw work trees and private environment captures are intentionally excluded from
the tracked packet.

The first isolated pre-review candidate tree
`38e7e33cbe1458e6c7de1bf70b7ec30ee8414ca0` was discarded after tree-mode
manifest verification detected LF normalization of imported Windows receipts.
The final candidate includes a narrow `.gitattributes` `-text` rule for this
packet subtree, preserving the exact curated bytes without changing runtime or
product paths.
