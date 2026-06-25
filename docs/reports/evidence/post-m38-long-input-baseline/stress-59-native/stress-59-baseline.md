# Post-M38 59-Character Long-Input Stress Baseline

Date: 2026-06-25

This is a controlled one-iteration native in-process stress run for the
59-character continuous pinyin input:

```text
zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong
```

It supplements the higher-sample baseline in
[`../baseline-native/`](../baseline-native/). The one-iteration shape is
intentional: the Yune row is already about `70.942 s` for one full input sample.

Command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\post-m38-long-input-baseline\stress-59-native -Iterations 1 -SessionIterations 1 -KeyIterations 1 -TrackAInputs "zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong" -DeployProductBeforeBenchmark
```

## Track A Latency

| Row | Yune median per key | librime median per key | Ratio | Full-input Yune | Full-input librime |
| --- | ---: | ---: | ---: | ---: | ---: |
| `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` | `1,202,404.588 us` | `702.212 us` | `1,712.310x` | `70.942 s` | `41.431 ms` |

## Track A Memory

| Row | Yune median working set | librime median working set | Ratio | Yune max peak | librime max peak | Ratio |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` | `114,728,960 B` | `15,884,288 B` | `7.22x` | `163,119,104 B` | `16,154,624 B` | `10.10x` |

## Owner Signals

| Signal | Value |
| --- | ---: |
| selected storage | `rsmarisa_byte_backed` |
| table mapping | `mmap` |
| prism mapping | `mmap` |
| raw prism median | `0.100 us` |
| raw table median | `57.500 us` |
| raw table candidates | `0` |
| translator median | `1,202,364.464 us` |
| context export median | `6.400 us` |
| process key total sample | `70.942 s` |
| translator share of process-key sample | `99.997%` |
| exact lookup calls | `111` |
| prefix lookup calls | `110` |
| rsmarisa exact lookup calls | `111` |
| rsmarisa prefix lookup calls | `110` |
| full-list fallback count | `52` |
| candidate request bounded calls | `59` |
| candidate request unbounded calls | `0` |
| owned candidates materialized | `115` |
| lookup views visited | `295` |

## Read

The stress row confirms that 50+ character uninterrupted input must be a hard
engine-performance gate. It is not a marisa activation failure and not a raw
lookup/context/export owner. The measured outer owner is still translator time,
with a strong signal that long-composition fallback is being exercised on most
keystrokes.

Do not overfit the exact complexity curve from this single low-sample run. M39
should add inner sentence/composition spans and a length-curve benchmark before
claiming the final algorithmic diagnosis.
