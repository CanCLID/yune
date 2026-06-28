# WEB-03 Three-Schema Launch Readiness & Compiled-Asset Contract

> **Status:** Partial asset/native closeout (Tasks 2-4 complete; Task 5 blocked on local Emscripten) — root cause **found + fix proven** — **Track:** Web harness deploy + compiled-asset delivery — **Created:** 2026-06-27 — **Updated:** 2026-06-27 (asset/native evidence added) — **Type:** engine fix + asset regen + guard plan
>
> Follows WEB-02 (`b216ca82`). Launch-ready (project owner) = the public demo offers **all three schemas selectable and working** — upstream `rime-luna-pinyin`, upstream `rime-cangjie` (`cangjie5`), and multilingual `jyut6ping3`. WEB-02 traced the Jyutping `893.1 MiB` to source-fallback from a stale `Rime::Prism/3.0` asset. This plan found *why* the assets are stale and can't be regenerated, and proved a fix.

## Asset/native execution update (2026-06-27)

Tasks 2-4 are complete in the WEB-03 asset slice. Evidence is under
`docs/reports/evidence/web03-three-schema-launch-readiness/`.

- Clean forced regeneration rebuilt the launch-schema compiled assets from
  source and produced `Rime::Prism/4.0` for every launch prism. No row used
  `ReusedPrebuilt`; repeated imports in the same clean workspace may show
  `ReusedFresh` after an earlier schema rebuilt them.
- `apps/yune-web/public/schema`, both schema manifests, the public-demo dist,
  and the worker asset lists now include Cangjie compiled assets and the
  Jyutping Luna reverse helper assets.
- Native storage diagnostics pass for `jyut6ping3_mobile`, `cangjie5`, and
  `luna_pinyin`: `source_fallback=false`, zero fallback rows, and
  `selected_storage=byte_backed` with positive `byte_source_len`.
- The Cangjie minimum correctness smoke is covered by the native guard:
  `cangjie5` input `a` returns U+65E5 as the first candidate.
- During regeneration, Luna correctness exposed one additional deploy gap:
  workspace dictionary rebuild used the dictionary YAML loader for vocabulary
  packs, so `essay.txt` weights were not applied to generated tables. WEB-03
  adds a `.txt` vocabulary loader before regenerating assets, preserving
  `luna_pinyin` input `ni` -> U+4F60 in the byte-backed path.

Task 5 is still blocked in this environment: `wasm32-unknown-emscripten` is
installed, but `emcc`/`emar` are not available on PATH and no `EMSDK` is
configured. The browser high-water memory number is therefore not remeasured
here, and this plan must not claim that the WEB-02 `893.1 MiB` browser result
is fixed until a toolchain-equipped run captures the new Playwright evidence.

## Root cause (CONFIRMED 2026-06-27)

**An engine deploy bug.** Yune's `schema_dictionary_artifact_requests` (`deployment.rs`) treats a `table_translator@custom_phrase` namespace — which has `dictionary: ''` and reads `custom_phrase.txt`, **not** a compiled dictionary — as a dictionary to build. It emits a build request with an empty `dictionary_id`, then `workspace_update_dictionary_artifact` fails on it and **aborts the entire deploy**. librime skips `custom_phrase` (no compiled artifact); Yune does not.

How it was pinned (native CLI deploy + targeted instrumentation; the deploy swallows its error to a generic message):

- The engine builds Jyutping/luna to `Prism/4.0` fine from the **M46 source** (simpler schema, no `custom_phrase`); the web's *feature-rich* schemas fail.
- Ruled out, in order: line endings (LF vs CRLF — both parse), dictionary parsing (all web dicts parse, with imports), the artifact-writer (instrumented — never fired), missing opencc (present).
- Instrumented the deploy branches → failure is `WEB03DBG artifact-build None schema=luna_pinyin dict= prism=` (empty id) from `WEB03DBG invalid-dict-request namespace=custom_phrase raw=""`. The deployed `luna_pinyin.schema.yaml` has `custom_phrase:\n  dictionary: ''`.

**Why this is the whole story:** because the feature-rich web schemas can't deploy/rebuild, the harness ships **pre-built** committed `.bin`s instead — which format-drifted to `Prism/3.0` → rejected → source-fallback → `893.1 MiB`. The product still *works* (the `yune_web` init path reuses pre-built assets and tolerates source-fallback — GPT's point: deploy-with-reuse succeeds; clean/forced **rebuild** is what fails). Related: [[m41-deploy-skip-regression]].

## Fix (validated)

In `schema_dictionary_artifact_requests`, when a namespace's `dictionary` value is **empty**, `continue` (skip) instead of emitting a build request — matching librime. One-line guard + comment.

