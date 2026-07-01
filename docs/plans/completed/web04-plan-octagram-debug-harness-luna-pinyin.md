# WEB-04 Octagram Debug Harness For Luna Pinyin Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **Status:** Complete - Task 0 green after native product-path fix; Tasks 1-5
> complete with browser evidence and closeout docs. - **Track:** Web
> harness startup and memory (`apps/yune-web`). - **Created:** 2026-07-01 -
> **Updated:** 2026-07-01 (WEB-04 completed; browser follow-up evidence passed
> after sourcing the local Emscripten environment for the WASM build; see
> `docs/reports/evidence/web04-octagram-debug-harness/browser-harness-followup-2026-07-01.md`)
> - **Type:** dogfooding/observability slice (data + app plumbing; no engine
> contract change).

**Goal:** Make the M54 native octagram grammar feature **observable and
toggleable in the `yune-web` browser harness for `luna_pinyin`**, so the engine
feature can be dogfooded and debugged, without widening the engine contract,
changing default public behavior, or vendoring third-party model data. Octagram
stays **default-off**; the deliverable is a "Luna Pinyin" vs "Luna Pinyin +
Octagram" schema-profile surface plus diagnostics and real-browser evidence.

**Architecture:** The engine already loads octagram through the ABI: a schema
with `grammar/language: <model>` and a deployed `<model>.gram` is wired into the
translator by `load_schema_octagram_grammar` in
`crates/yune-rime-api/src/schema_install.rs`. This slice is **data + app
plumbing only**. Decided review calls (see below): the toggle is a
**schema-profile switch** ("Luna Pinyin" vs "Luna Pinyin + Octagram"), *not* an
engine runtime flag; octagram stays default-off; and the reusable
`packages/yune-web-runtime` package is not modified - the app adapter's existing
`extraSharedAssets` seam delivers the optional `.gram`. Because grammar loads at
schema/deploy time, the octagram profile must set `grammar/language` **inline in
a dedicated schema** (`luna_pinyin_octagram.schema.yaml`); it must **not** add a
shared `grammar.yaml`/`hant` node, because plain `luna_pinyin` already imports
`- grammar:/hant?` and would otherwise start loading octagram too (default-off
would silently break).

Before any browser work, Phase 0 proves the **deployed native product path**
already ranks octagram candidates like librime. M54 proved core `.gram`
parsing/scoring against the real pinned `lotem/rime-octagram-data` and
`amzxyz/RIME-LMDG` models (a documented one-time `phase-3-yune-core-verification`
run) plus a committed executable synthetic edge oracle - but it did **not** prove
the deployed ABI/product path or the browser harness path.

---

## Decided Review Calls

Locked in from review (do not re-litigate without new evidence):

- Toggle = **schema-profile switch**, not an engine runtime live-gate. The
  runtime gate (translator holds the grammar, gated by a `yune_web_set_option`
  bool) is an engine-lane change and is **explicitly deferred** - out of scope
  for WEB-04.
- Octagram is **default-off**; plain `luna_pinyin` behavior is unchanged.
- No `packages/yune-web-runtime` change; use the app adapter `extraSharedAssets`
  seam.
- `RIME-LMDG` is **not** used for browser assets here (kept as engine
  validation/oracle research data only).

## Open Review Questions

- **Model + delivery.** Use one pinned `lotem/rime-octagram-data` `.gram` (e.g.
  `zh-hant-t-essay-bgw.gram`, `~10 MB`) referenced by URL + checksum for
  dev/debug, fetched into a gitignored local asset - not vendored. Confirm
  dev-only. A bundled public asset is a separate slice needing license/NOTICE
  handling (lotem is LGPL-3.0).
- **UX goal.** Keep this slice **grammar-only** (`contextual_suggestions` off),
  matching M54, as a correctness/observability demo? The "smarter typing" UX
  (homophone/homograph reranking) needs further deferred engine work.
- **Memory budget.** WASM has no mmap, so the `.gram` is copied into linear
  memory. Is a `~10 MB` model acceptable against the WEB-03 `luna_pinyin`
  `64.0 MiB` browser fair-lane number for a default-off debug profile, and should
  the asset load only when the octagram profile is selected (lazy)?

