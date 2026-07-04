# M55 Phase 3R Rejected Owned-Path Probes

Date: 2026-07-04

Verdict: no new green checkpoint. All code and generated raw evidence from
these local probes was reverted or removed. The current best committed owned
path remains
`docs/reports/evidence/m55-native-match-or-beat/phase-3r-incremental-prefix-state/`.

Baseline for comparison: the `phase-3r-incremental-prefix-state` same-run M55
ratchet recorded 37-char `2.010x`, 59-char `1.570x`, `n` `2.708x`, `ni`
`2.952x`, `hao` `2.039x`, Track A peak `186,085,376 B`, and Track B product
key-sequence `324.395 us`.

## Rejected Probes

| probe | check run | result |
| --- | --- | --- |
| Cached vocabulary first-code ranges on cached exact spans | Product-path candidate parity was green. Strict M55 ratchet passed the broad ceilings. | Rejected because target ratios worsened: 37-char `2.038x`; 59-char `1.579x`. |
| Direct candidate extraction from scratch `states_by_end` | Focused scratch tests and product-path candidate parity were green. Strict M55 ratchet passed the broad ceilings. | Rejected because raw Yune medians and ratios worsened on the long rows: 37-char `585.419 us` / `2.051x`; 59-char `1100.088 us` / `1.685x`. |
| Null-grammar duplicate-state precheck before `PathState` construction | Focused scratch tests passed; debug access-volume capture was run. | Rejected before release ratchet because constructed DP states and beam evictions were unchanged: 37-char `5,060` / `1,700`; 59-char `10,853` / `4,494`. |
| Full-rank null-grammar beam precheck before `PathState` construction | Focused scratch tests passed; debug access-volume capture was run. | Rejected before release ratchet because constructed DP states and beam evictions were unchanged: 37-char `5,060` / `1,700`; 59-char `10,853` / `4,494`. |
| Moving cached exact-span vectors out of scratch instead of cloning them | Product-path candidate parity was green. A strict-ratchet attempt was discarded because its Track A list accidentally included an extra typo input; the target rows from that discarded run were still inspected. | Rejected because the inspected target rows worsened: 37-char `591.157 us` / `2.082x`; 59-char `1030.937 us` / `1.580x`; short rows also worsened. |
| Smaller `PathWordLengths` inline capacity | Capacity `8` and `12` short probes were promising, so capacity `12` received a correct full strict M55 ratchet. Product-path candidate parity was green and the broad ceilings passed. | Rejected because the correct strict run still worsened long-row ratios versus the baseline: 37-char `585.938 us` / `2.052x`; 59-char `1029.234 us` / `1.590x`. |

## Decision

These probes do not justify a checkpoint commit. Future Phase 3R work should
look for a new measured owner instead of repeating cached vocabulary lookup,
scratch candidate extraction, duplicate prechecks, full-rank beam prechecks,
cached-span vector moves, or simple `PathWordLengths` capacity tuning.
