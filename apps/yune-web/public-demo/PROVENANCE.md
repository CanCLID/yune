# yune-web Provenance

`yune-web` exercises the Yune input-method engine through a
TypeDuck-Web-derived browser harness. The harness and Cantonese dictionary
assets are used with project-owner approval from the TypeDuck author.

Public scope:

- Public schemas: `jyut6ping3`, `cangjie5`, and upstream-derived
  `luna_pinyin`.
- Public output standards: Hong Kong Traditional, Taiwan Traditional, and
  Simplified Chinese through OpenCC chains.
- AI posture: default-off, local-only second-pass candidate logic; no remote AI
  calls, no telemetry, and no committed secrets.
- App path: `apps/yune-web/` is the tracked Vite app. The retired
  `apps/yune-web/patches/yune-web-runtime.patch` remains as the migration
  baseline from the old upstream-derived checkout.

This provenance note is public copy. It does not widen Yune's default librime
ABI and does not represent the separate TypeDuck-Web product repository.
