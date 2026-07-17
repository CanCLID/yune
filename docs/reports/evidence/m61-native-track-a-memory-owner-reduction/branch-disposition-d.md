# M61 Branch Disposition D

Disposition: **D — measured partial/no-go.**

The `91f59696` correction-source diagnostic passed candidate, Track B, signed,
fixed-binary, model, owner-shape, and projected supplemental memory checks. It
failed the two binding owner-reconciliation checks:

- coverage `0.746098930 < 0.80`;
- residual `6,371,950 B > 5,019,238 B`.

This was the first correction-source measured red, so no retry, threshold
change, successor owner branch, production-default candidate, or final
production five-round set was permitted. The correction was explicitly
reverted by `01a62f2a`; its tree exactly equals the pushed quality-repair tree.
The correction and revert remain in direct-main history for auditability.

Terminal requirement disposition:

- `M61-BASELINE-01`: complete with measured no-go;
- `M61-ATTR-01`: complete with measured no-go;
- `M61-BRANCH-01`: complete, disposition D and explicit revert;
- `M61-REDUCE-01`: closed by no-go, no production reduction;
- `M61-COMPAT-01`: complete, green diagnostics plus exact runtime-tree restore;
- `M61-RATCHET-01`: closed by no-go, supplemental projection unclaimed;
- `M61-EVIDENCE-01`: complete in the final closeout tree with both reviews.
