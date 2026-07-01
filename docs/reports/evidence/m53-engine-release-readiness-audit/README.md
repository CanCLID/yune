# M53 Engine Release-Readiness Audit Evidence

Date: 2026-06-30

Scope: docs/evidence-only consistency audit of the engine's launch-facing
surface for downstream engine consumers. No implementation, no optimization, no
ABI change, no new performance success bar. No frontend, browser, product,
package, deployment, or iOS-device proof is made.

## Method

Five dimensions were audited in parallel against the current working tree, and
every non-trivial finding was adversarially re-verified against the actual
files/code before disposition:

1. Support-contract consistency (contract vs conventions, roadmap, requirements,
   ledger).
2. ABI wording vs code (`abi.rs`, `api_table.rs`, `web_runtime.rs`,
   `scripts/yune-web-exports.txt`).
3. M52 guardrail freshness and numeric consistency.
4. Public performance-claim wording (lane-specific, no overclaim).
5. Evidence link/path integrity and 2026-06-30 dashboard-visual adoption.

## Dimension Verdicts

| Dimension | Verdict |
| --- | --- |
| ABI wording vs code | release-ready (every ABI claim backed by code; no drift) |
| M52 guardrail freshness | release-ready (all headline numbers reconcile to the CSVs) |
| Link/evidence integrity | release-ready (all links/anchors resolve; visuals adopted) |
| Support-contract consistency | release-ready after M53 bookkeeping |
| Public claim wording | release-ready after the README/history fixes below |

## Findings And Disposition

| Severity (verified) | Finding | Disposition |
| --- | --- | --- |
| high | `README.md` performance/status made M45-era "faster on seven of ten rows" claims and listed the 37/59-character pinyin rows, startup, and session under "faster than librime" — contradicting the corrected M52 reports it links to | Fixed: repointed to M52 (2026-06-30) numbers; Yune faster only on `zhongguo` + the two abbreviation rows; short keys and 37/59-char are slower; startup/session near parity. Chart repointed to the 2026-06-30 comprehensive visual. |
| medium | `README.md` quoted `127 MB` native Track A memory vs the reports' `188.4 MB` | Fixed: updated to `188.4 MB` vs librime `17.3 MB`; also corrected the browser fair-Luna figure (`64 MiB`, not the Jyutping `160 MiB` guard) and the Jyutping browser figure (`160 MiB` after WEB-03, not the old `893 MB`). |
| low | Archived history report still asserted "several times faster than librime on most native rows" (linked from the live report as "superseded") | Fixed: added a superseded banner reversing the claim and pointing to the live report. |
| low | Contract status line said only "Active after M51 closeout" | Fixed: notes it is unchanged by M52 and re-verified by M53. |
| low | M53 not yet present in requirements coverage counts, roadmap ledger/sequence, or milestone history | Fixed: M53 rows added to all of them as part of this closeout. |
| low | Track Map "Core compatibility" source-of-truth cell did not cite M53 | Fixed: added the M53 audit evidence pointer. |
| info | `track-a-thresholds.csv` `source_value` column holds phase-0 baseline (not final observed) values | No change: the notes column already says "phase-0 baseline"; distinct from `threshold-check.csv` observed values by design. |
| info | Unlinked M42 evidence SVG caption overclaims full-pinyin rows | No change: milestone-scoped historical evidence, correct for the M42 run, and not embedded by any live dashboard. |

## Release-Readiness Verdict

The engine docs are **release-ready for downstream engine consumers** after the
fixes above. The substantive invariants all held with no drift: default
`rime_get_api()` and `RimeCandidate` stay upstream-shaped, TypeDuck/Yune Windows
fork-only slots stay behind their named profile accessors, `yune_web_*` is a
separate WASM ABI with exactly 14 exports, the M52 guardrail numbers reconcile,
all evidence links resolve, and no doc implies Yune is broadly faster than
librime. The only real defects were stale public-facing performance claims in
`README.md` and one linked archived report, now corrected to the M52 lane-specific
numbers.
