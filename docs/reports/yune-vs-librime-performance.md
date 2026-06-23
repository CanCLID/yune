# Yune vs upstream librime performance report

Date: 2026-06-23

Evidence:

- M33 fairness/cache evidence: [`evidence/m33-2026-06-23/`](./evidence/m33-2026-06-23/)
- M34 before cross-engine rerun: [`evidence/m34-queryable-table-prism/baseline-yune-vs-librime/`](./evidence/m34-queryable-table-prism/baseline-yune-vs-librime/)
- M34 after cross-engine rerun: [`evidence/m34-queryable-table-prism/after-yune-vs-librime/`](./evidence/m34-queryable-table-prism/after-yune-vs-librime/)
- M34 native logs: [`evidence/m34-queryable-table-prism/frontend-baselines-before.txt`](./evidence/m34-queryable-table-prism/frontend-baselines-before.txt) and [`evidence/m34-queryable-table-prism/frontend-baselines-after-final.txt`](./evidence/m34-queryable-table-prism/frontend-baselines-after-final.txt)
- M34 visualizations: [`m34-cross-engine-gap.svg`](./evidence/m34-queryable-table-prism/m34-cross-engine-gap.svg), [`m34-native-improvement.svg`](./evidence/m34-queryable-table-prism/m34-native-improvement.svg), and [`m34-working-set-gap.svg`](./evidence/m34-queryable-table-prism/m34-working-set-gap.svg)

## Public summary

M33 corrected the unfair `luna_pinyin` comparison by lazy-loading the `stroke`
reverse lookup and sharing built dictionary translators across compatible schema
selects. M34 then landed a narrower first-page candidate-pipeline optimization:
short `luna_pinyin` inputs can keep complete cheap prefix enumeration while
materializing only a bounded candidate window for the ABI/context path.

The safe public claim is still conservative:

- Yune is no longer measuring luna-plus-stroke startup against luna-only librime.
- M34 improves a bounded native first-page `luna_pinyin` path, most clearly in
  `per_key_real_luna_pinyin_ni_full_abi`: `1,760.250 us` -> `1,132.950 us`
  (`-35.6%`).
- Yune still trails librime widely on per-key rows and memory footprint.
- No browser startup, browser typing, WASM, React, Cloudflare, or TypeDuck-Web
  delivery win is claimed from M34.

Final fair M34 cross-engine after-run:

- `hao`: Yune `12,216.900 us`, librime `35.100 us`; Yune is `348.1x` slower.
- `ni`: Yune `5,693.900 us`, librime `28.700 us`; Yune is `198.4x` slower.
- `zhongguo`: Yune `35,909.100 us`, librime `1,379.400 us`; Yune is `26.0x` slower.
- Session create/select/destroy: Yune `46,743.400 us`, librime `28,121.800 us`; Yune is `1.7x` slower.
- Warm startup/runtime-ready: Yune `47,126.800 us`, librime `30,315.200 us`; Yune is `1.6x` slower.
- Peak working set: Yune `182,333,440 bytes`, librime `22,585,344 bytes`; Yune peaks at about `8.1x` librime.

## Visual summary

These charts are generated from the final M34 evidence bundle, not from a
browser/runtime run. They support the native engine-performance claim only.

![M34 final fair Yune versus librime median latency gap](./evidence/m34-queryable-table-prism/m34-cross-engine-gap.svg)

The fair cross-engine chart is intentionally log-scale. It shows that M34 made
the comparison honest and kept the public per-key gap visible: Yune remains
`348.1x` slower on `hao`, `198.4x` slower on `ni`, and `26.0x` slower on
`zhongguo`.

![M34 native benchmark before and after movement](./evidence/m34-queryable-table-prism/m34-native-improvement.svg)

The native watched-row chart is the achievement view. M34's clearest landed win
is the bounded first-page `ni` full-ABI path, from `1,760.250 us` to
`1,132.950 us` (`-35.6%`). It also records the TypeDuck `hai` regression
(`+5.7%`) rather than hiding it.

![M34 peak working set gap](./evidence/m34-queryable-table-prism/m34-working-set-gap.svg)

The memory chart is the main unresolved gap: Yune peaks at about `8.1x` librime
in the fair harness. M34 did not land compiled storage, prism lookup, or mmap,
so this chart should remain visible in public material.

## Methodology

Both engines were measured through the same librime-shaped C API harness:
[`../../scripts/yune-vs-librime-benchmark.cs`](../../scripts/yune-vs-librime-benchmark.cs),
driven by [`../../scripts/benchmark-yune-vs-librime.ps1`](../../scripts/benchmark-yune-vs-librime.ps1).