Verified by re-running the previously-failing clean deploy of the web `public/schema`: it now **succeeds** and emits **`Rime::Prism/4.0`** for **every** schema — `jyut6ping3`, `jyut6ping3_scolar`, **`cangjie5`** (the one with no shipped compiled assets), `cangjie3`, `loengfan`, `luna_pinyin`. So the fix simultaneously unblocks Jyutping regeneration **and** Cangjie compile-from-scratch.

Gates (all green, 2026-06-27): `yune_web` **32/0** (deploy/ABI contract incl. the real-asset deploy test), `cantonese_parity` **37/0**, `upstream_luna_pinyin_parity` **12/0**, `clippy -D warnings` clean. A focused regression test, `empty_dictionary_namespace_yields_no_build_request` in `deployment.rs`, guards it — verified to **fail without the fix** (it produced the empty-id `custom_phrase` request).

## Engine vs harness — settled

**Primarily ENGINE.** The defect is Yune's deploy mishandling the standard librime `custom_phrase` translator. The web schemas are legitimate (`custom_phrase` is standard). Once the engine fix lands, the schemas regenerate cleanly. (The stale committed `.bin`s + hand-maintained workflow are a secondary harness cleanup, addressed by Tasks 2–3 below.)

## Tasks (after the fix is gate-validated)

**Task 1 — Land the engine fix.** Done in the tree: the fix + the `empty_dictionary_namespace_yields_no_build_request` regression test, all gates green. Commit the engine fix + test + this plan **separately** from regenerated assets (GPT condition 3).

**Task 2 — Regenerate + ship compiled assets for all three schemas (native; GPT points 1, 3, 5).**
- Exact tested regen: `cargo run -p yune-cli -- frontend --shared-data-dir apps/yune-web/public/schema --user-data-dir <tmp> --schema <id> --sequence "<keys>"` → collect `<tmp>/build/*.bin`; confirm every prism is `Rime::Prism/4.0`. `jyut6ping3_scolar` is imported by `jyut6ping3` (verify how its prism is emitted, not standalone).
- Update **both** `apps/yune-web/public/schema` **and** the public-demo dist via the Node build: `node apps/yune-web/public-demo/build.mjs` (it copies pinned assets from `public/schema` per `schema-asset-manifest.json` and throws on a missing one). Add `cangjie5` to the manifest **and the worker asset lists** (`worker.ts` currently loads only Cangjie schema/dict, not its `.bin`s); refresh Jyutping hashes; bump the cache bucket if `cache-policy.md` requires (content-addressed SHAs already create new cache keys). Write a closeout/evidence path.

**Task 3 — Regeneration script + byte-backed guard (GPT point 4).** Script to regenerate all web compiled assets from source (no more hand-maintenance). Guard test that fails unless each of the three schemas, via the WEB-02 storage diagnostics, shows `source_fallback=false`, `selected_storage=byte_backed`, `byte_source_len>0`, no fallback rows (behavior, not just a `Prism>=4.0` header check).

**Task 4 — Cangjie correctness.** No **local** Cangjie oracle capture exists yet (the upstream `rime-cangjie` is the oracle). Prefer a small upstream `rime-cangjie` oracle capture as the correctness gate; a deterministic shape-code smoke is the minimum.

**Task 5 — Browser remeasure (needs emcc + Playwright; toolchain side).** Verify all three schemas select, deploy byte-backed, type correctly, switch. Capture the Jyutping high-water drop from `893.1 MiB`; per-keystroke latency in the byte-backed path; verify the M41 deploy-skip init path still byte-backs. **Payoff wording (GPT point 6):** fair to say the fix removes the `529.6 MB` owned-heap owner; not fair to promise a specific new high-water or near-Luna startup until this remeasure runs.

## Boundaries

- **In scope:** the `custom_phrase` deploy fix; compiled-asset regeneration for the three selectable schemas; manifest + worker asset lists + dist; regeneration script + byte-backed guard; Cangjie smoke/oracle; browser remeasure.
- **Out of scope:** widening the default ABI; new `yune_web_*` exports; AI; octagram; schemas beyond the three.
- **Asset hygiene:** regenerated `.bin`s are binary — respect `.gitattributes`. Regeneration reproducible from committed source.

## Division of labor / toolchain (environment-neutral)

Tasks 1–4 are native + ABI-test verifiable without a WASM build. Task 5 needs a local Emscripten WASM build + Playwright; if Emscripten is unavailable in the executing environment, complete Tasks 1–4, commit, and mark the browser remeasure **blocked** for the toolchain-equipped run.
