# M60 Commands

Named gates were run from repository root. Raw output is external under
`yune-m60-schema-reachability-formalism/b8cd897f9d6c3158d864bac9d2629482c45c7427/`.

| Command | Source | Result |
| --- | --- | --- |
| `npm --prefix apps/yune-web run check:schema-manifest` | `c1f1f941` | pass; 60 assets, 17 tuples, 0 opt-outs |
| `node --test apps/yune-web/scripts/check-schema-asset-manifest.test.mjs` | `c1f1f941` | 52 passed, 0 failed |
| `node --test apps/yune-web/scripts/update-schema-asset-manifest.test.mjs` | `c1f1f941` | 2 passed, 0 failed |
| `cargo test -p yune-rime-api --test yune_web m59_schema_general_reachability_deployment_matrix_default_on_and_explicit_false` | `c1f1f941` | 1 passed, 0 failed |
| `cargo test -p yune-rime-api --test yune_web m59_manifest_plain_jyut6ping3_real_deploy_default_on_and_explicit_false` | `c1f1f941` | 1 passed, 0 failed |
| `cargo test -p yune-rime-api --test yune_web m60_namespaced_reachability_audit_matches_real_deploy` | `c1f1f941` | 1 passed, 0 failed |
| `cargo test -p yune-core prefix_fallback_without_owned_prefix_keeps_leading_syllable_reachability` | `c1f1f941` | named test passed |
| `python3 -m unittest scripts/tests/test_current_doc_links.py scripts/tests/test_packet_manifest.py` | `c1f1f941` | red: 14 run, 6 failures, 2 skipped |
| same utility command, owning-slice retry | `78a9e38a` | 14 run, OK, 2 skipped |
| same utility command, review-fix retry | `e352fba4` | 16 run, OK, 2 skipped; both new junction cases passed |
| same utility command, older-Python review-fix retry | `c9b34774` | 16 run, OK, 2 skipped; reparse-tag junction cases passed |

The npm command was finally executed through the installed `npm.cmd` shim
because PowerShell policy blocked `npm.ps1`; its arguments were unchanged.
Setup failures, the measured utility red, and the discarded first review tree
are preserved externally. After each utility fix, no already-green gate was
rerun.

Closeout-only checks use:

```text
python3 scripts/check-current-doc-links.py --paths-from <touched-current-docs>
python3 scripts/verify-packet-manifest.py docs/reports/evidence/m60-schema-general-reachability-formalism/packet-manifest.csv
python3 scripts/check-evidence-growth.py --repo-root . --paths-from <evidence-paths>
GIT_INDEX_FILE=<isolated-index> git diff --cached --check
```
