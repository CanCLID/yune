# TypeDuck-Windows Backend Requirements (engine contract)

> **Purpose.** This records what the **TypeDuck-Windows** native IME frontend needs from the
> engine, so Yune development can target it deliberately. It complements
> [`typeduck-web-integration-findings.md`](./plans/archive/typeduck-web-integration-findings.md), which covers the
> *web* frontend. As of 2026-06-17 the TypeDuck-Windows modernization is **intentionally parked**
> pending Yune reaching the contract below; the engine is on the critical path, the Windows
> frontend is the downstream consumer.
>
> **Source of truth.** The detailed analysis lives in the `TypeDuck-Windows` repo:
> `LIBRIME_INTEGRATION_PLAN.md` (the irreducible engine surface) and `INTEGRATION_PLAN.md`
> (the frontend modernization). This file is the engine-side summary of the same contract.

## Architecture (why this is clean)

TypeDuck is RIME-shaped: `weasel frontend  ↔  RIME C ABI  ↔  engine`.

- **Today:** `TypeDuck-Windows (weasel fork) → RIME C ABI → librime fork (TypeDuck-HK/librime @ v1.1.2)`.
- **Target:** `TypeDuck-Windows → RIME C ABI → Yune`.

Because the frontend only ever talks to the **RIME C ABI**, swapping librime→Yune is a *contained*
change **iff** Yune presents the same ABI surface and emits the same candidate data. The four
requirements below are exactly that contract. (Yune's `yune-rime-api` crate is the right home for
items 1–2.)

## The graduation contract — what Yune must satisfy to back TypeDuck-Windows

### 1. RIME C ABI parity, including the fork-only write API
The Windows deployer consumes a **fork-only** API that stock librime does **not** have:

- `config_list_append_string(RimeConfig*, key, value)` — used at **7 sites** in
  `WeaselDeployer/TypeDuckSettings.cpp` (writes the display-language list and the
  completion/correction/sentence/learning/cangjie patch toggles). Struct-pointer style, via the
  `RimeApi` function table — **not** a flat symbol.
- Siblings `config_list_append_{bool,int,double}` (declared; carry for symmetry).
- Plus the standard session / context / status / commit / config / **levers** / schema-list /
  deployment / key-processing surface any RIME frontend uses.

> Note: upstream rime/librime issue #1081 (`d71168e9`, "indexed list insertion") is a YAML config
> *syntax* feature, **not** a C-API equivalent of `config_list_append_string`. There is no upstream
> substitute — Yune must implement it.

### 2. Candidate **comment** data (the dictionary panel depends on this)
The TypeDuck multi-hint dictionary panel renders the **`RimeCandidate.comment`** string, not a
custom struct. Yune must emit comments **byte-compatible with the librime fork v1.1.2 output**:

- reverse code + original comment shown together;
- multiple reverse-lookup pronunciations joined by `"; "`;
- schema name shown in the prompt.

**Implemented in Yune:** the C ABI transport already had `RimeCandidate.comment`; Yune now also
emits the TypeDuck dictionary-panel payload through `dictionary_lookup_filter`: `\f` followed by
`\r1,` for the candidate's own source row and `\r0,` for alternate pronunciations. Captured
`jyut6ping3_mobile` source rows now assert byte output against the v1.1.2 fixture. Normal reverse
lookup joins currently use `"; "`, but that joiner still needs a dedicated v1.1.2 oracle case.
Schema-name-in-prompt parity is also still blocked on a captured oracle case. The older TypeDuck-Web
adapter mismatch around context-level `comments` and `highlighted_candidate_index` was web-only, is
resolved in the TypeDuck-Web adapter, and does not change the Windows C ABI contract.

### 3. Cantonese / Jyutping engine behaviors carried by the librime fork
These are the genuinely fork-only behaviors (everything else has converged with upstream librime):

- options: `combine_candidates`, `show_full_code`, `enable_sentence` (disable toggle);
- completion + prediction (freq-threshold tuned) and the **`enable_completion`** option — note
  upstream librime renamed this to **`enable_word_completion`**; pick one name and keep the
  TypeDuck schema YAML + the deployer's `DISABLE_COMPLETION_VALUE` patch consistent, or the
  toggle silently no-ops;
- correction (minimal-distance, monosyllabic, `m`-abbreviation penalty);
- reverse-lookup pronunciation formatting; schema-menu hiding (`hide lone schema`, `hide caret`);
- per-entry user-dictionary pronunciations.

A Cantonese/Jyutping **regression suite** should snapshot goldens from the released **v1.1.2** binary
+ pinned schema, then assert parity.

Yune now has `crates/yune-core/tests/cantonese_parity.rs` locking the captured
`jyut6ping3_mobile` menu/comment fixture. Full behavior parity remains unchecked until dedicated
v1.1.2 goldens are captured for the suite's ignored option, completion, correction, schema-menu,
and userdb-pronunciation cases.

### 4. A native (non-WASM) Windows build
The web path is Emscripten/WASM. Windows needs a **native** engine artifact:

- `rime.dll` + `rime.lib` + `dist/include/rime_*.h`, consumable by the weasel MSBuild release path;
- today these ship as `rime-TypeDuck-{x86,x64}` release archives that the Windows CI's
  `github.install.bat` downloads (keyed on the release tag = `git describe`);
- include the deployment / levers / config-compile (`__include`/`__patch`/list-append) APIs the
  deployer drives.

## Status checklist (update as Yune progresses)

- [x] (1) `config_list_append_string` (+ siblings) on the RIME C ABI
- [ ] (2) `RimeCandidate.comment` emitted with full TypeDuck shaping
  - [x] dictionary lookup payload bytes from captured source rows
  - [ ] reverse-lookup joiner and schema-name prompt parity captured against v1.1.2
- [ ] (3) Cantonese behavior parity vs v1.1.2 (regression suite added; full parity still has documented ignored oracle gaps)
- [ ] (4) Native Windows engine artifact (`rime.dll`/`.lib`/headers) + deployment APIs (packaging script added; MSVC-host smoke verification pending)

When all four are met (and real E2E passes), revisit `TypeDuck-Windows/INTEGRATION_PLAN.md`: the
engine swap behind the RIME C ABI is then a contained change, and the (engine-agnostic) frontend
modernization can proceed independently in the meantime.
