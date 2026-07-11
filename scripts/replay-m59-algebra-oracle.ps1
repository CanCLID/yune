[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$OracleBinaryRoot,

    [string]$Workspace = "target/upstream-oracle/1.17.0/m59-algebra-properties-replay"
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot
$FixtureRoot = Join-Path $RepoRoot "crates/yune-core/tests/fixtures/upstream-1.17.0"
$FixturePath = Join-Path $FixtureRoot "m59-algebra-properties.json"
$OracleManifestPath = Join-Path $FixtureRoot "oracle-manifest.json"
$SourceRoot = Join-Path $FixtureRoot "m59-algebra-properties-source"
$Decoder = Join-Path $RepoRoot "scripts/decode-m59-algebra-prisms.py"
$OracleManifest = Get-Content -Raw -Encoding UTF8 -LiteralPath $OracleManifestPath | ConvertFrom-Json
$FixtureRows = @($OracleManifest.files | Where-Object { $_.path -eq "m59-algebra-properties.json" })
if ($FixtureRows.Count -ne 1) {
    throw "oracle manifest must contain exactly one m59-algebra-properties.json row"
}
$ActualFixtureHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $FixturePath).Hash.ToLowerInvariant()
if ($ActualFixtureHash -ne ([string]$FixtureRows[0].sha256).ToLowerInvariant()) {
    throw "algebra fixture does not match its oracle-manifest binding: actual=$ActualFixtureHash manifest=$($FixtureRows[0].sha256)"
}
$Fixture = Get-Content -Raw -Encoding UTF8 -LiteralPath $FixturePath | ConvertFrom-Json

$OracleBinaryRoot = (Resolve-Path -LiteralPath $OracleBinaryRoot).Path
$DeployerCandidates = @(
    (Join-Path $OracleBinaryRoot "rime_deployer.exe"),
    (Join-Path $OracleBinaryRoot "bin\rime_deployer.exe"),
    (Join-Path $OracleBinaryRoot "binaries\rime_deployer.exe")
)
$Deployer = $DeployerCandidates | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } | Select-Object -First 1
if (-not $Deployer) {
    throw "rime_deployer.exe not found under $OracleBinaryRoot"
}
$BinaryDirectory = Split-Path -Parent $Deployer
$RimeDllCandidates = @(
    (Join-Path $BinaryDirectory "rime.dll"),
    (Join-Path $OracleBinaryRoot "lib\rime.dll"),
    (Join-Path (Split-Path -Parent $BinaryDirectory) "lib\rime.dll"),
    (Join-Path $OracleBinaryRoot "binaries\rime.dll")
)
$RimeDll = $RimeDllCandidates | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } | Select-Object -First 1
if (-not $RimeDll) {
    throw "rime.dll not found under $OracleBinaryRoot"
}
$RimeDllDirectory = Split-Path -Parent $RimeDll

function Assert-FileBinding {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][long]$Bytes,
        [Parameter(Mandatory = $true)][string]$Sha256,
        [Parameter(Mandatory = $true)][string]$Role
    )
    $Info = Get-Item -LiteralPath $Path
    $ActualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
    if ($Info.Length -ne $Bytes -or $ActualHash -ne $Sha256.ToLowerInvariant()) {
        throw "$Role binding mismatch: bytes=$($Info.Length)/$Bytes sha256=$ActualHash/$Sha256"
    }
}

function Get-BytesSha256 {
    param([Parameter(Mandatory = $true)][byte[]]$Bytes)
    $Hasher = [Security.Cryptography.SHA256]::Create()
    try {
        return [BitConverter]::ToString($Hasher.ComputeHash($Bytes)).Replace("-", "").ToLowerInvariant()
    }
    finally {
        $Hasher.Dispose()
    }
}

