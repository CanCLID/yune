# M61 Native Track A Memory-Owner Reduction Evidence

Verdict: **complete through disposition D, measured partial/no-go, subject to
the binding closeout proofs recorded outside this non-self-referential packet.**
M61 made no accepted production-default memory reduction.

The authorized correction source
`91f5969688a3d2dba96a67d1cfe813c7ba4ee861`, tree
`6626ed16d5e135fa477ca26e9786d11121c92b44`, completed five fixed-binary
owned rounds, one green exploratory byte-backed round, and five fixed-binary
byte-backed rounds. Both five-round sets passed `32/32` signed aggregate rows,
`160/160` individual observations, `17/17` candidate parity per round, fixed
source/binary/model checks, and Track B guards.

The byte-backed diagnostic moved Track A peak working-set median from
`154,030,080 B` to `116,162,560 B`; its worst peak was `116,334,592 B`, below
the frozen `125,000,000 B` supplemental cap. That projection is not an accepted
ratchet or production result. Binding owner reconciliation failed:

| Check | Result | Required | Verdict |
| --- | ---: | ---: | --- |
| whole-process private delta | `25,096,192 B` | positive | pass |
| explained named-owner delta | `18,724,242 B` | at least `10,000,000 B` | pass |
| coverage | `0.746098930` | `0.80..=1.20` | **fail** |
| residual | `6,371,950 B` | at most `5,019,238 B` | **fail** |

The first correction-source measured red exhausted disposition B and was not
retried. Commit `01a62f2a6cd2b3d668545a110de8c7c3fc2fbb10`
explicitly reverted the exact three-path correction and restored tree
`f1c36a0079d85628f5cbef140bd94288930cc2e8`, exactly equal to the pushed
quality-repair runtime tree. No production-default candidate, final production
five-round set, threshold change, or accepted supplemental row exists.

## Source boundary

- formal M60 closeout: `0eff06a088992f417602a71300c447cdfa525255`;
- M61 kickoff base: `bc0df36a6eee3ad63319d8c29336542082559c94`;
- immutable measurement tooling: `91b8991c5668ace690a4f6775bd8d91dfc0696f9`;
- corrected `17/17` candidate source: `a39c4d868820063dc3deaa42f7fdc9b3aee5e7a6`;
- source-clean POET binding repair: `f18b0df2d0149bc2a28cd9bd2c075c34030b5568`;
- Phase 0B corrected owned source: `67d32a2bea36a391a8a11ea4e725dbfebe118252`;
- pushed quality-repair source: `931c7c59d6d471c69b70dc0d2f082149665a4e68`;
- measured correction: `91f5969688a3d2dba96a67d1cfe813c7ba4ee861`;
- explicit disposition-D revert: `01a62f2a6cd2b3d668545a110de8c7c3fc2fbb10`.

The containing closeout SHA is intentionally recorded only in the external
post-push receipt.

## Restored-tree closeout recovery

The exact restored-tree closeout runner at `01a62f2a`, tree `f1c36a00`, passed
gates 1--19. Its literal `cargo test --workspace` gate exited `101` after a
successful prefix and a `37/41` `cantonese_parity` result. The raw workspace log
has SHA-256
`695dec04fc9fcc5f30b363fb4dedf2c17c31c06171427b633d8f8eee082fbd18`;
`closeout-gate-status.csv` is the unchanged source receipt.

The first cfg(test)-only correction makes four all-pages oracle comparisons
explicitly complete the candidate list while preserving the bounded initial
page assertions. Its one-path tree is
`bf4ef0b8d7d234b248cc61e9a1c5ad6b57ee61af`. The first serial retry was
interrupted before a verdict and is not reused. The next owning-slice retry
passed formatting, serial `cantonese_parity` `41/41`, and the real deployed
profile page guard `1/1`.

The disjoint never-reached tail retained the successful workspace prefix. It
passed strict workspace Clippy and eight core integration targets (`69` passed,
`8` declared ignores), then preserved a second deterministic API-library
contract red (`363` passed, `1` failed, `1` ignored). That old test predated
M56's narrow valid-session cross-thread contract. The second cfg(test)-only
correction now locks the current one-service and threading wording. The combined
two-test tree is `6cb28424f7bcf5a535ac6173b651e9ba1b7bd160`. Source-current
recovery passed formatting, exact strict workspace Clippy, the full owning API
library (`364 passed / 1 ignored`), every still-never-reached API bin and
integration target (`114 passed / 3 ignored`), and both remaining zero-test doc
groups. Nonduplicated successful accounting is `1,184 passed / 12 ignored`.
The literal broad workspace command was not rerun and is not claimed as an
exit-zero receipt.

## Preserved prerequisite and diagnostic results

- Source `7805882d` produced the preserved correctness red: `16/17`, with only
  `zhongdengchangdu` different. Source `a39c4d86` then passed `17/17` using the
  same pinned oracle shared/build identities.
- The `a39c4d86` five-round owned set passed. Its exploratory byte-backed
  artifact-preparation attempt completed measured red before usable Yune pages;
  it was preserved, diagnosed, and never reused as accepted evidence.
- Source `f18b0df2` completed two green owned rounds, then run 3 retained
  `17/17` parity but measured Track B session-private `32,727,040 B` against the
  frozen `32,084,378 B` ceiling. The three-round partial set was not aggregated
  or reused.
- Source `67d32a2b` passed five corrected owned rounds (`32/32`, `160/160`), then
  its exploratory byte-backed owner check found a fifth retained
  `poet.normal_character_code_index` row (`11,538 B`). That measured owner-shape
  red selected the one bounded disposition-B correction.
- Source `91f59696` removed that fifth retained owner and completed the full
  replacement diagnostic, but the binding coverage and residual checks above
  failed. `failure-disposition.txt` records `MEASURED_RED_NO_RETRY`.

## Evidence shape

The exact machine-generated aggregate, provenance, owner-budget, process, and
reconciliation receipts are retained here. `diagnostic-round-receipts.csv`
summarizes all eleven correction-source runs without copying raw benchmark
trees. `byte-backed-supplemental-projection.csv` is deliberately named as an
unaccepted diagnostic projection. The frozen supplemental threshold itself
remains external because disposition D produced no accepted result.

Raw logs, samples, traces, binaries, schemas, compiled table/prism/POET
artifacts, candidate snapshots, work trees, and machine-private captures remain
outside the repository under source-keyed roots. The final two independent
review receipts are absent from the preserved pre-review tree; after review,
only those receipts and `packet-manifest.csv` may change.

An initial isolated candidate exposed Git LF normalization of imported Windows
receipts: the working-tree manifest passed, while tree-mode verification failed
closed. That candidate was discarded before review. The narrow M61 packet
`-text` attribute preserves these curated bytes so all three manifest surfaces
verify identically; it does not apply to runtime or product paths.

## Scope boundary

The disposition-D closeout adds no runtime change. Its only source corrections
are `crates/yune-core/tests/cantonese_parity.rs` and
`crates/yune-rime-api/src/tests/lifecycle_safety.rs`, both test-only contract
locks. Across M61 there was no C
ABI/API table/export, schema/profile id, browser payload, product/frontend,
Windows TSF/UI, package, Cloudflare, iOS claim, oracle rebase, signed-threshold,
or historical M55/M59 evidence change. The separately authorized Phase 0A
oracle-parity correction, Phase 0B packed-syllabary prerequisite, and source-
quality repair remain accepted and are covered by their own receipts. The
rejected three-path Track A memory correction does not remain.
