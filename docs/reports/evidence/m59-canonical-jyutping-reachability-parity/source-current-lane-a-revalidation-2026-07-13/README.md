# M59 source-current Lane A revalidation

This packet closes the independent-review finding that M59's strict Lane A
capture was bound to Increment 4c rather than the later shipping behavior.
The review gap exposed a real post-4c regression, which was fixed on `main` and
then remeasured from a clean commit.

## Verdict

- Repair commit: `443cc636862806e4f0dd1e12ab2e2e45f4189154`
- Source tree: `875a5d8705ff70d4765ca4dde87a941343f57d7a`
- Release DLL SHA-256:
  `f829a14033c4cad5e594e50349ee40f104686159404628343bd7673a9467f49b`
  (`4,249,088` bytes; not copied into evidence)
- Canonical result: **13/13 inputs and all 5,705 candidate positions exact**
- Exceptions used: **none**
- Every case reached `termination_reason: last_page`; candidate counts, order,
  page size, menu state, and terminal-page state match the pinned oracle.

The accepted capture is source-clean and records `source_commit`, source tree,
DLL hash, source/staged shared-tree hashes, oracle hash, probe hash, capture-tool
hash, exact invocation, inputs, and page policy in `source-current-yune.json`.

## Finding and repair

The clean pre-fix capture at `07845e02` was 12/13: `ngohaig` retained all 2,050
oracle candidates but first diverged at global index 141. Librime ordered the
equal-weight group as `我夠`, `你過`, `泥膠`; Yune promoted the direct/model
overlap `泥膠` ahead of the two reconstructed rows. The same direct-overlap
promotion repeated through the equal-weight tail.

Commit `5879405c` had made direct-table collector phase ordering unconditional
while implementing Lane B. The repair keeps that phase for identity-normal
surface graphs, but lets transformed/abbreviation graphs retain the compiled
model traversal that owns their cross-code-family equal-weight order. The
predicate is graph-derived: there is no schema-ID branch, input allowlist,
promotion table, or baked oracle row. Outer translator merge quality remains
unchanged.

The internal regression test covers both policy arms across owned and parsed
byte-backed storage and checks a bounded field-identical prefix. The external
capture in this packet—not that synthetic test—is the canonical acceptance
oracle.

## Oracle stability

Before the Yune capture, the canonical oracle was rebuilt from official pinned
librime `33e78140250125871856cdc5b42ddc6a5fcd3cd4` binaries and clean pinned
schema repositories. Its fresh 13-input capture compared exact to the committed
oracle: 13/13, 5,705/5,705, no exception. The raw refresh remains external to
avoid duplicating another five-megabyte capture; `oracle-refresh-exact.*`
preserves its row hashes, complete-page verdict, tool identity, and raw capture
SHA-256 (`2997934b26db0da66e22d7f7f52a4ccf5d1cf48d85e4524b1c036b8b68e035df`).

## Targeted verification

Only load-bearing shared-path checks were run:

- new transformed/identity tie regression: `1 passed / 0 failed`;
- exact seven-input Lane B product order: `1 passed / 0 failed`;
- exact deployed Luna 37/59 page order: `1 passed / 0 failed`;
- Cangjie composition/CJ-1: `3 passed / 0 ignored`;
- Double Pinyin: `3 passed / 1 standing M17 ignore`;
- Bopomofo: `3 passed / 3 standing documented ignores`;
- `cargo fmt --check`: pass;
- targeted `yune-core` library Clippy with `-D warnings`: pass.

The schema-general default-on/explicit-false matrix was started as an additional
shared-path check, but the command wrapper timed out and did not retain the
child's eventual exit status. No result is inferred or claimed from that run;
the earlier shipping-source matrix evidence remains unchanged. No broad
workspace, browser, packaging, or startup suite was run for this narrow
ordering repair.

## Reproduction commands

```powershell
$env:CARGO_TARGET_DIR = 'C:\m59-lane-a-shipping-target-443cc636'
cargo build --release -p yune-rime-api

powershell -NoProfile -ExecutionPolicy Bypass `
  -File scripts/capture-yune-candidate-order.ps1 `
  -YuneDll C:\m59-lane-a-shipping-target-443cc636\release\yune_rime_api.dll `
  -SharedDataDir C:\m59-lane-a-shipping-oracle\m58-rime-cantonese-shared `
  -SchemaId jyut6ping3 `
  -OracleCapture docs/reports/evidence/m59-canonical-jyutping-reachability-parity/increment-1-executable-evidence/lane-a-oracle.json `
  -Output C:\m59-lane-a-shipping-443cc636\lane-a-shipping-yune.json `
  -DefaultYamlOverlay docs/reports/evidence/m59-canonical-jyutping-reachability-parity/lane-a-runner/default.yaml `
  -WorkRoot C:\m59-lane-a-shipping-443cc636\work `
  -ExpectedYuneDllSha256 f829a14033c4cad5e594e50349ee40f104686159404628343bd7673a9467f49b

python scripts/compare-candidate-order.py `
  --oracle docs/reports/evidence/m59-canonical-jyutping-reachability-parity/increment-1-executable-evidence/lane-a-oracle.json `
  --actual C:\m59-lane-a-shipping-443cc636\lane-a-shipping-yune.json `
  --policy exact `
  --output-json C:\m59-lane-a-shipping-443cc636\lane-a-shipping-exact.json `
  --output-csv C:\m59-lane-a-shipping-443cc636\lane-a-shipping-exact.csv
```

## Packet contents

- `source-current-yune.json`: accepted clean-commit Yune capture.
- `source-current-exact.json` / `.csv`: accepted strict comparator.
- `pre-fix-exact.json` / `.csv`: rejected 12/13 comparator preserving the
  discovered `ngohaig` regression without duplicating its large raw capture.
- `oracle-refresh-exact.json` / `.csv`: fresh-oracle stability comparator.
- `packet-manifest.json`: exact text-only file inventory and hashes.

No DLL, deployed tree, compiled table, grammar model, or other binary payload is
stored in this packet.
