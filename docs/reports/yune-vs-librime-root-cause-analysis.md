# Current Yune Root-Cause Dashboard

Date: 2026-07-04

This report keeps only the current root-cause read. Older milestone narratives,
WEB-01/WEB-02/WEB-03 closeout detail, and superseded measurements remain in
[`history/2026-06-28-yune-vs-librime-root-cause-analysis-pre-current-dashboard.md`](./history/2026-06-28-yune-vs-librime-root-cause-analysis-pre-current-dashboard.md).

The native lane was refreshed by the M55 final default-on ratchets on
2026-07-04. Browser rows are carried forward from the 2026-06-28 Playwright
run.

## Technical Summary

- **Current native guardrail owner**: M55 is now the standing native Track A
  regression gate. It extends M52 with the current win rows, Track B product
  absolutes, byte-backed memory ceiling, and Tier M caps for the short-key and
  long Luna rows.
- **Current native memory disposition**: validated `YUNE-POET/2` byte-backed
  poet storage is default-on. Final run 6 reports Track A peak
  `113,397,760 B`, under the tightened `118,831,104 B` ceiling and the `125 MB`
  Tier M bar.
- **Current native latency disposition**: the former long-row byte-backed
  access blocker is resolved by the Phase 3R lazy/graph-volume work. The final
  default-on gate reports 37-character Luna `0.237x` and 59-character Luna
  `0.086x`. The `n` row remains above librime at `1.794x`, but it is inside
  the Tier M `<=2.00x` bar.
- **Current startup/session owner**: repeated deployed YAML parsing during
  schema application was the final closeout instability. A metadata-validated
  runtime config cache reduces final startup/session medians to about `6.6 ms`
  for Track A without changing ABI or schema semantics.
- **Current browser fair memory owner**: the fair `luna_pinyin` browser gap is
  `64.0 MiB` Yune public demo versus `16.0 MiB` My RIME (carried 2026-06-28).

## Current Gap Map

| Area | Current root cause | Evidence | Current status |
| --- | --- | --- | --- |
| Native Track A standing guardrail | M55 final default-on ratchets are green twice | `phase-5-final/default-on-ratchet-5-config-cache/` and `default-on-ratchet-6-config-cache/` | standing gate |
| Native Track A memory | Full Luna poet payload is now file-backed; remaining gap is process/runtime scale vs librime | final peak `113,397,760 B`; poet owners recorded as `mmap_file_backed` in M55 diagnostics | Tier M pass; not memory parity |
| Native `n` short-key | The exact alias path removes the prefix trie scan, but the row still carries a small absolute gap | `36.600 us` vs `20.400 us`, `1.794x` | Tier M pass; future optional owner |
| Native long-row latency | Lazy native ABI refresh plus graph-volume reductions remove the previous long-row wall | 37-char `66.665 us` vs `281.559 us`; 59-char `55.641 us` vs `644.025 us` | resolved for M55 |
| Track B product guard | TypeDuck product long-row absolute guard remains green after M55 | `316.282 us` observed vs `375.253 us` ceiling | regression guard pass |
| Browser `luna_pinyin` memory | Yune WASM/runtime floor still larger than My RIME | `64.0 MiB` vs `16.0 MiB`; same schema (carried) | blocker |
| Browser `luna_pinyin` startup | Yune public-demo startup still slower | `1000 ms` vs My RIME `634 ms` (carried) | watch |
| Browser Jyutping | Larger TypeDuck profile; not a peer-comparable lane | Yune `160.0 MiB`, My RIME Jyutping `68.0 MiB` on different dictionary (carried) | guard only |

![Current performance gaps by lane](./evidence/dashboard-visuals-2026-06-30/root-cause-gaps.svg)

## Native Track A Cause

M55 ended up with four landed native owners:

- `YUNE-POET/2` byte-backed poet storage removes the large retained Luna poet
  payload while keeping validated artifacts stale-rejecting and default-on.
- Native ABI lazy refresh buffers unobserved exact `luna_pinyin` key sequences
  and flushes before observable C ABI boundaries, removing repeated long-row
  rebuild work from the deployed key path.
- The short-key exact alias path uses the evidenced `n -> na` / `h -> ha`
  aliases instead of scanning the prefix trie for those single-letter rows.
- The runtime config cache reuses parsed deployed YAML when the file metadata is
  unchanged, removing repeated schema-load work from startup/session gates.

Current native latency rows from final M55 run 6:

| Row | Yune median | librime median | Ratio | Current cause |
| --- | ---: | ---: | ---: | --- |
| `n` | `36.600 us` | `20.400 us` | `1.794x` | Tier M pass; small short-key gap remains |
| `ni` | `14.650 us` | `14.100 us` | `1.039x` | Tier M pass |
| `hao` | `9.267 us` | `11.367 us` | `0.815x` | faster in this gate |
| 37-char pinyin | `66.665 us` | `281.559 us` | `0.237x` | long-row wall resolved for native gate |
| 59-char pinyin | `55.641 us` | `644.025 us` | `0.086x` | long-row wall resolved for native gate |
| `zhongguo` (common word) | `6.038 us` | `158.338 us` | `0.038x` | faster; win-row guard pass |
| `cszysmsrsd` (10-char abbr) | `103.950 us` | `1,159.300 us` | `0.090x` | faster; win-row guard pass |
| `zybfshmsru` (8-char abbr) | `100.700 us` | `817.200 us` | `0.123x` | faster; win-row guard pass |

