# M31 AI Posture

Date: 2026-06-24

M31 did not add richer AI UX and did not add remote providers.

Public smoke evidence:

- AI Candidates remains default-off.
- Browser resource scan in `opencc-browser-evidence.json` found no requests
  matching `openai`, `anthropic`, `telemetry`, `analytics`, `segment`, or
  `sentry`.
- `opencc-browser-evidence.json` records `remoteCalls: []`.

Scope:

- The existing M13 local-only second-pass path remains available behind the
  explicit AI Candidates toggle.
- AI-off classic candidate output is preserved.
- No telemetry or secrets are committed in the public-demo package.
