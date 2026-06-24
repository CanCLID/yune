# Settings Profile Snapshot

Captured before comparing outputs. Product settings are pending manual capture, so ranking differences are not classified as bugs in this snapshot.

| Setting | Yune harness (M20 control) | Deployed product |
|---|---|---|
| completion (`enable_completion`) | On | pending manual capture |
| correction (`enable_correction`) | Off for baseline corpus; On only for `nri` correction-on probe | pending manual capture |
| auto-composition (`enable_sentence`) | On | pending manual capture |
| input memory (`enable_user_dict`) | On; fresh Playwright context before corpus | pending manual capture |
| combine vs separate (`combine_candidates`) | Combine on | pending manual capture |
| prediction never-first | On | pending manual capture |
| prediction threshold | `0` | pending manual capture |
| page size | `6` in the UI range control; Yune rows record the rendered candidate count for each case | pending manual capture |
| simplification (`hk2s`) | Off for baseline corpus; On only for `hk2s-ngohaigo-simplification-on` | pending manual capture |
| full-shape / ASCII mode | Full shape off; ASCII mode off | pending manual capture |
| userdb state (fresh / accumulated) | Fresh Playwright browser context; no learned phrase sequence before corpus capture | pending manual capture |
| engine + dict version | Yune `cdb7bd52e638647493e7b097a4deecb38d9efb04`, `jyut6ping3_mobile` checked-in assets | pending manual capture; may be newer than TypeDuck `v1.1.2` |

Confounder decision: because the deployed product settings and version are not pinned yet, this snapshot records Yune harness output and product-capture gaps only. It does not report ranking differences as bugs.
