# Report source notes

Audience: technical. Required sections are mapped directly to technical summary, findings, scope/definitions, methodology, limitations/robustness, recommendations, and further questions.

Chart map: the Track A finding uses two single-series horizontal bar charts (9 and 8 measured rows) of mac_median_ratio by a compact input-plus-exact-ratio label. Each chart adds a clearly named synthetic 1.000x parity anchor solely to keep the comparison boundary in view. The charts support only the same-Mac direction; Windows deltas remain tabular because their sources do not match. Palette policy is single-root preferred with no redundant series legend. Final surface is report/report.html.

Reproduction: run `python3 aggregate/analyze.py --evidence-root /absolute/path/to/yune-m59-current-macos-20260714 --repo-root /absolute/path/to/yune`, then this builder, then `node report/deliver_report.mjs --plugin-root /absolute/path/to/data-analytics-plugin --input report/artifact.json --output report/report.html`. The packet-local adapter delegates to the canonical portable-report delivery and verification pipeline, adding only a fail-closed desktop-width containment rule for the packaged reader's `100vw` header. The final receipt must report both 1440 px and 390 px viewports plus a passed source interaction.
