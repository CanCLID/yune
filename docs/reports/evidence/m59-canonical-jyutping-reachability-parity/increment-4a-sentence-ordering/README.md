# M59 Increment 4a sentence/phrase ordering review packet

Status: mechanism commit `ca52ec427111e2ec36b2a80dfe7b25b6f2d3c456`
and blocking-review fix `2257fbbe1e8de5ad0e3ac25e45e2e3b07e11878c`
have passed the exact owning deployment matrix and focused review gates. The
owner-provided Opus review substituted for unavailable Fable and its blocking
finding is fixed forward. On 2026-07-11 the owner signed the renewed, narrowly
scoped D-48 class-3 exception for the complete captured equal-weight residual,
so Increment 4b is permitted to start. This packet does **not** close Lane A,
D-48, or M59. See
[`review-fix-forward/`](./review-fix-forward/).

Later macOS follow-up exposed an additional Luna long-input defect that this
packet's committed 37- and 59-character snapshots still preserve: Yune emits
five full-span sentence alternatives where pinned librime emits one best
sentence followed by phrase candidates. The supplemental
[`increment-4a-luna-script-translation-order`](../increment-4a-luna-script-translation-order/)
packet repairs those two pages at measured Yune commit `89875ee2`. That packet
does not alter this Windows performance verdict, the strict Lane A comparator,
or the renewed class-3 owner disposition and 4b permission; it did not cause or
supersede that independently recorded disposition. Neither packet is a
performance measurement of the later combined source tree.

The implementation derives a Standard script-sentence policy from translator
configuration and `SchemaBehaviorProfile`, never schema id or input text. It
makes the upstream sentence model authoritative for the applicable Standard
script path, merges sentence/direct/completion families with comparable
semantics, and leaves TypeDuck on the legacy profile policy. The implementation
also writes and reads the current `YUNE-POET/3` sidecar format, rejects stale
`YUNE-POET/2`, and retains the
default-owned runtime path unless `YUNE_POET_BYTE_BACKED=1` is explicitly set.

## Strict oracle result

The raw exact comparator remains deliberately red and carries no exception.
That is an evidence boundary, not a hidden green rewrite:

| Input | Before | After | First after mismatch |
|---|---|---|---|
| `being` | 136 vs 140, fail at 0 | 139 vs 140, fail at 10 | missing `祕` |
| `beingo` | 136 vs 142, fail at 0 | 141 vs 142, fail at 12 | missing `祕` |
| `beixngoxx` | 37 vs 38, fail at 0 | 38 vs 38, exact | none |
| `mgoi` | 1,345 vs 4, fail at 1 | 4 vs 4, exact | none |
| `zijiguk` | 387 vs 416, fail at 0 | 416 vs 416, fail at 32 | `衹` placement |

The separate fail-closed classifier
[`behavior/opencc-equal-weight-residual-classification.json`](./behavior/opencc-equal-weight-residual-classification.json)
does not turn that comparator green. It binds the raw captures, strict diff,
pinned same-code OpenCC inventory, and dictionary weights. After normalizing
only the predeclared `祕 -> 秘 祕` and `只 -> 只 衹` effects, every remaining
relative-order inversion has equal effective weight: `315/315` for `being`,
`315/315` for `beingo`, and `5,456/5,456` for `zijiguk`; zero inversions cross a
weight boundary. The prior equal-weight class disposition is owner-signed, but
its record says to revisit if the diff grows. The blocking review accepted the
classifier mechanism and recommended a renewed, narrowly scoped class-3
exception. The owner explicitly signed that renewed exception on 2026-07-11;
the exact scope and revisit triggers are recorded in
[`review-fix-forward/`](./review-fix-forward/). The packet does not infer or
broaden the exception beyond that signature.

All compared lists are complete (`captured_all_pages=true`, `last_page`), so no
beyond-oracle-depth disposition is used. `beixngoxx` and `mgoi` prove that the
4a sentence/phrase mechanism itself can reach strict exact order; 4c owns the
one-to-many OpenCC behavior, while the signed tie class remains visible.

Classifier replay from the repository root (replace `$output` with an external
scratch path):

```powershell
$packet = 'docs/reports/evidence/m59-canonical-jyutping-reachability-parity/increment-4a-sentence-ordering/behavior'
$source = 'target/upstream-oracle/1.17.0/schema-src/rime-cantonese'
$output = Join-Path $env:TEMP 'm59-4a-residual-replay.json'
python -B scripts/classify-m59-4a-residuals.py `
  --oracle "$packet/lane-a-class1-oracle.json" `
  --expected-oracle-sha256 9cf5f91dfa81c050a3d55e54f423aa4648c64cc6f0f8447549004e3c810d755b `
  --actual "$packet/lane-a-class1-yune-after-default-owned.json" `
  --expected-actual-sha256 2a367341eb4ba2f4a947a11fa84a3d0f13f4ffdeae7f4225f8bd88b3b9a0b34e `
  --strict-comparator "$packet/lane-a-class1-exact-after-default-owned.json" `
  --expected-strict-comparator-sha256 cb7a1ed96cb3fc9980ab59c8257be07db3c5c9b5868f7840fd9c5c7bef92b65d `
  --opencc-inventory docs/reports/evidence/m59-canonical-jyutping-reachability-parity/increment-2-profile-paging/opencc-same-code-inventory.csv `
  --expected-opencc-inventory-sha256 01522f437038a3591d3a3b92cbdace2cced1b1e9076e566ca40662c736afcaf1 `
  --source-repository $source `
  --expected-dictionary-commit c99b16e44d2df77a5cb8fb0867dd2bab7a112cb0 `
  --expected-dictionary-tree eb193fb80675ffa60df3c32bf24afa7d7f68617a `
  --dictionary-manifest "$source/jyut6ping3.dict.yaml" `
  --expected-dictionary-manifest-sha256 4301001fb7bb52d5d1a9c032c519ac18ba50677e926e01006e34a48788385efa `
  --vocabulary "$source/essay-cantonese.txt" `
  --expected-vocabulary-sha256 a73fe3ea1004531d4872165cbcc753cea172d865277ad9996a4fe6415582f9cb `
  --opencc-source crates/yune-core/src/opencc/data/HKVariantsFull.txt `
  --expected-opencc-source-sha256 145b561c68a697d5f2197da0c091caf4a0e9457f0a4c56cdf2ae7ad4b8ff8cc2 `
  --output $output
```

