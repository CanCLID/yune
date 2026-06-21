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
if ($SkipSmoke) {
    throw "-SkipSmoke is not a valid M10 package gate; the TypeDuck profile smoke must load the packaged DLL."
}
if ($Profile -ne "release" -and $Profile -ne "debug") {
    throw "unsupported profile '$Profile'; expected 'release' or 'debug'"
}

if ($OutputDir -eq "") {
    $OutputDir = Join-Path $RepoRoot "target\typeduck-windows-native\$Target"
}
if ($HeaderSource -eq "") {
    $HeaderSource = Join-Path $RepoRoot "target\upstream-oracle\1.17.0\extract\dist\include"
}

if (-not $NoBuild) {
    $cargo = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
    if (-not (Test-Path $cargo)) {
        $cargo = "cargo"
    }
    $BuildArgs = @("build", "-p", "yune-rime-api", "--target", $Target)
    if ($Profile -eq "release") {
        $BuildArgs += "--release"
    }
    & $cargo @BuildArgs
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
if (Select-String -Path $ApiHeader -Pattern "double quality" -Quiet) {
    throw "rime_api.h is fork-shaped and widens RimeCandidate with quality; package must use the upstream-shaped default candidate ABI"
}
if (Select-String -Path $ApiHeader -Pattern "start_quick" -Quiet) {
    throw "rime_api.h is fork-shaped and exposes start_quick in the default RimeApi; package must use the upstream-shaped default table"
}
if (Select-String -Path $ApiHeader -Pattern "config_list_append_string" -Quiet) {
    throw "rime_api.h exposes TypeDuck fork-only config_list_append_string in the default RimeApi; use rime_typeduck_profile_api.h instead"
}
$ProfileHeader = Join-Path $RepoRoot "crates\yune-rime-api\include\rime_typeduck_profile_api.h"
if (-not (Test-Path $ProfileHeader)) {
    throw "missing TypeDuck profile header: $ProfileHeader"
}
if (-not (Select-String -Path $ProfileHeader -Pattern "rime_get_typeduck_profile_api" -Quiet)) {
    throw "TypeDuck profile header does not declare rime_get_typeduck_profile_api"
}
if (-not (Select-String -Path $ProfileHeader -Pattern "config_list_append_string" -Quiet)) {
    throw "TypeDuck profile header does not expose config_list_append_string"
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
Copy-Item -LiteralPath $ProfileHeader -Destination (Join-Path $DistInclude "rime_typeduck_profile_api.h") -Force

$PackagedDll = Join-Path $DistLib "rime.dll"
$previousPackageDll = $env:YUNE_TYPEDUCK_PACKAGE_RIME_DLL
$env:YUNE_TYPEDUCK_PACKAGE_RIME_DLL = $PackagedDll
try {
    $cargo = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
    if (-not (Test-Path $cargo)) {
        $cargo = "cargo"
    }
    & $cargo test -p yune-rime-api --test dynamic_loader dynamic_loader_harness_loads_packaged_typeduck_profile_dll -- --nocapture
    if ($LASTEXITCODE -ne 0) {
        throw "packaged TypeDuck profile smoke failed for $PackagedDll"
    }
}
finally {
    if ($null -eq $previousPackageDll) {
        Remove-Item Env:\YUNE_TYPEDUCK_PACKAGE_RIME_DLL -ErrorAction SilentlyContinue
    } else {
        $env:YUNE_TYPEDUCK_PACKAGE_RIME_DLL = $previousPackageDll
    }
}

Write-Host "Packaged TypeDuck Windows native artifacts:"
Write-Host "  $OutputDir\dist\lib\rime.dll"
Write-Host "  $OutputDir\dist\lib\rime.lib"
Write-Host "  $OutputDir\dist\include\rime_api.h"
Write-Host "  $OutputDir\dist\include\rime_levers_api.h"
Write-Host "  $OutputDir\dist\include\rime_typeduck_profile_api.h"
