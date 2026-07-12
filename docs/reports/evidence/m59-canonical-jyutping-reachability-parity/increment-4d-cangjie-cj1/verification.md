# Increment 4d verification record

Measured production source: `38e759f6ac0c79512713c33533df465e908538db`.
Measured source tree: `948504ed5ef79771cf647e8ad9429fcfb5f43fde`.
Release DLL SHA-256:
`a0fe13b7a5df3669d09425f48adaa5e2821ac578df00962cffc2d95a0420652e`.

## Executable behavior

- Marked all-page capture SHA-256:
  `49caff1a77cc3f9d13e673401cb22371d535a8b48a65e0c5942f7893aea0b3c1`.
- Marked strict comparator SHA-256:
  `f35c3641f60f05827abb4e066238cd7db4dff0f4993cf8e72b12031d53980d82`;
  `12/12` exact with no exception, replacement, or tail policy.
- Unmarked control capture SHA-256:
  `fac6140a02b1c522c62791aa801643d3293c4ed2d1ed4f5d100804f7db9e11d6`.
- Unmarked-vs-`fd6bd2a7` comparator SHA-256:
  `5485e78f54fd7d1b1c0b84578673bd91bebee4d4d61be7a3e13b9a5fbc8efe37`;
  `12/12` exact.
- Staging manifest SHA-256:
  `113573b3b6a2e5cb335635cc8de6b9016b85ea3e9d30ff8cd431c8b9edf11c7b`.

The staging manifest proves a clean create-new overlay: source shared tree
`a17ab574...`, marker `7a87b53e...`, source schema `eb9397f2...`, staged schema
`34b037fe...`, and staged shared tree `ef1ada0f...`. The Yune capture then
records that staged tree as its source tree and records its separately staged
user/build tree. The unmarked control records the original source shared tree.

## Focused owning gates

The following completed successfully against the final 4d implementation:

- `yune-core` library: 521/521;
- `yune-rime-api` library: 362 passed, 1 pre-existing ignored;
- Cangjie composition parity: 3/3, 0 ignored;
- reverse graph owning tests: 11/11;
- predictive hardening: 8/8;
- reverse-path hardening: 3/3;
- owned/byte-backed Poet engine parity: 2/2;
- loader laziness: 1/1;
- competing-fixture generator tests: 7/7;
- schema-overlay staging tests: 12/12;
- `cargo fmt --check`: exit 0;
- `cargo clippy --workspace --all-targets -- -D warnings`: exit 0; and
- `git diff --check`: exit 0.

The full `cargo test --workspace` wrapper reached the known slow `yune_web`
integration and exceeded both the initial two-minute and extended ten-minute
wrappers. It is therefore not claimed by this increment. The exact full
workspace/native release gate remains mandatory at final M59 closeout.

## Public-API predictive-code audit

The external, source-preserved audit opens the deployed compiled table and
prism through public `yune-core` APIs. It independently checks owned and
byte-backed predictive traversal for prefix `k`:

```text
syllabary_count=19975
owned_canonical_k_count=504
byte_backed_canonical_k_count=504
```

It asserts `504` for both paths and full row equality. The audit source SHA-256
is `121a4b696ba05b27d644ad0c05f9b07c7b779b76af5eaeaf7dc2dbc72b3e91bc`.

## Five-round signed ratchet

All runs bind to the measured source/tree, the same Yune DLL, upstream
`rime.dll` SHA-256
`86b4c7357d4c6d293ce5589b234d8859ca2ac30923a03bedfa3926eeaf97fb0b`,
upstream shared tree
`1c7542590f2f7fb22ed2e656c8cd0b67343f9df88ad4c11a9528456d6e60c5e2`,
upstream build tree
`15606d7783e9ab9136cb82acbd412c425a976f9d11156505de4a79fb96433997`,
and product schema tree
`1a61c7f3c759423d11755a7eaecdbedeb23d43bfa37f612ca822ca32b705063e`.

Run 1 built native harness SHA-256
`3e750729bedcf3675cc96343ec75f3e5a185bb453007e974cc700227a41523c5`;
runs 2-5 reused those exact bytes and build receipt. The aggregate tool reports
32/32 median rows green over 160 observations, with zero individual failures.
Aggregate verdict SHA-256:
`3556fd85ed0b915c4f3a001f493eae734d51c3c86532b73827aca44f63c3b644`.
Aggregate provenance SHA-256:
`197cb2419dd4d60b3e25cc8078885768318dbf425867c632d4b3ae549c4fe965`.

## Reviews and close boundary

Three independent final passes returned APPROVE with no blocking finding:
specification/acceptance, code quality/ABI/test coverage, and schema-staging
runner safety/reproducibility. Their scopes are preserved under `reviews/`.
Increment 4d closes M59-PARITY-03 only. Lane B, REACH-03/04, final evidence,
final release/browser/package gates, and M59 remain open.
