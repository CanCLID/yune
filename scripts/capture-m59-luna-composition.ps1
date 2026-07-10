# Captures the librime 1.17.0 oracle provenance for M59 leading-single
# reachability + partial-selection composition on luna_pinyin:
#   - paged candidate lists for the PRIMARY non-lexicon case moboyi -> mo/bo/yi
#     (remainders boyi / yi) plus zhonggao / zhongguo / gao / guo (the
#     reachable-single positions the M59 acceptance rows cite), and
#   - the moboyi partial-selection composition chain (commit U+83AB U+4F2F
#     U+6D22 from selecting those three leading singles one at a time).
#
# Runs the real rime.dll via scripts/oracle-rime-probe.cs. Requires the upstream
# oracle root laid out by the capture-upstream-* pipeline (pinned rime.dll,
# rime_deployer.exe, schema source repositories, and upstream OpenCC data). It
# deploys a clean disposable shared/user/build tree for every capture. Emits raw
# UTF-8 snapshot JSON, then curates a fresh scratch oracle via
# scripts/curate-m59-luna-composition.py for byte/diff review before any separate
# fixture import (Python reads
# the raw files as UTF-8 files, avoiding PowerShell native-pipe codepage
# corruption of the CJK candidate texts). This .ps1 is intentionally pure-ASCII.
param(
    [string]$OracleRoot,
    [string]$Output,
    [string]$ExpectedRimeDllSha256 = "86b4c7357d4c6d293ce5589b234d8859ca2ac30923a03bedfa3926eeaf97fb0b",
    [string]$ExpectedRimeDeployerSha256 = "3abb72b5bb56fcafcfe925d533ae5f832c68d5a0bc9952fd0eea0682fb1ab071",
    [string]$ExpectedLunaPinyinCommit = "18a80335c37522311f7cff02886cd81cec3b460a",
    [string]$ExpectedPreludeCommit = "082425ea0684bca36474415d4a0e8db9b016487e",
    [string]$ExpectedEssayCommit = "48c7538f0b760fcc8c9d6bf08711f82cfbd2e9ed",
    [string]$ExpectedStrokeCommit = "3a4b0f4013e2b4c14b1e80c92b1d4723eb65f39c"
)
$ErrorActionPreference = "Stop"
$RepoRoot = [System.IO.Path]::GetFullPath((Resolve-Path (Join-Path $PSScriptRoot "..")).Path)
$DefaultFixture = Join-Path $RepoRoot "crates/yune-core/tests/fixtures/upstream-1.17.0/m59-luna-leading-single-composition.json"
$DefaultOutput = Join-Path $RepoRoot "target/m59-luna-leading-single-composition.json"
if ([string]::IsNullOrWhiteSpace($OracleRoot)) {
    $OracleRoot = Join-Path $RepoRoot "target/upstream-oracle/1.17.0"
}
if ([string]::IsNullOrWhiteSpace($Output)) {
    $Output = $DefaultOutput
}
$OracleRoot = [System.IO.Path]::GetFullPath($OracleRoot)
$Output = [System.IO.Path]::GetFullPath($Output)
$Extract = Join-Path $OracleRoot "extract"
$Probe = Join-Path $RepoRoot "scripts/oracle-rime-probe.cs"
$Curate = Join-Path $RepoRoot "scripts/curate-m59-luna-composition.py"
$RimeDll = Join-Path $Extract "dist/lib/rime.dll"
$RimeDeployer = Join-Path $Extract "dist/bin/rime_deployer.exe"
$SchemaRoot = Join-Path $OracleRoot "schema-src"

$PageDown = 65366

function New-Action($type, $text, $kc, $label) {
    $a = [RimeProbe+ProbeAction]::new()
    $a.type = $type
    if ($text) { $a.text = $text }
    if ($kc) { $a.keycode = $kc; $a.mask = 0 }
    if ($label) { $a.label = $label }
    return $a
}

