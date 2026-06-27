# M45 Native Short-Key Latency And Memory Attribution Evidence

M45 closes as a partial native-engine result with measured blockers. It does
not claim browser, frontend, WASM, public-demo, packaging, deployment, or
product-delivery speed.

## Verdict

- Short-key latency: partial. `hao` passes the `<=3.0x` same-run librime gate,
  but `n` and `ni` still miss and remain measured benchmark-parity blockers.
- Memory: resident target only. Track A steady after-ready resident samples are
  below `107,797,708 B`, but the first startup still reaches a real
  `127,475,712 B` peak. M45 records
  `steady-state-meets-target-standing-peak-cost` and does not claim full memory
  success.
- Storage and behavior: Track A keeps `rsmarisa_byte_backed`, mmap table/prism
  mapping, zero selected heap mirrors, `source_fallback=false`, positive
  `rsmarisa` counters, page-bounded output/context, and short-key candidate
  output matching upstream librime `1.17.0`.

## Evidence Map

- Phase 0 native baseline:
  [`phase-0-native-baseline/`](./phase-0-native-baseline/)
- Phase 0 short-key candidate oracle:
  [`phase-0-short-key-oracle/`](./phase-0-short-key-oracle/)
- Final native benchmark:
  [`final-native-benchmark/`](./final-native-benchmark/)
- Final short-key candidate comparison:
  [`final-candidate-comparison/`](./final-candidate-comparison/)
- Phase 0 verdict:
  [`phase-0-verdict.md`](./phase-0-verdict.md)
- Final memory attribution:
  [`final-memory-attribution.md`](./final-memory-attribution.md)
- Visual evidence:
  [`visuals/`](./visuals/)

Final quality gates are recorded in
[`final-native-benchmark/final-gates.md`](./final-native-benchmark/final-gates.md).
