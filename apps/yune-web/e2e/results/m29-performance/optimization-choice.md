# M29 Optimization Choice

Date: 2026-06-22

## Startup Target

Chosen owner: `startup_trace_jyut6ping3_mobile_spelling_algebra_expand`.

Baseline:

- Median `5,253,577us`.
- P95 `5,320,971us`.
- Working-set delta `699,400,192` bytes.

Reason: fresh M29 attribution kept spelling algebra expansion as the dominant native startup owner under TypeDuck real assets. M27 had already added the per-original-code expansion cache, so M29 chose a distinct lower-risk lever: avoid allocating no-op regex replacement results by checking whether each transform pattern matches before calling `replace_all`.

Rejected startup targets:

- `source_dictionary_parse_if_any`: median `151,949us`, far smaller than spelling expansion.
- `translator_index_build`: median `141,716us`, far smaller than spelling expansion.
- Browser asset loading: browser fresh/reload startup stayed near `5.3s`; native attribution still identified the larger owner inside schema selection/runtime setup.

## Typing Target

Chosen owner: no typing code target in this milestone; M29 closes typing with attribution evidence and a startup-only optimization.

Baseline:

- Browser `hai` p95 keydown-to-paint: `61ms`.
- Browser long phrase p95 keydown-to-paint: `50ms`.
- Dominant owner for long phrase before optimization: worker/native process at `46ms`.

Reason: typing totals were already near or below the inherited M26 evidence and the owner profile was mixed. The startup owner was several seconds and had a semantics-preserving optimization available. No candidate ordering, comment, commit, ABI, or ranking behavior was changed for M29.

Rejected typing targets:

- Native key processing: long phrase worker/native p95 was measurable but small compared with startup, and the native after-run stayed mixed (`46ms` -> `45ms` for long phrase, `25ms` -> `25ms` for `hai`).
- Worker serialization: no evidence showed serialization alone dominated the measured scenarios.
- React rendering: `hai` React p95 was visible (`34ms` before, `35ms` after), but changing render structure would be UI-riskier than the M29 startup lever and did not explain long phrase latency.