function File-Sha256([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Assert-FileSha256Unchanged(
    [string]$Path,
    [string]$Label,
    [string]$ExpectedSha256
) {
    $ObservedSha256 = File-Sha256 $Path
    if ($ObservedSha256 -ne $ExpectedSha256) {
        throw "Capture input changed during Lane-B capture: $Label"
    }
}

function ConvertTo-CanonicalJsonText([object]$Value) {
    $Json = $Value | ConvertTo-Json -Depth 20
    $Json = $Json.Replace("`r`n", "`n").Replace("`r", "`n")
    return $Json.TrimEnd([char]10) + "`n"
}

function Get-CanonicalLaneBPath([string]$Path) {
    if (-not ("M59LaneBFinalPath" -as [type])) {
        Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;
using Microsoft.Win32.SafeHandles;

public static class M59LaneBFinalPath {
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
    $Canonical = [M59LaneBFinalPath]::Resolve($Existing)
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

function Test-LaneBPathWithinOrEqual([string]$Candidate, [string]$Root) {
    if ([string]::Equals($Candidate, $Root, [System.StringComparison]::OrdinalIgnoreCase)) {
        return $true
    }
    $Prefix = $Root.TrimEnd("\", "/") + [System.IO.Path]::DirectorySeparatorChar
    return $Candidate.StartsWith($Prefix, [System.StringComparison]::OrdinalIgnoreCase)
}

function Assert-LaneBOutputPreflight(
    [string]$Output,
    [string]$OracleRoot,
    [System.Collections.IDictionary]$ProtectedInputs
) {
    if (Test-Path -LiteralPath $Output) {
        throw "Output must not already exist: $Output"
    }
    $CanonicalOutput = Get-CanonicalLaneBPath $Output
    $CanonicalOracleRoot = Get-CanonicalLaneBPath $OracleRoot
    if (Test-LaneBPathWithinOrEqual $CanonicalOutput $CanonicalOracleRoot) {
        throw "Output must not be inside or equal to OracleRoot: $Output"
    }
    foreach ($Entry in $ProtectedInputs.GetEnumerator()) {
        $CanonicalInput = Get-CanonicalLaneBPath ([string]$Entry.Value)
        if ([string]::Equals(
                $CanonicalOutput,
                $CanonicalInput,
                [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Output must not alias protected input $($Entry.Key): $Output"
        }
        $InputItem = Get-Item -LiteralPath ([string]$Entry.Value) -Force
        if ($InputItem.PSIsContainer -and
            (Test-LaneBPathWithinOrEqual $CanonicalOutput $CanonicalInput)) {
            throw "Output must not be inside protected input $($Entry.Key): $Output"
        }
    }
    return $CanonicalOutput
}

function Write-Utf8NoBom([string]$Path, [string]$Content) {
    [System.IO.File]::WriteAllText(
        $Path,
        $Content,
        [System.Text.UTF8Encoding]::new($false, $true)
    )
}

function Assert-CanonicalJsonFile([string]$Path, [string]$Label) {
    $Bytes = [System.IO.File]::ReadAllBytes($Path)
    if ($Bytes.Length -eq 0) {
        throw "$Label is empty."
    }
    if ($Bytes.Length -ge 3 -and
        $Bytes[0] -eq 0xef -and $Bytes[1] -eq 0xbb -and $Bytes[2] -eq 0xbf) {
        throw "$Label must not contain a UTF-8 BOM."
    }
    if ($Bytes -contains [byte]0 -or $Bytes -contains [byte]13) {
        throw "$Label must be NUL-free and LF-only."
    }
    if ($Bytes[$Bytes.Length - 1] -ne [byte]10 -or
        ($Bytes.Length -ge 2 -and $Bytes[$Bytes.Length - 2] -eq [byte]10)) {
        throw "$Label must end in exactly one LF."
    }
    $StrictUtf8 = [System.Text.UTF8Encoding]::new($false, $true)
    $null = $StrictUtf8.GetString($Bytes)
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

function Tree-Sha256([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path -PathType Container)) {
        throw "Cannot hash missing evidence tree: $Path"
    }
    $Root = [System.IO.Path]::GetFullPath((Resolve-Path -LiteralPath $Path).Path).TrimEnd("\", "/")
    $Rows = [System.Collections.Generic.List[string]]::new()
    foreach ($File in (Get-ChildItem -LiteralPath $Root -Recurse -File -Force)) {
        $Relative = $File.FullName.Substring($Root.Length + 1).Replace("\", "/")
        $Rows.Add("$Relative`t$(File-Sha256 $File.FullName)")
    }
    $Ordered = $Rows.ToArray()
    [Array]::Sort($Ordered, [System.StringComparer]::Ordinal)
    $Payload = (($Ordered -join "`n") + "`n")
    return Bytes-Sha256 ([System.Text.Encoding]::UTF8.GetBytes($Payload))
}

function Git-Head([string]$Path) {
    return (& git -C $Path rev-parse HEAD).Trim()
}

function Git-Tree([string]$Path) {
    return (& git -C $Path rev-parse 'HEAD^{tree}').Trim()
}

function Git-State([string]$Path) {
    $Head = Git-Head $Path
    $Tree = Git-Tree $Path
    $Status = @(& git -C $Path status --porcelain=v1 --untracked-files=all)
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to inspect Git state at $Path"
    }
    return [pscustomobject]@{
        commit = $Head
        git_tree = $Tree
        clean = $Status.Count -eq 0
        status_short = @($Status)
    }
}

function Assert-GitStateUnchanged([string]$Path, [string]$Label, [object]$Before) {
    $After = Git-State $Path
    if ($After.commit -ne $Before.commit -or
        $After.git_tree -ne $Before.git_tree -or
        $After.clean -ne $Before.clean -or
        ((@($After.status_short) -join "`n") -cne (@($Before.status_short) -join "`n"))) {
        throw "Git source state changed during Lane-B capture: $Label"
    }
}

function Assert-Git-Clean([string]$Path, [string]$Identity) {
    $Status = @(& git -C $Path status --porcelain=v1 --untracked-files=all)
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to inspect source cleanliness for $Identity at $Path"
    }
    if ($Status.Count -ne 0) {
        throw "Source repository $Identity is not clean; exact Lane-B byte provenance requires a clean pinned checkout."
    }
}

function Copy-PinnedRimeData([string]$Source, [string]$Destination) {
    foreach ($File in (Get-ChildItem -LiteralPath $Source -File)) {
        if ($File.Name -notlike "*.yaml" -and $File.Name -ne "essay.txt") {
            continue
        }
        & git -C $Source ls-files --error-unmatch -- $File.Name 2>&1 | Out-Null
        if ($LASTEXITCODE -ne 0) {
            throw "Refusing untracked staged source file: $($File.FullName)"
        }
        Copy-Item -LiteralPath $File.FullName -Destination (Join-Path $Destination $File.Name)
    }
}

function Evidence-Path([string]$Path) {
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
        throw "Lane-B capture requires the ordered four-false RimeProbe runtime option policy."
    }
    $Source = [string][RimeProbe]::SharedCaptureRuntimeOptionsSource
    if ($Source -ne "RimeProbe.CaptureWithIdentity+CaptureScenariosWithIdentity/CaptureRuntimeOptionPolicy") {
        throw "Lane-B capture runtime option policy source changed: $Source"
    }
    return [pscustomobject]@{
        runtime_options = $RuntimeOptions
        runtime_options_source = $Source
    }
}

$ProtectedCaptureInputs = [ordered]@{
    tool_directory = $PSScriptRoot
    capture_script = $PSCommandPath
    probe_source = $Probe
    curator = $Curate
    existing_fixture = $DefaultFixture
}
$Output = Assert-LaneBOutputPreflight $Output $OracleRoot $ProtectedCaptureInputs
$ToolState = Git-State $RepoRoot
if (-not $ToolState.clean) {
    throw "Canonical Lane-B evidence requires a clean Yune tool source."
}
$CaptureScriptSha256 = File-Sha256 $PSCommandPath
$CuratorSha256 = File-Sha256 $Curate
$ProbeSha256 = File-Sha256 $Probe
$ActualRimeDllSha256 = File-Sha256 $RimeDll
$ActualRimeDeployerSha256 = File-Sha256 $RimeDeployer
if ($ActualRimeDllSha256 -ne $ExpectedRimeDllSha256.ToLowerInvariant()) {
    throw "Unexpected upstream rime.dll SHA-256. Expected $ExpectedRimeDllSha256, got $ActualRimeDllSha256."
}
if ($ActualRimeDeployerSha256 -ne $ExpectedRimeDeployerSha256.ToLowerInvariant()) {
    throw "Unexpected upstream rime_deployer.exe SHA-256. Expected $ExpectedRimeDeployerSha256, got $ActualRimeDeployerSha256."
}
$SchemaSourceCommit = Git-Head (Join-Path $SchemaRoot "rime-luna-pinyin")
$PreludeCommit = Git-Head (Join-Path $SchemaRoot "rime-prelude")
$EssayCommit = Git-Head (Join-Path $SchemaRoot "rime-essay")
$StrokeCommit = Git-Head (Join-Path $SchemaRoot "rime-stroke")
$PinnedCommits = [ordered]@{
    "rime/rime-luna-pinyin" = [ordered]@{ actual = $SchemaSourceCommit; expected = $ExpectedLunaPinyinCommit }
    "rime/rime-prelude" = [ordered]@{ actual = $PreludeCommit; expected = $ExpectedPreludeCommit }
    "rime/rime-essay" = [ordered]@{ actual = $EssayCommit; expected = $ExpectedEssayCommit }
    "rime/rime-stroke" = [ordered]@{ actual = $StrokeCommit; expected = $ExpectedStrokeCommit }
}
foreach ($Entry in $PinnedCommits.GetEnumerator()) {
    if ($Entry.Value.actual -ne $Entry.Value.expected.ToLowerInvariant()) {
        throw "Unexpected $($Entry.Key) commit. Expected $($Entry.Value.expected), got $($Entry.Value.actual)."
    }
}
$SourceRepoPaths = [ordered]@{
    "rime/rime-luna-pinyin" = Join-Path $SchemaRoot "rime-luna-pinyin"
    "rime/rime-prelude" = Join-Path $SchemaRoot "rime-prelude"
    "rime/rime-essay" = Join-Path $SchemaRoot "rime-essay"
    "rime/rime-stroke" = Join-Path $SchemaRoot "rime-stroke"
}
$SourceGitTrees = [ordered]@{}
foreach ($Entry in $SourceRepoPaths.GetEnumerator()) {
    Assert-Git-Clean $Entry.Value $Entry.Key
    $Tree = Git-Tree $Entry.Value
    if ($Tree -notmatch '^[0-9a-f]{40}$') {
        throw "Unexpected git tree identity for $($Entry.Key): $Tree"
    }
    $SourceGitTrees[$Entry.Key] = $Tree
}
$OldPath = $env:PATH
$env:PATH = (Join-Path $Extract "dist/lib") + ";" + (Join-Path $Extract "bin") + ";" + $env:PATH
Add-Type -Path $Probe
$RuntimeOptionProvenance = Get-RimeCaptureRuntimeOptionProvenance
$RuntimeOptions = $RuntimeOptionProvenance.runtime_options
$RuntimeOptionsSource = [string]$RuntimeOptionProvenance.runtime_options_source

# A composition scenario: type $input, then for each step turn $pd pages and
# press the $digit selection key (49='1' selects page position 0, etc.). Digit
# keys are 1-indexed within the page: global index N sits on page (N / page_size)
# at position (N % page_size), selected by digit (position + 1).
function New-Compose([string]$name, [string]$sequence, [object[]]$steps) {
    $acts = New-Object System.Collections.Generic.List[object]
    $acts.Add((New-Action "input" $sequence 0 $null))
    $acts.Add((New-Action "snapshot" $null 0 ($sequence + "_page0")))
    foreach ($step in $steps) {
        for ($i = 1; $i -le [int]$step.pd; $i++) {
            $acts.Add((New-Action "key" $null $PageDown ($step.label + "_pd$i")))
        }
        $acts.Add((New-Action "key" $null ([int]$step.digit) ("after_select_" + $step.label)))
    }
    $sc = [RimeProbe+ProbeScenario]::new()
    $sc.name = $name
    $sc.actions = [RimeProbe+ProbeAction[]]$acts.ToArray()
    return $sc
}

$WorkRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("m59-luna-capture-" + [guid]::NewGuid().ToString("N"))
if (Test-Path -LiteralPath $WorkRoot) {
    throw "Refusing to reuse an existing Lane-B capture root: $WorkRoot"
}
New-Item -ItemType Directory -Path $WorkRoot | Out-Null
$WorkRootMarker = Join-Path $WorkRoot ".yune-m59-luna-capture-root"
$WorkRootMarkerText = "created-by=capture-m59-luna-composition.ps1`n"
[System.IO.File]::WriteAllText(
    $WorkRootMarker,
    $WorkRootMarkerText,
    [System.Text.UTF8Encoding]::new($false)
)
$Shared = Join-Path $WorkRoot "shared"
$User = Join-Path $WorkRoot "user"
$Build = Join-Path $User "build"
$Raw = Join-Path $WorkRoot "raw"
try {
New-Item -ItemType Directory -Path $Shared, $User, $Build, $Raw | Out-Null
foreach ($Entry in $SourceRepoPaths.GetEnumerator()) {
    Copy-PinnedRimeData $Entry.Value $Shared
}
$OpenCcSource = Join-Path $Extract "share/opencc"
$OpenCcDestination = Join-Path $Shared "opencc"
if (-not (Test-Path -LiteralPath $OpenCcSource -PathType Container)) {
    throw "Missing pinned upstream OpenCC data: $OpenCcSource"
}
New-Item -ItemType Directory -Path $OpenCcDestination | Out-Null
Get-ChildItem -LiteralPath $OpenCcSource -Force | ForEach-Object {
    Copy-Item -LiteralPath $_.FullName -Destination $OpenCcDestination -Recurse
}
$DefaultCustom = Join-Path $Shared "default.custom.yaml"
[System.IO.File]::WriteAllText(
    $DefaultCustom,
    "patch:`n  schema_list:`n    - schema: luna_pinyin`n",
    [System.Text.UTF8Encoding]::new($false)
)
$StagedTimestampUtc = [DateTimeOffset]::FromUnixTimeSeconds(946684800).AddMilliseconds(500).UtcDateTime
Get-ChildItem -LiteralPath $Shared -Recurse -File -Force | ForEach-Object {
    $_.LastWriteTimeUtc = $StagedTimestampUtc
}
$TimestampDrift = @(
    Get-ChildItem -LiteralPath $Shared -Recurse -File -Force |
        Where-Object { $_.LastWriteTimeUtc.Ticks -ne $StagedTimestampUtc.Ticks }
)
if ($TimestampDrift.Count -ne 0) {
    throw "Staged timestamp readback was not exact for $($TimestampDrift.Count) files."
}
& $RimeDeployer --build $User $Shared $Build
if ($LASTEXITCODE -ne 0) {
    throw "Pinned rime_deployer.exe --build failed with exit code $LASTEXITCODE"
}
$SharedTreeSha256BeforeCapture = Tree-Sha256 $Shared
$BuildTreeSha256BeforeCapture = Tree-Sha256 $Build
$DefaultCustomSha256 = File-Sha256 $DefaultCustom
$OpenCcTreeSha256 = Tree-Sha256 $OpenCcDestination

$modules = [string[]]@("default")
$AdditionalRuntimeOptionPatches = @()

# PRIMARY (and only) case: moboyi -> the non-lexicon phrase U+83AB U+4F2F
# U+6D22. `boyi`/`yi` are its recompose remainders; zhonggao/zhongguo/gao/guo
# are the class rows.
$pagingInputs = @("moboyi", "boyi", "yi", "zhonggao", "zhongguo", "gao", "guo")
$pageSnaps = [RimeProbe]::Capture($Shared, $User, $Build, "luna_pinyin", $modules, [string[]]$pagingInputs)
if ($pageSnaps.Count -ne $pagingInputs.Count) {
    throw "Lane-B capture returned $($pageSnaps.Count) cases for $($pagingInputs.Count) inputs."
}
for ($InputIndex = 0; $InputIndex -lt $pagingInputs.Count; $InputIndex++) {
    if ([string]$pageSnaps[$InputIndex]["input"] -cne [string]$pagingInputs[$InputIndex]) {
        throw "Lane-B capture did not preserve the declared seven-input order."
    }
}
foreach ($Case in $pageSnaps) {
    if (-not $Case["captured_all_pages"]) {
        $Reason = if ($Case.ContainsKey("pagination_error")) { $Case["pagination_error"] } else { "unknown" }
        throw "Capture for input '$($Case["input"])' did not capture all pages: $Reason"
    }
}
$PageSizesObserved = @(
    $pageSnaps |
        ForEach-Object { [int]$_['page_size'] } |
        Select-Object -Unique
)
if ($PageSizesObserved.Count -eq 0 -or
    @($PageSizesObserved | Where-Object { $_ -le 0 }).Count -ne 0) {
    throw "Lane-B capture observed an invalid page size."
}

# moboyi -> U+83AB U+4F2F U+6D22: U+83AB@2 (page0, digit '3'); boyi
# U+4F2F@19 (page3 pos4, digit '5' after 3 Page_Down); yi U+6D22@155
# (page31 pos0, digit '1' after 31 Page_Down).
$moboyi = New-Compose "moboyi_compose" "moboyi" @(
    @{ pd = 0; digit = 51; label = "mo" },
    @{ pd = 3; digit = 53; label = "bo" },
    @{ pd = 31; digit = 49; label = "yi" }
)
$composeSnaps = [RimeProbe]::CaptureScenarios($Shared, $User, $Build, "luna_pinyin", $modules, [RimeProbe+ProbeScenario[]]@($moboyi))
$SharedTreeSha256AfterCapture = Tree-Sha256 $Shared
$BuildTreeSha256AfterCapture = Tree-Sha256 $Build
if ($SharedTreeSha256AfterCapture -ne $SharedTreeSha256BeforeCapture) {
    throw "Lane-B shared data changed during capture; refusing unbound evidence."
}
if ($BuildTreeSha256AfterCapture -ne $BuildTreeSha256BeforeCapture) {
    throw "Lane-B deployed build changed during capture; refusing unbound evidence."
}
foreach ($Entry in $SourceRepoPaths.GetEnumerator()) {
    Assert-Git-Clean $Entry.Value $Entry.Key
    if ((Git-Head $Entry.Value) -ne $PinnedCommits[$Entry.Key].actual -or
        (Git-Tree $Entry.Value) -ne $SourceGitTrees[$Entry.Key]) {
        throw "Pinned source identity changed during Lane-B capture: $($Entry.Key)"
    }
}
Assert-GitStateUnchanged $RepoRoot "yune" $ToolState
Assert-FileSha256Unchanged $RimeDll "rime.dll" $ActualRimeDllSha256
Assert-FileSha256Unchanged $RimeDeployer "rime_deployer.exe" $ActualRimeDeployerSha256
Assert-FileSha256Unchanged $PSCommandPath "capture script" $CaptureScriptSha256
Assert-FileSha256Unchanged $Curate "curator" $CuratorSha256
Assert-FileSha256Unchanged $Probe "RimeProbe source" $ProbeSha256

$PagesRaw = Join-Path $Raw "pages.json"
$ComposeRaw = Join-Path $Raw "compose.json"
$MetadataRaw = Join-Path $Raw "metadata.json"
    $Serialization = [ordered]@{
        encoding = "utf-8"
        bom = $false
        line_endings = "lf"
        terminal_newline = "exactly_one"
    }
    $RawPaths = [ordered]@{
        pages = "disposable/raw/pages.json"
        composition = "disposable/raw/compose.json"
        metadata = "disposable/raw/metadata.json"
    }
    $EffectiveParameters = [ordered]@{
        oracle_root = Evidence-Path $OracleRoot
        output = Evidence-Path $Output
        expected_rime_dll_sha256 = $ExpectedRimeDllSha256.ToLowerInvariant()
        expected_rime_deployer_sha256 = $ExpectedRimeDeployerSha256.ToLowerInvariant()
        expected_luna_pinyin_commit = $ExpectedLunaPinyinCommit.ToLowerInvariant()
        expected_prelude_commit = $ExpectedPreludeCommit.ToLowerInvariant()
        expected_essay_commit = $ExpectedEssayCommit.ToLowerInvariant()
        expected_stroke_commit = $ExpectedStrokeCommit.ToLowerInvariant()
        schema_id = "luna_pinyin"
        modules = @($modules)
        inputs = @($pagingInputs)
        page_policy = "RimeProbe.Capture all pages; incomplete pagination is fatal"
        runtime_options = $RuntimeOptions
        runtime_options_source = $RuntimeOptionsSource
        additional_runtime_option_patches = @($AdditionalRuntimeOptionPatches)
        serialization = $Serialization
    }
    $EffectiveInvocation = @(
        "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/capture-m59-luna-composition.ps1",
        "-OracleRoot $(Quote-CommandArg $EffectiveParameters.oracle_root)",
        "-Output $(Quote-CommandArg $EffectiveParameters.output)",
        "-ExpectedRimeDllSha256 $(Quote-CommandArg $EffectiveParameters.expected_rime_dll_sha256)",
        "-ExpectedRimeDeployerSha256 $(Quote-CommandArg $EffectiveParameters.expected_rime_deployer_sha256)",
        "-ExpectedLunaPinyinCommit $(Quote-CommandArg $EffectiveParameters.expected_luna_pinyin_commit)",
        "-ExpectedPreludeCommit $(Quote-CommandArg $EffectiveParameters.expected_prelude_commit)",
        "-ExpectedEssayCommit $(Quote-CommandArg $EffectiveParameters.expected_essay_commit)",
        "-ExpectedStrokeCommit $(Quote-CommandArg $EffectiveParameters.expected_stroke_commit)"
    ) -join " "
    $CuratorEffectiveParameters = [ordered]@{
        pages = $RawPaths.pages
        composition = $RawPaths.composition
        metadata = $RawPaths.metadata
        output = $EffectiveParameters.output
    }
    $CuratorInvocation = @(
        "python scripts/curate-m59-luna-composition.py",
        (Quote-CommandArg $CuratorEffectiveParameters.pages),
        (Quote-CommandArg $CuratorEffectiveParameters.composition),
        (Quote-CommandArg $CuratorEffectiveParameters.metadata),
        (Quote-CommandArg $CuratorEffectiveParameters.output)
    ) -join " "
    $Commands = [ordered]@{
        deploy = "rime_deployer.exe --build disposable/user disposable/shared disposable/user/build"
        capture = $EffectiveInvocation
        curate = $CuratorInvocation
    }
    $OutputProvenance = [ordered]@{
        path = $EffectiveParameters.output
        existed_before_capture = $false
        write_policy = "canonical_utf8_no_bom_lf_one_terminal_lf_create_new"
        generated_by = "scripts/curate-m59-luna-composition.py"
        raw_paths = $RawPaths
    }
    $Metadata = [ordered]@{
        rime_dll_sha256 = $ActualRimeDllSha256
        rime_deployer_sha256 = $ActualRimeDeployerSha256
        schema_source_repo = "rime/rime-luna-pinyin"
        schema_source_commit = $SchemaSourceCommit
        dependency_commits = [ordered]@{
            "rime/rime-prelude" = $PreludeCommit
            "rime/rime-essay" = $EssayCommit
            "rime/rime-stroke" = $StrokeCommit
        }
        source_repositories_clean = [ordered]@{
            "rime/rime-luna-pinyin" = $true
            "rime/rime-prelude" = $true
            "rime/rime-essay" = $true
            "rime/rime-stroke" = $true
        }
        source_git_trees = $SourceGitTrees
        queried_data = [ordered]@{
            shared_path = "disposable/shared"
            build_path = "disposable/user/build"
            shared_tree_sha256 = $SharedTreeSha256BeforeCapture
            build_tree_sha256 = $BuildTreeSha256BeforeCapture
            tree_hash_algorithm = "sha256 of ordinal path<TAB>file-sha256 rows joined by LF with final LF"
            mutation_policy = "raw shared/build hashes must remain identical before/after capture"
            deployment_policy = "clean disposable deploy from pinned tracked source files plus pinned upstream OpenCC"
            timestamp_normalization_policy = "all staged files use fixed half-second LastWriteTimeUtc verified by exact FileTimeUtc readback before deployment"
            staged_timestamp_utc = "2000-01-01T00:00:00.500Z"
            default_custom_sha256 = $DefaultCustomSha256
            opencc_tree_sha256 = $OpenCcTreeSha256
        }
        tool_source = [ordered]@{
            repository = "yune"
            commit = $ToolState.commit
            git_tree = $ToolState.git_tree
            clean = [bool]$ToolState.clean
            dirty = -not $ToolState.clean
            status_short = @($ToolState.status_short)
        }
        tool_hashes = [ordered]@{
            capture_script_sha256 = $CaptureScriptSha256
            curator_sha256 = $CuratorSha256
            probe_sha256 = $ProbeSha256
        }
        schema_id = "luna_pinyin"
        modules = @($modules)
        inputs = @($pagingInputs)
        input_count = $pagingInputs.Count
        page_sizes_observed = @($PageSizesObserved)
        captured_all_pages = $true
        page_policy = "RimeProbe.Capture all pages; incomplete pagination is fatal"
        runtime_options = $RuntimeOptions
        runtime_options_source = $RuntimeOptionsSource
        additional_runtime_option_patches = @($AdditionalRuntimeOptionPatches)
        serialization = $Serialization
        commands = $Commands
        actual_invocation = $EffectiveInvocation
        effective_parameters = $EffectiveParameters
        curator_effective_parameters = $CuratorEffectiveParameters
        output_provenance = $OutputProvenance
    }
    Write-Utf8NoBom $PagesRaw (ConvertTo-CanonicalJsonText $pageSnaps)
    Write-Utf8NoBom $ComposeRaw (ConvertTo-CanonicalJsonText $composeSnaps)
    Write-Utf8NoBom $MetadataRaw (ConvertTo-CanonicalJsonText $Metadata)
    Assert-CanonicalJsonFile $PagesRaw "Lane-B raw pages JSON"
    Assert-CanonicalJsonFile $ComposeRaw "Lane-B raw composition JSON"
    Assert-CanonicalJsonFile $MetadataRaw "Lane-B raw metadata JSON"

    & python $Curate $PagesRaw $ComposeRaw $MetadataRaw $Output
    if ($LASTEXITCODE -ne 0) {
        throw "curation failed with exit code $LASTEXITCODE"
    }
    Assert-CanonicalJsonFile $Output "Lane-B curated oracle JSON"
}
finally {
    $env:PATH = $OldPath
    $ResolvedWorkRoot = [System.IO.Path]::GetFullPath($WorkRoot)
    if (-not (Test-Path -LiteralPath $WorkRootMarker)) {
        throw "Refusing to remove an unmarked Lane-B capture root: $ResolvedWorkRoot"
    }
    if ([System.IO.File]::ReadAllText($WorkRootMarker, [System.Text.Encoding]::UTF8) -ne $WorkRootMarkerText) {
        throw "Refusing to remove a Lane-B capture root with an invalid marker: $ResolvedWorkRoot"
    }
    $TempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath()).TrimEnd("\", "/")
    if (-not $ResolvedWorkRoot.StartsWith(
            $TempRoot + [System.IO.Path]::DirectorySeparatorChar,
            [System.StringComparison]::OrdinalIgnoreCase
        ) -or $ResolvedWorkRoot -eq $TempRoot) {
        throw "Refusing to remove unsafe Lane-B capture root: $ResolvedWorkRoot"
    }
    Remove-Item -LiteralPath $ResolvedWorkRoot -Recurse -Force
}
Write-Output "captured M59 luna composition oracle -> $Output"
