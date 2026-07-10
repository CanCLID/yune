# M59 Increment 1 - final Lane A executable pre-fix diagnostic

Generated on 2026-07-10 from clean Yune commit
`1fd3ca5f10c3eaec432266085c9f58271be39d1f` in the ignored scratch root
`target/m59-increment1-lane-a-final/`, then copied byte-identically into this
six-file packet.

**Verdict: RED, 0/13 accepted.** This is executable pre-fix diagnostic evidence
for the remaining M59/D-48 work. It is not acceptance evidence, does not close
D-48 or M59, and applies no exceptions. Comparator exit `1` is the expected
result because all 13 strict-exact rows are preserved as failures.

## Generation, import, and canonical text provenance

The oracle capture, Yune capture, and both comparator outputs were generated in
the scratch root named above. The four packet copies retain the exact generated
bytes and hashes:

| Generated and imported file | Bytes | SHA-256 |
|---|---:|---|
| `lane-a-oracle.json` | 5,096,504 | `56e7aafabcfac7eb7d3b209d5929eff88c65aeaa1b98fee0464e6b1fcda8c1ca` |
| `lane-a-yune.json` | 7,794,087 | `5de0c573623e3117c11348f06e2d6e4055eea07f8440d963972fe6ad3ca5b12f` |
| `lane-a-exact-diff.json` | 50,304 | `f3e82e9698569ed2d39e08dae2267e8f7742917f33b02704ce961535e9d6897f` |
| `lane-a-exact-diff.csv` | 8,532 | `b83ec956fa2cff9395d8c4cd1801591f5a1b3e4cac5cdbbb1d5b32e9fc690aef` |

Every packet file is canonical UTF-8 without BOM, uses LF only, contains no
NUL or CR byte, and ends in exactly one LF. The two capture scripts normalize
their JSON text to LF and one terminal LF before no-BOM UTF-8 writes. The
comparator emits JSON with one terminal `\n`, CSV with `lineterminator="\n"`,
and writes with newline translation disabled. The packet README and manifest
follow the same byte contract.

Repository attributes resolve these paths to `text=auto eol=lf`. Filtered and
no-filter Git blob IDs are identical for every packet file, proving the LF
filter is an intended no-op rather than a post-capture rewrite.

The raw files intentionally retain their original scratch paths. The oracle
records `existed_before_capture=false`, `write_policy=utf8_no_bom_create_new`,
and its generating script. Yune's actual invocation and effective output
parameter point to the scratch root. Comparator provenance binds both scratch
inputs, both scratch outputs, their input hashes, the exact policy, and the
tool hash. The packet manifest binds these original output records to their
byte-identical packet copies; no provenance path or line ending was rewritten
after capture.

## Oracle identity and settings

- Engine: `rime/librime` 1.17.0 at
  `33e78140250125871856cdc5b42ddc6a5fcd3cd4`.
- `rime.dll`: 3,739,136 bytes, SHA-256
  `86b4c7357d4c6d293ce5589b234d8859ca2ac30923a03bedfa3926eeaf97fb0b`.
- `rime_deployer.exe`: 459,776 bytes, SHA-256
  `3abb72b5bb56fcafcfe925d533ae5f832c68d5a0bc9952fd0eea0682fb1ab071`.
- Capture script: `scripts/capture-upstream-rime-cantonese.ps1`, 26,579 bytes,
  SHA-256 `5c8d37216749aa8e7b2d675d9faa4e9cd1df9a706510d558906e2543444f17f3`.
- Shared probe: `scripts/oracle-rime-probe.cs`, 44,746 bytes, SHA-256
  `27bc878e5185d24982c58e90281c44752bca62fef3c7559b033fbfd301e00db0`.
- Schema: `rime/rime-cantonese` at
  `c99b16e44d2df77a5cb8fb0867dd2bab7a112cb0`; every pinned schema repository
  below recorded clean with an empty short status.

| Schema repository | Commit |
|---|---|
| `rime/rime-cantonese` | `c99b16e44d2df77a5cb8fb0867dd2bab7a112cb0` |
| `rime/rime-prelude` | `082425ea0684bca36474415d4a0e8db9b016487e` |
| `rime/rime-luna-pinyin` | `18a80335c37522311f7cff02886cd81cec3b460a` |
| `rime/rime-essay` | `48c7538f0b760fcc8c9d6bf08711f82cfbd2e9ed` |
| `rime/rime-stroke` | `3a4b0f4013e2b4c14b1e80c92b1d4723eb65f39c` |
| `rime/rime-cangjie` | `52d90a1b1312e74042b38c1cbc8142defbc53171` |
| `CanCLID/rime-loengfan` | `987ac95b02f957e8764a2f45222a4006c188ed50` |

