# Yune vs upstream librime performance dashboard

Date: 2026-06-27

This report separates native-engine comparison evidence from browser-harness
evidence. Native Track A claims remain native-engine only; browser/WASM claims
are made only where real-browser evidence is linked below.

Browser startup remains tracked separately. M41 closed the `apps/yune-web`
startup-harness milestone with production-browser evidence under
[`../../apps/yune-web/e2e/results/m41-yune-web-startup-optimization/`](../../apps/yune-web/e2e/results/m41-yune-web-startup-optimization/).
WEB-01 closed as a measured browser-harness no-go. M46 closed the
TypeDuck/Jyutping native Track B and browser WASM memory handoff as a useful
partial result: schema-switch correctness is fixed, but memory remains a
measured no-go/unclassified blocker. WEB-03 then fixed the compiled-asset
contract for the launch Jyutping path and remeasured the browser path; the
shipping/full Jyutping rows now peak and settle at `160.0 MiB`.

## Comparison Lanes

Three lanes, only two of which are fair cross-engine comparisons:

| Lane | Comparison | Schema | Fair? |
| --- | --- | --- | --- |
| **Track A** | Yune vs upstream **librime 1.17.0**, native | `luna_pinyin` | Yes — same schema/dictionary, same-run |
| **Track B** | Yune-web vs **My RIME**, browser | `luna_pinyin` | Yes — same schema (see [browser report](./yune-web-vs-my-rime-browser-baseline.md)) |
| **Jyutping guard** | Yune only, native + browser | `jyut6ping3_mobile` | No — TypeDuck multilingual dictionary; no librime/My-RIME equivalent |