## Current Starting Point

Verified repo facts (2026-07-01):

- Engine loads octagram via the ABI: `schema_octagram_language` reads
  `grammar/language`; `load_schema_octagram_grammar` resolves `{language}.gram`
  through `validate_data_resource_id` (logical IDs, no path traversal) and calls
  `translator.with_upstream_sentence_grammar(...)`
  (`crates/yune-rime-api/src/schema_install.rs`).
- **Plain `luna_pinyin` already imports the optional grammar hook**
  `- grammar:/hant?` (`apps/yune-web/public/schema/luna_pinyin.schema.yaml:136-138`);
  there is currently no `grammar.yaml`/`grammar/language` and no `.gram`, so it
  is a no-op today. A shared `grammar.yaml`/`hant` node would activate it for
  plain Luna - the default-off trap this plan must avoid.
- The browser Luna shared-asset list has no `.gram`
  (`apps/yune-web/src/worker.ts:401-405`).
- The app adapter already accepts and writes extra shared assets into the shared
  data dir via `extraSharedAssets` / `writeExtraSharedAsset`
  (`apps/yune-web/src/yune-integration/adapter.ts`).
- M54 verification: core parsing/scoring proven against the real pinned lotem +
  RIME-LMDG models (one-time `docs/reports/evidence/m54-native-octagram-grammar-support/phase-3-yune-core-verification.md`)
  plus a committed executable synthetic MIT `.gram` oracle test; the deployed
  ABI/product path and browser path are unproven.
- The native CLI frontend accepts `--shared-data-dir --user-data-dir --schema
  --sequence` (`crates/yune-cli/src/args.rs`), so it can deploy and drive a
  grammar-enabled `luna_pinyin` schema directly.

## Scope

In scope:

- **Phase 0 native validation** of the deployed product path (prerequisite gate).
- A **dedicated `luna_pinyin_octagram` schema profile** that sets
  `grammar/language` inline and is default-off, leaving plain `luna_pinyin`
  untouched.
- **Optional `.gram` delivery via the app adapter `extraSharedAssets` seam**,
  fetched by URL + checksum into a gitignored local asset, only when the octagram
  profile is selected.
- A **UI toggle or schema-selector entry** ("Luna Pinyin" vs "Luna Pinyin +
  Octagram") and **diagnostics** (grammar model delivered yes/no, id + checksum,
  measured schema-select high-water memory delta).
- **Playwright evidence**: inputs where octagram changes ranking, a negative
  control proving toggle-off preserves current Luna behavior, and the browser
  memory delta.
- A `.gitignore` rule so `.gram` model data cannot be accidentally committed, and
  closeout docs.

Out of scope:

- Any engine change, including the runtime live-gate (deferred), and any
  `packages/yune-web-runtime` change unless the adapter seam proves insufficient.
- A shared `grammar.yaml`/`hant` node that plain `luna_pinyin` can consume.
- The librime C++ plugin ABI, `contextual_translation`/`contextual_suggestions`,
  and other deferred plugin gears.
- Bundling `rime-octagram-data` or `RIME-LMDG` as a shipped public asset; any
  vendoring of third-party model bytes.
- Making octagram the default public `luna_pinyin` behavior.
- The M55 native Track A memory research; native performance claims.

## Files And Responsibilities

- Create: `docs/reports/evidence/web04-octagram-debug-harness/`
  - Phase 0 native validation, model pin (URL + checksum), browser memory delta,
    Playwright evidence, final gates.
- Create: `apps/yune-web/public/schema/luna_pinyin_octagram.schema.yaml`
  - Dedicated octagram profile with inline `grammar/language`; plain
    `luna_pinyin.schema.yaml` stays unchanged.
- Create: a fetch script (e.g. `scripts/fetch-octagram-dev-model.*`) + a
  `.gitignore` rule for the dev `.gram`.
- Modify: `apps/yune-web/src/worker.ts`
  - Register the octagram profile and its conditional/lazy `.gram` asset.
- Modify: `apps/yune-web/src/yune-integration/adapter.ts` (and integration UI)
  - Pass the `.gram` via `extraSharedAssets` for the octagram profile; add the
    profile switch and the delivered/missing-grammar diagnostic surface.
