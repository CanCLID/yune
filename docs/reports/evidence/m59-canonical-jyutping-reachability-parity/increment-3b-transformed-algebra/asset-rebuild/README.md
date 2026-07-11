# Increment 3b deterministic asset rebuild

This packet records the source-current product-table rebuild required after the
schema-general transformed-algebra change.

## Frozen source

- Parent `main`: `d55b203ebd9552f5af6b0f34a525cf9e3852bd70`
- Staged source tree: `d5121383ad51d12c0c84b0f0504e0ee4919edf9f`
- Disposable snapshot commit: `89847158444b2ae6d1e77c48b853b103c5e267ec`
- Run A worktree: `C:\y59r-3b-final4-a`
- Run B worktree: `C:\y59r-3b-final4-b`

The two worktrees were independent clean checkouts of the same frozen tree.
Each used its own Cargo target directory and external evidence directory.

## Verdict

- Run A rebuild gate: **PASS**, 1 passed / 0 failed, 140.43 seconds.
- Run B rebuild gate: **PASS**, 1 passed / 0 failed, 142.55 seconds.
- Compiled artifacts inventoried: **18**.
- Run A versus Run B byte-identical artifacts: **18/18**.
- Artifacts changed versus the tracked parent: **12**.
- Imported artifacts matching Run A: **18/18** (the six unchanged artifacts
  already matched).
- Total compiled bytes: **80,378,546 -> 109,065,750**
  (**+28,687,204 bytes**).
- Manifest-to-tree validation: **PASS**, 59 assets across the public and
  public-demo manifests.
- Public packaging build: **PASS**.

`artifact-sha256.csv` is the authoritative before/A/B/import reconciliation.
`source-tree-sha256.csv` records every non-binary schema source used by the
clean rebuild. The `run-a/` and `run-b/` folders preserve the native rebuild
inventories and per-dictionary rebuild dispositions.

The size increase is recorded rather than hidden by an aggregate. The largest
rows are `jyut6ping3_mobile.prism.bin` (+12,785,416 bytes),
`stroke.table.bin` (+12,700,498 bytes), and `stroke.reverse.bin` (+2,918,816
bytes). The mobile prism growth is the required cumulative deployed-algebra
surface: 338,156 -> 533,269 spellings and 2,097,152 -> 4,194,304 Darts units;
valid root offsets cross Darts' `2^21` large-offset boundary.

Native loads these artifacts through file-backed mappings and does not clone the
12.2 MiB mobile-prism delta into an equivalent private heap owner, although
touched resident pages may still increase. WASM reads the artifact into owned
linear memory, so it pays the full delivery/ownership delta and may incur
transient browser copies. The source-current storage-owner, packaging, WASM,
and signed performance gates therefore remain mandatory; this packet does not
quietly rebaseline size.

No compiled binary payload is copied into this evidence directory. The rebuilt
binary files remain only in `apps/yune-web/public/schema/`.
