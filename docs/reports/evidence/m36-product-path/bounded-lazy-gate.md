# Bounded/Lazy Candidate Gate

The existing bounded candidate path remains active only for requests that do
not require whole-list semantics.

M36 did not widen bounded/lazy lookup to the TypeDuck product profile because
the product path still depends on:

- dynamic correction lookup;
- tolerance/correction records;
- prediction never-first;
- prediction candidate limits;
- prefix fallback;
- sentence and recomposition behavior.

Those features make the whole-list ordering and filtering semantics part of
the compatibility surface. The safe M36 optimization is therefore storage
switching after compiled product artifacts are active, not a broader lazy
candidate pipeline change for `jyut6ping3_mobile`.
