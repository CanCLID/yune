# yune-web Compatibility Re-exports

The active Yune integration layer lives in
`apps/yune-web/src/yune-integration/`.

This top-level directory remains only for older repo references and tests. Its
implementation files re-export the active app-root modules:

- `adapter.ts` -> `../src/yune-integration/adapter.js`
- `assets.ts` -> `../src/yune-integration/assets.js`
- `response.ts` -> `../src/yune-integration/response.js`

New app work should import from `src/yune-integration/` inside the Vite app.
