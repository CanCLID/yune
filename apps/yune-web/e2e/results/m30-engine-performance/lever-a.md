# M30 Lever A Evidence

Date: 2026-06-22

## Change

Lever A removes the steady-state duplicate expanded-entry vector from `StaticTableTranslator` for spelling-algebra-backed translators. The translator now:

- keeps the original dictionary row stream only as a builder-only `source_entries` value,
- consumes that row stream in `with_spelling_algebra(...)` so spelling-algebra dedupe sees the original row order,
- builds the final `entries_by_code` map by moving `Candidate` values instead of cloning them, and
- leaves steady-state lookup, completion, prefix fallback, sentence lookup, correction scan, and debug lookup paths on `entries_by_code`.

## Audit

Audit command:

```powershell
rg -n "self\.entries|\bentries\b|entries_by_code|spelling_abbreviation_entries" crates\yune-core\src\translator\mod.rs
```

Result:

- No steady-state `StaticTableTranslator` query path reads `self.entries`; that field no longer exists.
- Builder-only row-stream uses:
  - `new(...)` and `from_dictionary(...)` initialize `source_entries` and the initial index.
  - `with_spelling_algebra(...)` consumes `source_entries` before building the final index.
  - `with_upstream_sentence_model(...)` consumes `source_entries` if present, otherwise snapshots from the index.
- Steady-state query/debug paths read `entries_by_code` and `spelling_abbreviation_entries` only:
  - exact lookup, completion, prefix fallback, correction scan, sentence segmentation, and spelling-algebra debug all route through `entries_by_code`.
- A first implementation rebuilt spelling algebra input from sorted `entries_by_code`; `typeduck_adapter_real_assets_prefix_fallback_commits_consumed_span` caught the row-order regression. The final implementation preserves original row order through `source_entries` and consumes it after spelling algebra.

## Behavior Guards

Pre-change:

- `cargo test -p yune-core --test cantonese_parity -- m28_followup`: passed, `3 passed`.
- `cargo test -p yune-core --test upstream_luna_pinyin_parity`: passed, `12 passed`.
- `cargo test -p yune-rime-api --test typeduck_web`: passed, `28 passed`.

After final Lever A patch:

- `cargo test -p yune-core --test upstream_luna_pinyin_parity`: passed, `12 passed`.
- `cargo test -p yune-core --test cantonese_parity -- m28_followup`: passed, `3 passed`.
- `cargo test -p yune-rime-api --test typeduck_web -- typeduck_adapter_real_assets_prefix_fallback_commits_consumed_span`: passed, `1 passed`.

Full post-change TypeDuck-Web Rust coverage is left to the closeout gate because the full file takes about 15 minutes locally.

## Native Measurement

After command:

```powershell
cmd /c "cargo bench -p yune-rime-api --bench frontend_baselines > target\m30-frontend-baselines-lever-a.txt 2>&1"
```

Captured output: `target/m30-frontend-baselines-lever-a.txt`.

Result: passed. Cargo printed the known `yune_rime_api` output filename collision warnings and exited 0.

### Startup And Memory

| Row | Before median | After median | Delta | Before p95 | After p95 |
| --- | ---: | ---: | ---: | ---: | ---: |
| `startup_real_jyut6ping3_mobile_runtime_ready` | `6,242,614.900us` | `5,952,128.400us` | `-290,486.500us` | `6,288,639.300us` | `5,996,458.400us` |
| `startup_trace_jyut6ping3_mobile_select_schema_total` | `5,472,559us` | `5,274,501us` | `-198,058us` | `5,577,741us` | `5,310,619us` |
| `startup_trace_jyut6ping3_mobile_translator_install` | `5,340,382us` | `5,136,455us` | `-203,927us` | `5,449,418us` | `5,170,983us` |
| `startup_trace_jyut6ping3_mobile_spelling_algebra_expand` | `5,015,352us` | `4,819,266us` | `-196,086us` | `5,127,754us` | `4,850,458us` |
| `startup_trace_jyut6ping3_mobile_translator_index_build` | `144,074us` | `143,657us` | `-417us` | `150,794us` | `145,296us` |

| Memory metric | Before | After | Delta |
| --- | ---: | ---: | ---: |
| Single-startup after-ready bytes | `1,103,331,328` | `838,209,536` | `-265,121,792` |
| Single-startup peak bytes | `1,123,745,792` | `1,023,639,552` | `-100,106,240` |
| Single-startup after-finalize bytes | `12,951,552` | `12,500,992` | `-450,560` |
| `select_schema_total` working-set delta | `956,739,584` | `448,724,992` | `-508,014,592` |
| `translator_install` working-set delta | `988,053,504` | `475,901,952` | `-512,151,552` |
| `spelling_algebra_expand` working-set delta | `923,234,304` | `452,104,192` | `-471,130,112` |

### Watched Per-Key Rows

| Row | Before median | After median | Before p95 | After p95 | Reading |
| --- | ---: | ---: | ---: | ---: | --- |
| `per_key_real_jyut6ping3_mobile_hai_full_abi` | `15,849.267us` | `16,251.867us` | `17,093.200us` | `16,589.233us` | flat/noisy |
| `per_key_real_jyut6ping3_mobile_hai_engine_only` | `8,121.467us` | `8,828.600us` | `8,628.800us` | `10,271.433us` | median slightly slower, p95 noisy; not an engine typing win |
| `per_key_real_jyut6ping3_mobile_jigaajiusihaa_full_abi` | `22,119.315us` | `23,216.315us` | `23,615.408us` | `24,407.700us` | flat/noisy |
| `per_key_real_jyut6ping3_mobile_jigaajiusihaa_engine_only` | `15,326.546us` | `15,349.631us` | `15,794.408us` | `15,767.523us` | flat |
| `per_key_real_jyut6ping3_mobile_jigaajiusihaa_correction_full_abi` | `22,380.546us` | `23,253.500us` | `23,042.715us` | `25,010.669us` | flat/noisy |
| `per_key_real_jyut6ping3_mobile_jigaajiusihaa_correction_engine_only` | `124,738.846us` | `124,976.169us` | `125,883.923us` | `126,996.023us` | flat |

## Decision

Lever A clears the M30 continuation gate by improving single-startup after-ready bytes by about `253MB` MiB-equivalent / `265MB` decimal, well above the `100MB` threshold. It also reduces runtime-ready startup median by about `290ms`.

The larger Lever B/C internal payload rewrite, sentence-DP backpointer rewrite, and correction-stress indexing are deferred from this M30 slice because the post-Lever-A per-key rows do not identify a new native hot owner and the remaining changes are broader ranking/storage work. M30 will close on Lever A plus evidence, with the remaining performance ceiling documented for a future scoped plan.
