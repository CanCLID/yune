param(
    [string]$OracleRoot,
    [string[]]$Inputs,
    [string]$Output,
    [string]$EvidenceMilestone = "M58",
    [string]$ReportedCaseInput = "zijiguk",
    [string]$ReportedCaseTargetCodepoints = "U+8AEE U+8B70 U+5C40",
    [string]$ExpectedRimeDllSha256 = "86b4c7357d4c6d293ce5589b234d8859ca2ac30923a03bedfa3926eeaf97fb0b",
    [string]$ExpectedRimeDeployerSha256 = "3abb72b5bb56fcafcfe925d533ae5f832c68d5a0bc9952fd0eea0682fb1ab071",
    [switch]$AllowMissingReportedCase,
    [switch]$AllowDirty
)

$ErrorActionPreference = "Stop"

function Write-NewUtf8NoBom([string]$Path, [string]$Text) {
    $Dir = Split-Path -Parent $Path
    if (-not [string]::IsNullOrWhiteSpace($Dir)) {
        New-Item -ItemType Directory -Force -Path $Dir | Out-Null
    }
    $Bytes = [System.Text.UTF8Encoding]::new($false).GetBytes($Text)
    $Stream = [System.IO.File]::Open(
        $Path,
        [System.IO.FileMode]::CreateNew,
        [System.IO.FileAccess]::Write,
        [System.IO.FileShare]::None
    )
    try {
        $Stream.Write($Bytes, 0, $Bytes.Length)
        $Stream.Flush()
    }
    finally {
        $Stream.Dispose()
    }
}

function Quote-CommandArg([string]$Value) {
    if ($null -eq $Value) {
        return "''"
    }
    return "'" + $Value.Replace("'", "''") + "'"
}

