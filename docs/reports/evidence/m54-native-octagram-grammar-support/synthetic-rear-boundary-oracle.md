# M54 Synthetic Rear-Boundary Oracle

> **Status:** Complete - **Milestone:** M54 follow-up - **Updated:** 2026-07-01 - **Type:** executable oracle evidence

This follow-up fixture closes OCTA-2 with an executable, non-circular oracle
case for OCTA-1. The fixture is
`crates/yune-core/tests/fixtures/upstream-octagram/synthetic-rear-boundary-oracle.json`.

The synthetic schema is Yune-owned and intentionally tiny:

- table rows: `B	a	1000` and `A	a	1`
- octagram row: `A$ = 1000000`
- input: `a`

Upstream librime plus `librime-octagram` returns `B`, then `A`. That proves the
rear-boundary `$` score does not apply to the first whole-input candidate with
empty context; upstream returns `non_collocation_penalty` before rear scoring
when `context.empty()`.

The committed fixture stores the tiny synthetic `.gram` bytes inline as hex
chunks, the librime-produced candidate bytes for input `a`, the oracle binary
hashes and capture command, and the schema/dictionary hashes used for capture.

The executable Rust regression decodes the committed `.gram` fixture bytes,
builds the matching Yune sentence model, and compares Yune's candidate order to
the librime-produced expected candidates. It does not derive expected candidate
bytes from Yune.

Capture recipe:

```powershell
$env:PATH = "target\m54-native-octagram\oracle-librime-octagram\lib;" +
  "target\m54-native-octagram\oracle-librime-octagram\bin;" + $env:PATH

target\m54-native-octagram\oracle-librime-octagram\bin\rime_deployer.exe `
  --build `
  target\m54-native-octagram\synthetic-oracle\rime-user `
  target\m54-native-octagram\synthetic-oracle\rime-shared `
  target\m54-native-octagram\synthetic-oracle\rime-user\build

Add-Type -Path scripts\oracle-rime-probe.cs
[RimeProbe]::Capture(
  "target\m54-native-octagram\synthetic-oracle\rime-shared",
  "target\m54-native-octagram\synthetic-oracle\rime-user",
  "target\m54-native-octagram\synthetic-oracle\rime-user\build",
  "m54_synthetic_octagram",
  [string[]]@("default", "octagram"),
  [string[]]@("a"))
```
