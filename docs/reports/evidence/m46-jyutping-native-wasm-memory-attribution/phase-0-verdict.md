# M46 Verdict

Phase 0 verdict: `schema-switch-regression-fix-first`

M46 Phase 0 does not authorize a Track B `rsmarisa`, candidate-payload,
scolar-deferral, reverse-index, or transient-deploy optimization branch yet.
The current hard blocker is correctness: the browser still loses Jyutping
candidates after Cangjie -> Luna -> Jyutping schema switching.

Final closeout verdict: `schema-switch-correctness-fixed-memory-unchanged`

Branch A fixed the correctness blocker selected by Phase 0. M46 still does not
authorize or claim a memory optimization branch because native Track B and
browser Jyutping headline memory did not move.

## Evidence

| Artifact | Result |
| --- | --- |
| [`provenance.md`](./provenance.md) | Records post-WEB-01 main, host/toolchain, and serialized native/browser run order. |
| [`phase-0-native/`](./phase-0-native/) | Track B native peak remains `504,627,200 B`; steady rows remain `427-443 MB`; storage stays `source_fallback=false` with zero selected heap mirrors. |
| [`native-owner-reconciliation.md`](./native-owner-reconciliation.md) | Named concrete native owners explain only about `59.7 MB`; process-level memory remains mostly unclassified. |
| [`browser-attribution.md`](./browser-attribution.md) | Jyutping browser WASM remains `893.1 MiB` for single-schema, core/full asset families, and schema-switch capture. |
| [`schema-switch-correctness.md`](./schema-switch-correctness.md) | Clean Jyutping passes, but Cangjie -> Luna -> Jyutping returns zero Jyutping candidates. |
| [`native-vs-wasm-gap.md`](./native-vs-wasm-gap.md) | Explains why native mmap/status owners do not automatically move WASM linear memory. |

## Memory Action Read

| Candidate branch | Phase 0 read |
| --- | --- |
| `schema-switch-regression-fix-first` | Selected. The no-candidate row is product-affecting for multi-schema web sessions and blocks browser memory closeout. |
| `candidate-payload-owner-authorized` | Not selected. Text/comment payload is visible but only `2,575,292 B` logical overlapping bytes. |
| `rsmarisa-track-b-spike-authorized` | Not selected. The code-string owner is `4,189,674 B`; D-33/M36 still blocks stale product-blob assumptions. |
| `scolar-defer-or-lazy-load-authorized` | Not selected. Scolar source/status bytes are visible, but native owner rows do not prove retained dominance and browser family rows do not move high-water. |
| `reverse-index-owner-authorized` | Not selected. Reverse/browser family rows do not move the `893.1 MiB` high-water. |
| `transient-deploy-peak-owner-authorized` | Not selected. Phase 0 lacks a named transient owner. |
| `measured-no-go-owner-unclassified` | Deferred. If Branch A fixes correctness without exposing a memory owner, this becomes the likely memory closeout. |

## Next Branch

Phase 0 required Branch A before any memory optimization branch:

1. Inspect browser worker/runtime schema lifecycle state after
   Cangjie -> Luna -> Jyutping.
2. Fix or explicitly own why the active schema is `jyut6ping3` but `nei`
   returns zero candidates.
3. Gate clean Jyutping, Cangjie -> Luna -> Jyutping,
   Jyutping -> Luna -> Jyutping, reverse lookup, userdb persistence,
   Shift ASCII, and candidate commit.
4. Re-run browser WASM memory after the correctness fix and only then decide
   whether M46 should continue to a memory-owner branch or close as measured
   no-go/unclassified.

M46 is not a full memory success. No native or browser headline memory moved in
Phase 0.

## Branch A Outcome

Evidence:
[`branch-a-closeout.md`](./branch-a-closeout.md)

Post-fix browser evidence shows:

- clean Jyutping: `nei -> 你`, six candidates, zero worker action errors;
- Cangjie -> Luna -> Jyutping: final `nei -> 你`, six candidates, zero worker
  action errors;
- Jyutping -> Luna -> Jyutping: final `nei -> 你`, six candidates, zero worker
  action errors;
- max observed WASM remains `936,509,440 B` (`893.1 MiB`) in all three
  scenarios.

M46 closes as a useful partial result with
`measured-no-go-owner-unclassified`. Future memory work needs a new scoped plan
and a larger measured owner before changing Track B storage or browser asset
loading.