function Get-CanonicalCapturePath([string]$Path) {
    if (-not ("UpstreamCaptureFinalPath" -as [type])) {
        Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;
using Microsoft.Win32.SafeHandles;

public static class UpstreamCaptureFinalPath {
  const uint FileShareRead = 0x00000001;
  const uint FileShareWrite = 0x00000002;
  const uint FileShareDelete = 0x00000004;
  const uint OpenExisting = 3;
  const uint FileFlagBackupSemantics = 0x02000000;

  [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
  static extern SafeFileHandle CreateFile(
      string fileName,
      uint desiredAccess,
      uint shareMode,
      IntPtr securityAttributes,
      uint creationDisposition,
      uint flagsAndAttributes,
      IntPtr templateFile);

  [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
  static extern uint GetFinalPathNameByHandle(
      SafeFileHandle file,
      StringBuilder path,
      uint pathLength,
      uint flags);

  public static string Resolve(string path) {
    using (SafeFileHandle handle = CreateFile(
        path,
        0,
        FileShareRead | FileShareWrite | FileShareDelete,
        IntPtr.Zero,
        OpenExisting,
        FileFlagBackupSemantics,
        IntPtr.Zero)) {
      if (handle.IsInvalid) {
        throw new Win32Exception(Marshal.GetLastWin32Error(), "Cannot open path for canonicalization");
      }
      StringBuilder buffer = new StringBuilder(4096);
      uint length = GetFinalPathNameByHandle(handle, buffer, (uint)buffer.Capacity, 0);
      if (length == 0) {
        throw new Win32Exception(Marshal.GetLastWin32Error(), "Cannot canonicalize path");
      }
      if (length >= buffer.Capacity) {
        buffer = new StringBuilder((int)length + 1);
        length = GetFinalPathNameByHandle(handle, buffer, (uint)buffer.Capacity, 0);
        if (length == 0 || length >= buffer.Capacity) {
          throw new Win32Exception(Marshal.GetLastWin32Error(), "Cannot canonicalize path");
        }
      }
      string resolved = buffer.ToString();
      if (resolved.StartsWith(@"\\?\UNC\", StringComparison.OrdinalIgnoreCase)) {
        return @"\\" + resolved.Substring(8);
      }
      if (resolved.StartsWith(@"\\?\", StringComparison.OrdinalIgnoreCase)) {
        return resolved.Substring(4);
      }
      return resolved;
    }
  }
}
'@
    }

    $Full = [System.IO.Path]::GetFullPath($Path)
    $Root = [System.IO.Path]::GetPathRoot($Full)
    if ($Full.Substring($Root.Length).Contains(":")) {
        throw "Alternate data stream paths are not allowed: $Path"
    }
    if ($Full.Length -gt $Root.Length) {
        $Full = $Full.TrimEnd("\", "/")
    }
    $Existing = $Full
    $Suffix = New-Object System.Collections.Generic.List[string]
    while (-not (Test-Path -LiteralPath $Existing)) {
        $Leaf = [System.IO.Path]::GetFileName($Existing)
        $Parent = [System.IO.Path]::GetDirectoryName($Existing)
        if ([string]::IsNullOrWhiteSpace($Leaf) -or
            [string]::IsNullOrWhiteSpace($Parent) -or
            [string]::Equals($Parent, $Existing, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Cannot resolve an existing parent for path: $Path"
        }
        $Suffix.Insert(0, $Leaf)
        $Existing = $Parent
    }
    $ExistingItem = Get-Item -LiteralPath $Existing -Force
    if ($Suffix.Count -gt 0 -and -not $ExistingItem.PSIsContainer) {
        throw "Nearest existing parent is not a directory: $Existing"
    }
    $Canonical = [UpstreamCaptureFinalPath]::Resolve($Existing)
    foreach ($Leaf in $Suffix) {
        $Canonical = Join-Path $Canonical $Leaf
    }
    $Canonical = [System.IO.Path]::GetFullPath($Canonical)
    $CanonicalRoot = [System.IO.Path]::GetPathRoot($Canonical)
    if ($Canonical.Length -gt $CanonicalRoot.Length) {
        $Canonical = $Canonical.TrimEnd("\", "/")
    }
    return $Canonical
}

function Test-CapturePathWithinOrEqual([string]$Candidate, [string]$Root) {
    if ([string]::Equals($Candidate, $Root, [System.StringComparison]::OrdinalIgnoreCase)) {
        return $true
    }
    $Prefix = $Root.TrimEnd("\", "/") + [System.IO.Path]::DirectorySeparatorChar
    return $Candidate.StartsWith($Prefix, [System.StringComparison]::OrdinalIgnoreCase)
}

function Assert-UpstreamOutputPreflight(
    [string]$Output,
    [string]$OracleRoot,
    [string]$Shared,
    [string]$User
) {
    if (Test-Path -LiteralPath $Output) {
        throw "Output must not already exist: $Output"
    }
    $CanonicalOutput = Get-CanonicalCapturePath $Output
    $CanonicalOracleRoot = Get-CanonicalCapturePath $OracleRoot
    $CanonicalShared = Get-CanonicalCapturePath $Shared
    $CanonicalUser = Get-CanonicalCapturePath $User
    $ExpectedCanonicalShared = [System.IO.Path]::GetFullPath(
        (Join-Path $CanonicalOracleRoot "m58-rime-cantonese-shared")
    ).TrimEnd("\", "/")
    $ExpectedCanonicalUser = [System.IO.Path]::GetFullPath(
        (Join-Path $CanonicalOracleRoot "m58-rime-cantonese-user")
    ).TrimEnd("\", "/")
    if (-not [string]::Equals(
            $CanonicalShared,
            $ExpectedCanonicalShared,
            [System.StringComparison]::OrdinalIgnoreCase) -or
        -not [string]::Equals(
            $CanonicalUser,
            $ExpectedCanonicalUser,
            [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Recreated Shared/User roots must resolve to their exact expected OracleRoot leaf paths."
    }
    foreach ($RecreatedRoot in @($CanonicalShared, $CanonicalUser)) {
        if (-not (Test-CapturePathWithinOrEqual $RecreatedRoot $CanonicalOracleRoot) -or
            [string]::Equals(
                $RecreatedRoot,
                $CanonicalOracleRoot,
                [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Recreated Shared/User roots must be strict descendants of OracleRoot."
        }
    }
    if ((Test-CapturePathWithinOrEqual $CanonicalShared $CanonicalUser) -or
        (Test-CapturePathWithinOrEqual $CanonicalUser $CanonicalShared)) {
        throw "Recreated Shared/User roots must be distinct and non-nested."
    }
    if (Test-CapturePathWithinOrEqual $CanonicalOutput $CanonicalOracleRoot) {
        throw "Output must not be inside or equal to OracleRoot: $Output"
    }
    foreach ($CanonicalRoot in @($CanonicalShared, $CanonicalUser)) {
        if (Test-CapturePathWithinOrEqual $CanonicalOutput $CanonicalRoot) {
            throw "Output must not be inside a recreated Shared/User root: $Output"
        }
    }
    return $CanonicalOutput
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
    $Head = (& git -C $Path rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0 -or $Head -notmatch '^[0-9a-fA-F]{40}$') {
        throw "Unable to resolve Git HEAD for $Path"
    }
    return $Head.ToLowerInvariant()
}

function Git-State([string]$Path) {
    $Head = Git-Head $Path
    $Status = @(& git -C $Path status --short)
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to read Git status for $Path"
    }
    return [pscustomobject]@{
        commit = $Head
        clean = $Status.Count -eq 0
        status_short = @($Status)
    }
}

function Assert-GitStateUnchanged([string]$Path, [string]$Label, [object]$Before) {
    $After = Git-State $Path
    $BeforeStatus = @($Before.status_short) -join "`n"
    $AfterStatus = @($After.status_short) -join "`n"
    if ($After.commit -ne $Before.commit -or
        $After.clean -ne $Before.clean -or
        $AfterStatus -cne $BeforeStatus) {
        throw "Git source state changed during capture: $Label"
    }
}

function File-Sha256([string]$Path) {
    (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Assert-FileSha256Unchanged(
    [string]$Path,
    [string]$Label,
    [string]$ExpectedSha256
) {
    $ObservedSha256 = File-Sha256 $Path
    if ($ObservedSha256 -ne $ExpectedSha256) {
        throw "Binary changed during capture: $Label"
    }
}

function Convert-ToEvidencePath([string]$Path, [string]$Role = "path") {
    if ([string]::IsNullOrWhiteSpace($Path)) {
        return $Path
    }
    $FullPath = [System.IO.Path]::GetFullPath($Path)
    $RootPath = [System.IO.Path]::GetFullPath($RepoRoot).TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar)
    if ($FullPath.StartsWith($RootPath + [System.IO.Path]::DirectorySeparatorChar, [System.StringComparison]::OrdinalIgnoreCase) -or
        $FullPath.StartsWith($RootPath + [System.IO.Path]::AltDirectorySeparatorChar, [System.StringComparison]::OrdinalIgnoreCase)) {
        return $FullPath.Substring($RootPath.Length + 1).Replace("\", "/")
    }
    return "external/$Role"
}

function Get-RimeCaptureRuntimeOptionProvenance {
    $RuntimeOptions = [ordered]@{}
    $SeenNames = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
    foreach ($Option in [RimeProbe]::GetCaptureRuntimeOptions()) {
        $Name = [string]$Option.name
        if ($Name -notmatch '^[A-Za-z0-9_]+$' -or -not $SeenNames.Add($Name)) {
            throw "RimeProbe capture runtime options must have unique logical names."
        }
        if ($Option.enabled -isnot [bool]) {
            throw "RimeProbe capture runtime option '$Name' must be boolean."
        }
        $RuntimeOptions[$Name] = [bool]$Option.enabled
    }
    if ($RuntimeOptions.Count -eq 0) {
        throw "RimeProbe capture runtime option policy must not be empty."
    }
    $Source = [string][RimeProbe]::CaptureRuntimeOptionsSource
    if ([string]::IsNullOrWhiteSpace($Source)) {
        throw "RimeProbe capture runtime option policy must name its source."
    }
    return [pscustomobject]@{
        runtime_options = $RuntimeOptions
        runtime_options_source = $Source
    }
}

$InputsWereProvided = $PSBoundParameters.ContainsKey("Inputs") -and $null -ne $Inputs -and $Inputs.Count -gt 0
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
if ($EvidenceMilestone -notmatch '^M[0-9]+$') {
    throw "EvidenceMilestone must be an M-number such as M58 or M59."
}
$InputSet = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
foreach ($InputValue in @($Inputs)) {
    if ([string]::IsNullOrWhiteSpace($InputValue) -or -not $InputSet.Add($InputValue)) {
        throw "Capture inputs must be non-empty and unique."
    }
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

$CanonicalOutput = Assert-UpstreamOutputPreflight $Output $OracleRoot $Shared $User

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

$ToolState = Git-State $RepoRoot
$RepoStates = [ordered]@{}
foreach ($Repo in $RequiredRepos.Keys) {
    $RepoStates[$Repo] = Git-State (Join-Path $SchemaRoot $RequiredRepos[$Repo])
}
$DirtySources = New-Object System.Collections.Generic.List[string]
if (-not $ToolState.clean) {
    $DirtySources.Add("yune")
}
foreach ($Repo in $RepoStates.Keys) {
    if (-not $RepoStates[$Repo].clean) {
        $DirtySources.Add($Repo)
    }
}
if ($DirtySources.Count -gt 0 -and -not $AllowDirty.IsPresent) {
    throw "Refusing canonical oracle capture from dirty source trees: $($DirtySources -join ', '). Use -AllowDirty only for diagnostic evidence."
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
    throw "$EvidenceMilestone requires exact ASCII keystrokes for the reported plan/goal case. Include -ReportedCaseInput $ReportedCaseInput or re-run with -AllowMissingReportedCase only for provisional capture evidence, not milestone closeout."
}

$ProbeSha256 = File-Sha256 $ProbeSource
$CaptureScriptSha256 = File-Sha256 $PSCommandPath
Add-Type -Path $ProbeSource
$RuntimeOptionProvenance = Get-RimeCaptureRuntimeOptionProvenance
$RuntimeOptions = $RuntimeOptionProvenance.runtime_options
$RuntimeOptionsSource = [string]$RuntimeOptionProvenance.runtime_options_source
$AdditionalRuntimeOptionPatches = @()

$EffectiveParameters = [ordered]@{
    oracle_root = Convert-ToEvidencePath $OracleRoot "oracle-root"
    inputs = @($Inputs)
    inputs_source = if ($InputsWereProvided) { "explicit" } else { "default_13" }
    output = Convert-ToEvidencePath $Output "output"
    evidence_milestone = $EvidenceMilestone
    reported_case_input = $ReportedCaseInput
    reported_case_target_codepoints = $ReportedCaseTargetCodepoints
    expected_rime_dll_sha256 = if ($ExpectedRimeDllSha256) { $ExpectedRimeDllSha256.ToLowerInvariant() } else { $null }
    expected_rime_deployer_sha256 = if ($ExpectedRimeDeployerSha256) { $ExpectedRimeDeployerSha256.ToLowerInvariant() } else { $null }
    allow_missing_reported_case = $AllowMissingReportedCase.IsPresent
    allow_dirty = $AllowDirty.IsPresent
    shared_root = Convert-ToEvidencePath $Shared "shared-root"
    user_root = Convert-ToEvidencePath $User "user-root"
    build_root = Convert-ToEvidencePath $Build "build-root"
    runtime_options = $RuntimeOptions
    runtime_options_source = $RuntimeOptionsSource
    additional_runtime_option_patches = $AdditionalRuntimeOptionPatches
    path_serialization_policy = "repo-relative forward-slash paths; external paths replaced with external/<role>"
}
$Invocation = @(
    "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/capture-upstream-rime-cantonese.ps1"
)
if ($PSBoundParameters.ContainsKey("OracleRoot")) { $Invocation += "-OracleRoot $(Quote-CommandArg $EffectiveParameters.oracle_root)" }
if ($InputsWereProvided) { $Invocation += "-Inputs $(Quote-CommandArg ($Inputs -join ','))" }
if ($PSBoundParameters.ContainsKey("Output")) { $Invocation += "-Output $(Quote-CommandArg $EffectiveParameters.output)" }
if ($PSBoundParameters.ContainsKey("EvidenceMilestone")) { $Invocation += "-EvidenceMilestone $(Quote-CommandArg $EvidenceMilestone)" }
if ($PSBoundParameters.ContainsKey("ReportedCaseInput")) { $Invocation += "-ReportedCaseInput $(Quote-CommandArg $ReportedCaseInput)" }
if ($PSBoundParameters.ContainsKey("ReportedCaseTargetCodepoints")) { $Invocation += "-ReportedCaseTargetCodepoints $(Quote-CommandArg $ReportedCaseTargetCodepoints)" }
if ($PSBoundParameters.ContainsKey("ExpectedRimeDllSha256")) { $Invocation += "-ExpectedRimeDllSha256 $(Quote-CommandArg $EffectiveParameters.expected_rime_dll_sha256)" }
if ($PSBoundParameters.ContainsKey("ExpectedRimeDeployerSha256")) { $Invocation += "-ExpectedRimeDeployerSha256 $(Quote-CommandArg $EffectiveParameters.expected_rime_deployer_sha256)" }
if ($AllowMissingReportedCase.IsPresent) { $Invocation += "-AllowMissingReportedCase" }
if ($AllowDirty.IsPresent) { $Invocation += "-AllowDirty" }
$ActualInvocation = $Invocation -join " "

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

$Modules = [string[]]@("default")
$Cases = [RimeProbe]::Capture($Shared, $User, $Build, "jyut6ping3", $Modules, [string[]]$Inputs)
if ($Cases.Count -ne $Inputs.Count) {
    throw "Oracle capture returned $($Cases.Count) cases for $($Inputs.Count) inputs."
}
foreach ($Case in $Cases) {
    if (-not $Case["captured_all_pages"]) {
        $Reason = "unknown"
        if ($Case.ContainsKey("pagination_error")) {
            $Reason = $Case["pagination_error"]
        }
        throw "Capture for input '$($Case["input"])' did not capture all pages: $Reason"
    }
}

Assert-GitStateUnchanged $RepoRoot "yune" $ToolState
foreach ($Repo in $RequiredRepos.Keys) {
    Assert-GitStateUnchanged (Join-Path $SchemaRoot $RequiredRepos[$Repo]) $Repo $RepoStates[$Repo]
}
Assert-FileSha256Unchanged $RimeDll "rime.dll" $ActualRimeDllSha256
Assert-FileSha256Unchanged $RimeDeployer "rime_deployer.exe" $ActualRimeDeployerSha256

$RepoCommits = [ordered]@{}
$RepoClean = [ordered]@{}
$RepoStatusShort = [ordered]@{}
foreach ($Repo in $RequiredRepos.Keys) {
    $RepoCommits[$Repo] = $RepoStates[$Repo].commit
    $RepoClean[$Repo] = [bool]$RepoStates[$Repo].clean
    $RepoStatusShort[$Repo] = @($RepoStates[$Repo].status_short)
}

$Pages = @($Cases | ForEach-Object { $_["page_size"] } | Select-Object -Unique)
$AllPagesCaptured = @($Cases | Where-Object { -not $_["captured_all_pages"] }).Count -eq 0
$OutputProvenance = [ordered]@{
    path = $EffectiveParameters.output
    existed_before_capture = $false
    write_policy = "utf8_no_bom_create_new"
    generated_by = "scripts/capture-upstream-rime-cantonese.ps1"
}
$Evidence = [ordered]@{
    milestone = $EvidenceMilestone
    status = if ($AllowMissingReportedCase) {
        "provisional_blocked_missing_reported_case_ascii"
    }
    elseif ($DirtySources.Count -gt 0) {
        "diagnostic_dirty_source_capture"
    }
    else {
        "canonical_capture_complete"
    }
    canonical = (-not $AllowMissingReportedCase.IsPresent) -and $DirtySources.Count -eq 0
    capture = [ordered]@{
        engine = "rime/librime"
        version = "1.17.0"
        librime_commit = "33e78140250125871856cdc5b42ddc6a5fcd3cd4"
        source_commit = $ToolState.commit
        source_clean = [bool]$ToolState.clean
        source_dirty = -not $ToolState.clean
        source_status_short = @($ToolState.status_short)
        source_repo = "yune"
        schema_id = "jyut6ping3"
        modules = @($Modules)
        inputs = @($Inputs)
        input_count = $Inputs.Count
        inputs_source = $EffectiveParameters.inputs_source
        page_sizes_observed = @($Pages)
        captured_all_pages = $AllPagesCaptured
        page_policy = "RimeProbe.Capture all pages; hard failure on non-advancing or incomplete pagination"
        runtime_options = $RuntimeOptions
        runtime_options_source = $RuntimeOptionsSource
        additional_runtime_option_patches = $AdditionalRuntimeOptionPatches
        path_serialization_policy = $EffectiveParameters.path_serialization_policy
        rime_dll_sha256 = $ActualRimeDllSha256
        rime_deployer_sha256 = $ActualRimeDeployerSha256
        schema_repo_commits = $RepoCommits
        source_repositories_clean = $RepoClean
        source_repositories_status_short = $RepoStatusShort
        capture_script_sha256 = $CaptureScriptSha256
        probe_sha256 = $ProbeSha256
        actual_invocation = $ActualInvocation
        effective_parameters = $EffectiveParameters
        output_provenance = $OutputProvenance
    }
    oracle = [ordered]@{
        engine = "rime/librime"
        version = "1.17.0"
        commit = "33e78140250125871856cdc5b42ddc6a5fcd3cd4"
        dll = Convert-ToEvidencePath $RimeDll "rime-dll"
        dll_sha256 = $ActualRimeDllSha256
        deployer = Convert-ToEvidencePath $RimeDeployer "rime-deployer"
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
        runtime_option_patches_scope = "legacy alias: no additional overrides beyond runtime_options"
        runtime_options = $RuntimeOptions
        runtime_options_source = $RuntimeOptionsSource
        additional_runtime_option_patches = $AdditionalRuntimeOptionPatches
        custom_yaml = "default.custom.yaml only selects jyut6ping3"
        schema_defaults = "ascii_mode reset 0; character-style switch default comes from rime-cantonese jyut6ping3.schema.yaml"
        page_sizes_observed = @($Pages)
    }
    commands = [ordered]@{
        deploy = "rime_deployer.exe --build $(Convert-ToEvidencePath $User 'user-root') $(Convert-ToEvidencePath $Shared 'shared-root') $(Convert-ToEvidencePath $Build 'build-root')"
        capture = $ActualInvocation
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

Write-NewUtf8NoBom $Output (($Evidence | ConvertTo-Json -Depth 100) + "`n")
Write-Host "Wrote canonical rime-cantonese capture evidence to $Output"
