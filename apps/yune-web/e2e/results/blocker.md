# Browser E2E Blocker / Supersession Notes

**Date**: 2026-06-18

The earlier WI-4 evidence in this directory used Yune's echo placeholder path and is superseded for the real-assets candidate gate by HR-1b. The refreshed artifacts are:

- `browser-run.log` — focused HR-1b real-assets browser smoke.
- `browser-console.json` — console evidence for reload/init and `processKey({n/e/i})`.
- `dom-snapshot-candidates.txt` — candidate panel DOM excerpt showing `nei -> 你 / 呢 / 尼`.
- `screenshot-real-assets-nei.png` — browser screenshot of the real candidate panel.

**Current status**: HR-1b PASS for real-assets candidate rendering.

**HR-3 update**: `deploy()` now returns true with real assets. The failure was an
incomplete browser preload list: deployment reaches the plain
`jyut6ping3.schema.yaml` through TypeDuck's real workspace/default schema path.
See `deploy-browser.log` for the browser console proof.

**HR-4 update**: live persistence is browser-proven. `persistence-sync.log`
records a fresh-origin load where before-init persistence is empty, startup
`customize` writes `page_size: '6'`, deploy syncs after mutation, and a real page
reload restores the persisted custom config before runtime init.

**HR-5 update**: the full real-assets browser matrix now passes. See
`hr5-real-assets-matrix.json`, `screenshot-hr5-dictionary-panel.png`, and
`screenshot-hr5-after-delete.png` for composition, candidate list, paging,
selection, Space commit, long-press deletion, deploy, customize, persistence
sync, reload, and dictionary-panel comment evidence. The final HR-5 browser
capture has zero warning/error console entries.

**Post-review M9 closeout**: the delete/backspace banner condition is closed.
The banner was a real delete-path issue, not just stale evidence: TypeDuck-Web
sent a standalone `{Control_L}` keydown before `{Control+Delete}` while
composing, and the adapter tried to send that pure modifier to Yune. The later
`{Control+Delete}` still deleted the candidate, but the rejected modifier event
showed the operation banner. The adapter now passes pure modifier keydowns
through like key releases. `hr5-final-delete-state.json` and
`hr5-final-backspace-state.json` were recaptured from fresh pages and show no
runtime-error banner, zero warning/error entries, and functional state changes.

**Post-review partial-final sentence completion**: the `ngohaig` compatibility
regression is closed. TypeDuck v1.1.2 returns `我係個` for `ngohaig`, treating the
final `g` as a prefix completion after the exact `ngohai -> 我係` segment. Yune now
allows sentence completion on the final segment only when completion and sentence
mode are enabled. `hr5-final-ngohaig-state.json` records a real-assets browser run
where `ngohaig` renders `1. 我係個` and Space commits `我係個` with no runtime-error
banner or warning/error entries.

**Post-review dictionary-comment reproducibility**: the browser-shaped native
rich-comment test now byte-asserts the v1.1.2 `nei` comment only when local
oracle build assets exist under `target/typeduck-oracle/v1.1.2/rime-user/build`.
If those ignored assets are absent, it emits an explicit skip reason rather than
passing against a degraded fallback. The committed clean-checkout byte-parity
guarantee is `cargo test -p yune-core --test cantonese_parity`.

**HR-6 update**: shared reverse-lookup parity is oracle-covered for the `"; "`
joiner and schema-prompt preedit bytes. The five broader Cantonese/Jyutping
goldens remain explicit ignored tests until dedicated v1.1.2 fixtures are
captured.

**HR-7 update**: the final recommendation is **GO WITH CONDITIONS** for gated
AI-native frontend exposure. The real-assets browser matrix has no remaining
blocking rows; AI-native behavior stays disabled by default in real frontends
until the separate M11 provider/ranking/privacy contracts are proven and
explicitly enabled.

**Still open**: none for the M9 TypeDuck-Web browser matrix. Non-browser
Cantonese parity captures remain the five explicit ignored tests in
`crates/yune-core/tests/cantonese_parity.rs`.
