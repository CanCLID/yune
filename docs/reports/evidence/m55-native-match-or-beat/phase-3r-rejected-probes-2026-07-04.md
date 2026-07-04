# M55 Phase 3R Rejected Owned-Path Probes

Date: 2026-07-04

Verdict: no new green checkpoint. All code and generated raw evidence from
these local probes was reverted or removed. The current best committed owned
path remains
`docs/reports/evidence/m55-native-match-or-beat/phase-3r-sentence-path-cache/`.

Baseline for comparison: the `phase-3r-sentence-path-cache` same-run M55
ratchet recorded 37-char `2.001x`, 59-char `1.519x`, `n` `2.667x`, `ni`
`2.979x`, `hao` `2.026x`, Track A peak `185,520,128 B`, and Track B product
key-sequence `313.461 us`.

## Rejected Probes

| probe | check run | result |
| --- | --- | --- |
| Cached vocabulary first-code ranges on cached exact spans | Product-path candidate parity was green. Strict M55 ratchet passed the broad ceilings. | Rejected because target ratios worsened: 37-char `2.038x`; 59-char `1.579x`. |
| Direct candidate extraction from scratch `states_by_end` | Focused scratch tests and product-path candidate parity were green. Strict M55 ratchet passed the broad ceilings. | Rejected because raw Yune medians and ratios worsened on the long rows: 37-char `585.419 us` / `2.051x`; 59-char `1100.088 us` / `1.685x`. |
| Null-grammar duplicate-state precheck before `PathState` construction | Focused scratch tests passed; debug access-volume capture was run. | Rejected before release ratchet because constructed DP states and beam evictions were unchanged: 37-char `5,060` / `1,700`; 59-char `10,853` / `4,494`. |
| Full-rank null-grammar beam precheck before `PathState` construction | Focused scratch tests passed; debug access-volume capture was run. | Rejected before release ratchet because constructed DP states and beam evictions were unchanged: 37-char `5,060` / `1,700`; 59-char `10,853` / `4,494`. |
| Moving cached exact-span vectors out of scratch instead of cloning them | Product-path candidate parity was green. A strict-ratchet attempt was discarded because its Track A list accidentally included an extra typo input; the target rows from that discarded run were still inspected. | Rejected because the inspected target rows worsened: 37-char `591.157 us` / `2.082x`; 59-char `1030.937 us` / `1.580x`; short rows also worsened. |
| Smaller `PathWordLengths` inline capacity | Capacity `8` and `12` short probes were promising, so capacity `12` received a correct full strict M55 ratchet. Product-path candidate parity was green and the broad ceilings passed. | Rejected because the correct strict run still worsened long-row ratios versus the baseline: 37-char `585.938 us` / `2.052x`; 59-char `1029.234 us` / `1.590x`. |
| Byte-indexed vector storage for the temporary borrowed incremental graph | Focused scratch tests passed and a correct full strict M55 ratchet passed the broad ceilings. | Rejected because it was not monotonic on the target rows: 37-char improved to `574.900 us` / `1.991x`, but 59-char worsened to `1059.812 us` / `1.612x`; short rows also worsened. |
| Final-end-only null-grammar candidate emission | A small same-run release probe was run after focused scratch tests had passed. | Rejected because it was not monotonic versus the baseline: 59-char improved to `1026.242 us` / `1.557x`, but 37-char worsened to `582.586 us` / `2.055x`; `n`/`ni`/`hao` also worsened to `2.812x` / `3.024x` / `2.080x`. |
| Iterate non-empty sentence-end buckets during candidate merge | Focused sentence scratch tests and expanded Luna green-row parity passed. A small same-run release probe was run. | Rejected before strict ratchet because it was not monotonic versus the current best: 59-char improved to `1046.441 us` / `1.552x`, but 37-char worsened to `596.543 us` / `2.032x`; short rows also remained above Tier M. |
| Avoid candidate/key clones during cached sentence-path merge | Product-path candidate parity was green. Focused sentence scratch tests, expanded Luna green-row parity, `cargo fmt --check`, `cargo clippy -p yune-core --all-targets -- -D warnings`, and a full strict M55 ratchet passed the broad ceilings. A repeat 5/40/60 probe checked stability. | Rejected because it was mixed and not a clean replacement for the current best: the full strict run improved 37-char to `565.259 us` / `1.970x` but worsened 59-char to `1058.349 us` / `1.621x`; the repeat probe was also worse than the current best at 37-char `584.446 us` / `2.027x` and 59-char `1028.647 us` / `1.563x`. |
| Retain a merged sentence-candidate map in scratch | Product-path candidate parity was green. Focused sentence scratch tests, expanded Luna green-row parity, `cargo fmt --check`, and `cargo clippy -p yune-core --all-targets -- -D warnings` passed. A small same-run release probe was run. | Rejected before strict ratchet because it was worse than the current best across the target rows and short keys: 37-char `598.986 us` / `2.074x`, 59-char `1072.942 us` / `1.624x`, `n`/`ni`/`hao` `2.854x` / `3.071x` / `2.168x`. |
| Single-pass owned vocabulary phrase derivation | Focused upstream sentence-model tests and expanded Luna green-row parity passed. A 5/40/60 same-run release probe used the exact M55 Track A and Track B input sets. | Rejected because deriving phrase codes without the cheap precheck made non-matching vocabulary entries expensive and severely regressed the long rows: 37-char `2337.968 us` / `7.739x`; 59-char `4720.827 us` / `7.018x`. |
| Transient per-composition vocabulary frontier | Focused upstream sentence-model tests and expanded Luna green-row parity passed. A fixture per-key diagnostic and a 5/40/60 same-run release probe used the exact M55 Track A and Track B input sets. | Rejected because the additional frontier bookkeeping outweighed any row filtering and regressed the long rows: 37-char `840.857 us` / `2.752x`; 59-char `1591.685 us` / `2.222x`. |
| Transient normalized-weight cache | Focused upstream sentence-model tests and expanded Luna green-row parity passed. A 5/40/60 same-run release probe used the exact M55 Track A and Track B input sets. | Rejected because the linear cache lookup cost outweighed repeated `ln` avoidance and worsened both target rows: 37-char `588.311 us` / `2.058x`; 59-char `1012.225 us` / `1.551x`. |
| Incremental `PathState` text hash precheck | Focused upstream sentence-model tests and expanded Luna green-row parity passed. A 5/40/60 same-run release probe used the exact M55 Track A and Track B input sets. | Rejected because added hash maintenance helped abbreviation win rows but worsened both target long rows: 37-char `595.595 us` / `2.096x`; 59-char `1049.936 us` / `1.607x`. |

## Decision

These probes do not justify a checkpoint commit. Future Phase 3R work should
look for a new measured owner instead of repeating cached vocabulary lookup,
scratch candidate extraction, duplicate prechecks, full-rank beam prechecks,
cached-span vector moves, simple `PathWordLengths` capacity tuning, or
byte-indexed temporary borrowed graph storage, final-end-only candidate
emission, sentence-end bucket-only candidate merge iteration, simple
candidate/key clone avoidance in the cached sentence-path merge, or retaining
a merged sentence-candidate map in scratch, or replacing owned vocabulary
precheck-plus-derive with single-pass derivation, or adding a transient
per-composition vocabulary frontier, or adding a transient normalized-weight
cache to the incremental scratch, or adding an incremental text hash to
`PathState`.
