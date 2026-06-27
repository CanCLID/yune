# M45 Phase 0 Verdict

Date: 2026-06-27

M45 Phase 0 captured a fresh same-run native benchmark, upstream
`luna_pinyin` short-key candidate output for `n`, `ni`, and `hao`, and memory
columns that split working set from private bytes and process high-water
signals.

## Short-Key Verdict

Verdict: `short-key-measured-no-go`.

The Phase 0 candidate-output guard passed for all three short-key rows:
candidate text, comments, order, preedit, commit preview, and page metadata
match upstream librime `1.17.0`. The final comparison keeps that guard passing.

Final benchmark evidence still misses the `<=3.0x` target for `n` and `ni`:

| Row | Yune median | librime median | Ratio | M45 result |
| --- | ---: | ---: | ---: | --- |
| `n` | `68.900 us` | `20.800 us` | `3.313x` | Miss; measured blocker |
| `ni` | `49.450 us` | `14.300 us` | `3.458x` | Miss; measured blocker |
| `hao` | `24.267 us` | `11.500 us` | `2.110x` | Pass |

Owner counters show `upstream_sentence_model_calls=0` for `n`, `ni`, and
`hao`, so the residual short-key blocker is not the M40/M42 sentence path. The
remaining owner is the short-prefix translator/prefix lookup constant factor;
no bounded code branch moved the target enough to retain.

## Memory Verdict

Verdict: `steady-state-meets-target-standing-peak-cost`.

Track A steady after-ready resident rows are below `107,797,708 B`, but the
first startup sample reaches the same `127 MB`-class high-water peak as the key
rows. That makes the peak a real per-cold-start/deploy cost rather than a value
M45 can reframe away as a pure benchmark-cumulative artifact.

| Measurement | Final value | Verdict |
| --- | ---: | --- |
| Startup after-ready working set | `90,161,152 B` | Below resident target |
| `n` median working set | `91,058,176 B` | Below resident target |
| `ni` median working set | `91,754,496 B` | Below resident target |
| `hao` median working set | `92,749,824 B` | Below resident target |
| Track A max peak working set | `127,475,712 B` | Standing peak-cost blocker |
| Track A max peak pagefile | `112,218,112 B` | Standing peak-cost blocker |

The retained owner profile still names `poet.entries_by_code` as the largest
heap-owned reducible owner (`18,694,662 B`) and `compact_table.storage` as
file-backed mmap bytes (`13,013,460 B`). No new bounded owner was proven likely
to move the peak in M45, so no storage rewrite was retained.
