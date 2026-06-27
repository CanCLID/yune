# M46 Branch A Closeout

Date: 2026-06-27

Verdict: `schema-switch-correctness-fixed-memory-unchanged`

Branch A fixed the product-affecting multi-schema browser correctness bug that
Phase 0 selected as the mandatory first branch. It does not reduce the native
Track B memory headline or browser Jyutping WASM high-water, so M46 closes as a
useful partial result with a measured memory no-go/unclassified blocker.

## What Changed

- `apps/yune-web/src/App.tsx` skips the redundant default deploy on unchanged
  Jyutping schema selection. The pre-fix assertion gate showed the old deploy
  path aborting with a WASM allocation failure after Cangjie -> Luna ->
  Jyutping.
- `apps/yune-web/src/worker.ts` ships the Luna and Jyutping source dictionaries
  needed by the packaged reverse lookup paths.
- `apps/yune-web/public/schema/cangjie5.schema.yaml` restores Cangjie reverse
  lookup through the plain `jyut6ping3` dictionary with tone stripping while
  keeping the existing scolar dictionary lookup filter for TypeDuck comments.
- `crates/yune-core` and `crates/yune-rime-api` let reverse lookup translators
  honor their own namespaced spelling algebra.

## Behavior Evidence

| Gate | Result | Evidence |
| --- | --- | --- |
| Native Jyutping reverse lookup | Pass | `cargo test -p yune-rime-api select_schema_served_jyutping_mobile_routes_bare_grave_to_luna_reverse_lookup -- --nocapture` |
| Native Cangjie reverse lookup | Pass | `cargo test -p yune-rime-api select_schema_served_cangjie_routes_grave_jyutping_reverse_lookup -- --nocapture` |
| Browser M22 reverse lookup | Pass | [`../../../../apps/yune-web/e2e/results/m46-branch-a-m22-reverse-after-schema-fix/`](../../../../apps/yune-web/e2e/results/m46-branch-a-m22-reverse-after-schema-fix/) |
| Browser Branch A behavior set | Pass | [`../../../../apps/yune-web/e2e/results/m46-branch-a-behavior-gates-final/`](../../../../apps/yune-web/e2e/results/m46-branch-a-behavior-gates-final/) |
| Browser schema-switch attribution | Pass | [`../../../../apps/yune-web/e2e/results/yune-web-jyutping-memory-attribution/branch-a-final-after-reverse-assets/`](../../../../apps/yune-web/e2e/results/yune-web-jyutping-memory-attribution/branch-a-final-after-reverse-assets/) |

Post-fix schema-switch attribution:

| Scenario | Final Jyutping result | Candidate count | Worker action errors | Max observed WASM |
| --- | --- | ---: | ---: | ---: |
| clean Jyutping | `nei -> 你` | 6 | 0 | `893.1 MiB` |
| Cangjie -> Luna -> Jyutping | `nei -> 你` | 6 | 0 | `893.1 MiB` |
| Jyutping -> Luna -> Jyutping | `nei -> 你` | 6 | 0 | `893.1 MiB` |

## Memory Verdict

Branch A did not move memory:

- Native Track B remains at the Phase 0 baseline: peak `504,627,200 B`, steady
  resident `427,356,160-442,966,016 B`, and mostly unclassified process memory.
- Browser Jyutping remains `936,509,440 B` (`893.1 MiB`) for clean and
  schema-switch scenarios.
- The older WEB-01 `~1.9 GiB` schema-switch high-water did not reproduce in
  structured M46 captures before or after the fix.

M46 therefore records the remaining memory owner as
`measured-no-go-owner-unclassified`. A future memory milestone needs a larger
measured owner before attempting candidate payload interning, scolar deferral,
fresh Track B `rsmarisa`, or another storage rewrite.
