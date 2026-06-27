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
    throw "-SkipSmoke is not a valid M10 package gate; the Yune Windows profile smoke must load the packaged DLL."
}
if ($Profile -ne "release" -and $Profile -ne "debug") {
    throw "unsupported profile '$Profile'; expected 'release' or 'debug'"
}

function Find-MsvcTool([string]$ToolName) {
    $command = Get-Command $ToolName -ErrorAction SilentlyContinue
    if ($command) {
        return $command.Source
    }

    $programFiles = @($env:ProgramFiles, ${env:ProgramFiles(x86)}) | Where-Object { $_ }
    $patterns = foreach ($root in $programFiles) {
        Join-Path $root "Microsoft Visual Studio\2022\*\VC\Tools\MSVC\*\bin\Hostx64\x64\$ToolName"
        Join-Path $root "Microsoft Visual Studio\2022\*\VC\Tools\MSVC\*\bin\Hostx86\x64\$ToolName"
    }
    foreach ($pattern in $patterns) {
        $match = Get-ChildItem -Path $pattern -ErrorAction SilentlyContinue |
            Sort-Object FullName -Descending |
            Select-Object -First 1
        if ($match) {
            return $match.FullName
        }
    }

    throw "missing MSVC tool $ToolName; install Visual Studio C++ tools or run from a Developer PowerShell"
}

function New-RimeImportLibrary([string]$DllPath, [string]$DefPath, [string]$LibPath, [string]$TargetTriple) {
    $dumpbin = Find-MsvcTool "dumpbin.exe"
    $lib = Find-MsvcTool "lib.exe"
    if ($TargetTriple -like "x86_64-*") {
        $machine = "x64"
    } elseif ($TargetTriple -like "i686-*") {
        $machine = "x86"
    } else {
        throw "unsupported MSVC import-library machine for target '$TargetTriple'"
    }

    $exports = & $dumpbin /exports $DllPath |
        ForEach-Object {
            if ($_ -match '^\s+\d+\s+[0-9A-Fa-f]+\s+[0-9A-Fa-f]+\s+([A-Za-z_][A-Za-z0-9_@?$]*)\s*$') {
                $Matches[1]
            }
        } |
        Sort-Object -Unique
    if (-not $exports -or -not ($exports -contains "rime_get_yune_windows_profile_api")) {
        throw "failed to derive Yune Windows profile exports from $DllPath"
    }

    Set-Content -LiteralPath $DefPath -Encoding ASCII -Value ((
        @("LIBRARY rime.dll", "EXPORTS") + ($exports | ForEach-Object { "    $_" })
    ) -join [Environment]::NewLine)
    & $lib /nologo "/machine:$machine" "/def:$DefPath" "/out:$LibPath" | Out-Host
    if ($LASTEXITCODE -ne 0) {
        throw "failed to generate import library $LibPath"
    }

    $headers = & $dumpbin /headers $LibPath
    if (-not ($headers -match 'DLL name\s+:\s+rime\.dll')) {
        throw "generated import library does not point at rime.dll"
    }
    if ($headers -match 'DLL name\s+:\s+yune_rime_api\.dll') {
        throw "generated import library still points at yune_rime_api.dll"
    }
}

