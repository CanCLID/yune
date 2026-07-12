# Increment 4b performance disposition

Owner decision recorded 2026-07-11 (America/Los_Angeles): retain the existing
signed M59 performance ceilings and proceed with a lazy, page-bounded
abbreviation-admission strategy. Complete oracle order must remain available on
demand; the red checkpoint does not authorize a re-baseline, an exact-order
waiver, or a silent revert of Increment 4b behavior.

The accepted implementation must therefore keep the bounded typing path within
the signed short-key and standing ceilings, while forward navigation and the
complete-candidate APIs expose the full captured order. The preserved red run is
under
[`performance-ratchet/failed-attempts/e97811a5-eager-materialization/`](./performance-ratchet/failed-attempts/e97811a5-eager-materialization/).

Outcome recorded 2026-07-12: the lazy/page-bounded implementation satisfies the
signed disposition without changing a ceiling. Five source-bound rounds at
production source `d508e05b` pass all 32 aggregate rows and all 160 individual
observations, including `n`, `ni`, `hao`, and the 37/59-character cases. This
clears the 4b performance stop only; the designated Opus review still blocks 4c.
