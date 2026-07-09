# Lane A Yune canonical runner (rime-cantonese)

The reproducible setup for running Yune's **real production path** over pinned
`rime/rime-cantonese`, to diff against the committed oracle capture
(`../../m58-jyutping-exact-before-fuzzy/phase-1/canonical-rime-cantonese-capture.json`).

## Setup

1. **Stage rime-cantonese** into the oracle root (the capture script does this;
   pinned `rime/rime-cantonese` `c99b16e44d2df77a5cb8fb0867dd2bab7a112cb0`):
   `target/upstream-oracle/1.17.0/m58-rime-cantonese-shared/`.
2. **Overlay `default.yaml`** from this directory onto the staged shared dir,
   replacing the stock six-schema list with a jyut6ping3-only one:
   ```
   cp docs/reports/evidence/m59-canonical-jyutping-reachability-parity/lane-a-runner/default.yaml \
      target/upstream-oracle/1.17.0/m58-rime-cantonese-shared/default.yaml
   ```
   Full fidelity — only `schema_list` is narrowed; `menu/page_size` stays 5 to
   mirror the capture. See the header of `default.yaml` for the root cause.
3. **Run the real path** (fresh user dir):
   ```
   cargo run -p yune-cli -- frontend \
     --shared-data-dir target/upstream-oracle/1.17.0/m58-rime-cantonese-shared \
     --user-data-dir <fresh-tmp> \
     --schema jyut6ping3 --sequence "<input>"
   ```

## Notes

- **Cold from-source deploy is ~9 min (debug build).** rime-cantonese compiles a
  large multi-file dict (`import_tables` chars/words/phrase/lettered/maps +
  `vocabulary: essay-cantonese`) from source; there are no byte-backed compiled
  assets for this lane. Budget accordingly.
- **Resolved lane config is Standard-profile, no TypeDuck shims,
  `leading_syllable_reachability` ON** — see `../lane-a-revalidation-2026-07-08.md`
  (the landmine disposition). The runner inherits that automatically.
- **On deploy failure**, the CLI's bool ABI currently swallows the cause; surfacing
  `workspace_dictionary_rebuild_reports()` through the CLI is a named diagnostic
  work item (see the plan's Lane A work items).
- **Provenance to record with any committed runner output**: rime-cantonese commit
  `c99b16e4…`, the overlaid `default.yaml` (this file), and the staged shared-dir
  file hashes (the staged dir is untracked local state).
