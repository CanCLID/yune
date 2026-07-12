# Stage a source shared-data tree with one schema-local validation patch.
#
# This helper is intentionally separate from capture-yune-candidate-order.ps1:
# historical M59 evidence classifiers pin that capture tool byte-for-byte.  The
# Rime reads `.custom.yaml` from user data, while the generic capture tool owns
# creation of its disposable user directory.  This helper therefore applies a
# tightly validated one-key patch to `<schema-id>.schema.yaml` in a disposable
# shared-data copy.  The existing capture tool then independently records both
# its source and copied-tree hashes.
param(
    [Parameter(Mandatory = $true)]
    [string]$SourceSharedDataDir,
    [Parameter(Mandatory = $true)]
    [string]$OutputSharedDataDir,
    [Parameter(Mandatory = $true)]
    [string]$SchemaId,
    [Parameter(Mandatory = $true)]
    [string]$SchemaCustomOverlay,
    [Parameter(Mandatory = $true)]
    [string]$ManifestOutput
)

$ErrorActionPreference = "Stop"
$RepoRoot = [System.IO.Path]::GetFullPath((Resolve-Path (Join-Path $PSScriptRoot "..")).Path)

function Full-ExistingPath([string]$Path, [string]$Label, [bool]$RequireDirectory) {
    if (-not (Test-Path -LiteralPath $Path)) {
        throw "Missing $Label`: $Path"
    }
    $Resolved = [System.IO.Path]::GetFullPath((Resolve-Path -LiteralPath $Path).Path)
    $Item = Get-Item -LiteralPath $Resolved -Force
    if ($RequireDirectory -and -not $Item.PSIsContainer) {
        throw "$Label must be a directory: $Path"
    }
    if (-not $RequireDirectory -and $Item.PSIsContainer) {
        throw "$Label must be a file: $Path"
    }
    return $Resolved
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

function Evidence-Path([string]$Path) {
    $Full = [System.IO.Path]::GetFullPath($Path)
    $Root = $RepoRoot.TrimEnd("\", "/")
    if ($Full.StartsWith($Root + [System.IO.Path]::DirectorySeparatorChar, [System.StringComparison]::OrdinalIgnoreCase)) {
        return $Full.Substring($Root.Length + 1).Replace("\", "/")
    }
    return $Full.Replace("\", "/")
}

function Get-CanonicalWindowsPath([string]$Path) {
    if (-not ("YuneM59SchemaStageFinalPath" -as [type])) {
        Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;
using Microsoft.Win32.SafeHandles;

public static class YuneM59SchemaStageFinalPath {
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
    $Canonical = [YuneM59SchemaStageFinalPath]::Resolve($Existing)
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

function Write-Utf8NoBomCreateNew([string]$Path, [string]$Text) {
    $Parent = Split-Path -Parent $Path
    if (-not [string]::IsNullOrWhiteSpace($Parent) -and
        -not (Test-Path -LiteralPath $Parent)) {
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

function Git-State([string]$Path) {
    $Commit = (& git -C $Path rev-parse HEAD).Trim().ToLowerInvariant()
    if ($LASTEXITCODE -ne 0 -or $Commit -notmatch '^[0-9a-f]{40}$') {
        throw "Unable to resolve the Yune source commit."
    }
    $Tree = (& git -C $Path rev-parse "HEAD^{tree}").Trim().ToLowerInvariant()
    if ($LASTEXITCODE -ne 0 -or $Tree -notmatch '^[0-9a-f]{40}$') {
        throw "Unable to resolve the Yune source tree."
    }
    $Status = @(& git -C $Path status --short)
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to read the Yune worktree status."
    }
    return [ordered]@{
        commit = $Commit
        tree = $Tree
        clean = $Status.Count -eq 0
        status_short = @($Status)
    }
}

function Assert-CanonicalValidationOverlay([string]$Path) {
    $Encoding = [System.Text.UTF8Encoding]::new($false, $true)
    $Text = [System.IO.File]::ReadAllText($Path, $Encoding)
    $Normalized = $Text.Replace("`r`n", "`n").Replace("`r", "`n")
    $Expected = "patch:`n  translator/yune_sentence_policy: upstream_script`n"
    if ($Normalized -cne $Expected) {
        throw "Schema validation overlay must contain only the canonical upstream-table policy patch."
    }
}

function Assert-StageableSchema(
    [string]$SchemaPath,
    [string]$ExpectedSchemaId
) {
    if (-not (Test-Path -LiteralPath $SchemaPath)) {
        throw "Shared data is missing $ExpectedSchemaId.schema.yaml."
    }
    $Item = Get-Item -LiteralPath $SchemaPath -Force
    if ($Item.PSIsContainer) {
        throw "Schema path must be a file: $SchemaPath"
    }
    $Bytes = [System.IO.File]::ReadAllBytes($SchemaPath)
    if ($Bytes.Length -ge 3 -and
        $Bytes[0] -eq 0xef -and $Bytes[1] -eq 0xbb -and $Bytes[2] -eq 0xbf) {
        throw "Schema must use UTF-8 without BOM."
    }
    $Encoding = [System.Text.UTF8Encoding]::new($false, $true)
    $Text = $Encoding.GetString($Bytes)

    $SchemaKeys = [System.Text.RegularExpressions.Regex]::Matches(
        $Text,
        '(?m)^schema[ \t]*:'
    )
    $SchemaHeaders = [System.Text.RegularExpressions.Regex]::Matches(
        $Text,
        '(?m)^schema:[ \t]*(?:#[^\r\n]*)?(?<newline>\r?\n)'
    )
    if ($SchemaKeys.Count -ne 1 -or $SchemaHeaders.Count -ne 1) {
        throw "Schema must contain exactly one top-level schema block mapping."
    }
    $SchemaHeader = $SchemaHeaders[0]
    $SchemaBlockStart = $SchemaHeader.Index + $SchemaHeader.Length
    $SchemaRemainder = $Text.Substring($SchemaBlockStart)
    $NextTopLevelAfterSchema = [System.Text.RegularExpressions.Regex]::Match(
        $SchemaRemainder,
        '(?m)^[A-Za-z0-9_][^\r\n]*:'
    )
    $SchemaBlockLength = if ($NextTopLevelAfterSchema.Success) {
        $NextTopLevelAfterSchema.Index
    }
    else {
        $SchemaRemainder.Length
    }
    $SchemaBlock = $SchemaRemainder.Substring(0, $SchemaBlockLength)
    $SchemaLines = [System.Text.RegularExpressions.Regex]::Matches(
        $SchemaBlock,
        '(?m)^(?<indent> +)(?<body>[^\r\n]*)\r?$'
    ) | Where-Object {
        -not [string]::IsNullOrWhiteSpace($_.Groups['body'].Value) -and
        -not $_.Groups['body'].Value.TrimStart().StartsWith('#')
    }
    if ($SchemaLines.Count -eq 0) {
        throw "Top-level schema block must contain mapping entries."
    }
    $DirectIndent = ($SchemaLines | ForEach-Object {
            $_.Groups['indent'].Value.Length
        } | Measure-Object -Minimum).Minimum
    $DirectSchemaIdLines = @($SchemaLines | Where-Object {
            $_.Groups['indent'].Value.Length -eq $DirectIndent -and
            $_.Groups['body'].Value -match '^schema_id[ \t]*:'
        })
    if ($DirectSchemaIdLines.Count -ne 1) {
        throw "Top-level schema block must contain exactly one direct schema_id declaration."
    }
    $SchemaIdPattern = [System.Text.RegularExpressions.Regex]::new(
        '^(?:schema_id)[ \t]*:[ \t]*(?:(?<bare>[A-Za-z0-9_][A-Za-z0-9_.-]*)|''(?<single>[A-Za-z0-9_][A-Za-z0-9_.-]*)''|"(?<double>[A-Za-z0-9_][A-Za-z0-9_.-]*)")[ \t]*(?:#[^\r\n]*)?$'
    )
    $SchemaIdMatch = $SchemaIdPattern.Match($DirectSchemaIdLines[0].Groups['body'].Value)
    if (-not $SchemaIdMatch.Success) {
        throw "schema/schema_id must be one logical schema identifier."
    }
    $DeclaredSchemaId = @(
        $SchemaIdMatch.Groups['bare'].Value,
        $SchemaIdMatch.Groups['single'].Value,
        $SchemaIdMatch.Groups['double'].Value
    ) | Where-Object { -not [string]::IsNullOrEmpty($_) } | Select-Object -First 1
    if (-not [string]::Equals(
            $DeclaredSchemaId,
            $ExpectedSchemaId,
            [System.StringComparison]::Ordinal
        )) {
        throw "schema/schema_id '$DeclaredSchemaId' does not match requested SchemaId '$ExpectedSchemaId'."
    }

    $TranslatorKeys = [System.Text.RegularExpressions.Regex]::Matches(
        $Text,
        '(?m)^translator[ \t]*:'
    )
    $TranslatorHeaders = [System.Text.RegularExpressions.Regex]::Matches(
        $Text,
        '(?m)^translator:[ \t]*(?:#[^\r\n]*)?(?<newline>\r?\n)'
    )
    if ($TranslatorKeys.Count -ne 1 -or $TranslatorHeaders.Count -ne 1) {
        throw "Schema must contain exactly one top-level translator block mapping."
    }

    return [pscustomobject]@{
        text = $Text
        translator_header = $TranslatorHeaders[0]
    }
}

function Apply-CanonicalValidationOverlay(
    [string]$SchemaPath,
    [string]$ExpectedSchemaId
) {
    $Validation = Assert-StageableSchema $SchemaPath $ExpectedSchemaId
    $Text = $Validation.text
    $Match = $Validation.translator_header
    $BlockStart = $Match.Index + $Match.Length
    $Remainder = $Text.Substring($BlockStart)
    $NextTopLevel = [System.Text.RegularExpressions.Regex]::Match(
        $Remainder,
        '(?m)^[A-Za-z0-9_][^\r\n]*:'
    )
    $BlockLength = if ($NextTopLevel.Success) { $NextTopLevel.Index } else { $Remainder.Length }
    $TranslatorBlock = $Remainder.Substring(0, $BlockLength)
    if ([System.Text.RegularExpressions.Regex]::IsMatch(
            $TranslatorBlock,
            '(?m)^[ \t]+yune_sentence_policy\s*:'
        )) {
        throw "Staged schema already declares translator/yune_sentence_policy."
    }
    $Newline = $Match.Groups['newline'].Value
    $Patched = $Text.Substring(0, $BlockStart) +
        "  yune_sentence_policy: upstream_script$Newline" +
        $Text.Substring($BlockStart)
    [System.IO.File]::WriteAllText($SchemaPath, $Patched, [System.Text.UTF8Encoding]::new($false))
}

if ($SchemaId -notmatch '^[A-Za-z0-9_][A-Za-z0-9_.-]*$') {
    throw "SchemaId must be a logical schema identifier, not a path."
}

$SourceSharedDataDir = Full-ExistingPath $SourceSharedDataDir "source shared-data directory" $true
$SchemaCustomOverlay = Full-ExistingPath $SchemaCustomOverlay "schema custom overlay" $false
$OutputSharedDataDir = [System.IO.Path]::GetFullPath($OutputSharedDataDir)
$ManifestOutput = [System.IO.Path]::GetFullPath($ManifestOutput)

if (Test-Path -LiteralPath $OutputSharedDataDir) {
    throw "Output shared-data directory must not already exist: $OutputSharedDataDir"
}
if (Test-Path -LiteralPath $ManifestOutput) {
    throw "Manifest output must not already exist: $ManifestOutput"
}
$SourceSharedDataDir = Get-CanonicalWindowsPath $SourceSharedDataDir
$SchemaCustomOverlay = Get-CanonicalWindowsPath $SchemaCustomOverlay
$OutputSharedDataDir = Get-CanonicalWindowsPath $OutputSharedDataDir
$ManifestOutput = Get-CanonicalWindowsPath $ManifestOutput
$SourcePrefix = $SourceSharedDataDir.TrimEnd("\", "/") + [System.IO.Path]::DirectorySeparatorChar
$OutputPrefix = $OutputSharedDataDir.TrimEnd("\", "/") + [System.IO.Path]::DirectorySeparatorChar
if ([string]::Equals($SourceSharedDataDir, $OutputSharedDataDir, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Source and output shared-data directories must be distinct."
}
if ($OutputSharedDataDir.StartsWith($SourcePrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Output shared-data directory must not be inside the source tree."
}
if ($SourceSharedDataDir.StartsWith($OutputPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Source shared-data directory must not be inside the output tree."
}
if ($ManifestOutput.StartsWith($OutputPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Manifest output must stay outside the staged shared-data tree."
}
if ($ManifestOutput.StartsWith($SourcePrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Manifest output must stay outside the source shared-data tree."
}

$ToolSha256 = File-Sha256 $PSCommandPath
$SourceTreeSha256 = Tree-Sha256 $SourceSharedDataDir
$OverlaySha256 = File-Sha256 $SchemaCustomOverlay
$RepoState = Git-State $RepoRoot
$DestinationName = "$SchemaId.schema.yaml"
$SourceSchema = Join-Path $SourceSharedDataDir $DestinationName
if (-not (Test-Path -LiteralPath $SourceSchema)) {
    throw "Source shared data is missing $DestinationName."
}
$SourceSchemaSha256 = File-Sha256 $SourceSchema
Assert-CanonicalValidationOverlay $SchemaCustomOverlay
Assert-StageableSchema $SourceSchema $SchemaId | Out-Null

New-Item -ItemType Directory -Path $OutputSharedDataDir | Out-Null
Get-ChildItem -LiteralPath $SourceSharedDataDir -Force | ForEach-Object {
    Copy-Item -LiteralPath $_.FullName -Destination $OutputSharedDataDir -Recurse -Force
}
$StagedSchema = Join-Path $OutputSharedDataDir $DestinationName
Apply-CanonicalValidationOverlay $StagedSchema $SchemaId

if ((Tree-Sha256 $SourceSharedDataDir) -ne $SourceTreeSha256) {
    throw "Source shared-data tree changed during staging."
}
if ((File-Sha256 $SchemaCustomOverlay) -ne $OverlaySha256) {
    throw "Schema custom overlay changed during staging."
}
$StagedSchemaSha256 = File-Sha256 $StagedSchema
if ($StagedSchemaSha256 -eq $SourceSchemaSha256) {
    throw "Schema validation overlay did not change the staged schema."
}
$StagedTreeSha256 = Tree-Sha256 $OutputSharedDataDir

$Manifest = [ordered]@{
    tool = "stage-m59-schema-validation-overlay.ps1"
    tool_version = "2"
    tool_sha256 = $ToolSha256
    yune_source = $RepoState
    source_shared_data_dir = Evidence-Path $SourceSharedDataDir
    source_shared_tree_sha256 = $SourceTreeSha256
    schema_id = $SchemaId
    schema_patch_overlay = Evidence-Path $SchemaCustomOverlay
    schema_patch_overlay_sha256 = $OverlaySha256
    schema_patch_key = "translator/yune_sentence_policy"
    schema_patch_value = "upstream_script"
    patched_schema_destination = $DestinationName
    source_schema_sha256 = $SourceSchemaSha256
    staged_schema_sha256 = $StagedSchemaSha256
    output_shared_data_dir = Evidence-Path $OutputSharedDataDir
    staged_shared_tree_sha256 = $StagedTreeSha256
}
$Json = $Manifest | ConvertTo-Json -Depth 20
$Json = $Json.Replace("`r`n", "`n").Replace("`r", "`n").TrimEnd([char]10) + "`n"
Write-Utf8NoBomCreateNew $ManifestOutput $Json
Write-Output "staged M59 schema validation overlay -> $OutputSharedDataDir"
Write-Output "staging manifest -> $ManifestOutput"
