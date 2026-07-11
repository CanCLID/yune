# M59 Increment 4a — Luna ScriptTranslation page-order repair

**Status:** implementation and five-round macOS diagnostic complete at measured
Yune commit `89875ee2f812d070b43d12e6700407dccbb78435`; M59 remains open. This
packet demonstrates repair of only the expanded Luna long-input first-page
defect at that source commit. It does not close the seven-row Lane B
complete-list requirement, M59-REACH-03/04, M59-EVIDENCE-01, M59-GATES-01, or
M59 itself.

No signed ceiling, exception class, milestone, public C ABI, or schema-specific
promotion table changed.

The evidence remains bound to the measured commit above, which is preserved in
repository history. Later source reconciliation with the Windows Increment 4a
series is not a remeasurement of the combined tree. This packet therefore makes
no claim about `origin/main` inclusion or combined-tree performance.

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

- librime's ScriptEncoder phrase expansion applies its inclusive 5%
  pronunciation filter to source-domain double weights;
- independently, the compiler retains the collected word entries and stores
  `ln(raw_weight)` as `f32` in `.table.bin`;
- Yune reconstructed its default-owned sentence model from that compiled table
  but treated the stored logarithms as raw weights, summed them for the 5% test,
  and logged them again for graph scoring.

The accepted repair carries the entry-weight domain explicitly, scores
natural-log entries without double logging, reconstructs raw totals for the 5%
admission rule, and handles the exact inclusive boundary through the stored
`f32` rounding interval. A purpose-built, non-circular table compiled by pinned
librime locks the distinction: the accepted page is `這個引擎, 這個, 這歌, 這格,
這`, and the false `遮蓋` candidate is forbidden.

The review follow-up also keeps the existing exact-user merge from leaking
across translators. That rule is deliberately bounded to exactly one active,
default-quality Luna ScriptTranslation owner; inactive/tag-mismatched,
multiple-owner, and non-default-quality configurations retain the legacy merge.
Generic ScriptTranslation predictive-userdb parity is not claimed by this
page-order increment and no predictive rule was added.

## Source and binary identity

- Measured and independently reviewed Yune engine commit:
  `89875ee2f812d070b43d12e6700407dccbb78435`
- Pinned librime commit:
  `33e78140250125871856cdc5b42ddc6a5fcd3cd4`
- Yune dylib SHA-256, identical at the pre-packet build and after every
  accepted round:
  `57bcf505e86136ae7badeb1333ff654f48c09f97120bfa85b7a0133396accaf0`
- librime dylib SHA-256, single-valued across all five accepted rounds:
  `af019c3dccde16d875b9543a1cbc950517e309e11fb4d0bf379b7d576aae13d3`
- Complete candidate-snapshot SHA-256, identical in all five accepted rounds:
  `1e79ecf566e3ed3f17907ddfed588b869fd5200094a8e5085b57a02f4bb32a88`
- Purpose-built decoded librime `.table.bin` SHA-256:
  `34784ffd5af9bdc79926a00057cbf8c201a64473a2334acd748685e2d1fd6405`
- Yune and librime source trees were clean for measurement. The only Yune
  worktree extra was an ignored symlink to external evidence.

The fixture and provenance checks live under
`crates/yune-core/tests/fixtures/upstream-1.17.0/m59-librime-log-weight/`.

## Accepted macOS protocol

Host: MacBook Air Mac17,3, Apple M5, 16 GB RAM, macOS 26.5.1 (25F80), APFS.
It remained on AC power at 100%, with Low Power Mode disabled and no recorded
thermal/performance warning.
Rust/Cargo were 1.96.1; Command Line Tools were 26.6. Full Xcode was not the
selected developer directory.

Measurement ran from `2026-07-11T16:41:56Z` through
`2026-07-11T16:57:33Z`, after release prebuild and a 40-second stabilization
interval. No separate compilation, indexing, export, or backup job was
concurrent with the timed samples. The prescribed script rebuilt its harness
between lanes; those builds completed before each timed process began. Codex,
ChatGPT, Claude, WindowServer, Chrome, Telegram, Slack, Docker, and Granola UI
processes remained visible. The host is therefore not claimed to be perfectly
idle, and the short-row and pooled-maximum spreads retain an explicit noise
caveat.

Exact protocol: 9 iterations, 60 session iterations, 80 key iterations,
product deployment enabled, the same 17 Track A inputs and Track B product
input in every round.

External accepted run paths (generated output is intentionally not tracked):

1. `/Users/laufei/yune-m59-luna-page-order-parity-verification-20260711/post-review-fix/run-1`
2. `/Users/laufei/yune-m59-luna-page-order-parity-verification-20260711/post-review-fix/run-2`
3. `/Users/laufei/yune-m59-luna-page-order-parity-verification-20260711/post-review-fix/run-3`
4. `/Users/laufei/yune-m59-luna-page-order-parity-verification-20260711/post-review-fix/run-4`
5. `/Users/laufei/yune-m59-luna-page-order-parity-verification-20260711/post-review-fix/run-5`