if ($OutputDir -eq "") {
    $OutputDir = Join-Path $RepoRoot "target\yune-windows-native\$Target"
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
$DeprecatedApiHeader = Join-Path $HeaderSource "rime_api_deprecated.h"
$StdBoolApiHeader = Join-Path $HeaderSource "rime_api_stdbool.h"
$LeversHeader = Join-Path $HeaderSource "rime_levers_api.h"
if (-not (Test-Path $ApiHeader)) {
    throw "missing rime_api.h in $HeaderSource"
}
if (-not (Test-Path $DeprecatedApiHeader)) {
    throw "missing rime_api_deprecated.h in $HeaderSource"
}
if (-not (Test-Path $StdBoolApiHeader)) {
    throw "missing rime_api_stdbool.h in $HeaderSource"
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
    throw "rime_api.h exposes profile-only config_list_append_string in the default RimeApi; use rime_yune_windows_profile_api.h instead"
}
$ProfileHeader = Join-Path $RepoRoot "crates\yune-rime-api\include\rime_yune_windows_profile_api.h"
if (-not (Test-Path $ProfileHeader)) {
    throw "missing Yune Windows profile header: $ProfileHeader"
}
if (-not (Select-String -Path $ProfileHeader -Pattern "rime_get_yune_windows_profile_api" -Quiet)) {
    throw "Yune Windows profile header does not declare rime_get_yune_windows_profile_api"
}
if (-not (Select-String -Path $ProfileHeader -Pattern "config_list_append_string" -Quiet)) {
    throw "Yune Windows profile header does not expose config_list_append_string"
}

$DistLib = Join-Path $OutputDir "dist\lib"
$DistInclude = Join-Path $OutputDir "dist\include"
New-Item -ItemType Directory -Path $DistLib -Force | Out-Null
New-Item -ItemType Directory -Path $DistInclude -Force | Out-Null

Copy-Item -LiteralPath $SourceDll -Destination (Join-Path $DistLib "rime.dll") -Force
$PackagedDll = Join-Path $DistLib "rime.dll"
$PackagedLib = Join-Path $DistLib "rime.lib"
$PackagedDef = Join-Path $DistLib "rime.def"
New-RimeImportLibrary $PackagedDll $PackagedDef $PackagedLib $Target
if (Test-Path $SourcePdb) {
    Copy-Item -LiteralPath $SourcePdb -Destination (Join-Path $DistLib "rime.pdb") -Force
}
Copy-Item -LiteralPath $ApiHeader -Destination (Join-Path $DistInclude "rime_api.h") -Force
Copy-Item -LiteralPath $DeprecatedApiHeader -Destination (Join-Path $DistInclude "rime_api_deprecated.h") -Force
Copy-Item -LiteralPath $StdBoolApiHeader -Destination (Join-Path $DistInclude "rime_api_stdbool.h") -Force
Copy-Item -LiteralPath $LeversHeader -Destination (Join-Path $DistInclude "rime_levers_api.h") -Force
Copy-Item -LiteralPath $ProfileHeader -Destination (Join-Path $DistInclude "rime_yune_windows_profile_api.h") -Force

$DistApiHeader = Join-Path $DistInclude "rime_api.h"
if (-not (Select-String -Path $DistApiHeader -Pattern "rime_api_deprecated.h" -Quiet)) {
    Add-Content -LiteralPath $DistApiHeader -Value @(
        "",
        "/* Yune Windows v1.1.2 includes <rime_api.h> while using upstream-deprecated direct-call symbols such as RimeSetup. Keep upstream-shaped structs and RimeApi table slots, and expose the declarations through upstream 1.17.0's deprecated header. */",
        '#include "rime_api_deprecated.h"'
    )
}

$previousPackageDll = $env:YUNE_WINDOWS_PACKAGE_RIME_DLL
$env:YUNE_WINDOWS_PACKAGE_RIME_DLL = $PackagedDll
try {
    $cargo = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
    if (-not (Test-Path $cargo)) {
        $cargo = "cargo"
    }
    & $cargo test -p yune-rime-api --test dynamic_loader dynamic_loader_harness_loads_packaged_yune_windows_profile_dll -- --nocapture
    if ($LASTEXITCODE -ne 0) {
        throw "packaged Yune Windows profile smoke failed for $PackagedDll"
    }
}
finally {
    if ($null -eq $previousPackageDll) {
        Remove-Item Env:\YUNE_WINDOWS_PACKAGE_RIME_DLL -ErrorAction SilentlyContinue
    } else {
        $env:YUNE_WINDOWS_PACKAGE_RIME_DLL = $previousPackageDll
    }
}

Write-Host "Packaged Yune Windows native artifacts:"
Write-Host "  $OutputDir\dist\lib\rime.dll"
Write-Host "  $OutputDir\dist\lib\rime.lib"
Write-Host "  $OutputDir\dist\include\rime_api.h"
Write-Host "  $OutputDir\dist\include\rime_api_deprecated.h"
Write-Host "  $OutputDir\dist\include\rime_api_stdbool.h"
Write-Host "  $OutputDir\dist\include\rime_levers_api.h"
Write-Host "  $OutputDir\dist\include\rime_yune_windows_profile_api.h"

