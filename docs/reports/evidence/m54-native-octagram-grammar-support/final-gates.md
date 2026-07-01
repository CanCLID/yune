# M54 Final Gates

Date: 2026-07-01.

Scope: native octagram-compatible grammar support for the named upstream
`luna_pinyin` target. This is not evidence for the librime C++ plugin ABI,
contextual translation, Lua/predict/proto, browser/package behavior, platform
frontends, AI ranking, or public performance claims.

## Evidence Summary

| Gate | Evidence | Result |
| --- | --- | --- |
| lotem canonical lane pinned | `task-0-target-selection.md`, `external-pins.json`, `phase-0-oracle/`, `crates/yune-core/tests/fixtures/upstream-octagram/lotem-luna-pinyin-octagram.json` | pass |
| RIME-LMDG validation lane pinned and validated | `task-0-target-selection.md`, `external-pins.json`, `phase-0-oracle/`, `phase-3-yune-core-verification.json`, `crates/yune-core/tests/fixtures/upstream-octagram/rime-lmdg-luna-pinyin-validation.json` | pass |
| Full third-party `.gram` files not vendored | fixture manifest plus Task 0 vendoring decision | pass |
| Clean-room implementation boundary | `clean-room-design.md` plus native Rust implementation in `crates/yune-core/src/poet/octagram.rs` | pass |
| Canonical accepted octagram behavior | `phase-3-yune-core-verification.md`, `phase-3-yune-core-verification.json` | pass |
| Empty-context rear-boundary oracle | `synthetic-rear-boundary-oracle.md`, `synthetic-rear-boundary-oracle.json`, `cargo test -p yune-core octagram_empty_context_rear_boundary_matches_librime_oracle_fixture` | pass |
| Null-grammar upstream `luna_pinyin` unchanged | `cargo test -p yune-core --test upstream_luna_pinyin_parity` | pass |
| TypeDuck `jyut6ping3` behavior unchanged | `cargo test -p yune-rime-api --test typeduck_windows_boundary yune_abi_jyut6ping3_ngohaig_comments_match_v112` | pass |
| Public C ABI unchanged | `cargo test -p yune-rime-api --test typeduck_profile_abi_surface` | pass |
| Broader plugin/contextual behavior deferred | `cargo test -p yune-rime-api schema_selection_defers_poet_grammar_contextual_translation` | pass |

## Commands Run

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p yune-core octagram
cargo test -p yune-core octagram_empty_context_rear_boundary_matches_librime_oracle_fixture
cargo test -p yune-core darts_double_array_supports_non_utf8_binary_keys
cargo test -p yune-core make_sentences_passes_only_last_two_prior_words_to_grammar
cargo test -p yune-core --test upstream_luna_pinyin_parity
cargo test -p yune-core --test oracle_fixture_provenance
cargo test -p yune-rime-api schema_octagram_loading
cargo test -p yune-rime-api schema_selection_defers_poet_grammar_contextual_translation
cargo test -p yune-rime-api --test typeduck_profile_abi_surface
cargo test -p yune-rime-api --test typeduck_windows_boundary yune_abi_jyut6ping3_ngohaig_comments_match_v112
cargo build -p yune-cli
```

All commands above passed.

## Follow-Up Review Fixes

The initial M54 commit was not mergeable until OCTA-1/OCTA-2 and clippy review
findings were fixed. The follow-up changed `OctagramGrammar::query` to mirror
upstream `Octagram::Query`: empty context returns `non_collocation_penalty`
before any rear `$` lookup. It also adds the executable synthetic librime
oracle fixture above, refreshes fixture provenance with capture commands, and
cleans the clippy `-D warnings` issues in octagram and Darts byte-key code.

## Canonical Oracle Verification

The ignored local verification harness under `target/m54-native-octagram/`
loaded the pinned upstream `luna_pinyin` dictionary/vocabulary, external
`.gram` models, and the M54 `OctagramGrammar` implementation. The committed
machine-readable summary is `phase-3-yune-core-verification.json`.

The canonical lotem lane used `zh-hant-t-essay-bgw.gram` from
`lotem/rime-octagram-data`. The produced ignored full report hash is:

`18521c346272a998f816011fd891bfa1275eeae48b008f24465c6acfb3ed74e7`

The RIME-LMDG validation lane used `wanxiang-lts-zh-hant.gram` from the LTS
release. The produced ignored full report hash is:

`5e4b1daa1141be5d8b0634a87b0ba5d82db88ed4e472723710825e8898c854c3`

The accepted gate compares the top candidate for each committed oracle row
because Yune still has known non-M54 table/menu ordering differences outside the
octagram grammar provider. All seven lotem rows and all nine RIME-LMDG rows
matched. The full reports remain intentionally ignored:
`target/m54-native-octagram/yune-core-verify-lotem-integrated.json` and
`target/m54-native-octagram/yune-core-verify-rime-lmdg-integrated.json`.

## M52 Guardrail Decision

The M52 native Track A benchmark gate was not rerun. M54 changes sentence
scoring only when an explicit octagram grammar model is loaded. Schemas without
`.gram` keep the existing `NullGrammar` path, and
`cargo test -p yune-core --test upstream_luna_pinyin_parity` passed after the
change. No M52 tracked null-grammar Track A row was intentionally changed.

## Memory And Scope

M54 records octagram retained bytes separately through the
`poet.octagram_double_array` memory-owner row when an octagram grammar provider
is attached. No timing or memory number here is a public performance claim.

RIME-LMDG remains a pinned real-world validation lane. Its LTS models are about
420 MB each and are CC-BY-4.0, so they remain external by URL/checksum/license
evidence rather than checked-in model data.
