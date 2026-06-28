# Current Yune Root-Cause Dashboard

Date: 2026-06-28

This report keeps only the current root-cause read. Older milestone narratives,
WEB-01/WEB-02/WEB-03 closeout detail, and superseded measurements have been
archived at
[`history/2026-06-28-yune-vs-librime-root-cause-analysis-pre-current-dashboard.md`](./history/2026-06-28-yune-vs-librime-root-cause-analysis-pre-current-dashboard.md).

## Technical Summary

- **Current native latency owner**: `n` and `ni` remain the clear Track A
  latency misses. Both are short-prefix translator/prefix lookup constant-factor
  problems; they are not sentence-model or abbreviation-path regressions. The
  37-character pinyin row is a narrow watch row at `1.021x`.
- **Current native memory owner**: Track A resident rows are below the old
  resident target, but the `128.5 MB` peak working set is still real. Existing
  retained-owner rows do not explain a safe large reduction; the next owner is
  allocator/transient/private high-water attribution.
- **Current browser fair memory owner**: the fair `luna_pinyin` browser gap is
  now `64.0 MiB` Yune public demo versus `16.0 MiB` My RIME. This is the clean
  browser memory target because it uses the same schema and avoids Jyutping
  dictionary confounds.
- **Current Jyutping launch state**: the shipping public-demo Jyutping path is
  byte-backed at `160.0 MiB`, not the old `893.1 MiB` source-fallback shape.
  Long-input latency and phrase composition are guarded by WEB-03 follow-up
  tests.

## Current Gap Map

![Current remaining performance gaps](./evidence/current-performance-dashboard-2026-06-28/visuals/current-root-cause-gaps.svg)

| Area | Current root cause | Evidence | Current status |
| --- | --- | --- | --- |
| Native `n` | Short-prefix translator/prefix lookup constant factor | `71.500 us` vs librime `20.100 us`; `3.557x` | blocker |
| Native `ni` | Same short-prefix path | `50.300 us` vs librime `13.850 us`; `3.632x` | blocker |
| Native 37-character pinyin | Slight same-run miss on one long pinyin row | `292.373 us` vs librime `286.422 us`; `1.021x` | watch |
| Native Track A peak memory | High-water peak not explained by easy retained owners | Yune peak `128.5 MB`; librime max peer peak `18.3 MB` | blocker |
| Browser `luna_pinyin` memory | Yune WASM/runtime floor still larger than My RIME | `64.0 MiB` vs `16.0 MiB`; same schema | blocker |
| Browser `luna_pinyin` startup | Yune public-demo startup still slower | `1000 ms` vs My RIME `634 ms` | watch |
| Browser Jyutping | Larger TypeDuck profile; not a peer-comparable lane | Yune `160.0 MiB`, My RIME Jyutping `68.0 MiB` on different dictionary | guard only |

## Native Track A Cause

![Current native Track A latency ratios](./evidence/current-performance-dashboard-2026-06-28/visuals/current-native-latency-ratios.svg)

The current native latency problem is narrow. Startup, session create/select,
`zhongguo`, the 59-character pinyin row, and abbreviation rows are at or faster
than same-run upstream librime. The clear misses are the two short-prefix rows;
the 37-character pinyin row is a slight watch item:

| Row | Yune median | librime median | Ratio | Current cause |
| --- | ---: | ---: | ---: | --- |
| `n` | `71.500 us` | `20.100 us` | `3.557x` | short-prefix translator/prefix constant factor |
| `ni` | `50.300 us` | `13.850 us` | `3.632x` | same owner |
| `hao` | `25.233 us` | `11.400 us` | `2.213x` | slower, but not the dominant owner |
| 37-char pinyin | `292.373 us` | `286.422 us` | `1.021x` | narrow watch row |

The current evidence keeps the sentence paths out of this diagnosis. The
short-key owner counters show no upstream sentence model calls for the short
rows, and the 59-character and abbreviation rows remain green. The next native latency
diagnostic should isolate prefix lookup and translator dispatch cost without
widening the full-sentence or TypeDuck-profile behavior surfaces.

## Native Memory Cause

![Current memory high-water by lane](./evidence/current-performance-dashboard-2026-06-28/visuals/current-memory-peaks.svg)

Native memory is not solved because the peak remains high, even though steady
resident rows are lower:

