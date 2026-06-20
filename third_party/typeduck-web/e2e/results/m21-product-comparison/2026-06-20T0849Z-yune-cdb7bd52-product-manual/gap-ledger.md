# Gap Ledger

Product-side rows are pending manual capture. No row below is a Yune bug signal until the deployed product output is manually captured, stamped, and compared against the hard `v1.1.2` oracle path.

| Input | Product output | Yune output | Label | Disposition |
|---|---|---|---|---|
| `nei` | pending manual capture | Top 5: `你`, `呢`, `尼`, `妮`, `彌` | pending-product-capture | Capture product top-N manually before classifying. |
| `ngo` | pending manual capture | Top 5: `我`, `俄`, `柯`, `餓`, `屙` | pending-product-capture | Capture product top-N manually before classifying. |
| `santai` | pending manual capture | Top 5: `身體`, `神體`, `身體健康`, `身體力行`, `身體部分` | pending-product-capture | F08 prediction ranking differences are expected-by-design unless a v1.1.2 golden says otherwise. |
| `sigin` | pending manual capture | Top 5: `事件`, `使勁`, `市建局`, `史景遷`, `事件相關電位` | pending-product-capture | Capture product top-N manually; if core candidate set differs, capture a `v1.1.2` golden before fixing. |
| `m` | pending manual capture | Top 5: `唔`, `無`, `面`, `明`, `民` | pending-product-capture | Should-match standalone-m/fuzzy path, but no bug claim without product capture. |
| `mgoi` | pending manual capture | Top 2: `唔該`, `唔該晒` | pending-product-capture | Should-match fuzzy/容錯 path, but no bug claim without product capture. |
| `ngohaigo` | pending manual capture | Top 1: `我係個` | pending-product-capture | Sentence/composition differences may be pending-M17-M19; classify only after manual product output. |
| `leoicijyu` | live-site observation: `類似於`; capture still pending | Oracle v1.1.2 / Yune M21-GAP-01: top 1 `類似如` | expected-by-design | Version skew: hard `v1.1.2` oracle differs from the moving deployed product, so do not re-investigate this as a regression unless a new pinned oracle fixture changes. |
| `hou` | pending manual capture | Top 5: `好`, `號`, `豪`, `毫`, `浩` | pending-product-capture | Should-match combine/separate behavior needs matched product setting before classification. |
| tone letters `seov` | pending manual capture | Top 1: `瀡板` | pending-product-capture | Should-match `letter_to_tone`; capture product row before classifying. |
| 1-edit typo `nri`, correction off | pending manual capture | No browser candidate panel rendered | pending-product-capture | Yune browser-surface N/A for correction candidate rendering; engine proof remains `cantonese_parity`. |
| 1-edit typo `nri`, correction on | pending manual capture | No browser candidate panel rendered | pending-product-capture | Yune browser-surface N/A for correction candidate rendering; engine proof remains `cantonese_parity`. |
| hk2s case `ngohaigo`, simplification on | pending manual capture | Top 1: `我系个` | pending-product-capture | Should-match `hk2s`; capture product row before classifying. |
| reverse-lookup/comment case `nei` | pending manual capture | Candidate rows include Jyutping and dictionary text, e.g. top row `nei5 你 you (singular)` | pending-product-capture | Reverse/Cangjie side lookup remains current-browser-surface N/A for `jyut6ping3_mobile`. |
| multi-page input `nei` | pending manual capture | Page 1: `你`, `呢`, `尼`, `妮`, `彌`; Page 2: `妳`, `您`, `膩`, `你估`, `餌` | pending-product-capture | Capture product paging manually before classifying. |

Real should-match signal count in this snapshot: `0`.

Reason: the deployed product column is pending manual capture, and Yune's known browser-surface N/A cases are explicitly documented rather than treated as output proof.
