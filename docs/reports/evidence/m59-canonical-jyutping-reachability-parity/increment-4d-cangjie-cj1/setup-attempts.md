# Increment 4d setup attempts

Two non-acceptance setup events are excluded from the five preserved rounds:

1. A complete diagnostic pass was run while the restored oracle tree used
   copied cached OpenCC support bytes instead of the separately published
   official dependency archive. It used the correct clean Yune source, DLL,
   and already pinned schema repositories, but its upstream shared/build tree
   identities (`8b9b75a6...` / `fe83078c...`) did not satisfy the official
   dependency-asset provenance required by this milestone. It was classified
   as a setup pass before any aggregate decision. No row from it is counted,
   substituted, or copied into this packet. The canonical five-run sequence
   began only after the dependency asset's GitHub digest was verified and that
   asset was extracted.
2. The first aggregate command used a mistaken noncanonical output path. The
   deterministic aggregator was rerun over the same five untouched final run
   directories with the canonical output path. No benchmark was rerun, no
   observation changed, and the bulky noncanonical output is not copied.

The accepted oracle tree then used ordinary files with the pinned release DLL,
pinned schema sources, and immutable shared/build hashes recorded in all five
run receipts. Run 1 built the source-bound harness; runs 2-5 reused those exact
bytes and receipt. The accepted mode sequence and all five inputs/iterations
are independently checked by `gate-verdict.provenance.json`.
