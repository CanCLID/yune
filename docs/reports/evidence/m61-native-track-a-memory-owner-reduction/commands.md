# M61 Commands

## Binding Windows measurement shape

The source-bound wrappers used the plan-frozen Track A 17-input list, Track B
input, product deployment, and `--iterations 9 --session-iterations 60
--key-iterations 80`. Owned and byte-backed modes were explicit diagnostics.
Each accepted set used one fixed DLL, benchmark executable, benchmark receipt,
oracle identity, product schema tree, and create-new per-round roots.

The correction-source sequence was:

1. build the release DLL and benchmark once;
2. run five `owned` rounds and aggregate against the unchanged M55 registry;
3. run one separately named `byte-backed-exploratory` round;
4. run five `byte-backed` rounds and aggregate the signed registry plus the
   separate supplemental projection;
5. reconcile the five owned and five byte-backed owner budgets; and
6. stop at the measured reconciliation red, create the explicit revert, and do
   not create a production-default candidate.

## Closeout verification

```text
cargo test -p yune-core --test upstream_luna_pinyin_parity
cargo test -p yune-core upstream_script_surface_segmentation_prunes_stale_raw_identity_overlap
cargo test -p yune-core poet
cargo test -p yune-core packed_syllabary_codes_preserve_order_boundaries_and_owner_accounting
cargo test -p yune-rime-api dictionary_data
cargo test -p yune-rime-api deployment
cargo test -p yune-rime-api --test yune_web m61_luna_zhongdengchangdu_page_zero_matches_pinned_librime
cargo test -p yune-rime-api --test yune_web m59_luna_
cargo test -p yune-rime-api --test yune_web m59_schema_general_reachability_deployment_matrix_default_on_and_explicit_false
npm --prefix apps/yune-web run check:schema-manifest
python3 -B -m unittest scripts/tests/test_native_candidate_parity.py
python3 -B -m unittest scripts/tests/test_native_benchmark_script.py
python3 -B -m unittest scripts/tests/test_m59_evidence_tools.py
python3 -B -m unittest scripts/tests/test_m61_native_mode_contract.py
python3 -B -m unittest scripts/tests/test_m61_luna_poet_rebind.py
python3 -B -m unittest scripts/tests/test_m61_supplemental_ratchet.py
python3 -B -m unittest scripts/tests/test_public_evidence_privacy.py
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 -B scripts/check-current-doc-links.py --paths-from <external-current-doc-list>
python3 -B scripts/verify-packet-manifest.py docs/reports/evidence/m61-native-track-a-memory-owner-reduction/packet-manifest.csv
python3 -B scripts/check-evidence-growth.py --repo-root . --paths-from <external-evidence-list>
python3 -B scripts/check-public-evidence-privacy.py --paths-from <external-evidence-list> --forbid-literal-file <external-private-deny-file>
git diff --check
```

The exact workspace command above remains preserved red after its successful
prefix. Recovery does not rerun that prefix. The first owning slice used:

```text
cargo fmt --check
cargo test -p yune-core --test cantonese_parity -- --test-threads=1
cargo test -p yune-rime-api --test yune_web m58_yune_web_browser_app_assets_reach_profile_ranked_report_candidates
```

The first never-reached tail used exact strict workspace Clippy plus the eight
unreached core integrations. Its API package attempt preserved a later library-
contract red:

```text
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p yune-core --test oracle_fixture_provenance --test 'upstream_*'
cargo test -p yune-rime-api
```

After the second cfg(test)-only correction, the final disjoint recovery uses:

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p yune-rime-api --lib
cargo test -p yune-rime-api --bin yune_web_module --bin yune-schema-reachability-audit --test abi_abuse --test cold_start_conformance --test dynamic_loader --test frontend_client --test frontend_hosts --test native_memory_probe --test typeduck_profile_abi_surface --test typeduck_windows_boundary --test yune_web --test yune_windows_profile_abi_surface
cargo test -p yune-rime-api --doc
cargo test -p yune-core --doc
```

The original workspace red, the interrupted no-verdict retry, and the API-
library red are retained externally. No partial passing tests from either red
are counted in place of a complete owning-target result.

No browser latency, Cloudflare, package, Windows frontend, iOS, benchmark retry,
or reassurance-only performance run was added at closeout.
