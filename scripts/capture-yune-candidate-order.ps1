# Capture complete Yune candidate lists through the librime-shaped native ABI.
# The supplied DLL is copied as rime.dll into an isolated work root, deploys a
# copied shared-data tree, then runs the same all-pages RimeProbe.Capture used by
# the upstream oracle scripts. The output embeds the raw cases plus reproducible
# hashes; no DLL or deployed artifact belongs in tracked evidence.
param(
    [Parameter(Mandatory = $true)]
    [string]$YuneDll,
    [Parameter(Mandatory = $true)]
    [string]$SharedDataDir,
    [Parameter(Mandatory = $true)]
    [string]$SchemaId,
    [Parameter(Mandatory = $true)]
    [string]$OracleCapture,
    [Parameter(Mandatory = $true)]
    [string]$Output,
    [string[]]$Inputs,
    [string]$DefaultYamlOverlay,
    [switch]$NarrowSchemaList,
    [string]$WorkRoot,
    [string]$ExpectedYuneDllSha256,
    [switch]$AllowDirty,
    [switch]$KeepWorkRoot
)

$ErrorActionPreference = "Stop"
$RepoRoot = [System.IO.Path]::GetFullPath((Resolve-Path (Join-Path $PSScriptRoot "..")).Path)
$ProbeSource = Join-Path $RepoRoot "scripts\oracle-rime-probe.cs"

function Full-ExistingPath([string]$Path, [string]$Label) {
    if (-not (Test-Path -LiteralPath $Path)) {
        throw "Missing $Label`: $Path"
    }
    return [System.IO.Path]::GetFullPath((Resolve-Path -LiteralPath $Path).Path)
}

