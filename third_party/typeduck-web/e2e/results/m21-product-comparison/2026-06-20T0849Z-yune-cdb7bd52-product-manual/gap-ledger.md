# Gap Ledger

All rows below are dispositioned against the hard TypeDuck v1.1.2 oracle fixtures.
The deployed `typeduck.hk/web` product remains a moving feel target; any future
manual product capture should be compared to these pinned rows rather than treated
as a new hard oracle by itself.

| Input | Product output | Yune output | Label | Disposition |
|---|---|---|---|---|
| `nei` | Feel target only; no hard-oracle action pending | `jyut6ping3-m21-closeout.json` top 5: `你`, `呢`, `尼`, `妮`, `彌` | match | Yune matches v1.1.2. |
| `ngo` | Feel target only; no hard-oracle action pending | `jyut6ping3-m21-closeout.json` top 5: `我`, `俄`, `柯`, `餓`, `屙` | match | Yune matches v1.1.2. |
| `santai` | Feel target only; no hard-oracle action pending | M21-GAP-02 oracle/Yune top 5: `身體`, `身體健康`, `神體`, `新`, `身` | oracle-backed-fixed | `jyut6ping3-m21-prediction-ranking.json` proves the TypeDuck v1.1.2 prediction-count limit; Yune adopts that `jyut6ping3` profile exception without broad fork ranking byte parity. |
| `sigin` | Feel target only; no hard-oracle action pending | M21-GAP-02 oracle/Yune top 5: `事件`, `市建局`, `時`, `是`, `事` | oracle-backed-fixed | Same M21-GAP-02 prediction-count fixture; single-character rows stay on page 1. |
| `m` | Feel target only; no hard-oracle action pending | `jyut6ping3-m21-closeout.json` top 5: `唔`, `五`, `午`, `誤`, `吳` | oracle-backed-fixed | Closeout fixture exposed and fixed the standalone `m` abbreviation/fuzzy ordering gap. |
| `mgoi` | Feel target only; no hard-oracle action pending | `jyut6ping3-m21-closeout.json` top 5: `唔該`, `唔該晒`, `唔過`, `五個`, `每個` | oracle-backed-fixed | Closeout fixture exposed and fixed the two-syllable `m` abbreviation/fuzzy correction family. |
| `ngohaigo` | Feel target only; no hard-oracle action pending | `jyut6ping3-m21-closeout.json` top 5: `我係個`, `我係`, `我喺`, `我`, `俄` | match | Yune matches v1.1.2 sentence plus prefix-fallback rows. |
| `leoicijyu` | Live-site observation: `類似於` | Oracle v1.1.2 / Yune M21-GAP-01: top 1 `類似如` | expected-by-design | Version skew: hard `v1.1.2` oracle differs from the moving deployed product, so do not re-investigate this as a regression unless a new pinned oracle fixture changes. |
| `hou` | Feel target only; no hard-oracle action pending | `jyut6ping3-m21-closeout.json` top 5: `好`, `號`, `豪`, `毫`, `浩` | match | Yune matches v1.1.2. |
| tone letters `neivv` | Feel target only; no hard-oracle action pending | `jyut6ping3-m21-closeout.json` top 1: `尼`; preedit `nei4` | match | Replaces `seov` so the letter-to-tone corpus does not collide with the v1.1.2 `eo`/`oe` lazy-sound fuzzy rule. |
| tone letters `seov` | Live-site appears to differ | Yune/v1.1.2 applies `include.yaml` `derive/eo/oe/ # 容錯 eo/oe 不分` | expected-by-design | Version skew: the hard v1.1.2 schema has `eo`/`oe` fuzzy, while the moving deployed product appears to have refined or dropped it. Do not re-investigate as a regression without a new pinned oracle fixture. |
| 1-edit typo `nri`, correction off | Feel target only; no hard-oracle action pending | Browser renders top 5 `我`, `你`, `外`, `能`, `內`; engine fixture also locks row 6 `呢` and commit preview `我ri` | oracle-backed-fixed | M21-GAP-02 prefix fallback: full `nri` miss falls back to leading `n` and leaves raw `ri`. |
| 1-edit typo `nri`, correction on | Feel target only; no hard-oracle action pending | Browser renders `你` first and commits `你` | oracle-backed-fixed | M21-GAP-02 correction-enabled path matches the existing v1.1.2 M14 fixture. |
| hk2s case `ngohaigo`, simplification on | Feel target only; no hard-oracle action pending | `jyut6ping3-m21-closeout.json` top 5: `我系个`, `我系`, `我喺`, `我`, `俄` | match | Yune matches v1.1.2 `hk2s` simplification behavior. |
| reverse-lookup/comment case `nei` | Current single-schema browser surface cannot exercise the side-lookup UI | Candidate rows include Jyutping and dictionary text, e.g. top row `nei5 你 you (singular)` | browser-surface-N/A | Engine/comment bytes are already oracle-backed; M22 multi-schema work can expose a richer browser surface for reverse lookup. |
| multi-page input `nei` | Feel target only; no hard-oracle action pending | Page 1: `你`, `呢`, `尼`, `妮`, `彌`; Page 2: `妳`, `您`, `膩`, `餌`, `瀰` | match | Yune matches v1.1.2 top-10 paging split at page size 5. |

Real should-match signal count in this snapshot: `6` hard-oracle-backed fixes
(`santai`, `sigin`, `m`, `mgoi`, `nri` correction off, `nri` correction on).
All other rows are either v1.1.2 matches, expected version skew, or current
browser-surface N/A. M21 has no remaining hard-oracle action rows.
