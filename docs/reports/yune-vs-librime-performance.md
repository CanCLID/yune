# Yune vs upstream librime performance report

Date: 2026-06-23

Evidence:

- M33 before: [`evidence/m33-2026-06-23/before-yune-vs-librime/`](./evidence/m33-2026-06-23/before-yune-vs-librime/)
- M33 after low-risk slice: [`evidence/m33-2026-06-23/after-low-risk-yune-vs-librime/`](./evidence/m33-2026-06-23/after-low-risk-yune-vs-librime/)
- Native Criterion logs: [`evidence/m33-2026-06-23/frontend-baselines-before.txt`](./evidence/m33-2026-06-23/frontend-baselines-before.txt) and [`evidence/m33-2026-06-23/frontend-baselines-after-low-risk.txt`](./evidence/m33-2026-06-23/frontend-baselines-after-low-risk.txt)

## Public summary

M33 corrects the earlier unfair `luna_pinyin` comparison. Yune now lazy-loads the
`stroke` reverse-lookup dictionary, matching upstream librime's behavior for the
timed no-reverse-lookup rows. The final public comparison can safely show the
startup/session improvement, but it must also show that per-key lookup still
trails librime by a wide margin.

Final M33 result on the shared upstream `luna_pinyin` C-ABI workload:

- Startup/runtime-ready median: Yune `47,788.2 us`; librime `27,628.3 us`; Yune is `1.7x` slower.
- Session create/select/destroy median: Yune `47,813.7 us`; librime `25,765.9 us`; Yune is `1.9x` slower.
- Startup ready working-set delta: Yune `24,576 bytes`; librime `847,872 bytes`.
- Key processing still trails: `ni` `212.8x`, `hao` `361.3x`, and `zhongguo` `25.4x` slower than librime.

Against the M33 before run, Yune startup dropped from `2,881,852.7 us` to
`47,788.2 us` (`-98.3%`) and session create/select/destroy dropped from
`2,985,364.0 us` to `47,813.7 us` (`-98.4%`). Per-key Yune rows regressed in
this run: `ni` `+8.7%`, `hao` `+12.9%`, and `zhongguo` `+10.4%`. Those
regressions are small relative to the startup win but are not a win claim.

No browser startup, browser typing, WASM, React, or TypeDuck-Web UI result is
claimed from this benchmark. No chart SVG was regenerated for M33; a chart is
safe to publish only if it shows both the startup/session win and the unresolved
per-key gap.

## Methodology

Both engines were measured through the same librime-shaped C API harness:
[`../../scripts/yune-vs-librime-benchmark.cs`](../../scripts/yune-vs-librime-benchmark.cs),
driven by [`../../scripts/benchmark-yune-vs-librime.ps1`](../../scripts/benchmark-yune-vs-librime.ps1).

