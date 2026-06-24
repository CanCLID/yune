# M36 Product-Path Engine Optimization Evidence

This folder contains the checked-in evidence for M36 product-path engine
optimization.

Primary reports:

- [`../../yune-vs-librime-performance.md`](../../yune-vs-librime-performance.md)
- [`../../yune-vs-librime-root-cause-analysis.md`](../../yune-vs-librime-root-cause-analysis.md)

Raw native in-process benchmark runs:

- [`phase-0-baseline/`](./phase-0-baseline/) - Track A fair `luna_pinyin`
  comparison plus Track B `jyut6ping3_mobile` baseline with current shipped
  stale/unsupported product `.bin` files.
- [`phase-4-final/`](./phase-4-final/) - same Track A comparison plus Track B
  after schema-scoped deploy rebuilds Yune-native product artifacts before
  measurement.

Visualizations generated from the final M36 CSV evidence:

- [`m36-product-latency-before-after.svg`](./m36-product-latency-before-after.svg)
- [`m36-product-memory-before-after.svg`](./m36-product-memory-before-after.svg)
- [`m36-track-a-latency-gap.svg`](./m36-track-a-latency-gap.svg)
- [`m36-track-a-working-set-gap.svg`](./m36-track-a-working-set-gap.svg)

Task evidence:

- [`phase-0-baseline.md`](./phase-0-baseline.md)
- [`product-storage-switch.md`](./product-storage-switch.md)
- [`rsmarisa-gate.md`](./rsmarisa-gate.md)
- [`bounded-lazy-gate.md`](./bounded-lazy-gate.md)
- [`strategy-outcomes.md`](./strategy-outcomes.md)
- [`phase-4-final.md`](./phase-4-final.md)
- [`task-gates.md`](./task-gates.md)
