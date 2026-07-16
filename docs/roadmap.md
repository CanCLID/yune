# Roadmap

Yune is a Rust input-method engine that uses upstream librime as a
compatibility oracle while building a cleaner, AI-native-capable engine. This
file is the live operating dashboard: it records current state, the next
authorized sequence, scope boundaries, and readiness gates. Completed
milestone detail belongs in
[`ledgers/milestone-history.md`](./ledgers/milestone-history.md), completed
plans, reports, and evidence packets.

Current status: M59 is complete, WEB03-11 is closed as maintenance, and the
finalized M60 plan is ready for execution but not started. M60 remains the sole
authorized execution milestone. A draft M61 native Track A memory-owner plan is
queued behind M60 for independent review; it is not authorized for execution.
M60 formalizes and audits M59's already-shipped schema-general reachability
capability without changing runtime behavior. Current performance results and
bottleneck analysis live only in
the
[`Yune Performance Dashboard`](./reports/yune-vs-librime-performance.md).
TypeDuck-Windows product/frontend work is owned by the separate
[`CanCLID/yune-windows`](https://github.com/CanCLID/yune-windows) repository.

> **Compatibility authority.** The default core oracle is upstream
> `rime/librime 1.17.0` at
> `33e78140250125871856cdc5b42ddc6a5fcd3cd4`. Canonical Jyutping also uses
> pinned `rime/rime-cantonese`; TypeDuck-HK/librime `v1.1.2` remains a
> profile-only oracle. The complete target and precedence contract is in
> [`contracts/engine-support-contract.md`](./contracts/engine-support-contract.md)
> and [`decisions.md`](./decisions.md).

## Authority Map

- [`conventions.md`](./conventions.md) — architecture, repository and testing
  conventions, evidence handling, C ABI rules, and current risks.
- [`contracts/engine-support-contract.md`](./contracts/engine-support-contract.md)
  — supported targets, oracle precedence, ABI boundaries, and evidence lanes.
- [`requirements.md`](./requirements.md) — requirement status and traceability.
- [`decisions.md`](./decisions.md) — standing principles and binding decisions.
- [`ledgers/milestone-history.md`](./ledgers/milestone-history.md) — completed
  milestone outcomes and historical closeout pointers.
- [`reports/yune-vs-librime-performance.md`](./reports/yune-vs-librime-performance.md)
  — the single current cross-platform performance dashboard.
- [`plans/active/m60-plan-schema-general-single-character-reachability.md`](./plans/active/m60-plan-schema-general-single-character-reachability.md)
  — the current execution authority and sole next milestone.
- [`plans/active/m61-plan-native-track-a-memory-owner-reduction.md`](./plans/active/m61-plan-native-track-a-memory-owner-reduction.md)
  — a non-authoritative draft queued after M60; it becomes executable only
  after M60 closes and the M61 plan, requirements, and evidence contract pass
  independent review.
- [`plans/`](./plans) — active, reference, and completed execution records.

## Current Snapshot

| Lane | Current state | Next decision or gate |
| --- | --- | --- |
| Schema-general reachability | M59 ships default-on single-character reachability for every current and future schema automatically on install. Final native behavior/performance authority is `443cc636`; browser/WASM/package closeout remains bound to `5fa986d8`, with manifest follow-up `07845e02`. | Execute M60 as formalism and static governance only: retain `version: m59-reach03-v1`; add `reachabilityFormalismVersion: m60-reachability-v1` and `reachabilityOptOuts: []`; and audit onboarding through production merge semantics. No shipped opt-out is currently approved. |
| Native performance | The signed M59 Windows ceilings remain authoritative. Final Windows source `443cc636` passes `32/32` aggregate and `160/160` individual observations. The reviewed Mac packet at `0111cf47` is diagnostic and source-unmatched. | Keep the Windows ratchet unchanged. Any causal platform claim or optimization proposal needs fresh same-source, same-lane evidence and an identified structural owner. |
| Native memory | Track A remains materially above same-run librime on Windows and macOS. M47's comments-intact TypeDuck/Jyutping keyboard profile remains complete for its Windows private/dirty proxy, not Apple `phys_footprint`. | After M60, review/finalize the M61 draft for the Windows native Track A lane. Reproduce the exact post-M60 source, rank non-overlapping owners, and run an owned-versus-byte-backed POET diagnostic before selecting one branch. Historical bytes are context only. |
| Web harness | WEB03-11 is closed at clean source `ef485b10`. A deployment-maintenance migration is separating certification from Cloudflare delivery: the unchanged gate runs on a source-pinned CI artifact, a preview canary proves it, and only the identical archive may be promoted. Cloudflare Git auto-builds are disabled and the branch-restricted GitHub environments are provisioned. This is not a refresh of M59 browser authority or the browser peer lane. | Activate the direct-upload workflow and require one green source-bound preview/promotion receipt; keep WEB03-11 fail-closed and preserve every measured red. |
| AI-native layer | M11/M13 proved a default-off local AI layer above the deterministic engine. | Keep AI outside the classic path unless a named experiment explicitly enables it and owns privacy, memory, and behavior evidence. |
| Platform products | TypeDuck-Windows product/frontend work has transferred to `CanCLID/yune-windows`. | Bring back only a separately proposed engine, package, or profile-API requirement with oracle evidence and owning tests. |

## Authoritative Sequence

1. **Execute M60.** Add the canonical capability contract; retain
   `version: m59-reach03-v1`; add
   `reachabilityFormalismVersion: m60-reachability-v1` and
   `reachabilityOptOuts: []`; add a production-semantic read-only Rust audit,
   fail-closed onboarding checks, compact source-bound evidence, and closeout
   documentation. Correct stale schema-wide TypeDuck precedence prose to the
   actual per-input `prefix_fallback_owned` rule.
2. **Close M60 without widening scope.** No runtime behavior, C ABI/API table
   or export, schema/profile id, oracle fixture or capture, benchmark, signed
   threshold, browser surface, Windows product, or M61 allocation belongs in
   M60.
3. **Review and finalize M61 after M60.** Bind the actual M60 closeout SHA, add
   the provisional requirement IDs to traceability, independently review the
   measurement/threshold contract, and keep the plan non-executable until
   those gates pass.
4. **Execute M61 only if the fresh baseline confirms an owner.** The sole
   acceptance lane is Windows native Track A `luna_pinyin`. Begin with five
   fixed-binary owned rounds. Run the five-round byte-backed diagnostic only
   after one green exploratory byte-backed round; retain a production change
   only if same-process Windows private bytes and named-owner evidence
   corroborate the whole-process reduction and every unchanged signed row stays
   green.
5. **Re-rank other future work after M61.** Browser memory, Apple-device
   validation, product-profile storage, schema-id migration, and AI remain
   separate lanes requiring their own owner evidence and scope.

Trigger-gated only: extracting the full processor pipeline from
`yune-rime-api` into `yune-core` waits for a real non-ABI consumer. Do not open
a milestone for speculative layering work.

## Current Guardrails

- **Reachability:** M59's schema-general default-on guarantee is shipped and is
  not reopened by M60. No per-schema enablement, input allowlist, baked oracle
  output, or silent onboarding exclusion is acceptable. No shipped opt-out is
  currently approved; M60 adds the explicit empty array and exact formalism
  version above. An opt-out requires a complete owner-approved record.
- **Onboarding:** every new schema asset in a registered repository-owned
  product root receives a blocking `status: open` row. Unsupported/N/A or an
  auto-created opt-out cannot close it. Every future product root must register
  with the checker, and the production updater must use the tested
  reconciliation helper without synthesizing, approving, suggesting, or
  implying an opt-out.
- **Performance:** the consolidated
  [`M59 threshold registry`](./reports/evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv)
  remains the native authority; the freshest source-bound proof is the
  [`443cc636` gate verdict](./reports/evidence/m59-canonical-jyutping-reachability-parity/source-current-performance-revalidation-2026-07-13/gate-verdict.csv).
  Raw benchmark output stays outside the tracked repository, and only compact
  curated receipts may be imported. A Mac diagnostic is not a new Windows
  baseline, platform-speed verdict, or threshold.
- **Evidence lanes:** native engine evidence does not prove browser, frontend,
  package, deployment, iOS-device, or application-visible performance. Preserve
  source identity and do not project a result across binaries or counters.
- **Compatibility and ABI:** preserve oracle behavior, the M51 support
  contract, default upstream `rime_get_api()`, TypeDuck-profile-only extension
  surfaces, and the synchronized `yune_web_*` export family.
- **Product ownership:** this repository owns the engine and shared web harness.
  Platform frontend/product work stays in its owning repository unless a named
  engine boundary is proposed explicitly.

## Performance North Star

Performance is lane-specific; Yune is not described as universally faster
than librime. A future performance milestone must name one lane, capture fresh
same-run baseline and noise evidence, identify one structural owner before code
changes, define a real win bar, preserve oracle and ABI behavior, and be willing
to close partial or no-go. No latency implementation is currently selected.
See the
[`current dashboard`](./reports/yune-vs-librime-performance.md#current-bottleneck-analysis)
for measurements and owner-ranked bottlenecks.

<a id="closing-the-188-mb-native-track-a-memory-gap"></a>

Historical M49–M55 work framed the native Track A memory problem as the
"188 MB gap." That number is not a current target or portable counter. The M61
draft deliberately replaces it with a fresh exact-source Windows baseline,
owner reconciliation, and a predeclared reduction bar while keeping memory and
CPU independently gated. Historical outcomes remain in the
[`milestone ledger`](./ledgers/milestone-history.md); current numbers remain in
the [`performance dashboard`](./reports/yune-vs-librime-performance.md#native-startup-session-and-memory).

## Scope Ledger

This is the current target boundary. Completed examples and chronology live in
the support contract and milestone history.

| In scope | Deferred until a named target needs it | Non-goal |
| --- | --- | --- |
| Upstream `luna_pinyin` behavior for the named target set, including the accepted native grammar lane | Broader learned grammar/plugin behavior and contextual translation | Bit-for-bit parity with librime internals |
| Captured common-schema targets, including `double_pinyin`, `cangjie5`, and `bopomofo`; `cangjie5` also retains its order-parity contract | Further schema breadth only with fresh oracle fixtures and owning tests | Unbounded schema-checklist work |
| Canonical Jyutping against upstream librime plus pinned rime-cantonese; TypeDuck multilingual/comment/profile behavior in its profile lane | A `jyut6ping3_typeduck` id split only after explicit sign-off and migration evidence | Treating a shared schema id as proof that differently sourced schemas have identical provenance |
| Default-on single-character composition for every current and future schema automatically on install, with zero per-schema adaptation; M60 actively formalizes and audits this shipped guarantee | Future opt-out or onboarding-formalism evolution only after a separately approved need | Per-input hardcoding, baked oracle data, schema allowlists, or silent unsupported onboarding |
| Native startup, session, lookup, translation, context, memory, and allocation guardrails | Frontend/application delivery and platform packaging evidence | Claiming application-visible wins from native-only evidence |
| Default-off AI behavior layered above the compatible deterministic engine | Richer local-first AI experiments with explicit privacy and memory ownership | Replacing or altering classic input paths by default |

## Historical Closeouts

Completed milestone narratives are deliberately not repeated here. Use the
[`milestone history ledger`](./ledgers/milestone-history.md), completed plans,
and evidence packets. Superseded performance interpretation and platform
diagnostics are indexed by
[`reports/history/README.md`](./reports/history/README.md); they are not current
acceptance authorities.

## Deferred / Future

- **Apple-device memory validation:** measure M47's portable product profile
  with Apple `phys_footprint` in a real host when that platform lane is opened.
- **M47 product-profile memory polish:** RED-09/10/11-style asset/profile,
  allocator, or keyboard-startup work remains separate from the queued M61
  Track A plan and requires fresh product-lane ownership.
- **Browser fair-lane work:** refresh the same-schema peer lane before choosing
  a startup, WASM-memory, or encoded-resource owner.
- **TypeDuck profile storage or schema-id migration:** require explicit scope,
  userdb/cache-key migration evidence, and profile-specific oracle guards.
- **librime C++ plugin ABI:** defer Lua, dynamic octagram plugins, predict, and
  proto until a concrete engine target requires them; prefer Yune-native
  extension points.
- **AI beyond M13:** require an explicit local-first product/engine experiment
  with privacy, memory, and behavior gates.

No deferred item allocates a milestone number or commits to a timeline.

## Principles

The standing principles—librime as oracle, target-driven scope,
support-contract and ABI boundaries, evidence-lane separation, raw-evidence
retention outside Git, and upstream-first oracle sequencing—have one canonical
home: [`conventions.md`](./conventions.md),
[`decisions.md`](./decisions.md), and
[`contracts/engine-support-contract.md`](./contracts/engine-support-contract.md).
