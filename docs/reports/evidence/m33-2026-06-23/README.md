# M33 Native Lookup Performance Evidence

Date: 2026-06-23

This folder contains the M33 before/after evidence for the bounded native
lookup/fairness milestone.

## Commands

Cross-engine before and after:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-yune-vs-librime.ps1 -OutputRoot docs\reports\evidence\m33-2026-06-23\before-yune-vs-librime -Iterations 9 -SessionIterations 9 -KeyIterations 25
powershell -ExecutionPolicy Bypass -File scripts\benchmark-yune-vs-librime.ps1 -OutputRoot docs\reports\evidence\m33-2026-06-23\after-low-risk-yune-vs-librime -Iterations 9 -SessionIterations 9 -KeyIterations 25
```

Native before and after:

```powershell
cmd /c "cargo bench -p yune-rime-api --bench frontend_baselines > target\m33-frontend-baselines-before.txt 2>&1"
cmd /c "cargo bench -p yune-rime-api --bench frontend_baselines > target\m33-frontend-baselines-after-low-risk.txt 2>&1"
```

The native logs were copied into this folder as:

- [`frontend-baselines-before.txt`](./frontend-baselines-before.txt)
- [`frontend-baselines-after-low-risk.txt`](./frontend-baselines-after-low-risk.txt)

## Accepted M33 slice

- Build-once sharing for immutable dictionary translators, with cache invalidated
  by schema and source/compiled asset signatures.
- Lazy `stroke` reverse-lookup dictionary loading, so no-reverse `luna_pinyin`
  startup/session rows no longer compare a luna-plus-stroke Yune load against a
  luna-only librime load.

## Deferred levers

- Lazy prism/spelling-algebra lookup: no-go for M33. The upstream prism fixture
  maps spellings to syllable descriptors but does not contain candidate
  text/comment/order payloads. A byte-identical rewrite needs a broader
  table-payload/index design.
- mmap compiled artifacts: deferred. The low-risk slice already moved
  startup/session into the same order of magnitude as librime, while the
  remaining measured gap is per-key lookup representation.

## Headline numbers

| Row | Yune before | Yune after | librime after |
| --- | ---: | ---: | ---: |
| Startup/runtime-ready median | `2,881,852.7 us` | `47,788.2 us` | `27,628.3 us` |
| Session create/select/destroy median | `2,985,364.0 us` | `47,813.7 us` | `25,765.9 us` |
| Startup ready working-set delta | `218,873,856 bytes` | `24,576 bytes` | `847,872 bytes` |
| Key `ni` median | `5,579.8 us` | `6,064.5 us` | `28.5 us` |
| Key `hao` median | `11,043.8 us` | `12,463.4 us` | `34.5 us` |
| Key `zhongguo` median | `34,024.0 us` | `37,572.3 us` | `1,479.8 us` |

## Interpretation

The startup/session rows are now fair and safe to show with caveats. The per-key
rows are still not competitive and should not be described as a typing win.
Browser startup and browser typing were not measured in M33.
