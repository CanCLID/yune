# M29 Memory Classification

Date: 2026-06-22

Command:

```powershell
cmd /c "cargo bench -p yune-rime-api --bench frontend_baselines > target\m29-frontend-baselines-before.txt 2>&1"
```

Captured output: `target/m29-frontend-baselines-before.txt`.

## Classification

Classification for the M27-style `1.79GB` peak: **benchmark cumulative high-water**.

Evidence also shows real per-startup ready pressure of about `1.10GB`, but the specific `1.79GB` peak comes from the repeated startup-trace benchmark high-water path rather than the single-startup probe.

| Category | Finding |
| --- | --- |
| Per-startup pressure | Real and still large: single-startup after-ready working set was `1,105,899,520` bytes before optimization. |
| Benchmark cumulative high-water | Selected for the `1.79GB` peak: repeated startup trace reported `1,795,403,776` peak bytes while the single-startup peak was `1,126,006,784` bytes. |
| Unresolved | No; the single-startup row separates the peak forms. |

## Single-Startup Evidence

- Before bytes: `4,739,072`.
- After-ready bytes: `1,105,899,520`.
- After-finalize bytes: `14,897,152`.
- Peak bytes: `1,126,006,784`.
- Ready delta bytes: `1,101,160,448`.
- Finalize delta bytes: `-1,091,002,368`.

## Repeated-Trace Evidence

- `startup_trace_jyut6ping3_mobile_select_schema_total` peak: `1,795,403,776` bytes.
- `startup_trace_jyut6ping3_mobile_spelling_algebra_expand` peak: `1,795,403,776` bytes.
- The repeated-trace working-set deltas around `690MB` remain useful owner attribution, but the peak value is not a single-startup retained footprint.