The fresh oracle and Yune capture use the **same ordered probe base policy** and
source:

1. `ascii_mode=false`
2. `full_shape=false`
3. `ascii_punct=false`
4. `zh_hans=false`

The source is
`RimeProbe.CaptureWithIdentity/CaptureRuntimeOptionPolicy`. The oracle records
that ordered object and source at both capture and effective-parameter levels;
its `additional_runtime_option_patches=[]`. The legacy root option-patch alias
is also empty. All 13 oracle cases observed page size 5,
`is_ascii_mode=false`, and complete all-page capture. The root schema-default
note remains `ascii_mode reset 0; character-style switch default comes from
rime-cantonese jyut6ping3.schema.yaml`; it is context, not a different runtime
policy for this fresh capture.

## Yune identity and staging

- Source: clean commit `1fd3ca5f10c3eaec432266085c9f58271be39d1f`
  with empty `source_status_short`.
- Release DLL: 3,624,960 bytes, SHA-256
  `2ac5d2550cd462d52d11aec995f609a605ccc6aa9837a8e55cec18a7ec63cfa5`.
- Capture script: `scripts/capture-yune-candidate-order.ps1`, 26,134 bytes,
  SHA-256 `f4d91a0d05b3a08e1649a3663e6418878884569878f9e770bbc9a30ba3020c60`.
- Shared probe: SHA-256
  `27bc878e5185d24982c58e90281c44752bca62fef3c7559b033fbfd301e00db0`.
- Source shared-data tree: 66 files, SHA-256
  `b76c540136b54162df6cc00a2b9159719fb7c207c99e5a6c9625abbe08aa9e5c`.
- Staged shared-data tree: SHA-256
  `a169531b04449c38875eca42c14ce2ddb95d32037f2178de603e0d80c47adfd8`.
- `lane-a-runner/default.yaml`: 2,595 bytes, SHA-256
  `ab0beef16410765c1b7157a27f406990c7a6f6330e4e0c76d95e2b44b4050f7f`.
  It effectively narrowed the staged list to `jyut6ping3`;
  `narrow_schema_list_switch_used=false` and
  `schema_list_narrowing_source=default_yaml_overlay`.

Yune records the same ordered four-false policy and source at both raw capture
and effective-parameter levels. Inputs were derived from the fresh oracle's 13
cases. Twelve Yune inputs returned page size 5 and a complete final page;
`nri` returned the complete no-menu shape documented below. Every case records
`captured_all_pages=true`.

## Fresh-oracle equivalence

The fresh oracle's ordered `all_candidates` arrays are structurally identical
for every one of the 13 inputs to the historical pinned fixture
`../lane-a-ranking-fix-2026-07-09/re-diff/canonical-13-input-oracle-capture.json`
(5,159,745 bytes, SHA-256
`58e4eae6019256ee5933e43781c1714e134b251c2247861c2686f305bd831b49`).
Only the hardened capture/probe provenance, related metadata, and canonical LF
serialization are newer; the candidate-order oracle is unchanged. The
manifest records the comparison surface and a shared ordered-list hash for
every input.

## Six-file packet closure

The packet contains exactly these six files:

1. `lane-a-oracle.json` - fresh pinned upstream oracle capture.
2. `lane-a-yune.json` - complete Yune capture against that oracle.
3. `lane-a-exact-diff.json` - machine-readable exact comparison.
4. `lane-a-exact-diff.csv` - review-friendly exact comparison.
5. `README.md` - this human-readable identity and replay record.
6. `lane-a-manifest.json` - machine-readable closure and cross-field binding.

The manifest hashes the other five files and intentionally omits its own hash
to avoid recursion. The packet contains JSON, CSV, and Markdown only. No DLL,
compiled table, or other binary payload is copied into evidence.

## Exact verdicts

`First mismatch` is zero-based. No exception file was supplied.

