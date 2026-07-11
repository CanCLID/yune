[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$Shared = Join-Path $Root "source\shared"
$User = Join-Path $Root "source\user"
$Build = Join-Path $User "build"
$BinaryDir = Join-Path $Root "binaries"
$Deployer = Join-Path $BinaryDir "rime_deployer.exe"
$RimeDll = Join-Path $BinaryDir "rime.dll"
$ProbeSource = Join-Path $Root "probe\oracle-rime-probe.cs"
$Output = Join-Path $Root "output\capture.json"
$DeployStdout = Join-Path $Root "logs\deploy.stdout.log"
$DeployStderr = Join-Path $Root "logs\deploy.stderr.log"
$CaptureStdout = Join-Path $Root "logs\capture.stdout.log"
$CaptureStderr = Join-Path $Root "logs\capture.stderr.log"
$Commands = Join-Path $Root "commands.txt"
$Provenance = Join-Path $Root "output\provenance.json"

function Write-Utf8NoBom([string]$Path, [string]$Text) {
    $Encoding = [System.Text.UTF8Encoding]::new($false)
    [System.IO.File]::WriteAllText($Path, $Text, $Encoding)
}

function Sha256([string]$Path) {
    (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

$CommandLines = @(
    "`$env:PATH = '$BinaryDir;' + `$env:PATH",
    "& '$Deployer' --build '$User' '$Shared' '$Build'",
    "Add-Type -Path '$ProbeSource'",
    "[RimeProbe]::Capture('$Shared', '$User', '$Build', 'correction_oracle', [string[]]@('default'), [string[]]@('cu'))"
)
Write-Utf8NoBom $Commands (($CommandLines -join "`n") + "`n")

$env:PATH = $BinaryDir + ";" + $env:PATH

$DeployStarted = [DateTimeOffset]::UtcNow
& $Deployer --build $User $Shared $Build 1> $DeployStdout 2> $DeployStderr
$DeployExitCode = $LASTEXITCODE
$DeployFinished = [DateTimeOffset]::UtcNow
if ($DeployExitCode -ne 0) {
    throw "rime_deployer.exe --build failed with exit code $DeployExitCode"
}

$CaptureStarted = [DateTimeOffset]::UtcNow
try {
    Add-Type -Path $ProbeSource
    $Modules = [string[]]@("default")
    $Inputs = [string[]]@("cu")
    $Cases = [RimeProbe]::Capture($Shared, $User, $Build, "correction_oracle", $Modules, $Inputs)
    $CaptureText = (($Cases | ConvertTo-Json -Depth 30).Replace("`r`n", "`n").TrimEnd([char]10)) + "`n"
    Write-Utf8NoBom $Output $CaptureText
    Write-Utf8NoBom $CaptureStdout "capture completed successfully`n"
    Write-Utf8NoBom $CaptureStderr ""
} catch {
    Write-Utf8NoBom $CaptureStderr (($_ | Out-String) + "`n")
    throw
}
$CaptureFinished = [DateTimeOffset]::UtcNow

$DllVersion = (Get-Item -LiteralPath $RimeDll).VersionInfo
$DeployerVersion = (Get-Item -LiteralPath $Deployer).VersionInfo
$ProvenanceObject = [ordered]@{
    capture_kind = "M59 deployed correction-spelling oracle"
    oracle_repository = "rime/librime"
    oracle_commit = "33e78140250125871856cdc5b42ddc6a5fcd3cd4"
    oracle_release = "1.17.0"
    copied_from_repo_evidence_root = "target/upstream-oracle/1.17.0/extract"
    schema_id = "correction_oracle"
    input = "cu"
    modules = @("default")
    runtime_options_source = "RimeProbe.Capture/CaptureRuntimeOptionPolicy"
    expected_runtime_options = [ordered]@{
        ascii_mode = $false
        full_shape = $false
        ascii_punct = $false
        zh_hans = $false
    }
    translator_config = [ordered]@{
        enable_correction = $false
        enable_completion = $false
        enable_sentence = $false
        spelling_hints = 9
        always_show_comments = $true
    }
    page_size = 5
    deploy = [ordered]@{
        started_utc = $DeployStarted.ToString("o")
        finished_utc = $DeployFinished.ToString("o")
        exit_code = $DeployExitCode
    }
    capture = [ordered]@{
        started_utc = $CaptureStarted.ToString("o")
        finished_utc = $CaptureFinished.ToString("o")
    }
    environment = [ordered]@{
        machine = $env:COMPUTERNAME
        os = [System.Environment]::OSVersion.VersionString
        powershell = $PSVersionTable.PSVersion.ToString()
        process_architecture = [System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture.ToString()
        current_directory = (Get-Location).Path
    }
    binaries = [ordered]@{
        rime_dll = [ordered]@{
            path = $RimeDll
            bytes = (Get-Item -LiteralPath $RimeDll).Length
            sha256 = Sha256 $RimeDll
            file_version = $DllVersion.FileVersion
            product_version = $DllVersion.ProductVersion
        }
        rime_deployer = [ordered]@{
            path = $Deployer
            bytes = (Get-Item -LiteralPath $Deployer).Length
            sha256 = Sha256 $Deployer
            file_version = $DeployerVersion.FileVersion
            product_version = $DeployerVersion.ProductVersion
        }
    }
    probe = [ordered]@{
        path = $ProbeSource
        bytes = (Get-Item -LiteralPath $ProbeSource).Length
        sha256 = Sha256 $ProbeSource
    }
}
$ProvenanceText = (($ProvenanceObject | ConvertTo-Json -Depth 20).Replace("`r`n", "`n").TrimEnd([char]10)) + "`n"
Write-Utf8NoBom $Provenance $ProvenanceText

$ArtifactRows = Get-ChildItem -LiteralPath $Root -Recurse -File |
    Where-Object { $_.FullName -notlike (Join-Path $Root "hashes\*") } |
    Sort-Object FullName |
    ForEach-Object {
        [pscustomobject]@{
            relative_path = $_.FullName.Substring($Root.Length + 1).Replace("\", "/")
            bytes = $_.Length
            sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $_.FullName).Hash.ToLowerInvariant()
        }
    }
$ArtifactRows | Export-Csv -NoTypeInformation -Encoding UTF8 -LiteralPath (Join-Path $Root "hashes\artifact-sha256.csv")

$Summary = [ordered]@{
    output = $Output
    provenance = $Provenance
    artifact_manifest = (Join-Path $Root "hashes\artifact-sha256.csv")
    deploy_exit_code = $DeployExitCode
    capture_sha256 = Sha256 $Output
    candidate_count = @($Cases[0]["all_candidates"]).Count
    candidates = @($Cases[0]["all_candidates"])
}
$Summary | ConvertTo-Json -Depth 20
