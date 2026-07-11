# M59 Increment 4a — Luna ScriptTranslation page-order repair

**Status:** implementation and five-round macOS diagnostic complete on the M59
repair branch; M59 remains open. This packet closes only the expanded Luna
long-input first-page defect. It does not close the seven-row Lane B complete-list
requirement, M59-REACH-03/04, M59-EVIDENCE-01, M59-GATES-01, or M59 itself.

No signed ceiling, exception class, milestone, public ABI, or schema-specific
promotion table changed.

At authoring, these commits are local and unpushed on
`codex/m59-luna-page-order-parity`; the report is a merge-ready handoff, not a
claim that `origin/main` already contains the repair.

## Finding and repair

The defect was real and cross-platform, not a macOS-only librime anomaly. Yune
modeled upstream Luna `ScriptTranslation` as a plural sentence beam and exposed
as many as five full-span sentence alternatives before phrase candidates.
Pinned librime exposes one best sentence and then walks an independent table
phrase stream.

The first repair separated the sentence and phrase streams and preserved
partial-selection display metadata. A retained five-round pre-domain-fix packet
then exposed one deterministic 59-character residual: `遮蓋` appeared in Yune's
first page but not librime's. The follow-up root cause was a second missing
librime semantic:

- librime's ScriptEncoder applies its inclusive 5% pronunciation filter to
  source-domain double weights;
- the compiler then stores `ln(raw_weight)` as `f32` in `.table.bin`;
- Yune reconstructed its default-owned sentence model from that compiled table
  but treated the stored logarithms as raw weights, summed them for the 5% test,
  and logged them again for graph scoring.

The accepted repair carries the entry-weight domain explicitly, scores
natural-log entries without double logging, reconstructs raw totals for the 5%
admission rule, and handles the exact inclusive boundary through the stored
`f32` rounding interval. A purpose-built, non-circular table compiled by pinned
librime locks the distinction: the accepted page is `這個引擎, 這個, 這歌, 這格,
這`, and the false `遮蓋` candidate is forbidden.

## Source and binary identity

- Measured Yune engine commit:
  `1f0fb0e5b90d50b0b16aef8195acab423c277fe5`
- Later integration-test-only commit: `fa7f3961`
- Pinned librime commit:
  `33e78140250125871856cdc5b42ddc6a5fcd3cd4`
- Yune dylib SHA-256, identical before/after every accepted round:
  `48e8848989af86c2941d6a89e5c5ba87bbdd0a2738fda693b4c2fd3b3b346977`
- librime dylib SHA-256, identical before/after every accepted round:
  `af019c3dccde16d875b9543a1cbc950517e309e11fb4d0bf379b7d576aae13d3`
- Complete candidate-snapshot SHA-256, identical in all five accepted rounds:
  `1e79ecf566e3ed3f17907ddfed588b869fd5200094a8e5085b57a02f4bb32a88`
- Purpose-built decoded librime `.table.bin` SHA-256:
  `8286e67cc60aa78c6e47bf871de130ee570bf6fe7dd99c8cc6b445cad73ea5fb`
- Yune and librime source trees were clean for measurement. The only Yune
  worktree extra was an ignored symlink to external evidence.

The fixture and provenance checks live under
`crates/yune-core/tests/fixtures/upstream-1.17.0/m59-librime-log-weight/`.

## Accepted macOS protocol

Host: MacBook Air Mac17,3 (MDH74LL/A), Apple M5 (4 performance + 6 efficiency
cores), 16 GB RAM, macOS 26.5.1 (25F80), APFS. It remained on AC power at 100%,
with Low Power Mode disabled and no recorded thermal/performance warning.
Rust/Cargo were 1.96.1; Command Line Tools were 26.6. Full Xcode was not the
selected developer directory.

Measurement ran from `2026-07-11T14:22:10Z` through
`2026-07-11T14:43:30Z`, after release prebuild and about 60 seconds idle. No
compilation, indexing, export, or backup job was intentionally concurrent.
Codex, WindowServer, Chrome, and Telegram remained visible; the end snapshot
showed modest Chrome/Codex activity, so short-row timing spread remains a host
noise caveat.

Exact protocol: 9 iterations, 60 session iterations, 80 key iterations,
product deployment enabled, the same 17 Track A inputs and Track B product
input in every round.

External accepted run paths (generated output is intentionally not tracked):

1. `/Users/laufei/yune-m59-luna-page-order-parity-verification-20260711/post-domain-fix/run-1`
2. `/Users/laufei/yune-m59-luna-page-order-parity-verification-20260711/post-domain-fix/run-2`
3. `/Users/laufei/yune-m59-luna-page-order-parity-verification-20260711/post-domain-fix/run-3`
4. `/Users/laufei/yune-m59-luna-page-order-parity-verification-20260711/post-domain-fix/run-4`
5. `/Users/laufei/yune-m59-luna-page-order-parity-verification-20260711/post-domain-fix/run-5`

Each path preserves environment, commands, comparison and raw summaries,
candidate snapshots, M37 counters, memory-owner profiles, product status,
macOS verdict, and pre/post binary hashes.

