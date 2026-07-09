# Captures the librime 1.17.0 cangjie5 oracle over pinned rime/rime-cangjie for
# D-48 item 2 (cangjie5 order-parity onboarding). The decisive question this
# capture answers: does upstream librime's cangjie5 COMPOSE the three owner
# multi-character phrases from their concatenated single-character shape codes?
#   hwmvsqtt  -> 粵拼   ( 粵=hwmvs + 拼=qtt )
#   ebcnyripm -> 測試   ( 測=ebcn  + 試=yripm )
#   takohaeosk-> 莫伯洢 ( 莫=tak   + 伯=oha + 洢=eosk )
# Plus the per-character shape codes (composition pins) and non-owner control
# compositions (香港=hdaetcu, 中文=lyk) proving generality, and the canonical
# single-char control a -> 日.
#
# Mirrors scripts/capture-upstream-rime-cantonese.ps1: pinned oracle provenance
# (rime.dll / rime_deployer.exe sha256), all-pages capture (hard pagination_error
# on any non-advancing page), byte-content-verifiable regeneration. This .ps1 is
# intentionally pure-ASCII; CJK candidate text is carried in-process from the DLL
# through ConvertTo-Json + a UTF-8 (no BOM) writer, never a native stdout pipe.
param(
    [string]$OracleRoot,
    [string[]]$Inputs,
    [string]$Output,
    [string]$ExpectedRimeDllSha256 = "86b4c7357d4c6d293ce5589b234d8859ca2ac30923a03bedfa3926eeaf97fb0b",
    [string]$ExpectedRimeDeployerSha256 = "3abb72b5bb56fcafcfe925d533ae5f832c68d5a0bc9952fd0eea0682fb1ab071"
)

$ErrorActionPreference = "Stop"

$RepoRoot = [System.IO.Path]::GetFullPath((Resolve-Path (Join-Path $PSScriptRoot "..")).Path)
if ([string]::IsNullOrWhiteSpace($OracleRoot)) {
    $OracleRoot = Join-Path $RepoRoot "target\upstream-oracle\1.17.0"
}
if ($null -eq $Inputs -or $Inputs.Count -eq 0) {
    $Inputs = @(
        # --- owner composition rows (full concatenated shape codes) ---
        "hwmvsqtt",   # 粵拼
        "ebcnyripm",  # 測試
        "takohaeosk", # 莫伯洢
        # --- per-character shape codes (composition pins / single-char generality) ---
        "hwmvs",      # 粵
        "qtt",        # 拼
        "ebcn",       # 測
        "yripm",      # 試
        "tak",        # 莫
        "oha",        # 伯
        "eosk",       # 洢
        # --- non-owner control compositions (generality; in no owner allowlist) ---
        "hdaetcu",    # 香港 (composes in librime; control)
        "lyk"         # 中文 (phrase at pos 1 behind single-char 奜; control)
    )
}
if ($Inputs.Count -eq 1 -and $Inputs[0].Contains(",")) {
    $Inputs = $Inputs[0].Split(",") |
        ForEach-Object { $_.Trim() } |
        Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
}
if ([string]::IsNullOrWhiteSpace($Output)) {
    $Output = Join-Path $RepoRoot "crates\yune-core\tests\fixtures\upstream-1.17.0\cangjie5-composition.json"
}

$OracleRoot = [System.IO.Path]::GetFullPath($OracleRoot)
$Output = [System.IO.Path]::GetFullPath($Output)
$Extract = Join-Path $OracleRoot "extract"
$Shared = Join-Path $OracleRoot "cangjie5-shared"
$User = Join-Path $OracleRoot "cangjie5-user"
$Build = Join-Path $User "build"
$SchemaRoot = Join-Path $OracleRoot "schema-src"
$ProbeSource = Join-Path $RepoRoot "scripts\oracle-rime-probe.cs"
$RimeDll = Join-Path $Extract "dist\lib\rime.dll"
$RimeDeployer = Join-Path $Extract "dist\bin\rime_deployer.exe"

