# M59 post-fix CPU sample analysis

## Verdict

**Structural validation: PASS (108/108 checks).** All 12 reports are authentic, parseable `/usr/bin/sample` outputs; the parser exactly reproduces Apple's collapsed top-of-stack counts from the call graph. This is qualitative attribution only—never timing evidence.

The packet has a load-bearing steady-state flaw not made explicit by its original README: sampling began after a fixed two-second sleep, not after a warmup-ready handshake. Every Yune capture was still in one-time `RimeCreateSession` work for **19.1–26.1%** of observed samples, while librime setup had already completed. Whole-sample Yune/librime shares therefore are not comparable. The primary tables below condition on `RimeProcessKey` only.

The librime `n` process completed naturally after just **65 total / 40 process-key samples**, so its function ranking is inadequate. `zh` also completed before the requested window, but its 2,101 process-key samples remain useful with a partial-window caveat. The remaining ten reports have about 3,872–3,901 total samples.

## What the samples add

1. **Yune short-input ownership can be upgraded from “likely” to qualitatively observed in the optimized symbolized sibling.** Within process-key samples, `n` spends 32.1% on `abbreviation_sentence_candidates`, 25.4% under `word_graph_for_code_spans`, and 56.7% on the overlapping MARISA lookup family. For `zh`, those shares are 48.2%, 37.4%, and 40.1%. The exact contribution to production latency remains unmeasured.
2. **librime's exact incomplete-prefix spike cause remains unresolved, but its likely owner family is now narrow.** The long process-key captures place 71.0% (37 default) and 77.9% (59 default) under dictionary/table-query traversal; Nano-off repeats at 70.5% and 78.6%. Allocator leaves nested under this family account for 29.5%/32.6% default and 30.3%/40.4% Nano-off. Because each long report aggregates all prefixes, no frame can be tied specifically to the previously measured spike prefixes. Classification: **likely table-query/allocation family; exact spike owner unresolved**, not verified.
3. **Nano-off changes allocator mechanics, not the visible high-level engine route.** Yune stays centered on compact-table/MARISA traversal; librime stays centered on `Dictionary::Lookup -> Table::Query/TableQuery`. The allocator leaf mix changes sharply, but single sample shares are not latency effect sizes.

## Process-key observations

Owner shares below are inclusive stack tags and can overlap; leaf counts are mutually exclusive top-of-stack observations. They are sample counts/proportions, not elapsed time or causal contributions.

| Engine | Allocator | Input | Process-key samples | Inclusive owner tags | Top three exclusive leaves |
|---|---|---:|---:|---|---|
| yune | default | n | 3066 | MARISA 56.7%; sentence 32.1%; abbrev 32.1%; leading-prefix 0.0%; allocator leaves 19.3% | `yune_core::dictionary::compiled_table::collect_marisa_code_paths::h9c360b0ad3415981` 741 (24.2%); `_platform_memcmp` 515 (16.8%); `_xzm_free` 272 (8.9%) |
| librime | default | n | 40 | dictionary/table 32.5%; TableQuery::Access 0.0%; syllabifier 20.0%; allocator leaves 22.5% | none at the >=5-sample reporting floor; ranking insufficient |
| yune | default | zh | 2931 | MARISA 40.1%; sentence 48.9%; abbrev 48.2%; leading-prefix 0.0%; allocator leaves 19.0% | `_platform_memcmp` 497 (17.0%); `yune_core::dictionary::compiled_table::collect_marisa_code_paths::h9c360b0ad3415981` 483 (16.5%); `_xzm_free` 262 (8.9%) |
| librime | default | zh | 2101 | dictionary/table 73.6%; TableQuery::Access 29.6%; syllabifier 10.2%; allocator leaves 30.0% | `rime::TableQuery::Access(int, double, double) const` 349 (16.6%); `_xzm_free` 260 (12.4%); `rime::Table::Query(rime::SyllableGraph const&, unsigned long, std::map<int, std::vector<rime::TableAccessor>>*)` 170 (8.1%) |
| yune | default | 37-char | 2971 | MARISA 69.2%; sentence 25.1%; abbrev 1.7%; leading-prefix 14.8%; allocator leaves 9.7% | `_platform_memcmp` 1050 (35.3%); `yune_core::dictionary::compiled_table::collect_marisa_code_paths::h9c360b0ad3415981` 784 (26.4%); `_xzm_free` 161 (5.4%) |
| librime | default | 37-char | 3780 | dictionary/table 71.0%; TableQuery::Access 16.7%; syllabifier 12.8%; allocator leaves 39.8% | `_xzm_free` 558 (14.8%); `rime::TableQuery::Access(int, double, double) const` 420 (11.1%); `rime::Table::Query(rime::SyllableGraph const&, unsigned long, std::map<int, std::vector<rime::TableAccessor>>*)` 362 (9.6%) |
| yune | default | 59-char | 3006 | MARISA 70.9%; sentence 24.5%; abbrev 2.7%; leading-prefix 14.6%; allocator leaves 10.9% | `_platform_memcmp` 1105 (36.8%); `yune_core::dictionary::compiled_table::collect_marisa_code_paths::h9c360b0ad3415981` 728 (24.2%); `_xzm_free` 176 (5.9%) |
| librime | default | 59-char | 3818 | dictionary/table 77.9%; TableQuery::Access 19.2%; syllabifier 8.8%; allocator leaves 40.8% | `_xzm_free` 603 (15.8%); `rime::TableQuery::Access(int, double, double) const` 484 (12.7%); `rime::Table::Query(rime::SyllableGraph const&, unsigned long, std::map<int, std::vector<rime::TableAccessor>>*)` 338 (8.9%) |
| yune | nano-off | 37-char | 2828 | MARISA 66.3%; sentence 27.7%; abbrev 1.6%; leading-prefix 17.9%; allocator leaves 15.2% | `_platform_memcmp` 933 (33.0%); `yune_core::dictionary::compiled_table::collect_marisa_code_paths::h9c360b0ad3415981` 661 (23.4%); `_xzm_free` 182 (6.4%) |
| librime | nano-off | 37-char | 3793 | dictionary/table 70.5%; TableQuery::Access 18.1%; syllabifier 13.6%; allocator leaves 40.2% | `_xzm_free` 602 (15.9%); `_xzm_xzone_malloc_tiny` 482 (12.7%); `rime::TableQuery::Access(int, double, double) const` 389 (10.3%) |
| yune | nano-off | 59-char | 3114 | MARISA 68.5%; sentence 27.1%; abbrev 3.1%; leading-prefix 17.2%; allocator leaves 16.5% | `_platform_memcmp` 1050 (33.7%); `yune_core::dictionary::compiled_table::collect_marisa_code_paths::h9c360b0ad3415981` 732 (23.5%); `_xzm_free` 198 (6.4%) |
| librime | nano-off | 59-char | 3832 | dictionary/table 78.6%; TableQuery::Access 20.1%; syllabifier 9.4%; allocator leaves 48.8% | `_xzm_free` 603 (15.7%); `_xzm_xzone_malloc_tiny` 443 (11.6%); `rime::TableQuery::Access(int, double, double) const` 409 (10.7%) |

