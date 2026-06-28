# yune-web Cache Policy

M31 uses a content-addressed public asset manifest rather than relying only on
query-string versioning.

- `schema-asset-manifest.json` pins every deployed schema/OpenCC/WASM-adjacent
  schema payload by SHA-256 and byte count.
- The public worker opens Cache Storage bucket
  `yune-web-assets-web03-three-schema-launch-v1`.
- Each schema asset is cached under `schema/<path>?sha256=<manifest hash>`.
- A warm visit can reuse unchanged schema payloads while a changed hash creates
  a new cache key.
- WEB-03 bumps the bucket because the launch bundle now ships regenerated
  `Rime::Prism/4.0` assets plus Cangjie compiled table/prism/reverse payloads.
- The existing Yune Emscripten IDBFS deploy cache still records runtime deploy
  cache hits/misses separately.
- The Cloudflare deployment uses Pages Direct Upload at
  `https://yune-web.pages.dev` with SPA fallback configured
  through Pages native SPA rendering; the deploy has no top-level `404.html`.

Evidence must keep the claim boundary explicit: this is a delivery/cache
readiness claim. It is not a browser startup or typing performance win unless a
fresh real-browser measurement records that separately.
