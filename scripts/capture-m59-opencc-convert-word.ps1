param(
    [string]$OracleBinaryRoot = "C:\rime-m59-4c-oracle\dist",
    [Parameter(Mandatory = $true)]
    [string]$Workspace,
    [Parameter(Mandatory = $true)]
    [string]$Output,
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^\d{4}-\d{2}-\d{2}$')]
    [string]$CaptureDate
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version 2.0

$OracleBinaryRoot = [System.IO.Path]::GetFullPath($OracleBinaryRoot)
$RepoRoot = Split-Path -Parent $PSScriptRoot
$ProbeSource = Join-Path $RepoRoot "scripts\oracle-rime-probe.cs"
$OpenCcSource = Join-Path $RepoRoot "crates\yune-core\src\opencc\data\HKVariantsFull.txt"
$RimeDll = Join-Path $OracleBinaryRoot "lib\rime.dll"
$RimeDeployer = Join-Path $OracleBinaryRoot "bin\rime_deployer.exe"
$RimeHeader = Join-Path $OracleBinaryRoot "include\rime_api.h"

$ExpectedBindings = [ordered]@{
    rime_dll = [ordered]@{
        path = $RimeDll
        sha256 = "86b4c7357d4c6d293ce5589b234d8859ca2ac30923a03bedfa3926eeaf97fb0b"
        bytes = 3739136
    }
    rime_deployer = [ordered]@{
        path = $RimeDeployer
        sha256 = "3abb72b5bb56fcafcfe925d533ae5f832c68d5a0bc9952fd0eea0682fb1ab071"
        bytes = 459776
    }
    rime_api_header = [ordered]@{
        path = $RimeHeader
        sha256 = "85caf744b4e5405a9a1de9c7aef3affc4ae315f4ae5d7ebdd08e191a2c16dad4"
        bytes = 20632
    }
    oracle_probe = [ordered]@{
        path = $ProbeSource
        sha256 = "94f7deb7c3632a6c3c918536295b03d88aa8a80bbbbc9d8a26e896fb70bf07e7"
        bytes = 44846
    }
    opencc_source = [ordered]@{
        path = $OpenCcSource
        sha256 = "145b561c68a697d5f2197da0c091caf4a0e9457f0a4c56cdf2ae7ad4b8ff8cc2"
        bytes = 784
    }
}

function File-Sha256([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Write-NewBytes([string]$Path, [byte[]]$Bytes) {
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

function Write-NewUtf8NoBom([string]$Path, [string]$Text) {
    Write-NewBytes $Path ([System.Text.UTF8Encoding]::new($false).GetBytes($Text))
}

function ConvertTo-CanonicalJsonText([object]$Value) {
    $Json = $Value | ConvertTo-Json -Depth 100
    $Json = $Json.Replace("`r`n", "`n").Replace("`r", "`n")
    return $Json.TrimEnd([char]10) + "`n"
}

function Get-CanonicalM59OpenCcPath([string]$Path) {
    if (-not ("M59OpenCcFinalPath" -as [type])) {
        Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;
using Microsoft.Win32.SafeHandles;

public static class M59OpenCcFinalPath {
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

    if ([string]::IsNullOrWhiteSpace($Path)) {
        throw "Capture paths must not be empty."
    }
    $Full = [System.IO.Path]::GetFullPath($Path)
    $Root = [System.IO.Path]::GetPathRoot($Full)
    if ([string]::IsNullOrWhiteSpace($Root) -or $Full.Substring($Root.Length).Contains(":")) {
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

    $Canonical = [M59OpenCcFinalPath]::Resolve($Existing)
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

function Test-M59OpenCcPathWithinOrEqual([string]$Candidate, [string]$Root) {
    if ([string]::Equals($Candidate, $Root, [System.StringComparison]::OrdinalIgnoreCase)) {
        return $true
    }
    $Prefix = $Root.TrimEnd("\", "/") + [System.IO.Path]::DirectorySeparatorChar
    return $Candidate.StartsWith($Prefix, [System.StringComparison]::OrdinalIgnoreCase)
}

function Assert-M59OpenCcCapturePathPreflight(
    [string]$Output,
    [string]$Workspace,
    [string]$OracleBinaryRoot
) {
    if (Test-Path -LiteralPath $Output) {
        throw "Output must not already exist: $Output"
    }
    if (Test-Path -LiteralPath $Workspace) {
        throw "Workspace must not already exist: $Workspace"
    }
    $OutputParent = Split-Path -Parent ([System.IO.Path]::GetFullPath($Output))
    $WorkspaceParent = Split-Path -Parent ([System.IO.Path]::GetFullPath($Workspace))
    foreach ($Parent in @($OutputParent, $WorkspaceParent)) {
        if (-not (Test-Path -LiteralPath $Parent -PathType Container)) {
            throw "Capture path parent must already exist: $Parent"
        }
    }

    $CanonicalOutput = Get-CanonicalM59OpenCcPath $Output
    $CanonicalWorkspace = Get-CanonicalM59OpenCcPath $Workspace
    $CanonicalOracleRoot = Get-CanonicalM59OpenCcPath $OracleBinaryRoot
    if (Test-M59OpenCcPathWithinOrEqual $CanonicalOutput $CanonicalOracleRoot) {
        throw "Output must not be inside or equal to OracleBinaryRoot: $Output"
    }
    if (Test-M59OpenCcPathWithinOrEqual $CanonicalWorkspace $CanonicalOracleRoot) {
        throw "Workspace must not be inside or equal to OracleBinaryRoot: $Workspace"
    }
    if (Test-M59OpenCcPathWithinOrEqual $CanonicalOutput $CanonicalWorkspace) {
        throw "Output must not be inside or equal to Workspace: $Output"
    }
    if ([string]::Equals(
            $CanonicalOutput,
            $CanonicalWorkspace,
            [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Output and Workspace must be distinct."
    }
    return [ordered]@{
        output = $CanonicalOutput
        workspace = $CanonicalWorkspace
        oracle_root = $CanonicalOracleRoot
    }
}

function Assert-FileBinding([string]$Label, [object]$Binding) {
    if (-not (Test-Path -LiteralPath $Binding.path -PathType Leaf)) {
        throw "Missing pinned $Label input: $($Binding.path)"
    }
    $Info = Get-Item -LiteralPath $Binding.path
    $ActualSha256 = File-Sha256 $Binding.path
    if ($Info.Length -ne [long]$Binding.bytes) {
        throw "$Label byte-size mismatch: expected $($Binding.bytes), observed $($Info.Length)"
    }
    if ($ActualSha256 -ne [string]$Binding.sha256) {
        throw "$Label SHA-256 mismatch: expected $($Binding.sha256), observed $ActualSha256"
    }
    return [ordered]@{
        bytes = [long]$Info.Length
        sha256 = $ActualSha256
    }
}

function Assert-FileUnchanged([string]$Label, [string]$Path, [object]$Before) {
    $After = [ordered]@{
        bytes = [long](Get-Item -LiteralPath $Path).Length
        sha256 = File-Sha256 $Path
    }
    if ($After.bytes -ne [long]$Before.bytes -or $After.sha256 -ne [string]$Before.sha256) {
        throw "$Label changed during capture."
    }
}

function Assert-LoadedRimeDll([string]$ExpectedPath, [string]$ExpectedSha256) {
    $Loaded = @(
        [System.Diagnostics.Process]::GetCurrentProcess().Modules |
            Where-Object { $_.ModuleName -ieq "rime.dll" }
    )
    if ($Loaded.Count -ne 1) {
        throw "Expected exactly one loaded rime.dll module, observed $($Loaded.Count)."
    }
    $ExpectedCanonical = Get-CanonicalM59OpenCcPath $ExpectedPath
    $LoadedCanonical = Get-CanonicalM59OpenCcPath ([string]$Loaded[0].FileName)
    if (-not [string]::Equals(
            $LoadedCanonical,
            $ExpectedCanonical,
            [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "The probe loaded rime.dll from an unpinned path: $LoadedCanonical"
    }
    $LoadedSha256 = File-Sha256 $LoadedCanonical
    if ($LoadedSha256 -ne $ExpectedSha256) {
        throw "Loaded rime.dll SHA-256 mismatch: expected $ExpectedSha256, observed $LoadedSha256"
    }
    return [ordered]@{
        bytes = [long](Get-Item -LiteralPath $LoadedCanonical).Length
        sha256 = $LoadedSha256
        loaded_from_staged_pinned_path = $true
    }
}

function New-FileRecord([string]$Path, [string]$EvidencePath) {
    return [ordered]@{
        path = $EvidencePath
        bytes = [long](Get-Item -LiteralPath $Path).Length
        sha256 = File-Sha256 $Path
    }
}

function Convert-CandidateRecord([object]$Candidate) {
    $Comment = $Candidate["comment"]
    return [ordered]@{
        index = [int]$Candidate["index"]
        global_index = [int]$Candidate["global_index"]
        text = [string]$Candidate["text"]
        comment = if ($null -eq $Comment) { $null } else { [string]$Comment }
    }
}

function Convert-PageRecord([object]$Page) {
    return [ordered]@{
        page_no = [int]$Page["page_no"]
        page_size = [int]$Page["page_size"]
        is_last_page = [bool]$Page["is_last_page"]
        candidates = @($Page["candidates"] | ForEach-Object { Convert-CandidateRecord $_ })
    }
}

function Convert-CaseRecord([object]$Case) {
    return [ordered]@{
        schema_id = [string]$Case["schema_id"]
        schema_name = [string]$Case["schema_name"]
        input = [string]$Case["input"]
        rime_get_input = [string]$Case["rime_get_input"]
        processed = @($Case["processed"] | ForEach-Object { [int]$_ })
        is_composing = [bool]$Case["is_composing"]
        is_ascii_mode = [bool]$Case["is_ascii_mode"]
        preedit = [string]$Case["preedit"]
        commit_text_preview = [string]$Case["commit_text_preview"]
        highlighted_candidate_index = [int]$Case["highlighted_candidate_index"]
        page_size = [int]$Case["page_size"]
        page_no = [int]$Case["page_no"]
        num_candidates = [int]$Case["num_candidates"]
        is_last_page = [bool]$Case["is_last_page"]
        candidate_pointer_null = [bool]$Case["candidate_pointer_null"]
        menu_present = [bool]$Case["menu_present"]
        selected_candidates = @($Case["selected_candidates"] | ForEach-Object { Convert-CandidateRecord $_ })
        pages = @($Case["pages"] | ForEach-Object { Convert-PageRecord $_ })
        all_candidates = @($Case["all_candidates"] | ForEach-Object { Convert-CandidateRecord $_ })
        captured_all_pages = [bool]$Case["captured_all_pages"]
        termination_reason = [string]$Case["termination_reason"]
    }
}

$ParsedCaptureDate = [datetime]::MinValue
if (-not [datetime]::TryParseExact(
        $CaptureDate,
        "yyyy-MM-dd",
        [System.Globalization.CultureInfo]::InvariantCulture,
        [System.Globalization.DateTimeStyles]::None,
        [ref]$ParsedCaptureDate)) {
    throw "CaptureDate must be a real calendar date in yyyy-MM-dd form."
}

$Workspace = [System.IO.Path]::GetFullPath($Workspace)
$Output = [System.IO.Path]::GetFullPath($Output)
$PathState = Assert-M59OpenCcCapturePathPreflight $Output $Workspace $OracleBinaryRoot

$ObservedBindings = [ordered]@{}
foreach ($Label in $ExpectedBindings.Keys) {
    $ObservedBindings[$Label] = Assert-FileBinding $Label $ExpectedBindings[$Label]
}
$CaptureScriptState = [ordered]@{
    bytes = [long](Get-Item -LiteralPath $PSCommandPath).Length
    sha256 = File-Sha256 $PSCommandPath
}

$Shared = Join-Path $Workspace "shared"
$OpenCc = Join-Path $Shared "opencc"
$User = Join-Path $Workspace "user"
$Build = Join-Path $User "build"
$OracleRuntime = Join-Path $Workspace "oracle-runtime"
New-Item -ItemType Directory -Path $Workspace | Out-Null
New-Item -ItemType Directory -Path $Shared, $OpenCc, $User, $Build, $OracleRuntime | Out-Null

if (-not [string]::Equals(
        (Get-CanonicalM59OpenCcPath $Workspace),
        [string]$PathState.workspace,
        [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Workspace canonical path changed during creation."
}

$SchemaId = "m59_opencc_convert_word"
$OpenCcConfigName = "m59-opencc-hkvariantsfull.json"
$OpenCcDataName = "HKVariantsFull.txt"
$DefaultYaml = @"
config_version: "1.0"
schema_list:
  - schema: $SchemaId
"@
$DefaultYaml = $DefaultYaml.Replace("`r`n", "`n").TrimEnd([char]10) + "`n"
$SchemaYaml = @"
schema:
  schema_id: $SchemaId
  name: M59 OpenCC ConvertWord Oracle
  version: "1.0"

switches:
  - name: m59_opencc_variants
    reset: 1

engine:
  processors:
    - speller
    - selector
    - navigator
    - express_editor
  segmentors:
    - abc_segmentor
  translators:
    - table_translator
  filters:
    - simplifier
    - uniquifier

speller:
  alphabet: abcde

translator:
  dictionary: $SchemaId
  enable_completion: false
  enable_sentence: false
  enable_user_dict: false

simplifier:
  option_name: m59_opencc_variants
  opencc_config: $OpenCcConfigName
  tips: none

menu:
  page_size: 2
"@
$SchemaYaml = $SchemaYaml.Replace("`r`n", "`n").TrimEnd([char]10) + "`n"

$ExactTwoSource = [string][char]0x7955
$ExactTwoDefault = [string][char]0x79D8
$ExactThreeSource = [string][char]0x7CC9
$PartialSource = $ExactTwoSource + $ExactThreeSource
$OriginalFirstSource = [string][char]0x53EA
$PassThroughSource = ([string][char]0x7532) + ([string][char]0x4E59)
$DictYaml = @"
---
name: $SchemaId
version: "1.0"
sort: original
use_preset_vocabulary: false
...
$ExactTwoSource`ta`t100
$ExactTwoDefault`ta`t90
$ExactThreeSource`tb`t100
$PartialSource`tc`t100
$OriginalFirstSource`td`t100
$PassThroughSource`te`t100
"@
$DictYaml = $DictYaml.Replace("`r`n", "`n").TrimEnd([char]10) + "`n"
$OpenCcConfig = @"
{
  "name": "M59 HK full-variant ConvertWord oracle",
  "segmentation": {
    "type": "mmseg",
    "dict": {
      "type": "text",
      "file": "$OpenCcDataName"
    }
  },
  "conversion_chain": [{
    "dict": {
      "type": "text",
      "file": "$OpenCcDataName"
    }
  }]
}
"@
$OpenCcConfig = $OpenCcConfig.Replace("`r`n", "`n").TrimEnd([char]10) + "`n"

$DefaultPath = Join-Path $Shared "default.yaml"
$SchemaPath = Join-Path $Shared "$SchemaId.schema.yaml"
$DictPath = Join-Path $Shared "$SchemaId.dict.yaml"
$OpenCcConfigPath = Join-Path $OpenCc $OpenCcConfigName
$OpenCcDataPath = Join-Path $OpenCc $OpenCcDataName
$StagedRimeDll = Join-Path $OracleRuntime "rime.dll"
$StagedRimeDeployer = Join-Path $OracleRuntime "rime_deployer.exe"
Write-NewUtf8NoBom $DefaultPath $DefaultYaml
Write-NewUtf8NoBom $SchemaPath $SchemaYaml
Write-NewUtf8NoBom $DictPath $DictYaml
Write-NewUtf8NoBom $OpenCcConfigPath $OpenCcConfig
Write-NewBytes $OpenCcDataPath ([System.IO.File]::ReadAllBytes($OpenCcSource))
Write-NewBytes $StagedRimeDll ([System.IO.File]::ReadAllBytes($RimeDll))
Write-NewBytes $StagedRimeDeployer ([System.IO.File]::ReadAllBytes($RimeDeployer))

$GeneratedSources = @(
    New-FileRecord $DefaultPath "shared/default.yaml"
    New-FileRecord $SchemaPath "shared/$SchemaId.schema.yaml"
    New-FileRecord $DictPath "shared/$SchemaId.dict.yaml"
    New-FileRecord $OpenCcConfigPath "shared/opencc/$OpenCcConfigName"
    New-FileRecord $OpenCcDataPath "shared/opencc/$OpenCcDataName"
)
$StagedRuntime = @(
    New-FileRecord $StagedRimeDll "oracle-runtime/rime.dll"
    New-FileRecord $StagedRimeDeployer "oracle-runtime/rime_deployer.exe"
)
if ($StagedRuntime[0].sha256 -ne [string]$ObservedBindings.rime_dll.sha256 -or
    $StagedRuntime[1].sha256 -ne [string]$ObservedBindings.rime_deployer.sha256) {
    throw "Staged oracle runtime bytes differ from the pinned protected inputs."
}

$PreviousPath = $env:PATH
try {
    $env:PATH = $OracleRuntime + ";" + $PreviousPath
    & $StagedRimeDeployer --build $User $Shared $Build
    if ($LASTEXITCODE -ne 0) {
        throw "rime_deployer.exe --build failed with exit code $LASTEXITCODE"
    }

    Add-Type -Path $ProbeSource
    $Inputs = [string[]]@("a", "b", "c", "d", "e")
    $Modules = [string[]]@("default")
    $RawCases = [RimeProbe]::Capture($Shared, $User, $Build, $SchemaId, $Modules, $Inputs)
    $LoadedRimeDll = Assert-LoadedRimeDll `
        $StagedRimeDll `
        ([string]$ObservedBindings.rime_dll.sha256)
}
finally {
    $env:PATH = $PreviousPath
}

if ($RawCases.Count -ne $Inputs.Count) {
    throw "Oracle capture returned $($RawCases.Count) cases for $($Inputs.Count) inputs."
}
for ($Index = 0; $Index -lt $Inputs.Count; $Index++) {
    $Case = $RawCases[$Index]
    if ([string]$Case["input"] -cne $Inputs[$Index]) {
        throw "Oracle capture input order drifted at index $Index."
    }
    if ([string]$Case["schema_id"] -cne $SchemaId) {
        throw "Oracle capture schema drifted for input '$($Inputs[$Index])'."
    }
    if (-not [bool]$Case["captured_all_pages"] -or [string]$Case["termination_reason"] -cne "last_page") {
        $Reason = if ($Case.ContainsKey("pagination_error")) {
            [string]$Case["pagination_error"]
        }
        else {
            [string]$Case["termination_reason"]
        }
        throw "Oracle capture did not reach the last page for input '$($Inputs[$Index])': $Reason"
    }
}

foreach ($Label in $ExpectedBindings.Keys) {
    Assert-FileUnchanged $Label $ExpectedBindings[$Label].path $ObservedBindings[$Label]
}
Assert-FileUnchanged "capture script" $PSCommandPath $CaptureScriptState
foreach ($Source in $GeneratedSources) {
    $GeneratedPath = Join-Path $Workspace ([string]$Source.path).Replace("/", "\")
    Assert-FileUnchanged ([string]$Source.path) $GeneratedPath $Source
}
foreach ($RuntimeFile in $StagedRuntime) {
    $RuntimePath = Join-Path $Workspace ([string]$RuntimeFile.path).Replace("/", "\")
    Assert-FileUnchanged ([string]$RuntimeFile.path) $RuntimePath $RuntimeFile
}
if ((File-Sha256 $OpenCcDataPath) -ne [string]$ObservedBindings.opencc_source.sha256) {
    throw "Staged HKVariantsFull.txt bytes differ from the pinned checked-in source."
}
if (Test-Path -LiteralPath $Output) {
    throw "Output appeared during capture and will not be overwritten: $Output"
}
if (-not [string]::Equals(
        (Get-CanonicalM59OpenCcPath $Output),
        [string]$PathState.output,
        [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Output canonical path changed during capture."
}
if (-not [string]::Equals(
        (Get-CanonicalM59OpenCcPath $Workspace),
        [string]$PathState.workspace,
        [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Workspace canonical path changed during capture."
}

$RuntimeOptions = @(
    foreach ($Option in [RimeProbe]::GetCaptureRuntimeOptions()) {
        [ordered]@{
            name = [string]$Option.name
            enabled = [bool]$Option.enabled
        }
    }
)
$Cases = @($RawCases | ForEach-Object { Convert-CaseRecord $_ })
$Evidence = [ordered]@{
    fixture_version = 1
    milestone = "M59 Increment 4c"
    status = "pinned_upstream_oracle_capture"
    source_row_policy = "m59_minimal_opencc_convert_word_and_default_segmentation_oracle"
    oracle = [ordered]@{
        engine = "rime/librime"
        engine_tag = "1.17.0"
        engine_commit = "33e78140250125871856cdc5b42ddc6a5fcd3cd4"
        canonical_repository = "https://github.com/rime/librime"
        release_url = "https://github.com/rime/librime/releases/tag/1.17.0"
        capture_date = $CaptureDate
        binaries = [ordered]@{
            rime_dll = [ordered]@{
                path = "external/oracle-binaries/lib/rime.dll"
                bytes = [long]$ObservedBindings.rime_dll.bytes
                sha256 = [string]$ObservedBindings.rime_dll.sha256
                staged_for_execution = $true
                loaded_module_verified = [bool]$LoadedRimeDll.loaded_from_staged_pinned_path
            }
            rime_deployer = [ordered]@{
                path = "external/oracle-binaries/bin/rime_deployer.exe"
                bytes = [long]$ObservedBindings.rime_deployer.bytes
                sha256 = [string]$ObservedBindings.rime_deployer.sha256
                staged_for_execution = $true
            }
            rime_api_header = [ordered]@{
                path = "external/oracle-binaries/include/rime_api.h"
                bytes = [long]$ObservedBindings.rime_api_header.bytes
                sha256 = [string]$ObservedBindings.rime_api_header.sha256
            }
        }
    }
    tools = [ordered]@{
        capture_script = [ordered]@{
            path = "scripts/capture-m59-opencc-convert-word.ps1"
            bytes = [long]$CaptureScriptState.bytes
            sha256 = [string]$CaptureScriptState.sha256
        }
        oracle_probe = [ordered]@{
            path = "scripts/oracle-rime-probe.cs"
            bytes = [long]$ObservedBindings.oracle_probe.bytes
            sha256 = [string]$ObservedBindings.oracle_probe.sha256
            capture_api = "RimeProbe.Capture"
        }
    }
    data = [ordered]@{
        opencc_dictionary = [ordered]@{
            path = "crates/yune-core/src/opencc/data/HKVariantsFull.txt"
            canonical_repository = "https://github.com/rime/rime-cantonese"
            repository_commit = "c99b16e44d2df77a5cb8fb0867dd2bab7a112cb0"
            repository_tree = "eb193fb80675ffa60df3c32bf24afa7d7f68617a"
            repository_path = "opencc/HKVariantsFull.txt"
            dictionary_manifest_sha256 = "4301001fb7bb52d5d1a9c032c519ac18ba50677e926e01006e34a48788385efa"
            bytes = [long]$ObservedBindings.opencc_source.bytes
            sha256 = [string]$ObservedBindings.opencc_source.sha256
            staged_byte_identical = $true
        }
        generated_sources = $GeneratedSources
    }
    capture = [ordered]@{
        schema_id = $SchemaId
        inputs = @($Inputs)
        page_size = 2
        captured_all_pages_required = $true
        modules = @($Modules)
        filter_chain = @("simplifier", "uniquifier")
        opencc_config = $OpenCcConfigName
        runtime_options = $RuntimeOptions
        runtime_options_source = [RimeProbe]::CaptureRuntimeOptionsSource
        table_rows = @(
            [ordered]@{ code = "a"; text = $ExactTwoSource; weight = 100; role = "exact_one_to_many_source" }
            [ordered]@{ code = "a"; text = $ExactTwoDefault; weight = 90; role = "later_already_normalized_dedup_source" }
            [ordered]@{ code = "b"; text = $ExactThreeSource; weight = 100; role = "exact_three_way_source" }
            [ordered]@{ code = "c"; text = $PartialSource; weight = 100; role = "multi_character_no_exact_mapping_source" }
            [ordered]@{ code = "d"; text = $OriginalFirstSource; weight = 100; role = "exact_mapping_whose_default_is_original" }
            [ordered]@{ code = "e"; text = $PassThroughSource; weight = 100; role = "unmapped_pass_through_control" }
        )
        observation_roles = [ordered]@{
            a = "whole-word ordered one-to-many plus stable dedup against a later normalized row"
            b = "whole-word ordered three-way conversion spanning all candidate pages"
            c = "no exact multi-character mapping; max-seg partial conversion uses defaults only"
            d = "whole-word exact mapping retains its original-valued default in declared first position"
            e = "unmapped whole word passes through unchanged"
        }
    }
    provenance = [ordered]@{
        capture_command = "powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/capture-m59-opencc-convert-word.ps1 -OracleBinaryRoot 'external/oracle-binaries' -Workspace 'external/workspace' -Output 'external/output' -CaptureDate '$CaptureDate'"
        path_serialization_policy = "repo-relative tool/data paths and external role placeholders; no local absolute paths"
        output_policy = "UTF-8 without BOM, LF-only, create-new; output must remain outside the protected oracle-binary root and workspace"
        workspace_policy = "caller supplies an absent isolated workspace outside the protected oracle-binary root; generated sources and exact-byte staged oracle executables are hash-bound and retained for audit but are never copied into the tracked fixture"
        expectation_policy = "capture records upstream output directly; no expected candidate text is derived from Yune"
    }
    cases = $Cases
}

$EvidenceJson = ConvertTo-CanonicalJsonText $Evidence
Write-NewUtf8NoBom $Output $EvidenceJson
Write-Output "PASS: captured pinned librime 1.17.0 OpenCC ConvertWord and default-segmentation behavior."
Write-Output "Output: $Output"
Write-Output "Workspace retained: $Workspace"
