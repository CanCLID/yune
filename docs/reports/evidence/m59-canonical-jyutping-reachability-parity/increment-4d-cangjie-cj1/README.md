# M59 Increment 4d Cangjie CJ-1 packet

Status: Increment 4d is implemented at clean production source
`38e759f6ac0c79512713c33533df465e908538db`. The marked upstream-Cangjie
validation lane is strict `12/12` over every captured page, the Cangjie
composition suite is `3 passed / 0 ignored`, and the unmarked control remains
exact `12/12` against the pre-4d `fd6bd2a7` capture. The five-round signed
performance guard is `32/32`. This closes M59-PARITY-03 and CJ-1. It does not
close Lane B, M59-REACH-03/04, M59-EVIDENCE-01, M59-GATES-01, or M59.
Increment 4e Lane B exact order and WEB-04 restoration are next.

## Mechanism

The marked validation lane sets the explicit schema marker value
`upstream_script` through a disposable schema overlay. For
`table_translator`, that marker selects the internal
`SentencePolicy::UpstreamTable`; it is not an `UpstreamScript` policy. The
selection is translator configuration plus `SchemaBehaviorProfile`, never
schema id. It makes the upstream table sentence/phrase stream authoritative
where pinned librime uses that stream, while TypeDuck and the unmarked Cangjie
control stay fail-closed on their existing behavior.

The implementation adds the missing compiled-prism predictive traversal,
reverse-syllabification graph, upstream table sentence tail, reverse
Poet/preset-vocabulary phrase reconstruction, and result-level stream-head
merge semantics needed by the pinned `rime/rime-cangjie` lane. All graph walks
are bounded and cycle-hardened. The byte-backed Poet path remains lazy and the
public ABI and compiled table/prism formats do not change.

The competing-segmentation fixture is generated from the already pinned
external Cangjie oracle plus byte-exact upstream source rows. Expected output
is never derived from Yune. The marker is staged into a create-new disposable
shared tree and is not copied into the product schema tree.

## Behavior verdict

[`behavior/cangjie-yune-marked.json`](./behavior/cangjie-yune-marked.json)
binds to clean source commit `38e759f6`, tree `948504ed`, release DLL SHA-256
`a0fe13b7a5df3669d09425f48adaa5e2821ac578df00962cffc2d95a0420652e`,
the pinned oracle fixture SHA-256
`24408c3b2b83db516ae1382d2ba743b41ead50c7c026aee2837a01137c7ecbcf`,
and complete all-page capture.

The untouched strict comparator is green `12/12`: candidate text, order,
count, page shape, termination, and all captured positions are exact with no
exception, replacement, or tail policy. The `tak` row is exact `30/30`, which
exercises the reverse preset-vocabulary phrase tail rather than only the three
headline composition rows.

The separate unmarked capture uses the same clean source and DLL over the
unpatched shared tree. Its comparator is exact `12/12` against the `fd6bd2a7`
pre-4d control capture, proving the new behavior remains scoped to the explicit
marker. Product Cangjie is not patched by this increment.

## Performance verdict

[`performance-ratchet/`](./performance-ratchet/) contains five fresh complete
runs from the same clean source and release DLL over all 17 Track A inputs plus
the Track B product row. Product deployment is enabled and iterations remain
`9/60/80`. Run 1 builds one source-bound native harness; runs 2-5 reuse its
exact bytes and receipt, for mode sequence
`build,reuse,reuse,reuse,reuse`.

All 32 aggregate median rows pass their unchanged signed ceilings across 160
preserved observations, with zero individual failures. The guarded medians
include `n 0.209x <= 3.006x`, `ni 0.247x <= 2.666x`,
`hao 0.283x <= 1.844x`, the 37-character row `0.019x <= 2.339x`, and the
59-character row `0.008x <= 1.748x`. This is an Increment 4d guard, not the
final M59-REACH-04 measurement after 4e.

## Packet map and boundary

- `behavior/` preserves the untouched final marked and unmarked captures and
  strict comparator outputs.
- `staging/staging-manifest.json` binds the source tree, marker bytes, source
  and staged schemas, staged tree, and staging tool.
- `audit/` preserves the public-API 504/504 owned/byte-backed predictive-code
  audit source, dependency lock, command, and result.
- `performance-ratchet/` preserves every text receipt from the five accepted
  runs plus the executable aggregate verdict and provenance.
- `commands-provenance.md`, `setup-attempts.md`, and `verification.md` record
  exact identities, rejected/non-counted setup, focused gates, and scope.
- `reviews/` records the independent specification, quality, and staging-runner
  approvals.
- `packet-manifest.json` inventories every packet-local file except itself.

No DLL, benchmark executable, compiled table, deployed tree, MARISA payload,
or other binary is copied into this packet. The owner-signed 4a class-3
exception remains in force exactly as recorded and is neither consumed nor
expanded here. Remaining M59 behavior work is Increment 4e Lane B exact order
and WEB-04 restoration, followed by final REACH-03/04 reconciliation and the
exact native/WASM/browser/package gates.
