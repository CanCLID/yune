# M50 Track A Launch-Readiness Evidence

Scope: native Track A `luna_pinyin` only. This folder must not be used for browser, frontend, package, deployment, public-demo, TypeDuck product, or iOS-device claims.

## Slices

| Slice | Status | Evidence |
| --- | --- | --- |
| Task 0 clippy gate | complete | `task0-clippy/` |
| Phase 0 baseline | complete | `phase-0-baseline/` |

## Current Decision Order

Phase 0 at `76edb38998b5d35e78491dff00ff548d9bb33dd3` shows:

- `n`: `57.300 us` vs librime `20.700 us`, `2.768x`, now inside the `<=3.0x` gate.
- `ni`: `44.900 us` vs librime `14.750 us`, `3.044x`, still a short-prefix blocker.
- 37-character `luna_pinyin` row: `915.897 us` vs librime `289.705 us`, `3.161x`, the largest current latency blocker.
- Track A peak working set: Yune `188,510,208 B` vs librime max peer `17,317,888 B`, still a memory blocker with a large process-level unclassified component.

Reduction order:

1. Sentence row first.
2. Short-prefix `ni` second; keep `n` as a passing guard row.
3. Full Track A memory attribution third.
