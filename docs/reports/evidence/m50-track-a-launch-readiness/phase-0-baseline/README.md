# M50 Phase 0 Baseline

Scope: native Track A `luna_pinyin` only for the M50 decision path. This evidence makes no browser, frontend, package, deployment, public-demo, TypeDuck product, or iOS-device claim.

Baseline commit: `76edb38998b5d35e78491dff00ff548d9bb33dd3`.

Command evidence: `commands.txt`; environment evidence: `environment.txt`. Summary values below are copied from `summary.csv`, `m37_metrics.csv`, and `memory-owner-profile.csv`.

## Tracked Rows

| Row | Yune median | librime median | Ratio | Yune peak working set | Yune private proxy | Verdict |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| `n` | `57.300 us` | `20.700 us` | `2.768x` | `188,510,208 B` | `190,382,080 B` | pass |
| `ni` | `44.900 us` | `14.750 us` | `3.044x` | `188,510,208 B` | `190,418,944 B` | blocker |
| `ceshiyixiachangjushuruxingnengzenyang` | `915.897 us` | `289.705 us` | `3.161x` | `188,510,208 B` | `193,261,568 B` | blocker |
| Track A max peak working set | `188,510,208 B` | `17,317,888 B` | n/a | `188,510,208 B` | `192,630,784 B` process private proxy row | blocker |

Passing guard rows in this same run: `hao` is `25.000 us` / `2.206x`, `zhongguo` is `46.450 us` / `0.285x`, the 59-character pinyin row is `1,653.115 us` / `2.436x`, `cszysmsrsd` is `535.350 us` / `0.432x`, and `zybfshmsru` is `544.070 us` / `0.628x`.

## Owner Notes

Short-prefix owner rows from `m37_metrics.csv`:

| Row | Median process key ns | Median translator ns | Prefix lookup | Bounded iterator | Short-key rows scanned | Main note |
| --- | ---: | ---: | --- | --- | ---: | --- |
| `n` | `55,400` | `53,400` | `1` call / `22,900 ns` / `7` candidates | `1` call / `7` selected / `7` full count | `7` | Now passes the launch-readiness ratio; keep as a guard row. |
| `ni` | `87,750` | `83,400` | `1` call / `23,300 ns` / `7` candidates | `2` calls / `14` selected / `14` full count | `14` | Just above `3.0x`; remaining owner is short-prefix translator/filter/materialization constant factor, not sentence model work. |

37-character row owner notes from `m37_metrics.csv`:

| Row | Median process key ns | Median translator ns | Prefix lookup | Upstream sentence model | Vocabulary considered | Graph rebuild |
| --- | ---: | ---: | --- | ---: | ---: | ---: |
| `ceshiyixiachangjushuruxingnengzenyang` | `33,869,450` | `33,703,900` | `36` calls / `324,800 ns` / `87` candidates | `27` calls / `31,669,100 ns` | `3,950` | `23,324,150 ns` |

Memory-owner notes from `memory-owner-profile.csv`:

| Owner | Retained estimate | Non-overlapping reducible | Classification | Note |
| --- | ---: | ---: | --- | --- |
| `poet.entries_by_code` | `18,694,662 B` | `18,694,662 B` | named heap owner | Sentence model entries cloned from table rows. |
| `poet.lookup_index` | `2,660,848 B` | `0 B` | guarded heap owner | M40 sorted code-range index used by sentence lookup. |
| `compact_table.storage` | `13,013,460 B` | `0 B` | mmap/file-backed | `byte_backed:mmap`; overlaps compact payload rows. |
| `process.after_ready_working_set_unclassified_lower_bound` | `159,831,391 B` | `0 B` | measured blocker | Median after-ready working set minus non-overlapping reducible owners. |
| `process.peak_working_set_high_water` | `188,510,208 B` | `0 B` | measured blocker | Whole-process peak includes allocator, loader, mappings, and overlap. |
| `process.private_bytes_proxy` | `192,630,784 B` | `0 B` | measured blocker | Windows private-bytes proxy, not iOS `phys_footprint`. |

## Decision

The M50 reduction order is:

1. Sentence row first, because the 37-character row is the largest current latency miss (`3.161x`) and its owner is the bounded upstream sentence model graph rebuild path.
2. Short-prefix second, focused on `ni`; `n` now passes but remains a guard row.
3. Memory attribution third, because the existing owner rows leave a large process-level unclassified working-set/private gap that must be named even if no safe reduction is retained.
