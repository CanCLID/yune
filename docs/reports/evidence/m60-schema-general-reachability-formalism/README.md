# M60 Schema-General Reachability Formalism Evidence

Verdict: **complete, subject to the binding closeout proofs recorded outside
this non-self-referential packet.** M60 formalizes M59's shipped default-on
single-character reachability; it does not change runtime behavior.

The live acceptance registry retains `version: m59-reach03-v1`, declares
`reachabilityFormalismVersion: m60-reachability-v1`, and contains exactly
`reachabilityOptOuts: []`. The canonical contract records request-local
`prefix_fallback_owned` precedence. A read-only Rust audit using production
configuration/deployment semantics supplies 17 current product tuples to the
mandatory checker. The checker classifies every tracked schema root and
enforces the exact explicit-false/approved-opt-out bijection. The updater keeps
new product schemas blocking-open.

## Source boundary

- kickoff base: `b8cd897f9d6c3158d864bac9d2629482c45c7427`;
- preserved premature pushed implementation: `9b55b9baf40fac1690388202e0ce08f41e4bfe09`;
- isolated implementation recovery: `c1f1f941e4777b9c14bc84494fdaf13a63759113`;
- isolated Windows packet-verifier fix: `78a9e38a98e79af98fe0f0c3c9ab9ab350252f28`;
- isolated Windows junction review fix: `e352fba451eeb32c13861cb3cbe9e38ea97eba45`;
- isolated pre-3.12 junction review fix: `c9b34774c82c6b1a3c252c90f95aba6c24bc3183`.

The combined kickoff-to-implementation delta is exactly the frozen 34-path M60
implementation envelope. The containing closeout SHA is intentionally recorded
only in the external post-push receipt.

## Verification

All eight named M60 gates passed. The production checker accepted 60 schema
assets, 17 audit tuples, and zero opt-outs; the Node checker suite passed 52/52;
the updater suite passed 2/2; each of the four focused Rust gates passed its
named test. The utility gate first completed red on Windows short-name path
handling, then completed with 14 tests run, OK, and two platform symlink skips
after the isolated fix. A first isolation review then found that Python's
separate Windows-junction API was not checked. After the four-path review fix,
the exact utility gate ran 16 tests with `OK`: both new junction cases passed,
and the same two privilege-gated symbolic-link cases remained skipped. A repeat
review then found that API absent before Python 3.12. The second four-path fix
uses Windows reparse tags directly and fails closed if that metadata is
unavailable; the same 16-test gate passed, both junction cases ran, and only the
same two symbolic-link cases skipped. No broad workspace, benchmark, WASM,
browser, packaging, memory, or latency suite was used as reassurance.

## Preserved deviations

A concurrent writer committed and pushed `9b55b9ba` before the prescribed
isolated implementation tree was recorded. That original proof does not exist
and was not backfilled. The user authorized forward-only completion after
disclosure. Later repair/fix commits have isolated-tree proofs; the final
candidate is independently isolated. The first PowerShell-policy setup
transcript was not retained; an external forward-correction receipt says so,
and no transcript was fabricated.

The first preserved pre-review tree, `057656af0aa319ccf42881381406264ccd19ed61`,
was discarded after requirement review passed but isolation review failed on
the junction gap. A second reviewed tree, `75ab7b955c893cf856a267ceab73e4c531ddb11f`,
was discarded after requirement review passed but isolation review failed on
the pre-3.12 fallback. Both attempts remain external and do not count toward
closeout. An intermediate unreviewed candidate was also invalidated for a
provenance wording correction. The candidate was rebuilt and both reviews were
repeated.

The two independent review receipts are intentionally absent from the preserved
pre-review tree. After review, only those receipts and this packet's manifest
may change. External receipts bind the reviewed tree to the final candidate,
the closeout commit tree, the pushed SHA, and remote equality.

## Scope boundary

No M59 or WEB03 evidence, runtime engine behavior, candidate order/ranking,
selection, recomposition, schema installation, C ABI/API table/export,
schema/profile id or payload, browser UI, Windows product behavior, oracle
fixture, signed threshold, benchmark, memory optimization, or M61 implementation
changed.
