# M45 Final Memory Attribution

Date: 2026-06-27

M45 separates steady after-ready resident memory from observed high-water peak
and private/pagefile counters. The final memory verdict is
`steady-state-meets-target-standing-peak-cost`.

## Resident And Peak Bands

| Track A row | Median working set | Max peak working set | Median private bytes | Max peak pagefile |
| --- | ---: | ---: | ---: | ---: |
| startup/runtime-ready | `90,161,152 B` | `127,475,712 B` | `74,104,832 B` | `112,218,112 B` |
| session create/select/destroy | `87,498,752 B` | `127,475,712 B` | `71,790,592 B` | `112,218,112 B` |
| `n` | `91,058,176 B` | `127,475,712 B` | `74,776,576 B` | `112,218,112 B` |
| `ni` | `91,754,496 B` | `127,475,712 B` | `75,522,048 B` | `112,218,112 B` |
| `hao` | `92,749,824 B` | `127,475,712 B` | `76,472,320 B` | `112,218,112 B` |
| `zhongguo` | `93,605,888 B` | `127,475,712 B` | `77,549,568 B` | `112,218,112 B` |
| 37-character Track A row | `94,449,664 B` | `127,475,712 B` | `78,004,224 B` | `112,218,112 B` |
| 59-character Track A row | `95,727,616 B` | `127,475,712 B` | `79,298,560 B` | `112,218,112 B` |
| `cszysmsrsd` | `97,689,600 B` | `127,475,712 B` | `81,412,096 B` | `112,218,112 B` |
| `zybfshmsru` | `98,684,928 B` | `127,475,712 B` | `82,468,864 B` | `112,218,112 B` |

All steady Track A rows are below the old `107,797,708 B` resident target. The
high-water peak is not solved: the first startup sample begins at `4,722,688 B`,
reaches `90,161,152 B` after ready, finalizes to `87,248,896 B`, and still
records `127,475,712 B` peak working set plus `112,218,112 B` peak pagefile.

## Retained Owner Classification

| Owner | Class | Retained estimate | Peak-moving verdict |
| --- | --- | ---: | --- |
| `poet.entries_by_code` | heap-owned reducible | `18,694,662 B` | Largest reducible retained owner, already reduced in M43; not enough to explain peak. |
| `compact_table.storage` | mmap file-backed | `13,013,460 B` | File-backed selected storage, not a heap mirror. |
| `poet.lookup_index` | heap-owned guarded | `2,660,848 B` | Required for M40 sentence lookup performance. |
| `compact_table.syllable_ids_by_code` | heap-owned reducible | `15,757 B` | Too small to move process peak. |
| `compact_table.syllabary_codes` | heap-owned reducible | `13,685 B` | Too small to move process peak. |
| `poet.abbreviation_vocabulary` | heap-owned reducible | `1,433 B` | Too small to move process peak. |
| `schema.processors`, `runtime.session_state`, `session.userdb` | heap-owned guarded | `5,789 B` total | Guarded runtime/session state. |

The profile reconciles known retained owners against measured process memory
but does not identify a new safe, bounded, peak-moving owner. M45 therefore
does not retain a memory-code change and does not claim a whole-process memory
win.
