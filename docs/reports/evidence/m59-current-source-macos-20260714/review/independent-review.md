# Independent review record

## Protocol and calculation review

Verdict: ready to share; no blocking error.

- Recomputed all 13,600 Track A and 400 Track B raw key samples with the
  benchmark's `ceil((n-1)·p)` index rule (an upper median at `p=0.5`), matching
  `crates/yune-rime-api/benches/native_inprocess_benchmark.rs`.
- Reconciled every per-run median/p95/p99/max, all 85 Track A ratios, the full
  17-row medians/pooled worsts/spreads, Windows deltas/classes, and Track B
  aggregate.
- Verified five complete unique rounds using the exact inputs, `9/60/80`
  iterations, product deployment, AC power, clean source identities, required
  artifacts, and stable Yune/librime/candidate hashes.
- Verified the signed/final Windows joins, unchanged ceilings, candidate
  comparisons, owner/product counts, no retry/discard evidence, and the exact
  two-line external adapter.
- Ratios and published spreads intentionally use the benchmark's three-decimal
  observations. Full-precision sensitivity checks confirm the large spreads
  are primarily genuine round-to-round variance; rounding changes their
  magnitude and can increase or decrease the published percentage.

## Behavior and interpretation review

Verdict: share with the explicit caveats now incorporated in the report.

- `zhongdengchangdu` is a deterministic cross-platform Yune engine-path
  discrepancy, not macOS noise or a macOS-only defect. The same suffix mismatch
  exists in the Windows M59 performance-ratchet candidate snapshots under
  `docs/reports/evidence/m59-canonical-jyutping-reachability-parity/` for
  increments 4c, 4d, and 4e.
- Round-boundary UI activity is a material noise confounder. It does not
  invalidate fixed binaries, deterministic candidate/owner/product shape, or
  the all-rows same-Mac direction, but prevents clean causal platform
  attribution.
- Current Track A candidate pages are exact versus same-run librime for 16/17
  inputs, including 37/59 and eight of nine newly signed rows. Track B remains
  exact versus M57.
- Normalized owner/product shapes are identical across all five current rounds;
  Track B remains byte-backed/mmap with unchanged checksums and readiness.
