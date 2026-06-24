# M36 Strategy Outcomes

| Strategy | Outcome | Evidence |
| --- | --- | --- |
| Native in-process harness | Landed | `native_inprocess_benchmark`, `scripts/benchmark-native-rime-inprocess.ps1`, `phase-0-baseline/` |
| Track A/Track B separation | Landed | CSV columns `track`, `engine`, `schema_id`; Track A uses only `luna_pinyin`, Track B uses only `jyut6ping3_mobile` |
| Product SourceFallback status | Landed | `product_path_status.csv` in baseline and final runs |
| Product interning/byte arena | Closed by no-go | Product profiler evidence pointed first to compiled-path activation; no standalone interning owner was isolated after compiled storage lowered working set |
| `rsmarisa` on real product blobs | Closed by no-go | stale checksums plus table/reverse marisa sections and prism/table syllabary mismatch |
| Yune-native no-marisa product artifacts | Landed | schema-scoped workspace update and final `compiled_ready=true` status |
| Product compact table+prism storage | Landed | final Track B compiled-ready status plus reduced key latency and working set |
| TypeDuck behavior guards | Landed | focused Cantonese and upstream parity gates passed; `typeduck_web` rerun is recorded in `task-gates.md` |
| Bounded/lazy product candidate pipeline | Closed by no-go | product profile still requires whole-list correction, prediction, prefix, sentence, and recomposition semantics |
| mmap/borrowed storage | Closed by no-go | not attempted after owned compact storage produced the M36 win and rsmarisa remained blocked |
| Public latency claims | Landed with guardrail | Track A ratios remain separate from Track B product before/after rows |