function Resolve-ContainedPath {
    param(
        [Parameter(Mandatory = $true)][string]$Root,
        [Parameter(Mandatory = $true)][string]$Relative,
        [Parameter(Mandatory = $true)][string]$Role
    )
    if ([IO.Path]::IsPathRooted($Relative)) {
        throw "$Role must be relative: $Relative"
    }
    $FullRoot = [IO.Path]::GetFullPath($Root).TrimEnd(
        [IO.Path]::DirectorySeparatorChar,
        [IO.Path]::AltDirectorySeparatorChar
    )
    $FullPath = [IO.Path]::GetFullPath((Join-Path $FullRoot $Relative))
    $Prefix = $FullRoot + [IO.Path]::DirectorySeparatorChar
    if (-not $FullPath.StartsWith($Prefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "$Role escapes its root: $Relative"
    }
    return $FullPath
}

function Get-CanonicalSourceBytes {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][long]$CheckoutBytes,
        [Parameter(Mandatory = $true)][string]$CheckoutSha256,
        [Parameter(Mandatory = $true)][long]$MaterializedBytes,
        [Parameter(Mandatory = $true)][string]$MaterializedSha256,
        [Parameter(Mandatory = $true)][string]$Materialization,
        [Parameter(Mandatory = $true)][string]$Role
    )
    $ExpectedCheckoutHash = $CheckoutSha256.ToLowerInvariant()
    $ExpectedMaterializedHash = $MaterializedSha256.ToLowerInvariant()
    [byte[]]$Raw = [IO.File]::ReadAllBytes($Path)
    $RawHash = Get-BytesSha256 -Bytes $Raw
    if ($Raw.LongLength -ne $CheckoutBytes -or $RawHash -ne $ExpectedCheckoutHash) {
        throw "$Role checkout binding mismatch: bytes=$($Raw.LongLength)/$CheckoutBytes sha256=$RawHash/$CheckoutSha256"
    }

    if ($Materialization -eq "identity") {
        if ($Raw.LongLength -ne $MaterializedBytes -or $RawHash -ne $ExpectedMaterializedHash) {
            throw "$Role identity materialization does not match the captured bytes/hash"
        }
        return ,$Raw
    }

    # Six checked-in replay source files intentionally omit the capture-only
    # final blank LF. Restore only that declared byte, and only when its exact
    # captured byte count and hash prove the materialized upstream payload.
    if ($Materialization -eq "append_terminal_lf" -and
        $Raw.LongLength + 1 -eq $MaterializedBytes) {
        [byte[]]$WithTerminalLf = [byte[]]::new($Raw.Length + 1)
        [Array]::Copy($Raw, $WithTerminalLf, $Raw.Length)
        $WithTerminalLf[$Raw.Length] = 0x0A
        $MaterializedHash = Get-BytesSha256 -Bytes $WithTerminalLf
        if ($MaterializedHash -eq $ExpectedMaterializedHash) {
            return ,$WithTerminalLf
        }
    }

    throw "$Role materialization mismatch: policy=$Materialization materialized-bytes=$MaterializedBytes materialized-sha256=$MaterializedSha256"
}

Assert-FileBinding -Path $Deployer `
    -Bytes ([long]$Fixture.oracle.rime_deployer.bytes) `
    -Sha256 ([string]$Fixture.oracle.rime_deployer.sha256) `
    -Role "pinned rime_deployer"
Assert-FileBinding -Path $RimeDll `
    -Bytes ([long]$Fixture.oracle.rime_dll.bytes) `
    -Sha256 ([string]$Fixture.oracle.rime_dll.sha256) `
    -Role "pinned rime.dll"
