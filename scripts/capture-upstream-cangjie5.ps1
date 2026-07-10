# Capture the pinned librime 1.17.0 cangjie5 oracle for M59 D-48.
#
# This script is intentionally strict ASCII. Owner targets are declared only as
# U+ codepoint specifications. The Python curator derives target strings from
# captured candidate zero and rejects any mismatch, so Windows PowerShell 5 can
# never transcode an expected CJK literal into the evidence.
param(
    [string]$OracleRoot,
    [Parameter(Mandatory = $true)]
    [string]$RawOutput,
    [Parameter(Mandatory = $true)]
    [string]$Output,
    [string]$WorkRoot,
    [switch]$KeepWorkRoot,
    [string]$ExpectedRimeDllSha256 = "86b4c7357d4c6d293ce5589b234d8859ca2ac30923a03bedfa3926eeaf97fb0b",
    [string]$ExpectedRimeDeployerSha256 = "3abb72b5bb56fcafcfe925d533ae5f832c68d5a0bc9952fd0eea0682fb1ab071",
    [string]$ExpectedCangjieCommit = "52d90a1b1312e74042b38c1cbc8142defbc53171",
    [string]$ExpectedPreludeCommit = "082425ea0684bca36474415d4a0e8db9b016487e",
    [string]$ExpectedLunaPinyinCommit = "18a80335c37522311f7cff02886cd81cec3b460a",
    [string]$ExpectedEssayCommit = "48c7538f0b760fcc8c9d6bf08711f82cfbd2e9ed",
    [string]$ExpectedStrokeCommit = "3a4b0f4013e2b4c14b1e80c92b1d4723eb65f39c",
    [string]$ExpectedCantoneseCommit = "c99b16e44d2df77a5cb8fb0867dd2bab7a112cb0",
    [string]$ExpectedLoengfanCommit = "987ac95b02f957e8764a2f45222a4006c188ed50"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version 2.0

$RepoRoot = [System.IO.Path]::GetFullPath((Resolve-Path (Join-Path $PSScriptRoot "..")).Path)
if ([string]::IsNullOrWhiteSpace($OracleRoot)) {
    $OracleRoot = Join-Path $RepoRoot "target\upstream-oracle\1.17.0"
}
$OracleRoot = [System.IO.Path]::GetFullPath($OracleRoot)
$RawOutput = [System.IO.Path]::GetFullPath($RawOutput)
$Output = [System.IO.Path]::GetFullPath($Output)
$WorkRootWasProvided = -not [string]::IsNullOrWhiteSpace($WorkRoot)
if (-not $WorkRootWasProvided) {
    $WorkRoot = Join-Path ([System.IO.Path]::GetTempPath()) (
        "yune-m59-cangjie5-capture-" + [guid]::NewGuid().ToString("N")
    )
}
$WorkRoot = [System.IO.Path]::GetFullPath($WorkRoot)

$Extract = Join-Path $OracleRoot "extract"
$SchemaRoot = Join-Path $OracleRoot "schema-src"
$Shared = Join-Path $WorkRoot "shared"
$User = Join-Path $WorkRoot "user"
$Build = Join-Path $User "build"
$RawDirectory = Join-Path $WorkRoot "raw"
$Marker = Join-Path $WorkRoot ".yune-m59-cangjie5-capture-root"
$MarkerText = "created-by=capture-upstream-cangjie5.ps1`n"
$ProbeSource = Join-Path $RepoRoot "scripts\oracle-rime-probe.cs"
$Curator = Join-Path $RepoRoot "scripts\curate-upstream-cangjie5.py"
$RimeDll = Join-Path $Extract "dist\lib\rime.dll"
$RimeDeployer = Join-Path $Extract "dist\bin\rime_deployer.exe"
$RimeHeader = Join-Path $Extract "dist\include\rime_api.h"
$UpstreamOpenCc = Join-Path $Extract "share\opencc"

$Inputs = @(
    "hwmvsqtt",
    "ebcnyripm",
    "takohaeosk",
    "hwmvs",
    "qtt",
    "ebcn",
    "yripm",
    "tak",
    "oha",
    "eosk",
    "hdaetcu",
    "lyk"
)
$Modules = [string[]]@("default")
$PagePolicy = "RimeProbe.Capture all pages; incomplete or non-advancing pagination is fatal"
$Serialization = [ordered]@{
    encoding = "utf-8"
    bom = $false
    line_endings = "lf"
    terminal_newline = "exactly_one"
}
$WritePolicy = "canonical_utf8_no_bom_lf_one_terminal_lf_create_new"
$AdditionalRuntimeOptionPatches = @()

$RequiredRepos = [ordered]@{
    "rime/rime-cangjie" = [ordered]@{
        directory = "rime-cangjie"
        expected_commit = $ExpectedCangjieCommit.ToLowerInvariant()
        expected_tree = "db11cf6ffd382ada3087e9765c0ba2e636a8b68d"
    }
    "rime/rime-prelude" = [ordered]@{
        directory = "rime-prelude"
        expected_commit = $ExpectedPreludeCommit.ToLowerInvariant()
        expected_tree = "d7e128f09ce6b1f920729ef2f848ca1294c9cb31"
    }
    "rime/rime-luna-pinyin" = [ordered]@{
        directory = "rime-luna-pinyin"
        expected_commit = $ExpectedLunaPinyinCommit.ToLowerInvariant()
        expected_tree = "0d5efcb75aa40689bf3be210a4f056db6d77b49d"
    }
    "rime/rime-essay" = [ordered]@{
        directory = "rime-essay"
        expected_commit = $ExpectedEssayCommit.ToLowerInvariant()
        expected_tree = "4769c4ef6c5f93f450c5f36c2c9ac5e6845d37bc"
    }
    "rime/rime-stroke" = [ordered]@{
        directory = "rime-stroke"
        expected_commit = $ExpectedStrokeCommit.ToLowerInvariant()
        expected_tree = "d60c793d8d68154847923f21aa73ba90441dab32"
    }
    "rime/rime-cantonese" = [ordered]@{
        directory = "rime-cantonese"
        expected_commit = $ExpectedCantoneseCommit.ToLowerInvariant()
        expected_tree = "eb193fb80675ffa60df3c32bf24afa7d7f68617a"
    }
    "CanCLID/rime-loengfan" = [ordered]@{
        directory = "rime-loengfan"
        expected_commit = $ExpectedLoengfanCommit.ToLowerInvariant()
        expected_tree = "0858d1087046b8d1c3d36c36000ece5630b09cb3"
    }
}

function Quote-CommandArg([string]$Value) {
    if ($null -eq $Value) {
        return "''"
    }
    return "'" + $Value.Replace("'", "''") + "'"
}

function File-Sha256([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Bytes-Sha256([byte[]]$Bytes) {
    $Hasher = [System.Security.Cryptography.SHA256]::Create()
    try {
        return ([System.BitConverter]::ToString($Hasher.ComputeHash($Bytes))).Replace("-", "").ToLowerInvariant()
    }
    finally {
        $Hasher.Dispose()
    }
}

function Tree-Sha256([string]$Root) {
    $RootFull = [System.IO.Path]::GetFullPath($Root).TrimEnd("\", "/")
    $Rows = Get-ChildItem -LiteralPath $RootFull -Recurse -File -Force |
        Sort-Object { $_.FullName.Substring($RootFull.Length + 1).Replace("\", "/") } |
        ForEach-Object {
            $Relative = $_.FullName.Substring($RootFull.Length + 1).Replace("\", "/")
            "$Relative`t$(File-Sha256 $_.FullName)"
        }
    $Payload = (($Rows -join "`n") + "`n")
    return Bytes-Sha256 ([System.Text.Encoding]::UTF8.GetBytes($Payload))
}

function ConvertTo-CanonicalJsonText([object]$Value) {
    $Json = $Value | ConvertTo-Json -Depth 100
    $Json = $Json.Replace("`r`n", "`n").Replace("`r", "`n")
    return $Json.TrimEnd([char]10) + "`n"
}

function Write-NewUtf8NoBom([string]$Path, [string]$Text) {
    $Parent = Split-Path -Parent $Path
    if (-not [string]::IsNullOrWhiteSpace($Parent)) {
        New-Item -ItemType Directory -Force -Path $Parent | Out-Null
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

function Assert-CanonicalJsonFile([string]$Path, [string]$Label) {
    $Bytes = [System.IO.File]::ReadAllBytes($Path)
    if ($Bytes.Length -eq 0) {
        throw "$Label is empty."
    }
    if ($Bytes.Length -ge 3 -and $Bytes[0] -eq 0xEF -and $Bytes[1] -eq 0xBB -and $Bytes[2] -eq 0xBF) {
        throw "$Label must not contain a UTF-8 BOM."
    }
    if ($Bytes -contains 0x00 -or $Bytes -contains 0x0D) {
        throw "$Label must use canonical UTF-8 LF bytes."
    }
    if ($Bytes[$Bytes.Length - 1] -ne 0x0A -or
        ($Bytes.Length -gt 1 -and $Bytes[$Bytes.Length - 2] -eq 0x0A)) {
        throw "$Label must have exactly one terminal LF."
    }
    $Utf8 = [System.Text.UTF8Encoding]::new($false, $true)
    $Text = $Utf8.GetString($Bytes)
    $null = $Text | ConvertFrom-Json
}

function Get-CanonicalCangjiePath([string]$Path) {
    if (-not ("CangjieCaptureFinalPath" -as [type])) {
        Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;
using Microsoft.Win32.SafeHandles;

public static class CangjieCaptureFinalPath {
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
    $Canonical = [CangjieCaptureFinalPath]::Resolve($Existing)
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

function Test-CangjiePathWithinOrEqual([string]$Candidate, [string]$Root) {
    if ([string]::Equals($Candidate, $Root, [System.StringComparison]::OrdinalIgnoreCase)) {
        return $true
    }
    $Prefix = $Root.TrimEnd("\", "/") + [System.IO.Path]::DirectorySeparatorChar
    return $Candidate.StartsWith($Prefix, [System.StringComparison]::OrdinalIgnoreCase)
}

function Assert-CangjiePublicationPathsUnchanged(
    [string]$RawOutput,
    [string]$Output,
    [string]$ExpectedCanonicalRaw,
    [string]$ExpectedCanonicalOutput,
    [bool]$RawMustExist
) {
    if ((Test-Path -LiteralPath $RawOutput) -ne $RawMustExist) {
        throw "Raw output existence changed before the declared publication phase."
    }
    if (Test-Path -LiteralPath $Output) {
        throw "Curated output appeared before create-new publication."
    }
    $CanonicalRaw = Get-CanonicalCangjiePath $RawOutput
    $CanonicalOutput = Get-CanonicalCangjiePath $Output
    if (-not [string]::Equals($CanonicalRaw, $ExpectedCanonicalRaw, [System.StringComparison]::OrdinalIgnoreCase) -or
        -not [string]::Equals($CanonicalOutput, $ExpectedCanonicalOutput, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Raw or curated publication path changed after canonical preflight."
    }
}

function Assert-CangjieCapturePathPreflight(
    [string]$RawOutput,
    [string]$Output,
    [string]$WorkRoot,
    [string]$OracleRoot,
    [string]$Extract,
    [string]$SchemaRoot,
    [string[]]$SourceRepositories,
    [hashtable]$ProtectedFiles
) {
    foreach ($FreshPath in @($RawOutput, $Output, $WorkRoot)) {
        if (Test-Path -LiteralPath $FreshPath) {
            throw "Capture output/work path must not already exist: $FreshPath"
        }
    }
    $CanonicalRaw = Get-CanonicalCangjiePath $RawOutput
    $CanonicalOutput = Get-CanonicalCangjiePath $Output
    $CanonicalWork = Get-CanonicalCangjiePath $WorkRoot
    $CanonicalOracle = Get-CanonicalCangjiePath $OracleRoot
    $CanonicalExtract = Get-CanonicalCangjiePath $Extract
    $CanonicalSchemaRoot = Get-CanonicalCangjiePath $SchemaRoot
    $ExpectedCanonicalExtract = [System.IO.Path]::GetFullPath(
        (Join-Path $CanonicalOracle "extract")
    ).TrimEnd("\", "/")
    $ExpectedCanonicalSchemaRoot = [System.IO.Path]::GetFullPath(
        (Join-Path $CanonicalOracle "schema-src")
    ).TrimEnd("\", "/")
    if (-not [string]::Equals(
            $CanonicalExtract,
            $ExpectedCanonicalExtract,
            [System.StringComparison]::OrdinalIgnoreCase) -or
        -not [string]::Equals(
            $CanonicalSchemaRoot,
            $ExpectedCanonicalSchemaRoot,
            [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Extract and SchemaRoot must not escape their exact canonical OracleRoot leaves."
    }

    foreach ($Pair in @(
            @($CanonicalRaw, $CanonicalOutput, "RawOutput and Output"),
            @($CanonicalRaw, $CanonicalWork, "RawOutput and WorkRoot"),
            @($CanonicalOutput, $CanonicalWork, "Output and WorkRoot")
        )) {
        if ((Test-CangjiePathWithinOrEqual $Pair[0] $Pair[1]) -or
            (Test-CangjiePathWithinOrEqual $Pair[1] $Pair[0])) {
            throw "$($Pair[2]) must be distinct and non-nested."
        }
    }
    foreach ($EvidencePath in @($CanonicalRaw, $CanonicalOutput)) {
        if (Test-CangjiePathWithinOrEqual $EvidencePath $CanonicalOracle) {
            throw "Raw and curated outputs must not be inside or equal to OracleRoot."
        }
    }
    foreach ($ProtectedRoot in @($CanonicalExtract, $CanonicalSchemaRoot)) {
        if ((Test-CangjiePathWithinOrEqual $CanonicalWork $ProtectedRoot) -or
            (Test-CangjiePathWithinOrEqual $ProtectedRoot $CanonicalWork)) {
            throw "WorkRoot must not alias, contain, or be inside protected oracle inputs."
        }
    }
    if ([string]::Equals($CanonicalWork, $CanonicalOracle, [System.StringComparison]::OrdinalIgnoreCase) -or
        (Test-CangjiePathWithinOrEqual $CanonicalOracle $CanonicalWork)) {
        throw "WorkRoot must not equal or contain OracleRoot."
    }
    foreach ($Repository in $SourceRepositories) {
        $CanonicalRepository = Get-CanonicalCangjiePath $Repository
        if (-not (Test-CangjiePathWithinOrEqual $CanonicalRepository $CanonicalSchemaRoot)) {
            throw "Source repository escaped the canonical schema root: $Repository"
        }
        if ((Test-CangjiePathWithinOrEqual $CanonicalWork $CanonicalRepository) -or
            (Test-CangjiePathWithinOrEqual $CanonicalRepository $CanonicalWork)) {
            throw "WorkRoot must not alias, contain, or be inside a source repository."
        }
    }
    foreach ($Entry in $ProtectedFiles.GetEnumerator()) {
        $CanonicalProtected = Get-CanonicalCangjiePath ([string]$Entry.Value)
        foreach ($EvidencePath in @($CanonicalRaw, $CanonicalOutput)) {
            if ([string]::Equals($EvidencePath, $CanonicalProtected, [System.StringComparison]::OrdinalIgnoreCase)) {
                throw "Evidence output must not alias protected input $($Entry.Key)."
            }
        }
        if ((Test-CangjiePathWithinOrEqual $CanonicalProtected $CanonicalWork) -or
            [string]::Equals($CanonicalProtected, $CanonicalWork, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "WorkRoot must not contain protected input $($Entry.Key)."
        }
    }
    return [pscustomobject]@{
        raw_output = $CanonicalRaw
        output = $CanonicalOutput
        work_root = $CanonicalWork
        oracle_root = $CanonicalOracle
    }
}

function Git-Value([string]$Path, [string]$Expression) {
    $Value = (& git -C $Path rev-parse $Expression).Trim().ToLowerInvariant()
    if ($LASTEXITCODE -ne 0 -or $Value -notmatch '^[0-9a-f]{40}$') {
        throw "Unable to resolve Git identity $Expression for $Path"
    }
    return $Value
}

function Git-State([string]$Path) {
    $Status = @(& git -C $Path status --short)
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to read Git status for $Path"
    }
    return [pscustomobject]@{
        commit = Git-Value $Path "HEAD"
        tree = Git-Value $Path "HEAD^{tree}"
        clean = $Status.Count -eq 0
        status_short = @($Status)
        canonical_path = Get-CanonicalCangjiePath $Path
    }
}

function Assert-GitStateUnchanged([string]$Path, [string]$Label, [object]$Before) {
    $After = Git-State $Path
    if ($After.commit -ne $Before.commit -or
        $After.tree -ne $Before.tree -or
        $After.clean -ne $Before.clean -or
        (@($After.status_short) -join "`n") -cne (@($Before.status_short) -join "`n") -or
        $After.canonical_path -cne $Before.canonical_path) {
        throw "Git source state changed during capture: $Label"
    }
}

function Assert-FileSha256Unchanged(
    [string]$Path,
    [string]$Label,
    [string]$ExpectedSha256
) {
    if ((File-Sha256 $Path) -ne $ExpectedSha256) {
        throw "Captured binary or tool changed during capture: $Label"
    }
}

function Assert-NoReparsePoints([string]$Root, [string]$Label) {
    $RootItem = Get-Item -LiteralPath $Root -Force
    if (($RootItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "$Label root must not be a reparse point: $Root"
    }
    $ReparsePoint = Get-ChildItem -LiteralPath $Root -Recurse -Force |
        Where-Object { ($_.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0 } |
        Select-Object -First 1
    if ($null -ne $ReparsePoint) {
        throw "$Label contains a reparse point: $($ReparsePoint.FullName)"
    }
}

function Convert-ToEvidencePath([string]$Path, [string]$Role) {
    if ([string]::IsNullOrWhiteSpace($Path)) {
        return $null
    }
    $Full = [System.IO.Path]::GetFullPath($Path)
    $Root = $RepoRoot.TrimEnd("\", "/")
    if ($Full.StartsWith($Root + [System.IO.Path]::DirectorySeparatorChar, [System.StringComparison]::OrdinalIgnoreCase)) {
        return $Full.Substring($Root.Length + 1).Replace("\", "/")
    }
    return "external/$Role"
}

function Assert-RepoPathIgnoredIfInternal([string]$Path, [string]$Label) {
    $Full = [System.IO.Path]::GetFullPath($Path)
    $Root = $RepoRoot.TrimEnd("\", "/")
    if (-not $Full.StartsWith($Root + [System.IO.Path]::DirectorySeparatorChar, [System.StringComparison]::OrdinalIgnoreCase)) {
        return
    }
    & git -C $RepoRoot check-ignore -q -- $Full
    if ($LASTEXITCODE -ne 0) {
        throw "$Label inside the Yune repository must be ignored so capture cannot dirty source identity: $Path"
    }
}

function Copy-PinnedRimeData([string]$Source, [string]$Destination) {
    Get-ChildItem -LiteralPath $Source -File -Force |
        Where-Object {
            $_.Name -like "*.yaml" -or
            $_.Name -like "*.dict.yaml" -or
            $_.Name -like "*.txt"
        } |
        ForEach-Object {
            Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $Destination $_.Name)
        }
    $OpenCcSource = Join-Path $Source "opencc"
    if (Test-Path -LiteralPath $OpenCcSource) {
        $OpenCcDestination = Join-Path $Destination "opencc"
        New-Item -ItemType Directory -Path $OpenCcDestination | Out-Null
        Get-ChildItem -LiteralPath $OpenCcSource -Force | ForEach-Object {
            Copy-Item -LiteralPath $_.FullName -Destination $OpenCcDestination -Recurse
        }
    }
}

function Set-PinnedTimestamps([string]$Root) {
    $Pinned = [DateTimeOffset]::FromUnixTimeSeconds(946684800).UtcDateTime.AddMilliseconds(500)
    foreach ($Item in @(Get-Item -LiteralPath $Root -Force) + @(Get-ChildItem -LiteralPath $Root -Recurse -Force)) {
        $Item.LastWriteTimeUtc = $Pinned
        if ($Item.LastWriteTimeUtc.Ticks -ne $Pinned.Ticks) {
            throw "Unable to pin staged timestamp: $($Item.FullName)"
        }
    }
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
    $ExpectedNames = @("ascii_mode", "full_shape", "ascii_punct", "zh_hans")
    if ((@($RuntimeOptions.Keys) -join "`n") -cne ($ExpectedNames -join "`n") -or
        @($RuntimeOptions.Values | Where-Object { $_ -ne $false }).Count -ne 0) {
        throw "RimeProbe capture runtime option policy is not the shared four-false policy."
    }
    $Source = [string][RimeProbe]::CaptureRuntimeOptionsSource
    if ($Source -ne "RimeProbe.CaptureWithIdentity/CaptureRuntimeOptionPolicy") {
        throw "RimeProbe capture runtime option source changed."
    }
    return [pscustomobject]@{
        runtime_options = $RuntimeOptions
        runtime_options_source = $Source
    }
}

$SourceRepositoryPaths = @(
    foreach ($Identity in $RequiredRepos.Values) {
        Join-Path $SchemaRoot $Identity.directory
    }
)
$ProtectedFiles = @{
    RimeDll = $RimeDll
    RimeDeployer = $RimeDeployer
    RimeHeader = $RimeHeader
    ProbeSource = $ProbeSource
    CaptureScript = $PSCommandPath
    Curator = $Curator
}
$PathPreflight = Assert-CangjieCapturePathPreflight `
    $RawOutput `
    $Output `
    $WorkRoot `
    $OracleRoot `
    $Extract `
    $SchemaRoot `
    $SourceRepositoryPaths `
    $ProtectedFiles
$RawOutput = $PathPreflight.raw_output
$Output = $PathPreflight.output
$WorkRoot = $PathPreflight.work_root
Assert-RepoPathIgnoredIfInternal $RawOutput "RawOutput"
Assert-RepoPathIgnoredIfInternal $Output "Output"
Assert-RepoPathIgnoredIfInternal $WorkRoot "WorkRoot"

foreach ($RequiredPath in @($RimeDll, $RimeDeployer, $RimeHeader, $ProbeSource, $Curator) + $SourceRepositoryPaths) {
    if (-not (Test-Path -LiteralPath $RequiredPath)) {
        throw "Missing required upstream Cangjie capture input: $RequiredPath"
    }
}
foreach ($RepositoryPath in $SourceRepositoryPaths) {
    Assert-NoReparsePoints $RepositoryPath "source repository"
}
if (Test-Path -LiteralPath $UpstreamOpenCc) {
    Assert-NoReparsePoints $UpstreamOpenCc "upstream OpenCC"
}

$ToolState = Git-State $RepoRoot
if (-not $ToolState.clean) {
    throw "Refusing canonical Cangjie capture from a dirty Yune source tree."
}
$RepositoryStates = [ordered]@{}
foreach ($Repository in $RequiredRepos.Keys) {
    $Identity = $RequiredRepos[$Repository]
    $RepositoryPath = Join-Path $SchemaRoot $Identity.directory
    $State = Git-State $RepositoryPath
    if (-not $State.clean -or $State.status_short.Count -ne 0) {
        throw "Refusing canonical Cangjie capture from dirty source repository: $Repository"
    }
    if ($State.commit -ne $Identity.expected_commit -or $State.tree -ne $Identity.expected_tree) {
        throw "Pinned source repository identity mismatch: $Repository"
    }
    $RepositoryStates[$Repository] = $State
}

$ActualRimeDllSha256 = File-Sha256 $RimeDll
$ActualRimeDeployerSha256 = File-Sha256 $RimeDeployer
if ($ActualRimeDllSha256 -ne $ExpectedRimeDllSha256.ToLowerInvariant()) {
    throw "Unexpected upstream rime.dll SHA-256."
}
if ($ActualRimeDeployerSha256 -ne $ExpectedRimeDeployerSha256.ToLowerInvariant()) {
    throw "Unexpected upstream rime_deployer.exe SHA-256."
}
$ProbeSha256 = File-Sha256 $ProbeSource
$CaptureScriptSha256 = File-Sha256 $PSCommandPath
$CuratorSha256 = File-Sha256 $Curator

Add-Type -Path $ProbeSource
$RuntimeOptionProvenance = Get-RimeCaptureRuntimeOptionProvenance
$RuntimeOptions = $RuntimeOptionProvenance.runtime_options
$RuntimeOptionsSource = [string]$RuntimeOptionProvenance.runtime_options_source

$ExpectedRepositoryCommits = [ordered]@{}
$ExpectedRepositoryTrees = [ordered]@{}
$SourceRepositoryEvidence = [ordered]@{}
foreach ($Repository in $RequiredRepos.Keys) {
    $ExpectedRepositoryCommits[$Repository] = $RequiredRepos[$Repository].expected_commit
    $ExpectedRepositoryTrees[$Repository] = $RequiredRepos[$Repository].expected_tree
    $SourceRepositoryEvidence[$Repository] = [ordered]@{
        commit = $RepositoryStates[$Repository].commit
        tree = $RepositoryStates[$Repository].tree
        clean = [bool]$RepositoryStates[$Repository].clean
        status_short = @($RepositoryStates[$Repository].status_short)
    }
}

$EffectiveParameters = [ordered]@{
    oracle_root = Convert-ToEvidencePath $OracleRoot "oracle-root"
    cangjie_dir = Convert-ToEvidencePath (Join-Path $SchemaRoot "rime-cangjie") "cangjie-dir"
    essay = Convert-ToEvidencePath (Join-Path $SchemaRoot "rime-essay\essay.txt") "essay"
    raw_output = Convert-ToEvidencePath $RawOutput "raw-output"
    output = Convert-ToEvidencePath $Output "output"
    work_root = if ($WorkRootWasProvided) { Convert-ToEvidencePath $WorkRoot "work-root" } else { "generated_disposable" }
    work_root_source = if ($WorkRootWasProvided) { "explicit" } else { "generated_unique_temp" }
    keep_work_root = $KeepWorkRoot.IsPresent
    inputs = @($Inputs)
    schema_id = "cangjie5"
    modules = @($Modules)
    expected_rime_dll_sha256 = $ExpectedRimeDllSha256.ToLowerInvariant()
    expected_rime_deployer_sha256 = $ExpectedRimeDeployerSha256.ToLowerInvariant()
    expected_repository_commits = $ExpectedRepositoryCommits
    expected_repository_trees = $ExpectedRepositoryTrees
    runtime_options = $RuntimeOptions
    runtime_options_source = $RuntimeOptionsSource
    additional_runtime_option_patches = $AdditionalRuntimeOptionPatches
    page_policy = $PagePolicy
    serialization = $Serialization
    path_serialization_policy = "repo-relative forward-slash paths; external paths replaced with external/<role>"
}

$CaptureInvocation = @(
    "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/capture-upstream-cangjie5.ps1",
    "-OracleRoot $(Quote-CommandArg $EffectiveParameters.oracle_root)",
    "-RawOutput $(Quote-CommandArg $EffectiveParameters.raw_output)",
    "-Output $(Quote-CommandArg $EffectiveParameters.output)"
)
if ($WorkRootWasProvided) {
    $CaptureInvocation += "-WorkRoot $(Quote-CommandArg $EffectiveParameters.work_root)"
}
if ($KeepWorkRoot.IsPresent) {
    $CaptureInvocation += "-KeepWorkRoot"
}
$CaptureInvocation += @(
    "-ExpectedRimeDllSha256 $(Quote-CommandArg $EffectiveParameters.expected_rime_dll_sha256)",
    "-ExpectedRimeDeployerSha256 $(Quote-CommandArg $EffectiveParameters.expected_rime_deployer_sha256)",
    "-ExpectedCangjieCommit $(Quote-CommandArg $ExpectedRepositoryCommits['rime/rime-cangjie'])",
    "-ExpectedPreludeCommit $(Quote-CommandArg $ExpectedRepositoryCommits['rime/rime-prelude'])",
    "-ExpectedLunaPinyinCommit $(Quote-CommandArg $ExpectedRepositoryCommits['rime/rime-luna-pinyin'])",
    "-ExpectedEssayCommit $(Quote-CommandArg $ExpectedRepositoryCommits['rime/rime-essay'])",
    "-ExpectedStrokeCommit $(Quote-CommandArg $ExpectedRepositoryCommits['rime/rime-stroke'])",
    "-ExpectedCantoneseCommit $(Quote-CommandArg $ExpectedRepositoryCommits['rime/rime-cantonese'])",
    "-ExpectedLoengfanCommit $(Quote-CommandArg $ExpectedRepositoryCommits['CanCLID/rime-loengfan'])"
)
$ActualInvocation = $CaptureInvocation -join " "
$CuratorInvocation = @(
    "python -B scripts/curate-upstream-cangjie5.py",
    "--raw-input $(Quote-CommandArg $EffectiveParameters.raw_output)",
    "--output $(Quote-CommandArg $EffectiveParameters.output)",
    "--oracle-root $(Quote-CommandArg $EffectiveParameters.oracle_root)",
    "--cangjie-dir $(Quote-CommandArg $EffectiveParameters.cangjie_dir)",
    "--essay $(Quote-CommandArg $EffectiveParameters.essay)"
) -join " "

$OldPath = $env:PATH
try {
    New-Item -ItemType Directory -Path $WorkRoot, $Shared, $User, $Build, $RawDirectory | Out-Null
    Write-NewUtf8NoBom $Marker $MarkerText

    foreach ($Repository in $RequiredRepos.Keys) {
        $Source = Join-Path $SchemaRoot $RequiredRepos[$Repository].directory
        Copy-PinnedRimeData $Source $Shared
    }
    if (Test-Path -LiteralPath $UpstreamOpenCc) {
        $OpenCcDestination = Join-Path $Shared "opencc"
        if (-not (Test-Path -LiteralPath $OpenCcDestination)) {
            New-Item -ItemType Directory -Path $OpenCcDestination | Out-Null
        }
        Get-ChildItem -LiteralPath $UpstreamOpenCc -Force | ForEach-Object {
            $Destination = Join-Path $OpenCcDestination $_.Name
            if (-not (Test-Path -LiteralPath $Destination)) {
                Copy-Item -LiteralPath $_.FullName -Destination $OpenCcDestination -Recurse
            }
        }
    }
    $SourceSharedTreeSha256 = Tree-Sha256 $Shared
    $DefaultCustom = Join-Path $Shared "default.custom.yaml"
    Write-NewUtf8NoBom $DefaultCustom "patch:`n  schema_list:`n    - schema: cangjie5`n"
    Set-PinnedTimestamps $Shared
    $StagedSharedTreeSha256 = Tree-Sha256 $Shared

    $env:PATH = (Join-Path $Extract "dist\lib") + ";" + (Join-Path $Extract "dist\bin") + ";" + $OldPath
    & $RimeDeployer --build $User $Shared $Build
    if ($LASTEXITCODE -ne 0) {
        throw "rime_deployer.exe --build failed with exit code $LASTEXITCODE"
    }
    $Cases = [RimeProbe]::Capture($Shared, $User, $Build, "cangjie5", $Modules, [string[]]$Inputs)
    if ($Cases.Count -ne $Inputs.Count) {
        throw "Oracle capture returned $($Cases.Count) cases for $($Inputs.Count) inputs."
    }
    for ($Index = 0; $Index -lt $Inputs.Count; $Index++) {
        $Case = $Cases[$Index]
        if ([string]$Case["input"] -cne $Inputs[$Index] -or [string]$Case["schema_id"] -cne "cangjie5") {
            throw "Oracle capture did not preserve the declared 12-input/schema order."
        }
        if (-not [bool]$Case["captured_all_pages"]) {
            $Reason = if ($Case.ContainsKey("pagination_error")) { $Case["pagination_error"] } else { "unknown" }
            throw "Capture for input '$($Inputs[$Index])' did not capture all pages: $Reason"
        }
    }

    $SharedTreeAfterCapture = Tree-Sha256 $Shared
    if ($SharedTreeAfterCapture -ne $StagedSharedTreeSha256) {
        throw "Staged shared tree changed during Cangjie capture."
    }
    $DeployedBuildTreeSha256 = Tree-Sha256 $Build

    Assert-GitStateUnchanged $RepoRoot "yune" $ToolState
    foreach ($Repository in $RequiredRepos.Keys) {
        Assert-GitStateUnchanged `
            (Join-Path $SchemaRoot $RequiredRepos[$Repository].directory) `
            $Repository `
            $RepositoryStates[$Repository]
    }
    Assert-FileSha256Unchanged $RimeDll "rime.dll" $ActualRimeDllSha256
    Assert-FileSha256Unchanged $RimeDeployer "rime_deployer.exe" $ActualRimeDeployerSha256
    Assert-FileSha256Unchanged $ProbeSource "oracle-rime-probe.cs" $ProbeSha256
    Assert-FileSha256Unchanged $PSCommandPath "capture-upstream-cangjie5.ps1" $CaptureScriptSha256
    Assert-FileSha256Unchanged $Curator "curate-upstream-cangjie5.py" $CuratorSha256
    Assert-CangjiePublicationPathsUnchanged `
        $RawOutput `
        $Output `
        $PathPreflight.raw_output `
        $PathPreflight.output `
        $false

    $PageSizes = @($Cases | ForEach-Object { [int]$_["page_size"] } | Sort-Object -Unique)
    $OwnerTargetSpecs = @(
        [ordered]@{
            input = "hwmvsqtt"
            target_codepoints = "U+7CB5 U+62FC"
            atomic_codes = @("hwmvs", "qtt")
            declaration_source = "owner-signed D-47/D-48 Cangjie composition row"
        },
        [ordered]@{
            input = "ebcnyripm"
            target_codepoints = "U+6E2C U+8A66"
            atomic_codes = @("ebcn", "yripm")
            declaration_source = "owner-signed D-47/D-48 Cangjie composition row"
        },
        [ordered]@{
            input = "takohaeosk"
            target_codepoints = "U+83AB U+4F2F U+6D22"
            atomic_codes = @("tak", "oha", "eosk")
            declaration_source = "owner-signed D-47/D-48 Cangjie composition row"
        }
    )
    $ControlTargetSpecs = @(
        [ordered]@{
            input = "hdaetcu"
            target_codepoints = "U+9999 U+6E2F"
            candidate_index = 0
        },
        [ordered]@{
            input = "lyk"
            target_codepoints = "U+4E2D U+6587"
            candidate_index = 1
        }
    )
    $OutputProvenance = [ordered]@{
        raw = [ordered]@{
            path = $EffectiveParameters.raw_output
            existed_before_capture = $false
            write_policy = $WritePolicy
            generated_by = "scripts/capture-upstream-cangjie5.ps1"
        }
        curated = [ordered]@{
            path = $EffectiveParameters.output
            existed_before_capture = $false
            write_policy = $WritePolicy
            generated_by = "scripts/curate-upstream-cangjie5.py"
        }
    }
    $Evidence = [ordered]@{
        milestone = "M59"
        task = "D-48 item 2: cangjie5 order-parity onboarding"
        status = "raw_cangjie5_capture_complete"
        canonical = $true
        capture = [ordered]@{
            engine = "rime/librime"
            version = "1.17.0"
            librime_commit = "33e78140250125871856cdc5b42ddc6a5fcd3cd4"
            source_commit = $ToolState.commit
            source_tree = $ToolState.tree
            source_clean = [bool]$ToolState.clean
            source_status_short = @($ToolState.status_short)
            schema_id = "cangjie5"
            modules = @($Modules)
            inputs = @($Inputs)
            input_count = $Inputs.Count
            inputs_source = "fixed_m59_cangjie_order_lane"
            page_sizes_observed = @($PageSizes)
            captured_all_pages = $true
            page_policy = $PagePolicy
            runtime_options = $RuntimeOptions
            runtime_options_source = $RuntimeOptionsSource
            additional_runtime_option_patches = $AdditionalRuntimeOptionPatches
            rime_dll_sha256 = $ActualRimeDllSha256
            rime_deployer_sha256 = $ActualRimeDeployerSha256
            source_repositories = $SourceRepositoryEvidence
            tool_hashes = [ordered]@{
                capture_script_sha256 = $CaptureScriptSha256
                probe_sha256 = $ProbeSha256
                curator_sha256 = $CuratorSha256
            }
            source_shared_tree_sha256 = $SourceSharedTreeSha256
            staged_shared_tree_sha256 = $StagedSharedTreeSha256
            deployed_build_tree_sha256 = $DeployedBuildTreeSha256
            commands = [ordered]@{
                deploy = "rime_deployer.exe --build disposable/user disposable/shared disposable/user/build"
                capture = $ActualInvocation
                curate = $CuratorInvocation
            }
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
            yune_facing_schema_id = "cangjie5"
            source_repo = "rime/rime-cangjie"
            source_commit = $ExpectedRepositoryCommits["rime/rime-cangjie"]
            source_tree = $ExpectedRepositoryTrees["rime/rime-cangjie"]
            dependency_commits = $ExpectedRepositoryCommits
            dependency_trees = $ExpectedRepositoryTrees
            note = "Upstream rime-cangjie validation lane; this is not the shipped product Cangjie dictionary."
        }
        options = [ordered]@{
            runtime_options = $RuntimeOptions
            runtime_options_source = $RuntimeOptionsSource
            additional_runtime_option_patches = $AdditionalRuntimeOptionPatches
            custom_yaml = "default.custom.yaml only selects cangjie5"
            page_sizes_observed = @($PageSizes)
        }
        owner_target_specs = $OwnerTargetSpecs
        control_target_specs = $ControlTargetSpecs
        inputs = @($Inputs)
        cases = @($Cases)
    }

    $EvidenceJson = ConvertTo-CanonicalJsonText $Evidence
    Assert-GitStateUnchanged $RepoRoot "yune immediately before raw output" $ToolState
    foreach ($Repository in $RequiredRepos.Keys) {
        Assert-GitStateUnchanged `
            (Join-Path $SchemaRoot $RequiredRepos[$Repository].directory) `
            "$Repository immediately before raw output" `
            $RepositoryStates[$Repository]
    }
    Assert-FileSha256Unchanged $RimeDll "rime.dll immediately before raw output" $ActualRimeDllSha256
    Assert-FileSha256Unchanged $RimeDeployer "rime_deployer.exe immediately before raw output" $ActualRimeDeployerSha256
    Assert-FileSha256Unchanged $ProbeSource "oracle-rime-probe.cs immediately before raw output" $ProbeSha256
    Assert-FileSha256Unchanged $PSCommandPath "capture script immediately before raw output" $CaptureScriptSha256
    Assert-FileSha256Unchanged $Curator "curator immediately before raw output" $CuratorSha256
    if ((Tree-Sha256 $Shared) -ne $StagedSharedTreeSha256) {
        throw "Staged shared tree changed immediately before raw output."
    }
    Assert-CangjiePublicationPathsUnchanged `
        $RawOutput `
        $Output `
        $PathPreflight.raw_output `
        $PathPreflight.output `
        $false
    Write-NewUtf8NoBom $RawOutput $EvidenceJson
    Assert-CanonicalJsonFile $RawOutput "raw Cangjie oracle capture"
    $RawOutputSha256 = File-Sha256 $RawOutput

    Assert-CangjiePublicationPathsUnchanged `
        $RawOutput `
        $Output `
        $PathPreflight.raw_output `
        $PathPreflight.output `
        $true

    & python -B $Curator `
        --raw-input $RawOutput `
        --output $Output `
        --oracle-root $OracleRoot `
        --cangjie-dir (Join-Path $SchemaRoot "rime-cangjie") `
        --essay (Join-Path $SchemaRoot "rime-essay\essay.txt")
    if ($LASTEXITCODE -ne 0) {
        throw "Cangjie source-slice curation failed with exit code $LASTEXITCODE"
    }
    Assert-CanonicalJsonFile $Output "curated Cangjie oracle capture"
    Assert-FileSha256Unchanged $RawOutput "untouched raw Cangjie oracle capture" $RawOutputSha256
    if (-not [string]::Equals(
            (Get-CanonicalCangjiePath $RawOutput),
            $PathPreflight.raw_output,
            [System.StringComparison]::OrdinalIgnoreCase) -or
        -not [string]::Equals(
            (Get-CanonicalCangjiePath $Output),
            $PathPreflight.output,
            [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Raw or curated publication path changed during curation."
    }
    Assert-GitStateUnchanged $RepoRoot "yune after curation" $ToolState
    foreach ($Repository in $RequiredRepos.Keys) {
        Assert-GitStateUnchanged `
            (Join-Path $SchemaRoot $RequiredRepos[$Repository].directory) `
            "$Repository after curation" `
            $RepositoryStates[$Repository]
    }
    Assert-FileSha256Unchanged $RimeDll "rime.dll after curation" $ActualRimeDllSha256
    Assert-FileSha256Unchanged $RimeDeployer "rime_deployer.exe after curation" $ActualRimeDeployerSha256
    Assert-FileSha256Unchanged $ProbeSource "oracle-rime-probe.cs after curation" $ProbeSha256
    Assert-FileSha256Unchanged $PSCommandPath "capture script after curation" $CaptureScriptSha256
    Assert-FileSha256Unchanged $Curator "curator after curation" $CuratorSha256
    Write-Host "Wrote untouched raw Cangjie capture to $RawOutput"
    Write-Host "Wrote create-new curated Cangjie capture to $Output"
}
finally {
    $env:PATH = $OldPath
    if (-not $KeepWorkRoot.IsPresent -and (Test-Path -LiteralPath $WorkRoot)) {
        $ResolvedWorkRoot = Get-CanonicalCangjiePath $WorkRoot
        if (-not [string]::Equals($ResolvedWorkRoot, $PathPreflight.work_root, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Refusing to remove a capture work root whose canonical path changed."
        }
        if (-not (Test-Path -LiteralPath $Marker)) {
            throw "Refusing to remove an unmarked Cangjie capture work root: $ResolvedWorkRoot"
        }
        if ([System.IO.File]::ReadAllText($Marker, [System.Text.Encoding]::UTF8) -ne $MarkerText) {
            throw "Refusing to remove a Cangjie capture work root with an invalid marker: $ResolvedWorkRoot"
        }
        $RootPath = [System.IO.Path]::GetPathRoot($ResolvedWorkRoot)
        if ($ResolvedWorkRoot.Length -le $RootPath.Length -or
            [string]::Equals($ResolvedWorkRoot, $RepoRoot, [System.StringComparison]::OrdinalIgnoreCase) -or
            [string]::Equals($ResolvedWorkRoot, $OracleRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Refusing to remove unsafe Cangjie capture work root: $ResolvedWorkRoot"
        }
        Remove-Item -LiteralPath $ResolvedWorkRoot -Recurse -Force -ErrorAction Stop
    }
}