## Track A — complete 17-row diagnostic

`Worst` is the maximum of the five run-level Yune/librime median ratios.
`Spread` is `(max-min)/min`. The Windows values are the signed Increment-0
references and are diagnostic on macOS; they are not new Mac acceptance gates.
Classification uses the requested absolute Mac-versus-Windows median
difference: within 10% `close`, over 10% through 25% `notable`, over 25%
`material`.

| Input | Run 1 | Run 2 | Run 3 | Run 4 | Run 5 | Mac median | Worst | Spread | Windows median | Windows ceiling | Mac vs Windows | Class |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `n` | 3.766 | 4.334 | 3.945 | 3.742 | 4.278 | 3.945 | 4.334 | 15.8% | 2.820 | 3.006 | +39.9% | material |
| `ni` | 2.835 | 2.838 | 2.589 | 2.457 | 3.016 | 2.835 | 3.016 | 22.8% | 2.599 | 2.666 | +9.1% | close |
| `hao` | 1.681 | 1.730 | 1.909 | 1.690 | 2.022 | 1.730 | 2.022 | 20.3% | 1.720 | 1.844 | +0.6% | close |
| `zhongguo` | 1.045 | 0.940 | 1.011 | 1.003 | 1.038 | 1.011 | 1.045 | 11.2% | 0.293 | 0.323 | +245.1% | material |
| 37-char `ceshiyixiachangjushuruxingnengzenyang` | 2.410 | 2.428 | 2.380 | 2.438 | 2.416 | 2.416 | 2.438 | 2.4% | 2.132 | 2.339 | +13.3% | notable |
| 59-char `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` | 1.677 | 1.788 | 1.767 | 1.809 | 1.808 | 1.788 | 1.809 | 7.9% | 1.681 | 1.748 | +6.4% | close |
| `cszysmsrsd` | 0.565 | 0.589 | 0.592 | 0.606 | 0.596 | 0.592 | 0.606 | 7.3% | 0.396 | 0.474 | +49.5% | material |
| `zybfshmsru` | 0.821 | 0.846 | 0.837 | 0.844 | 0.879 | 0.844 | 0.879 | 7.1% | 0.569 | 0.695 | +48.3% | material |
| `zh` (newly signed) | 3.181 | 3.094 | 3.039 | 3.120 | 3.095 | 3.095 | 3.181 | 4.7% | 0.986 | 1.047 | +213.9% | material |
| `j` (newly signed) | 7.282 | 6.908 | 6.795 | 7.053 | 8.533 | 7.053 | 8.533 | 25.6% | 4.000 | 4.372 | +76.3% | material |
| `yi` (newly signed) | 4.115 | 3.922 | 3.820 | 4.140 | 3.861 | 3.922 | 4.140 | 8.4% | 5.777 | 6.098 | -32.1% | material |
| `che` (newly signed) | 2.573 | 2.443 | 2.418 | 2.537 | 2.452 | 2.452 | 2.573 | 6.4% | 1.081 | 1.160 | +126.8% | material |
| `chuang` (newly signed) | 2.358 | 2.082 | 2.108 | 2.267 | 2.185 | 2.185 | 2.358 | 13.3% | 1.266 | 1.357 | +72.6% | material |
| `b` (newly signed) | 5.977 | 5.503 | 5.404 | 5.637 | 6.003 | 5.637 | 6.003 | 11.1% | 3.439 | 3.775 | +63.9% | material |
| `ceshi` (newly signed) | 1.404 | 1.305 | 1.715 | 1.353 | 1.719 | 1.404 | 1.719 | 31.7% | 0.895 | 0.966 | +56.9% | material |
| `zhongdengchangdu` (newly signed) | 0.586 | 0.626 | 0.621 | 0.629 | 0.611 | 0.621 | 0.629 | 7.3% | 0.322 | 0.342 | +92.9% | material |
| `dazisudu` (newly signed) | 3.147 | 3.231 | 3.261 | 3.162 | 3.229 | 3.229 | 3.261 | 3.6% | 1.034 | 1.098 | +212.3% | material |

Thirteen rows are material, one notable, and three close. Fifteen macOS
medians are above the Windows ceiling; only `hao` and `yi` are at or below it.
That is a platform diagnostic, not a failed Windows gate or authority to
re-baseline.

### Explicit 37/59 behavior

All five 37-character pages match librime exactly:

`測試一下長句輸入性能怎樣, 測試一下, 測試儀, 測試, 側室`

All five 59-character pages match librime exactly:

`這個引擎其實應該支持超長句子輸入才能用, 這個, 這歌, 這格, 這`

The false `遮蓋` candidate is absent. The 37-character timing difference is
notable (+13.3%) with only 2.4% spread; the 59-character difference is close
(+6.4%) with 7.9% spread. Candidate exactness is deterministic and therefore
not explained by thermal or binary variation.

## Track B product input

Absolute Track B latency and process memory are macOS-only diagnostics and are
not interchangeable with Windows counters.

