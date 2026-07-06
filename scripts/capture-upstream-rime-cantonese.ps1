param(
    [string]$OracleRoot,
    [string[]]$Inputs,
    [string]$Output,
    [string]$ReportedCaseInput = "zijiguk",
    [string]$ReportedCaseTargetCodepoints = "U+8AEE U+8B70 U+5C40",
    [string]$ExpectedRimeDllSha256 = "86b4c7357d4c6d293ce5589b234d8859ca2ac30923a03bedfa3926eeaf97fb0b",
    [string]$ExpectedRimeDeployerSha256 = "3abb72b5bb56fcafcfe925d533ae5f832c68d5a0bc9952fd0eea0682fb1ab071",
    [switch]$AllowMissingReportedCase
)

$ErrorActionPreference = "Stop"

$RepoRoot = [System.IO.Path]::GetFullPath((Resolve-Path (Join-Path $PSScriptRoot "..")).Path)
if ([string]::IsNullOrWhiteSpace($OracleRoot)) {
    $OracleRoot = Join-Path $RepoRoot "target\upstream-oracle\1.17.0"
}
if ($null -eq $Inputs -or $Inputs.Count -eq 0) {
    $Inputs = @(
        "be",
        "bei",
        "bein",
        "being",
        "beingo",
        "beix",
        "beixngoxx",
        "ngohaig",
        "ngohaigo",
        "n",
        "nri",
        "mgoi",
        "zijiguk"
    )
}
if ($Inputs.Count -eq 1 -and $Inputs[0].Contains(",")) {
    $Inputs = $Inputs[0].Split(",") |
        ForEach-Object { $_.Trim() } |
        Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
}
if ([string]::IsNullOrWhiteSpace($Output)) {
    $Output = Join-Path $RepoRoot "docs\reports\evidence\m58-jyutping-exact-before-fuzzy\phase-1\canonical-rime-cantonese-capture.json"
}

$OracleRoot = [System.IO.Path]::GetFullPath($OracleRoot)
$Output = [System.IO.Path]::GetFullPath($Output)
$Extract = Join-Path $OracleRoot "extract"
$Shared = Join-Path $OracleRoot "m58-rime-cantonese-shared"
$User = Join-Path $OracleRoot "m58-rime-cantonese-user"
$Build = Join-Path $User "build"
$SchemaRoot = Join-Path $OracleRoot "schema-src"
$ProbeSource = Join-Path $RepoRoot "scripts\oracle-rime-probe.cs"
$RimeDll = Join-Path $Extract "dist\lib\rime.dll"
$RimeDeployer = Join-Path $Extract "dist\bin\rime_deployer.exe"

$RequiredRepos = [ordered]@{
    "rime/rime-cantonese" = "rime-cantonese"
    "rime/rime-prelude" = "rime-prelude"
    "rime/rime-luna-pinyin" = "rime-luna-pinyin"
    "rime/rime-essay" = "rime-essay"
    "rime/rime-stroke" = "rime-stroke"
    "rime/rime-cangjie" = "rime-cangjie"
    "CanCLID/rime-loengfan" = "rime-loengfan"
}

function Write-Utf8NoBom([string]$Path, [string]$Text) {
    $Dir = Split-Path -Parent $Path
    if (-not [string]::IsNullOrWhiteSpace($Dir)) {
        New-Item -ItemType Directory -Force -Path $Dir | Out-Null
    }
    $Encoding = [System.Text.UTF8Encoding]::new($false)
    [System.IO.File]::WriteAllText($Path, $Text, $Encoding)
}

function Copy-RimeData([string]$Source, [string]$Destination) {
    Get-ChildItem -LiteralPath $Source -File |
        Where-Object {
            $_.Name -like "*.yaml" -or
            $_.Name -like "*.dict.yaml" -or
            $_.Name -like "*.txt"
        } |
        ForEach-Object {
            Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $Destination $_.Name) -Force
        }
    $OpenCcSource = Join-Path $Source "opencc"
    if (Test-Path -LiteralPath $OpenCcSource) {
        $OpenCcDest = Join-Path $Destination "opencc"
        New-Item -ItemType Directory -Force -Path $OpenCcDest | Out-Null
        Get-ChildItem -LiteralPath $OpenCcSource | ForEach-Object {
            Copy-Item -LiteralPath $_.FullName -Destination $OpenCcDest -Recurse -Force
        }
    }
}

