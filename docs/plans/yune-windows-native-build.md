# Yune Windows Native Build

> **Status:** Parked (M10 — deferred behind M9) · **Updated:** 2026-06-17 · **Type:** reference

This is the TypeDuck-Windows native package path for Yune's RIME-compatible ABI.
It produces the weasel-shaped layout:

```text
dist/
  include/
    rime_api.h
    rime_levers_api.h
  lib/
    rime.dll
    rime.lib
    rime.pdb        # present when Cargo emits it
```

## Prerequisites

- Rust target: `x86_64-pc-windows-msvc`
- MSVC linker/toolchain available to Cargo
- TypeDuck fork headers from the v1.1.2 oracle archive, defaulting to:

```powershell
target\typeduck-oracle\v1.1.2\extract\dist\include
```

Those headers are used because `RimeApi` field order is the C ABI. The Item 2 ABI
tests verify Yune's `config_list_append_*` slots match the TypeDuck fork header
order: after `config_list_size` and before `config_begin_list`.

## Build And Package

From the repository root:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\package-typeduck-windows.ps1
```

The script runs:

```powershell
cargo build -p yune-rime-api --release --target x86_64-pc-windows-msvc
```

Then it copies:

- `target\x86_64-pc-windows-msvc\release\yune_rime_api.dll` to `dist\lib\rime.dll`
- `target\x86_64-pc-windows-msvc\release\yune_rime_api.dll.lib` to `dist\lib\rime.lib`
- `rime_api.h` and `rime_levers_api.h` to `dist\include\`

Default output:

```text
target\typeduck-windows-native\x86_64-pc-windows-msvc\dist
```

Use `-OutputDir` or `-HeaderSource` to override the destination or header source.

## Smoke Check

By default the packaging script loads the packaged `dist\lib\rime.dll`, resolves
`rime_get_api`, verifies the returned `RimeApi` table has a positive `data_size`,
and checks that the TypeDuck-required `config_list_append_string` slot is non-null.

Skip only when packaging on a host that cannot load the Windows DLL:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\package-typeduck-windows.ps1 -SkipSmoke
```

## Verification Status

This repository currently provides the packaging script and smoke-check path. Do
not treat a native artifact as verified until the following commands have been
run on a Windows host with the MSVC target/toolchain and TypeDuck fork headers:

```powershell
cargo build -p yune-rime-api --release --target x86_64-pc-windows-msvc
powershell -ExecutionPolicy Bypass -File scripts\package-typeduck-windows.ps1
```

Expected result: the build produces `yune_rime_api.dll` and
`yune_rime_api.dll.lib`, the script packages them as `rime.dll`/`rime.lib`, and
the packaged DLL passes the `rime_get_api` / `config_list_append_string` smoke
check.
