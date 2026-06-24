# Product Storage Switch

M36 makes the product path compiled-active without widening `RimeApi`,
`RimeCandidate`, or the TypeDuck profile ABI.

Changes:

- Added `RimeDictRebuildSources::prism_artifact_stem`, so deployment can write
  table/reverse under the dictionary id and prism under the configured
  `translator/prism` id.
- Added schema-scoped `RimeRunTask("workspace_update:<schema_id>")` so the
  measured product schema can rebuild its artifacts without changing the ABI
  table.
- Added `native_inprocess_benchmark --deploy-before-benchmark` so Track B final
  runs can rebuild product artifacts once, then measure normal runtime
  setup/select/key paths.
- Enabled compact storage for TypeDuck only when the schema actually loaded a
  compiled dictionary with a prism payload. Unsupported or stale product paths
  still use the existing heap fallback.

The final product-path status confirms the switch:

| Dictionary | Table path | Prism path | Reverse path | Compiled ready |
| --- | --- | --- | --- | --- |
| `jyut6ping3` | `user/build/jyut6ping3.table.bin` | `user/build/jyut6ping3_mobile.prism.bin` | `user/build/jyut6ping3.reverse.bin` | true |
| `jyut6ping3_scolar` | `user/build/jyut6ping3_scolar.table.bin` | `user/build/jyut6ping3_scolar.prism.bin` | `user/build/jyut6ping3_scolar.reverse.bin` | true |

Heap fallback remains the stop condition for unsupported schemas, stale
compiled artifacts, missing prism payloads, and source-only test fixtures.
