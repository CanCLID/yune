# Increment 4b verification record

Production behavior/performance source: `d508e05b638fc21de7f8f8dfc45c82d33a8bbde8`.
Lifecycle integration-test alignment: `c279755955c630755e1c74e2ea333760247b1bf3`.
The later `0ba07cf6` changes only repo-local Codex model policy and does not
change the measured Rust, schema, product, or evidence-tool sources.

## Focused owning gates

The following completed successfully during implementation:

- `cargo test -p yune-core`: 478 unit tests plus all owning integrations;
- `cargo test -p yune-rime-api --lib`: 355 passed, 1 pre-existing ignored;
- `cantonese_parity`: 41/41;
- `typeduck_windows_boundary`: 4/4;
- `cold_start_conformance`: 1/1;
- upstream Luna parity: 25 passed, 3 pre-existing capture-only ignored;
- fixture provenance: 18/18;
- exact deployed 37/59 Luna product page test: 1/1;
- schema-general default-on/explicit-false deployment matrix: 1/1;
- Cangjie, Double Pinyin, Bopomofo, and transformed-algebra owning suites;
- Increment 4b residual-classifier tests: 21/21;
- M59 evidence-tool tests: 78/78;
- benchmark-script tests: 18/18;
- `frontend_client` after lifecycle assertion alignment: 35/35 in parallel; and
- bounded same-text upgrade and saturated-worst equal-weight regressions: 2/2
  on both owned and byte-backed Poet storage;
- oversized uncached prefix-family bounded/complete regression: 1/1; and
- exact WEB-03 long-input expansion guard at `d508e05b`: 1/1, independently
  rerun 1/1.

## External behavior and performance

- Raw strict class-4 comparator: 1 passed / 4 failed, expected exit 1.
- Declared OpenCC-normalized candidate text/position: 5/5.
- Preedit/segmentation: 5/5.
- Commit preview: 5/5.
- Unowned residuals: 0; exception: false; beyond-depth: false.
- Final lazy versus pre-lazy behavior signature: exact 5/5.
- Five source-bound performance rounds: 32/32 aggregate rows and 160/160
  individual observations pass; no threshold changed. The source-bound harness
  mode is exactly `build,reuse,reuse,reuse,reuse`.

## Release-gate findings

The literal release sequence was not inferred from focused tests:

1. The first `cargo clippy --workspace --all-targets -- -D warnings` exposed two
   closure type-complexity findings and one Rust-1.76 MSRV violation. The
   behavior-preserving repair is `4bed300e`; its first source-bound behavior and
   performance rerun was later rejected by finding 3 below.
2. The first literal `cargo test --workspace` then exposed a stale
   `frontend_client` assertion: workspace deployment correctly invalidates the
   live session under the approved D-32 lifecycle, while the test expected the
   old handle to survive. `c2797559` aligns that assertion with the contract.
3. Independent final review found that replacing a saturated bounded-collector
   row in place could reverse equal-weight source order, and that a later better
   duplicate had the same defect. `eb117c53` removes the displaced row and
   appends the later emission in both paths; owned and byte-backed
   bounded-versus-complete regressions cover both counterexamples. The prior
   green packet is preserved as rejected review evidence, and all accepted
   behavior/performance evidence was regenerated from `eb117c53`.
4. The first clean-detached `cargo test --workspace` attempt passed every
   preceding binary but ended with 47 passed / 2 failed / 2 ignored in
   `yune_web`: both failures were setup panics because the gitignored generated
   `apps/yune-web/public-demo/dist/schema` tree was absent. That attempt is
   preserved under `native-gates/` and is not counted green.
5. After mounting the required source-current generated tree, the exact WEB-03
   long-input test exposed a real `eb117c53` regression: 442,856 prefix views
   against a 5,000 ceiling. Intermediate 6,224/6,000 and 6,044/6,000 fixes are
   preserved as red diagnostics. `d2499358` bounded that scan, but a later
   current-head review rejected its equal-weight source order. `c5d954e2`
   restores librime's per-chunk current-head order and `d508e05b` preserves
   field-identical bounded/complete qualities. The exact WEB-03 gate and an
   independent rerun both pass 1/1. All earlier packets remain named rejected
   history; accepted behavior and performance evidence was regenerated from
   clean `d508e05b`.

## WEB-04 disclosure

The source-current committed WEB-04 Playwright gate is red. With its two stale
plain-control literals refreshed only in a scratch copy from the pinned null-
grammar fixture, Octagram ranking is `2/4`: `youhuiyong` and the long control
pass; `jintianhuiyi` and `jintianwanshangyouhui` remain plain-ranked. Native
diagnostics reproduce the same `2/4` result at pre-4b `afb7079b` and final 4b
source `d508e05b`. Although schema installation assigns the Standard
`UpstreamScript` policy, its surface-translation branch explicitly returns
`None` for the structurally untoned Luna dictionary, so this is recorded as the
existing Increment 4e Lane-B
merge/order blocker and a live regression of completed WEB-04. No broad web-
gate pass is claimed for 4b, and M59 cannot close until 4e restores WEB-04 4/4.

## Final native release gate

The exact native sequence was rerun successfully after the production tree was
refrozen at `d508e05b`:

- `cargo fmt --check`: exit 0;
- `cargo clippy --workspace --all-targets -- -D warnings`: exit 0;
- `cargo test --workspace`: exit 0 in 2,522.9 seconds, including `yune-core`
  484/484, `yune-rime-api` 355 passed plus 1 ignored, cold-start 1/1,
  `frontend_client` 35/35, TypeDuck-Windows 4/4, and `yune_web` 49 passed plus
  2 evidence-only ignored; and
- `cargo build --release -p yune-rime-api`: exit 0.

The execution HEAD was policy-only `0ba07cf6`; committed and working production
paths were verified unchanged from d508. The full receipt is
[`native-gates/passed-d508-production-code.txt`](./native-gates/passed-d508-production-code.txt).
No earlier focused result substitutes for this literal sequence, and the native
pass does not overwrite the separate WEB-04 red disclosure.

## Review boundary

Earlier implementation reviews and an independent tooling review found no 4b
mechanism, anti-gaming, or source-bound-receipt blocker. Two final independent
packet reviews approved the specification/acceptance boundary and the
code-quality, ABI, lifecycle, and evidence-integrity boundary after directly
rechecking the native receipt and all packet files. The final publication audit
reconciled every path and byte count recorded by the manifest; all paths, sizes,
and SHA-256 hashes match, all 29 JSON files parse, relative
links resolve, only declared text extensions are present, and no NUL byte is
present. Because 313 imported performance receipts intentionally retain their
original Windows line endings, the scoped `.gitattributes` rule disables text
normalization only for that raw evidence subtree. `git hash-object --path`
versus `--no-filters` is identical for all 384 inventoried paths, proving the
manifest also matches the bytes Git will commit.

The designated external Opus review independently reran the load-bearing gates
and approved 4b with no blocking finding. Increment 4c may begin. Its complete
verdict and the carried nonblocking watch items are recorded in
[`external-review-opus.md`](./external-review-opus.md).