| Input | Oracle | Yune | First mismatch | Missing | Extra | Failure classes | Verdict |
|---|---:|---:|---:|---:|---:|---|---|
| `be` | 4 | 779 | 4 | 0 | 775 | over-admission, order | fail |
| `bei` | 139 | 638 | 9 | 4 | 503 | under-admission, over-admission, order | fail |
| `bein` | 5 | 169 | 0 | 4 | 168 | under-admission, over-admission, order | fail |
| `being` | 140 | 151 | 0 | 5 | 16 | under-admission, over-admission, order | fail |
| `beingo` | 142 | 148 | 0 | 6 | 12 | under-admission, over-admission, order | fail |
| `beix` | 36 | 294 | 36 | 0 | 258 | over-admission, order | fail |
| `beixngoxx` | 38 | 146 | 0 | 1 | 109 | under-admission, over-admission, order | fail |
| `ngohaig` | 2,050 | 46 | 0 | 2,005 | 1 | under-admission, over-admission, order | fail |
| `ngohaigo` | 113 | 46 | 0 | 68 | 1 | under-admission, over-admission, order | fail |
| `n` | 1,309 | 5,767 | 0 | 3 | 4,461 | under-admission, over-admission, order | fail |
| `nri` | 1,309 | 0 | 0 | 1,309 | 0 | page-size, menu-presence, under-admission, order | fail |
| `mgoi` | 4 | 11 | 1 | 0 | 7 | over-admission, order | fail |
| `zijiguk` | 416 | 387 | 0 | 30 | 1 | under-admission, over-admission, order | fail |

For `nri`, the ABI returned a complete no-menu result:
`menu_present=false`, `termination_reason=no_menu`, `page_size=0`,
`page_no=0`, `num_candidates=0`, `candidate_pointer_null=true`, no pages, and
no candidates. Its `captured_all_pages=true` records a real D-48 class-4
under-admission gap, not a paging interruption or truncated capture.

## Replay

Run the sequence in a **clean disposable checkout** at
`1fd3ca5f10c3eaec432266085c9f58271be39d1f`, with the pinned upstream assets
available and `target/m59-increment1-lane-a-final/` absent. Every generated
output must be new; do not replay over this packet or a prior scratch run.

```powershell
if ((git rev-parse HEAD) -ne '1fd3ca5f10c3eaec432266085c9f58271be39d1f') {
  throw 'Replay checkout is not at the bound source commit.'
}
if (git status --porcelain) {
  throw 'Replay checkout must start clean.'
}

$scratch = 'target/m59-increment1-lane-a-final'
if (Test-Path -LiteralPath $scratch) {
  throw "Replay scratch root must be absent: $scratch"
}

cargo build --release -p yune-rime-api

powershell.exe -NoProfile -ExecutionPolicy Bypass `
  -File scripts/capture-upstream-rime-cantonese.ps1 `
  -Output "$scratch/lane-a-oracle.json" `
  -EvidenceMilestone M59 `
  -ReportedCaseInput zijiguk `
  -ExpectedRimeDllSha256 86b4c7357d4c6d293ce5589b234d8859ca2ac30923a03bedfa3926eeaf97fb0b `
  -ExpectedRimeDeployerSha256 3abb72b5bb56fcafcfe925d533ae5f832c68d5a0bc9952fd0eea0682fb1ab071

powershell.exe -NoProfile -ExecutionPolicy Bypass `
  -File scripts/capture-yune-candidate-order.ps1 `
  -YuneDll target/release/yune_rime_api.dll `
  -SharedDataDir target/upstream-oracle/1.17.0/m58-rime-cantonese-shared `
  -SchemaId jyut6ping3 `
  -OracleCapture "$scratch/lane-a-oracle.json" `
  -Output "$scratch/lane-a-yune.json" `
  -DefaultYamlOverlay docs/reports/evidence/m59-canonical-jyutping-reachability-parity/lane-a-runner/default.yaml `
  -ExpectedYuneDllSha256 2ac5d2550cd462d52d11aec995f609a605ccc6aa9837a8e55cec18a7ec63cfa5

python -B scripts/compare-candidate-order.py `
  --oracle "$scratch/lane-a-oracle.json" `
  --actual "$scratch/lane-a-yune.json" `
  --policy exact `
  --output-json "$scratch/lane-a-exact-diff.json" `
  --output-csv "$scratch/lane-a-exact-diff.csv"

if ($LASTEXITCODE -ne 1) {
  throw "Expected the preserved pre-fix exact comparison to exit 1; got $LASTEXITCODE"
}
```

After verifying the four expected hashes and canonical text contract, copy the
four scratch outputs without transformation into a new packet directory.
`lane-a-manifest.json` binds the raw captures, commands, effective parameters,
output/import and canonical-text provenance, tools, trees, options,
historical-oracle equivalence, and all 13 red verdicts.