Assert-FileBinding -Path $MyInvocation.MyCommand.Path `
    -Bytes ([long]$Fixture.capture.replay_script.bytes) `
    -Sha256 ([string]$Fixture.capture.replay_script.sha256) `
    -Role "M59 algebra replay script"

$WorkspacePath = if ([IO.Path]::IsPathRooted($Workspace)) {
    [IO.Path]::GetFullPath($Workspace)
}
else {
    [IO.Path]::GetFullPath((Join-Path $RepoRoot $Workspace))
}
if (Test-Path -LiteralPath $WorkspacePath) {
    throw "Replay workspace already exists; choose an absent clean path: $WorkspacePath"
}

$Shared = Join-Path $WorkspacePath "shared"
$User = Join-Path $WorkspacePath "user"
$Build = Join-Path $User "build"
$Verify = Join-Path $WorkspacePath "verify"
New-Item -ItemType Directory -Path $Shared, $Build, $Verify | Out-Null

foreach ($Source in $Fixture.capture.source_files) {
    $Relative = ([string]$Source.path).Replace('/', '\')
    $CheckedIn = Resolve-ContainedPath -Root $SourceRoot -Relative $Relative -Role "oracle source"
    [byte[]]$CanonicalBytes = Get-CanonicalSourceBytes -Path $CheckedIn `
        -CheckoutBytes ([long]$Source.checkout_bytes) `
        -CheckoutSha256 ([string]$Source.checkout_sha256) `
        -MaterializedBytes ([long]$Source.bytes) `
        -MaterializedSha256 ([string]$Source.sha256) `
        -Materialization ([string]$Source.materialization) `
        -Role "checked-in oracle source $($Source.path)"
    $Destination = Resolve-ContainedPath -Root $WorkspacePath -Relative $Relative -Role "materialized oracle source"
    $DestinationParent = Split-Path -Parent $Destination
    if (-not (Test-Path -LiteralPath $DestinationParent)) {
        New-Item -ItemType Directory -Path $DestinationParent | Out-Null
    }
    [IO.File]::WriteAllBytes($Destination, $CanonicalBytes)
    $Timestamp = [DateTimeOffset]::FromUnixTimeSeconds(
        [long]$Source.librime_timestamp_epoch_seconds
    ).UtcDateTime.AddMilliseconds(500)
    [IO.File]::SetLastWriteTimeUtc($Destination, $Timestamp)
    $ObservedTimestamp = ([DateTimeOffset](Get-Item -LiteralPath $Destination).LastWriteTimeUtc).ToUnixTimeSeconds()
    if ($ObservedTimestamp -ne [long]$Source.librime_timestamp_epoch_seconds) {
        throw "materialized oracle source timestamp mismatch for $($Source.path): observed=$ObservedTimestamp expected=$($Source.librime_timestamp_epoch_seconds)"
    }
}

$PreviousPath = $env:PATH
try {
    $env:PATH = "$BinaryDirectory;$RimeDllDirectory;$PreviousPath"
    & $Deployer --build $User $Shared $Build
    if ($LASTEXITCODE -ne 0) {
        throw "pinned rime_deployer failed with exit $LASTEXITCODE"
    }
}
finally {
    $env:PATH = $PreviousPath
}

$DefaultSourceRows = @($Fixture.capture.source_files | Where-Object { $_.path -eq "shared/default.yaml" })
if ($DefaultSourceRows.Count -ne 1) {
    throw "fixture must contain exactly one shared/default.yaml timestamp binding"
}
$ExpectedDefaultTimestamp = [long]$DefaultSourceRows[0].librime_timestamp_epoch_seconds
foreach ($Case in $Fixture.cases) {
    $SchemaId = [string]$Case.schema_id
    $SchemaSourcePath = "shared/$SchemaId.schema.yaml"
    $SchemaSourceRows = @($Fixture.capture.source_files | Where-Object { $_.path -eq $SchemaSourcePath })
    if ($SchemaSourceRows.Count -ne 1) {
        throw "fixture must contain exactly one timestamp binding for $SchemaSourcePath"
    }
    $ExpectedSchemaTimestamp = [long]$SchemaSourceRows[0].librime_timestamp_epoch_seconds
    $DeployedSchema = Resolve-ContainedPath -Root $Build -Relative "$SchemaId.schema.yaml" -Role "deployed oracle schema"
    if (-not (Test-Path -LiteralPath $DeployedSchema -PathType Leaf)) {
        throw "missing deployed oracle schema: $DeployedSchema"
    }
    $DeployedSchemaText = [IO.File]::ReadAllText($DeployedSchema)
    $DefaultPattern = "(?m)^    default: $ExpectedDefaultTimestamp\r?$"
    $SchemaPattern = "(?m)^    $([regex]::Escape($SchemaId)).schema: $ExpectedSchemaTimestamp\r?$"
    if ($DeployedSchemaText -notmatch $DefaultPattern -or $DeployedSchemaText -notmatch $SchemaPattern) {
        throw "deployed $SchemaId build timestamps do not match default=$ExpectedDefaultTimestamp schema=$ExpectedSchemaTimestamp"
    }
}

Copy-Item -LiteralPath $FixturePath -Destination (Join-Path $Verify "m59-algebra-properties.json")
foreach ($Case in $Fixture.cases) {
    $ArtifactName = [string]$Case.artifact.path
    $Generated = Resolve-ContainedPath -Root $Build -Relative $ArtifactName -Role "generated oracle prism"
    if (-not (Test-Path -LiteralPath $Generated -PathType Leaf)) {
        throw "missing generated oracle prism: $Generated"
    }
    Assert-FileBinding -Path $Generated `
        -Bytes ([long]$Case.artifact.bytes) `
        -Sha256 ([string]$Case.artifact.sha256) `
        -Role "replayed oracle prism $ArtifactName"
    $VerifyArtifact = Resolve-ContainedPath -Root $Verify -Relative $ArtifactName -Role "verified oracle prism"
    $VerifyArtifactParent = Split-Path -Parent $VerifyArtifact
    if (-not (Test-Path -LiteralPath $VerifyArtifactParent)) {
        New-Item -ItemType Directory -Path $VerifyArtifactParent | Out-Null
    }
    Copy-Item -LiteralPath $Generated -Destination $VerifyArtifact
}

$Python = Get-Command python -ErrorAction SilentlyContinue
if (-not $Python) {
    throw "python executable not found for independent prism verification"
}
& $Python.Source $Decoder --fixture (Join-Path $Verify "m59-algebra-properties.json") --verify
if ($LASTEXITCODE -ne 0) {
    throw "independent prism verification failed with exit $LASTEXITCODE"
}
& $Python.Source $Decoder --fixture (Join-Path $Verify "m59-algebra-properties.json") --verify --self-test
if ($LASTEXITCODE -ne 0) {
    throw "independent prism decoder self-test failed with exit $LASTEXITCODE"
}

Write-Output "PASS: clean M59 algebra oracle replay verified nine checkout bindings, materialized the exact source bytes/timestamps, and matched four pinned Prism/4.0 artifacts."
Write-Output "Workspace: $WorkspacePath"
