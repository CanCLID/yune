# TypeDuck-Windows Backend Requirements (engine contract)

> **Purpose.** This records what the **TypeDuck-Windows** native IME frontend needs from the
> engine, so Yune development can target it deliberately. It complements
> [`m09-findings-typeduck-web-integration.md`](./plans/archive/m09-findings-typeduck-web-integration.md), which covers the
> *web* frontend. This M10 contract is parked as a TypeDuck compatibility
> profile after M12 made upstream `rime/librime 1.17.0` the default core oracle.
> The M10 resume completed T1/T2/T3 on 2026-06-21: Yune now has a current
> native TypeDuck-profile package/header smoke, packaged DLL lifecycle gate, a
> pinned TypeDuck-Windows x64 build/link against the Yune package, and stock
> TypeDuck-Windows real-server IPC smoke evidence. The package also exposes
> upstream-deprecated direct-call declarations for the existing TypeDuck-Windows
> source without widening default ABI structs or the default `rime_get_api()`
> table. T3 evidence shows stock `TypeDuckServer.exe` loading packaged
> `output\rime.dll` and stock `TestTypeDuckIPC.exe /console` returning a
> nonzero session, sending `ngohaig` key events, and receiving
> `status.schema_id=jyut6ping3` plus candidate/context data.
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

> Current Yune package note: Yune implements `config_list_append_*` through the
> named, opt-in `rime_get_typeduck_profile_api()` table. The default
> upstream-shaped `rime_get_api()` table intentionally does not contain these
> fork-only slots, so TypeDuck-Windows must use the profile accessor when
> linked to a Yune package. The package keeps upstream-shaped
> `RimeCandidate`/`RimeApi` layout while including upstream
> `rime_api_deprecated.h` declarations from packaged `rime_api.h`, because the
> pinned TypeDuck-Windows source calls deprecated direct symbols such as
> `RimeSetup` after including `<rime_api.h>`.

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
  toggle silently no-ops. Yune now distinguishes the schema-default optional
  marker `common:/disable_completion?` from the explicit TypeDuck-Windows
  deployer patch `common:/disable_completion`: the optional marker stays
  inactive, while the explicit patch deploys to
  `translator/enable_completion: false`;
- correction (minimal-distance, monosyllabic, `m`-abbreviation penalty);
- reverse-lookup pronunciation formatting; schema-menu hiding (`hide lone schema`, `hide caret`);
- per-entry user-dictionary pronunciations.

A Cantonese/Jyutping **regression suite** should snapshot goldens from the released **v1.1.2** binary
+ pinned schema, then assert parity.

Yune now has `crates/yune-core/tests/cantonese_parity.rs` locking the captured
`jyut6ping3_mobile` menu/comment fixture plus the M14-M21 captured TypeDuck
engine surfaces. Schema-menu and userdb-pronunciation observations remain
native/frontend evidence topics for T3, not default ABI changes.

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
- [x] (3) Cantonese behavior parity vs v1.1.2 for captured engine fixtures is active in `cantonese_parity`; remaining schema-menu/userdb observations are frontend/T3 evidence limits.
- [x] (4) Native Windows engine artifact (`rime.dll`/`.lib`/headers) current TypeDuck-profile package/header smoke passes through `rime_get_typeduck_profile_api()`.
  - [x] T0 ABI/header decision: package uses upstream-shaped `RimeCandidate` and default `rime_get_api()`, plus `rime_typeduck_profile_api.h`.
  - [x] T2 packaged host-loader lifecycle: packaged `dist/lib/rime.dll` loads, profile append slots round-trip, and the native lifecycle smoke passes.
  - [x] T1 TypeDuck-Windows build/link: Visual Studio 2022 Community MSBuild builds the pinned x64 solution plus the deployer/server projects against the Yune package after local profile-accessor and x64 WinSparkle fixes.
  - [x] T3 real TypeDuck-Windows frontend smoke: stock `TypeDuckServer.exe` starts from `output\`, loads packaged Yune `output\rime.dll`, and stock `TestTypeDuckIPC.exe /console` returns a nonzero session, sends `ngohaig` key events, and receives `status.schema_id=jyut6ping3` plus candidate/context data. Evidence: `target\typeduck-windows-e2e\evidence\m10-t3-20260621-100337-stock-real-server`.

The real TypeDuck-Windows stock server IPC smoke is green: a pinned checkout was
captured under `target/typeduck-windows-e2e/TypeDuck-Windows`. The initial M10
resume attempt could not find `msbuild.exe` on PATH; Visual Studio 2022
Community was later found, Boost 1.84.0 was built at `C:\b184`, ATL/MFC was
installed, and the Yune package was copied into the checkout. The current T1
build command is:

```powershell
& 'C:\Program Files\Microsoft Visual Studio\2022\Community\Msbuild\Current\Bin\MSBuild.exe' target\typeduck-windows-e2e\TypeDuck-Windows\weasel.sln /p:Configuration=Release /p:Platform=x64 /m:1 /v:minimal
```

and now succeeds for `TypeDuckIPC.lib`, `TypeDuckUI.lib`, `typeduckx64.dll`,
and `typeduckx64.ime`. Direct project builds also produce
`output\TypeDuckDeployer.exe` and `output\TypeDuckServer.exe`. The checkout was
locally patched so `WeaselDeployer/TypeDuckSettings.cpp` uses
`rime_get_typeduck_profile_api()` for fork-only `config_list_append_*`, and the
server links the x64 WinSparkle import library. The T3 stock IPC smoke then
starts `TypeDuckServer.exe` from `output\`, points
`HKCU\Software\Rime\TypeDuck\RimeUserDir` at an isolated deployed user data
directory, runs `TestTypeDuckIPC.exe /console` with `ngohaig`, and records
handled key replies, `status.schema_id=jyut6ping3`, `ctx.preedit=ngohaig`, and
candidate data.

The engine swap behind the M19 profile ABI is now a contained compatibility
path. Future full TSF automation or frontend modernization can proceed
independently without changing Yune's default ABI.
