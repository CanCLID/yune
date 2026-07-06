# M58 Phase 0 Harness And Provenance

Status: complete.

M58 canonical capture uses upstream `rime/librime 1.17.0` plus pinned
`rime/rime-cantonese`. TypeDuck-HK/librime v1.1.2 is not used as the canonical
candidate-ordering, segmentation, fallback, or completion oracle.

## Harness

- Script: `scripts/capture-upstream-rime-cantonese.ps1`
- Probe: `scripts/oracle-rime-probe.cs`
- Output: `docs/reports/evidence/m58-jyutping-exact-before-fuzzy/phase-1/canonical-rime-cantonese-capture.json`
- Command:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\capture-upstream-rime-cantonese.ps1 -ReportedCaseInput zijiguk
```

The script stages schema data under `target/upstream-oracle/1.17.0`, deploys
with the upstream `rime_deployer.exe`, then captures all pages through
upstream `rime.dll`. The script validates the expected SHA-256 for both
upstream binaries before capture and records the observed hashes in the raw
Phase 1 JSON.

## Provenance

| Field | Value |
| --- | --- |
| Oracle engine | `rime/librime` `1.17.0` |
| Oracle commit | `33e78140250125871856cdc5b42ddc6a5fcd3cd4` |
| `rime.dll` SHA-256 | `86b4c7357d4c6d293ce5589b234d8859ca2ac30923a03bedfa3926eeaf97fb0b` |
| `rime_deployer.exe` SHA-256 | `3abb72b5bb56fcafcfe925d533ae5f832c68d5a0bc9952fd0eea0682fb1ab071` |
| Yune-facing schema id | `jyut6ping3` |
| Canonical schema repo | `rime/rime-cantonese` |
| Canonical schema commit | `c99b16e44d2df77a5cb8fb0867dd2bab7a112cb0` |
| Reported target | `諮議局` (`U+8AEE U+8B70 U+5C40`) |
| Reported capture input | `zijiguk` |
| Runtime option patches | none |
| Custom YAML | `default.custom.yaml` selects `jyut6ping3` only |
| Observed page size | `5` |

Dependency commits:

| Repo | Commit |
| --- | --- |
| `rime/rime-prelude` | `082425ea0684bca36474415d4a0e8db9b016487e` |
| `rime/rime-luna-pinyin` | `18a80335c37522311f7cff02886cd81cec3b460a` |
| `rime/rime-essay` | `48c7538f0b760fcc8c9d6bf08711f82cfbd2e9ed` |
| `rime/rime-stroke` | `3a4b0f4013e2b4c14b1e80c92b1d4723eb65f39c` |
| `rime/rime-cangjie` | `52d90a1b1312e74042b38c1cbc8142defbc53171` |
| `CanCLID/rime-loengfan` | `987ac95b02f957e8764a2f45222a4006c188ed50` |
