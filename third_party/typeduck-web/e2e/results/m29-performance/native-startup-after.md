# M29 Native Startup After

Date: 2026-06-22

Command:

```powershell
cmd /c "cargo bench -p yune-rime-api --bench frontend_baselines > target\m29-frontend-baselines-after-startup.txt 2>&1"
```

Captured output: `target/m29-frontend-baselines-after-startup.txt`.

Optimization: `crates/yune-core/src/spelling_algebra.rs` now skips no-op regex replacement allocation by checking `Regex::is_match` before `replace_all` for syllable-scoped and whole-string transforms.

## Startup Before/After

| Row | Before median | After median | Delta | Before p95 | After p95 |
| --- | ---: | ---: | ---: | ---: | ---: |
| `startup_real_jyut6ping3_mobile_runtime_ready` | `6,441,132.800us` | `6,280,297.700us` | `-160,835.100us` | `6,487,698.400us` | `6,338,902.700us` |
| `startup_trace_jyut6ping3_mobile_select_schema_total` | `5,705,357us` | `5,490,829us` | `-214,528us` | `5,779,699us` | `5,498,645us` |
| `startup_trace_jyut6ping3_mobile_translator_install` | `5,576,695us` | `5,353,859us` | `-222,836us` | `5,644,827us` | `5,370,854us` |
| `startup_trace_jyut6ping3_mobile_spelling_algebra_expand` | `5,253,577us` | `5,045,837us` | `-207,740us` | `5,320,971us` | `5,058,813us` |
| `startup_trace_jyut6ping3_mobile_source_dictionary_parse_if_any` | `151,949us` | `149,178us` | `-2,771us` | `162,382us` | `158,978us` |
| `startup_trace_jyut6ping3_mobile_translator_index_build` | `141,716us` | `135,690us` | `-6,026us` | `152,295us` | `147,292us` |

## Memory Before/After

| Row | Before | After | Delta |
| --- | ---: | ---: | ---: |
| Single-startup after-ready bytes | `1,105,899,520` | `1,102,348,288` | `-3,551,232` |
| Single-startup after-finalize bytes | `14,897,152` | `12,808,192` | `-2,088,960` |
| Single-startup peak bytes | `1,126,006,784` | `1,123,426,304` | `-2,580,480` |
| Repeated-trace peak bytes | `1,795,403,776` | `1,792,008,192` | `-3,395,584` |
| `spelling_algebra_expand` working-set delta | `699,400,192` | `668,954,624` | `-30,445,568` |

## Browser Before/After

| Scenario | Before | After | Finding |
| --- | ---: | ---: | --- |
| Fresh startup | `5,299ms` | `5,378ms` | Flat/mixed; browser startup did not materially move. |
| Reload startup | `5,211ms` | `5,245ms` | Flat/mixed; browser startup did not materially move. |

## Typing Before/After

| Scenario | Before p95 keydown-to-paint | After p95 keydown-to-paint | Main owner note |
| --- | ---: | ---: | --- |
| `hai` | `61ms` | `62ms` | Worker/native stayed roughly flat (`25ms` -> `26ms`); React stayed about `35ms`. |
| Long phrase | `50ms` | `59ms` | Worker/native stayed flat (`46ms` -> `46ms`); total moved with browser variance. |
| Long composition | `39ms` | `44ms` | Worker/native moved `33ms` -> `35ms`; total moved with browser variance. |
| Paging | `13ms` | `16ms` | Single-sample UI path; not a startup optimization target. |
| Reverse lookup | `16ms` | `29ms` | Worker/native stayed `7ms`; total moved with paint-proxy variance. |

Finding: the M29 code change materially reduces the measured native startup owner by about `208ms` median and `262ms` p95, while browser startup and typing are flat/mixed. Candidate behavior, ranking, comments, and ABI surfaces are unchanged by this performance slice.
