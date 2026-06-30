# M50 Track A Launch-Readiness Evidence

Scope: native Track A `luna_pinyin` only. This folder must not be used for browser, frontend, package, deployment, public-demo, TypeDuck product, or iOS-device claims.

## Slices

| Slice | Status | Evidence |
| --- | --- | --- |
| Task 0 clippy gate | complete | `task0-clippy/` |
| Phase 0 baseline | complete | `phase-0-baseline/` |
| Sentence row reduction | complete | `sentence-row/` |
| Short-prefix reduction | measured partial | `short-prefix-final/` |

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

## Sentence Row Slice

Evidence: `sentence-row/`.

The sentence-row slice keeps the scope to native Track A `luna_pinyin`. It adds
normal sentence vocabulary owner attribution and removes unused retained
`WordGraphEntry.code` strings from sentence graph edges. The fresh benchmark
shows the 37-character tracked row inside the `<=3.0x` gate:

- 37-character `luna_pinyin` row: `860.011 us` vs librime `294.551 us`,
  `2.920x` (baseline was `3.161x`).
- 59-character `luna_pinyin` row: `1630.029 us` vs librime `664.893 us`,
  `2.452x`.
- `n`: `59.400 us` vs librime `20.900 us`, `2.842x`.
- `ni`: `44.800 us` vs librime `14.550 us`, `3.079x`, still the short-prefix
  latency blocker for Task 2.

Memory attribution now names `poet.vocabulary` as `53,644,752 B` retained heap
for normal preset vocabulary. This is attribution, not a retained prefix index;
the focused guard rejects any hidden `prefix_index` owner row.

## Short-Prefix Slice

Evidence: `short-prefix-final/`.

The short-prefix slice keeps M44's bounded first-page behavior and filter
underfill fallback intact. It avoids raw-comment cloning while materializing
bounded lookup candidates when the formatted comment is empty or derived from
the entry-code suffix, and it adds empty-set fast paths for Track A dictionary
exclusion and spelling-abbreviation checks.

The final run improved or held the Yune medians versus the Phase 0 baseline for
the short-prefix rows, but `ni` remains above the same-run `<=3.0x` gate:

- `n`: `60.000 us` vs librime `21.600 us`, `2.778x`.
- `ni`: `44.500 us` vs librime `14.600 us`, `3.048x`; measured blocker.

M37 attribution for `ni` still shows the core blocker as exact-row scan work
under charset filtering: `196` lookup views for the two-key sequence, `14`
materialized candidates, and `26.550 us` median short-key filter time. No
retained heap prefix or vocabulary index was added.