function Git-Head([string]$Path) {
    (& git -C $Path rev-parse HEAD).Trim()
}

function File-Sha256([string]$Path) {
    (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Convert-ToEvidencePath([string]$Path) {
    if ([string]::IsNullOrWhiteSpace($Path)) {
        return $Path
    }
    $FullPath = [System.IO.Path]::GetFullPath($Path)
    $RootPath = [System.IO.Path]::GetFullPath($RepoRoot).TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar)
    if ($FullPath.StartsWith($RootPath + [System.IO.Path]::DirectorySeparatorChar, [System.StringComparison]::OrdinalIgnoreCase) -or
        $FullPath.StartsWith($RootPath + [System.IO.Path]::AltDirectorySeparatorChar, [System.StringComparison]::OrdinalIgnoreCase)) {
        return $FullPath.Substring($RootPath.Length + 1).Replace("\", "/")
    }
    return $FullPath
}

$RequiredPaths = @(
    $RimeDll,
    $RimeDeployer,
    (Join-Path $Extract "dist\include\rime_api.h"),
    $ProbeSource
)
foreach ($RepoPath in $RequiredRepos.Values) {
    $RequiredPaths += Join-Path $SchemaRoot $RepoPath
}
foreach ($Path in $RequiredPaths) {
    if (-not (Test-Path -LiteralPath $Path)) {
        throw "Missing required upstream rime-cantonese capture input: $Path"
    }
}

$ActualRimeDllSha256 = File-Sha256 $RimeDll
$ActualRimeDeployerSha256 = File-Sha256 $RimeDeployer
if (-not [string]::IsNullOrWhiteSpace($ExpectedRimeDllSha256) -and $ActualRimeDllSha256 -ne $ExpectedRimeDllSha256.ToLowerInvariant()) {
    throw "Unexpected upstream rime.dll SHA-256. Expected $ExpectedRimeDllSha256, got $ActualRimeDllSha256 at $RimeDll"
}
if (-not [string]::IsNullOrWhiteSpace($ExpectedRimeDeployerSha256) -and $ActualRimeDeployerSha256 -ne $ExpectedRimeDeployerSha256.ToLowerInvariant()) {
    throw "Unexpected upstream rime_deployer.exe SHA-256. Expected $ExpectedRimeDeployerSha256, got $ActualRimeDeployerSha256 at $RimeDeployer"
}

if (-not $AllowMissingReportedCase -and -not ($Inputs -contains $ReportedCaseInput)) {
    throw "M58 requires exact ASCII keystrokes for the reported plan/goal case. Include -ReportedCaseInput $ReportedCaseInput or re-run with -AllowMissingReportedCase only for provisional capture evidence, not milestone closeout."
}

foreach ($Dir in @($Shared, $User)) {
    $ResolvedRoot = [System.IO.Path]::GetFullPath($OracleRoot)
    $ResolvedDir = [System.IO.Path]::GetFullPath($Dir)
    if (-not $ResolvedDir.StartsWith($ResolvedRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to recreate outside oracle root: $ResolvedDir"
    }
    if (Test-Path -LiteralPath $Dir) {
        Remove-Item -LiteralPath $Dir -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $Dir | Out-Null
}

foreach ($RepoPath in $RequiredRepos.Values) {
    Copy-RimeData (Join-Path $SchemaRoot $RepoPath) $Shared
}

$UpstreamOpenCc = Join-Path $Extract "share\opencc"
if (Test-Path -LiteralPath $UpstreamOpenCc) {
    $OpenCcDest = Join-Path $Shared "opencc"
    New-Item -ItemType Directory -Force -Path $OpenCcDest | Out-Null
    Get-ChildItem -LiteralPath $UpstreamOpenCc | ForEach-Object {
        Copy-Item -LiteralPath $_.FullName -Destination $OpenCcDest -Recurse -Force
    }
}

@"
patch:
  schema_list:
    - schema: jyut6ping3
"@ | Set-Content -LiteralPath (Join-Path $Shared "default.custom.yaml") -Encoding UTF8

New-Item -ItemType Directory -Force -Path $Build | Out-Null
$env:PATH = (Join-Path $Extract "dist\lib") + ";" + (Join-Path $Extract "bin") + ";" + $env:PATH
& (Join-Path $Extract "dist\bin\rime_deployer.exe") --build $User $Shared $Build
if ($LASTEXITCODE -ne 0) {
    throw "rime_deployer.exe --build failed with exit code $LASTEXITCODE"
}

Add-Type -Path $ProbeSource
$Modules = [string[]]@("default")
$Cases = [RimeProbe]::Capture($Shared, $User, $Build, "jyut6ping3", $Modules, [string[]]$Inputs)
foreach ($Case in $Cases) {
    if (-not $Case["captured_all_pages"]) {
        $Reason = "unknown"
        if ($Case.ContainsKey("pagination_error")) {
            $Reason = $Case["pagination_error"]
        }
        throw "Capture for input '$($Case["input"])' did not capture all pages: $Reason"
    }
}

$RepoCommits = [ordered]@{}
foreach ($Repo in $RequiredRepos.Keys) {
    $RepoCommits[$Repo] = Git-Head (Join-Path $SchemaRoot $RequiredRepos[$Repo])
}

$Pages = @($Cases | ForEach-Object { $_["page_size"] } | Select-Object -Unique)
$Evidence = [ordered]@{
    milestone = "M58"
    status = if ($AllowMissingReportedCase) { "provisional_blocked_missing_reported_case_ascii" } else { "canonical_capture_complete" }
    canonical = $true
    oracle = [ordered]@{
        engine = "rime/librime"
        version = "1.17.0"
        commit = "33e78140250125871856cdc5b42ddc6a5fcd3cd4"
        dll = Convert-ToEvidencePath $RimeDll
        dll_sha256 = $ActualRimeDllSha256
        deployer = Convert-ToEvidencePath $RimeDeployer
        deployer_sha256 = $ActualRimeDeployerSha256
    }
    schema = [ordered]@{
        yune_facing_schema_id = "jyut6ping3"
        source_repo = "rime/rime-cantonese"
        source_commit = $RepoCommits["rime/rime-cantonese"]
        dependency_commits = $RepoCommits
    }
    options = [ordered]@{
        runtime_option_patches = @()
        custom_yaml = "default.custom.yaml only selects jyut6ping3"
        schema_defaults = "ascii_mode reset 0; character-style switch default comes from rime-cantonese jyut6ping3.schema.yaml"
        page_sizes_observed = @($Pages)
    }
    commands = [ordered]@{
        deploy = "rime_deployer.exe --build $(Convert-ToEvidencePath $User) $(Convert-ToEvidencePath $Shared) $(Convert-ToEvidencePath $Build)"
        capture = if ($AllowMissingReportedCase) { "scripts/capture-upstream-rime-cantonese.ps1 -AllowMissingReportedCase" } else { "scripts/capture-upstream-rime-cantonese.ps1 -ReportedCaseInput $ReportedCaseInput" }
    }
    inputs = @($Inputs)
    reported_case = [ordered]@{
        target_codepoints = $ReportedCaseTargetCodepoints
        input = $ReportedCaseInput
        provenance = "User-specified M58 unblock decision: untoned Jyutping for zi1 ji5 guk6; use only as capture input, not as expected output derived from Yune."
        complete = -not $AllowMissingReportedCase
    }
    cases = $Cases
}

Write-Utf8NoBom $Output (($Evidence | ConvertTo-Json -Depth 100) + "`n")
Write-Host "Wrote canonical rime-cantonese capture evidence to $Output"
