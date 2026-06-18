param(
    [string]$Target = "x86_64-pc-windows-msvc",
    [string]$Profile = "release",
    [string]$OutputDir = "",
    [string]$HeaderSource = "",
    [switch]$NoBuild,
    [switch]$SkipSmoke
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path (Join-Path $ScriptDir "..")

if ($OutputDir -eq "") {
    $OutputDir = Join-Path $RepoRoot "target\typeduck-windows-native\$Target"
}
if ($HeaderSource -eq "") {
    $HeaderSource = Join-Path $RepoRoot "target\typeduck-oracle\v1.1.2\extract\dist\include"
}

if (-not $NoBuild) {
    $cargo = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
    if (-not (Test-Path $cargo)) {
        $cargo = "cargo"
    }
    & $cargo build -p yune-rime-api --release --target $Target
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build failed for target $Target"
    }
}

$ArtifactDir = Join-Path $RepoRoot "target\$Target\$Profile"
$SourceDll = Join-Path $ArtifactDir "yune_rime_api.dll"
$SourceLib = Join-Path $ArtifactDir "yune_rime_api.dll.lib"
$SourcePdb = Join-Path $ArtifactDir "yune_rime_api.pdb"

if (-not (Test-Path $SourceDll)) {
    throw "missing built DLL: $SourceDll"
}
if (-not (Test-Path $SourceLib)) {
    throw "missing import library: $SourceLib"
}
if (-not (Test-Path $HeaderSource)) {
    throw "missing header source: $HeaderSource"
}

$ApiHeader = Join-Path $HeaderSource "rime_api.h"
$LeversHeader = Join-Path $HeaderSource "rime_levers_api.h"
if (-not (Test-Path $ApiHeader)) {
    throw "missing rime_api.h in $HeaderSource"
}
if (-not (Test-Path $LeversHeader)) {
    throw "missing rime_levers_api.h in $HeaderSource"
}
if (-not (Select-String -Path $ApiHeader -Pattern "config_list_append_string" -Quiet)) {
    throw "rime_api.h does not expose config_list_append_string"
}

$DistLib = Join-Path $OutputDir "dist\lib"
$DistInclude = Join-Path $OutputDir "dist\include"
New-Item -ItemType Directory -Path $DistLib -Force | Out-Null
New-Item -ItemType Directory -Path $DistInclude -Force | Out-Null

Copy-Item -LiteralPath $SourceDll -Destination (Join-Path $DistLib "rime.dll") -Force
Copy-Item -LiteralPath $SourceLib -Destination (Join-Path $DistLib "rime.lib") -Force
if (Test-Path $SourcePdb) {
    Copy-Item -LiteralPath $SourcePdb -Destination (Join-Path $DistLib "rime.pdb") -Force
}
Copy-Item -LiteralPath $ApiHeader -Destination (Join-Path $DistInclude "rime_api.h") -Force
Copy-Item -LiteralPath $LeversHeader -Destination (Join-Path $DistInclude "rime_levers_api.h") -Force

if (-not $SkipSmoke) {
    $SmokeSource = @"
using System;
using System.Runtime.InteropServices;

public static class YuneRimeSmoke {
    [DllImport("kernel32", SetLastError = true, CharSet = CharSet.Unicode)]
    private static extern IntPtr LoadLibraryW(string path);

    [DllImport("kernel32", SetLastError = true, CharSet = CharSet.Ansi)]
    private static extern IntPtr GetProcAddress(IntPtr module, string name);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate IntPtr RimeGetApi();

    public static void Check(string path) {
        IntPtr module = LoadLibraryW(path);
        if (module == IntPtr.Zero) {
            throw new InvalidOperationException("LoadLibraryW failed for " + path);
        }
        IntPtr symbol = GetProcAddress(module, "rime_get_api");
        if (symbol == IntPtr.Zero) {
            throw new InvalidOperationException("missing rime_get_api");
        }
        RimeGetApi getApi = (RimeGetApi)Marshal.GetDelegateForFunctionPointer(symbol, typeof(RimeGetApi));
        IntPtr api = getApi();
        if (api == IntPtr.Zero) {
            throw new InvalidOperationException("rime_get_api returned null");
        }
        int dataSize = Marshal.ReadInt32(api);
        if (dataSize <= 0) {
            throw new InvalidOperationException("RimeApi data_size is not positive");
        }
        int tableStart = IntPtr.Size == 8 ? 8 : 4;
        IntPtr appendString = Marshal.ReadIntPtr(api, tableStart + 71 * IntPtr.Size);
        if (appendString == IntPtr.Zero) {
            throw new InvalidOperationException("config_list_append_string slot is null");
        }
    }
}
"@
    Add-Type -TypeDefinition $SmokeSource
    [YuneRimeSmoke]::Check((Join-Path $DistLib "rime.dll"))
}

Write-Host "Packaged TypeDuck Windows native artifacts:"
Write-Host "  $OutputDir\dist\lib\rime.dll"
Write-Host "  $OutputDir\dist\lib\rime.lib"
Write-Host "  $OutputDir\dist\include\rime_api.h"
Write-Host "  $OutputDir\dist\include\rime_levers_api.h"
