# Gap Ledger

Product-side rows are pending manual capture. No row below is a Yune bug signal until the deployed product output is manually captured, stamped, and compared against the hard `v1.1.2` oracle path.

| Input | Product output | Yune output | Label | Disposition |
|---|---|---|---|---|
| `nei` | pending manual capture | Top 5: `你`, `呢`, `尼`, `妮`, `彌` | pending-product-capture | Capture product top-N manually before classifying. |
| `ngo` | pending manual capture | Top 5: `我`, `俄`, `柯`, `餓`, `屙` | pending-product-capture | Capture product top-N manually before classifying. |
| `santai` | pending manual capture | M21-GAP-02 oracle/Yune top 5: `身體`, `身體健康`, `神體`, `新`, `身` | oracle-backed-fixed | `jyut6ping3-m21-prediction-ranking.json` proves the TypeDuck v1.1.2 prediction-count limit; Yune now adopts that `jyut6ping3` profile exception without broad fork ranking byte parity. |
| `sigin` | pending manual capture | M21-GAP-02 oracle/Yune top 5: `事件`, `市建局`, `時`, `是`, `事` | oracle-backed-fixed | Same M21-GAP-02 prediction-count fixture; single-character rows stay on page 1. |
| `m` | pending manual capture | Top 5: `唔`, `無`, `面`, `明`, `民` | pending-product-capture | Should-match standalone-m/fuzzy path, but no bug claim without product capture. |
| `mgoi` | pending manual capture | Top 2: `唔該`, `唔該晒` | pending-product-capture | Should-match fuzzy/容錯 path, but no bug claim without product capture. |
| `ngohaigo` | pending manual capture | Top 1: `我係個` | pending-product-capture | Sentence/composition differences may be pending-M17-M19; classify only after manual product output. |
| `leoicijyu` | live-site observation: `類似於`; capture still pending | Oracle v1.1.2 / Yune M21-GAP-01: top 1 `類似如` | expected-by-design | Version skew: hard `v1.1.2` oracle differs from the moving deployed product, so do not re-investigate this as a regression unless a new pinned oracle fixture changes. |
| `hou` | pending manual capture | Top 5: `好`, `號`, `豪`, `毫`, `浩` | pending-product-capture | Should-match combine/separate behavior needs matched product setting before classification. |
| tone letters `neivv` | pending manual capture | Top 1: `尼`; preedit `nei4` | pending-product-capture | Replaces `seov` so the letter-to-tone corpus does not collide with the v1.1.2 `eo`/`oe` lazy-sound fuzzy rule. |
| tone letters `seov` | live-site appears to differ; capture still pending | Yune/v1.1.2 applies `include.yaml` `derive/eo/oe/ # 容錯 eo/oe 不分` | expected-by-design | Version skew: the hard v1.1.2 schema has `eo`/`oe` fuzzy, while the moving deployed product appears to have refined or dropped it. Do not re-investigate as a regression without a new pinned oracle fixture. |
| 1-edit typo `nri`, correction off | pending manual capture | Browser now renders top 5 `我`, `你`, `外`, `能`, `內`; engine fixture also locks row 6 `呢` and commit preview `我ri` | oracle-backed-fixed | M21-GAP-02 prefix fallback: full `nri` miss falls back to leading `n` and leaves raw `ri`. |
| 1-edit typo `nri`, correction on | pending manual capture | Browser now renders `你` first and commits `你` | oracle-backed-fixed | M21-GAP-02 correction-enabled path matches the existing v1.1.2 M14 fixture. |
| hk2s case `ngohaigo`, simplification on | pending manual capture | Top 1: `我系个` | pending-product-capture | Should-match `hk2s`; capture product row before classifying. |
| reverse-lookup/comment case `nei` | pending manual capture | Candidate rows include Jyutping and dictionary text, e.g. top row `nei5 你 you (singular)` | pending-product-capture | Reverse/Cangjie side lookup remains current-browser-surface N/A for `jyut6ping3_mobile`. |
| multi-page input `nei` | pending manual capture | Page 1: `你`, `呢`, `尼`, `妮`, `彌`; Page 2: `妳`, `您`, `膩`, `你估`, `餌` | pending-product-capture | Capture product paging manually before classifying. |

Real should-match signal count in this snapshot: `4` hard-oracle-backed M21-GAP-02 fixes (`santai`, `sigin`, `nri` correction off, `nri` correction on); live-product capture remains pending.

Reason: the deployed product column is pending manual capture. Hard-oracle fixes are classified against pinned TypeDuck v1.1.2 fixtures rather than the moving live site.
