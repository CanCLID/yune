# M31 OpenCC Native Gates

Date: 2026-06-24

Focused native gate:

```powershell
cargo test -p yune-rime-api --test typeduck_web m31_typeduck_web_hk2s_option_changes_real_asset_candidates
```

Result: passed.

What it proves:

- The real `jyut6ping3_mobile` browser app asset path initializes.
- Default output for `ngohaigo` remains Hong Kong Traditional:
  `\u6211\u4fc2\u500b`.
- Setting `yune_typeduck_set_option("simplification", TRUE)` changes the top
  real-assets candidate to hk2s simplified output: `\u6211\u7cfb\u4e2a`.

What it does not prove:

- It does not prove any broader OpenCC standard matrix.
- It does not change the default `RimeApi`, `RimeCandidate`, or upstream ABI.
