# M59 Increment 4c OpenCC variants packet

Status: Increment 4c is implemented at clean production source
`e11557e2bbb05e3598e2d96dd6eb669ded88d33d`. Its complete external Lane A
capture and five-round signed performance guard are green. This closes the
Lane A D-48 candidate text/order requirement; it does not close Lane B,
Cangjie, M59-REACH-03/04, M59-GATES-01, or M59. Increment 4d Cangjie CJ-1 is
next. The designated review for 4c is nonblocking under the locked plan.

## Mechanism

The OpenCC filter now follows pinned librime `ConvertWord` behavior on the
named Hong Kong conversion chain:

- an exact whole-word mapping emits every declared output in source order;
- each stage deduplicates stably;
- when no exact whole-word mapping exists, maximum-prefix recomposition uses
  only the default output from each partial mapping; and
- unrelated candidate duplicates remain untouched, while duplicates created
  by one exact expansion family collapse stably and retain the longest owned
  span.

One-to-many activation is derived from the configured
`TraditionalToHongKong` / `t2hkf` chain, never from a schema id or input
allowlist. Other output standards keep their established default-only behavior
until a named oracle requires more. No public ABI, `Rime::Table/4.0` version,
compiled data format, schema id, or product asset changes.

The pinned external fixture covers ordered exact conversion (`祕 -> 秘,祕`,
`糉 -> 粽,糉,糭`, and `只 -> 只,衹`), default-only partial recomposition
(`祕糉 -> 秘粽`), and unchanged input (`甲乙`). Source, owned, byte-backed,
compiled/deployed, and TypeDuck profile paths have owning tests. The existing
complete same-code inventory contains 83 rows and exactly reconciles 64 of the
65 `HKVariantsFull.txt` keys; the one remaining key has no same-code sibling.

## Lane A behavior verdict

The refreshed oracle capture under [`oracle-refresh/`](./oracle-refresh/) binds
to librime `33e78140250125871856cdc5b42ddc6a5fcd3cd4`, official `rime.dll`
SHA-256 `86b4c7357d4c6d293ce5589b234d8859ca2ac30923a03bedfa3926eeaf97fb0b`,
and `rime/rime-cantonese`
`c99b16e44d2df77a5cb8fb0867dd2bab7a112cb0`. It is exact against the prior
pinned 13-input oracle for candidate text/order.

The Yune capture under [`behavior/`](./behavior/) binds to clean source tree
`48bf8dbad8811e8c23e90af683a4c2bbcd9831cc`, release DLL SHA-256
`5a95705fa33e74dc7954d356339621c1462ddd9198eccf51dbb9b9abbd0f5eb7`,
and complete all-page capture SHA-256
`28baf32eeade77b54b0c8208c32b7f6a072ad4ccf66454cf61a38b351280d099`.

The strict comparator is green for all 13 inputs and all 5,705 candidates:
candidate text, order, global/page position, page shape, termination, preedit,
and commit preview are exact. The fail-closed 4c classifier proves 14 visible
occurrences from five applicable inventory mappings, with zero mapping, order,
position, or OpenCC residuals. It accepts no exception or tail policy and uses
no beyond-oracle-depth row. The historical owner-signed 4a class-3 exception
remains recorded but is unused by this final capture.

Lane A is exact for the complete D-48 candidate text/order/position contract.
Candidate comments retain 854 differences across 9 of 13 inputs outside that
contract; no canonical Lane A comment parity or whole-capture byte identity is
claimed. The classifier records the ordered mismatch-tuple SHA-256
`c68be6b33cd519962b09037e7bbca144ebce9b80aa00510551fc6ecc704793b1`
as explicitly non-gating. The separate TypeDuck comment/profile contract is
unchanged and its full Cantonese regression suite remains 41/41.

## Performance verdict

[`performance-ratchet/`](./performance-ratchet/) contains five fresh complete
runs from the same clean source and release DLL over the full 17-input Track A
set plus the Track B product row. Product deployment is enabled and iterations
remain `9/60/80`. Run 1 builds one source-bound native harness; runs 2-5 reuse
the exact executable and receipt, for mode sequence
`build,reuse,reuse,reuse,reuse`.

All 32 aggregate median rows pass their unchanged signed ceilings across 160
preserved observations. Two individual observations are red and remain visible:
one Track A session-latency sample and one Track B session private-memory
sample. Their required five-run medians pass; no adaptive rerun, re-baseline,
threshold change, or observation deletion occurred.

The guarded medians include `n 0.200x <= 3.006x`, `ni 0.241x <= 2.666x`,
`hao 0.278x <= 1.844x`, the 37-character row `0.018x <= 2.339x`, and the
59-character row `0.008x <= 1.748x`. This is an Increment 4c guard, not the
final M59-REACH-04 measurement after 4d and 4e.

## Packet map and boundary

- `behavior/lane-a-13-yune.json` is the complete production capture.
- `behavior/lane-a-13-exact.{json,csv}` is the untouched strict comparator.
- `behavior/lane-a-13-4c-classification.json` binds the complete OpenCC
  inventory, source mapping, external fixture, manifest, capture, and strict
  comparator.
- `oracle-refresh/` preserves the fresh canonical capture and exact comparison
  with the previously pinned oracle.
- `performance-ratchet/` preserves every raw text receipt from all five rounds,
  the executable aggregate verdict, and its provenance.
- `setup-attempts.md` records the two pre-measurement fail-closed setup stops.
- `verification.md` records focused gates, hashes, reviews, and publication
  integrity.
- `packet-manifest.json` inventories every packet-local file except itself.

No DLL, benchmark executable, compiled table, deployed tree, or other binary
payload is copied into this evidence packet. Remaining M59 behavior work is 4d
Cangjie CJ-1 and 4e Lane B exact order/WEB-04 restoration, followed by final
REACH-03/04 reconciliation and the exact native/WASM/browser/package gates.
