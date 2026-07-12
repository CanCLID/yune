# M59 Increment 4b abbreviation/segmentation packet

Status: implementation landed through `d508e05b`; designated Opus blocking
review pending. Increment 4c and every later engine-behavior increment remain
blocked. This packet does not close Lane A, D-48, M59-REACH-04, or M59.

## Behavior verdict

The accepted external production-path capture is under
[`behavior/accepted-d508e05b/`](./behavior/accepted-d508e05b/) and binds to clean
source `d508e05b638fc21de7f8f8dfc45c82d33a8bbde8`, tree
`4cc2ca6e6ca2f43666bbff942cee54e1e95abd07`, and release DLL SHA-256
`d0ab4d8d1e653ab45f258b953c7b07b2b245130885615597e6eabf9ba087e189`.
It captures `n`, `nri`, `ngohaig`, `ngohaigo`, and `bein` through every page.

The authoritative raw comparator stays visibly red: `1/5` rows pass. A separate
fail-closed classifier applies only the already-declared Increment 4c same-code
OpenCC inventory and then reports:

- candidate text and position: `5/5`;
- preedit/segmentation: `5/5`;
- commit preview: `5/5`;
- unowned residual cases: `0`;
- exception used: `false`; and
- beyond-oracle-depth used: `false`.

Every raw residual is one of the declared 4c siblings `僞`, `臥`, or `鉤`.
The final bounded/lazy implementation is exact `5/5` against the pre-lazy
`e97811a5` accepted candidate/page signature. The pre-lazy capture and the
otherwise-green `ea8656c3` capture rejected by the literal Clippy gate remain
preserved as named history.

## Performance verdict

The owner selected the retained-ceiling, lazy/page-bounded disposition in
[`owner-disposition.md`](./owner-disposition.md). The accepted packet under
[`performance-ratchet/`](./performance-ratchet/) contains five complete runs
over the full 17-input Track A set and Track B product row, product deployment
enabled, `9/60/80` iterations, and unchanged signed thresholds. Run 1 builds a
source-bound native harness; runs 2-5 reuse those exact executable bytes and
receipt. Aggregate provenance requires the mode sequence
`build,reuse,reuse,reuse,reuse`.

All `32/32` aggregate rows and all `160/160` individual observations pass.
Short-key medians are `n 0.202x <= 3.006x`, `ni 0.241x <= 2.666x`, and
`hao 0.278x <= 1.844x`. The 37- and 59-character medians are `0.018x <=
2.339x` and `0.008x <= 1.748x`. Track B session medians are `63,569,920 <=
66,872,115` working-set bytes and `28,409,856 <= 32,084,378` private bytes. No
ceiling changed.

Rejected attempts are preserved under
[`performance-ratchet/failed-attempts/`](./performance-ratchet/failed-attempts/):
eager materialization crossed the short-key gate; the first bounded cache kept
too much Track B session memory; two five/three-run groups were green but
provenance-invalid because Windows linker output was not byte-deterministic;
the first source-bound green packet was rejected when the literal release
Clippy gate found MSRV/type-complexity blockers; and the next green packet was
rejected when internal review found an equal-weight bounded-prefix inversion.
The otherwise-green `eb117c53` packet was then rejected when the literal
WEB-03 long-input gate exposed an unbounded oversized-prism scan. `d2499358`
bounded that scan, but its global-head merge failed a later equal-weight source-
order review. `c5d954e2` restores librime's per-chunk current-head order and
`d508e05b` keeps bounded and complete shared-prefix candidate qualities field-
identical. The accepted behavior and five-run performance packets were both
regenerated from clean `d508e05b`.

## Known non-4b close blocker

The source-current committed WEB-04 Playwright gate is red. Two plain-Luna
control literals became stale after 4a; when those controls are refreshed in a
scratch copy from the pinned null-grammar fixture, the Octagram rows themselves
are `2/4`: `youhuiyong` and the 37-character control pass, while
`jintianhuiyi` and `jintianwanshangyouhui` retain the plain ranking. The same
Octagram `2/4` result reproduces before 4b at `afb7079b` and at `d508e05b`.

This is not attributed to the 4b abbreviation mechanism. Luna's untoned
identity dictionary remains on the explicitly deferred legacy merge path that
Increment 4e owns. The designated review packet therefore makes no broad web-
gate claim, and the WEB-04 regression remains a mandatory 4e/final-M59 blocker.
The scratch direct-witness experiment was rejected and no engine or WEB-04 test
change from that experiment is included here.

## Acceptance boundary

The mechanism is structure-driven: prism spelling-type metadata, global
abbreviation-family merging, longest-recognized-prefix recomposition, and
page-bounded admission. There is no schema-id gate, input allowlist, promotion
table, baked oracle output, public ABI change, or `Rime::Table/4.0`
header/version change. The inclusive implementation chain does advance Yune's
optional backward-compatible table metadata from v1 to v2 to persist the
compiled weight-domain marker; legacy and external tables without it retain
their established defaults. `RimeFinalize` clears sessions and ephemeral
page/input state while preserving fingerprinted
same-root immutable caches; explicit cleanup, root changes, and workspace
updates clear sessions and all shared translator/lookup caches.

Earlier implementation reviews and an independent provenance-tool review found
no 4b mechanism or evidence-tool blocker. Two final independent packet reviews
also approved the completed native receipt, manifest, Git-filter integrity, and
publication boundary. The externally designated Opus review remains pending;
`next_engine_increment_allowed` stays `false` until its findings are returned
and resolved. Exact verification and packet hashes are recorded in
[`verification.md`](./verification.md) and
[`packet-manifest.json`](./packet-manifest.json).
