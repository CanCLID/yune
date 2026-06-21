# Yune Windows Native Build

> **Status:** T2 verified / M10 blocked - **Milestone:** M10 (TypeDuck-Windows compatibility profile) - **Updated:** 2026-06-21 - **Type:** reference

This records the current TypeDuck-Windows native package evidence after the M10
resume. Yune now produces a Windows package and validates it through the named
TypeDuck-profile ABI, but the real TypeDuck-Windows build and frontend smoke
are still blocked by missing ATL/MFC components in the local Visual Studio
installation and by the remaining TypeDuck-Windows settings call sites that
must switch fork-only append slots to the profile accessor.

## Tier Status

- **T0 ABI/header decision:** complete. The package uses an upstream-shaped
  default `rime_api.h` and a separate `rime_typeduck_profile_api.h` extension.
- **T1 package/link:** blocked. An explicit Visual Studio 2022 MSBuild path can
  build the solution far enough to compile against the Yune package, but the
  selected TypeDuck-Windows projects require ATL/MFC headers that are not
  installed locally (`atlbase.h`, `afxres.h`). The updated package headers let
  the rime-facing `RimeWithWeasel` static-library target compile when project
  references are disabled; the full frontend build/link still does not pass.
- **T2 packaged host-loader lifecycle:** complete. The package script loads the
  packaged `dist/lib/rime.dll`, resolves `rime_get_typeduck_profile_api()`,
  verifies profile append slots, and runs the dynamic-loader lifecycle smoke.
- **T3 real TypeDuck-Windows frontend smoke:** blocked behind T1. No real
  frontend binary was built from the Yune package in this environment.

Highest verified tier: **T2**. M10 remains **blocked**, not complete.

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

## TypeDuck-Windows Build Blocker

Pinned checkout:

```text
target\typeduck-windows-e2e\TypeDuck-Windows
f3ffcfe3b6a3018b1c3c9d256a6f0d587a2d2e27
```

The checkout had local batch-file modifications under `target/`; they were not
reset or edited.

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

Attempted T1 commands:

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

The missing headers were confirmed absent under the installed MSVC tree:

```text
C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\atlmfc\include\atlbase.h
C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\atlmfc\include\afxres.h
```

After the package was updated to include upstream deprecated direct-call
declarations, the smallest rime-facing static-library target compiled against
the Yune package when project references were disabled:

```powershell
& 'C:\Program Files\Microsoft Visual Studio\2022\Community\Msbuild\Current\Bin\MSBuild.exe' target\typeduck-windows-e2e\TypeDuck-Windows\RimeWithWeasel\RimeWithWeasel.vcxproj /p:Configuration=Release /p:Platform=x64 /p:BuildProjectReferences=false /p:SolutionDir="target\typeduck-windows-e2e\TypeDuck-Windows\" /m /v:minimal
```

```text
RimeWithWeasel.vcxproj -> ...\x64\Release\RimeWithTypeDuck.lib
```

This is not T1 completion: the full solution still requires ATL/MFC, and the
deployer/settings path still needs the documented profile-accessor patch for
`config_list_append_*` before a Yune package can be the real frontend engine.

Because T1 still did not complete, T3 real TypeDuck-Windows frontend smoke also
did not run. A future T1/T3 worker should install the Visual Studio ATL/MFC C++
components, reuse the package above, patch the settings path to include
`rime_typeduck_profile_api.h` and call `rime_get_typeduck_profile_api()` for
`config_list_append_*`, build from a
Visual Studio developer shell, then record real frontend input/output smoke
against the Yune package.
