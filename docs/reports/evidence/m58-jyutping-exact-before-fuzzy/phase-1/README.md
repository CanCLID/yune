# M58 Phase 1 Canonical Upstream Capture

Status: complete.

This directory contains complete all-pages canonical upstream
`rime-cantonese` capture evidence for M58. The reported case is the
user-specified target `諮議局`, captured only by its exact ASCII keystrokes
`zijiguk`; expected output is not derived from Yune.

## Harness

- Script: `scripts/capture-upstream-rime-cantonese.ps1`
- Probe: `scripts/oracle-rime-probe.cs`
- Command:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\capture-upstream-rime-cantonese.ps1 -ReportedCaseInput zijiguk
```

The script stages upstream schema data under `target/upstream-oracle/1.17.0`,
runs the upstream deployer, captures candidate pages through the real upstream
Rime API path, validates the expected upstream binary SHA-256s, and fails unless
every case reports `captured_all_pages: true`.

## Provenance

Canonical lane only. TypeDuck-HK/librime v1.1.2 was not used for canonical
candidate ordering, segmentation, fallback, or completion.

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
| Reported input | `zijiguk` |
| Runtime option patches | none |
| Custom YAML | `default.custom.yaml` selects `jyut6ping3` only |
| Observed page size | `5` |

Dependency commits match the Phase 0 provenance note.

## Captured Inputs

`canonical-rime-cantonese-capture.json` contains complete all-pages captures for:

- `be`, `bei`, `bein`, `being`, `beingo`
- `beix`, `beixngoxx`
- `ngohaig`, `ngohaigo`
- `n`, `nri`
- `mgoi`
- `zijiguk`

Every captured case has `captured_all_pages: true`. The largest captured case is
`ngohaig`, with 2,050 ordered candidates under page size 5.

## Observations

| Input | Candidate count | First candidates | Relevant reachability |
| --- | ---: | --- | --- |
| `zijiguk` | 416 | `諮議局`, `子怡`, `自已`, `旨意`, `之二` | Reported target `諮議局` is first; no canonical admission/order issue reproduced. |
| `beingo` | 142 | `比我`, `被我`, `畀我`, `畀`, `比` | Standalone `畀` is on page 1 at index 3. |
| `being` | 140 | `畀嗯`, `畀`, `比`, `被`, `鼻` | Standalone `畀` is on page 1 at index 1. |
| `beixngoxx` | 38 | `比我`, `畀我`, `畀`, `比`, `髀` | Tone-scoped standalone `畀` is on page 1 at index 2. |
| `mgoi` | 4 | `唔該`, `唔`, `呣`, `嘸` | Common multi-syllable control captured. |

## Disposition

The canonical upstream lane does not require a Yune candidate-ordering fix for
`zijiguk`. The shipped `beingo` bug is therefore handled as a scoped
TypeDuck/profile product-lane issue in Phase 2b, not as a canonical
`rime-cantonese` ordering claim.
