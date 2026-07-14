# Increment 4c verification record

Measured production source: `e11557e2bbb05e3598e2d96dd6eb669ded88d33d`.
Measured source tree: `48bf8dbad8811e8c23e90af683a4c2bbcd9831cc`.
Release DLL SHA-256:
`5a95705fa33e74dc7954d356339621c1462ddd9198eccf51dbb9b9abbd0f5eb7`.

## External oracle and executable behavior

- Fresh canonical oracle: all 13 rows exact against the pinned oracle;
  refresh SHA-256
  `c7f04fb008ea4e848e3a04f37ddda22a7bcfd4a414e0c448de0446673c071c49`.
- Yune production capture: 13/13 exact, 5,705 candidates across all pages;
  capture SHA-256
  `28baf32eeade77b54b0c8208c32b7f6a072ad4ccf66454cf61a38b351280d099`.
- Strict comparator: pass, no exception/replacement/tail policy; SHA-256
  `5ca42ab5f3330db2f3ee330d3cdc139a28de0b3e5162b7dc06e98bde5eabfb3b`.
- 4c classifier: pass; 83 inventory rows, 64/65 source keys, 14 visible
  occurrences, zero OpenCC residual/exception/beyond-depth; SHA-256
  `fb81935d12e69218f62d6cd7e111788f60d56eb2162d9cdf730b9fe2f86db343`.
- Non-gating comment disclosure: 854 mismatches across nine inputs; ordered
  tuple SHA-256
  `c68be6b33cd519962b09037e7bbca144ebce9b80aa00510551fc6ecc704793b1`.

The independent behavior audit recomputed the source/tree, capture script,
probe, oracle, DLL, shared-tree, and comparator bindings. It confirmed exact
candidate text/order/count/page shape and positions, preedit, commit preview,
and complete pagination for all 13 rows, plus the expected OpenCC sibling
positions and duplicate multiplicities. It independently reran the TypeDuck
profile guard 1/1 and `cantonese_parity` 41/41.

## Focused owning gates

The following completed successfully against the 4c implementation:

- `cargo test -p yune-core`: all unit and integration tests green, including
  `cantonese_parity` 41/41 and fixture provenance 18/18;
- `cargo test -p yune-rime-api --lib`: 356 passed, 1 pre-existing ignored;
- `typeduck_windows_boundary`: 4/4;
- the real deployed TypeDuck `variants_hk` guard: 1/1, preserving the existing
  top five and ordered unique `卧` / `臥` siblings;
- source and byte-backed compiled ABI OpenCC oracle path: 1/1;
- OpenCC capture-tool tests: 6/6, including two fresh byte-identical captures
  and bad-DLL fail-before-mutation;
- Increment 4c classifier tests: 20/20, including hash, mutation, provenance,
  atomic-create, exact-inventory, exception-policy, and non-gating comment
  tamper controls;
- `cargo fmt --check`: exit 0; and
- `cargo clippy --workspace --all-targets -- -D warnings`: exit 0.

The full workspace test and final native release build are final-M59 gates and
are not claimed by this increment packet. No browser or packaging gate is
claimed; source-current WEB-04 remains an explicit 4e/final-closeout blocker.

## Five-round signed ratchet

All five runs bind to the measured source/tree, the same Yune DLL, upstream
`rime.dll` SHA-256
`86b4c7357d4c6d293ce5589b234d8859ca2ac30923a03bedfa3926eeaf97fb0b`,
upstream shared tree
`8b9b75a6559c7adba6b403c3b3c28d83718a7412d4e2f6f0f09dbbcc4787b24c`,
upstream build tree
`27c7612854f7583a39ee1878dc1cf06aa75bd99c3674ae4ccb1ce15828d6771d`,
and product schema tree
`1a61c7f3c759423d11755a7eaecdbedeb23d43bfa37f612ca822ca32b705063e`.

Run 1 built native harness SHA-256
`0be04a93a55deb41359a1a7eeaa1618cf97de50171681f077cd6db0bc9db7e9d`;
runs 2-5 reused those exact bytes and build receipt. The aggregate tool reports
32/32 median rows green over 160 observations. It preserves two individual
failures rather than silently calling 160/160 green. Aggregate verdict SHA-256:
`e2e1ec02fb0d77a2980b98a2308636bfcdea90caa52a741f7fb13bebee92d1b0`.

## Evidence-tool identities

- 4c classifier:
  `58e11dc48446c9746329019798e27ca09b7fe740714e57d1e86c898acab751c6`;
- classifier tests:
  `6ef9249fed876e7b6e8a2ad8e670af1d20df9670f1f668fdaba0b3b72046e828`;
- OpenCC oracle fixture:
  `fafdb3b6ae5f7ac77d797dcc282c359e315669d2bb44bfca804cf7b7c56f8437`;
- complete same-code inventory:
  `01522f437038a3591d3a3b92cbdace2cced1b1e9076e566ca40662c736afcaf1`;
- `HKVariantsFull.txt`:
  `145b561c68a697d5f2197da0c091caf4a0e9457f0a4c56cdf2ae7ad4b8ff8cc2`.

## Review and publication integrity

Two independent final reviews passed with no P1/P2 finding: one for the
specification/acceptance boundary and one for code quality, ABI safety, test
coverage, and evidence integrity. Their verdicts are preserved under
`reviews/`. The staged publication audit reconciles all 260 packet-local files
and 29,498,879 bytes excluding the manifest, every path/size/SHA-256, seven JSON
files, relative links, text-only extensions, NUL checks, and Git-filter identity.
No unrelated path is staged.
