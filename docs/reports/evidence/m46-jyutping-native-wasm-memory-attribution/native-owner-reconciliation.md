# M46 Phase 0 Native Owner Reconciliation

Evidence root:
[`phase-0-native/`](./phase-0-native/)

## Track B Baseline

Fresh Track B product rows were captured with `source_fallback=false` and the
deployed product path.

| Row | Samples | Median us | p95 us | Median working set | Peak working set |
| --- | ---: | ---: | ---: | ---: | ---: |
| `h` | 80 | `1767.200` | `1785.900` | `441,155,584 B` | `504,627,200 B` |
| `ha` | 80 | `1198.400` | `1206.200` | `441,958,400 B` | `504,627,200 B` |
| `hai` | 80 | `813.767` | `839.767` | `441,950,208 B` | `504,627,200 B` |
| `hau` | 80 | `822.200` | `1002.633` | `441,966,592 B` | `504,627,200 B` |
| `nei` | 80 | `399.367` | `473.100` | `441,982,976 B` | `504,627,200 B` |
| `ngo` | 80 | `600.533` | `604.867` | `442,011,648 B` | `504,627,200 B` |
| 50+ guard | 80 | `33.480` | `33.787` | `442,966,016 B` | `504,627,200 B` |
| session create/select/destroy | 60 | `144658.800` | `183366.200` | `427,356,160 B` | `504,627,200 B` |
| startup/runtime-ready | 9 | `124019.300` | `929430.900` | `436,989,952 B` | `504,627,200 B` |

The native memory headline is unchanged in shape from M45: Track B still has a
real `~504 MB` peak and `~427-443 MB` steady resident rows.

## Product Path Status

| Schema | Dictionary | Source fallback | Storage | Table mapping | Prism mapping | Source bytes | Table heap mirror | Prism heap mirror | rsmarisa |
| --- | --- | --- | --- | --- | --- | ---: | ---: | ---: | --- |
| `jyut6ping3_mobile` | `jyut6ping3` | `false` | `byte_backed` | `mmap` | `mmap` | `15,248,382` | `0` | `0` | `missing_string_table` |
| `jyut6ping3_mobile` | `jyut6ping3_scolar` | `false` | `byte_backed` | `mmap` | `mmap` | `27,325,622` | `0` | `0` | `missing_string_table` |

These are source/status byte lengths. They are not retained-owner rows unless
the owner profile also names them as retained process memory.

## Owner Rows

The owner instrumentation adds required, overlapping, transient, and
unclassified classes to separate structural estimates from process-memory
proxies.

| Owner | Class | Retained estimate | Count | Read |
| --- | --- | ---: | ---: | --- |
| `compact_table.lookup_records` | `heap_owned_required` | `31,920,140 B` | 127,144 | Largest named required heap owner; dictionary-panel data. |
| `compact_table.storage` | `mmap_file_backed` | `15,248,382 B` | 241,796 | Base Jyutping selected table bytes, file-backed. |
| `translator.entries_by_code` | `heap_owned_guarded` | `8,327,700 B` | 48,970 | Guarded/source-YAML or small-test state; not the selected compact product table. |
| `compact_table.syllabary_codes` | `heap_owned_reducible` | `4,189,674 B` | 114,653 | Code-string owner; too small to explain the headline. |
| `compact_table.candidate_comment_payload` | `shared_or_overlapping` | `1,516,569 B` | 127,143 | Logical bytes overlapping `compact_table.storage`. |
| `compact_table.candidate_text_payload` | `shared_or_overlapping` | `1,058,723 B` | 127,143 | Logical bytes overlapping `compact_table.storage`. |
| `runtime.session_state` | `heap_owned_guarded` | `3,133 B` | 1 | Session state. |
| `schema.processors` | `heap_owned_guarded` | `2,327 B` | 2 | Processor state. |
| `schema.config` | `overlap_estimate` | `2,040 B` | 1 | Logical schema overlap. |

Class totals excluding process proxy rows:

| Class | Retained estimate |
| --- | ---: |
| `heap_owned_required` | `31,920,212 B` |
| `mmap_file_backed` | `15,248,382 B` |
| `heap_owned_guarded` | `8,333,184 B` |
| `heap_owned_reducible` | `4,189,674 B` |
| `shared_or_overlapping` | `2,575,292 B` |
| `overlap_estimate` | `2,040 B` |

Process proxy rows:

| Proxy | Value | Read |
| --- | ---: | --- |
| `process.peak_working_set_high_water` | `504,627,200 B` | Observed native peak working set. |
| `process.after_ready_working_set_unclassified_lower_bound` | `437,776,918 B` | Median after-ready working set minus non-overlapping reducible owners only. |
| `process.private_bytes_proxy` | `423,641,088 B` | Median private bytes proxy. |
| `process.peak_pagefile_high_water` | `535,744,512 B` | Peak pagefile proxy. |

The profile now names more native owners, but the headline remains mostly
unclassified. The concrete named selected rows (`required + guarded +
reducible + mmap`) total `59,691,452 B`, about `11.8%` of the peak working set.
Even if all of those rows were perfectly non-overlapping, the gap to the
`504,627,200 B` peak is still about `444,935,748 B`.

## M37 Metric Schema

No new `m37_metrics.csv` columns were added in Phase 0. The new native data is
exported through `memory-owner-profile.csv`, which has its own owner-row schema.
Therefore `M37_METRIC_FIELDS` did not require an update for this evidence.

## Read

- A Track B `rsmarisa` string-table spike is not authorized by this evidence:
  the code-string owner is `4,189,674 B`, not a `~200 MB` owner.
- Candidate text/comment payload rows are visible but overlapping and small
  (`2,575,292 B` logical combined). They are not a headline memory branch yet.
- `compact_table.lookup_records` is the largest newly named native owner, but
  it is required for dictionary-panel behavior and still only `31,920,140 B`.
- `jyut6ping3_scolar` remains a status/source-byte concern, not a retained
  owner row in this native profile.
- The dominant native memory blocker is still process-level unclassified
  resident/high-water memory, not a single safe structural owner.
