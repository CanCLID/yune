# Yune Windows Native Build

> **Status:** T1/T2/T3 verified / M10 complete - **Milestone:** M10 (TypeDuck-Windows compatibility profile) - **Updated:** 2026-06-21 - **Type:** reference

This records the current TypeDuck-Windows native package evidence after the M10
resume. Yune now produces a Windows package and validates it through the named
TypeDuck-profile ABI. With ATL/MFC installed, the pinned TypeDuck-Windows
checkout also builds and links against that package after local frontend
handshake patches. T3 now passes with the stock real server IPC path:
`TypeDuckServer.exe` starts from `output\`, loads packaged `output\rime.dll`,
and stock `TestTypeDuckIPC.exe /console` returns a nonzero session, sends
`ngohaig` key events, and receives `status.schema_id=jyut6ping3` plus
candidate/context data.

## Tier Status

- **T0 ABI/header decision:** complete. The package uses an upstream-shaped
  default `rime_api.h` and a separate `rime_typeduck_profile_api.h` extension.
- **T1 package/link:** complete. Visual Studio 2022 Community MSBuild builds
  the pinned TypeDuck-Windows x64 solution and the deployer/server projects
  against the Yune package after the local profile-accessor settings patch and
  x64 WinSparkle import-library fix.
- **T2 packaged host-loader lifecycle:** complete. The package script loads the
  packaged `dist/lib/rime.dll`, resolves `rime_get_typeduck_profile_api()`,
  verifies profile append slots, and runs the dynamic-loader lifecycle smoke.
- **T3 real TypeDuck-Windows frontend smoke:** complete. Stock
  `TypeDuckServer.exe` starts from `output`, loads packaged `output\rime.dll`,
  and stock `TestTypeDuckIPC.exe /console` returns a nonzero session, sends
  `ngohaig` key events, and receives `status.schema_id=jyut6ping3` plus
  candidate/context data.

Highest verified tier: **T1 + T2 + documented T3**. M10 is **complete** as a
TypeDuck-Windows compatibility-profile backend milestone.

## ABI/Header Decision

The audited TypeDuck v1.1.2 fork header is not safe to package as Yune's default
header:

- `RimeCandidate` is fork-shaped: `text`, `comment`, `double quality`,
  `reserved`.
- `RimeApi` inserts fork-only `start_quick` in the default table.
- `RimeApi` inserts fork-only `config_list_append_{bool,int,double,string}` in
  the default table.

Yune keeps the default upstream ABI:

- default `RimeCandidate`: `text`, `comment`, `reserved`;
- default `rime_get_api()`: upstream `rime/librime 1.17.0` table;
- no `start_quick` and no list-append slots in the default table.

The package therefore copies upstream-shaped `rime_api.h`,
`rime_api_deprecated.h`, `rime_api_stdbool.h`, and `rime_levers_api.h` from:

```text
target\upstream-oracle\1.17.0\extract\dist\include
```

and adds:

```text
dist\include\rime_typeduck_profile_api.h
```

That header declares `RimeTypeDuckProfileApi` and
`rime_get_typeduck_profile_api()`. TypeDuck-Windows must include this profile
header and use the profile accessor for `config_list_append_*` when linked to a
Yune package. The pinned TypeDuck-Windows checkout does not directly read
`RimeCandidate.quality`, so an upstream-shaped candidate header is viable, but
the settings code still calls `rime_get_api()->config_list_append_string(...)`
today and needs the profile-accessor handshake before T1 can pass.

TypeDuck v1.1.2 exposes deprecated direct-call declarations such as
`RimeSetup` in `rime_api.h`. Upstream 1.17.0 keeps those declarations in
`rime_api_deprecated.h`. The Yune TypeDuck-Windows package keeps the upstream
struct/table layout but makes the packaged `rime_api.h` include the upstream
deprecated header, because the pinned TypeDuck-Windows source includes
`<rime_api.h>` while calling `RimeSetup`, `RimeInitialize`, and related direct
symbols.

## Package Layout

Current command from the repository root:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\package-typeduck-windows.ps1
```

Default output:

```text
target\typeduck-windows-native\x86_64-pc-windows-msvc\dist
  include\
    rime_api.h
    rime_api_deprecated.h
    rime_api_stdbool.h
    rime_levers_api.h
    rime_typeduck_profile_api.h
  lib\
    rime.dll
    rime.lib
    rime.pdb        # present when Cargo emits it
```

The script rejects fork-shaped default headers containing `double quality`,
`start_quick`, or default-table `config_list_append_string`. `-SkipSmoke` is
rejected and is not a valid M10 gate.

## T2 Smoke

The script sets `YUNE_TYPEDUCK_PACKAGE_RIME_DLL` to the packaged DLL and runs:

```powershell
cargo test -p yune-rime-api --test dynamic_loader dynamic_loader_harness_loads_packaged_typeduck_profile_dll -- --nocapture
```

The test verifies:

- `rime_get_api()` from the packaged DLL is upstream-sized;
- `rime_get_typeduck_profile_api()` is exported and advertises the larger
  profile table;
- the packaged DLL exports representative upstream-deprecated direct-call
  symbols used by TypeDuck-Windows (`RimeSetup`, `RimeInitialize`,
  `RimeFinalize`, `RimeGetContext`, `RimeConfigGetString`);
- `config_list_append_{bool,int,double,string}` are present and round-trip
  through config accessors;
- the native host lifecycle runs through the packaged profile table.

Verified locally on 2026-06-21:

```text
test dynamic_loader_harness_loads_packaged_typeduck_profile_dll ... ok

Packaged TypeDuck Windows native artifacts:
  C:\Users\laubonghaudoi\Documents\GitHub\yune\target\typeduck-windows-native\x86_64-pc-windows-msvc\dist\lib\rime.dll
  C:\Users\laubonghaudoi\Documents\GitHub\yune\target\typeduck-windows-native\x86_64-pc-windows-msvc\dist\lib\rime.lib
  C:\Users\laubonghaudoi\Documents\GitHub\yune\target\typeduck-windows-native\x86_64-pc-windows-msvc\dist\include\rime_api.h
  C:\Users\laubonghaudoi\Documents\GitHub\yune\target\typeduck-windows-native\x86_64-pc-windows-msvc\dist\include\rime_api_deprecated.h
  C:\Users\laubonghaudoi\Documents\GitHub\yune\target\typeduck-windows-native\x86_64-pc-windows-msvc\dist\include\rime_api_stdbool.h
  C:\Users\laubonghaudoi\Documents\GitHub\yune\target\typeduck-windows-native\x86_64-pc-windows-msvc\dist\include\rime_levers_api.h
  C:\Users\laubonghaudoi\Documents\GitHub\yune\target\typeduck-windows-native\x86_64-pc-windows-msvc\dist\include\rime_typeduck_profile_api.h
```

## TypeDuck-Windows Build And T3 Blocker

Pinned checkout:

```text
target\typeduck-windows-e2e\TypeDuck-Windows
f3ffcfe3b6a3018b1c3c9d256a6f0d587a2d2e27
```

The checkout had local batch-file and dependency modifications under `target/`;
they were not reset.

Initial tool lookup from this shell showed `msbuild.exe` was not on PATH:

```text
msbuild: MISSING
devenv: MISSING
cmake: MISSING
nuget: MISSING
nmake: MISSING
```

Visual Studio 2022 Community was later found at:

```text
C:\Program Files\Microsoft Visual Studio\2022\Community
```

and the installed MSBuild was usable by absolute path:

```text
C:\Program Files\Microsoft Visual Studio\2022\Community\Msbuild\Current\Bin\MSBuild.exe
```

The T1 checkout was prepared with the Yune package copied into:

```text
target\typeduck-windows-e2e\TypeDuck-Windows\include
target\typeduck-windows-e2e\TypeDuck-Windows\lib
target\typeduck-windows-e2e\TypeDuck-Windows\output
```

and Boost 1.84.0 was built locally at the short path:

```text
C:\b184
```

`weasel.props` was generated with `BOOST_ROOT=C:\b184` and
`PLATFORM_TOOLSET=v143`.

Earlier T1 commands:

```powershell
msbuild target\typeduck-windows-e2e\TypeDuck-Windows\weasel.sln /p:Configuration=Release /p:Platform=x64
& 'C:\Program Files\Microsoft Visual Studio\2022\Community\Msbuild\Current\Bin\MSBuild.exe' target\typeduck-windows-e2e\TypeDuck-Windows\weasel.sln /p:Configuration=Release /p:Platform=x64
```

Results:

```text
msbuild : The term 'msbuild' is not recognized as the name of a cmdlet, function, script file, or operable program.
FullyQualifiedErrorId : CommandNotFoundException

WeaselIPC.vcxproj -> ...\x64\Release\TypeDuckIPC.lib
WeaselUI\stdafx.h(12,10): error C1083: Cannot open include file: 'atlbase.h': No such file or directory
WeaselIME.rc(11): fatal error RC1015: cannot open include file 'afxres.h'.
```

That blocker was cleared after installing the Visual Studio ATL/MFC C++
components:

```text
atlbase.h: present
afxres.h: present
atls.lib: present
mfc140.lib: present
```

