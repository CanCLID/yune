# Cangjie oracle-validation runner

This directory contains the one-key marker used only by M59's marked Cangjie
oracle-validation lane. It is not installed into the product schema tree and
does not modify the pinned `rime/rime-cangjie` source checkout.

`scripts/stage-m59-schema-validation-overlay.ps1` copies a supplied shared-data
tree to a create-new disposable destination, validates the patch bytes exactly,
and inserts:

```yaml
translator:
  yune_sentence_policy: upstream_script
```

into the disposable `<schema-id>.schema.yaml`. The destination filename is
derived from a validated logical schema id. The staging manifest records the
source tree, source schema, patch, staged schema, and staged tree SHA-256 values
plus the Yune commit/tree/status and staging-tool identity.

The patch is applied to the disposable schema copy because Rime consumes
`<schema-id>.custom.yaml` from user data, while the existing byte-pinned M59
capture tool owns and creates its isolated user-data directory. Historical
capture-tool bytes therefore remain unchanged.

Example:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File scripts/stage-m59-schema-validation-overlay.ps1 `
  -SourceSharedDataDir C:\path\to\unmarked-shared `
  -OutputSharedDataDir C:\m59-4d-marked\shared `
  -SchemaId cangjie5 `
  -SchemaCustomOverlay docs\reports\evidence\m59-canonical-jyutping-reachability-parity\cangjie-validation-runner\cangjie5.custom.yaml `
  -ManifestOutput C:\m59-4d-marked\staging-manifest.json
```

The staged shared tree is then passed unchanged to
`scripts/capture-yune-candidate-order.ps1`. Final acceptance also captures the
unmarked control separately.

Final Increment 4d acceptance is preserved under
[`../increment-4d-cangjie-cj1/`](../increment-4d-cangjie-cj1/): marked strict
`12/12`, owning parity `3 passed / 0 ignored`, and unmarked control exact
`12/12` at clean source `38e759f6`.
