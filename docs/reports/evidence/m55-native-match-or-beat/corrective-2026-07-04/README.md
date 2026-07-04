# M55 Corrective Re-Baseline - 2026-07-04

This directory records the corrective series applied after the M55 final
closeout (`531dbcf2`) was independently reviewed and three of its headline
results were found to be measurement artifacts rather than engine wins. The
corrective series preserves everything real that M55 landed, removes the
artifact mechanisms, makes the benchmark structurally resistant to the same
class of gaming, and re-baselines the standing gate on honest numbers.

## What the review found (all verified with file:line evidence and probe runs)

1. **Key deferral** (`b0fb8cc9`): `RimeProcessKey` for `luna_pinyin` lowercase
   keys buffered the character with zero engine work and flushed only at the
   first observable read. The benchmark read context once per sequence, so the
   per-key medians measured one flush divided by N keys. The repo's own probe
   evidence shows the 59-char row moving `1.519x -> 0.004x` from that commit
   alone (`phase-3r-native-lazy-refresh-probe/`). A real frontend reads
   context after every keystroke; the deferral also reported deferred keys as
   consumed without evaluating the pipeline.
2. **Benchmark-input aliases**: `short_key_exact_alias_for_prefix`
   (`n -> na`, `h -> ha`) special-cased exactly the Track A short keys and
   skipped the completion/prefix scan every other single letter receives. On
   the committed dictionary the visible first page happened to be unchanged
   (verified post-revert), but the synthetic tests proved dropped entries, and
   input-specific lookup shortcuts targeting the gate's own key set are
   forbidden by the plan regardless of visible effect.
3. **Uninvalidated process-global config cache**: mtime+length-keyed parsed
   YAML, surviving `RimeFinalize`, no invalidation anywhere - converted the
   in-process startup/session metrics into cache-hit measurements
   (startup `1.153x -> 0.286x` from the cache alone) and constituted a
   WEB-02-class staleness hazard for real deployments.

The full audit trail (claim-integrity audit, caching-semantics audit, and the
independent gate reproduction) is summarized in the corrective commits'
messages; the gamed-era evidence dirs are intentionally preserved unmodified.

## Corrective series

1. `Revert M55 native lazy refresh key deferral` - code reverted, probe
   evidence preserved.
2. `Remove benchmark-input short-key aliases n->na and h->ha`.
3. `Remove uninvalidated process-global runtime config cache` (a
   content-hashed, deploy-invalidated cache may return under M56's
   staleness-proofing track).
4. `Read context per keypress inside the timed benchmark loop` - the
   structural fix: the metric now bills every keystroke for the work it
   forces, identically for Yune and librime. All pre-corrective
   `key_sequence_process_with_context` numbers are batch-shaped and not
   comparable.
5. `Flip byte-backed poet back to explicit opt-in` - decided by same-run
   evidence (below): the owned path holds the latency ceilings, byte-backed
   does not (the incremental sentence scratch only works on owned storage).
6. Corrective thresholds re-baseline (this directory) - the standing gate is
   re-derived from the honest per-key runs; win rows guarded `<1.00x`.
7. Dashboards/README/requirements truth pass.

## Run inventory (all same-run vs upstream librime 1.17.0, per-key harness)

| Run | Mode | 37-char | 59-char | n / ni / hao | Track A peak | Purpose |
| --- | --- | ---: | ---: | --- | ---: | --- |
| `run-a-byte-backed-default/` | byte-backed | `4.619x` | `3.227x` | `2.741/2.496/1.650x` | `113.2 MB` | default decision |
| `run-b-owned-optout/` | owned | `1.994x` | `1.548x` | `2.752/2.539/1.649x` | `185.5 MB` | default decision + derivation |
| `run-c-owned-default/` | owned (default after flip) | `1.945x` | `1.520x` | `2.724/2.503/1.631x` | `185.7 MB` | derivation |
| `gate-run-d/` | owned default | gate | gate | gate | gate | first `-FailOnRegression` green |
| `gate-run-e/` | owned default | gate | gate | gate | gate | second consecutive green |

Startup `0.949/0.939x`, session absolute `23.2/22.4 ms` (owned runs, no
config cache). Track B (mode-independent; sentence off in the mobile
profile): key row `~316 us` vs the `341 us` Phase 0 source baseline; startup
and session absolutes improved to `~35-36 ms` against Phase 0 ceilings near
`107 ms` - real improvements from the landed M55 work, ratcheted accordingly.

## The honest M55 ledger (what survives, vs the pre-M55 M52 record)

Metric semantics changed in part 4 (per-key context reads), so pre/post
ratios are directionally comparable but not identical-basis; the
pre-corrective owned-path measurement on the old batch shape was 37-char
`2.001x` / 59-char `1.519x`, in line with the per-key numbers below.

- 37-char sentence: `3.05x` -> **`~1.97x`** (real graph/DP-reduction work)
- 59-char sentence: `2.25x` -> **`~1.53x`**
- `ni`: `3.14x` -> **`~2.52x`**; `hao`: `2.15x` -> **`~1.64x`**; `n`: `2.82x`
  -> **`~2.74x`**
- startup: `1.11x` -> **`~0.94x`**; session: parity -> **`0.78-0.97x`**
- Win rows kept and improved; Track B product improved on every guard row
- Track A peak: `188.4 MB` -> `185.5 MB` owned default; **`113.2 MB`
  available behind `YUNE_POET_BYTE_BACKED=1`** (fails long-row latency until
  the incremental scratch is ported to byte-backed storage - the named
  follow-up owner)
- 11 oracle sentence fixtures added; 13 rows recorded as named blocked
  `#[ignore]` tests exposing **pre-existing** sentence-lattice mismatches
  (e.g. the 37-char top candidate differs from librime and did before M55) -
  a correctness debt now on the record, owned by future parity work
- Candidate-page state vs librime on the benchmark rows (post-corrective,
  `run-b`/`run-c` snapshots): `ni`, `hao`, `cszysmsrsd`, `zybfshmsru` match;
  `n`, `zhongguo` (completion ranking) and the 37/59-char sentence tops
  (lattice) differ - all pre-existing gaps, none introduced or fixed by the
  corrective series

## What M55 does NOT claim after this correction

- No match-or-beat claim on any latency row still measured above `1.00x`.
- No "faster than librime" startup/session claim beyond the measured
  `0.78-0.97x` same-run medians with the noise caveat.
- No candidate-output-identity claim on the benchmark first pages; identity
  holds on the oracle fixture suites and on 4 of 8 benchmark rows.
- No 113 MB memory claim for the shipping default; that number requires the
  opt-in flag and currently costs long-row latency.

`../phase-5-final/review-verify-run/` is the independent reproduction of the
gamed gate that triggered this review, kept as part of the record.