Each path preserves environment, commands, comparison and raw summaries,
candidate snapshots, M37 counters, memory-owner profiles, product status,
macOS verdict, and binary identity hashes. Run 1 additionally preserves the
outer-wrapper post-step failure described below; all measurements had already
completed and were not retried or replaced.

## Track A — complete 17-row diagnostic

`Worst` is the maximum of the five run-level Yune/librime median ratios.
`Spread` is `(max-min)/min`. The Windows values are the signed Increment-0
references and are diagnostic on macOS; they are not new Mac acceptance gates.
Classification uses the requested absolute Mac-versus-Windows median
difference: within 10% `close`, over 10% through 25% `notable`, over 25%
`material`.

| Input | Run 1 | Run 2 | Run 3 | Run 4 | Run 5 | Mac median | Worst | Spread | Windows median | Windows ceiling | Mac vs Windows | Class |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `n` | 3.996 | 3.832 | 3.618 | 2.864 | 3.909 | 3.832 | 3.996 | 39.5% | 2.820 | 3.006 | +35.9% | material |
| `ni` | 2.703 | 2.532 | 2.707 | 2.259 | 2.471 | 2.532 | 2.707 | 19.8% | 2.599 | 2.666 | -2.6% | close |
| `hao` | 1.681 | 1.748 | 1.685 | 1.306 | 1.775 | 1.685 | 1.775 | 35.9% | 1.720 | 1.844 | -2.0% | close |
| `zhongguo` | 0.946 | 1.016 | 0.966 | 0.770 | 0.988 | 0.966 | 1.016 | 31.9% | 0.293 | 0.323 | +229.7% | material |
| 37-char `ceshiyixiachangjushuruxingnengzenyang` | 2.428 | 2.706 | 2.388 | 2.028 | 2.463 | 2.428 | 2.706 | 33.4% | 2.132 | 2.339 | +13.9% | notable |
| 59-char `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` | 1.809 | 2.052 | 1.774 | 1.434 | 1.831 | 1.809 | 2.052 | 43.1% | 1.681 | 1.748 | +7.6% | close |
| `cszysmsrsd` | 0.600 | 0.649 | 0.590 | 0.480 | 0.586 | 0.590 | 0.649 | 35.2% | 0.396 | 0.474 | +49.0% | material |
| `zybfshmsru` | 0.846 | 0.900 | 0.831 | 0.788 | 0.826 | 0.831 | 0.900 | 14.2% | 0.569 | 0.695 | +46.0% | material |
| `zh` (newly signed) | 3.026 | 2.997 | 2.989 | 2.696 | 2.991 | 2.991 | 3.026 | 12.2% | 0.986 | 1.047 | +203.3% | material |
| `j` (newly signed) | 7.274 | 7.422 | 7.259 | 6.195 | 6.657 | 7.259 | 7.422 | 19.8% | 4.000 | 4.372 | +81.5% | material |
| `yi` (newly signed) | 3.878 | 3.946 | 3.864 | 3.857 | 3.795 | 3.864 | 3.946 | 4.0% | 5.777 | 6.098 | -33.1% | material |
| `che` (newly signed) | 2.558 | 2.465 | 2.519 | 2.304 | 2.362 | 2.465 | 2.558 | 11.0% | 1.081 | 1.160 | +128.0% | material |
| `chuang` (newly signed) | 2.236 | 2.262 | 2.319 | 1.933 | 2.060 | 2.236 | 2.319 | 20.0% | 1.266 | 1.357 | +76.6% | material |
| `b` (newly signed) | 5.884 | 5.600 | 5.445 | 3.768 | 5.274 | 5.445 | 5.884 | 56.2% | 3.439 | 3.775 | +58.3% | material |
| `ceshi` (newly signed) | 1.336 | 1.377 | 1.355 | 1.037 | 1.212 | 1.336 | 1.377 | 32.8% | 0.895 | 0.966 | +49.3% | material |
| `zhongdengchangdu` (newly signed) | 0.633 | 0.707 | 0.617 | 0.509 | 0.620 | 0.620 | 0.707 | 38.9% | 0.322 | 0.342 | +92.5% | material |
| `dazisudu` (newly signed) | 3.167 | 3.261 | 3.174 | 2.931 | 3.085 | 3.167 | 3.261 | 11.3% | 1.034 | 1.098 | +206.3% | material |

Thirteen rows are material, one notable, and three close. Fourteen macOS
medians are above the Windows ceiling; `ni`, `hao`, and `yi` are at or below it.
That is a platform diagnostic, not a failed Windows gate or authority to
re-baseline.

### Explicit 37/59 behavior