Cross-engine command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-yune-vs-librime.ps1 -OutputRoot <evidence-dir> -Iterations 9 -SessionIterations 9 -KeyIterations 25
```

Native benchmark command:

```powershell
cmd /c "cargo bench -p yune-rime-api --bench frontend_baselines > target\m34-frontend-baselines-*.txt 2>&1"
```

The cross-engine rows use the same upstream `luna_pinyin` schema id, the same
shared/user data roots, and the same default module list. This is a no-deploy
comparison. It does not measure TypeDuck-Web delivery, browser paint, Cloudflare
cache behavior, or public-demo startup.

## Results

### Cross-engine summary

| Workload | Engine | Baseline median | M34 after median | After p95 | Peak working set |
| --- | --- | ---: | ---: | ---: | ---: |
| `hao` key sequence | Yune | `13,336.800 us` | `12,216.900 us` | `13,688.700 us` | `182,333,440 bytes` |
| `hao` key sequence | librime | `35.200 us` | `35.100 us` | `35.900 us` | `22,507,520 bytes` |
| `ni` key sequence | Yune | `5,858.800 us` | `5,693.900 us` | `5,822.400 us` | `182,333,440 bytes` |
| `ni` key sequence | librime | `28.300 us` | `28.700 us` | `58.500 us` | `22,495,232 bytes` |
| `zhongguo` key sequence | Yune | `36,451.100 us` | `35,909.100 us` | `39,995.800 us` | `182,333,440 bytes` |
| `zhongguo` key sequence | librime | `1,503.400 us` | `1,379.400 us` | `1,446.800 us` | `22,585,344 bytes` |
| session create/select/destroy | Yune | `48,329.000 us` | `46,743.400 us` | `51,333.100 us` | `182,333,440 bytes` |
| session create/select/destroy | librime | `30,778.900 us` | `28,121.800 us` | `30,889.900 us` | `22,470,656 bytes` |
| startup/runtime-ready | Yune | `50,065.200 us` | `47,126.800 us` | `885,728.100 us` | `182,333,440 bytes` |
| startup/runtime-ready | librime | `31,804.000 us` | `30,315.200 us` | `75,034.500 us` | `22,392,832 bytes` |

Yune baseline-to-M34-after movement in the fair cross-engine harness:

| Workload | Change |
| --- | ---: |
| `hao` key sequence | `-8.4%` |
| `ni` key sequence | `-2.8%` |
| `zhongguo` key sequence | `-1.5%` |
| session create/select/destroy | `-3.3%` |
| startup/runtime-ready | `-5.9%` |

These rows are mixed. They are safe to publish only with the unresolved per-key
gap visible.

### Native watched rows

| Row | Before median | M34 after median | Change |
| --- | ---: | ---: | ---: |
| `per_key_real_luna_pinyin_ni_full_abi` | `1,760.250 us` | `1,132.950 us` | `-35.6%` |
| `per_key_real_luna_pinyin_ni_engine_only` | `569.700 us` | `575.250 us` | `+1.0%` |
| `per_key_real_luna_pinyin_zhongguo_full_abi` | `12,697.600 us` | `12,119.013 us` | `-4.6%` |
| `per_key_real_luna_pinyin_zhongguo_engine_only` | `532.575 us` | `515.713 us` | `-3.2%` |
| `per_key_real_jyut6ping3_mobile_hai_full_abi` | `18,389.567 us` | `19,446.467 us` | `+5.7%` |
| `per_key_real_jyut6ping3_mobile_jigaajiusihaa_full_abi` | `29,937.777 us` | `28,155.585 us` | `-6.0%` |
| `per_key_real_jyut6ping3_mobile_jigaajiusihaa_correction_full_abi` | `29,649.146 us` | `28,032.915 us` | `-5.5%` |
| `startup_trace_luna_pinyin_select_schema_total` | `240,094.000 us` | `227,901.000 us` | `-5.1%` |
| `startup_trace_luna_pinyin_translator_install` | `188,404.000 us` | `176,787.000 us` | `-6.2%` |
| `startup_trace_luna_pinyin_spelling_algebra_expand` | `121,609.000 us` | `112,475.000 us` | `-7.5%` |
| `startup_trace_luna_pinyin_translator_index_build` | `11,657.000 us` | `11,289.000 us` | `-3.2%` |

`per_key_real_luna_pinyin_hao_*` is a new native row added in M34; it has no
same-harness before value. Final after medians are `1,378.800 us` full ABI and
`761.667 us` engine-only.

Engine-only before/after rows are attribution-only because M34 fixed the native
engine-only benchmark to set the real schema id. Full-ABI rows and the
cross-engine harness are the public compare surfaces.

## Interpretation

M34 landed Lever A only:

- internal `CandidateRequest` / `TranslationResult`
- bounded `StaticTableTranslator` request path for the safe subset
- lazy engine candidate-window completion on out-of-window access
- full-list reader preservation for candidate-list iterator APIs
- internal `TableLookup` abstraction implemented for the current heap map

M34 deliberately did not land:

- compiled table query storage
- prism-backed candidate lookup
- mmap/borrowed storage
- browser/runtime delivery work
- TypeDuck profile behavior changes

The remaining performance gap is now better split:

- Native first-page short-prefix context work can be bounded and improved.
- Engine-only lookup is still not close to librime.
- Cold startup and peak memory still need a queryable table/prism representation
  before mmap can pay off.

## Safe public claim

It is safe to say:

> Yune's fair upstream `luna_pinyin` comparison now separates native engine
> work from browser delivery. After M34, the native first-page `ni` full-ABI row
> improved from about `1.76 ms` to `1.13 ms`, but Yune still trails librime by
> roughly `26x` to `348x` on the fair per-key rows and about `8x` on peak
> working set.

It is not safe to say:

> Yune is faster than librime, Yune uses less memory than librime, Yune browser
> startup or browser typing improved, or the queryable table/prism storage work
> is done.