Command used for both M33 cross-engine runs:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-yune-vs-librime.ps1 -OutputRoot <evidence-dir> -Iterations 9 -SessionIterations 9 -KeyIterations 25
```

Native benchmark command:

```powershell
cmd /c "cargo bench -p yune-rime-api --bench frontend_baselines > target\m33-frontend-baselines-*.txt 2>&1"
```

The cross-engine rows use the same upstream `luna_pinyin` schema id, the same
shared/user data roots, and the same `default` module list. The timed key rows
are `ni`, `hao`, and `zhongguo`; none triggers reverse lookup. After M33, Yune
does not load `stroke` during schema select for those rows, so the former
luna-plus-stroke Yune vs luna-only librime startup/session mismatch is gone.

This is a warm/no-deploy comparison. It does not measure dictionary deployment,
web asset loading, browser paint, or TypeDuck `jyut6ping3` profile behavior.
Memory counters are Windows process working-set counters from the benchmark
host; deltas are more useful than absolute resident values.

## Results

### Cross-engine summary

| Workload | Engine | Median | p95 | Ready delta |
| --- | --- | ---: | ---: | ---: |
| Startup/runtime-ready | Yune before | `2,881,852.7 us` | `3,141,449.8 us` | `218,873,856 bytes` |
| Startup/runtime-ready | Yune after | `47,788.2 us` | `909,375.4 us` | `24,576 bytes` |
| Startup/runtime-ready | librime after | `27,628.3 us` | `80,260.8 us` | `847,872 bytes` |
| Session create/select/destroy | Yune before | `2,985,364.0 us` | `3,027,234.7 us` | `5,283,840 bytes` |
| Session create/select/destroy | Yune after | `47,813.7 us` | `55,848.1 us` | `0 bytes` |
| Session create/select/destroy | librime after | `25,765.9 us` | `30,130.8 us` | `147,456 bytes` |

The after-run startup p95 includes a visible slow sample (`909.4 ms`) even
though the median is now in the same order of magnitude as librime. The median
session row is the cleanest accepted win from the build-once cache.

### Key processing

| Input | Yune before | Yune after | librime after | After ratio |
| --- | ---: | ---: | ---: | ---: |
| `ni` | `5,579.8 us` | `6,064.5 us` | `28.5 us` | `212.8x` |
| `hao` | `11,043.8 us` | `12,463.4 us` | `34.5 us` | `361.3x` |
| `zhongguo` | `34,024.0 us` | `37,572.3 us` | `1,479.8 us` | `25.4x` |

These rows remain the main unresolved native lookup gap. M33 did not rewrite the
table/prism lookup model, so it should not be described as a per-key latency win.

### Native watched rows

The in-repo `frontend_baselines` benchmark confirms the same shape:

| Row | Before median | After median | Change |
| --- | ---: | ---: | ---: |
| `startup_trace_luna_pinyin_select_schema_total` | `261,245 us` | `223,858 us` | `-14.3%` |
| `startup_trace_luna_pinyin_translator_install` | `194,531 us` | `171,438 us` | `-11.9%` |
| `startup_trace_luna_pinyin_spelling_algebra_expand` | `104,343 us` | `107,597 us` | `+3.1%` |
| `startup_trace_luna_pinyin_translator_index_build` | `8,132 us` | `11,283 us` | `+38.8%` |
| `per_key_real_jyut6ping3_mobile_hai_full_abi` | `20,557.833 us` | `20,979.133 us` | `+2.0%` |
| `per_key_real_jyut6ping3_mobile_jigaajiusihaa_full_abi` | `29,935.692 us` | `31,033.692 us` | `+3.7%` |
| `per_key_real_luna_pinyin_ni_full_abi` | `1,429.950 us` | `1,913.350 us` | `+33.8%` |
| `per_key_real_luna_pinyin_zhongguo_full_abi` | `12,064.550 us` | `12,705.675 us` | `+5.3%` |

The native startup trace is useful for owner attribution, but the cross-engine
harness is the public comparison because it isolates the shared C-ABI surface.

## Interpretation

M33 landed two low-risk changes:

- Process-wide sharing of immutable built dictionary translators keyed by schema
  and resolved asset signatures, with deploy/source invalidation coverage.
- Lazy reverse-lookup dictionary loading, so `stroke` is loaded on first reverse
  lookup rather than during `luna_pinyin` schema select.

The lazy spelling-algebra/prism rewrite was not accepted in M33. The checked-in
upstream prism fixture proves the current prism can map spellings such as `ni`
to syllable descriptors, but it does not contain the candidate text/comment/order
payload needed to preserve current byte-identical output without the table-backed
translator state. That rewrite needs a broader representation plan and should
not block M31.

Memory-mapping was also deferred. With the low-risk slice already reducing
startup/session medians into the same order of magnitude as librime, and with the
remaining gap now dominated by per-key lookup representation rather than reverse
dictionary loading, mmap alone was not justified as an M33 follow-on.

## Safe public claim

It is safe to say:

> After M33, Yune's fair upstream `luna_pinyin` startup/session comparison is no
> longer distorted by eager `stroke` reverse-lookup loading. The low-risk native
> cache/lazy-reverse slice cut Yune startup and session medians by about 98%,
> bringing those rows into the same order of magnitude as upstream librime.

It is not safe to say:

> Yune is faster than librime, Yune typing is faster, or browser typing/startup
> improved.

The remaining before-M31 recommendation is to use this fair report for public
copy, avoid a one-sided chart, and keep per-key native lookup optimization as a
future representation milestone if product evidence justifies it.
