# M59 Increment-0 macOS librime analysis

> **Follow-up resolution (2026-07-11):** M59 Increment 4a repairs this
> cross-platform Yune defect without changing a signed ceiling or creating a
> milestone. Five post-fix macOS rounds now emit the pinned-librime 37- and
> 59-character first pages exactly. The 37-character page is
> `測試一下長句輸入性能怎樣, 測試一下, 測試儀, 測試, 側室`. The 59-character page is
> `這個引擎其實應該支持超長句子輸入才能用, 這個, 這歌, 這格, 這`.
> The follow-up also found and fixed a second
> compiled-table weight-domain defect that briefly admitted the false `遮蓋`
> phrase. See the
> [Increment 4a evidence](../m59-canonical-jyutping-reachability-parity/increment-4a-luna-script-translation-order/README.md).
> This original Increment-0 packet remains historical pre-fix evidence.

**Status:** Preserved Increment-0 read-only diagnostic; its candidate defect is
resolved by Increment 4a. The original measurements are not discarded. This
packet is not a replacement performance gate, a re-baseline, a new milestone,
or authority to change the active M59 execution or any signed ceiling.

The analysis measures Yune
`457751824b8944676dc44912b9ce31ff29d78403` against pinned librime
`33e78140250125871856cdc5b42ddc6a5fcd3cd4`. It records materially lower librime
timings on this Mac and allocator/platform evidence, but does not isolate one
causal component. It also reproduces the platform-independent Yune
candidate-page discrepancy already captured by M55.

## Candidate-page finding

For the 37- and 59-character Luna rows, librime's first page contains its
one-best full sentence followed by shorter phrase candidates; Yune exposes up
to five full-span sentence paths. That page shape is input-specific, not a
universal rule that every Luna page must contain exactly one full-input
candidate. Under D-24/D-48, Yune must nevertheless match the captured oracle
text and order for the governed Luna range.

At Increment-0, this was pre-existing candidate-source/interleaving parity debt
rooted in M17's then-narrow `zhongguo` coverage. M55 later captured it explicitly as ignored
oracle-backed blockers. It is separate from M57's macOS compiled-table
model-shape repair, which remains closed.

## Roadmap disposition

This note created no new milestone, did not amend `M59-PARITY-02`'s named input
list, and did not redirect the concurrently active Windows work. Increment 4a's
follow-up now records the repair under the existing D-48 Lane B policy. The
signed Windows ceilings remain unchanged.

## Artifacts

- [Interactive report](./report.html)
- [Canonical report artifact](./artifact.json)
- [Report source notes](./report-source-notes.md)
- [Hashes and provenance](./provenance.txt)
- [Pinned librime source review](./sources/librime-poet-source-notes.md)
- [Compact report queries](./sql/)
- [M55 expanded Luna fixture finding](../m55-native-match-or-beat/phase-3r-fixture-expansion/README.md)
- [M57 macOS model-shape closeout](../m57-macos-track-a-sentence-model-parity/README.md)

Portable artifact validation and packaging passed. Verification was structural
only because no compatible installed headless Chromium was available; this
limitation is preserved in the source notes.
