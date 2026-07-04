# M55 Phase 2R Poet V2 Evidence

Date: 2026-07-03

Verdict: no-go for default flip. The `YUNE-POET/2` redesign is implemented and
measured only behind `YUNE_POET_BYTE_BACKED=1`. It keeps the memory win but does
not satisfy the full M55 latency ratchet, so byte-backed poet consumption stays
default-off.

## Implementation Under Test

- Artifact magic is `YUNE-POET/2`.
- `YUNE-POET/1` artifacts are rejected by the parser.
- The compiled artifact adds fixed-width entry row ranges and a 32-byte
  hash-sorted prefix index.
- Byte-backed runtime no longer builds or reports a retained
  `poet.lookup_index`; the index evidence is `poet.prefix_index` as
  `mmap_file_backed`.
- Plain/default Luna remains on the owned sentence-model path unless
  `YUNE_POET_BYTE_BACKED=1` is set.

## Flag-On Ratchet

Evidence root:
`docs/reports/evidence/m55-native-match-or-beat/phase-2r-poet-v2/ratchet-gate-3-hash-prefix-index/`

Command:

```powershell
$env:YUNE_POET_BYTE_BACKED='1'
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 `
  -OutputRoot docs\reports\evidence\m55-native-match-or-beat\phase-2r-poet-v2\ratchet-gate-3-hash-prefix-index `
  -Iterations 9 -SessionIterations 60 -KeyIterations 80 `
  -TrackAInputs n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru `
  -TrackBInputs neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung `
  -DeployProductBeforeBenchmark `
  -TrackAThresholds docs\reports\evidence\m55-native-match-or-beat\thresholds\m55-thresholds.csv `
  -FailOnRegression
```

Result: exit 1, expected no-go.

Key rows from `threshold-check.csv`:

| Row | Observed | Ceiling | Status |
| --- | ---: | ---: | --- |
| `n` | `2.686x` | `3.050x` | pass |
| `ni` | `2.986x` | `3.223x` | pass |
| `hao` | `2.053x` | `2.287x` | pass |
| 37-char Luna | `5.395x` | `3.267x` | fail |
| 59-char Luna | `3.733x` | `2.447x` | fail |
| Track A peak working set | `113,090,560 B` | `198,000,000 B` | pass |
| Track B product long row | `378.274 us` | `375.253 us` | fail |

Memory-owner proof from `memory-owner-profile.csv`:

| Owner | Class | Estimated bytes | Items |
| --- | --- | ---: | ---: |
| `poet.entries_by_code` | `mmap_file_backed` | `3,048,137` | `70,805` |
| `poet.prefix_index` | `mmap_file_backed` | `2,875,064` | `84,852` |
| `poet.vocabulary` | `mmap_file_backed` | `25,492,848` | `421,966` |
| `poet.abbreviation_vocabulary` | `mmap_file_backed` | `713` | `11` |

Root cause: the v2 hash-prefix index removes the retained heap lookup index and
the v1-style prefix scan, but byte-backed long-row graph construction is still
far above the M55 ceilings. The remaining wall is not the named poet payload
residency; it is the byte-backed long-row graph/scoring constant factor.

## Default-Off Guard

Evidence root:
`docs/reports/evidence/m55-native-match-or-beat/phase-2r-poet-v2/default-off-m52-full-rerun-3/`

Command:

```powershell
Remove-Item Env:\YUNE_POET_BYTE_BACKED -ErrorAction SilentlyContinue
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 `
  -OutputRoot docs\reports\evidence\m55-native-match-or-beat\phase-2r-poet-v2\default-off-m52-full-rerun-3 `
  -Iterations 9 -SessionIterations 60 -KeyIterations 80 `
  -TrackAInputs n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru `
  -TrackBInputs neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung `
  -DeployProductBeforeBenchmark `
  -TrackAThresholds docs\reports\evidence\m52-track-a-guardrails-and-disposition\track-a-thresholds.csv `
  -FailOnRegression
```

Result: exit 0.

Key rows from `threshold-check.csv`:

| Row | Observed | Ceiling | Status |
| --- | ---: | ---: | --- |
| `n` | `2.725x` | `3.050x` | pass |
| `ni` | `2.989x` | `3.223x` | pass |
| `hao` | `2.068x` | `2.287x` | pass |
| 37-char Luna | `3.038x` | `3.267x` | pass |
| 59-char Luna | `2.272x` | `2.447x` | pass |
| Track A peak working set | `185,823,232 B` | `198,000,000 B` | pass |

`product_path_status.csv` confirms the default-off Luna row selected
`rsmarisa_byte_backed` table storage and did not consume the poet artifact.

## Product-Path Candidate Parity

Evidence:
`product-path-v2-candidate-parity-2026-07-03.md` and
`product-path-v2-candidate-parity-2026-07-03.json`.

The product-path comparison runs `target\debug\yune-cli.exe frontend` over
`apps\yune-web\public\schema` and compares default-off owned Luna against
flag-on `YUNE-POET/2`. The two long Luna benchmark rows plus `zhongguo`,
`jianli`, and `biancheng` match candidate text, comments, and trailing commit
text.

## Decision

Do not flip `YUNE_POET_BYTE_BACKED` default-on. Keep M52 as the standing native
Track A guard. Continue M55 only with a new, explicitly scoped design that can
move the long-row graph/scoring constant factor without widening ABI, changing
candidate output, or loosening thresholds.
