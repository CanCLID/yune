# M38 Final Gates

Date: 2026-06-25

Final native evidence:
`docs/reports/evidence/m38-engine-performance-parity/phase-5-final-native/`

## Final Native Ratios

| Row | Yune median | librime median | Ratio | Gate |
| --- | ---: | ---: | ---: | --- |
| startup/runtime-ready | `23,363.300us` | `24,351.000us` | `0.959x` | pass, under `1.25x` |
| session create/select/destroy | `24,243.500us` | `27,969.500us` | `0.867x` | pass, under `1.25x` |
| `hao` | `38.933us` | `11.400us` | `3.415x` | pass, under `5x` |
| `ni` | `56.750us` | `14.300us` | `3.969x` | pass, under `5x` |
| `zhongguo` | `64.263us` | `181.375us` | `0.354x` | pass, under `5x` |

## Selected Hot Path

The final Track A status row records `selected_storage=rsmarisa_byte_backed`,
`table_mapping_mode=mmap`, `prism_mapping_mode=mmap`,
`source_fallback=false`, table byte source length `13013460`, stored entries
`498564`, table heap mirror bytes `0`, and prism heap mirror bytes `0`.

The checksum exception is narrow: `accepted_upstream_marisa_import_checksum`
appears only for the known upstream `luna_pinyin` source/table checksum pair
where the imported marisa table checksum differs from Yune's current source
checksum owner.

The per-key metrics prove real hot-path use and zero ordinary no-marisa
fallback:

| Input | rsmarisa exact calls | rsmarisa prefix calls | no-marisa exact | no-marisa prefix | full-list fallback | bounded reads | unbounded reads |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `hao` | `150` | `100` | `0` | `0` | `0` | `150` | `0` |
| `ni` | `100` | `50` | `0` | `0` | `0` | `100` | `0` |
| `zhongguo` | `400` | `350` | `0` | `0` | `0` | `400` | `0` |

## Raw Lookup And Owner Rows

`raw_lookup_microbench.csv` records Yune-only diagnostic rows:

| Input | Raw prism median | Raw prism codes | Raw table median | Raw table candidates | Translator median | Context export median |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `hao` | `0.100us` | `1` | `13.900us` | `139` | `34.767us` | `1.000us` |
| `ni` | `0.100us` | `1` | `18.200us` | `182` | `52.800us` | `1.000us` |
| `zhongguo` | `0.000us` | `0` | `2.200us` | `4` | `60.462us` | `1.100us` |

Allocation/context counters are page-bounded for ordinary first-page reads:

| Input | Owned candidates per op | Context page clones per op | ABI C-string allocations per op | ABI C-string bytes per op |
| --- | ---: | ---: | ---: | ---: |
| `hao` | `20.000` | `1.667` | `4.000` | `16.000` |
| `ni` | `20.000` | `2.500` | `6.000` | `21.000` |
| `zhongguo` | `20.000` | `0.625` | `1.500` | `15.125` |

## Memory

Final Track A median working set:

| Engine / row | Median working set | Max peak working set |
| --- | ---: | ---: |
| Yune startup | `111,624,192 B` | `163,151,872 B` |
| librime startup | `11,534,336 B` | `14,045,184 B` |
| Yune session | `108,019,712 B` | `163,151,872 B` |
| librime session | `10,948,608 B` | `14,098,432 B` |
| Yune key rows | `111,824,896-112,279,552 B` | `163,151,872 B` |
| librime key rows | `12,247,040-13,193,216 B` | `14,262,272 B` |

Yune remains materially larger than librime at whole-process working-set level.
M38 closes the latency/storage gates because the selected Track A hot path has
no table/prism heap mirror (`0` bytes), uses mmap-backed deployed bytes, and
reports the remaining memory gap honestly.

## Review Fixes

The final code-review pass found and M38 fixed:

- MARISA exact lookup now enumerates all valid pinyin segmentations for a
  normalized code such as `xian`, instead of using only one longest-first path.
- The mmap-backed `rsmarisa` string table now stores an `Arc` to the exact mmap
  owner, rather than accepting an arbitrary caller-supplied byte-source owner.
- The upstream `luna_pinyin` checksum exception is restricted to the known
  source/table checksum pair.
- Namespaced translator spelling algebra has a regression test proving that
  secondary namespaces do not inherit global `speller/algebra` unless they opt
  in with `<namespace>/speller/algebra`.
- Metrics timers no longer take `Instant::now()` on hot paths when M37/M38
  metrics are disabled.

## Quality Gates

Final verification:

- `cargo fmt --check` - pass.
- `cargo clippy --workspace --all-targets -- -D warnings` - pass.
- `cargo test -p yune-core compact_table_lookup_resolves_marisa_backed_upstream_table_entries` - pass.
- `cargo test -p yune-core --test upstream_luna_pinyin_parity` - pass, 12 passed.
- `cargo test -p yune-rime-api schema_selection` - pass, 74 passed, 1 ignored.
- `cargo test -p yune-rime-api context_status` - pass, 10 passed.
- `cargo test -p yune-rime-api --test yune_web yune_web_adapter_browser_app_assets_load_jyutping_mandarin_pinyin_reverse_lookup -- --nocapture` - pass.
- `cargo test --workspace` - pass.
- `npm.cmd --prefix packages/yune-web-runtime test` - pass, 65 passed.
- `npm.cmd --prefix packages/yune-web-runtime run build` - pass.
- `scripts/benchmark-native-rime-inprocess.ps1 -OutputRoot docs/reports/evidence/m38-engine-performance-parity/phase-5-final-native -Iterations 9 -SessionIterations 20 -KeyIterations 50` - pass.
- `git diff --check` - pass.

No frontend, browser, product, packaging, deployment, or public-delivery speed
claim is made from this evidence.
