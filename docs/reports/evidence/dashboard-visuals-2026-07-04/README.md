# Dashboard Visuals 2026-07-04

Current native dashboard SVGs regenerated after M55 final run 6.

- `native-track-a-latency-ratios.svg` - native Track A same-run Yune/librime
  latency ratios from
  `../m55-native-match-or-beat/phase-5-final/default-on-ratchet-6-config-cache/summary-comparison.csv`.
- `native-track-a-memory.svg` - native Track A process high-water and owner
  diagnostics from the same final run's `summary-comparison.csv` and
  `memory-owner-profile.csv`.
- `native-track-b-memory.svg` and `native-track-b-latency.svg` - Track B
  TypeDuck `jyut6ping3_mobile` product guard rows from the same final run's
  `summary.csv`.
- `root-cause-gaps.svg` - current remaining same-run peer ratios for native
  Track A, plus carried browser fair-lane ratios.

Browser visualizations are not regenerated here; M55 did not remeasure the
browser lane, so browser charts remain carried from
`../current-performance-dashboard-2026-06-29/visuals/`.

These are the single-platform (Windows) visuals. The Windows-vs-macOS
cross-platform Track A latency chart (post-M57) lives in
[`../dashboard-visuals-2026-07-05-cross-platform/`](../dashboard-visuals-2026-07-05-cross-platform/)
(matplotlib generator).

Re-run:

```powershell
python docs\reports\evidence\dashboard-visuals-2026-07-04\generate.py
```