function Evidence-Path([string]$Path) {
    if ([string]::IsNullOrWhiteSpace($Path)) {
        return $null
    }
    $Full = [System.IO.Path]::GetFullPath($Path)
    $Root = $RepoRoot.TrimEnd("\", "/")
    if ($Full.StartsWith($Root + [System.IO.Path]::DirectorySeparatorChar, [System.StringComparison]::OrdinalIgnoreCase)) {
        return $Full.Substring($Root.Length + 1).Replace("\", "/")
    }
    return $Full.Replace("\", "/")
}

function Quote-CommandArg([string]$Value) {
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

function Write-Utf8NoBom([string]$Path, [string]$Text) {
    $Parent = Split-Path -Parent $Path
    if (-not [string]::IsNullOrWhiteSpace($Parent)) {
        New-Item -ItemType Directory -Force -Path $Parent | Out-Null
    }
    [System.IO.File]::WriteAllText($Path, $Text, [System.Text.UTF8Encoding]::new($false))
}

function Get-CanonicalWindowsPath([string]$Path) {
    if (-not ("YuneCaptureFinalPath" -as [type])) {
        Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;
using Microsoft.Win32.SafeHandles;

public static class YuneCaptureFinalPath {
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
    $RootRelative = $Full.Substring($Root.Length)
    if ($RootRelative.Contains(":")) {
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
    $Canonical = [YuneCaptureFinalPath]::Resolve($Existing)
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

function Test-CanonicalPathWithinOrEqual([string]$Candidate, [string]$Root) {
    if ([string]::Equals($Candidate, $Root, [System.StringComparison]::OrdinalIgnoreCase)) {
        return $true
    }
    $RootPrefix = $Root.TrimEnd("\", "/") + [System.IO.Path]::DirectorySeparatorChar
    return $Candidate.StartsWith($RootPrefix, [System.StringComparison]::OrdinalIgnoreCase)
}

function Assert-CapturePathPreflight(
    [string]$Output,
    [string]$WorkRoot,
    [string]$SharedDataDir,
    [string]$YuneDll,
    [string]$OracleCapture,
    [string]$DefaultYamlOverlay,
    [string]$ProbeSource,
    [string]$CaptureScript,
    [bool]$KeepWorkRoot
) {
    if (Test-Path -LiteralPath $Output) {
        throw "Output must not already exist: $Output"
    }
    if (Test-Path -LiteralPath $WorkRoot) {
        throw "Capture work root already exists; refusing to reuse or delete it: $WorkRoot"
    }

    $CanonicalOutput = Get-CanonicalWindowsPath $Output
    $CanonicalWorkRoot = Get-CanonicalWindowsPath $WorkRoot
    $CanonicalSharedData = Get-CanonicalWindowsPath $SharedDataDir
    $ProtectedPaths = [ordered]@{
        YuneDll = $YuneDll
        OracleCapture = $OracleCapture
        ProbeSource = $ProbeSource
        CaptureScript = $CaptureScript
    }
    if (-not [string]::IsNullOrWhiteSpace($DefaultYamlOverlay)) {
        $ProtectedPaths["DefaultYamlOverlay"] = $DefaultYamlOverlay
    }
    foreach ($Entry in $ProtectedPaths.GetEnumerator()) {
        $CanonicalProtected = Get-CanonicalWindowsPath ([string]$Entry.Value)
        if ([string]::Equals(
                $CanonicalOutput,
                $CanonicalProtected,
                [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Output must not alias protected input $($Entry.Key): $Output"
        }
    }
    if (Test-CanonicalPathWithinOrEqual $CanonicalOutput $CanonicalSharedData) {
        throw "Output must not be inside SharedDataDir: $Output"
    }
    if (Test-CanonicalPathWithinOrEqual $CanonicalWorkRoot $CanonicalSharedData) {
        throw "WorkRoot must not be inside SharedDataDir: $WorkRoot"
    }
    if (Test-CanonicalPathWithinOrEqual $CanonicalWorkRoot $CanonicalOutput) {
        throw "WorkRoot must not be inside or equal to Output."
    }
    $OutputUnderWorkRoot = Test-CanonicalPathWithinOrEqual $CanonicalOutput $CanonicalWorkRoot
    if ($OutputUnderWorkRoot -and -not $KeepWorkRoot) {
        throw "Output is inside the disposable work root; pass -KeepWorkRoot or choose an external output path."
    }
    return [pscustomobject]@{
        output = $CanonicalOutput
        work_root = $CanonicalWorkRoot
        shared_data_dir = $CanonicalSharedData
        output_under_work_root = $OutputUnderWorkRoot
    }
}

function Get-TopLevelSchemaList([string]$Path) {
    $Utf8 = [System.Text.UTF8Encoding]::new($false, $true)
    $Lines = [System.IO.File]::ReadAllLines($Path, $Utf8)
    $SchemaListIndexes = @(
        for ($Index = 0; $Index -lt $Lines.Count; $Index++) {
            if ($Lines[$Index] -match '^schema_list:\s*(?:#.*)?$') {
                $Index
            }
        }
    )
    if ($SchemaListIndexes.Count -ne 1) {
        throw "default.yaml must contain exactly one top-level schema_list block."
    }
    $Entries = New-Object System.Collections.Generic.List[string]
    $SeenEntries = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
    for ($Index = $SchemaListIndexes[0] + 1; $Index -lt $Lines.Count; $Index++) {
        $Line = $Lines[$Index]
        if ($Line -match '^\s*$' -or $Line -match '^\s*#') {
            continue
        }
        if ($Line -match '^\S') {
            break
        }
        if ($Line -notmatch '^[ \t]+-\s+schema:\s*(?<schema>[A-Za-z0-9_][A-Za-z0-9_.-]*)\s*(?:#.*)?$') {
            throw "default.yaml schema_list must contain only plain schema identifiers."
        }
        if (-not $SeenEntries.Add($Matches.schema)) {
            throw "default.yaml schema_list contains duplicate schema '$($Matches.schema)'."
        }
        $Entries.Add($Matches.schema)
    }
    if ($Entries.Count -eq 0) {
        throw "default.yaml schema_list must contain at least one schema."
    }
    return @($Entries)
}

function Resolve-SchemaListNarrowing(
    [string[]]$Entries,
    [string]$ExpectedSchemaId,
    [bool]$NarrowSwitchUsed,
    [bool]$OverlayUsed
) {
    if ($NarrowSwitchUsed -and $OverlayUsed) {
        throw "Schema-list narrowing cannot use both the generated switch and an overlay."
    }
    $Narrowed = $Entries.Count -eq 1 -and
        [string]::Equals($Entries[0], $ExpectedSchemaId, [System.StringComparison]::Ordinal)
    if ($NarrowSwitchUsed -and -not $Narrowed) {
        throw "Generated schema-list narrowing did not produce exactly '$ExpectedSchemaId'."
    }
    $Source = if (-not $Narrowed) {
        "none"
    }
    elseif ($NarrowSwitchUsed) {
        "generated_narrow_schema_list_switch"
    }
    elseif ($OverlayUsed) {
        "default_yaml_overlay"
    }
    else {
        "source_default_yaml"
    }
    return [pscustomobject]@{
        schema_list_narrowed = $Narrowed
        narrow_schema_list_switch_used = $NarrowSwitchUsed
        schema_list_narrowing_source = $Source
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

function Copy-Tree([string]$Source, [string]$Destination) {
    New-Item -ItemType Directory -Force -Path $Destination | Out-Null
    Get-ChildItem -LiteralPath $Source -Force | ForEach-Object {
        Copy-Item -LiteralPath $_.FullName -Destination $Destination -Recurse -Force
    }
}

function Inputs-FromOracle([object]$Oracle) {
    if ($null -ne $Oracle.cases -and @($Oracle.cases).Count -gt 0) {
        return @($Oracle.cases | ForEach-Object { [string]$_.input })
    }
    throw "Oracle capture must contain a non-empty cases array."
}

$YuneDll = Full-ExistingPath $YuneDll "Yune DLL"
$SharedDataDir = Full-ExistingPath $SharedDataDir "shared-data directory"
$OracleCapture = Full-ExistingPath $OracleCapture "oracle capture"
$ProbeSource = Full-ExistingPath $ProbeSource "Rime probe"
$Output = [System.IO.Path]::GetFullPath($Output)
if ($SchemaId -notmatch '^[A-Za-z0-9_][A-Za-z0-9_.-]*$') {
    throw "SchemaId must be a logical schema identifier, not a path."
}
if (-not [string]::IsNullOrWhiteSpace($DefaultYamlOverlay)) {
    $DefaultYamlOverlay = Full-ExistingPath $DefaultYamlOverlay "default.yaml overlay"
}
if ($DefaultYamlOverlay -and $NarrowSchemaList.IsPresent) {
    throw "Use either -DefaultYamlOverlay or -NarrowSchemaList, not both."
}

$RepoHead = (& git -C $RepoRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $RepoHead.Length -ne 40) {
    throw "Unable to resolve the Yune source commit."
}
$RepoStatus = @(& git -C $RepoRoot status --short)
if ($LASTEXITCODE -ne 0) {
    throw "Unable to read the Yune worktree status."
}
if ($RepoStatus.Count -gt 0 -and -not $AllowDirty.IsPresent) {
    throw "Refusing a release evidence capture from a dirty worktree. Re-run with -AllowDirty only for a diagnostic capture."
}

$YuneDllSha256 = File-Sha256 $YuneDll
if (-not [string]::IsNullOrWhiteSpace($ExpectedYuneDllSha256) -and
    $YuneDllSha256 -ne $ExpectedYuneDllSha256.ToLowerInvariant()) {
    throw "Unexpected Yune DLL SHA-256. Expected $ExpectedYuneDllSha256, got $YuneDllSha256."
}

$Oracle = Get-Content -LiteralPath $OracleCapture -Raw -Encoding UTF8 | ConvertFrom-Json
$InputsWereProvided = $PSBoundParameters.ContainsKey("Inputs") -and $null -ne $Inputs -and $Inputs.Count -gt 0
$OracleInputs = Inputs-FromOracle $Oracle
$OracleInputSet = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
foreach ($OracleInput in $OracleInputs) {
    if ([string]::IsNullOrWhiteSpace($OracleInput) -or -not $OracleInputSet.Add($OracleInput)) {
        throw "Oracle capture inputs must be non-empty and unique."
    }
}
if ($null -eq $Inputs -or $Inputs.Count -eq 0) {
    $Inputs = $OracleInputs
}
$NormalizedInputs = New-Object System.Collections.Generic.List[string]
foreach ($InputValue in @($Inputs)) {
    foreach ($Part in ([string]$InputValue).Split(",")) {
        $Normalized = $Part.Trim()
        if (-not [string]::IsNullOrWhiteSpace($Normalized)) {
            $NormalizedInputs.Add($Normalized)
        }
    }
}
$Inputs = @($NormalizedInputs)
$InputSet = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
if ($Inputs.Count -eq 0 -or @($Inputs | Where-Object { -not $InputSet.Add($_) }).Count -gt 0) {
    throw "Capture inputs must be non-empty and unique."
}
foreach ($InputValue in $Inputs) {
    if (-not $OracleInputSet.Contains($InputValue)) {
        throw "Capture input '$InputValue' is absent from the oracle capture."
    }
}

$WorkRootWasProvided = -not [string]::IsNullOrWhiteSpace($WorkRoot)
if ([string]::IsNullOrWhiteSpace($WorkRoot)) {
    $WorkRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("yune-m59-candidate-capture-" + [guid]::NewGuid().ToString("N"))
}
$WorkRoot = [System.IO.Path]::GetFullPath($WorkRoot)
$PathPreflight = Assert-CapturePathPreflight `
    $Output `
    $WorkRoot `
    $SharedDataDir `
    $YuneDll `
    $OracleCapture `
    $DefaultYamlOverlay `
    $ProbeSource `
    $PSCommandPath `
    $KeepWorkRoot.IsPresent

$NarrowSchemaListSwitchUsed = $NarrowSchemaList.IsPresent
$DefaultYamlOverlayUsed = -not [string]::IsNullOrWhiteSpace($DefaultYamlOverlay)
$EffectiveParameters = [ordered]@{
    yune_dll = Evidence-Path $YuneDll
    shared_data_dir = Evidence-Path $SharedDataDir
    schema_id = $SchemaId
    oracle_capture = Evidence-Path $OracleCapture
    output = Evidence-Path $Output
    inputs = @($Inputs)
    inputs_source = if ($InputsWereProvided) { "explicit" } else { "oracle_cases" }
    default_yaml_overlay = Evidence-Path $DefaultYamlOverlay
    narrow_schema_list = $NarrowSchemaListSwitchUsed
    schema_list_narrowed = $null
    narrow_schema_list_switch_used = $NarrowSchemaListSwitchUsed
    schema_list_narrowing_source = $null
    runtime_options = $null
    runtime_options_source = $null
    work_root = if ($WorkRootWasProvided) { Evidence-Path $WorkRoot } else { "generated_disposable" }
    expected_yune_dll_sha256 = if ($ExpectedYuneDllSha256) { $ExpectedYuneDllSha256.ToLowerInvariant() } else { $null }
    allow_dirty = $AllowDirty.IsPresent
    keep_work_root = $KeepWorkRoot.IsPresent
}
$Invocation = @(
    "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/capture-yune-candidate-order.ps1",
    "-YuneDll $(Quote-CommandArg $EffectiveParameters.yune_dll)",
    "-SharedDataDir $(Quote-CommandArg $EffectiveParameters.shared_data_dir)",
    "-SchemaId $(Quote-CommandArg $SchemaId)",
    "-OracleCapture $(Quote-CommandArg $EffectiveParameters.oracle_capture)",
    "-Output $(Quote-CommandArg $EffectiveParameters.output)"
)
if ($InputsWereProvided) { $Invocation += "-Inputs $(Quote-CommandArg ($Inputs -join ','))" }
if ($DefaultYamlOverlay) { $Invocation += "-DefaultYamlOverlay $(Quote-CommandArg $EffectiveParameters.default_yaml_overlay)" }
if ($NarrowSchemaList.IsPresent) { $Invocation += "-NarrowSchemaList" }
if ($WorkRootWasProvided) { $Invocation += "-WorkRoot $(Quote-CommandArg $EffectiveParameters.work_root)" }
if ($ExpectedYuneDllSha256) { $Invocation += "-ExpectedYuneDllSha256 $(Quote-CommandArg $EffectiveParameters.expected_yune_dll_sha256)" }
if ($AllowDirty.IsPresent) { $Invocation += "-AllowDirty" }
if ($KeepWorkRoot.IsPresent) { $Invocation += "-KeepWorkRoot" }
$ActualInvocation = $Invocation -join " "

$Shared = Join-Path $WorkRoot "shared"
$User = Join-Path $WorkRoot "user"
$Build = Join-Path $User "build"
$Bin = Join-Path $WorkRoot "bin"
$Marker = Join-Path $WorkRoot ".yune-m59-candidate-capture-root"
$MarkerText = "created-by=capture-yune-candidate-order.ps1`n"
$OldPath = $env:PATH

try {
    New-Item -ItemType Directory -Force -Path $WorkRoot, $User, $Build, $Bin | Out-Null
    Write-Utf8NoBom $Marker $MarkerText
    Copy-Tree $SharedDataDir $Shared
    if (-not [string]::IsNullOrWhiteSpace($DefaultYamlOverlay)) {
        Copy-Item -LiteralPath $DefaultYamlOverlay -Destination (Join-Path $Shared "default.yaml") -Force
    }
    if ($NarrowSchemaList.IsPresent) {
        $DefaultYaml = Join-Path $Shared "default.yaml"
        if (-not (Test-Path -LiteralPath $DefaultYaml)) {
            throw "-NarrowSchemaList requires shared/default.yaml."
        }
        $DefaultText = [System.IO.File]::ReadAllText($DefaultYaml, [System.Text.Encoding]::UTF8)
        $SchemaListPattern = "(?ms)^schema_list:\s*\r?\n(?:[ \t]+-\s+schema:\s*[^\r\n]+\r?\n)+"
        $SchemaListRegex = [System.Text.RegularExpressions.Regex]::new($SchemaListPattern)
        $Narrowed = $SchemaListRegex.Replace($DefaultText, "schema_list:`n  - schema: $SchemaId`n", 1)
        if ($Narrowed -eq $DefaultText) {
            throw "Unable to narrow schema_list in staged default.yaml."
        }
        Write-Utf8NoBom $DefaultYaml $Narrowed
    }
    $StagedDefaultYaml = Join-Path $Shared "default.yaml"
    if (-not (Test-Path -LiteralPath $StagedDefaultYaml)) {
        throw "Staged shared data must contain default.yaml for schema-list provenance."
    }
    $StagedSchemaList = @(Get-TopLevelSchemaList $StagedDefaultYaml)
    $SchemaListState = Resolve-SchemaListNarrowing `
        $StagedSchemaList `
        $SchemaId `
        $NarrowSchemaListSwitchUsed `
        $DefaultYamlOverlayUsed
    $SchemaListNarrowed = [bool]$SchemaListState.schema_list_narrowed
    $SchemaListNarrowingSource = [string]$SchemaListState.schema_list_narrowing_source
    $EffectiveParameters["schema_list_narrowed"] = $SchemaListNarrowed
    $EffectiveParameters["schema_list_narrowing_source"] = $SchemaListNarrowingSource
    Copy-Item -LiteralPath $YuneDll -Destination (Join-Path $Bin "rime.dll") -Force

    $env:PATH = $Bin + ";" + $OldPath
    Add-Type -Path $ProbeSource
    $RuntimeOptionProvenance = Get-RimeCaptureRuntimeOptionProvenance
    $RuntimeOptions = $RuntimeOptionProvenance.runtime_options
    $RuntimeOptionsSource = [string]$RuntimeOptionProvenance.runtime_options_source
    $EffectiveParameters["runtime_options"] = $RuntimeOptions
    $EffectiveParameters["runtime_options_source"] = $RuntimeOptionsSource
    $Modules = [string[]]@("default")
    $DeployResult = [RimeProbe]::DeployWorkspace($Shared, $User, $Build, $Modules)
    if ($DeployResult -eq 0) {
        throw "Yune RimeDeployWorkspace returned false."
    }
    $Cases = [RimeProbe]::Capture($Shared, $User, $Build, $SchemaId, $Modules, [string[]]$Inputs)
    if ($Cases.Count -ne $Inputs.Count) {
        throw "Yune capture returned $($Cases.Count) cases for $($Inputs.Count) inputs."
    }
    foreach ($Case in $Cases) {
        if (-not $Case["captured_all_pages"]) {
            $Reason = if ($Case.ContainsKey("pagination_error")) { $Case["pagination_error"] } else { "unknown" }
            throw "Yune capture for '$($Case["input"])' is incomplete: $Reason"
        }
    }

    $Evidence = [ordered]@{
        capture = [ordered]@{
            engine = "yune"
            source_commit = $RepoHead
            source_dirty = ($RepoStatus.Count -gt 0)
            source_status_short = @($RepoStatus)
            schema_id = $SchemaId
            modules = @($Modules)
            yune_dll_sha256 = $YuneDllSha256
            probe_sha256 = File-Sha256 $ProbeSource
            capture_script_sha256 = File-Sha256 $PSCommandPath
            oracle_capture_sha256 = File-Sha256 $OracleCapture
            source_shared_tree_sha256 = Tree-Sha256 $SharedDataDir
            staged_shared_tree_sha256 = Tree-Sha256 $Shared
            default_yaml_overlay_sha256 = if ($DefaultYamlOverlay) { File-Sha256 $DefaultYamlOverlay } else { $null }
            schema_list_narrowed = $SchemaListNarrowed
            narrow_schema_list_switch_used = $NarrowSchemaListSwitchUsed
            schema_list_narrowing_source = $SchemaListNarrowingSource
            runtime_options = $RuntimeOptions
            runtime_options_source = $RuntimeOptionsSource
            page_policy = "RimeProbe.Capture all pages; hard failure on non-advancing or incomplete pagination"
            actual_invocation = $ActualInvocation
            effective_parameters = $EffectiveParameters
        }
        inputs = @($Inputs)
        cases = @($Cases)
    }
    Write-Utf8NoBom $Output (($Evidence | ConvertTo-Json -Depth 100) + "`n")
    Write-Output "captured complete Yune candidate order -> $Output"
}
finally {
    $env:PATH = $OldPath
    if (-not $KeepWorkRoot.IsPresent -and (Test-Path -LiteralPath $WorkRoot)) {
        $ResolvedWorkRoot = [System.IO.Path]::GetFullPath($WorkRoot)
        if (-not (Test-Path -LiteralPath $Marker)) {
            throw "Refusing to remove an unmarked capture work root: $ResolvedWorkRoot"
        }
        if ([System.IO.File]::ReadAllText($Marker, [System.Text.Encoding]::UTF8) -ne $MarkerText) {
            throw "Refusing to remove a capture work root with an invalid marker: $ResolvedWorkRoot"
        }
        if ($ResolvedWorkRoot -eq $RepoRoot -or $ResolvedWorkRoot.Length -lt 8) {
            throw "Refusing to remove unsafe capture work root: $ResolvedWorkRoot"
        }
        try {
            Remove-Item -LiteralPath $ResolvedWorkRoot -Recurse -Force -ErrorAction Stop
        }
        catch {
            # Windows keeps rime.dll loaded until this PowerShell process exits.
            # Do not mask the capture result; the retained path is outside the
            # tracked tree and can be removed by the caller after process exit.
            Write-Warning "Work-root cleanup is deferred until rime.dll unloads: $ResolvedWorkRoot"
        }
    }
}
