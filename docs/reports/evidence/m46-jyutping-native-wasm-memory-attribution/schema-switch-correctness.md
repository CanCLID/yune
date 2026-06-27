# M46 Phase 0 Schema-Switch Correctness

Evidence:
[`../../../../apps/yune-web/e2e/results/yune-web-jyutping-memory-attribution/phase-0-schema-switch-current/`](../../../../apps/yune-web/e2e/results/yune-web-jyutping-memory-attribution/phase-0-schema-switch-current/)

## Final Verdict

`schema-switch-correctness-fixed-memory-unchanged`

Phase 0 reproduced a correctness failure in a real multi-schema browser
session: Cangjie -> Luna -> Jyutping left the active schema at `jyut6ping3`,
but `nei` returned zero Jyutping candidates.

Branch A fixes the correctness half. Post-fix browser evidence shows clean
Jyutping, Cangjie -> Luna -> Jyutping, and Jyutping -> Luna -> Jyutping all
return the expected top candidate with no worker action errors.

Clean Jyutping startup is not broken. A fresh page with only Jyutping selected
returns `nei -> first candidate index 0` with six candidates.

## Current Runtime Evidence

| Step | Active schema | Input | Expected | Result | Candidate count | WASM peak |
| --- | --- | --- | --- | --- | ---: | ---: |
| clean page | `jyut6ping3` | `nei` | Jyutping candidate | pass | 6 | `893.1 MiB` |
| Cangjie step | `cangjie5` | `a` | Cangjie candidate | pass | 6 | `893.1 MiB` |
| Luna step | `luna_pinyin` | `hao` | Luna candidate | pass | 6 | `893.1 MiB` |
| Jyutping after switch | `jyut6ping3` | `nei` | Jyutping candidate | fail | 0 | `893.1 MiB` |

The new structured capture did not reproduce the WEB-01 `~1.9 GiB` high-water;
it stayed at `893.1 MiB`. The no-candidate correctness half did reproduce.

## Branch A Fix Evidence

Evidence:
[`../../../../apps/yune-web/e2e/results/yune-web-jyutping-memory-attribution/branch-a-final-after-reverse-assets/`](../../../../apps/yune-web/e2e/results/yune-web-jyutping-memory-attribution/branch-a-final-after-reverse-assets/)

| Scenario | Result | Final Jyutping step | Candidate count | Worker action errors | Max observed WASM |
| --- | --- | --- | ---: | ---: | ---: |
| clean Jyutping | pass | `nei -> 你` | 6 | 0 | `893.1 MiB` |
| Cangjie -> Luna -> Jyutping | pass | `nei -> 你` | 6 | 0 | `893.1 MiB` |
| Jyutping -> Luna -> Jyutping | pass | `nei -> 你` | 6 | 0 | `893.1 MiB` |

Additional focused behavior evidence:

- [`../../../../apps/yune-web/e2e/results/m46-branch-a-m22-reverse-after-schema-fix/`](../../../../apps/yune-web/e2e/results/m46-branch-a-m22-reverse-after-schema-fix/)
  proves the M22 Cangjie and Luna reverse lookup row.
- [`../../../../apps/yune-web/e2e/results/m46-branch-a-behavior-gates-final/`](../../../../apps/yune-web/e2e/results/m46-branch-a-behavior-gates-final/)
  proves schema switching, reverse lookup, userdb persistence, Shift ASCII, and
  candidate commit after the fix.

The fix has two owners:

- `apps/yune-web/src/App.tsx` no longer performs a redundant default
  `customize()` + `deploy()` on the final unchanged Jyutping schema selection.
  The pre-fix assertion gate recorded that deploy as a worker action error with
  a WASM allocation abort.
- `crates/yune-core` / `crates/yune-rime-api` now let a reverse lookup
  translator honor its own namespaced spelling algebra, and
  `apps/yune-web/public/schema/cangjie5.schema.yaml` uses the plain
  `jyut6ping3` dictionary with tone-stripping for Cangjie reverse lookup. The
  browser worker also ships the missing Luna/Jyutping source dictionaries
  needed by those reverse lookup paths.

This is a correctness fix, not a memory optimization. The post-fix browser
high-water remains `936,509,440 B` (`893.1 MiB`), so M46 does not claim browser
WASM memory success.

## Pre-WEB-01 Classification

A live pre-WEB-01 executable rebuild was not attempted in this Phase 0 run
because the current direct-on-main worktree already contains M46 instrumentation
and evidence edits. The practical baseline used here is the published WEB-01
final gate:
[`../../../../apps/yune-web/e2e/results/yune-web-wasm-heap-optimization/final/final-gates.md`](../../../../apps/yune-web/e2e/results/yune-web-wasm-heap-optimization/final/final-gates.md).

That gate already recorded the same Cangjie -> Luna -> Jyutping no-candidate
shape, with an older `~1.9 GiB` memory observation. M46 Phase 0 therefore
classifies the correctness issue as present on current post-WEB-01 main, but
does not prove whether WEB-01 introduced it.

## Product Impact

Before Branch A, the failure affected the real `apps/yune-web` multi-schema
product flow for users who switched through Cangjie and Luna before returning
to Jyutping in the same page/session. It did not affect the clean single-schema
Jyutping startup flow tested here.

After Branch A, that correctness blocker is fixed for the tested flows.
Severity for the old failure was high for multi-schema browser sessions; the
remaining blocker is memory attribution, not candidate availability.

## Owner

The owner was split between browser schema lifecycle and reverse lookup
dictionary semantics, not a native compact-table storage rewrite:

- the page reports the active schema as `jyut6ping3`;
- the worker initializes and previous Cangjie/Luna steps produce candidates;
- a fresh Jyutping page produces candidates;
- the failure appears only after multi-schema state has accumulated.

Branch A crossed into `crates/yune-rime-api` only for the reverse lookup
translator path required by the Cangjie/Luna behavior gate. It did not change
default `rime_get_api()` or TypeDuck profile ABI surfaces.
