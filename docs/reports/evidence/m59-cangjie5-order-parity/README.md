# D-48 item 2 — cangjie5 order-parity onboarding (2026-07-09)

> **Current Increment 4d resolution (2026-07-12):** CJ-1 and
> M59-PARITY-03 are complete at clean source `38e759f6`. The explicitly marked
> upstream-Cangjie validation lane is strict `12/12` through every captured
> page, including `tak` exact `30/30`; the owning parity suite is `3 passed / 0
> ignored`. A separately captured unmarked control remains exact `12/12`
> against `fd6bd2a7`. The implementation is configuration/profile-derived and
> uses no schema-id/input/oracle gate or new exception. Final evidence:
> [`../m59-canonical-jyutping-reachability-parity/increment-4d-cangjie-cj1/`](../m59-canonical-jyutping-reachability-parity/increment-4d-cangjie-cj1/).
> The Increment 1 material below is retained as the pre-fix diagnosis.

Executes D-48 item 2 (`docs/decisions.md`): onboard `cangjie5` as an order-parity
lane by capturing upstream librime 1.17.0 cangjie5 over pinned `rime/rime-cangjie`
for the three owner composition rows + controls, answering the decisive question
**does librime's cangjie5 compose the multi-character phrases at all?**, and wiring a
non-circular real-path test. No engine changes; the Yune divergence is filed as a
finding, not fixed inline.

> **Current Increment 1 update (2026-07-10):**
> [`increment-1-executable-evidence/`](./increment-1-executable-evidence/)
> is the source-clean, seven-file executable pre-fix packet. It preserves the
> untouched raw oracle, validated curated oracle, Yune capture, exact JSON/CSV
> diff, and deterministic manifest. The strict exact result is deliberately red:
> 4 passed / 8 failed, comparator exit `1`, no exceptions. The packet corrects
> the old fixture's mojibaked hand-embedded owner targets without changing any
> captured candidate or page arrays. Its CJ-1 finding is historical and is
> superseded by the Increment 4d resolution above.

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

## Historical Yune real-path result — FINDING CJ-1 (fixed by Increment 4d)

**Corrected 2026-07-09 after fable re-verification — the original "Yune lacks
☯-sentence composition" framing was FALSIFIED.** The gap is a lane-specific
segmentation SCORING divergence, not a missing capability.

Yune's real production path (`yune-cli frontend --schema cangjie5`):

- **Upstream rime-cangjie lane picks a mis-scored segmentation.** For every full-code
  row Yune's ☯ sentence is the **eight single-letter roots**, not the composed phrase:
  - `hwmvsqtt` → `竹田一女尸手廿廿`(☯ = h|w|m|v|s|q|t|t) · 粵 · 粤 — **粵拼 absent** (is_last_page).
  - `ebcnyripm` → `水月金弓卜口戈心一`(☯) · 測 — **測試 absent**.
  - `takohaeosk` → `廿日大人竹日水人尸大`(☯) · 莫 — **莫伯洢 absent**.
  - `hdaetcu` → `竹木日水廿金山`(☯) · 香 — **香港 absent**.
- **The PRODUCT cangjie5 lane composes correctly** (fable re-verified; re-confirmed here
  via `yune-cli --shared-data-dir apps/yune-web/public/schema`): `hwmvsqtt`→**粵拼@0**
  (n=3: 粵拼 · 粵 · 粤), `ebcnyripm`→**測試@0**. The product dict is Jackchows with
  `max_phrase_length: 1`, so 粵拼/測試 **cannot be dict entries** — they are
  **runtime-composed by segmentation**. Therefore Yune's engine CAN do runtime ☯-sentence
  composition; the upstream lane simply scores the 8-single-root segmentation above
  `hwmvs|qtt`→粵拼.
- **Char-by-char composition works on both lanes.** All seven single constituent codes
  produce the oracle's candidate-0 character at position 0 (`hwmvs`→粵, `qtt`→拼,
  `ebcn`→測, `yripm`→試, `tak`→莫, `oha`→伯, `eosk`→洢). So **D-47 is not violated.**

**Historical interpretation:** this was a **segmentation-scoring divergence specific to the upstream
`rime/rime-cangjie` lane** (`sort: by_weight` + `essay` preset vocabulary), where the
common root characters (竹 田 一 女 尸 手 廿 — each a high-frequency char) score the
8-single-root path above the 2-character 粵拼. The product lane, with different weights,
segments correctly. **Owner decision — diagnose+fix the upstream-lane segmentation
scoring divergence.** Increment 4d resolved it through the explicit
upstream-table sentence policy, compiled-prism predictive traversal,
reverse-syllabification graph, and pinned stream-head merge. The original
hypotheses were: (1) the weight model — upstream
`by_weight`+essay hands root chars huge weights, so a segmentation summing eight of them
out-scores 粵拼; (2) whether the sentence scorer on this lane sums **raw frequencies**
rather than log-probs (the M48 luna raw-frequency class of bug —
`memory/luna-pinyin-sentence-raw-frequency-scoring.md`). The ignored test
`cangjie5_upstream_lane_segmentation_scoring_is_blocked` kept this real
upstream-lane gap named until Increment 4d removed the ignore and made the
owning suite 3 passed / 0 ignored.

## Historical pre-fix test wiring (non-circular, per D-48)

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
   **Scope (honest):** this is a composition-**reachability** guard (each constituent
   char is produced for its shape code — the char-by-char path both lanes support), not
   a weight-ranking discriminator: for these seven codes the oracle order coincides with
   dictionary insertion order, so by_weight ranking is exercised by the M19
   `upstream_cangjie_parity` lane, not here.
3. `cangjie5_upstream_lane_segmentation_scoring_is_blocked` —
   `#[ignore = "blocked: … finding CJ-1"]` with a `panic!()` body. Names the real
   upstream-lane segmentation-scoring gap; no silent hole.

The lock test pins the oracle **dll/deployer sha256** (not just commit strings), so
non-owner candidate rows cannot drift under a different librime build.

### Adversarial verification (2026-07-09)

A three-lens adversarial pass (provenance / non-circularity / divergence-accuracy) ran
before the first commit. Provenance and the decisive oracle answer HELD at high
confidence; the non-circularity lens confirmed expected values are oracle-derived and
the dict is byte-exact upstream, and flagged the char-by-char test as a thin ranking
guard + a missing dll-sha lock — both fixed.

**However, that pass confirmed the OBSERVATION (upstream lane emits the 8-root ☯
segmentation, phrase absent) but accepted the wrong INTERPRETATION ("Yune lacks
☯-sentence composition") because it never ran the PRODUCT lane.** fable's review did:
the product cangjie5 composes `hwmvsqtt`→粵拼@0 despite `max_phrase_length: 1`, proving
the capability exists and the gap is upstream-lane segmentation *scoring*. Re-confirmed
here. The CJ-1 framing above is the corrected finding. Lesson: a divergence's
*interpretation* needs a differential control (here, the other lane), not just the
observation.

## Reproduce

```powershell
# The packet README carries the exact create-new capture and comparator commands.
cargo test -p yune-core --test upstream_cangjie5_composition_parity
```
