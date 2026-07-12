# Increment 4c final specification review

Verdict: **PASS — no P1 or P2 specification/acceptance finding.**

The independent review verified the current implementation, packet-local
classifier, five-round aggregate, and live M59 truth surfaces. It confirmed:

- pinned librime `ConvertWord` semantics require ordered exact forms,
  per-stage stable deduplication, and default-only maximum-prefix conversion
  when an exact match is absent;
- the fail-closed classifier is 13/13 over 5,705 candidates, with 83 inventory
  rows covering 64/65 source keys, 14 visible occurrences, and zero mapping,
  order, position, OpenCC residual, exception, or beyond-depth use;
- preedit, commit preview, complete pagination, page shape, and termination are
  exact;
- 854 candidate-comment differences are explicitly non-gating and no canonical
  comment or whole-capture byte-identity claim is made;
- the five-round aggregate is 32/32 over 160 observations, with the run-2 Track
  A session-latency failure and run-4 Track B private-memory failure preserved;
- `n`, `ni`, `hao`, and the 37/59-character rows pass unchanged ceilings;
- focused OpenCC core tests passed 11/11, source/compiled API oracle 1/1, real
  deployed TypeDuck sibling-order guard 1/1, and classifier tests 20/20; and
- Lane A alone is complete; 4d, 4e/WEB-04, REACH-03/04, final evidence, final
  release/browser/package gates, and M59 remain open.

The pre-manifest packet audit found 258 text-only files, zero NUL-bearing files,
six parseable JSON files, resolving links, and a clean `git diff --check`.
Its sole P3 publication follow-up was to add both review receipts and then
generate the final packet manifest, which is handled by the publication pass.

Reviewed source: `e11557e2bbb05e3598e2d96dd6eb669ded88d33d`.
Review date: 2026-07-12.