# cangjie5 needs: rime-cangjie (dict/schema), rime-essay (preset vocabulary),
# rime-prelude (default/key_bindings/punctuation/symbols presets), and
# rime-luna-pinyin (the luna_quanpin reverse-lookup dependency). Copy the same
# repo set the rime-cantonese capture uses so every dependency is present.
$RequiredRepos = [ordered]@{
    "rime/rime-cangjie" = "rime-cangjie"
    "rime/rime-prelude" = "rime-prelude"
    "rime/rime-luna-pinyin" = "rime-luna-pinyin"
    "rime/rime-essay" = "rime-essay"
    "rime/rime-stroke" = "rime-stroke"
    "rime/rime-cantonese" = "rime-cantonese"
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
    if ([string]::IsNullOrWhiteSpace($Path)) { return $Path }
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
        throw "Missing required upstream cangjie5 capture input: $Path"
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
    - schema: cangjie5
"@ | Set-Content -LiteralPath (Join-Path $Shared "default.custom.yaml") -Encoding UTF8

New-Item -ItemType Directory -Force -Path $Build | Out-Null
$env:PATH = (Join-Path $Extract "dist\lib") + ";" + (Join-Path $Extract "bin") + ";" + $env:PATH
& (Join-Path $Extract "dist\bin\rime_deployer.exe") --build $User $Shared $Build
if ($LASTEXITCODE -ne 0) {
    throw "rime_deployer.exe --build failed with exit code $LASTEXITCODE"
}

Add-Type -Path $ProbeSource
$Modules = [string[]]@("default")
$Cases = [RimeProbe]::Capture($Shared, $User, $Build, "cangjie5", $Modules, [string[]]$Inputs)
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
    milestone = "M59"
    task = "D-48 item 2: cangjie5 order-parity onboarding"
    status = "cangjie5_capture_complete"
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
        yune_facing_schema_id = "cangjie5"
        source_repo = "rime/rime-cangjie"
        source_commit = $RepoCommits["rime/rime-cangjie"]
        dependency_commits = $RepoCommits
        note = "Upstream rime/rime-cangjie (sort: by_weight, use_preset_vocabulary, encoder, max_phrase_length 7). NOTE: the shipped Yune product cangjie5 is a DIFFERENT dict (Jackchows Cangjie5, sort: original, max_phrase_length 1) - this lane validates the UPSTREAM rime-cangjie order, not the product dict."
    }
    options = [ordered]@{
        runtime_option_patches = @()
        custom_yaml = "default.custom.yaml only selects cangjie5"
        schema_defaults = "ascii_mode 0; full_shape 0; ascii_punct 0; zh_hans 0 (probe defaults)"
        page_sizes_observed = @($Pages)
    }
    commands = [ordered]@{
        deploy = "rime_deployer.exe --build $(Convert-ToEvidencePath $User) $(Convert-ToEvidencePath $Shared) $(Convert-ToEvidencePath $Build)"
        capture = "scripts/capture-upstream-cangjie5.ps1"
    }
    composition_rows = @(
        [ordered]@{ input = "hwmvsqtt"; target = "粵拼"; target_codepoints = "U+7CB5 U+62FC"; char_codes = @("粵=hwmvs", "拼=qtt"); provenance = "owner composition row (D-47/D-48); oracle_backed pending capture" }
        [ordered]@{ input = "ebcnyripm"; target = "測試"; target_codepoints = "U+6E2C U+8A66"; char_codes = @("測=ebcn", "試=yripm"); provenance = "owner composition row (D-47/D-48); oracle_backed pending capture" }
        [ordered]@{ input = "takohaeosk"; target = "莫伯洢"; target_codepoints = "U+83AB U+4F2F U+6D22"; char_codes = @("莫=tak", "伯=oha", "洢=eosk"); provenance = "owner composition row (D-47/D-48); non-lexicon phrase (same target as luna moboyi); oracle_backed pending capture" }
    )
    inputs = @($Inputs)
    cases = $Cases
}

Write-Utf8NoBom $Output (($Evidence | ConvertTo-Json -Depth 100) + "`n")
Write-Host "Wrote upstream cangjie5 composition capture to $Output"

# Embed the minimal upstream source slice (exact-code cohorts + rime-essay
# weights) so the non-circular char-by-char test can drive Yune's real
# translator without any Yune-derived expected values. Deterministic; part of
# the byte-content-verifiable regeneration pipeline.
$Curate = Join-Path $RepoRoot "scripts\curate-upstream-cangjie5.py"
$CangjieSrc = Join-Path $SchemaRoot "rime-cangjie"
$EssayTxt = Join-Path $SchemaRoot "rime-essay\essay.txt"
& python $Curate $Output $CangjieSrc $EssayTxt
if ($LASTEXITCODE -ne 0) {
    throw "cangjie5 source-slice curation failed with exit code $LASTEXITCODE"
}
Write-Host "Augmented capture with upstream source slice for the char-by-char test"