The Jyutping path is a Yune-only integration / correctness / performance guard,
not a head-to-head number: My RIME's Jyutping uses Cantonese-only
[`rime-cantonese`](https://github.com/rime/rime-cantonese), while Yune carries
TypeDuck's multilingual `jyut6ping3` (Cantonese plus English/Hindi/Urdu/Nepali).
**Every cross-engine speed and memory claim in this report is a fair
`luna_pinyin` lane; the Jyutping numbers are guard evidence, never a
comparison.** On the fair lanes, Yune is several times faster than librime on
most native rows, and several times heavier in memory — about `127 MB` native
(Track A) versus `13-17 MB` for librime, and about `160 MiB` browser (Track B)
versus My RIME's `16 MiB`, both on `luna_pinyin` (~10x).

## Latest Review Snapshot

The run-labeled review snapshot under
[`./evidence/reframed-comparison-review-2026-06-27/`](./evidence/reframed-comparison-review-2026-06-27/)
confirms the same current Track A shape while showing normal short-key
run-to-run noise. Fresh Track A ratios are `hao 2.199x`, `n 3.534x`, and
`ni 3.698x`; the public README and README SVG intentionally summarize those as
approximate ranges rather than repinning public docs to one benchmark run.

Only the fresh Track A rows from that native snapshot are valid for comparison
claims. Its Track B native rows are invalid product evidence because the run did
not use `-DeployProductBeforeBenchmark`: `product_path_status.csv` records
`compiled_ready=false`, `selected_storage=unavailable`, and
`source_fallback=true` for the Jyutping dictionaries. Those rows materialize a
source-YAML fallback `translator.entries_by_code` BTreeMap and peak near
`1.05 GB`; they must not replace M46's valid byte-backed Track B result below.

A clean follow-up Track B rerun under
[`./evidence/reframed-comparison-review-2026-06-27/native-track-b-clean/`](./evidence/reframed-comparison-review-2026-06-27/native-track-b-clean/)
uses `-DeployProductBeforeBenchmark` and is valid product evidence:
`compiled_ready=true`, `selected_storage=byte_backed`, table/prism `mmap`, and
`source_fallback=false`. It confirms the M46 memory result with peak working set
`504,676,352 B` and steady rows from `427,171,840 B` to `440,688,640 B`.

## Current Verdict

M45 closes as a partial native-engine result with measured blockers, not as a
full performance or memory success.

Results:

- `hao` passes the short-key target at `24.267us`, `2.110x` same-run upstream
  librime. This preserves the M44 short-key pass; it is not a new M45 speed
  win.
- `n` remains a measured short-key blocker at `68.900us`, `3.313x`.
- `ni` remains a measured short-key blocker at `49.450us`, `3.458x`.
- Phase 0 selected `short-key-measured-no-go`. M45 did not retain a short-key
  engine implementation branch; final short-key medians are fresh same-run
  evidence used to lock candidate-output parity and blocker status, not proof
  of an optimization versus the post-M44 diagnostic baseline.
- Startup, session, `zhongguo`, both M40 full-pinyin long rows, and both
  M42/M44 abbreviation rows stay inside their native no-regression gates.
- Track A steady after-ready working set meets the resident target, with final
  steady rows from `87,498,752 B` to `98,684,928 B`.
- Track A peak memory remains a real standing cost at `127,475,712 B`, above
  the `107,797,708 B` target, so M45 does not claim full memory success.

M45 therefore records two remaining native blockers: short-prefix constant
factor for `n`/`ni`, and the real per-cold-start peak-memory cost. It does not
claim any browser, WASM, frontend, public-demo, packaging, deployment, broad
product, AI, learned `.gram`/octagram, or plugin ABI win.

## M46 Track B Memory Update

M46 is TypeDuck/Jyutping memory attribution, not a Track A short-key latency
plan. Fresh Track B native product evidence records:

| Row | Median us | p95 us | Median working set | Peak working set |
| --- | ---: | ---: | ---: | ---: |
| `h` | `1767.200` | `1785.900` | `441,155,584 B` | `504,627,200 B` |
| `ha` | `1198.400` | `1206.200` | `441,958,400 B` | `504,627,200 B` |
| `hai` | `813.767` | `839.767` | `441,950,208 B` | `504,627,200 B` |
| `hau` | `822.200` | `1002.633` | `441,966,592 B` | `504,627,200 B` |
| `nei` | `399.367` | `473.100` | `441,982,976 B` | `504,627,200 B` |
| `ngo` | `600.533` | `604.867` | `442,011,648 B` | `504,627,200 B` |
| 50+ guard | `33.480` | `33.787` | `442,966,016 B` | `504,627,200 B` |

Track B storage remains source-fallback-free:
`selected_storage=byte_backed`, table/prism mapping `mmap`, selected table and
prism heap mirrors `0`, and `rsmarisa_status=missing_string_table` for both
`jyut6ping3` and `jyut6ping3_scolar`.

M46 Phase 0 selected `schema-switch-regression-fix-first`, then Branch A fixed
the multi-schema browser correctness blocker. The memory headline did not move:
native Track B remains `504,627,200 B` peak, and browser Jyutping remains
`893.1 MiB` for clean and schema-switch rows. M46 therefore closes as
`schema-switch-correctness-fixed-memory-unchanged` with
`measured-no-go-owner-unclassified`; no Track B memory optimization branch is
claimed. The evidence root is
[`./evidence/m46-jyutping-native-wasm-memory-attribution/`](./evidence/m46-jyutping-native-wasm-memory-attribution/).

![M46 Branch A browser memory and correctness](./evidence/m46-jyutping-native-wasm-memory-attribution/m46-branch-a-browser-memory.svg)

WEB-02 follow-up classifies the public-demo Jyutping browser owner that M46
left unclassified. The measured web ABI path now reports
`source_fallback=true`, `selected_storage=owned_heap`, and
`byte_source_len=0` for the public-demo Jyutping path. The fallback reason is
`prism parse failed: UnsupportedVersion`: shipped public-demo Jyutping prisms
are `Rime::Prism/3.0`, while the compact byte-backed path uses current
`Rime::Prism/4.0` artifacts. Retained owner rows name
`translator.entries_by_code` at `510,925,748 B` plus `18,676,626 B`
(`529,602,374 B` total, `505.1 MiB`). This is not a memory reduction and does
not change the `893.1 MiB` browser high-water; it names the first reduction
target as the web/public-demo compiled-asset contract. Evidence:
[`./evidence/web02-jyutping-wasm-memory-attribution/`](./evidence/web02-jyutping-wasm-memory-attribution/).

![WEB-02 public-demo Jyutping storage owner scale](./evidence/web02-jyutping-wasm-memory-attribution/visuals/web02-public-demo-storage-owner.svg)

WEB-03 closes the launch compiled-asset contract follow-up. After the engine
deploy fix in `3ffd4b21` and the regenerated public-demo assets in `ef37bfe9`,
fresh Emscripten/Playwright evidence shows the shipping Jyutping launch rows no
longer select source fallback. The public-demo `full-jyutping` row records
ready `1306 ms`, input-to-candidate `100 ms`, commit `110 ms`, and ready/peak/
steady WASM memory all at `160.0 MiB`. The schema-switch run covers
`jyut6ping3_mobile`, `cangjie5`, and `luna_pinyin`, returning the expected
smoke candidates with zero worker action errors and a max observed WASM value
of `160.0 MiB`.

This is a browser-harness/public-demo compiled-asset fix, not a native-engine
memory win and not a broad product speed claim. The synthetic `extras` row in
the attribution benchmark still reaches `893.1 MiB` because it intentionally
withholds the launch compiled assets; it is retained as a negative control, not
the shipped path. The fair browser comparison lane also remains unchanged:
`luna_pinyin` is still `160.0 MiB` for Yune versus My RIME's `16.0 MiB`.
Evidence:
[`./evidence/web03-three-schema-launch-readiness/`](./evidence/web03-three-schema-launch-readiness/).

![WEB-03 browser WASM memory closeout](./evidence/web03-three-schema-launch-readiness/visuals/web03-browser-wasm-memory.svg)

![WEB-03 browser timing closeout](./evidence/web03-three-schema-launch-readiness/visuals/web03-browser-timing.svg)

## M45 Visual Dashboard

The checked-in M45 visuals summarize the final native evidence under
[`./evidence/m45-native-short-key-memory-attribution/`](./evidence/m45-native-short-key-memory-attribution/).

![M45 short-key same-run ratio gates](./evidence/m45-native-short-key-memory-attribution/visuals/m45-short-key-ratios.svg)

![M45 steady resident and peak memory bands](./evidence/m45-native-short-key-memory-attribution/visuals/m45-memory-bands.svg)

![M45 retained owner attribution](./evidence/m45-native-short-key-memory-attribution/visuals/m45-owner-attribution.svg)

## M45 Final Native Dashboard

Same-run oracle: upstream `rime/librime 1.17.0` with `luna_pinyin`.

| Row | Yune median | librime median | Ratio / guard | M45 result |
| --- | ---: | ---: | ---: | --- |
| startup/runtime-ready | `23,386.400 us` | `26,740.300 us` | `0.875x` | Pass; no startup claim |
| session create/select/destroy | `23,444.800 us` | `25,462.900 us` | `0.921x` | Pass; no session claim |
| `n` | `68.900 us` | `20.800 us` | `3.313x` | Misses target; measured blocker |
| `ni` | `49.450 us` | `14.300 us` | `3.458x` | Misses target; measured blocker |
| `hao` | `24.267 us` | `11.500 us` | `2.110x` | Target met |
| `zhongguo` | `61.225 us` | `164.225 us` | `0.373x` | Pass |
| `ceshiyixiachangjushuruxingnengzenyang` | `279.459 us` | `297.611 us` | `0.939x` | Pass; no abbreviation expansion |
| `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` | `478.915 us` | `665.317 us` | `0.720x` | Pass; no abbreviation expansion |
| `cszysmsrsd` | `525.480 us` | `1,193.170 us` | `0.440x` | Pass; behavior guard preserved |
| `zybfshmsru` | `534.090 us` | `833.980 us` | `0.640x` | Pass; behavior guard preserved |

The final short-key candidate comparison passes for `n`, `ni`, and `hao`:
candidate text, comments, order, context preedit, commit preview, and
first-page metadata match upstream librime `1.17.0`.

## M45 Memory And Storage

| Metric | Phase 0 | Final | Result |
| --- | ---: | ---: | --- |
| Startup steady working set | `89,931,776 B` | `90,161,152 B` | Below resident target |
| Session steady working set | `87,539,712 B` | `87,498,752 B` | Below resident target |
| `n` steady working set | `91,295,744 B` | `91,058,176 B` | Below resident target |
| Highest Track A steady row | `98,058,240 B` | `98,684,928 B` | Below resident target |
| Track A peak working set | `127,528,960 B` | `127,475,712 B` | Still above target; standing peak-cost blocker |
| Track A peak pagefile | `112,230,400 B` | `112,218,112 B` | Still a peak-cost signal |
| Whole-process memory target | n/a | `<=107,797,708 B` required | Resident met; peak not met |

M45 records `steady-state-meets-target-standing-peak-cost`: steady Track A
resident memory is below the old target, but the first startup sample reaches
the same `127 MB` high-water value, so the peak remains a real cold-start cost.

Track A final storage/status:

- `selected_storage=rsmarisa_byte_backed`
- table/prism mapping mode: `mmap`
- selected table/prism heap mirror bytes: `0`
- `source_fallback=false`
- `rsmarisa_status=ok`
- `rsmarisa_mapping_mode=mmap`
- positive `rsmarisa` exact/prefix counters remain present in target rows
- first-page output and `RimeGetContext` stay page-bounded

## M45 Short-Key Owner Profile

Final counters keep the sentence paths out of the short-key rows:
`upstream_sentence_model_calls=0` for `n`, `ni`, and `hao`.

The per-owner figures below come from the metrics-instrumented run (m37
counters enabled), so the `Process key` column exceeds the clean benchmark
medians above (for example `ni` is `96.900 us` instrumented versus `49.450 us`
clean). Read this table for relative owner share, not absolute latency.

| Row | Process key | Translator | Prefix lookup | Rows scanned | First page materialize | Result |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| `n` | `66.900 us` | `64.800 us` | `35.100 us` | `7` | `1.700 us` | Miss; measured blocker |
| `ni` | `96.900 us` | `92.200 us` | `35.600 us` | `14` | `5.200 us` | Miss; measured blocker |
| `hao` | `70.700 us` | `64.000 us` | `9.300 us` | `21` | `8.600 us` | Ratio target met |

The residual issue is a short-prefix translator/prefix lookup constant factor,
not the M40 full-pinyin sentence lookup or M42 abbreviation routing. M45 does
not claim a perceptible typing UX win; these rows are tens of microseconds. No
short-key code branch was retained in M45, so the small movement from the
diagnostic ratios should be treated as normal same-machine run variance rather
than optimization progress.

## Evidence Bundle

Primary evidence root:
[`./evidence/m45-native-short-key-memory-attribution/`](./evidence/m45-native-short-key-memory-attribution/)

Key artifacts:

- Phase 0 benchmark:
  [`phase-0-native-baseline/`](./evidence/m45-native-short-key-memory-attribution/phase-0-native-baseline/)
- Phase 0 short-key oracle:
  [`phase-0-short-key-oracle/`](./evidence/m45-native-short-key-memory-attribution/phase-0-short-key-oracle/)
- Phase 0 verdict:
  [`phase-0-verdict.md`](./evidence/m45-native-short-key-memory-attribution/phase-0-verdict.md)
- Final benchmark bundle:
  [`final-native-benchmark/`](./evidence/m45-native-short-key-memory-attribution/final-native-benchmark/)
- Final candidate-output comparison:
  [`final-candidate-comparison/oracle-vs-yune-candidate-output.md`](./evidence/m45-native-short-key-memory-attribution/final-candidate-comparison/oracle-vs-yune-candidate-output.md)
- Final memory attribution:
  [`final-memory-attribution.md`](./evidence/m45-native-short-key-memory-attribution/final-memory-attribution.md)
- Final visual evidence:
  [`visuals/`](./evidence/m45-native-short-key-memory-attribution/visuals/)
- Final gates:
  [`final-native-benchmark/final-gates.md`](./evidence/m45-native-short-key-memory-attribution/final-native-benchmark/final-gates.md)

## Prior Native Context

M44 remains the predecessor native/profile closeout. It passed `hao`, both
abbreviation rows, and the selected Track B short-row lookup targets, while
recording `ni` and peak memory as measured blockers. Its evidence remains under
[`./evidence/m44-native-performance-owner-reduction/`](./evidence/m44-native-performance-owner-reduction/).

## Remaining Gaps

| Rank | Gap | Evidence | Next diagnostic action |
| ---: | --- | --- | --- |
| 1 | Track A `n`/`ni` short-prefix constant factor | Final `n` `68.900us` / `3.313x`; final `ni` `49.450us` / `3.458x`; `upstream_sentence_model_calls=0`. | Isolate a bounded prefix/translator constant-factor owner without widening long-row, abbreviation, or TypeDuck-profile behavior. |
| 2 | Track A real peak memory | Final peak `127,475,712 B` with steady resident rows below target; first startup still records the peak. | Profile allocator/transient/private and mapped residency before any storage rewrite; keep peak and steady resident numbers separate. |