The tool exits `0` only when classification is complete. It exits `1` while
preserving a behavioral report for an unclassified/cross-weight residual, and
exits `2` while invalidating stale output for malformed or provenance-invalid
inputs. Its 13 focused tests include forged aggregate state and atomic existing
output replacement.

## Compiled-storage boundary

The optional-sidecar replay reproduces default-owned output exactly for all
five lists (`139 / 141 / 38 / 4 / 416`), as recorded by
[`default-owned-vs-replay-exact.json`](./behavior/default-owned-vs-replay-exact.json).
The replay capture did not embed `YUNE_POET_BYTE_BACKED` or an M43 storage-owner
snapshot, so it is corroborating equality evidence, **not** standalone proof
that the capture selected mmap storage.

Executable tests provide the storage proof separately:

- `upstream_script_policy_merges_phrase_sentence_and_partial_families_across_storage_paths`
  compares the 4a merge model across owned and compiled-byte stores;
- `upstream_sentence_model_reads_candidates_from_byte_backed_poet_artifact`
  checks `/3` sections, compiled prefix indexing, mmap-class owners, and the
  absence of a retained heap lookup index;
- `dictionary_data_ignores_compiled_poet_artifact_until_explicitly_enabled`
  creates an on-disk sidecar and proves that the native production loader only
  selects it under the explicit opt-in.

Each exact rerun passed `1/1`; see [`verification.md`](./verification.md).

## Performance verdict

The fixed 3,891,200-byte release DLL has SHA-256
`7ed2dc4468524d6e9c21fd5559f4fe6f49f19eb7d90dc6f5d044f200246391e8`.
Five sequential accepted runs used that same DLL, implementation commit,
upstream DLL, 48-file product schema tree, signed thresholds, 17 Track A inputs,
one Track B input, product deployment, and `9 / 60 / 80` iterations. The
aggregate is `32/32` pass with zero individual row failures.

The explicitly requested long-input evidence is:

| Row | Run 1 | Run 2 | Run 3 | Run 4 | Run 5 | Median | Ceiling | Verdict |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| 37 characters | 0.230 | 0.223 | 0.229 | 0.227 | 0.233 | 0.229 | 2.339 | pass |
| 59 characters | 0.131 | 0.126 | 0.126 | 0.125 | 0.128 | 0.126 | 1.748 | pass |
| `n` | 2.938 | 2.758 | 2.804 | 2.788 | 2.848 | 2.804 | 3.006 | pass |
| `ni` | 2.057 | 1.974 | 2.000 | 1.994 | 2.011 | 2.000 | 2.666 | pass |
| `hao` | 1.364 | 1.330 | 1.311 | 1.316 | 1.328 | 1.328 | 1.844 | pass |

The 37-character input is
`ceshiyixiachangjushuruxingnengzenyang`; the 59-character input is
`zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong`. Both exact
strings appear in every run's environment, summary, threshold check, candidate
snapshot, and aggregate row. All five candidate snapshots are byte-identical
(canonical-LF SHA-256
`306758a6b5489275b83fda80fcf9772d8250a1992c265c5f6bdd9f0395415385`).

This is an increment guard, not M59-REACH-04's final acceptance; that requirement
still needs five fresh rounds from the final behavior commit. Full details and
the two preserved invalid setup attempts are in
[`performance-ratchet/`](./performance-ratchet/).

Fix-forward commit `2257fbbe` also has a fresh single-run 17+1 product-deployed
review guard with `-FailOnRegression`; all 32 signed rows pass, including `n`,
`ni`, `hao`, and the 37/59-character rows. Its curated text evidence is in
[`review-fix-forward/performance-ratchet/`](./review-fix-forward/performance-ratchet/).
It does not replace either the accepted five-round 4a packet above or the five
final rounds required from M59's eventual final behavior commit.

## Verification and review boundary

The review found one blocking regression the original focused packet missed:
`UpstreamScript` bypassed the explicit-false reachability opt-out. The exact
deployment matrix reproduced red on `2b4a169a` and passes on fix-forward commit
`2257fbbe`. Full workspace clippy, core/API, Cantonese, Luna, Zhuyin,
TypeDuck-Windows, and classifier gates pass; `cantonese_parity` is `41/41`,
`upstream_luna_pinyin_parity` is `14/14`, and `typeduck_windows_boundary` is
`4/4`. Two independent internal reviews approved the final boundary-aware fix.
The classifier now uses `classification_status: "complete"` rather than an
ambiguous top-level `verdict: "pass"`; the raw comparator remains explicitly
unaccepted.

An attempted full 50-test `yune_web` process timed out after 15 minutes under
cache pressure. The load-bearing focused browser/API controls passed in isolated
processes, but this packet does **not** claim the full browser suite green; it
remains part of M59-GATES-01. The final native release, source-current WASM,
full browser, packaging, and final-commit performance gates likewise remain
open.

The packet contains text evidence only. It intentionally excludes DLLs,
`.marisa` tables, compiled schema payloads, deploy trees, samples, and bulky
console logs. [`packet-manifest.json`](./packet-manifest.json) binds every
retained file except itself.
