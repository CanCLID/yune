# TypeDuck-Windows Backend Requirements (engine contract)

> **Purpose.** This records what the **TypeDuck-Windows** native IME frontend needs from the
> engine, so Yune development can target it deliberately. It complements
> [`m09-findings-typeduck-web-integration.md`](./plans/archive/m09-findings-typeduck-web-integration.md), which covers the
> *web* frontend. This M10 contract is parked as a TypeDuck compatibility
> profile after M12 made upstream `rime/librime 1.17.0` the default core oracle.
> The Windows frontend remains the downstream consumer when this profile is
> explicitly resumed behind the named TypeDuck profile ABI surface added in M19.
>
> **Source of truth.** The local execution notes are
> [`plans/m10-reference-typeduck-windows-contract.md`](./plans/m10-reference-typeduck-windows-contract.md)
> and [`plans/m10-reference-typeduck-windows-native-build.md`](./plans/m10-reference-typeduck-windows-native-build.md). Earlier
> downstream analysis referenced `LIBRIME_INTEGRATION_PLAN.md` and `INTEGRATION_PLAN.md`
> in the external `TypeDuck-Windows` repo (<https://github.com/TypeDuck-HK/TypeDuck-Windows>),
> but the 2026-06-19 pinned checkout at
> `f3ffcfe3b6a3018b1c3c9d256a6f0d587a2d2e27` did not contain those files. Treat the
> external fork as reference-only until a pinned E2E harness or current equivalent docs are
> identified; this file is the engine-side contract summary.
>
> **Oracle URLs.** Core Yune behavior is checked against upstream librime
> (<https://github.com/rime/librime>, target `1.17.0`, commit
> `33e78140250125871856cdc5b42ddc6a5fcd3cd4`). This TypeDuck profile is checked
> against the TypeDuck fork (<https://github.com/TypeDuck-HK/librime>, tag
> `v1.1.2`, commit `74cb52b78fb2411137a7643f6c8bc6517acfde69`). The committed
> v1.1.2 fixture also records
> `TypeDuck-HK/schema` (<https://github.com/TypeDuck-HK/schema>, commit
> `1bed1ae6a0ab48055f073774d7dfd152a171c548`) and
> `TypeDuck-HK/rime-dictionary-lookup-filter`
> (<https://github.com/TypeDuck-HK/rime-dictionary-lookup-filter>, commit
> `3e4605c4fae99f068df2edb85aaeab5a97752795`).

## Architecture (why this is clean)

TypeDuck is RIME-shaped: `weasel frontend  ↔  RIME C ABI  ↔  engine`.

- **Today:** `TypeDuck-Windows` weasel fork (<https://github.com/TypeDuck-HK/TypeDuck-Windows>) -> RIME C ABI -> librime fork (`TypeDuck-HK/librime @ v1.1.2`).
- **Future target:** `TypeDuck-Windows -> named TypeDuck profile RIME C ABI -> Yune`.

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
`jyut6ping3_mobile` source rows now assert byte output against the v1.1.2 fixture. HR-6 also locks
normal reverse-lookup joins to `"; "` and captures schema-name prompt/preedit bytes against the
same v1.1.2 oracle. The older TypeDuck-Web adapter mismatch around context-level `comments` and
`highlighted_candidate_index` was web-only, is resolved in the TypeDuck-Web adapter, and does not
change the Windows C ABI contract.

### 3. Cantonese / Jyutping engine behaviors carried by the librime fork
These are the genuinely fork-only behaviors and should not be treated as core
Yune behavior unless upstream `rime/librime 1.17.0` also matches them:

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

- [x] (1) `config_list_append_string` (+ siblings) exposed through the named,
  opt-in `rime_get_typeduck_profile_api()` surface; still not exposed by default
  upstream `rime_get_api()` (see
  [`plans/m19-reference-typeduck-profile-abi.md`](./plans/m19-reference-typeduck-profile-abi.md))
- [x] (2) `RimeCandidate.comment` emitted with current TypeDuck shaping
  - [x] dictionary lookup payload bytes from captured source rows
  - [x] reverse-lookup joiner and schema-name prompt parity captured against v1.1.2
- [ ] (3) Cantonese behavior parity vs v1.1.2 (regression suite added; full parity still has documented ignored oracle gaps)
- [x] (4) Native Windows engine artifact (`rime.dll`/`.lib`/headers) archived pre-M12 package smoke; current packaging still needs a fresh TypeDuck-profile package/header smoke against the M19 accessor before it can be resumed

The real TypeDuck-Windows frontend E2E is still **not** green: a pinned checkout
was captured under `target/typeduck-windows-e2e/TypeDuck-Windows`, but the
referenced integration-plan files were absent and the local shell did not expose
`msbuild.exe`, `devenv.exe`, `cmake.exe`, `nuget.exe`, or `nmake.exe`.

When the remaining Cantonese parity gaps and real E2E pass against the M19
profile accessor, revisit the current TypeDuck-Windows frontend lifecycle docs
or harness: the engine swap behind that profile ABI is then a contained change,
and the engine-agnostic frontend modernization can proceed independently in the
meantime.