M55 therefore supports a bounded native Track A performance claim, not an
unqualified "Yune is faster than librime" claim. Rows below `1.00x` are
match-or-beat rows; `n` remains a bounded-gap pass.

![Native Track A latency across all input dimensions, Yune vs librime 1.17.0](./evidence/dashboard-visuals-2026-06-30/native-track-a-latency-ratios.svg)

## Native Memory Cause

Native Track A memory now satisfies M55 Tier M but remains larger than librime's
peer process:

| Measurement | Current value | Read |
| --- | ---: | --- |
| M55 final Track A max peak working set | `113.4 MB` | final default-on ratchet; memory guard pass |
| M55 Track A ceiling | `118.8 MB` | stricter than the `125 MB` Tier M bar |
| M52 Track A max peak working set | `188.4 MB` | previous standing dashboard value |
| M52 librime Track A max peer peak | `17.3 MB` | same-run peer scale from old M52 guard |
| `poet.vocabulary` | `25.5 MB` | `mmap_file_backed`, `poet_bin:mmap` diagnostic owner |
| `poet.entries_by_code` | `3.0 MB` | `mmap_file_backed`, `poet_bin:mmap` diagnostic owner |

This does not invalidate M47. M47's comments-intact `jyut6ping3_mobile`
keyboard profile remains the separate iOS-target lane and reports about
`22 MB` private in the lean native probe. The M55 values are the full
`luna_pinyin` Track A peer-comparison harness after M48 loaded the upstream
preset vocabulary.

![Native Track A memory peak and named owners](./evidence/dashboard-visuals-2026-06-30/native-track-a-memory.svg)

## Native Track B Cause (product lane)

Track B is the native TypeDuck `jyut6ping3` product path and is a separate lane
from Track A: it has no librime peer, so the read is absolute cost plus the M47
byte-backing trajectory. M55 uses Track B only as a regression guard. The final
M55 run 6 Track B rows are green:

| Row | Observed | Ceiling | Status |
| --- | ---: | ---: | --- |
| 50+ key-sequence latency | `316.282 us` | `375.253 us` | pass |
| key-sequence median working set | `80,150,528 B` | `280,518,656 B` | pass |
| key-sequence max peak working set | `511,053,824 B` | `564,065,075 B` | pass |
| key-sequence median private bytes | `36,982,784 B` | `200,620,851 B` | pass |

No TypeDuck-vs-librime speed claim follows from this guard.

![Native Track B memory, TypeDuck jyut6ping3 product path](./evidence/dashboard-visuals-2026-06-30/native-track-b-memory.svg)

## Browser Root Cause

Carried forward from the 2026-06-28 Playwright run.

The fair browser target is `luna_pinyin`, not Jyutping:

| Scenario | Ready | Input -> candidate | Commit | WASM peak | Resource payload | Read |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| Yune public demo `luna_pinyin` | `1000 ms` | `74 ms` | `107 ms` | `64.0 MiB` | `29.5 MiB` | fair Yune row |
| My RIME live `luna_pinyin` | `634 ms` | `95 ms` | `119 ms` | `16.0 MiB` | `8.5 MiB` | fair peer row |

The fair browser gap remains `4.0x`; startup and WASM memory are the
browser-side blockers. Jyutping remains a launch guard lane, not a peer lane,
because the dictionary families differ.

![Browser memory and payload by lane](./evidence/current-performance-dashboard-2026-06-29/visuals/current-browser-memory-payload.svg)

## Current Evidence

Key M55 tables:

- [`phase-5-final/default-on-ratchet-6-config-cache/summary-comparison.csv`](./evidence/m55-native-match-or-beat/phase-5-final/default-on-ratchet-6-config-cache/summary-comparison.csv)
- [`phase-5-final/default-on-ratchet-6-config-cache/threshold-check.csv`](./evidence/m55-native-match-or-beat/phase-5-final/default-on-ratchet-6-config-cache/threshold-check.csv)
- [`phase-5-final/default-on-ratchet-5-config-cache/threshold-check.csv`](./evidence/m55-native-match-or-beat/phase-5-final/default-on-ratchet-5-config-cache/threshold-check.csv)
- [`phase-5-final/closeout-2026-07-04.md`](./evidence/m55-native-match-or-beat/phase-5-final/closeout-2026-07-04.md)
- [`thresholds/m55-thresholds.csv`](./evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv)

Browser evidence remains under
[`current-performance-dashboard-2026-06-29/`](./evidence/current-performance-dashboard-2026-06-29/).

## Next Diagnostic Order

| Rank | Work | Why this is next |
| ---: | --- | --- |
| 1 | Browser fair-lane memory floor on `luna_pinyin` | Same-schema browser gap is `64.0 MiB` vs `16.0 MiB`. |
| 2 | Browser startup phases | Yune public-demo `luna_pinyin` ready-to-input is `1000 ms` vs My RIME `634 ms`. |
| 3 | Optional native `n` short-key micro-owner | `n` passes Tier M but remains above librime by about `16 us`. |
| 4 | Native Track A memory peer gap | M55 hits the 125 MB bar but is still much larger than the librime peer process. |

## History

Archived milestone-style report:
[`history/2026-06-28-yune-vs-librime-root-cause-analysis-pre-current-dashboard.md`](./history/2026-06-28-yune-vs-librime-root-cause-analysis-pre-current-dashboard.md).