| Measurement | Current value | Read |
| --- | ---: | --- |
| Yune Track A max peak working set | `128.5 MB` | standing high-water blocker |
| librime Track A max peer peak | `18.3 MB` | same-run peer scale |
| Yune highest Track A steady row | `98.7 MB` | lower than the peak, but not enough to reframe it away |
| Largest retained reducible row | `poet.entries_by_code`, `18.7 MB` | not enough to explain the process peak |

The current root cause is therefore not another broad structural-owner rewrite.
The next useful step is peak attribution: allocator behavior, transient buffers,
private bytes, mapped residency, and startup high-water timing.

## Browser Root Cause

![Current browser memory and payload](./evidence/current-performance-dashboard-2026-06-28/visuals/current-browser-memory-payload.svg)

The fair browser target is `luna_pinyin`, not Jyutping. Current browser peer
evidence shows:

| Scenario | Ready | Input -> candidate | Commit | WASM peak | Resource payload | Read |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| Yune public demo `luna_pinyin` | `1000 ms` | `74 ms` | `107 ms` | `64.0 MiB` | `29.5 MiB` | fair Yune row |
| My RIME live `luna_pinyin` | `634 ms` | `95 ms` | `119 ms` | `16.0 MiB` | `8.5 MiB` | fair peer row |

This changes the old browser-memory story. The current fair gap is `4.0x`, not
the earlier `10x` `160 MiB` Luna row. Yune's first-input and commit latencies
are competitive in this comparator, while startup and WASM memory remain the
browser-side blockers.

## Jyutping Guard State

Jyutping is currently a guard lane, not a peer lane. The Yune public demo ships
TypeDuck's larger multilingual `jyut6ping3` profile; My RIME's Jyutping row uses
a Cantonese-only dictionary. The current Yune state is still important because
it is the launch path:

| Guard | Current value | Read |
| --- | ---: | --- |
| Public-demo Jyutping WASM peak | `160.0 MiB` | byte-backed launch path; old `893.1 MiB` source-fallback row is historical |
| Ready-to-input | `1347 ms` | current public-demo comparator |
| First input -> candidate | `103 ms` | current public-demo comparator |
| Long row `sihaacoenggeoisyujapgecukdou` | `130 ms` exact keydown-to-paint | WEB-03 latency guard |
| Long row `taihaajyugwodaahoucoenggegeoizigosingnangwuidimjoeng` | `74 ms` exact keydown-to-paint | WEB-03 latency guard |

The root cause of the old `893.1 MiB` path was stale public-demo compiled
assets causing source fallback. That root cause is fixed on the shipping path.
It should stay in history, not in the current blocker list.

## Current Evidence

The dashboard evidence bundle is
[`evidence/current-performance-dashboard-2026-06-28/`](./evidence/current-performance-dashboard-2026-06-28/).

Key normalized tables:

- [`current-native-track-a.csv`](./evidence/current-performance-dashboard-2026-06-28/current-native-track-a.csv)
- [`current-browser-peer-comparator.csv`](./evidence/current-performance-dashboard-2026-06-28/current-browser-peer-comparator.csv)
- [`current-yune-browser-input-latency.csv`](./evidence/current-performance-dashboard-2026-06-28/current-yune-browser-input-latency.csv)
- [`current-root-cause-gaps.csv`](./evidence/current-performance-dashboard-2026-06-28/current-root-cause-gaps.csv)

## Next Diagnostic Order

| Rank | Work | Why this is next |
| ---: | --- | --- |
| 1 | Browser fair-lane memory floor on `luna_pinyin` | Same-schema browser gap is now the cleanest current memory target: `64.0 MiB` vs `16.0 MiB`. |
| 2 | Native peak attribution | Track A peak is still `128.5 MB`; existing retained-owner rows do not explain it. |
| 3 | Native short-prefix constant factor | `n` and `ni` are the only current native latency misses. |
| 4 | Browser startup phases | Yune public-demo `luna_pinyin` ready-to-input is `1000 ms` vs My RIME `634 ms`. |

## History

Archived milestone-style report:
[`history/2026-06-28-yune-vs-librime-root-cause-analysis-pre-current-dashboard.md`](./history/2026-06-28-yune-vs-librime-root-cause-analysis-pre-current-dashboard.md).