All five 37-character pages match librime exactly:

`測試一下長句輸入性能怎樣, 測試一下, 測試儀, 測試, 側室`

All five 59-character pages match librime exactly:

`這個引擎其實應該支持超長句子輸入才能用, 這個, 這歌, 這格, 這`

The false `遮蓋` candidate is absent. The 37-character timing difference is
notable (+13.9%) with 33.4% spread; the 59-character difference is close
(+7.6%) with 43.1% spread. Those timing spreads are consistent with ordinary
host/runtime noise, while candidate exactness is deterministic and is not
explained by thermal or binary variation.

## Track B product input

Absolute Track B latency and process memory are macOS-only diagnostics and are
not interchangeable with Windows counters.

| Run | Median µs | p95 µs | Max µs | Median working set | Peak working set |
|---|---:|---:|---:|---:|---:|
| 1 | 260.006 | 460.676 | 704.124 | 335.9 MiB | 446.2 MiB |
| 2 | 261.372 | 282.441 | 283.036 | 336.8 MiB | 446.6 MiB |
| 3 | 258.631 | 261.548 | 262.589 | 398.8 MiB | 454.1 MiB |
| 4 | 258.167 | 266.324 | 315.360 | 334.9 MiB | 445.2 MiB |
| 5 | 260.473 | 262.284 | 264.590 | 333.7 MiB | 444.1 MiB |

Median of run medians: 260.006 µs. Worst run median: 261.372 µs. Pooled worst
sample: 704.124 µs. Median spread: 1.2%. Run 1's p95 and maximum are isolated
tail outliers consistent with the recorded foreground-workload caveat; they are
preserved rather than discarded.

The first page is identical in every accepted run. Its normalized
candidate/comment subset is byte-equivalent to M57 `full-pass-1`, the pass used
by this aggregation:

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
to the pinned librime page shape. Track B's normalized candidate/comment subset
remains byte-identical to M57 `full-pass-1`; its compiled table files are not
byte-identical.

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

The final-binary packet also preserves two non-measurement orchestration
failures. After run 1 had fully completed, an outer zsh wrapper rejected an
assignment to its reserved `status` variable; the round was retained and its
post-run hash recorded without rerunning. The first aggregation attempt then
stopped before output because it expected a different librime hash filename;
identical-content aliases were added and aggregation was retried without
changing any measurement.

Portable report validation, packaging, and structural verification passed.
Automated interactive HTML verification was skipped because the packaged
runtime had no Chromium headless shell and the in-app browser blocks automated
local `file://` navigation. Native browser/product gates are not claimed by
this macOS engine diagnostic.

## Interpretation and remaining M59 scope

- **Real engine-path discrepancy, observed on macOS:** the candidate-page defect
  was cross-platform Yune behavior and has been repaired against the pinned
  oracle. It was not caused by macOS, thermal state, or a variable binary.
- **Platform-specific performance:** the broad Mac-versus-Windows ratio shifts
  are plausibly dominated by CPU/OS/compiler/allocator and librime platform
  differences. This packet cannot attribute them to one component.
- **Thermal/noise contribution:** no warning was recorded, but short-row and
  process-memory spreads show ordinary host/runtime noise. That caveat does not
  affect byte-identical candidate or hash findings.
- **Still open:** M59-PARITY-02's seven-row complete-list lane is not closed by
  these two long-input pages. At measured commit `89875ee2`, `moboyi` is exact;
  `boyi`, `yi`, `zhonggao`, `zhongguo`, `gao`, and `guo` retain separate
  ordering/admission work. No new exception is authorized.

## External report and validation

- Portable HTML:
  `/Users/laufei/yune-m59-luna-page-order-parity-verification-20260711/post-review-fix/report.html`
- Full 17-row CSV:
  `/Users/laufei/yune-m59-luna-page-order-parity-verification-20260711/post-review-fix/track-a-17-row-comparison.csv`
- Validation receipt:
  `/Users/laufei/yune-m59-luna-page-order-parity-verification-20260711/post-review-fix/report-validation.json`
- Source notes:
  `/Users/laufei/yune-m59-luna-page-order-parity-verification-20260711/post-review-fix/report-source-notes.md`

The independent validation recomputed all 17 rows from the five raw run files
and signed Windows CSV, confirmed both binary hashes and the complete candidate
snapshot hash, checked Track B aggregates, and reconciled the portable artifact.
It passed 92 checks with no discrepancy. SHA-256 identities are:

- report HTML:
  `6574cb32ff4ce92241a05bfc0e4e28d8f6b7e426e64353de6f967053757b88ab`;
- artifact JSON:
  `32e5178254b0418a1940f9f56187bb403b4d510984fd05aeafc88ee2a449c712`;
- validation receipt:
  `fceeace8395764a26b5b061b9bbbc127365b2df74f3bb3107c56c075136327fe`.
