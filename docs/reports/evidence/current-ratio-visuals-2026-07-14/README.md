# Current ratio visualizations — 2026-07-14

This bundle contains report-facing derived data and reproducible SVGs. It does
not replace any signed baseline or raw benchmark packet.

## Ratio contract

For native latency, `ratio = Yune latency / same-run librime latency`:

- below `1.000x`: Yune used less latency;
- exactly `1.000x`: parity;
- above `1.000x`: librime used less latency.

For the browser peer chart, the denominator is My RIME rather than librime and
the same lower/equal/higher convention applies. A ratio such as `0.250x` means
Yune used 25% of the peer value, not “0.250x faster.”

## Sources

- `current-windows-track-a-ratios.csv`: derived from
  `../m59-canonical-jyutping-reachability-parity/source-current-performance-revalidation-2026-07-13/gate-verdict.csv`.
  Measured Yune source is `443cc636862806e4f0dd1e12ab2e2e45f4189154`;
  pinned librime is `33e78140250125871856cdc5b42ddc6a5fcd3cd4`.
- `current-macos-track-a-ratios.csv`: derived from the external five-round
  source-current packet at
  `$HOME/yune-m59-current-macos-20260714/aggregate/track-a-17-row-comparison.csv`.
  Measured Yune source is `0111cf47c09bfe7a4a3d55a1832f35a55bc59435`;
  pinned librime is `33e78140250125871856cdc5b42ddc6a5fcd3cd4`.
- `browser-peer-ratios.csv`: derived from
  `../current-performance-dashboard-2026-06-28/current-browser-peer-comparator.csv`.

The current Mac values are not source-matched to final-Windows `443cc636`, so
the current dashboard keeps them in separate platform charts and forms no
cross-platform delta.

`external-evidence-manifest.csv` records SHA-256 identities for the external
Mac aggregate files and the local/deployed web receipts. It also preserves the
deployed setup failure and explicit `setup-retry-1` disposition. The web rows
are pre-hardening `0111cf47` evidence: later fail-closed receipt validation at
`68df2d16` has not been remeasured, so these hashes do not prove current-main
web performance.

## Chart map

| SVG | Question | Form | Comparison boundary |
| --- | --- | --- | --- |
| `visuals/current-windows-track-a-parity-1-of-2.svg` and `-2-of-2.svg` | Are final-M59 Windows medians and worst observations below librime parity? | two mobile-readable 9/8 logarithmic interval panels | Windows `443cc636`, same-run librime peer |
| `visuals/current-macos-track-a-parity-1-of-2.svg` and `-2-of-2.svg` | Are the latest measured Mac medians and pooled worsts below librime parity? | two mobile-readable 9/8 logarithmic interval panels | Mac `0111cf47`, same-Mac librime peer |
| `visuals/browser-luna-peer-parity.svg` | Which fair same-schema browser metrics favor Yune or My RIME? | logarithmic diverging ratio bars | dated 2026-06-28 `luna_pinyin` peer snapshot |

All charts use position plus direct text/shape, a dark `1.000x` line, exact
values visible without hover, and explicit numerator/denominator semantics.
The dashboard-facing native panels include `760 px` intrinsic SVG dimensions,
split the 17 rows 9/8, and use 18–20 px primary labels/values. The browser
panel uses the same narrow-reader width and typography, wrapping its longest
metric label and leaving raw values to the adjacent dashboard table. The
original combined native SVGs remain in the packet as preserved superseded
renderings; the dashboard uses the split panels. Regenerate with
`python3 build_visuals.py`.

The source-matched `5879405c` and `afb7079b` behavior-control charts were moved
to [`../history/performance-ratio-visuals-2026-07-14/`](../history/performance-ratio-visuals-2026-07-14/)
with the other historical performance material.