| Run | Median µs | p95 µs | Max µs | Median working set | Peak working set |
|---|---:|---:|---:|---:|---:|
| 1 | 286.232 | 304.314 | 313.249 | 328.0 MiB | 440.2 MiB |
| 2 | 260.118 | 287.585 | 333.896 | 329.0 MiB | 440.0 MiB |
| 3 | 261.323 | 264.772 | 267.070 | 328.1 MiB | 433.9 MiB |
| 4 | 260.727 | 264.760 | 286.240 | 331.3 MiB | 444.0 MiB |
| 5 | 280.585 | 285.719 | 291.132 | 387.6 MiB | 443.2 MiB |

Median of run medians: 261.323 µs. Worst run median: 286.232 µs. Pooled worst
sample: 333.896 µs. Median spread: 10.0%.

The first page is identical in every accepted run and byte-equivalent to both
M57 passes:

`你個人經其實應該支援超場句子輸入先可以用, 你個, 你, 呢, 尼`

Track B's behavior/comment shape is therefore stable. Its deterministic work
counters changed from M57: spelling expansions +33.9%, exact lookups +4.2%,
prefix lookups unchanged, materialized candidates +95.2%, bounded selections
+110.9%, and full counts +0.6%. That is a future optimization lead, not a
behavior discrepancy or an M59 threshold change.

## Candidate and model-owner reconciliation with M57

- Track A source/table checksums remain `0xb3d4e98e` / `0x29d56c89`.
- Track A still reports accepted upstream-Marisa, mmap-backed storage, 498,564
  stored entries, 332,604 codes, 513,353 expanded entries, 513,353 poet entries,
  332,604 lookup rows, 421,966 vocabulary items, and the 11-row abbreviation
  vocabulary.
- The vocabulary retained-byte estimate changed from 53,644,752 to 50,061,633
  because of representation; item counts and storage identity did not change.
- Track B primary/scolar checksums remain `0xf6589c0c` / `0x822bccba`; storage is
  fresh, byte-backed, and mmap-backed with zero upstream-poet ownership.
- The two Track B table files are each 28 bytes longer than M57, while logical
  counts, candidate/comment behavior, and owner class remain unchanged.

The repair changes the 37/59 pages from M57's multiple full-span alternatives
to the pinned librime page shape. Track B remains byte-identical to M57.

## Preserved failures and limitations

The original post-page-order, pre-weight-domain five-run packet remains under
`/Users/laufei/yune-m59-luna-page-order-parity-verification-20260711/run-1`
through `run-5`. It found the deterministic `遮蓋` residual and is superseded,
not discarded or cherry-picked.

Six explicitly named setup retries remain at the external root; none reached a
measurement sample:

1. private tool/dependency bootstrap interrupted during Boost download;
2. partial Boost tarball rejected after a wrong relative cleanup path;
3. manual `make deps` omitted the script's `PYTHONPATH`;
4. optional zsh pre-clone loop relied on POSIX scalar word splitting;
5. moving the build to `.noindex` changed the CMake install prefix and triggered relinking;
6. logical-versus-physical CMake paths triggered a rebuild, stopped at 39%.

Portable report validation, packaging, and structural verification passed.
Automated interactive HTML verification was skipped because the packaged
runtime had no Chromium headless shell and the in-app browser blocks automated
local `file://` navigation. Native browser/product gates are not claimed by
this macOS engine diagnostic.

## Interpretation and remaining M59 scope

- **Real macOS engine-path discrepancy:** the candidate-page defect was real in
  Yune and has been repaired against the pinned oracle. It was not caused by
  macOS, thermal state, or a variable binary.
- **Platform-specific performance:** the broad Mac-versus-Windows ratio shifts
  are plausibly dominated by CPU/OS/compiler/allocator and librime platform
  differences. This packet cannot attribute them to one component.
- **Thermal/noise contribution:** no warning was recorded, but short-row and
  process-memory spreads show ordinary host/runtime noise. That caveat does not
  affect byte-identical candidate or hash findings.
- **Still open:** M59-PARITY-02's seven-row complete-list lane is not closed by
  these two long-input pages. `moboyi` is exact in the current diagnostic;
  `boyi`, `yi`, `zhonggao`, `zhongguo`, `gao`, and `guo` retain separate
  ordering/admission work. No new exception is authorized.

## External report and validation

- Portable HTML:
  `/Users/laufei/yune-m59-luna-page-order-parity-verification-20260711/post-domain-fix/report.html`
- Full 17-row CSV:
  `/Users/laufei/yune-m59-luna-page-order-parity-verification-20260711/post-domain-fix/track-a-17-row-comparison.csv`
- Validation receipt:
  `/Users/laufei/yune-m59-luna-page-order-parity-verification-20260711/post-domain-fix/report-validation.json`
- Source notes:
  `/Users/laufei/yune-m59-luna-page-order-parity-verification-20260711/post-domain-fix/report-source-notes.md`

The independent validation recomputed all 17 rows from the five raw run files
and signed Windows CSV, confirmed both binary hashes and the complete candidate
snapshot hash, checked Track B aggregates, and reconciled the portable artifact.
It passed 43 checks with no discrepancy.