- Do **not** modify `packages/yune-web-runtime/` or any `crates/` engine file
  (unless review later adopts the deferred runtime gate).
- Modify on closeout only: `docs/roadmap.md`, `docs/requirements.md`,
  `docs/ledgers/milestone-history.md`, and this plan moved to
  `docs/plans/completed/`.

## Task 0: Native Product-Path Validation (prerequisite gate)

**Status:** Green after native follow-up fix (2026-07-01). The dedicated
`luna_pinyin_octagram` native product path now matches the fresh librime +
octagram oracle top candidates for the WEB-04 rows, while plain `luna_pinyin`
remains stable/null-grammar. See
`docs/reports/evidence/web04-octagram-debug-harness/phase-0-native/followup-native-fix-2026-07-01.md`.

- [x] Assemble a native `luna_pinyin` shared-data dir with a dedicated octagram
  profile (`grammar/language` inline) and a pinned `.gram` (URL + checksum; not
  committed). *(Completed by the 2026-07-01 follow-up evidence.)*
- [x] Drive it with `cargo run -p yune-cli -- frontend --shared-data-dir <dir>
  --user-data-dir <tmp> --schema luna_pinyin_octagram --sequence <inputs>` for
  inputs where octagram is expected to change ranking. *(Completed by the
  2026-07-01 follow-up evidence.)*
- [x] Confirm the **deployed ABI path** produces octagram-ranked candidates that
  match a same-run librime + octagram oracle capture (non-circular), and that a
  plain-`luna_pinyin` control still matches the M54 null-grammar behavior.
  *(Completed by the 2026-07-01 follow-up evidence; accepted top-candidate gate.)*
- [x] Record the pin, commands, and oracle comparison under
  `docs/reports/evidence/web04-octagram-debug-harness/phase-0-native/`.
  *(Completed: `followup-native-fix-2026-07-01.md` / `.json`.)*

**No-go:** Stop if the deployed native path does **not** reproduce librime's
octagram ranking - that is an engine defect for a separate native milestone, not
a harness slice.

## Task 1: Dev Model Fetch + Ignore

**Status:** Green; model fetch implemented and browser evidence passed.

- [x] Add a script that downloads the pinned `.gram` by URL and verifies its
  checksum into a gitignored local asset path (no browser-runtime fetch of the
  model; no committed bytes).
- [x] Add a `.gitignore` rule covering `.gram` under the harness so LGPL model
  data cannot be accidentally committed.

## Task 2: Dedicated Octagram Schema Profile

**Status:** Green; browser deploy proof passed.

- [x] Create `luna_pinyin_octagram.schema.yaml` setting `grammar/language`
  **inline** for the pinned model. Do **not** add a shared `grammar.yaml`/`hant`
  node (plain `luna_pinyin` imports `grammar:/hant?` and would consume it).
- [x] Confirm plain `luna_pinyin` still deploys with **no grammar model delivered**,
  and the octagram profile deploys **with** it, through the existing schema-switch
  path. *(Completed by the 2026-07-01 browser follow-up evidence.)*

## Task 3: Adapter Asset Delivery, Toggle, Diagnostics

**Status:** Green; adapter delivery, toggle, diagnostics, and browser evidence
passed.

- [x] Deliver the dev `.gram` for the octagram profile through the existing
  adapter `extraSharedAssets` seam, lazily (only when the profile is selected).
  If the model is missing or the checksum mismatches, fall back to plain Luna and
  surface a "grammar model not delivered" diagnostic - never error. **This graceful
  fallback is dev-UX only**; the verification gate and Playwright evidence for the
  octagram profile must fail-closed on a missing/bad model (Task 4), not pass on
  the fallback.
- [x] Expose the profile switch as a **default-off** UI toggle or schema-selector
  entry in `yune-integration`.
- [x] Add diagnostics: grammar model delivered yes/no, model id + checksum, and
  the measured schema-select high-water browser-memory delta with vs without the
  model.

## Task 4: Browser Evidence

**Status:** Green. The local WASM build blocker was resolved by sourcing the
local Emscripten environment before running `scripts/yune-web-wasm-build.sh`;
the WEB-04 Playwright gate now passes. See
`browser-harness-followup-2026-07-01.md`.

