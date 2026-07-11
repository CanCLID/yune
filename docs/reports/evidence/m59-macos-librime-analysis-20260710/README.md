# M59 Increment-0 macOS librime analysis

**Status:** Read-only diagnostic. This packet is not a replacement performance
gate, a re-baseline, a new milestone, or authority to change the active M59
execution or any signed ceiling.

The analysis measures Yune
`457751824b8944676dc44912b9ce31ff29d78403` against pinned librime
`33e78140250125871856cdc5b42ddc6a5fcd3cd4`. It confirms that macOS materially
accelerates this librime build, especially through allocator behavior, while
also reproducing a platform-independent Yune candidate-page discrepancy already
captured by M55.

## Candidate-page finding

For the 37- and 59-character Luna rows, librime's first page contains its
one-best full sentence followed by shorter phrase candidates; Yune exposes up
to five full-span sentence paths. That page shape is input-specific, not a
universal rule that every Luna page must contain exactly one full-input
candidate. Under D-24/D-48, Yune must nevertheless match the captured oracle
text and order for the governed Luna range.

This is pre-existing candidate-source/interleaving parity debt rooted in M17's
then-narrow `zhongguo` coverage. M55 later captured it explicitly as ignored
oracle-backed blockers. It is separate from M57's macOS compiled-table
model-shape repair, which remains closed.

## Roadmap disposition

This note creates no new milestone, does not amend `M59-PARITY-02`'s named input
list, and does not redirect the concurrently active Windows work. It records the
debt for reconciliation under the existing D-48 Lane B policy. The signed
Windows ceilings remain unchanged.

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