For librime `n`, the displayed percentages have a denominator of only 40 and must not be treated as a ranking.

## Collection and attribution boundaries

- **Yune binary:** SHA-256 `2b71a3a2374a2340d5489f9c94a3522d93d15df76fd1687c76ed67f503c36c0b`, UUID `AE0FE639-71DF-320C-84EA-FA1EFAE5781C`. It is an `opt-level=3`, LTO, one-codegen-unit, debuginfo-enabled, unstripped sibling of commit `afb7079b71f7f9353845114ff3e310c0a38b9b87`, not the frozen production dylib `3dd5a414c68f7884884c5dc172b3f0b088d1f5ae19cb983eb0eeb2f95bc6c710`. It may locate hot functions but cannot supply accepted latency evidence.
- **librime binary:** frozen accepted SHA-256 `743acf3e3a0b64f94680a2f822b00ae42d35ce1e2ab3c8994441bc305adaf8f6`, UUID `CD461F61-D3AE-3324-80CC-07AAF241F5DB`.
- **LTO/inlining:** optimized Rust symbols are useful at function/module-family level, but inlining, recursive flattening, and `<deduplicated_symbol>` frames make instruction-level ownership unsafe. Do not add recursive `collect_marisa_code_paths` nodes; the parser uses exclusive leaf samples and inclusive “stack contains owner” tags.
- **Termination:** cases 02 and 04 retained complete driver rows and visibly finished naturally. The other ten empty CSVs are consistent with the runner terminating them after sampling, as designed, but the runner discards exit status with `wait ... || true`; exact termination signal/status was not retained.
- **External work:** the pre-sample snapshot shows 12 active Spotlight metadata processes totaling 60.6% snapshot CPU (maximum 10.0% for one process); the post snapshot shows 0.0%. External tasks do not create frames in the sampled target, but scheduling can distort exact shares. Cases ran in fixed order, not randomized order.
- **Thermal/power:** AC power, 100% battery, no thermal or performance warning recorded; memory-free snapshot moved from 39% to 41%.
- **Replication:** one capture per case, no confidence interval. Compare stable route shape, not small percentage-point changes.

## Files

- `analyze_cpu_samples.py` — deterministic parser and validation generator.
- `case-summary.csv` — sample coverage, process/setup phase counts, binary identity, and termination evidence.
- `api-phase-samples.csv` — direct samples by public Rime API phase; exposes the Yune setup overlap.
- `top-functions-process-key.csv` — exclusive leaf functions after excluding startup; primary function table.
- `top-functions-whole-process.csv` — Apple's raw whole-process ranking, retained for audit only.
- `hot-stacks-process-key.csv` — top collapsed process-key stacks.
- `owner-tags.csv` — inclusive, explicitly overlapping stack-family counts.
- `nano-comparison.csv` — descriptive default/Nano-off owner shares.
- `finding-classification.csv` — what can and cannot be upgraded.
- `environment-summary.csv`, `input-hashes.csv`, `validation.json`, `output-manifest.csv` — provenance and reproducibility.

## Decision

The page/order parity fix did not leave sentence generation as Yune's dominant long-input route: the long symbolized samples are MARISA/table-lookup heavy. Yune's short `n`/`zh` extra sentence/abbreviation work is now directly observed at function-family level. librime's long aggregate is directly observed as dictionary/table-query and allocation heavy, but the specific incomplete-prefix spike owner is **not** proven because the capture has no per-prefix marker. No performance fix or threshold change follows from these samples alone.
