# D-47 / D-48 reconciliation

## D-47 performance

The signed final ceilings contain four adjusted rows, not three:

| Row | Ceiling |
| --- | ---: |
| `n` | 3.006 |
| `hao` | 1.844 |
| 37-character Luna input | 2.339 |
| 59-character Luna input | 1.748 |

Increment 4e source `5879405c` passed five rounds at `32/32` aggregate rows and
`160/160` individual observations. The later REACH-03 source `5fa986d8`
contained no production behavior change. Independent review then found a
shipping-source transformed-graph regression; after repair, final behavior
source `443cc636` passed a fresh five rounds at `32/32` aggregate and `160/160`
individual observations under the same ceilings. The sibling source-current
performance packet is authoritative for final M59-REACH-04 acceptance.

## D-48 exact-order closure

- Lane A: strict 13/13, all 5,705 positions exact.
- Lane B: seven inputs exact by candidate and position.
- Cangjie: marked 12/12; owning suite 3/0; unmarked 12/12.
- Deployed 37/59 Luna page shape: exact.
- No new exception was created for 4b, 4c, 4d, or 4e.

The owner-approved Increment 4a class-3 exception remains a historical,
increment-local disposition only: the complete post-OpenCC equal-weight
residual contained 6,086 inversions, zero cross-weight inversions, and no
beyond-oracle-depth use; the cause was librime's equal-weight
import/traversal tie-break. Revisit is mandatory for any cross-weight
inversion, incomplete capture/provenance, or a tie residual moving onto page 1
for a common input. The final Lane A result consumes no exception.

## Scope integrity

- No schema-ID rename.
- No default C ABI expansion.
- No promotion table, baked oracle table, or input allowlist.
- TypeDuck remains a profile/regression lane; upstream librime and pinned
  canonical schema repositories remain the ordering oracle.
- M60 remains formalism/onboarding/static-audit work only.
