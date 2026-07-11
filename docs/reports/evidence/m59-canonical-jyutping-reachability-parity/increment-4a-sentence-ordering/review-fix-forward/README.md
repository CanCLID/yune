# Increment 4a blocking-review resolution

Status: the owner-provided Claude Opus blocking review (used because Fable was
temporarily unavailable) has been resolved by fix-forward commit
`2257fbbe1e8de5ad0e3ac25e45e2e3b07e11878c`. Increment 4b remains blocked only
on the owner's explicit D-48 re-disposition of the expanded equal-weight
surface. This record does not close Lane A, D-48, or M59.

## Blocking finding and reproduction

The review found that `SentencePolicy::UpstreamScript` could emit a
single-character leading partial independently of
`translator/leading_syllable_reachability`. That bypassed the signed
explicit-false opt-out contract.

The finding was independently reproduced from review target `2b4a169a` with:

```powershell
cargo test -p yune-rime-api --test yune_web m59_reachability::m59_schema_general_reachability_deployment_matrix_default_on_and_explicit_false -- --exact --nocapture
```

The run failed after `439.35 s` at `m59_reachability.rs:568`. The first local
failure was the Bopomofo control, where explicit false still exposed `好` in
`["好你玩", "好", "郝", "𡥆", "𤫧"]`.

Opus's final corrected review reported the same Bopomofo failure and supplied
this reviewer-run parent/target bisection:

- direct parent `212a4a39`: exact matrix passed `1/1` in `537 s`;
- mechanism/review target `ca52ec42` / `2b4a169a`: exact matrix failed on the
  Bopomofo explicit-false control above.

The earlier interim attribution to canonical Jyutping `being -> 畀` was
explicitly retracted after the complete failure output arrived; it is not used
as evidence here. The parent run was not independently repeated for this
resolution packet. Static history confirms that `212a4a39` is the direct parent,
the matrix test already existed there, and `UpstreamScript` first appears in
`ca52ec42`. Our independent behavioral checks reproduced red at `2b4a169a` and
green after the fix at `2257fbbe`.

## Fix

Commit `2257fbbe` keeps the config/profile-derived `UpstreamScript` sentence
policy active but suppresses a one-scalar `PartialTable` candidate only when:

- its consumed span is one of the selected or alternate first-syllable spans;
- `leading_syllable_reachability` is false; and
- independent `prefix_fallback` is also false.

Full-input rows, sentences, multi-character partials, and one-scalar phrases
that consume multiple syllables remain available. The owning regression covers
default on, explicit false, and independent prefix fallback across owned and
byte-backed storage. Two independent internal reviews approved the final
boundary-aware implementation; no ABI surface changed.

## Exact verification

All commands below ran against `2257fbbe`:

- exact default-on/explicit-false deployment matrix: `1 passed`, `49 filtered`,
  `556.49 s`;
- `cargo clippy --workspace --all-targets -- -D warnings`: passed;
- `cargo test -p yune-core --lib`: `413 passed`;
- `cargo test -p yune-rime-api --lib`: `345 passed`, `1 ignored`;
- `upstream_luna_pinyin_parity`: `14 passed`, `13 ignored`;
- `cantonese_parity`: `41 passed`;
- `upstream_zhuyin_parity`: `3 passed`, `3 ignored`;
- four `m59_luna` product/byte-backed controls: `4 passed`;
- `typeduck_windows_boundary`: `4 passed`;
- upstream-script policy controls: `4 passed`; and
- residual-classifier adversarial tests: `13 passed`.

The first detached-worktree Cantonese attempt had `29 passed / 12 failed`
because the ignored `apps/yune-web/source` checkout was absent. It is not
counted as a behavior result. After wiring the verified TypeDuck source checkout
through a junction, the complete rerun passed `41/41`; no golden changed.

The classifier output schema is now explicit:
`classification_status: "complete"` means every residual was classified;
`raw_comparator_all_accepted: false` continues to state that D-48 is not yet
accepted. Tool version 2 reproduces the same `6,086` inversions and zero
cross-weight inversions.

## Post-fix performance guard

A clean detached worktree at `2257fbbe` built a fixed 3,892,224-byte release DLL
with SHA-256
`9873dae8fac00753544ce49fb5816976362bf95eac576bacaf6d74f6f55b4d1b`.
One fresh `9 / 60 / 80` expanded Track A plus product Track B run used product
deployment, the unchanged signed thresholds, and `-FailOnRegression`. All
`32/32` rows passed:

| Row | Observed | Ceiling | Verdict |
|---|---:|---:|---|
| `n` | 2.743 | 3.006 | pass |
| `ni` | 1.937 | 2.666 | pass |
| `hao` | 1.295 | 1.844 | pass |
| 37 characters | 0.227 | 2.339 | pass |
| 59 characters | 0.126 | 1.748 | pass |

The product source tree re-hashed to
`1e0c9ccfa2a208af0359a29a6e6b6153dd2d632a977e875858d86c3ce2cdd046`,
the same 48-file tree used by the accepted five-round 4a packet. Curated text
evidence is under [`performance-ratchet/`](./performance-ratchet/); binaries,
deploy trees, raw samples, and `.marisa` payloads are excluded. This single run
is a fix-forward guard, not M59-REACH-04's final five-round acceptance.

## Disposition still requiring the owner

The recommended renewed D-48 class-3 exception is limited to the complete
captured equal-weight residual: after the already declared OpenCC normalization,
`6,086` inversions, zero cross-weight inversions, and no beyond-oracle-depth
use. The cause is librime's equal-weight import/traversal tie-break. Increment 4c
still owns the two OpenCC one-to-many rows. Revisit is mandatory for any
cross-weight inversion, incomplete capture/provenance, or a tie residual moving
onto page 1 for a common input. Signing this exception does not close Lane A or
waive 4b/4c/4d/4e.

## Nonblocking findings retained

- The 5% reading threshold is not a Yune heuristic. Pinned librime
  [`EntryCollector::TranslateWord`](https://github.com/rime/librime/blob/33e78140250125871856cdc5b42ddc6a5fcd3cd4/src/rime/dict/entry_collector.cc#L223-L244)
  uses the same inclusive threshold, and
  [`ScriptEncoder::DfsEncode`](https://github.com/rime/librime/blob/33e78140250125871856cdc5b42ddc6a5fcd3cd4/src/rime/algo/encoder.cc#L292-L333)
  consumes that filtered family. The shared constant and owning 4%/5% test now
  cite the pinned source.
- Source-only script deployment still serializes and reparses an internal prism
  with `.expect()`. A proper repair requires a typed fallible prism builder and
  fail-closed schema-install error propagation. It remains a named P3 follow-up,
  not part of this blocker fix.
- Rare-reading-dependent long sentences and the canonical toned lattice remain
  watch items for later 4a/final performance and parity coverage; the final M59
  five-round and full release/WASM/browser gates remain open.
