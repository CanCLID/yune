# M31 OpenCC Support Audit

Date: 2026-06-24

Decision: `current_simplification_only`.

The first public demo exposes only:

- Hong Kong Traditional.
- Simplified Chinese through the current `hk2s` OpenCC chain.

Unsupported output standards are not exposed as public working controls. Yune's
core recognizes additional internal config names, but the public web path has
browser/runtime proof only for the `simplification` option backed by
`hk2s.json` in the `jyut6ping3_mobile` schema.

Evidence:

- Native/API assertion: `cargo test -p yune-rime-api --test typeduck_web
  m31_typeduck_web_hk2s_option_changes_real_asset_candidates`.
- Browser smoke: `M31 PUBLIC hk2s output standard is browser-visible and AI
  stays default-off @smoke`.
- Browser JSON: `opencc-browser-evidence.json`.
