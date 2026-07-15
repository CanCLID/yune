# M59 current-source macOS Yune/librime verification

**Status:** reviewed diagnostic pass. This curated packet preserves the
decision-bearing report, aggregates, provenance, generators, and review
resolution from the complete external five-round packet at
`$HOME/yune-m59-current-macos-20260714`.

Primary report: [`report/report.html`](./report/report.html)

## Answer

At Yune `0111cf47c09bfe7a4a3d55a1832f35a55bc59435`, every one of the 17
Track A macOS median Yune/librime ratios is below `1.0x`, and every retained
pooled-worst ratio is also below `1.0x`. The 37-character median is `0.019x`,
the 59-character median is `0.008x`, and Track B's five-run median is
`5.607 µs/key`.

This is strong same-Mac evidence that current Yune outperforms pinned librime
on the named workloads. It is not a pure macOS-versus-Windows isolation result:
the Mac source postdates final-Windows M59 source `443cc636`, and substantial UI
activity was captured around some round boundaries. Windows ratios and signed
ceilings remain source-mismatched diagnostics only.

Candidate behavior matches same-run librime for 16/17 complete-input Track A
pages, including both 37/59 pages. The sole `zhongdengchangdu` suffix mismatch
also appears in all 15 cited Windows increment-4c/4d/4e performance-ratchet
runs, so it is a deterministic cross-platform Yune engine-path discrepancy,
not a macOS-only defect.

No threshold, signed baseline, or acceptance authority is changed by this
packet.

## Identity and retained rounds

- Yune: `0111cf47c09bfe7a4a3d55a1832f35a55bc59435`, clean detached source
- librime: `33e78140250125871856cdc5b42ddc6a5fcd3cd4`, clean detached source
- Yune dylib SHA-256:
  `f3365aae19d15b9d7b57dcccd30ce1c77347b8ee96a20f09ab001468074b226c`
- librime dylib SHA-256:
  `1973349f4da44c5b71765f8d064ec30428a0fd42d66c9ae95bdb6dc27cd4eecc`
- Full run roots:
  `$HOME/yune-m59-current-macos-20260714/accepted/run-1` through `run-5`
- Five complete logical rounds; no measured retry or discarded round
- Host: MacBook Air, Apple M5, 16 GB RAM, macOS 26.5.2, APFS
- Power: AC throughout, Low Power Mode disabled, no recorded thermal warning

The current Mac source is neither signed Increment-0 source `45775182` nor
final-Windows source `443cc636`. The earlier source-matched Increment-4e Mac
packet at [`../m59-final-source-macos-20260713/`](../m59-final-source-macos-20260713/)
remains valid historical evidence for Yune `5879405c`; it is not the current
dashboard source.

## Review resolution

Fable found no blocker or major issue. The four minor findings are resolved in
[`review/fable-review-resolution.md`](./review/fable-review-resolution.md):

- full-precision spread sensitivity is disclosed in both directions;
- the Windows `zhongdengchangdu` claim cites all 15 exact source rows;
- unsupported Time Machine wording was replaced with the captured `backupd`
  observations;
- percentile terminology now names the actual `ceil((n-1)·p)` index rule and
  upper median.

The independent calculation and behavior review is in
[`review/independent-review.md`](./review/independent-review.md). The delivered
HTML and this curated copy both pass the canonical portable-report validator at
1440 px and 390 px, including keyboard source interaction; receipts are under
[`report/`](./report/).

## Packet map

- [`aggregate/track-a-17-row-comparison.csv`](./aggregate/track-a-17-row-comparison.csv):
  complete five-round 17-row table with final-Windows and signed diagnostics.
- [`aggregate/track-b-five-observations.csv`](./aggregate/track-b-five-observations.csv):
  all five product-input observations and the absolute-platform boundary.
- [`aggregate/windows-zhongdengchangdu-evidence.csv`](./aggregate/windows-zhongdengchangdu-evidence.csv):
  exact 4c/4d/4e Windows source paths and candidate pages.
- [`aggregate/findings.md`](./aggregate/findings.md): generated technical
  narrative, 37/59 findings, candidate/model-owner comparison, and limitations.
- [`aggregate/analyze.py`](./aggregate/analyze.py): fail-closed aggregation over
  the complete external run packet and repository Windows/M57 references.
- [`report/artifact.json`](./report/artifact.json),
  [`report/snapshot.sqlite`](./report/snapshot.sqlite), and
  [`report/build_report_artifact.py`](./report/build_report_artifact.py):
  deterministic technical-report source and bounded query snapshot.
- [`driver/external-output-adapter.diff`](./driver/external-output-adapter.diff):
  the exact two path-only changes that kept measured output outside the source
  worktree.
- [`setup/hardware.txt`](./setup/hardware.txt) and
  [`setup/filesystem.txt`](./setup/filesystem.txt): diagnostic hardware and
  APFS facts with stable device/filesystem identifiers redacted only in this
  curated copy; the external raw receipts remain unchanged and unredacted.
- [`external-packet-manifest.sha256`](./external-packet-manifest.sha256):
  normalized relative-path hashes for all 409 non-manifest files in the full
  external packet.
- `packet-manifest.csv`: hash and size for every curated tracked file except the
  manifest itself.

The full raw rounds, console logs, and per-lane samples remain external. This
tracked copy intentionally does not duplicate that raw corpus.

## Reproduce and verify

Run the aggregate generator against the complete external packet:

```sh
python3 aggregate/analyze.py \
  --evidence-root "$HOME/yune-m59-current-macos-20260714" \
  --repo-root /absolute/path/to/yune
```

Then regenerate and verify the report from the packet root:

```sh
python3 report/build_report_artifact.py
node report/deliver_report.mjs \
  --plugin-root /absolute/path/to/data-analytics-plugin \
  --input report/artifact.json \
  --output report/report.html \
  > report/delivery-receipt.json
python3 finalize_packet_manifest.py
```

The delivery adapter delegates to the canonical portable-report packaging and
verification pipeline. Its only presentation change is a fail-closed
desktop-width containment rule for the packaged reader's `100vw` header;
internal table scrolling remains intact.