- [x] **Fail-closed gate:** the octagram evidence must first assert the
  diagnostic shows the grammar model was delivered (id + checksum match), then
  prove all named rows changed versus plain Luna. A missing or checksum-bad
  model **fails** the octagram gate; it must not silently serve plain Luna and
  pass. Only the negative control below expects plain-Luna output.
- [x] Capture Playwright evidence for the octagram profile: inputs where the top
  candidates/ordering differ from plain Luna (the observable win), with
  before/after candidate lists recorded.
- [x] Capture a negative control: with the plain profile selected, Luna candidate
  behavior is unchanged from the current harness.
- [x] Record the browser memory high-water with the model delivered vs not,
  framed against the WEB-03 `64.0 MiB` Luna fair-lane number as
  schema-select/redeploy context - measured evidence, not a model-only
  performance claim.
- [x] Keep all evidence under
  `docs/reports/evidence/web04-octagram-debug-harness/`, browser lane separated
  from native/engine claims.

## Task 5: Closeout

**Status:** Complete.

- [x] Update roadmap (Snapshot/Sequence/Ledger), requirements (WEB-04 IDs +
  coverage), and milestone-history; archive this plan to `completed/`.
- [x] Keep the support contract's octagram row scoped to the named engine target;
  this slice adds a harness dogfooding surface, not a broadened engine support
  claim.

## Definition Of Done

WEB-04 closes as complete when:

- The deployed native path is proven to match librime octagram ranking (Task 0).
- The harness offers a default-off `luna_pinyin_octagram` profile that delivers a
  pinned, non-vendored `.gram`; **plain `luna_pinyin` loads no grammar model and
  is byte-for-byte unchanged**.
- Diagnostics show grammar-delivery state, model id/checksum, and
  schema-select high-water memory delta.
- Playwright evidence shows octagram changing ranking on named inputs plus the
  plain-profile negative control, and **fails closed** if the grammar model did
  not deliver (a fallback-to-plain-Luna cannot produce green octagram evidence).
- **No engine contract change, no `packages/yune-web-runtime` change, and no
  `.gram` bytes committed.**

WEB-04 closes as partial/no-go if:

- The deployed native path does not reproduce librime octagram ranking (engine
  defect - hand off to a native milestone).
- A pinned `.gram` cannot be delivered under an acceptable license/size posture.
- The browser memory cost is unacceptable even for a default-off debug profile.

## Proposed Requirement IDs

Add to `docs/requirements.md` only on closeout:

- **WEB-04-01**: The deployed native `luna_pinyin` + octagram path reproduces a
  same-run librime + octagram candidate-ranking oracle (product path, not just
  the M54 engine tests).
- **WEB-04-02**: The harness exposes a default-off dedicated `luna_pinyin_octagram`
  profile with inline `grammar/language`; plain `luna_pinyin` loads no grammar
  model and stays unchanged.
- **WEB-04-03**: The dev `.gram` is delivered via the app adapter
  `extraSharedAssets` seam, fetched by URL/checksum into a gitignored asset; no
  model bytes are committed and `packages/yune-web-runtime` is unchanged.
- **WEB-04-04**: Diagnostics report grammar-model delivery state, id/checksum,
  and the browser schema-select high-water memory delta.
- **WEB-04-05**: Playwright evidence shows octagram changing ranking plus a
  plain-profile negative control; browser claims stay in the browser lane and no
  engine support boundary is widened.

## Review Prompt

Suggested prompt for review:

> Please review `docs/plans/completed/web04-plan-octagram-debug-harness-luna-pinyin.md`
> as a completed WEB-04 closeout. Focus on: whether the dedicated `luna_pinyin_octagram`
> profile (inline `grammar/language`, no shared `grammar.yaml`/`hant`) actually
> keeps plain Luna default-off; whether the native Phase 0 gate is the right
> prerequisite; the `.gram` model/license/size + memory posture (default-off,
> lazy, non-vendored, gitignored, WASM has no mmap); and whether it preserves M51
> ABI, M53 claim discipline, `packages/yune-web-runtime`, current null-grammar
> Luna, and TypeDuck behavior.
