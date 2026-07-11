# Increment 4a verification record

Mechanism commit: `ca52ec427111e2ec36b2a80dfe7b25b6f2d3c456`.
Blocking-review fix-forward commit:
`2257fbbe1e8de5ad0e3ac25e45e2e3b07e11878c`.

The owner-provided Opus review found the explicit-false reachability regression
that the original focused packet missed. The before run failed the exact
deployment matrix after `439.35 s`; the fix-forward run passed the same command
after `556.49 s`. Full diagnosis, candidate bytes, review dispositions, and the
post-fix performance guard are in
[`review-fix-forward/`](./review-fix-forward/).

The following completed successfully against `2257fbbe`:

- `cargo fmt --check`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- `cargo test -p yune-core --lib` - 413 passed;
- `cargo test -p yune-rime-api --lib` - 345 passed, 1 ignored;
- `cargo test -p yune-core --test upstream_luna_pinyin_parity` - 14 passed,
  13 ignored;
- `cargo test -p yune-core --test cantonese_parity` - 41/41;
- `cargo test -p yune-core --test upstream_zhuyin_parity` - 3 passed,
  3 ignored;
- `cargo test -p yune-rime-api --test yune_web m59_luna -- --nocapture` -
  4/4;
- `cargo test -p yune-rime-api --test typeduck_windows_boundary` - 4/4;
- exact
  `m59_reachability::m59_schema_general_reachability_deployment_matrix_default_on_and_explicit_false`
  - 1 passed, 49 filtered;
- focused upstream-script policy controls - 4/4; and
- `python -B -m unittest scripts.tests.test_m59_evidence_tools.ResidualClassifierTests`
  - 13/13.

The classifier replay is classification-only. It reports
`classification_status: "complete"`, preserves
`raw_comparator_all_accepted: false`, and reproduces 6,086 inversions with zero
cross-weight inversions. It does not accept D-48.

One clean post-fix expanded Track A/Track B run used a fixed release DLL,
product deployment, `9 / 60 / 80` iterations, the complete 17+1 input set, and
`-FailOnRegression`; all 32 signed rows passed. It is a review guard, not the
five final rounds required by M59-REACH-04.

The full 50-test `yune_web` process remains a final M59-GATES-01 obligation.
The exact slow deployment test that exposed the review blocker is now green;
this packet does not infer that the final native release, WASM, browser,
packaging, or final-commit performance gates have run.
