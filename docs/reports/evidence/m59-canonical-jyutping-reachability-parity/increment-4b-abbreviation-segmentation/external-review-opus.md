# Increment 4b designated external review

Verdict recorded 2026-07-12: **approved; Increment 4c may begin**.

The owner-designated Opus review inspected packet commit `f9d20ed6`, the
production implementation range `e97811a5..d508e05b`, and the real evidence
files. It independently reran the load-bearing checks rather than accepting the
packet narrative:

- full workspace Clippy with warnings denied;
- `yune-core` 484/484 and `yune-rime-api` 355 passed plus 1 ignored;
- Cantonese 41/41, upstream Luna 25/25, cold start 1/1,
  `yune_web` 49 passed plus 2 evidence-only ignored, and `frontend_client`
  35/35;
- the 4b classifier at raw 1/5 and OpenCC-normalized text/position, preedit,
  and commit preview 5/5, with zero unowned cases, no exception, and no
  beyond-depth use;
- the pre-lazy-to-final comparator at exact 5/5;
- the 21/78/18 Python evidence-tool suites;
- five-run executable/receipt identity, one build plus four exact-byte reuses,
  32/32 aggregate rows, and 160/160 individual observations under unchanged
  ceilings; and
- all 384 nonself manifest rows against the committed bytes, including the
  narrowly scoped raw-receipt line-ending rule.

The review found no schema-id/input allowlist, promotion table, baked oracle
data, public ABI expansion, or `Rime::Table/4.0` format break. It also accepted
the cache lifecycle and v1/v2/legacy/external metadata compatibility and found
no P1 or P2 issue.

WEB-04 remains deliberately outside the 4b verdict. The reviewer confirmed the
structural untoned-Luna boundary, no 4b Octagram/grammar/WEB-04 source change,
and green Luna parity. The disclosed 2/4 ranking result remains a mandatory
Increment 4e and final-M59 blocker; it is not waived.

Nonblocking watch items carried forward:

- `prefix_fallback_views_visited` can undercount disallowed views because its
  increment follows a `continue`;
- executable identity cannot by itself prove measurement authenticity;
- the count-independent UpstreamScript quality band remains a cross-translator
  watch despite unchanged observed order;
- table parsing retains an unbounded ADV-marker `rposition` scan and a silent
  header-marker-less invalid-ADV fallback; and
- bounded and complete tie comparisons rely on monotonic equivalence between
  `raw_quality` and `table_comparison_weight`.

None of these findings blocks 4b or 4c. This record supersedes the packet's
pre-review stop while leaving Lane A, D-48, REACH-04, WEB-04, and M59 open.
