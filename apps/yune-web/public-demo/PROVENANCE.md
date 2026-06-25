# yune-web Provenance

`yune-web` exercises the Yune input-method engine through a
TypeDuck-Web-derived browser harness. The harness and Cantonese dictionary
assets are used with project-owner approval from the TypeDuck author.

M31 public scope is intentionally narrow:

- Public schema: `jyut6ping3_mobile`.
- Public output standards: Hong Kong Traditional and Simplified Chinese through
  the `hk2s` OpenCC chain.
- AI posture: default-off, local-only second-pass candidate logic; no remote AI
  calls, no telemetry, and no committed secrets.
- App path: `apps/yune-web/` is the tracked Vite app. The retired
  `apps/yune-web/patches/yune-web-runtime.patch` remains as the migration
  baseline from the old upstream-derived checkout.

This provenance note is public copy. It does not widen Yune's default librime
ABI and does not represent the separate TypeDuck-Web product repository.
