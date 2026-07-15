# Evidence-pruning archive ledger

This directory records the recovery-bound archive of bulky benchmark artifacts
removed from Yune's current tree. The builder itself remains read-only: it
inventories a recovery commit, scans a separately selectable post-pruning tree,
and writes deterministic ledger files. It never deletes files, alters the Git
index, or rewrites history.

The inventory contract matches tracked files named `m37_metrics.csv`,
`samples.csv`, `startup_session_trace.csv`, and `*.marisa`. Paths under
`docs/reports/evidence/` default to `archive-remove`. Matching files elsewhere,
or files used directly by current code or tests, must appear in `allowlist.csv`
as `retain-fixture` or `retain-dependency`. Direct links from current documents
and evidence-packet documents are blockers until redirected. Historical packet
manifests remain byte-for-byte provenance and receive
`historical-manifest-reference`; that label deliberately does not claim that a
manifest's recorded external-packet bytes equal the tracked leaf. The archive
ledger, not those historical manifests, binds each removed tracked blob to its
recovery commit. References from retained evidence artifacts receive
`retained-artifact-recovery-reference`. Generic or ambiguous prose that merely
names an output class remains informational.

Generate the checked-in ledger and dependency scan from a commit that contains
the exact recoverable bytes:

```sh
dependency_tree="$(git write-tree)"
python3 scripts/build-evidence-pruning-ledger.py \
  --repo-root . \
  --treeish 9ee0b98d2c6d1e6fa339f50141860994a8a7b2f6 \
  --dependency-treeish "$dependency_tree" \
  --recovery-commit 9ee0b98d2c6d1e6fa339f50141860994a8a7b2f6 \
  --allowlist docs/ledgers/evidence-pruning/allowlist.csv \
  --retained-summary docs/ledgers/milestone-history.md \
  --ledger-out docs/ledgers/evidence-pruning/current-ledger.csv \
  --dependency-out docs/ledgers/evidence-pruning/current-dependencies.csv
```

After committing, use `--dependency-treeish HEAD --check` with the same recovery
commit. Do not substitute the new ledger commit for `--treeish` or
`--recovery-commit`: `9ee0b98d…` intentionally identifies the pre-pruning tree
that owns the archived bytes. The
tool fails closed when paths or allowlist rows are duplicated or unsorted, Git
blob IDs or sizes drift, the recovery commit is missing or cannot restore the
same bytes, a summary pointer is absent, a matching non-evidence path is not
allowlisted, or a current code/test/document dependency or link is found.

Recover one listed leaf without changing the worktree by reading the recorded
commit and path:

```sh
git show "<recovery-commit>:<ledger-path>" > "/external/archive/<ledger-path>"
```

Before any later history rewrite removes the recovery commit, materialize the
external archive and verify every recovered byte against the ledger's Git blob
ID. The recovery commit is sufficient for an ordinary pruning commit, but it is
not a substitute for an external archive after destructive history rewriting.

`current-ledger.csv` is sorted by repository path and records the path, Git
SHA-1 blob ID, exact byte size, owning packet, removal class, retained summary
pointer, and full recovery commit. `current-dependencies.csv` distinguishes
direct current dependencies from generic output-name mentions, historical
documentation, historical manifests, and retained evidence-artifact references.
The dependency scan is run against the staged post-pruning tree before commit
and against `HEAD` after commit, so working-tree redirects cannot make a stale
pre-pruning scan look safe.

The current archive is bound to recovery commit
`9ee0b98d2c6d1e6fa339f50141860994a8a7b2f6`. It inventories 1,928 paths:
1,911 `archive-remove` rows totaling 527,235,943 bytes and 17 allowlisted
browser fixtures totaling 107,817 bytes. The 1,309-row post-pruning dependency
scan contains no code/test blocker and no current-document redirect. It records
223 historical manifest references, 234 retained evidence-artifact recovery
references, three historical direct references, one reference to an allowlisted
fixture, and 848 informational or ambiguous output-name mentions. The current
curated M59 macOS packet manifest contains none of the four inventory filename
patterns, so it does not require this historical-leaf recovery path.

| Archive-removal pattern | Paths | Bytes |
| --- | ---: | ---: |
| `m37_metrics.csv` | 630 | 168,962,447 |
| `samples.csv` | 617 | 88,207,874 |
| `startup_session_trace.csv` | 544 | 72,951,246 |
| `*.marisa` | 120 | 197,114,376 |
| **Total** | **1,911** | **527,235,943** |

The allowlist accounts for the other 17 `samples.csv` paths and 107,817 bytes.

## Future benchmark evidence

The native benchmark entrypoints now default raw output to
`~/.yune/evidence/<benchmark-kind>/<UTC>` (or `YUNE_EVIDENCE_ROOT`) and reject a
destination in any Git worktree. Output roots are create-new. Import only an
explicit compact allowlist with `scripts/curate-compact-evidence.py`; the
curator rejects raw classes, traversal, symlinks, case-insensitive collisions,
unexcepted files over 5 MiB, and packets over 10 MiB, then writes a deterministic
SHA-256 manifest.

Run `python3 scripts/check-evidence-growth.py --repo-root .` before committing
evidence changes. The guard checks staged additions, copies, modifications, and
rename destinations against the same raw-class and size policy. The
`Evidence growth guard` GitHub Actions workflow repeats the policy for committed
push and pull-request diffs.