The package was also corrected to generate a real `rime.dll` import library.
The original copied `yune_rime_api.dll.lib` still pointed import records at
`yune_rime_api.dll`; `scripts/package-typeduck-windows.ps1` now derives
`dist\lib\rime.def` from packaged DLL exports and runs MSVC `lib.exe` so
`dumpbin /headers dist\lib\rime.lib` reports `DLL name : rime.dll`.

The TypeDuck-Windows checkout was locally prepared for the Yune package:

- `include\` received the packaged upstream-shaped headers plus
  `rime_typeduck_profile_api.h`;
- `lib\rime.lib` and `output\rime.dll` were copied from the Yune package;
- `WeaselDeployer\TypeDuckSettings.cpp` includes
  `rime_typeduck_profile_api.h` and calls
  `rime_get_typeduck_profile_api()->config_list_append_string(...)` for the
  fork-only settings lists;
- `WeaselServer.vcxproj` links `WinSparkle.lib`, and the checkout's 32-bit
  WinSparkle artifacts were replaced with the official x64 0.6.0
  `WinSparkle.lib`/`WinSparkle.dll`.

Current T1 commands:

```powershell
& 'C:\Program Files\Microsoft Visual Studio\2022\Community\Msbuild\Current\Bin\MSBuild.exe' target\typeduck-windows-e2e\TypeDuck-Windows\weasel.sln /p:Configuration=Release /p:Platform=x64 /m:1 /v:minimal
& 'C:\Program Files\Microsoft Visual Studio\2022\Community\Msbuild\Current\Bin\MSBuild.exe' target\typeduck-windows-e2e\TypeDuck-Windows\WeaselDeployer\WeaselDeployer.vcxproj /p:Configuration=Release /p:Platform=x64 /p:BuildProjectReferences=false /p:SolutionDir="target\typeduck-windows-e2e\TypeDuck-Windows\" /m /v:minimal
& 'C:\Program Files\Microsoft Visual Studio\2022\Community\Msbuild\Current\Bin\MSBuild.exe' target\typeduck-windows-e2e\TypeDuck-Windows\WeaselServer\WeaselServer.vcxproj /p:Configuration=Release /p:Platform=x64 /p:BuildProjectReferences=false /p:SolutionDir="target\typeduck-windows-e2e\TypeDuck-Windows\" /m /v:minimal
```

```text
WeaselIPC.vcxproj -> ...\x64\Release\TypeDuckIPC.lib
WeaselUI.vcxproj -> ...\x64\Release\TypeDuckUI.lib
WeaselTSF.vcxproj -> ...\output\typeduckx64.dll
WeaselIME.vcxproj -> ...\output\typeduckx64.ime
WeaselDeployer.vcxproj -> ...\output\TypeDuckDeployer.exe
WeaselServer.vcxproj -> ...\output\TypeDuckServer.exe
```

T1 is complete for the pinned checkout with those local patches.

T3 evidence:

- `TypeDuckServer.exe` starts from the pinned checkout's `output\` directory.
- The process loads
  `target\typeduck-windows-e2e\TypeDuck-Windows\output\rime.dll`.
- With HKCU `Software\Rime\TypeDuck\RimeUserDir` pointed at an isolated
  directory, first start deploys TypeDuck schema data and generated
  `jyut6ping3`, `cangjie3`, `cangjie5`, `loengfan`, and `luna_pinyin`
  `.schema.yaml`/`.table.bin`/`.prism.bin`/`.reverse.bin` artifacts.
- The packaged DLL, probed directly with the same shared/user directories,
  returns `RimeStartMaintenance(FALSE) == FALSE`, `RimeCreateSession() == 1`,
  and `RimeProcessKey(session, 'n', 0) == TRUE`,
  `RimeProcessKey(session, 'g', 0) == TRUE`.
- Root cause of the prior stock IPC blocker: fresh sessions stayed on Yune's
  placeholder `default` schema, so TypeDuck-Windows reached schema-specific
  settings with a non-schema id. Librime's default engine immediately asks the
  switcher for the first schema from `default.yaml`; Yune now mirrors that by
  applying the first deployed schema on session creation when a deployed schema
  list exists.
- Focused regression:
  `fresh_session_uses_first_deployed_schema_for_schema_specific_settings`.
- Final stock IPC evidence:
  `target\typeduck-windows-e2e\evidence\m10-t3-20260621-100337-stock-real-server`.
- Final packaged DLL SHA256:
  `6F6BABFD8C09EC1706D471457D7758D1D1F246D23D078992F3DD4ED1A6E2A6F2`.
- Final stock `TestTypeDuckIPC.exe /console` result: exit code `0`, handled key
  replies present, `status.schema_id=jyut6ping3`, `ctx.preedit=ngohaig`, and
  candidate data present; no temporary diagnostic fallback remained.
