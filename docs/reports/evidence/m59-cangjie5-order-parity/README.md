# D-48 item 2 — cangjie5 order-parity onboarding (2026-07-09)

Executes D-48 item 2 (`docs/decisions.md`): onboard `cangjie5` as an order-parity
lane by capturing upstream librime 1.17.0 cangjie5 over pinned `rime/rime-cangjie`
for the three owner composition rows + controls, answering the decisive question
**does librime's cangjie5 compose the multi-character phrases at all?**, and wiring a
non-circular real-path test. No engine changes; the Yune divergence is filed as a
finding, not fixed inline.

## Deliverables

- `scripts/capture-upstream-cangjie5.ps1` — pinned-provenance, all-pages oracle
  capture; calls `scripts/curate-upstream-cangjie5.py` to embed the upstream source
  slice. Byte-content-verifiable regeneration.
- `crates/yune-core/tests/fixtures/upstream-1.17.0/cangjie5-composition.json` — the
  committed oracle capture + source slice.
- `crates/yune-core/tests/upstream_cangjie5_composition_parity.rs` — the non-circular
  real-path test (fixture lock + char-by-char composition + named blocked full-code
  test).

## Provenance (fable reviews this)

- **Oracle:** rime/librime **1.17.0** `33e78140…`, `rime.dll` sha256 `86b4c735…`,
  `rime_deployer.exe` sha256 `3abb72b5…` (pinned + re-verified by the script).
- **Dict:** `rime/rime-cangjie` `52d90a1b…` (`sort: by_weight`,
  `use_preset_vocabulary: essay`, encoder, `max_phrase_length: 7`,
  `enable_sentence: true`, `enable_encoder: true`). Dependency commits recorded in the
  fixture header.
- **Same-dict note (finding CJ-2):** the SHIPPED Yune product `cangjie5` is a
  *different* dict — Jackchows "Cangjie5 補完計劃" (`sort: original`,
  `max_phrase_length: 1`), matching how Lane A validates upstream rime-cantonese while
  the product ships TypeDuck. This lane validates **upstream `rime/rime-cangjie`**
  (matching the existing `cangjie5-basic.json` `schema_data`), not the product dict.
  The product↔upstream split is recorded for owner review.

## Decisive question — ANSWERED: **YES, librime composes all three (at candidate 0).**

Full concatenated shape-code → composed phrase at **position 0**, in librime's ` ☯ `
(sentence) slot:

| input | = char codes | oracle candidate 0 | pos |
|---|---|---|---|
| `hwmvsqtt` | 粵`hwmvs` + 拼`qtt` | **粵拼** | 0 |
| `ebcnyripm` | 測`ebcn` + 試`yripm` | **測試** | 0 |
| `takohaeosk` | 莫`tak` + 伯`oha` + 洢`eosk` | **莫伯洢** | 0 |
| `hdaetcu` (control) | 香`hda` + 港`etcu` | **香港** | 0 |
| `lyk` (control) | 中`l` + 文`yk` | 奜 (single), **中文** @1 | 1 |

Per D-48: librime composes them → **oracle-backed order rows** (phrase pinned at its
captured position). The `lyk` control (中文 at pos 1, behind single-char 奜) proves the
ordering is not a trivial "phrase always first" rule.

## Yune real-path result — FINDING CJ-1 (owner review; not fixed inline)

Yune's real production path (`yune-cli frontend --schema cangjie5`) over the **same**
upstream rime-cangjie deploy:

- **Full-code one-shot composition DIVERGES.** For every ☯-sentence row Yune fills the
  sentence slot with the **raw code rendered as root glyphs**, not the composed phrase:
  - `hwmvsqtt` → `竹田一女尸手廿廿`(☯) · 粵 · 粤 — **粵拼 absent** (is_last_page).
  - `ebcnyripm` → `水月金弓卜口戈心一`(☯) · 測 — **測試 absent**.
  - `takohaeosk` → `廿日大人竹日水人尸大`(☯) · 莫 — **莫伯洢 absent**.
  - `hdaetcu` → `竹木日水廿金山`(☯) · 香 — **香港 absent**.
  - (`lyk` → 奜@0, **中文@1** — MATCHES; `中文` is a pre-encoded phrase entry, not a
    runtime segmentation, so it survives.)
- **Char-by-char composition WORKS.** All seven single constituent codes produce the
  oracle's candidate-0 character at position 0 (`hwmvs`→粵, `qtt`→拼, `ebcn`→測,
  `yripm`→試, `tak`→莫, `oha`→伯, `eosk`→洢). Verified via the real deploy path AND
  asserted in the committed test.

**Interpretation:** Yune has encoder/exact phrase entries (中文) but lacks runtime
**sentence *segmentation*** of a concatenated full code into constituent characters —
the already-named M17-blocked area
(`upstream_cangjie_parity.rs::cangjie5_phrase_encoder_full_page_parity_is_blocked`).
The capture now supplies the missing oracle evidence. **This does NOT violate D-47**
(compose one character at a time) — char-by-char works; only the librime full-code
one-shot *shortcut* is unsupported. **Owner decision:** prioritize the cangjie
segmentation-compose fix (M17) so the full-code path reaches oracle parity, or accept
char-by-char as cangjie's D-47 path and record the full-code shortcut as an
owner-signed unsupported behavior.

## Test wiring (non-circular, per D-48 + CLAUDE.md "no silent gaps")

`cargo test -p yune-core --test upstream_cangjie5_composition_parity` → 2 passed, 1
ignored:

1. `upstream_cangjie5_composition_fixture_is_locked` — locks oracle commit, dict
   commit, and that librime composes each owner phrase @0. Pure oracle lock.
2. `yune_cangjie5_composes_each_constituent_char_at_top` — **real translator path**
   over the upstream source slice; asserts each constituent code's top candidate ==
   the oracle's candidate 0. Guards M59-REACH-02 / D-47 for cangjie. Expected values
   are the oracle's, never Yune-derived (dict rows are byte-exact upstream rime-cangjie
   source; the oracle cases additionally carry essay phrases the slice cannot produce —
   positive proof the expected column is real librime, not round-tripped Yune).
   **Scope (honest):** this is a composition-**reachability** guard (the constituent
   char is produced, not dropped/replaced by a code glyph — the CJ-1 failure mode), not
   a weight-ranking discriminator: for these seven codes the oracle order coincides with
   dictionary insertion order, so by_weight ranking is exercised by the M19
   `upstream_cangjie_parity` lane, not here.
3. `cangjie5_full_code_sentence_composition_is_blocked` —
   `#[ignore = "blocked: … finding CJ-1"]` with a `panic!()` body. Names the gap; no
   silent hole.

The lock test pins the oracle **dll/deployer sha256** (not just commit strings), so
non-owner candidate rows cannot drift under a different librime build.

### Adversarial verification (2026-07-09)

A three-lens adversarial pass (provenance / non-circularity / divergence-accuracy)
was run before this report. Provenance and the decisive answer HELD at high
confidence; the divergence characterization HELD at high confidence (a verifier
independently proved the Yune runs were over genuine upstream rime-cangjie — the qtt
encoder phrases are impossible under the product's `max_phrase_length: 1`). The
non-circularity lens confirmed the expected values are oracle-derived and the dict is
byte-exact upstream, but flagged the char-by-char test as a thin ranking guard (above)
and a missing dll-sha lock — both addressed here (comment reframed, sha lock added).

## Reproduce

```
pwsh -File scripts/capture-upstream-cangjie5.ps1        # deploy rime-cangjie, capture, curate
cargo test -p yune-core --test upstream_cangjie5_composition_parity
```
