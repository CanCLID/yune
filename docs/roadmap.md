# Roadmap

Yune is a Rust input-method engine that uses upstream librime as a
compatibility oracle while building a cleaner, AI-native-capable engine. This
file is the live operating dashboard: it records current state, the next
authorized sequence, scope boundaries, and readiness gates. Completed
milestone detail belongs in
[`ledgers/milestone-history.md`](./ledgers/milestone-history.md), completed
plans, reports, and evidence packets.

Current status: M60 and M61 are complete and WEB03-11 remains closed as
maintenance. M61 completed the binding Windows attribution sequence and closed
with disposition D: the correction-source diagnostic passed candidate, Track B,
signed, and projected memory-cap checks, but failed owner reconciliation.
Correction `91f59696` was explicitly reverted by `01a62f2a`, restoring runtime
tree `f1c36a0079d85628f5cbef140bd94288930cc2e8`. No production-default memory
reduction or supplemental-ratchet claim was accepted. Closeout also preserves
the literal workspace red and uses two cfg(test)-only contract corrections plus
nonduplicative owning-target/never-reached recovery; no runtime behavior changed.
No numbered milestone is currently active. Current performance results and
bottleneck analysis live only in the
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
- [`plans/completed/m60-plan-schema-general-single-character-reachability.md`](./plans/completed/m60-plan-schema-general-single-character-reachability.md)
  — the completed schema-general reachability formalism record.
- [`plans/completed/m61-plan-native-track-a-memory-owner-reduction.md`](./plans/completed/m61-plan-native-track-a-memory-owner-reduction.md)
  — the completed Windows Track A measured partial/no-go record.
- [`plans/`](./plans) — active, reference, and completed execution records.

## Current Snapshot

| Lane | Current state | Next decision or gate |
| --- | --- | --- |
| Schema-general reachability | M59 ships default-on single-character reachability for every current and future schema automatically on install. M60 binds it to the canonical contract, retained `version: m59-reach03-v1`, `reachabilityFormalismVersion: m60-reachability-v1`, exactly `reachabilityOptOuts: []`, classified tracked roots, a production-semantic Rust audit, and blocking-open onboarding. Final native behavior/performance authority remains `443cc636`; browser/WASM/package closeout remains bound to `5fa986d8`, with manifest follow-up `07845e02`. | Keep the M60 contract and mandatory checker fail closed. Any future opt-out requires a separately approved complete row and an exact explicit-false bijection. |
| Native performance | The signed M59 Windows ceilings remain authoritative. Final Windows source `443cc636` passes `32/32` aggregate and `160/160` individual observations. The reviewed Mac packet at `0111cf47` is diagnostic and source-unmatched. | Keep the Windows ratchet unchanged. Any causal platform claim or optimization proposal needs fresh same-source, same-lane evidence and an identified structural owner. |
| Native memory | M61 completed the exact-source Windows owned/byte-backed diagnostic. Correction source `91f59696` reduced Track A peak working-set median from `154,030,080 B` to `116,162,560 B` and owner-snapshot private median from `108,482,560 B` to `83,386,368 B`, but named owners explained only `74.61%` of the private delta and the `6,371,950 B` residual exceeded its `5,019,238 B` bound. Disposition D restored runtime tree `f1c36a…`; no default memory win is claimed. | Keep M61 closed. Any future native-memory proposal requires a new plan, fresh source-bound attribution, and a newly authorized owner; do not retry or extend the exhausted M61 branch. |
| Web harness | WEB03-11 remains closed at clean source `ef485b10`; its deployment-maintenance delivery boundary is active at clean source `d5f2ca7b`. The unchanged local gate passed 8/8 scenarios and 186/186 keys, the source-pinned preview canary passed, the identical archive was promoted, and production verified all 11 required files. Cloudflare Git auto-builds are disabled and credentialed jobs reassert that interlock. This is not a refresh of M59 browser authority or the browser peer lane. | Keep the source classifier, secret-free certification, one preview canary, identical-byte promotion, and production hash verification fail-closed. Preserve every measured red; use a fresh explicitly named run only for a diagnosed setup failure. |
| AI-native layer | M11/M13 proved a default-off local AI layer above the deterministic engine. | Keep AI outside the classic path unless a named experiment explicitly enables it and owns privacy, memory, and behavior evidence. |
| Platform products | TypeDuck-Windows product/frontend work has transferred to `CanCLID/yune-windows`. | Bring back only a separately proposed engine, package, or profile-API requirement with oracle evidence and owning tests. |

## Authoritative Sequence

1. **Preserve the completed M60/M61 boundaries.** Keep the schema-general
   reachability contract, default runtime tree, signed registry, and M61
   measured-red evidence unchanged.
2. **Do not continue the exhausted M61 branch.** A future native-memory effort
   requires a new plan and fresh owner evidence; the diagnostic correction,
   projected supplemental pass, and working-set movement are not an accepted
   production reduction.
3. **Open other future work only through its owning lane.** Browser memory,
   Apple-device validation, product-profile storage, schema-id migration, and
   AI remain separate scopes.

Trigger-gated only: extracting the full processor pipeline from
`yune-rime-api` into `yune-core` waits for a real non-ABI consumer. Do not open
a milestone for speculative layering work.

## Current Guardrails

- **Reachability:** M59's schema-general default-on guarantee is shipped and was
  not reopened by M60. No per-schema enablement, input allowlist, baked oracle
  output, or silent onboarding exclusion is acceptable. No shipped opt-out is
  currently approved; M60 added the explicit empty array and exact formalism
  version above. An opt-out requires a complete owner-approved record. The
  [canonical contract](./contracts/schema-general-reachability.md) and mandatory
  checker remain the fail-closed authority.
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
"188 MB gap." That number is not a current target or portable counter. M61
replaced it with a fresh exact-source Windows baseline, owner reconciliation,
and a predeclared reduction bar while keeping memory and CPU independently
gated. That experiment closed as a measured partial/no-go without a
production-default reduction. Historical outcomes remain in the
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
| Default-on single-character composition for every current and future schema automatically on install, with zero per-schema adaptation; M60 formalizes and audits this shipped guarantee | Future opt-out or onboarding-formalism evolution only after a separately approved need | Per-input hardcoding, baked oracle data, schema allowlists, or silent unsupported onboarding |
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
  allocator, or keyboard-startup work remains separate from the completed M61
  Track A no-go and requires fresh product-lane ownership.
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
