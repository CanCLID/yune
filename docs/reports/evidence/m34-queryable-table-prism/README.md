# M34 queryable table/prism evidence

Date: 2026-06-23

M34 closed as a bounded lazy-candidate-pipeline slice plus the internal lookup
abstraction needed for later table/prism storage work. It did not land a
compiled table storage swap, prism-backed candidate lookup, mmap, browser
runtime change, or public-demo delivery work.

## Evidence files

- `baseline-yune-vs-librime/` - fresh fair cross-engine M33 baseline rerun.
- `after-yune-vs-librime/` - final fair cross-engine M34 after-run.
- `frontend-baselines-before.txt` - native `frontend_baselines` before log.
- `frontend-baselines-after-final.txt` - final native `frontend_baselines` log.
- `m34-cross-engine-gap.svg` - final fair Yune/librime latency gap chart.
- `m34-native-improvement.svg` - native before/after watched-row chart.
- `m34-working-set-gap.svg` - final fair peak working-set gap chart.
- `baseline.md` - commands and core before/after numbers.
- `hot-path-attribution.md` - owner split and lever decision.
- `hot-path-attribution-diagnostic.txt` - temporary test-only `ni`/`hao`
  diagnostic output; no diagnostic helper is retained in production code.
- `reader-audit.md` - full-list reader/caller audit.
- `my-rime-reference.md` - local `my_rime` reference split.
- `bounded-translation-contract.md` - bounded translator contract evidence.
- `bounded-engine-refresh.md` - engine lazy-window evidence.
- `table-lookup-abstraction.md` - heap-backed lookup abstraction evidence.
- `compiled-table-query.md` - compiled table storage stop gate.
- `prism-table-integration.md` - prism/table integration stop gate.
- `storage-hot-path-swap.md` - storage swap decision.
- `mmap-spike.md` - mmap/borrowed-storage deferral.
- `task-8-gates.md` - verification gate record.

## Headline native rows

| Row | Before median | After median | Change |
| --- | ---: | ---: | ---: |
| `per_key_real_luna_pinyin_ni_full_abi` | `1,760.250 us` | `1,132.950 us` | `-35.6%` |
| `per_key_real_luna_pinyin_zhongguo_full_abi` | `12,697.600 us` | `12,119.013 us` | `-4.6%` |
| `per_key_real_jyut6ping3_mobile_hai_full_abi` | `18,389.567 us` | `19,446.467 us` | `+5.7%` |
| `per_key_real_jyut6ping3_mobile_jigaajiusihaa_full_abi` | `29,937.777 us` | `28,155.585 us` | `-6.0%` |
| `per_key_real_jyut6ping3_mobile_jigaajiusihaa_correction_full_abi` | `29,649.146 us` | `28,032.915 us` | `-5.5%` |

`per_key_real_luna_pinyin_hao_*` was added as a native benchmark row during
M34, so it has no same-harness before row. The final after medians are
`1,378.800 us` full ABI and `761.667 us` engine-only.

Engine-only before/after rows are attribution-only for M34 because the benchmark
now calls `Engine::set_schema(...)` for real-schema engine-only runs. That makes
the engine-only surface more realistic and lets `luna_pinyin` exercise the M34
schema-gated bounded path, but it means old engine-only rows are not the public
no-regression surface. Full-ABI and cross-engine rows remain the public compare.

## Headline cross-engine rows

| Workload | Yune baseline | Yune after | librime after |
| --- | ---: | ---: | ---: |
| `hao` key sequence median | `13,336.800 us` | `12,216.900 us` | `35.100 us` |
| `ni` key sequence median | `5,858.800 us` | `5,693.900 us` | `28.700 us` |
| `zhongguo` key sequence median | `36,451.100 us` | `35,909.100 us` | `1,379.400 us` |
| session create/select/destroy | `48,329.000 us` | `46,743.400 us` | `28,121.800 us` |
| warm startup/runtime-ready | `50,065.200 us` | `47,126.800 us` | `30,315.200 us` |

The safe public claim is narrow: M34 reduces bounded native first-page work for
short `luna_pinyin` inputs and keeps fair public numbers updated. Yune still
trails librime widely on per-key rows and memory footprint.
